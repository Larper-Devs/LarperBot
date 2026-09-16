#![allow(clippy::too_many_arguments)]

mod mongo_sync;
mod profile_card;

use axum::{routing::get, Router};
use mongo_sync::{CacheRecord, MongoSync, SYNC_INTERVAL};
use profile_card::{
    is_profile_theme, render_profile_card, render_ranking_card, ProfileCardInput, RankingCardInput,
    RankingEntry, PROFILE_THEMES,
};
use rand::Rng;
use reqwest::Client as HttpClient;
use rusqlite::{params, Connection, OptionalExtension};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use serenity::all::*;
use serenity::async_trait;
use serenity::http::{LightMethod, Request, Route};
use std::collections::HashMap;
use std::env;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};
use tokio::net::TcpListener;
use tokio::time::{sleep, timeout};
use tracing::{debug, error, info};

type BotResult<T> = Result<T, Box<dyn std::error::Error + Send + Sync>>;

const PURPLE: u32 = 0x8B5CF6;
const GREEN: u32 = 0x2ECC71;
const RED: u32 = 0xED4245;
const BLUE: u32 = 0x5865F2;
const DEFAULT_HTTP_PORT: u16 = 10_000;
const DEFAULT_UNVERIFIED_ROLE_ID: u64 = 918_519_844_020_830_236;
const VERIFIED_ROLE_ID: u64 = 918_519_844_020_830_235;
const CAPTCHA_CHANNEL_ID: u64 = 918_519_844_591_255_613;
const PUNISHMENT_LOG_CHANNEL_ID: u64 = 918_519_846_461_923_329;
const COMPONENTS_V2_FLAG: u64 = 1 << 15;
const CAPTCHA_WARNING: &str = "Não mande mensagens aqui ou você será retirado do servidor!\n\nDo not send messages here or you will be removed from the server!";
const COLOR_ROLE_NAMES: &[&str] = &[
    "Vermelho",
    "Branco",
    "Preto",
    "Vermelho Vinho",
    "Rosa",
    "Amarelo",
    "Verde",
    "Verde Escuro",
    "Azul",
    "Roxo",
    "Laranja",
    "Marrom",
];

async fn health_check() -> &'static str {
    "LarperBot online"
}

fn health_router() -> Router {
    Router::new()
        .route("/", get(health_check))
        .route("/health", get(health_check))
        .route("/healthz", get(health_check))
}

async fn run_health_server(listener: TcpListener) -> BotResult<()> {
    axum::serve(listener, health_router()).await?;
    Ok(())
}

fn http_port() -> Result<u16, String> {
    match env::var("PORT") {
        Ok(value) => value
            .parse::<u16>()
            .map_err(|_| format!("PORT inválida: {value}")),
        Err(_) => Ok(DEFAULT_HTTP_PORT),
    }
}

#[derive(Clone)]
struct Config {
    token: String,
    prefix: String,
    prefix_commands: bool,
    automod: bool,
    levels: bool,
    member_events: bool,
    unverified_role_id: RoleId,
    database_path: String,
    mongo_uri: Option<String>,
}

impl Config {
    fn from_env() -> Result<Self, String> {
        let token = env::var("DISCORD_TOKEN")
            .or_else(|_| env::var("TOKEN"))
            .map_err(|_| "Defina DISCORD_TOKEN (ou TOKEN) no ambiente.".to_string())?;
        if token.trim().is_empty() {
            return Err("DISCORD_TOKEN não pode estar vazio.".into());
        }
        Ok(Self {
            token,
            prefix: env::var("DEFAULT_PREFIX").unwrap_or_else(|_| "!".into()),
            prefix_commands: env_bool("ENABLE_PREFIX_COMMANDS"),
            automod: env_bool("ENABLE_AUTOMOD"),
            levels: env_bool_default("ENABLE_LEVELS", true),
            member_events: env_bool("ENABLE_MEMBER_EVENTS"),
            unverified_role_id: env::var("UNVERIFIED_ROLE_ID")
                .unwrap_or_else(|_| DEFAULT_UNVERIFIED_ROLE_ID.to_string())
                .parse::<u64>()
                .map(RoleId::new)
                .map_err(|_| "UNVERIFIED_ROLE_ID precisa ser um ID numérico válido.".to_string())?,
            database_path: env::var("DATABASE_PATH")
                .unwrap_or_else(|_| "./larperbot.sqlite".into()),
            mongo_uri: env::var("MONGO_URI")
                .ok()
                .filter(|value| !value.trim().is_empty()),
        })
    }
}

fn env_bool(name: &str) -> bool {
    env_bool_default(name, false)
}

fn env_bool_default(name: &str, default: bool) -> bool {
    env::var(name)
        .map(|v| v.eq_ignore_ascii_case("true"))
        .unwrap_or(default)
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct Profile {
    guild_id: String,
    user_id: String,
    username: String,
    wallet: i64,
    bank: i64,
    total_xp: i64,
    weekly_xp: i64,
    monthly_xp: i64,
    week_key: String,
    month_key: String,
    last_xp_at: i64,
    last_daily_at: i64,
    last_work_at: i64,
    afk_message: Option<String>,
    profile_background: String,
    inventory: Vec<InventoryItem>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct InventoryItem {
    item_id: String,
    quantity: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct GuildSettings {
    automod_enabled: bool,
    spam_limit: i64,
    spam_window: i64,
    mention_limit: i64,
    caps_percent: i64,
    blocked_words: Vec<String>,
    blocked_domains: Vec<String>,
    exempt_roles: Vec<String>,
    exempt_channels: Vec<String>,
    automod_log_channel: Option<String>,
    welcome_enabled: bool,
    welcome_channel: Option<String>,
    welcome_message: String,
    leave_enabled: bool,
    leave_channel: Option<String>,
    leave_message: String,
    auto_role: Option<String>,
    level_enabled: bool,
    xp_cooldown: i64,
    xp_min: i64,
    xp_max: i64,
    level_channel: Option<String>,
    rules_title: String,
    rules_description: String,
    rules_color: String,
    rules: Vec<RuleItem>,
    rules_channel: Option<String>,
    rules_message: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct RuleItem {
    id: i64,
    title: String,
    description: String,
}

impl Default for GuildSettings {
    fn default() -> Self {
        Self {
            automod_enabled: false,
            spam_limit: 6,
            spam_window: 8,
            mention_limit: 5,
            caps_percent: 75,
            blocked_words: vec![],
            blocked_domains: vec![],
            exempt_roles: vec![],
            exempt_channels: vec![],
            automod_log_channel: None,
            welcome_enabled: false,
            welcome_channel: None,
            welcome_message: "Bem-vindo(a), {mention}!".into(),
            leave_enabled: false,
            leave_channel: None,
            leave_message: "{username} saiu do servidor.".into(),
            auto_role: None,
            level_enabled: true,
            xp_cooldown: 60,
            xp_min: 15,
            xp_max: 25,
            level_channel: None,
            rules_title: "Regras do servidor".into(),
            rules_description: "Leia e respeite as regras para manter a comunidade organizada."
                .into(),
            rules_color: "#8B5CF6".into(),
            rules: vec![],
            rules_channel: None,
            rules_message: None,
        }
    }
}

struct Db {
    conn: Connection,
}

impl Db {
    fn open(path: &str) -> rusqlite::Result<Self> {
        let conn = Connection::open(path)?;
        conn.busy_timeout(Duration::from_secs(5))?;
        conn.execute_batch(
            "PRAGMA journal_mode = WAL;
             PRAGMA synchronous = NORMAL;
             CREATE TABLE IF NOT EXISTS profiles (
               guild_id TEXT NOT NULL, user_id TEXT NOT NULL, username TEXT NOT NULL DEFAULT '',
               wallet INTEGER NOT NULL DEFAULT 0, bank INTEGER NOT NULL DEFAULT 0,
               total_xp INTEGER NOT NULL DEFAULT 0, weekly_xp INTEGER NOT NULL DEFAULT 0,
               monthly_xp INTEGER NOT NULL DEFAULT 0, week_key TEXT NOT NULL DEFAULT '',
               month_key TEXT NOT NULL DEFAULT '', last_xp_at INTEGER NOT NULL DEFAULT 0,
               last_daily_at INTEGER NOT NULL DEFAULT 0, last_work_at INTEGER NOT NULL DEFAULT 0,
               afk_message TEXT, profile_background TEXT NOT NULL DEFAULT 'midnight',
               inventory TEXT NOT NULL DEFAULT '[]', PRIMARY KEY(guild_id, user_id)
             );
             CREATE TABLE IF NOT EXISTS guild_settings (guild_id TEXT PRIMARY KEY, data TEXT NOT NULL);
             CREATE TABLE IF NOT EXISTS transactions (
               id INTEGER PRIMARY KEY, guild_id TEXT NOT NULL, user_id TEXT NOT NULL,
               from_user TEXT, to_user TEXT, kind TEXT NOT NULL, amount INTEGER NOT NULL,
               reason TEXT NOT NULL, created_at INTEGER NOT NULL
             );
             CREATE INDEX IF NOT EXISTS transactions_lookup ON transactions(guild_id, user_id, created_at DESC);
             CREATE TABLE IF NOT EXISTS reminders (
               id INTEGER PRIMARY KEY, guild_id TEXT NOT NULL, user_id TEXT NOT NULL,
               channel_id TEXT NOT NULL, message TEXT NOT NULL, remind_at INTEGER NOT NULL, sent INTEGER NOT NULL DEFAULT 0
             );
             CREATE TABLE IF NOT EXISTS warnings (
               id INTEGER PRIMARY KEY, guild_id TEXT NOT NULL, target_id TEXT NOT NULL,
               target_tag TEXT NOT NULL, moderator_id TEXT NOT NULL, reason TEXT NOT NULL, created_at INTEGER NOT NULL
             );
             CREATE TABLE IF NOT EXISTS reels (
               id INTEGER PRIMARY KEY, guild_id TEXT NOT NULL, platform TEXT NOT NULL,
               url TEXT NOT NULL, title TEXT NOT NULL, added_by TEXT NOT NULL,
               likes TEXT NOT NULL DEFAULT '[]', reposts TEXT NOT NULL DEFAULT '[]', share_count INTEGER NOT NULL DEFAULT 0,
               UNIQUE(guild_id, url)
             );",
        )?;
        Ok(Self { conn })
    }

    fn settings(&self, guild_id: &str) -> rusqlite::Result<GuildSettings> {
        let data: Option<String> = self
            .conn
            .query_row(
                "SELECT data FROM guild_settings WHERE guild_id = ?1",
                params![guild_id],
                |row| row.get(0),
            )
            .optional()?;
        match data {
            Some(json) => Ok(serde_json::from_str(&json).unwrap_or_default()),
            None => Ok(GuildSettings::default()),
        }
    }

    fn save_settings(&self, guild_id: &str, settings: &GuildSettings) -> rusqlite::Result<()> {
        let json = serde_json::to_string(settings).unwrap_or_else(|_| "{}".into());
        self.conn.execute(
            "INSERT INTO guild_settings(guild_id,data) VALUES(?1,?2)
             ON CONFLICT(guild_id) DO UPDATE SET data=excluded.data",
            params![guild_id, json],
        )?;
        Ok(())
    }

    fn profile(&self, guild_id: &str, user_id: &str, username: &str) -> rusqlite::Result<Profile> {
        let week = week_key();
        let month = month_key();
        self.conn.execute(
            "INSERT INTO profiles(guild_id,user_id,username,week_key,month_key)
             VALUES(?1,?2,?3,?4,?5)
             ON CONFLICT(guild_id,user_id) DO UPDATE SET username=excluded.username",
            params![guild_id, user_id, username, week, month],
        )?;
        let mut p: Profile = self.conn.query_row(
            "SELECT guild_id,user_id,username,wallet,bank,total_xp,weekly_xp,monthly_xp,week_key,month_key,
                    last_xp_at,last_daily_at,last_work_at,afk_message,profile_background,inventory
             FROM profiles WHERE guild_id=?1 AND user_id=?2", params![guild_id, user_id], |r| {
                let inventory: String = r.get(15)?;
                Ok(Profile { guild_id: r.get(0)?, user_id: r.get(1)?, username: r.get(2)?, wallet: r.get(3)?, bank: r.get(4)?, total_xp: r.get(5)?, weekly_xp: r.get(6)?, monthly_xp: r.get(7)?, week_key: r.get(8)?, month_key: r.get(9)?, last_xp_at: r.get(10)?, last_daily_at: r.get(11)?, last_work_at: r.get(12)?, afk_message: r.get(13)?, profile_background: r.get(14)?, inventory: serde_json::from_str(&inventory).unwrap_or_default() })
            },
        )?;
        if p.week_key != week {
            p.week_key = week;
            p.weekly_xp = 0;
        }
        if p.month_key != month {
            p.month_key = month;
            p.monthly_xp = 0;
        }
        if p.profile_background.is_empty() {
            p.profile_background = "midnight".into();
        }
        self.save_profile(&p)?;
        Ok(p)
    }

    fn save_profile(&self, p: &Profile) -> rusqlite::Result<()> {
        let inventory = serde_json::to_string(&p.inventory).unwrap_or_else(|_| "[]".into());
        self.conn.execute(
            "UPDATE profiles SET username=?3,wallet=?4,bank=?5,total_xp=?6,weekly_xp=?7,monthly_xp=?8,
             week_key=?9,month_key=?10,last_xp_at=?11,last_daily_at=?12,last_work_at=?13,afk_message=?14,
             profile_background=?15,inventory=?16 WHERE guild_id=?1 AND user_id=?2",
            params![p.guild_id,p.user_id,p.username,p.wallet,p.bank,p.total_xp,p.weekly_xp,p.monthly_xp,p.week_key,p.month_key,p.last_xp_at,p.last_daily_at,p.last_work_at,p.afk_message,p.profile_background,inventory],
        )?;
        Ok(())
    }

    fn transaction(
        &self,
        guild_id: &str,
        user_id: &str,
        from: Option<&str>,
        to: Option<&str>,
        kind: &str,
        amount: i64,
        reason: &str,
    ) -> rusqlite::Result<()> {
        self.conn.execute("INSERT INTO transactions(guild_id,user_id,from_user,to_user,kind,amount,reason,created_at) VALUES(?1,?2,?3,?4,?5,?6,?7,?8)", params![guild_id,user_id,from,to,kind,amount,reason,now()])?;
        Ok(())
    }

    fn next_warning(
        &self,
        guild_id: &str,
        target_id: &str,
        target_tag: &str,
        moderator_id: &str,
        reason: &str,
    ) -> rusqlite::Result<i64> {
        self.conn.execute("INSERT INTO warnings(guild_id,target_id,target_tag,moderator_id,reason,created_at) VALUES(?1,?2,?3,?4,?5,?6)", params![guild_id,target_id,target_tag,moderator_id,reason,now()])?;
        Ok(self.conn.last_insert_rowid())
    }

    fn cache_snapshot(&self) -> rusqlite::Result<Vec<CacheRecord>> {
        let mut records = Vec::new();

        {
            let mut statement = self.conn.prepare(
                "SELECT guild_id,user_id,username,wallet,bank,total_xp,weekly_xp,monthly_xp,
                        week_key,month_key,last_xp_at,last_daily_at,last_work_at,afk_message,
                        profile_background,inventory
                 FROM profiles",
            )?;
            let rows = statement.query_map([], |row| {
                let guild_id: String = row.get(0)?;
                let user_id: String = row.get(1)?;
                let data = mongodb::bson::doc! {
                    "guild_id": &guild_id,
                    "user_id": &user_id,
                    "username": row.get::<_, String>(2)?,
                    "wallet": row.get::<_, i64>(3)?,
                    "bank": row.get::<_, i64>(4)?,
                    "total_xp": row.get::<_, i64>(5)?,
                    "weekly_xp": row.get::<_, i64>(6)?,
                    "monthly_xp": row.get::<_, i64>(7)?,
                    "week_key": row.get::<_, String>(8)?,
                    "month_key": row.get::<_, String>(9)?,
                    "last_xp_at": row.get::<_, i64>(10)?,
                    "last_daily_at": row.get::<_, i64>(11)?,
                    "last_work_at": row.get::<_, i64>(12)?,
                    "afk_message": bson_optional_string(row.get::<_, Option<String>>(13)?),
                    "profile_background": row.get::<_, String>(14)?,
                    "inventory": row.get::<_, String>(15)?,
                };
                Ok(CacheRecord {
                    id: format!("profiles:{guild_id}:{user_id}"),
                    table: "profiles".to_string(),
                    data,
                })
            })?;
            for row in rows {
                records.push(row?);
            }
        }

        {
            let mut statement = self
                .conn
                .prepare("SELECT guild_id,data FROM guild_settings")?;
            let rows = statement.query_map([], |row| {
                let guild_id: String = row.get(0)?;
                Ok(CacheRecord {
                    id: format!("guild_settings:{guild_id}"),
                    table: "guild_settings".to_string(),
                    data: mongodb::bson::doc! {
                        "guild_id": &guild_id,
                        "data": row.get::<_, String>(1)?,
                    },
                })
            })?;
            for row in rows {
                records.push(row?);
            }
        }

        {
            let mut statement = self.conn.prepare(
                "SELECT id,guild_id,user_id,from_user,to_user,kind,amount,reason,created_at
                 FROM transactions",
            )?;
            let rows = statement.query_map([], |row| {
                let id: i64 = row.get(0)?;
                Ok(CacheRecord {
                    id: format!("transactions:{id}"),
                    table: "transactions".to_string(),
                    data: mongodb::bson::doc! {
                        "id": id,
                        "guild_id": row.get::<_, String>(1)?,
                        "user_id": row.get::<_, String>(2)?,
                        "from_user": bson_optional_string(row.get::<_, Option<String>>(3)?),
                        "to_user": bson_optional_string(row.get::<_, Option<String>>(4)?),
                        "kind": row.get::<_, String>(5)?,
                        "amount": row.get::<_, i64>(6)?,
                        "reason": row.get::<_, String>(7)?,
                        "created_at": row.get::<_, i64>(8)?,
                    },
                })
            })?;
            for row in rows {
                records.push(row?);
            }
        }

        {
            let mut statement = self.conn.prepare(
                "SELECT id,guild_id,user_id,channel_id,message,remind_at,sent FROM reminders",
            )?;
            let rows = statement.query_map([], |row| {
                let id: i64 = row.get(0)?;
                Ok(CacheRecord {
                    id: format!("reminders:{id}"),
                    table: "reminders".to_string(),
                    data: mongodb::bson::doc! {
                        "id": id,
                        "guild_id": row.get::<_, String>(1)?,
                        "user_id": row.get::<_, String>(2)?,
                        "channel_id": row.get::<_, String>(3)?,
                        "message": row.get::<_, String>(4)?,
                        "remind_at": row.get::<_, i64>(5)?,
                        "sent": row.get::<_, i64>(6)?,
                    },
                })
            })?;
            for row in rows {
                records.push(row?);
            }
        }

        {
            let mut statement = self.conn.prepare(
                "SELECT id,guild_id,target_id,target_tag,moderator_id,reason,created_at FROM warnings",
            )?;
            let rows = statement.query_map([], |row| {
                let id: i64 = row.get(0)?;
                Ok(CacheRecord {
                    id: format!("warnings:{id}"),
                    table: "warnings".to_string(),
                    data: mongodb::bson::doc! {
                        "id": id,
                        "guild_id": row.get::<_, String>(1)?,
                        "target_id": row.get::<_, String>(2)?,
                        "target_tag": row.get::<_, String>(3)?,
                        "moderator_id": row.get::<_, String>(4)?,
                        "reason": row.get::<_, String>(5)?,
                        "created_at": row.get::<_, i64>(6)?,
                    },
                })
            })?;
            for row in rows {
                records.push(row?);
            }
        }

        {
            let mut statement = self.conn.prepare(
                "SELECT id,guild_id,platform,url,title,added_by,likes,reposts,share_count FROM reels",
            )?;
            let rows = statement.query_map([], |row| {
                let id: i64 = row.get(0)?;
                Ok(CacheRecord {
                    id: format!("reels:{id}"),
                    table: "reels".to_string(),
                    data: mongodb::bson::doc! {
                        "id": id,
                        "guild_id": row.get::<_, String>(1)?,
                        "platform": row.get::<_, String>(2)?,
                        "url": row.get::<_, String>(3)?,
                        "title": row.get::<_, String>(4)?,
                        "added_by": row.get::<_, String>(5)?,
                        "likes": row.get::<_, String>(6)?,
                        "reposts": row.get::<_, String>(7)?,
                        "share_count": row.get::<_, i64>(8)?,
                    },
                })
            })?;
            for row in rows {
                records.push(row?);
            }
        }

        Ok(records)
    }

    fn restore_cache(&mut self, records: &[CacheRecord]) -> rusqlite::Result<usize> {
        let tx = self.conn.transaction()?;
        let mut restored = 0;

        for record in records {
            let data = &record.data;
            match record.table.as_str() {
                "profiles" => {
                    tx.execute(
                        "INSERT INTO profiles(
                           guild_id,user_id,username,wallet,bank,total_xp,weekly_xp,monthly_xp,
                           week_key,month_key,last_xp_at,last_daily_at,last_work_at,afk_message,
                           profile_background,inventory
                         ) VALUES(?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13,?14,?15,?16)
                         ON CONFLICT(guild_id,user_id) DO UPDATE SET
                           username=excluded.username,wallet=excluded.wallet,bank=excluded.bank,
                           total_xp=excluded.total_xp,weekly_xp=excluded.weekly_xp,monthly_xp=excluded.monthly_xp,
                           week_key=excluded.week_key,month_key=excluded.month_key,last_xp_at=excluded.last_xp_at,
                           last_daily_at=excluded.last_daily_at,last_work_at=excluded.last_work_at,
                           afk_message=excluded.afk_message,profile_background=excluded.profile_background,
                           inventory=excluded.inventory",
                        params![
                            cache_string(data, "guild_id"),
                            cache_string(data, "user_id"),
                            cache_string(data, "username"),
                            cache_number(data, "wallet"),
                            cache_number(data, "bank"),
                            cache_number(data, "total_xp"),
                            cache_number(data, "weekly_xp"),
                            cache_number(data, "monthly_xp"),
                            cache_string(data, "week_key"),
                            cache_string(data, "month_key"),
                            cache_number(data, "last_xp_at"),
                            cache_number(data, "last_daily_at"),
                            cache_number(data, "last_work_at"),
                            cache_optional_string(data, "afk_message"),
                            cache_string_or(data, "profile_background", "midnight"),
                            cache_string_or(data, "inventory", "[]"),
                        ],
                    )?;
                }
                "guild_settings" => {
                    tx.execute(
                        "INSERT INTO guild_settings(guild_id,data) VALUES(?1,?2)
                         ON CONFLICT(guild_id) DO UPDATE SET data=excluded.data",
                        params![cache_string(data, "guild_id"), cache_string(data, "data")],
                    )?;
                }
                "transactions" => {
                    tx.execute(
                        "INSERT INTO transactions(id,guild_id,user_id,from_user,to_user,kind,amount,reason,created_at)
                         VALUES(?1,?2,?3,?4,?5,?6,?7,?8,?9)
                         ON CONFLICT(id) DO UPDATE SET guild_id=excluded.guild_id,user_id=excluded.user_id,
                           from_user=excluded.from_user,to_user=excluded.to_user,kind=excluded.kind,
                           amount=excluded.amount,reason=excluded.reason,created_at=excluded.created_at",
                        params![
                            cache_number(data, "id"),
                            cache_string(data, "guild_id"),
                            cache_string(data, "user_id"),
                            cache_optional_string(data, "from_user"),
                            cache_optional_string(data, "to_user"),
                            cache_string(data, "kind"),
                            cache_number(data, "amount"),
                            cache_string(data, "reason"),
                            cache_number(data, "created_at"),
                        ],
                    )?;
                }
                "reminders" => {
                    tx.execute(
                        "INSERT INTO reminders(id,guild_id,user_id,channel_id,message,remind_at,sent)
                         VALUES(?1,?2,?3,?4,?5,?6,?7)
                         ON CONFLICT(id) DO UPDATE SET guild_id=excluded.guild_id,user_id=excluded.user_id,
                           channel_id=excluded.channel_id,message=excluded.message,remind_at=excluded.remind_at,
                           sent=excluded.sent",
                        params![
                            cache_number(data, "id"),
                            cache_string(data, "guild_id"),
                            cache_string(data, "user_id"),
                            cache_string(data, "channel_id"),
                            cache_string(data, "message"),
                            cache_number(data, "remind_at"),
                            cache_number(data, "sent"),
                        ],
                    )?;
                }
                "warnings" => {
                    tx.execute(
                        "INSERT INTO warnings(id,guild_id,target_id,target_tag,moderator_id,reason,created_at)
                         VALUES(?1,?2,?3,?4,?5,?6,?7)
                         ON CONFLICT(id) DO UPDATE SET guild_id=excluded.guild_id,target_id=excluded.target_id,
                           target_tag=excluded.target_tag,moderator_id=excluded.moderator_id,
                           reason=excluded.reason,created_at=excluded.created_at",
                        params![
                            cache_number(data, "id"),
                            cache_string(data, "guild_id"),
                            cache_string(data, "target_id"),
                            cache_string(data, "target_tag"),
                            cache_string(data, "moderator_id"),
                            cache_string(data, "reason"),
                            cache_number(data, "created_at"),
                        ],
                    )?;
                }
                "reels" => {
                    tx.execute(
                        "INSERT INTO reels(id,guild_id,platform,url,title,added_by,likes,reposts,share_count)
                         VALUES(?1,?2,?3,?4,?5,?6,?7,?8,?9)
                         ON CONFLICT(id) DO UPDATE SET guild_id=excluded.guild_id,platform=excluded.platform,
                           url=excluded.url,title=excluded.title,added_by=excluded.added_by,likes=excluded.likes,
                           reposts=excluded.reposts,share_count=excluded.share_count",
                        params![
                            cache_number(data, "id"),
                            cache_string(data, "guild_id"),
                            cache_string(data, "platform"),
                            cache_string(data, "url"),
                            cache_string(data, "title"),
                            cache_string(data, "added_by"),
                            cache_string_or(data, "likes", "[]"),
                            cache_string_or(data, "reposts", "[]"),
                            cache_number(data, "share_count"),
                        ],
                    )?;
                }
                _ => continue,
            }
            restored += 1;
        }

        tx.commit()?;
        Ok(restored)
    }
}

fn bson_optional_string(value: Option<String>) -> mongodb::bson::Bson {
    value
        .map(mongodb::bson::Bson::String)
        .unwrap_or(mongodb::bson::Bson::Null)
}

fn cache_string(data: &mongodb::bson::Document, key: &str) -> String {
    data.get_str(key).unwrap_or_default().to_string()
}

fn cache_string_or(data: &mongodb::bson::Document, key: &str, default: &str) -> String {
    let value = cache_string(data, key);
    if value.is_empty() {
        default.to_string()
    } else {
        value
    }
}

fn cache_optional_string(data: &mongodb::bson::Document, key: &str) -> Option<String> {
    data.get_str(key).ok().map(str::to_owned)
}

fn cache_number(data: &mongodb::bson::Document, key: &str) -> i64 {
    match data.get(key) {
        Some(mongodb::bson::Bson::Int32(value)) => i64::from(*value),
        Some(mongodb::bson::Bson::Int64(value)) => *value,
        Some(mongodb::bson::Bson::Double(value)) => *value as i64,
        _ => 0,
    }
}

async fn push_local_cache(sync: &MongoSync, state: &Arc<State>, reason: &str) {
    let snapshot = {
        let db = state.db();
        match db.cache_snapshot() {
            Ok(snapshot) => snapshot,
            Err(error) => {
                error!("Falha ao criar snapshot do cache local ({reason}): {error}");
                return;
            }
        }
    };

    match sync.push_snapshot(&snapshot).await {
        Ok(count) => info!("Cache local enviado ao MongoDB ({reason}): {count} registro(s)."),
        Err(error) => {
            error!("Falha ao enviar cache local ao MongoDB ({reason}); haverá retry: {error}")
        }
    }
}

async fn load_remote_cache(sync: &MongoSync, state: &Arc<State>, reason: &str) -> bool {
    let records = match timeout(Duration::from_secs(30), sync.load_snapshot()).await {
        Ok(Ok(records)) => records,
        Ok(Err(error)) => {
            error!("Falha ao carregar o MongoDB ({reason}); cache local preservado: {error}");
            return false;
        }
        Err(_) => {
            error!("Timeout ao carregar o MongoDB ({reason}); cache local preservado.");
            return false;
        }
    };

    let restore_result = {
        let mut db = state.db();
        db.restore_cache(&records)
    };
    match restore_result {
        Ok(restored) => {
            info!("Cache local carregado do MongoDB ({reason}): {restored} registro(s).");
            true
        }
        Err(error) => {
            error!(
                "Falha ao gravar o snapshot do MongoDB no cache local ({reason}); carga rejeitada: {error}"
            );
            false
        }
    }
}

async fn mongo_sync_loop(
    uri: String,
    state: Arc<State>,
    mut sync: Option<MongoSync>,
    mut cache_loaded: bool,
) {
    loop {
        sleep(SYNC_INTERVAL).await;

        if sync.is_none() {
            match timeout(Duration::from_secs(20), MongoSync::connect(&uri)).await {
                Ok(Ok(connected)) => {
                    info!("MongoDB reconectado; retomando sincronização do cache local.");
                    sync = Some(connected);
                    cache_loaded = false;
                }
                Ok(Err(error)) => {
                    error!("Retry do MongoDB falhou; cache local continua ativo: {error}");
                    continue;
                }
                Err(_) => {
                    error!("Timeout no retry de conexão com MongoDB; cache local continua ativo.");
                    continue;
                }
            }
        }

        if let Some(connected) = sync.as_ref() {
            if !cache_loaded {
                if !load_remote_cache(connected, &state, "retry da carga inicial").await {
                    sync = None;
                    continue;
                }
                cache_loaded = true;
            }
            push_local_cache(connected, &state, "sincronização periódica").await;
        }
    }
}

struct State {
    config: Config,
    db: Mutex<Db>,
    http: HttpClient,
    spam: Mutex<HashMap<String, Vec<Instant>>>,
    resume: Mutex<HashMap<UserId, (Instant, u8)>>,
    blackjack: Mutex<HashMap<String, BlackjackGame>>,
    bot_id: Mutex<Option<UserId>>,
}

#[derive(Clone)]
struct BlackjackGame {
    bet: i64,
    player: Vec<u8>,
    dealer: Vec<u8>,
}

impl State {
    fn db(&self) -> std::sync::MutexGuard<'_, Db> {
        self.db.lock().expect("database mutex poisoned")
    }
}

struct Handler {
    state: Arc<State>,
}

#[async_trait]
impl EventHandler for Handler {
    async fn ready(&self, ctx: Context, ready: Ready) {
        *self.state.bot_id.lock().expect("bot id mutex poisoned") = Some(ready.user.id);
        info!(
            "Bot {} online em {} servidor(es).",
            ready.user.name,
            ready.guilds.len()
        );
        if let Err(e) = register_commands(&ctx).await {
            error!("Falha ao registrar slash commands: {e}");
        }
        let state = Arc::clone(&self.state);
        let http = ctx.http.clone();
        tokio::spawn(async move {
            reminder_loop(http, state).await;
        });
    }

    async fn interaction_create(&self, ctx: Context, interaction: Interaction) {
        let result = match &interaction {
            Interaction::Command(command) => handle_command(&ctx, &self.state, command).await,
            Interaction::Component(component) => {
                handle_component(&ctx, &self.state, component).await
            }
            Interaction::Autocomplete(auto) => handle_autocomplete(&ctx, auto).await,
            _ => Ok(()),
        };
        if let Err(e) = result {
            error!("Erro no processamento da interação: {e}");
            if let Interaction::Command(command) = &interaction {
                let _ = reply(
                    command,
                    &ctx.http,
                    "Ocorreu um erro ao executar esse comando.",
                    true,
                )
                .await;
            }
        }
    }

    async fn message(&self, ctx: Context, message: Message) {
        if message.channel_id == ChannelId::new(CAPTCHA_CHANNEL_ID) {
            handle_captcha_message(&ctx, &self.state, &message).await;
            return;
        }
        if message.author.bot {
            return;
        }
        if message.guild_id.is_some() {
            if self.state.config.automod && automod(&ctx, &self.state, &message).await {
                return;
            }
            if self.state.config.levels {
                gain_xp(&ctx, &self.state, &message).await;
            }
        }
        if !self.state.config.prefix_commands
            || !message.content.starts_with(&self.state.config.prefix)
        {
            return;
        }
        let mut parts = message.content[self.state.config.prefix.len()..].split_whitespace();
        let Some(name) = parts.next() else {
            return;
        };
        let args: Vec<&str> = parts.collect();
        if let Err(e) = handle_prefix(&ctx, &self.state, &message, name, &args).await {
            error!("Erro em comando de prefixo: {e}");
        }
    }

    async fn guild_audit_log_entry_create(
        &self,
        ctx: Context,
        entry: AuditLogEntry,
        guild_id: GuildId,
    ) {
        handle_audit_log_entry(&ctx, &self.state, entry, guild_id).await;
    }

    async fn guild_member_addition(&self, ctx: Context, member: Member) {
        if !self.state.config.member_events {
            return;
        }
        info!(
            "Novo membro detectado: {} entrou no servidor {}.",
            member.user.name, member.guild_id
        );

        let settings = {
            let db = self.state.db();
            db.settings(&member.guild_id.to_string())
        };
        match settings {
            Ok(settings) if settings.welcome_enabled => {
                if let Some(channel) = settings
                    .welcome_channel
                    .and_then(|id| id.parse::<u64>().ok())
                    .map(ChannelId::new)
                {
                    let content = replace_placeholders(
                        &settings.welcome_message,
                        &[
                            ("mention", member.user.mention().to_string()),
                            ("server", member.guild_id.to_string()),
                            ("memberCount", "".into()),
                        ],
                    );
                    if let Err(error) = channel.say(&ctx.http, content).await {
                        error!(
                            "Falha ao enviar boas-vindas para {}: {error}",
                            member.user.name
                        );
                    }
                }
            }
            Ok(_) => {}
            Err(error) => {
                error!(
                    "Falha ao ler configurações de entrada do servidor {}: {error}",
                    member.guild_id
                );
            }
        }

        let role = self.state.config.unverified_role_id;
        match member.add_role(&ctx.http, role).await {
            Ok(()) => info!(
                "Cargo não verificado {} atribuído a {} no servidor {}.",
                role, member.user.name, member.guild_id
            ),
            Err(error) => error!(
                "Falha ao atribuir o cargo não verificado {} para {} no servidor {}: {error}.",
                role, member.user.name, member.guild_id
            ),
        }
    }

    async fn guild_member_removal(
        &self,
        ctx: Context,
        guild_id: GuildId,
        user: User,
        _member: Option<Member>,
    ) {
        if !self.state.config.member_events {
            return;
        }
        let Ok(settings) = self.state.db().settings(&guild_id.to_string()) else {
            return;
        };
        if settings.leave_enabled {
            if let Some(id) = settings
                .leave_channel
                .and_then(|v| v.parse::<u64>().ok())
                .map(ChannelId::new)
            {
                let _ = id
                    .say(
                        &ctx.http,
                        replace_placeholders(
                            &settings.leave_message,
                            &[("username", user.name), ("server", guild_id.to_string())],
                        ),
                    )
                    .await;
            }
        }
    }
}

fn now() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs() as i64
}
fn week_key() -> String {
    format!("{}-W{:02}", chrono_year(), (now() / 604800) % 53 + 1)
}
fn month_key() -> String {
    format!("{}-{:02}", chrono_year(), ((now() / 2_629_746) % 12 + 1))
}
fn chrono_year() -> i64 {
    1970 + now() / 31_556_952
}
fn replace_placeholders(input: &str, values: &[(&str, String)]) -> String {
    values.iter().fold(input.to_string(), |acc, (key, value)| {
        acc.replace(&format!("{{{key}}}"), value)
    })
}

async fn register_commands(ctx: &Context) -> BotResult<()> {
    let commands = command_definitions();
    if let Some(guild) = env::var("GUILD_ID")
        .ok()
        .and_then(|v| v.parse::<u64>().ok())
        .map(GuildId::new)
    {
        guild.set_commands(&ctx.http, commands).await?;
        info!("Slash commands registrados no servidor de desenvolvimento.");
    } else {
        Command::set_global_commands(&ctx.http, commands).await?;
        info!("Slash commands registrados globalmente.");
    }
    Ok(())
}

fn admin(mut command: CreateCommand) -> CreateCommand {
    command = command.default_member_permissions(Permissions::ADMINISTRATOR);
    command
}
fn opt(
    kind: CommandOptionType,
    name: &str,
    description: &str,
    required: bool,
) -> CreateCommandOption {
    CreateCommandOption::new(kind, name, description).required(required)
}
fn sub(name: &str, description: &str, options: Vec<CreateCommandOption>) -> CreateCommandOption {
    options.into_iter().fold(
        CreateCommandOption::new(CommandOptionType::SubCommand, name, description),
        |s, o| s.add_sub_option(o),
    )
}

fn command_definitions() -> Vec<CreateCommand> {
    let profile_theme_option = PROFILE_THEMES.iter().fold(
        opt(
            CommandOptionType::String,
            "tema",
            "Tema visual do perfil.",
            true,
        ),
        |option, (value, label)| option.add_string_choice(*label, *value),
    );
    let c = vec![
        CreateCommand::new("help")
            .description("Abre a central de ajuda por categoria.")
            .add_option(opt(
                CommandOptionType::String,
                "comando",
                "Comando consultado.",
                false,
            )),
        CreateCommand::new("ping").description("Exibe a latência do bot."),
        CreateCommand::new("avatar")
            .description("Exibe o avatar de um usuário.")
            .add_option(opt(
                CommandOptionType::User,
                "usuario",
                "Usuário consultado.",
                false,
            )),
        CreateCommand::new("userinfo")
            .description("Exibe informações de um usuário.")
            .add_option(opt(
                CommandOptionType::User,
                "usuario",
                "Usuário consultado.",
                false,
            )),
        CreateCommand::new("serverinfo").description("Exibe informações do servidor."),
        CreateCommand::new("calc")
            .description("Calcula uma expressão com segurança.")
            .add_option(opt(
                CommandOptionType::String,
                "expressao",
                "Expressão matemática.",
                true,
            )),
        CreateCommand::new("devex")
            .description("Calcula o valor estimado de Robux.")
            .add_option(opt(
                CommandOptionType::Integer,
                "robux",
                "Quantidade de Robux.",
                true,
            ))
            .add_option(
                opt(
                    CommandOptionType::String,
                    "moeda",
                    "Código ISO da moeda.",
                    true,
                )
                .set_autocomplete(true),
            ),
        CreateCommand::new("resume")
            .description("Resume o conteúdo de um site.")
            .add_option(opt(CommandOptionType::String, "url", "URL do site.", true)),
        admin(
            CreateCommand::new("verify")
                .description("Publica o painel de verificação.")
                .add_option(opt(
                    CommandOptionType::Channel,
                    "canal",
                    "Canal do painel.",
                    false,
                )),
        ),
        admin(
            CreateCommand::new("captcha")
                .description("Publica o aviso e ativa a proteção do canal #captcha."),
        ),
        admin(
            CreateCommand::new("tech")
                .description("Publica o painel de tecnologias.")
                .add_option(opt(
                    CommandOptionType::Channel,
                    "canal",
                    "Canal do painel.",
                    false,
                )),
        ),
        admin(
            CreateCommand::new("cores")
                .description("Publica o painel de cores.")
                .add_option(opt(
                    CommandOptionType::Channel,
                    "canal",
                    "Canal do painel.",
                    false,
                )),
        ),
        admin(
            CreateCommand::new("ticket")
                .description("Publica o painel de tickets.")
                .add_option(opt(
                    CommandOptionType::Channel,
                    "canal",
                    "Canal do painel.",
                    false,
                )),
        ),
        CreateCommand::new("reels").description("Exibe um Reel cadastrado."),
        CreateCommand::new("tkk").description("Exibe um TikTok cadastrado."),
        admin(
            CreateCommand::new("feed")
                .description("Administra o feed de vídeos.")
                .add_option(sub(
                    "adicionar",
                    "Adiciona um vídeo.",
                    vec![
                        opt(
                            CommandOptionType::String,
                            "plataforma",
                            "instagram ou tiktok.",
                            true,
                        ),
                        opt(CommandOptionType::String, "url", "URL original.", true),
                        opt(CommandOptionType::String, "titulo", "Título.", false),
                    ],
                ))
                .add_option(sub(
                    "remover",
                    "Remove um vídeo.",
                    vec![opt(CommandOptionType::String, "url", "URL.", true)],
                ))
                .add_option(sub(
                    "listar",
                    "Lista os vídeos.",
                    vec![opt(
                        CommandOptionType::String,
                        "plataforma",
                        "Filtra a plataforma.",
                        false,
                    )],
                )),
        ),
        admin(
            CreateCommand::new("novidades")
                .description("Publica uma novidade.")
                .add_option(opt(CommandOptionType::Channel, "canal", "Canal.", true))
                .add_option(opt(CommandOptionType::String, "titulo", "Título.", true))
                .add_option(opt(
                    CommandOptionType::String,
                    "descricao",
                    "Descrição.",
                    true,
                )),
        ),
        admin(
            CreateCommand::new("regras")
                .description("Gerencia as regras do servidor.")
                .add_option(sub(
                    "enviar",
                    "Publica as regras.",
                    vec![opt(CommandOptionType::Channel, "canal", "Canal.", true)],
                ))
                .add_option(sub(
                    "adicionar",
                    "Adiciona uma regra.",
                    vec![
                        opt(CommandOptionType::String, "titulo", "Título.", true),
                        opt(CommandOptionType::String, "texto", "Texto.", true),
                    ],
                ))
                .add_option(sub(
                    "editar",
                    "Edita uma regra.",
                    vec![
                        opt(CommandOptionType::Integer, "id", "ID.", true),
                        opt(CommandOptionType::String, "titulo", "Novo título.", false),
                        opt(CommandOptionType::String, "texto", "Novo texto.", false),
                    ],
                ))
                .add_option(sub(
                    "remover",
                    "Remove regra(s).",
                    vec![opt(CommandOptionType::String, "id", "ID ou all.", true)],
                ))
                .add_option(sub(
                    "config",
                    "Configura o embed.",
                    vec![
                        opt(CommandOptionType::String, "titulo", "Título.", false),
                        opt(CommandOptionType::String, "descricao", "Descrição.", false),
                        opt(CommandOptionType::String, "cor", "Cor hexadecimal.", false),
                    ],
                ))
                .add_option(sub("listar", "Lista as regras.", vec![])),
        ),
        CreateCommand::new("afk")
            .description("Define ou remove seu status de ausência.")
            .add_option(opt(
                CommandOptionType::String,
                "mensagem",
                "Mensagem; vazio remove.",
                false,
            )),
        CreateCommand::new("fun")
            .description("Comandos sociais.")
            .add_option(sub(
                "8ball",
                "Responde uma pergunta.",
                vec![opt(
                    CommandOptionType::String,
                    "pergunta",
                    "Pergunta.",
                    true,
                )],
            ))
            .add_option(sub(
                "ship",
                "Calcula compatibilidade.",
                vec![opt(CommandOptionType::User, "usuario", "Pessoa.", true)],
            ))
            .add_option(sub(
                "social",
                "Interage com alguém.",
                vec![
                    opt(
                        CommandOptionType::String,
                        "acao",
                        "hug, kiss, pat ou cuddle.",
                        true,
                    ),
                    opt(CommandOptionType::User, "usuario", "Pessoa.", true),
                ],
            )),
        CreateCommand::new("poll")
            .description("Cria uma enquete.")
            .add_option(opt(
                CommandOptionType::String,
                "pergunta",
                "Pergunta.",
                true,
            ))
            .add_option(opt(
                CommandOptionType::String,
                "opcao1",
                "Primeira opção.",
                true,
            ))
            .add_option(opt(
                CommandOptionType::String,
                "opcao2",
                "Segunda opção.",
                true,
            ))
            .add_option(opt(
                CommandOptionType::String,
                "opcao3",
                "Terceira opção.",
                false,
            )),
        CreateCommand::new("remind")
            .description("Cria um lembrete persistente.")
            .add_option(opt(
                CommandOptionType::String,
                "duracao",
                "Ex.: 10m, 2h, 1d.",
                true,
            ))
            .add_option(opt(
                CommandOptionType::String,
                "mensagem",
                "Texto do lembrete.",
                true,
            )),
        CreateCommand::new("coinflip")
            .description("Aposta em cara ou coroa.")
            .add_option(opt(
                CommandOptionType::String,
                "escolha",
                "cara ou coroa.",
                true,
            ))
            .add_option(opt(
                CommandOptionType::Integer,
                "aposta",
                "Valor da aposta.",
                true,
            )),
        CreateCommand::new("blackjack")
            .description("Joga blackjack com moedas.")
            .add_option(opt(
                CommandOptionType::Integer,
                "aposta",
                "Valor da aposta.",
                true,
            )),
        CreateCommand::new("balance")
            .description("Consulta o saldo.")
            .add_option(opt(CommandOptionType::User, "usuario", "Usuário.", false)),
        CreateCommand::new("daily").description("Resgata sua recompensa diária."),
        CreateCommand::new("work").description("Trabalha para ganhar moedas."),
        CreateCommand::new("pay")
            .description("Transfere moedas.")
            .add_option(opt(
                CommandOptionType::User,
                "usuario",
                "Destinatário.",
                true,
            ))
            .add_option(opt(
                CommandOptionType::Integer,
                "valor",
                "Quantidade.",
                true,
            )),
        CreateCommand::new("shop")
            .description("Consulta ou compra itens.")
            .add_option(sub("list", "Lista itens.", vec![]))
            .add_option(sub(
                "buy",
                "Compra item.",
                vec![opt(CommandOptionType::String, "item", "ID do item.", true)],
            )),
        CreateCommand::new("inventory")
            .description("Consulta o inventário.")
            .add_option(opt(CommandOptionType::User, "usuario", "Usuário.", false)),
        CreateCommand::new("transactions").description("Consulta seu extrato."),
        admin(
            CreateCommand::new("addsaldo")
                .description("Adiciona saldo à carteira de um usuário.")
                .add_option(opt(
                    CommandOptionType::User,
                    "usuario",
                    "Usuário que receberá o saldo.",
                    true,
                ))
                .add_option(opt(
                    CommandOptionType::Integer,
                    "valor",
                    "Valor positivo em moedas.",
                    true,
                ))
                .add_option(opt(
                    CommandOptionType::String,
                    "motivo",
                    "Motivo da adição.",
                    false,
                )),
        ),
        CreateCommand::new("level")
            .description("Consulta seu nível.")
            .add_option(opt(CommandOptionType::User, "usuario", "Usuário.", false)),
        CreateCommand::new("leaderboard")
            .description("Exibe o ranking de níveis.")
            .add_option(opt(
                CommandOptionType::String,
                "periodo",
                "total, weekly ou monthly.",
                false,
            )),
        CreateCommand::new("ranking")
            .description("Exibe o ranking visual de níveis.")
            .add_option(opt(
                CommandOptionType::String,
                "periodo",
                "total, weekly ou monthly.",
                false,
            )),
        CreateCommand::new("profile")
            .description("Gera seu perfil visual com XP, nível e economia.")
            .add_option(opt(
                CommandOptionType::User,
                "usuario",
                "Usuário consultado.",
                false,
            )),
        CreateCommand::new("profile-config")
            .description("Configura o tema do seu perfil visual.")
            .add_option(profile_theme_option),
        admin(
            CreateCommand::new("level-config")
                .description("Configura níveis.")
                .add_option(opt(
                    CommandOptionType::Boolean,
                    "ativado",
                    "Ativa XP.",
                    true,
                ))
                .add_option(opt(
                    CommandOptionType::Channel,
                    "canal_anuncio",
                    "Canal de anúncios.",
                    false,
                ))
                .add_option(opt(
                    CommandOptionType::Integer,
                    "cooldown",
                    "Cooldown em segundos.",
                    false,
                )),
        ),
        admin(
            CreateCommand::new("warn")
                .description("Adverte um membro.")
                .add_option(opt(CommandOptionType::User, "usuario", "Membro.", true))
                .add_option(opt(CommandOptionType::String, "motivo", "Motivo.", true)),
        ),
        admin(
            CreateCommand::new("warnings")
                .description("Lista advertências.")
                .add_option(opt(CommandOptionType::User, "usuario", "Membro.", true)),
        ),
        admin(
            CreateCommand::new("timeout")
                .description("Aplica timeout.")
                .add_option(opt(CommandOptionType::User, "usuario", "Membro.", true))
                .add_option(opt(CommandOptionType::String, "duracao", "Ex.: 10m.", true))
                .add_option(opt(CommandOptionType::String, "motivo", "Motivo.", false)),
        ),
        admin(
            CreateCommand::new("untimeout")
                .description("Remove timeout.")
                .add_option(opt(CommandOptionType::User, "usuario", "Membro.", true))
                .add_option(opt(CommandOptionType::String, "motivo", "Motivo.", false)),
        ),
        admin(
            CreateCommand::new("kick")
                .description("Expulsa um membro.")
                .add_option(opt(CommandOptionType::User, "usuario", "Membro.", true))
                .add_option(opt(CommandOptionType::String, "motivo", "Motivo.", false)),
        ),
        admin(
            CreateCommand::new("ban")
                .description("Bane um membro.")
                .add_option(opt(CommandOptionType::User, "usuario", "Membro.", true))
                .add_option(opt(CommandOptionType::String, "motivo", "Motivo.", false)),
        ),
        admin(
            CreateCommand::new("unban")
                .description("Remove um banimento.")
                .add_option(opt(
                    CommandOptionType::String,
                    "usuario_id",
                    "ID do usuário.",
                    true,
                ))
                .add_option(opt(CommandOptionType::String, "motivo", "Motivo.", false)),
        ),
        admin(
            CreateCommand::new("clear")
                .description("Apaga mensagens.")
                .add_option(opt(
                    CommandOptionType::Integer,
                    "quantidade",
                    "Quantidade de 1 a 100.",
                    true,
                )),
        ),
        admin(
            CreateCommand::new("lock")
                .description("Bloqueia mensagens em um canal.")
                .add_option(opt(CommandOptionType::Channel, "canal", "Canal.", false)),
        ),
        admin(
            CreateCommand::new("unlock")
                .description("Desbloqueia mensagens em um canal.")
                .add_option(opt(CommandOptionType::Channel, "canal", "Canal.", false)),
        ),
        admin(
            CreateCommand::new("automod")
                .description("Configura o automod.")
                .add_option(sub(
                    "config",
                    "Ativa ou desativa.",
                    vec![
                        opt(CommandOptionType::Boolean, "ativado", "Ativado.", true),
                        opt(
                            CommandOptionType::Integer,
                            "spam_limite",
                            "Limite de spam.",
                            false,
                        ),
                        opt(
                            CommandOptionType::Integer,
                            "mencoes_limite",
                            "Limite de menções.",
                            false,
                        ),
                    ],
                ))
                .add_option(sub(
                    "palavra",
                    "Gerencia palavras.",
                    vec![
                        opt(CommandOptionType::String, "acao", "add ou remove.", true),
                        opt(CommandOptionType::String, "valor", "Palavra.", true),
                    ],
                ))
                .add_option(sub(
                    "dominio",
                    "Gerencia domínios.",
                    vec![
                        opt(CommandOptionType::String, "acao", "add ou remove.", true),
                        opt(CommandOptionType::String, "valor", "Domínio.", true),
                    ],
                ))
                .add_option(sub(
                    "whitelist",
                    "Gerencia exceções.",
                    vec![
                        opt(CommandOptionType::String, "tipo", "channel ou role.", true),
                        opt(CommandOptionType::String, "acao", "add ou remove.", true),
                        opt(CommandOptionType::String, "id", "ID.", true),
                    ],
                ))
                .add_option(sub(
                    "log",
                    "Define o canal de logs.",
                    vec![opt(CommandOptionType::Channel, "canal", "Canal.", true)],
                )),
        ),
        admin(
            CreateCommand::new("welcome")
                .description("Configura boas-vindas.")
                .add_option(sub(
                    "config",
                    "Ativa boas-vindas.",
                    vec![
                        opt(CommandOptionType::Boolean, "ativado", "Ativado.", true),
                        opt(CommandOptionType::Channel, "canal", "Canal.", false),
                        opt(CommandOptionType::String, "mensagem", "Mensagem.", false),
                    ],
                ))
                .add_option(sub("disable", "Desativa.", vec![]))
                .add_option(sub("test", "Testa.", vec![]))
                .add_option(sub(
                    "color-role",
                    "Gerencia cargo de cor.",
                    vec![
                        opt(CommandOptionType::String, "acao", "add ou remove.", true),
                        opt(CommandOptionType::Role, "cargo", "Cargo.", true),
                    ],
                ))
                .add_option(sub(
                    "panel",
                    "Publica painel.",
                    vec![opt(CommandOptionType::Channel, "canal", "Canal.", true)],
                )),
        ),
    ];
    c
}

fn admin_command(name: &str) -> bool {
    matches!(
        name,
        "verify"
            | "captcha"
            | "tech"
            | "cores"
            | "ticket"
            | "feed"
            | "novidades"
            | "regras"
            | "level-config"
            | "warn"
            | "warnings"
            | "timeout"
            | "untimeout"
            | "kick"
            | "ban"
            | "unban"
            | "clear"
            | "lock"
            | "unlock"
            | "automod"
            | "welcome"
            | "addsaldo"
    )
}

#[derive(Clone, Copy)]
struct HelpCommand {
    name: &'static str,
    description: &'static str,
    category: &'static str,
    aliases: &'static [&'static str],
    admin_only: bool,
}

const HELP_COMMANDS: &[HelpCommand] = &[
    HelpCommand {
        name: "help",
        description: "Abre a central de ajuda por categoria.",
        category: "Informação",
        aliases: &["ajuda", "h"],
        admin_only: false,
    },
    HelpCommand {
        name: "ping",
        description: "Exibe a latência do bot.",
        category: "Informação",
        aliases: &[],
        admin_only: false,
    },
    HelpCommand {
        name: "avatar",
        description: "Exibe o avatar de um usuário.",
        category: "Informação",
        aliases: &[],
        admin_only: false,
    },
    HelpCommand {
        name: "userinfo",
        description: "Exibe informações de um usuário.",
        category: "Informação",
        aliases: &[],
        admin_only: false,
    },
    HelpCommand {
        name: "serverinfo",
        description: "Exibe informações do servidor.",
        category: "Informação",
        aliases: &[],
        admin_only: false,
    },
    HelpCommand {
        name: "calc",
        description: "Calcula uma expressão com segurança.",
        category: "Utilitários",
        aliases: &[],
        admin_only: false,
    },
    HelpCommand {
        name: "devex",
        description: "Calcula o valor estimado de Robux.",
        category: "Utilitários",
        aliases: &[],
        admin_only: false,
    },
    HelpCommand {
        name: "resume",
        description: "Resume o conteúdo de um site.",
        category: "Utilitários",
        aliases: &[],
        admin_only: false,
    },
    HelpCommand {
        name: "verify",
        description: "Publica o painel de verificação.",
        category: "Utilitários",
        aliases: &[],
        admin_only: true,
    },
    HelpCommand {
        name: "captcha",
        description: "Publica o aviso e ativa a proteção do canal #captcha.",
        category: "Utilitários",
        aliases: &[],
        admin_only: true,
    },
    HelpCommand {
        name: "tech",
        description: "Publica o painel de tecnologias.",
        category: "Utilitários",
        aliases: &[],
        admin_only: true,
    },
    HelpCommand {
        name: "cores",
        description: "Publica o painel de cores.",
        category: "Utilitários",
        aliases: &[],
        admin_only: true,
    },
    HelpCommand {
        name: "ticket",
        description: "Publica o painel de tickets.",
        category: "Utilitários",
        aliases: &[],
        admin_only: true,
    },
    HelpCommand {
        name: "reels",
        description: "Exibe um Reel cadastrado.",
        category: "Utilitários",
        aliases: &[],
        admin_only: false,
    },
    HelpCommand {
        name: "tkk",
        description: "Exibe um TikTok cadastrado.",
        category: "Utilitários",
        aliases: &[],
        admin_only: false,
    },
    HelpCommand {
        name: "feed",
        description: "Administra o feed de vídeos.",
        category: "Utilitários",
        aliases: &[],
        admin_only: true,
    },
    HelpCommand {
        name: "novidades",
        description: "Publica uma novidade.",
        category: "Utilitários",
        aliases: &[],
        admin_only: true,
    },
    HelpCommand {
        name: "regras",
        description: "Gerencia as regras do servidor.",
        category: "Utilitários",
        aliases: &[],
        admin_only: true,
    },
    HelpCommand {
        name: "afk",
        description: "Define ou remove seu status de ausência.",
        category: "Convivência",
        aliases: &[],
        admin_only: false,
    },
    HelpCommand {
        name: "fun",
        description: "Comandos sociais.",
        category: "Convivência",
        aliases: &[],
        admin_only: false,
    },
    HelpCommand {
        name: "poll",
        description: "Cria uma enquete.",
        category: "Convivência",
        aliases: &[],
        admin_only: false,
    },
    HelpCommand {
        name: "remind",
        description: "Cria um lembrete persistente.",
        category: "Convivência",
        aliases: &[],
        admin_only: false,
    },
    HelpCommand {
        name: "coinflip",
        description: "Aposta em cara ou coroa.",
        category: "Jogos",
        aliases: &[],
        admin_only: false,
    },
    HelpCommand {
        name: "blackjack",
        description: "Joga blackjack com moedas.",
        category: "Jogos",
        aliases: &[],
        admin_only: false,
    },
    HelpCommand {
        name: "balance",
        description: "Consulta o saldo de um usuário.",
        category: "Economia",
        aliases: &["bal"],
        admin_only: false,
    },
    HelpCommand {
        name: "daily",
        description: "Resgata sua recompensa diária.",
        category: "Economia",
        aliases: &[],
        admin_only: false,
    },
    HelpCommand {
        name: "work",
        description: "Trabalha para ganhar moedas.",
        category: "Economia",
        aliases: &[],
        admin_only: false,
    },
    HelpCommand {
        name: "pay",
        description: "Transfere moedas.",
        category: "Economia",
        aliases: &[],
        admin_only: false,
    },
    HelpCommand {
        name: "shop",
        description: "Consulta ou compra itens.",
        category: "Economia",
        aliases: &[],
        admin_only: false,
    },
    HelpCommand {
        name: "inventory",
        description: "Consulta o inventário de um usuário.",
        category: "Economia",
        aliases: &[],
        admin_only: false,
    },
    HelpCommand {
        name: "transactions",
        description: "Consulta seu extrato de economia.",
        category: "Economia",
        aliases: &[],
        admin_only: false,
    },
    HelpCommand {
        name: "addsaldo",
        description: "Adiciona saldo à carteira de um usuário.",
        category: "Economia",
        aliases: &[],
        admin_only: true,
    },
    HelpCommand {
        name: "level",
        description: "Consulta seu nível e experiência.",
        category: "Níveis",
        aliases: &["rank"],
        admin_only: false,
    },
    HelpCommand {
        name: "leaderboard",
        description: "Exibe o ranking de níveis.",
        category: "Níveis",
        aliases: &["rankings"],
        admin_only: false,
    },
    HelpCommand {
        name: "ranking",
        description: "Exibe o ranking visual de níveis.",
        category: "Níveis",
        aliases: &["rank-card"],
        admin_only: false,
    },
    HelpCommand {
        name: "profile",
        description: "Gera seu perfil visual com XP, nível e economia.",
        category: "Níveis",
        aliases: &["perfil"],
        admin_only: false,
    },
    HelpCommand {
        name: "profile-config",
        description: "Configura o tema do seu perfil visual.",
        category: "Níveis",
        aliases: &[],
        admin_only: false,
    },
    HelpCommand {
        name: "level-config",
        description: "Configura níveis.",
        category: "Níveis",
        aliases: &[],
        admin_only: true,
    },
    HelpCommand {
        name: "warn",
        description: "Adverte um membro.",
        category: "Moderação",
        aliases: &[],
        admin_only: true,
    },
    HelpCommand {
        name: "warnings",
        description: "Lista advertências.",
        category: "Moderação",
        aliases: &["warns"],
        admin_only: true,
    },
    HelpCommand {
        name: "timeout",
        description: "Aplica timeout.",
        category: "Moderação",
        aliases: &[],
        admin_only: true,
    },
    HelpCommand {
        name: "untimeout",
        description: "Remove timeout.",
        category: "Moderação",
        aliases: &[],
        admin_only: true,
    },
    HelpCommand {
        name: "kick",
        description: "Expulsa um membro.",
        category: "Moderação",
        aliases: &[],
        admin_only: true,
    },
    HelpCommand {
        name: "ban",
        description: "Bane um membro.",
        category: "Moderação",
        aliases: &[],
        admin_only: true,
    },
    HelpCommand {
        name: "unban",
        description: "Remove o banimento de um usuário.",
        category: "Moderação",
        aliases: &[],
        admin_only: true,
    },
    HelpCommand {
        name: "clear",
        description: "Apaga mensagens recentes do canal.",
        category: "Moderação",
        aliases: &["mod", "c"],
        admin_only: true,
    },
    HelpCommand {
        name: "lock",
        description: "Bloqueia mensagens em um canal.",
        category: "Moderação",
        aliases: &[],
        admin_only: true,
    },
    HelpCommand {
        name: "unlock",
        description: "Desbloqueia mensagens em um canal.",
        category: "Moderação",
        aliases: &[],
        admin_only: true,
    },
    HelpCommand {
        name: "automod",
        description: "Configura a moderação automática.",
        category: "Moderação",
        aliases: &[],
        admin_only: true,
    },
    HelpCommand {
        name: "welcome",
        description: "Configura boas-vindas, saídas e painéis de cores.",
        category: "Boas-vindas",
        aliases: &[],
        admin_only: true,
    },
];

fn visible_help_commands(
    can_see_admin_commands: bool,
) -> impl Iterator<Item = &'static HelpCommand> {
    HELP_COMMANDS
        .iter()
        .filter(move |command| can_see_admin_commands || !command.admin_only)
}

fn help_categories(can_see_admin_commands: bool) -> Vec<(&'static str, Vec<&'static HelpCommand>)> {
    let mut grouped: HashMap<&'static str, Vec<&'static HelpCommand>> = HashMap::new();
    for command in visible_help_commands(can_see_admin_commands) {
        grouped.entry(command.category).or_default().push(command);
    }

    let mut categories = grouped.into_iter().collect::<Vec<_>>();
    categories.sort_by(|left, right| left.0.cmp(right.0));
    categories
}

fn find_help_command(query: &str) -> Option<&'static HelpCommand> {
    let query = query.to_lowercase();
    HELP_COMMANDS.iter().find(|command| {
        command.name.eq_ignore_ascii_case(&query)
            || command
                .aliases
                .iter()
                .any(|alias| alias.eq_ignore_ascii_case(&query))
    })
}

fn help_menu(
    owner_id: UserId,
    can_see_admin_commands: bool,
    selected_category: Option<&str>,
) -> Value {
    let options = help_categories(can_see_admin_commands)
        .into_iter()
        .take(25)
        .map(|(category, commands)| {
            json!({
                "label": category,
                "value": category,
                "description": format!("{} comando(s)", commands.len()),
                "default": selected_category == Some(category),
            })
        })
        .collect::<Vec<_>>();

    json!({
        "type": 1,
        "components": [{
            "type": 3,
            "custom_id": format!("help:category:{owner_id}"),
            "placeholder": "Selecione uma categoria",
            "options": options,
        }],
    })
}

fn help_container(
    content: String,
    owner_id: UserId,
    can_see_admin_commands: bool,
    selected_category: Option<&str>,
) -> Value {
    // Serenity 0.12.5 ainda não possui builders para Components V2. Estes são
    // os mesmos componentes enviados pelo discord.js: Container + TextDisplay
    // + ActionRow/StringSelectMenu.
    json!({
        "type": 17,
        "accent_color": PURPLE,
        "components": [
            { "type": 10, "content": content },
            help_menu(owner_id, can_see_admin_commands, selected_category),
        ],
    })
}

fn help_home_panel(owner_id: UserId, can_see_admin_commands: bool, state: &State) -> Value {
    let mut description = vec![
        "# Central de ajuda".to_string(),
        String::new(),
        "Use o menu abaixo para navegar pelos comandos.".to_string(),
        String::new(),
    ];
    description.extend(
        help_categories(can_see_admin_commands)
            .into_iter()
            .map(|(category, commands)| format!("**{category}** — {} comando(s)", commands.len())),
    );

    if state.config.prefix_commands {
        description.push(String::new());
        description.push(format!("Prefixo legado ativo: `{}`", state.config.prefix));
    }

    help_container(
        description.join("\n"),
        owner_id,
        can_see_admin_commands,
        None,
    )
}

fn help_category_panel(owner_id: UserId, can_see_admin_commands: bool, category: &str) -> Value {
    let categories = help_categories(can_see_admin_commands);
    let Some((_, commands)) = categories.iter().find(|(name, _)| *name == category) else {
        return help_container(
            "# Categoria não encontrada\n\nSelecione uma categoria válida no menu abaixo.".into(),
            owner_id,
            can_see_admin_commands,
            None,
        );
    };

    let mut content = vec![format!("# Ajuda • {category}"), String::new()];
    content.extend(
        commands
            .iter()
            .map(|command| format!("`/{}` — {}", command.name, command.description)),
    );
    content.extend([
        String::new(),
        "-# Use `/help comando` para detalhes de um comando.".to_string(),
    ]);

    help_container(
        content.join("\n"),
        owner_id,
        can_see_admin_commands,
        Some(category),
    )
}

fn command_help_panel(query: &str, owner_id: UserId, can_see_admin_commands: bool) -> Value {
    let command =
        find_help_command(query).filter(|command| can_see_admin_commands || !command.admin_only);
    let Some(command) = command else {
        return help_container(
            format!(
                "# Comando não encontrado\n\nNenhum comando disponível foi encontrado para `{query}`.\nUse `/help` para abrir o painel por categoria."
            ),
            owner_id,
            can_see_admin_commands,
            None,
        );
    };

    let aliases = if command.aliases.is_empty() {
        "*Nenhum*".to_string()
    } else {
        command
            .aliases
            .iter()
            .map(|alias| format!("`{alias}`"))
            .collect::<Vec<_>>()
            .join(", ")
    };
    let content = [
        format!("# Detalhes do comando: `/{}`", command.name),
        String::new(),
        format!("**Descrição:** {}", command.description),
        format!("**Categoria:** {}", command.category),
        format!("**Como usar:** `/{}`", command.name),
        format!("**Aliases legados:** {aliases}"),
    ]
    .join("\n");

    help_container(
        content,
        owner_id,
        can_see_admin_commands,
        Some(command.category),
    )
}

async fn send_components_v2_interaction(
    http: &Http,
    interaction_id: InteractionId,
    token: &str,
    response_type: u8,
    panel: Value,
) -> BotResult<()> {
    let payload = json!({
        "type": response_type,
        "data": {
            "flags": COMPONENTS_V2_FLAG,
            "components": [panel],
        },
    });
    let body = serde_json::to_vec(&payload)?;
    http.request(
        Request::new(
            Route::InteractionResponse {
                interaction_id,
                token,
            },
            LightMethod::Post,
        )
        .body(Some(body)),
    )
    .await?;
    Ok(())
}

async fn send_components_v2_channel(
    http: &Http,
    channel_id: ChannelId,
    panel: Value,
    reply_to: Option<MessageId>,
) -> BotResult<()> {
    let mut payload = json!({
        "flags": COMPONENTS_V2_FLAG,
        "components": [panel],
    });
    if let Some(message_id) = reply_to {
        payload["message_reference"] = json!({ "message_id": message_id.get() });
    }

    let body = serde_json::to_vec(&payload)?;
    http.request(
        Request::new(Route::ChannelMessages { channel_id }, LightMethod::Post).body(Some(body)),
    )
    .await?;
    Ok(())
}

struct PanelTechnology {
    key: &'static str,
    name: &'static str,
}

const PANEL_TECHNOLOGIES: &[PanelTechnology] = &[
    PanelTechnology {
        key: "javascript",
        name: "JavaScript",
    },
    PanelTechnology {
        key: "typescript",
        name: "TypeScript",
    },
    PanelTechnology {
        key: "python",
        name: "Python",
    },
    PanelTechnology {
        key: "java",
        name: "Java",
    },
    PanelTechnology {
        key: "c-sharp",
        name: "C#",
    },
    PanelTechnology {
        key: "c-plus-plus",
        name: "C++",
    },
    PanelTechnology {
        key: "c",
        name: "C",
    },
    PanelTechnology {
        key: "go",
        name: "Go",
    },
    PanelTechnology {
        key: "rust",
        name: "Rust",
    },
    PanelTechnology {
        key: "php",
        name: "PHP",
    },
    PanelTechnology {
        key: "ruby",
        name: "Ruby",
    },
    PanelTechnology {
        key: "kotlin",
        name: "Kotlin",
    },
    PanelTechnology {
        key: "swift",
        name: "Swift",
    },
    PanelTechnology {
        key: "html",
        name: "HTML",
    },
    PanelTechnology {
        key: "css",
        name: "CSS",
    },
    PanelTechnology {
        key: "react",
        name: "React",
    },
    PanelTechnology {
        key: "next-js",
        name: "Next.js",
    },
    PanelTechnology {
        key: "node-js",
        name: "Node.js",
    },
    PanelTechnology {
        key: "discord-js",
        name: "discord.js",
    },
    PanelTechnology {
        key: "vue",
        name: "Vue",
    },
    PanelTechnology {
        key: "angular",
        name: "Angular",
    },
    PanelTechnology {
        key: "svelte",
        name: "Svelte",
    },
    PanelTechnology {
        key: "sql",
        name: "SQL",
    },
    PanelTechnology {
        key: "mongodb",
        name: "MongoDB",
    },
    PanelTechnology {
        key: "docker",
        name: "Docker",
    },
];

// IDs sincronizados com cargos.md.
const TECHNOLOGY_ROLE_IDS: &[(&str, u64)] = &[
    ("javascript", 1_548_761_291_789_828_216),
    ("typescript", 1_548_767_613_000_351_834),
    ("python", 1_548_767_726_552_490_055),
    ("java", 1_548_767_767_732_424_806),
    ("c-sharp", 1_548_767_808_605_782_087),
    ("c-plus-plus", 1_548_767_852_855_693_352),
    ("go", 1_548_767_913_572_565_123),
    ("rust", 1_548_767_946_725_483_303),
    ("php", 1_548_767_983_172_853_862),
    ("kotlin", 1_548_768_052_978_520_185),
    ("swift", 1_548_768_090_597_228_698),
    ("html", 1_548_768_135_182_553_138),
    ("css", 1_548_768_178_130_205_160),
    ("react", 1_548_768_218_246_549_515),
    ("next-js", 1_548_768_260_361_818_223),
    ("node-js", 1_548_768_309_795_627_078),
    ("vue", 1_548_768_387_486_978_273),
    ("discord-js", 1_548_768_351_051_063_346),
    ("angular", 1_548_768_425_482_911_814),
    ("svelte", 1_548_768_462_275_350_610),
    ("sql", 1_548_768_499_449_471_196),
    ("docker", 1_548_768_531_095_485_665),
    ("mongodb", 1_548_768_571_922_976_848),
    ("c", 1_548_767_881_624_424_590),
];

fn technology_role_id(key: &str) -> Option<RoleId> {
    TECHNOLOGY_ROLE_IDS
        .iter()
        .find(|(technology, _)| *technology == key)
        .map(|(_, role_id)| RoleId::new(*role_id))
}

fn panel_container(content: &str, children: Vec<Value>) -> Value {
    let mut components = vec![json!({ "type": 10, "content": content })];
    components.extend(children);
    json!({
        "type": 17,
        "accent_color": PURPLE,
        "components": components,
    })
}

fn verify_panel(role_id: RoleId) -> Value {
    panel_container(
        "# Verificação\n\nClique no botão abaixo para receber o cargo **Membro** e liberar o acesso ao servidor.",
        vec![json!({
            "type": 1,
            "components": [{
                "type": 2,
                "custom_id": format!("panel:verify:{role_id}"),
                "label": "Receber cargo Membro",
                "style": 3,
            }],
        })],
    )
}

fn colors_panel(roles: Vec<&Role>) -> Value {
    let options = roles
        .into_iter()
        .map(|role| json!({ "label": role.name, "value": role.id.to_string() }))
        .collect::<Vec<_>>();
    panel_container(
        "# Cores\n\nEscolha uma cor para receber o cargo correspondente. Selecionar outra cor remove a anterior.",
        vec![json!({
            "type": 1,
            "components": [{
                "type": 3,
                "custom_id": "panel:colors",
                "placeholder": "Escolha uma cor",
                "min_values": 0,
                "max_values": 1,
                "options": options,
            }],
        })],
    )
}

fn technology_emoji_names(key: &str) -> &'static [&'static str] {
    match key {
        "javascript" => &["javascript"],
        "typescript" => &["typescript", "ts"],
        "python" => &["python"],
        "java" => &["java"],
        "c-sharp" => &["csharp", "c-sharp"],
        "c-plus-plus" => &["cpp", "c-plus-plus", "clang"],
        "c" => &["clang", "c"],
        "go" => &["golang", "go"],
        "rust" => &["rust"],
        "php" => &["php"],
        "ruby" => &["ruby"],
        "kotlin" => &["kotlin"],
        "swift" => &["swift"],
        "html" => &["html"],
        "css" => &["css"],
        "react" => &["react"],
        "next-js" => &["nextjs", "next-js"],
        "node-js" => &["nodejs", "node-js"],
        "discord-js" => &["djs", "discordjs", "discord-js"],
        "vue" => &["vuejs", "vue"],
        "angular" => &["angular"],
        "svelte" => &["svelte"],
        "sql" => &["sql"],
        "mongodb" => &["mongodb"],
        "docker" => &["docker"],
        _ => &[],
    }
}

fn technology_emoji<'a>(key: &str, emojis: &'a [Emoji]) -> Option<&'a Emoji> {
    technology_emoji_names(key).iter().find_map(|name| {
        emojis
            .iter()
            .find(|emoji| emoji.available && emoji.name.eq_ignore_ascii_case(name))
    })
}

fn technology_panel(emojis: &[Emoji]) -> Value {
    let options = PANEL_TECHNOLOGIES
        .iter()
        .filter_map(|technology| {
            let role_id = technology_role_id(technology.key)?;
            let mut option = json!({
                "label": technology.name,
                "value": role_id.to_string(),
            });
            if let Some(emoji) = technology_emoji(technology.key, emojis) {
                option["emoji"] = json!({
                    "id": emoji.id.to_string(),
                    "name": emoji.name,
                    "animated": emoji.animated,
                });
            }
            Some(option)
        })
        .collect::<Vec<_>>();
    panel_container(
        "# Tecnologias\n\nSelecione uma ou mais tecnologias para receber os cargos correspondentes.",
        vec![json!({
            "type": 1,
            "components": [{
                "type": 3,
                "custom_id": "panel:tech",
                "placeholder": "Escolha suas tecnologias",
                "min_values": 0,
                "max_values": options.len(),
                "options": options,
            }],
        })],
    )
}

fn panel_technology(key: &str) -> Option<&'static PanelTechnology> {
    PANEL_TECHNOLOGIES.iter().find(|technology| {
        technology.key.eq_ignore_ascii_case(key) || technology.name.eq_ignore_ascii_case(key)
    })
}

fn find_option<'a>(
    options: &'a [CommandDataOption],
    name: &str,
) -> Option<&'a CommandDataOptionValue> {
    for option in options {
        if option.name == name {
            return Some(&option.value);
        }
        match &option.value {
            CommandDataOptionValue::SubCommand(inner)
            | CommandDataOptionValue::SubCommandGroup(inner) => {
                if let Some(found) = find_option(inner, name) {
                    return Some(found);
                }
            }
            _ => {}
        }
    }
    None
}

fn option_string(command: &CommandInteraction, name: &str) -> Option<String> {
    match find_option(&command.data.options, name) {
        Some(CommandDataOptionValue::String(value))
        | Some(CommandDataOptionValue::Autocomplete { value, .. }) => Some(value.clone()),
        _ => None,
    }
}

fn option_integer(command: &CommandInteraction, name: &str) -> Option<i64> {
    match find_option(&command.data.options, name) {
        Some(CommandDataOptionValue::Integer(value)) => Some(*value),
        _ => None,
    }
}

fn option_boolean(command: &CommandInteraction, name: &str) -> Option<bool> {
    match find_option(&command.data.options, name) {
        Some(CommandDataOptionValue::Boolean(value)) => Some(*value),
        _ => None,
    }
}

fn option_user(command: &CommandInteraction, name: &str) -> Option<UserId> {
    match find_option(&command.data.options, name) {
        Some(CommandDataOptionValue::User(value)) => Some(*value),
        _ => None,
    }
}

fn option_channel(command: &CommandInteraction, name: &str) -> Option<ChannelId> {
    match find_option(&command.data.options, name) {
        Some(CommandDataOptionValue::Channel(value)) => Some(*value),
        _ => None,
    }
}

fn option_channel_or_current(command: &CommandInteraction, name: &str) -> ChannelId {
    option_channel(command, name).unwrap_or(command.channel_id)
}

fn command_guild(command: &CommandInteraction) -> BotResult<GuildId> {
    command
        .guild_id
        .ok_or_else(|| "Este comando só pode ser usado em um servidor.".into())
}

fn is_admin(command: &CommandInteraction) -> bool {
    command
        .member
        .as_ref()
        .and_then(|member| member.permissions)
        .map(|p| p.contains(Permissions::ADMINISTRATOR))
        .unwrap_or(false)
}

async fn reply(
    command: &CommandInteraction,
    http: &Http,
    content: impl Into<String>,
    ephemeral: bool,
) -> BotResult<()> {
    command
        .create_response(
            http,
            CreateInteractionResponse::Message(
                CreateInteractionResponseMessage::new()
                    .content(content)
                    .ephemeral(ephemeral),
            ),
        )
        .await?;
    Ok(())
}

async fn embed_reply(
    command: &CommandInteraction,
    http: &Http,
    embed: CreateEmbed,
    ephemeral: bool,
) -> BotResult<()> {
    command
        .create_response(
            http,
            CreateInteractionResponse::Message(
                CreateInteractionResponseMessage::new()
                    .embed(embed)
                    .ephemeral(ephemeral),
            ),
        )
        .await?;
    Ok(())
}

async fn handle_command(
    ctx: &Context,
    state: &Arc<State>,
    command: &CommandInteraction,
) -> BotResult<()> {
    if admin_command(&command.data.name) && !is_admin(command) {
        return reply(
            command,
            &ctx.http,
            "Apenas administradores podem usar este comando.",
            true,
        )
        .await;
    }
    let name = command.data.name.as_str();
    match name {
        "help" => command_help_components_v2(ctx, state, command).await?,
        "ping" => reply(command, &ctx.http, "Pong! 🏓", false).await?,
        "avatar" => command_avatar(ctx, command).await?,
        "userinfo" => command_userinfo(ctx, command).await?,
        "serverinfo" => command_serverinfo(ctx, command).await?,
        "calc" => command_calc(&ctx.http, command).await?,
        "devex" => command_devex(state, &ctx.http, command).await?,
        "resume" => command_resume(state, &ctx.http, command).await?,
        "verify" => command_verify(ctx, command).await?,
        "captcha" => command_captcha(ctx, command).await?,
        "tech" => command_tech(ctx, command).await?,
        "cores" => command_cores(ctx, command).await?,
        "ticket" => command_ticket(ctx, command).await?,
        "feed" => command_feed(ctx, state, command).await?,
        "reels" | "tkk" => command_reel(ctx, state, command, name).await?,
        "novidades" => command_news(ctx, command).await?,
        "regras" => command_rules(ctx, state, command).await?,
        "afk" => command_afk(state, &ctx.http, command).await?,
        "fun" => command_fun(&ctx.http, command).await?,
        "poll" => command_poll(ctx, command).await?,
        "remind" => command_remind(state, &ctx.http, command).await?,
        "coinflip" => command_coinflip(state, &ctx.http, command).await?,
        "blackjack" => command_blackjack(state, &ctx.http, command).await?,
        "addsaldo" => command_add_balance(state, &ctx.http, command).await?,
        "balance" | "daily" | "work" | "pay" | "shop" | "inventory" | "transactions" => {
            command_economy(state, &ctx.http, command, name).await?
        }
        "level" | "leaderboard" | "ranking" | "profile" => {
            command_levels(state, &ctx.http, command, name).await?
        }
        "profile-config" => command_profile_config(state, &ctx.http, command).await?,
        "level-config" => command_level_config(state, &ctx.http, command).await?,
        "warn" | "warnings" | "timeout" | "untimeout" | "kick" | "ban" | "unban" | "clear"
        | "lock" | "unlock" => command_moderation(ctx, state, command, name).await?,
        "automod" => command_automod(state, &ctx.http, command).await?,
        "welcome" => command_welcome(ctx, state, command).await?,
        _ => {
            reply(
                command,
                &ctx.http,
                "Esse comando não está disponível.",
                true,
            )
            .await?
        }
    }
    Ok(())
}

async fn command_help_components_v2(
    ctx: &Context,
    state: &Arc<State>,
    command: &CommandInteraction,
) -> BotResult<()> {
    let query = option_string(command, "comando")
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty());
    let can_see_admin_commands = is_admin(command);
    let panel = query
        .as_deref()
        .map(|value| command_help_panel(value, command.user.id, can_see_admin_commands))
        .unwrap_or_else(|| help_home_panel(command.user.id, can_see_admin_commands, state));

    send_components_v2_interaction(&ctx.http, command.id, &command.token, 4, panel).await
}

async fn command_avatar(ctx: &Context, command: &CommandInteraction) -> BotResult<()> {
    let user_id = option_user(command, "usuario").unwrap_or(command.user.id);
    let user = ctx.http.get_user(user_id).await?;
    embed_reply(
        command,
        &ctx.http,
        CreateEmbed::new()
            .title(format!("Avatar de {}", user.name))
            .image(
                user.avatar_url()
                    .unwrap_or_else(|| user.default_avatar_url()),
            )
            .color(PURPLE),
        false,
    )
    .await
}

async fn command_userinfo(ctx: &Context, command: &CommandInteraction) -> BotResult<()> {
    let user_id = option_user(command, "usuario").unwrap_or(command.user.id);
    let user = ctx.http.get_user(user_id).await?;
    let created = user.id.created_at().unix_timestamp();
    let embed = CreateEmbed::new()
        .title(&user.name)
        .description(format!("ID: `{}`\nConta criada: <t:{created}:R>", user.id))
        .thumbnail(
            user.avatar_url()
                .unwrap_or_else(|| user.default_avatar_url()),
        )
        .color(BLUE);
    embed_reply(command, &ctx.http, embed, false).await
}

async fn command_serverinfo(ctx: &Context, command: &CommandInteraction) -> BotResult<()> {
    let guild_id = command_guild(command)?;
    let guild = guild_id.to_partial_guild(&ctx.http).await?;
    embed_reply(
        command,
        &ctx.http,
        CreateEmbed::new()
            .title(guild.name)
            .description(format!(
                "ID: `{}`\nMembros: `{}`",
                guild.id,
                guild.approximate_member_count.unwrap_or(0)
            ))
            .color(BLUE),
        false,
    )
    .await
}

async fn command_calc(http: &Http, command: &CommandInteraction) -> BotResult<()> {
    let expression = option_string(command, "expressao").unwrap_or_default();
    match calculate(&expression) {
        Ok(value) => {
            embed_reply(
                command,
                http,
                CreateEmbed::new()
                    .title("Calculadora")
                    .field(
                        "Expressão",
                        format!("`{}`", truncate(&expression, 900)),
                        true,
                    )
                    .field("Resultado", format!("**{}**", format_number(value)), false)
                    .color(PURPLE),
                false,
            )
            .await
        }
        Err(message) => {
            reply(
                command,
                http,
                format!("Expressão inválida: {message}"),
                true,
            )
            .await
        }
    }
}

async fn command_devex(
    state: &Arc<State>,
    http: &Http,
    command: &CommandInteraction,
) -> BotResult<()> {
    let robux = option_integer(command, "robux").unwrap_or(0).max(0) as f64;
    let currency = option_string(command, "moeda")
        .unwrap_or_else(|| "USD".into())
        .to_uppercase();
    let usd = robux * 0.0038;
    let rate = if currency == "USD" {
        1.0
    } else {
        let url = format!("https://api.frankfurter.app/latest?from=USD&to={currency}");
        let value: serde_json::Value = state.http.get(url).send().await?.json().await?;
        value
            .get("rates")
            .and_then(|v| v.get(&currency))
            .and_then(serde_json::Value::as_f64)
            .ok_or("moeda não encontrada")?
    };
    embed_reply(
        command,
        http,
        CreateEmbed::new()
            .title("Calculadora DevEx")
            .field("Valor em USD", format_money(usd, "USD"), true)
            .field(
                format!("Valor em {currency}"),
                format_money(usd * rate, &currency),
                true,
            )
            .field("Taxa", "US$ 0,0038 por Robux", true)
            .color(PURPLE),
        false,
    )
    .await
}

async fn command_resume(
    state: &Arc<State>,
    http: &Http,
    command: &CommandInteraction,
) -> BotResult<()> {
    let url = option_string(command, "url").unwrap_or_default();
    if !(url.starts_with("http://") || url.starts_with("https://")) {
        return reply(command, http, "Informe uma URL http(s) válida.", true).await;
    }
    let allowed = {
        let mut limits = state.resume.lock().expect("resume mutex poisoned");
        let entry = limits.entry(command.user.id).or_insert((Instant::now(), 0));
        if entry.0.elapsed() >= Duration::from_secs(60) {
            *entry = (Instant::now(), 0);
        }
        if entry.1 >= 3 {
            false
        } else {
            entry.1 += 1;
            true
        }
    };
    if !allowed {
        return reply(
            command,
            http,
            "Limite de 3 resumos por minuto atingido.",
            true,
        )
        .await;
    }
    command.defer(http).await?;
    let jina_url = format!("https://r.jina.ai/{url}");
    let text = state.http.get(jina_url).send().await?.text().await?;
    command
        .edit_response(
            http,
            EditInteractionResponse::new().embed(
                CreateEmbed::new()
                    .title("Resumo")
                    .description(truncate(&text, 3900))
                    .field("Fonte", format!("[Abrir site]({url})"), false)
                    .color(PURPLE),
            ),
        )
        .await?;
    Ok(())
}

async fn command_verify(ctx: &Context, command: &CommandInteraction) -> BotResult<()> {
    let guild_id = command_guild(command)?;
    let roles = guild_id.roles(&ctx.http).await?;
    let Some(member_role) = roles.get(&RoleId::new(VERIFIED_ROLE_ID)) else {
        return reply(
            command,
            &ctx.http,
            format!(
                "O cargo de verificado (`{VERIFIED_ROLE_ID}`) não foi encontrado neste servidor."
            ),
            true,
        )
        .await;
    };
    let channel = option_channel_or_current(command, "canal");
    send_components_v2_channel(&ctx.http, channel, verify_panel(member_role.id), None).await?;
    reply(command, &ctx.http, "Painel publicado.", true).await
}

async fn command_captcha(ctx: &Context, command: &CommandInteraction) -> BotResult<()> {
    command_guild(command)?;
    let channel = ChannelId::new(CAPTCHA_CHANNEL_ID);

    channel
        .send_message(
            &ctx.http,
            CreateMessage::new().embed(CreateEmbed::new().description(CAPTCHA_WARNING)),
        )
        .await?;
    reply(
        command,
        &ctx.http,
        "Aviso do canal #captcha publicado.",
        true,
    )
    .await
}

async fn command_tech(ctx: &Context, command: &CommandInteraction) -> BotResult<()> {
    let guild_id = command_guild(command)?;
    let emojis = guild_id.emojis(&ctx.http).await?;
    let channel = option_channel_or_current(command, "canal");
    send_components_v2_channel(&ctx.http, channel, technology_panel(&emojis), None).await?;
    reply(command, &ctx.http, "Painel publicado.", true).await
}

async fn command_cores(ctx: &Context, command: &CommandInteraction) -> BotResult<()> {
    let guild_id = command_guild(command)?;
    let roles = guild_id.roles(&ctx.http).await?;
    let missing = COLOR_ROLE_NAMES
        .iter()
        .filter(|name| {
            !roles
                .values()
                .any(|role| role.name.eq_ignore_ascii_case(name))
        })
        .copied()
        .collect::<Vec<_>>();
    if !missing.is_empty() {
        return reply(
            command,
            &ctx.http,
            format!(
                "Crie estes cargos antes de publicar o painel: {}.",
                missing
                    .iter()
                    .map(|name| format!("`{name}`"))
                    .collect::<Vec<_>>()
                    .join(", ")
            ),
            true,
        )
        .await;
    }

    let color_roles = COLOR_ROLE_NAMES
        .iter()
        .filter_map(|name| {
            roles
                .values()
                .find(|role| role.name.eq_ignore_ascii_case(name))
        })
        .collect::<Vec<_>>();
    let channel = option_channel_or_current(command, "canal");
    send_components_v2_channel(&ctx.http, channel, colors_panel(color_roles), None).await?;
    reply(command, &ctx.http, "Painel publicado.", true).await
}

async fn command_ticket(ctx: &Context, command: &CommandInteraction) -> BotResult<()> {
    let channel = option_channel_or_current(command, "canal");
    let options = vec![
        CreateSelectMenuOption::new("Reportar erro", "error"),
        CreateSelectMenuOption::new("Parcerias", "partnership"),
        CreateSelectMenuOption::new("Denunciar membro", "member_report"),
        CreateSelectMenuOption::new("Denunciar staff", "staff_report"),
    ];
    let select = CreateSelectMenu::new("ticket:open", CreateSelectMenuKind::String { options })
        .placeholder("Escolha o tipo de atendimento");
    channel
        .send_message(
            &ctx.http,
            CreateMessage::new()
                .embed(
                    CreateEmbed::new()
                        .title("Suporte")
                        .description("Abra um ticket privado escolhendo uma categoria.")
                        .color(PURPLE),
                )
                .components(vec![CreateActionRow::SelectMenu(select)]),
        )
        .await?;
    reply(command, &ctx.http, "Painel de tickets publicado.", true).await
}

async fn command_news(ctx: &Context, command: &CommandInteraction) -> BotResult<()> {
    let channel = option_channel_or_current(command, "canal");
    let title = option_string(command, "titulo").unwrap_or_else(|| "Novidades".into());
    let description = option_string(command, "descricao").unwrap_or_default();
    channel
        .send_message(
            &ctx.http,
            CreateMessage::new().embed(
                CreateEmbed::new()
                    .title(title)
                    .description(description)
                    .color(PURPLE),
            ),
        )
        .await?;
    reply(command, &ctx.http, "Novidade publicada.", true).await
}

async fn command_rules(
    ctx: &Context,
    state: &Arc<State>,
    command: &CommandInteraction,
) -> BotResult<()> {
    let guild = command_guild(command)?.to_string();
    let subcommand = top_subcommand(command).unwrap_or_default();
    let mut settings = state.db().settings(&guild)?;
    match subcommand.as_str() {
        "adicionar" => {
            let id = settings.rules.iter().map(|rule| rule.id).max().unwrap_or(0) + 1;
            settings.rules.push(RuleItem {
                id,
                title: option_string(command, "titulo").unwrap_or_default(),
                description: option_string(command, "texto").unwrap_or_default(),
            });
            state.db().save_settings(&guild, &settings)?;
            reply(
                command,
                &ctx.http,
                format!("Regra **{id}** adicionada."),
                true,
            )
            .await
        }
        "editar" => {
            let id = option_integer(command, "id").unwrap_or(0);
            if let Some(rule) = settings.rules.iter_mut().find(|rule| rule.id == id) {
                if let Some(title) = option_string(command, "titulo") {
                    rule.title = title;
                }
                if let Some(text) = option_string(command, "texto") {
                    rule.description = text;
                }
                state.db().save_settings(&guild, &settings)?;
                reply(command, &ctx.http, "Regra atualizada.", true).await
            } else {
                reply(command, &ctx.http, "Regra não encontrada.", true).await
            }
        }
        "remover" => {
            let id = option_string(command, "id").unwrap_or_default();
            if id.eq_ignore_ascii_case("all") {
                settings.rules.clear();
            } else if let Ok(id) = id.parse::<i64>() {
                settings.rules.retain(|rule| rule.id != id);
            }
            state.db().save_settings(&guild, &settings)?;
            reply(command, &ctx.http, "Regra(s) removida(s).", true).await
        }
        "config" => {
            if let Some(title) = option_string(command, "titulo") {
                settings.rules_title = title;
            }
            if let Some(description) = option_string(command, "descricao") {
                settings.rules_description = description;
            }
            if let Some(color) = option_string(command, "cor") {
                settings.rules_color = color;
            }
            state.db().save_settings(&guild, &settings)?;
            reply(command, &ctx.http, "Configuração atualizada.", true).await
        }
        "listar" => {
            let description = if settings.rules.is_empty() {
                "Nenhuma regra cadastrada.".into()
            } else {
                settings
                    .rules
                    .iter()
                    .map(|rule| format!("**{} — {}**\n{}", rule.id, rule.title, rule.description))
                    .collect::<Vec<_>>()
                    .join("\n\n")
            };
            embed_reply(
                command,
                &ctx.http,
                CreateEmbed::new()
                    .title(&settings.rules_title)
                    .description(description)
                    .color(parse_color(&settings.rules_color)),
                true,
            )
            .await
        }
        "enviar" => {
            let channel = option_channel_or_current(command, "canal");
            let description = rules_text(&settings);
            let message = channel
                .send_message(
                    &ctx.http,
                    CreateMessage::new().embed(
                        CreateEmbed::new()
                            .title(&settings.rules_title)
                            .description(description)
                            .color(parse_color(&settings.rules_color)),
                    ),
                )
                .await?;
            settings.rules_channel = Some(channel.to_string());
            settings.rules_message = Some(message.id.to_string());
            state.db().save_settings(&guild, &settings)?;
            reply(command, &ctx.http, "Regras publicadas.", true).await
        }
        _ => reply(command, &ctx.http, "Subcomando de regras inválido.", true).await,
    }
}

fn rules_text(settings: &GuildSettings) -> String {
    if settings.rules.is_empty() {
        return settings.rules_description.clone();
    }
    format!(
        "{}\n\n{}",
        settings.rules_description,
        settings
            .rules
            .iter()
            .map(|rule| format!("**{} — {}**\n{}", rule.id, rule.title, rule.description))
            .collect::<Vec<_>>()
            .join("\n\n")
    )
}

async fn command_feed(
    ctx: &Context,
    state: &Arc<State>,
    command: &CommandInteraction,
) -> BotResult<()> {
    let guild = command_guild(command)?.to_string();
    match top_subcommand(command).as_deref() {
        Some("adicionar") => {
            let platform =
                option_string(command, "plataforma").unwrap_or_else(|| "instagram".into());
            let url = option_string(command, "url").unwrap_or_default();
            let title = option_string(command, "titulo").unwrap_or_default();
            state.db().conn.execute("INSERT OR IGNORE INTO reels(guild_id,platform,url,title,added_by) VALUES(?1,?2,?3,?4,?5)", params![guild, platform, url, title, command.user.id.to_string()])?;
            reply(command, &ctx.http, "Vídeo adicionado ao feed.", true).await
        }
        Some("remover") => {
            let url = option_string(command, "url").unwrap_or_default();
            state.db().conn.execute(
                "DELETE FROM reels WHERE guild_id=?1 AND url=?2",
                params![guild, url],
            )?;
            reply(command, &ctx.http, "Vídeo removido do feed.", true).await
        }
        Some("listar") => {
            let platform = option_string(command, "plataforma");
            let text = {
                let sql = if platform.is_some() {
                    "SELECT platform,url,title FROM reels WHERE guild_id=?1 AND platform=?2 ORDER BY id DESC LIMIT 25"
                } else {
                    "SELECT platform,url,title FROM reels WHERE guild_id=?1 ORDER BY id DESC LIMIT 25"
                };
                let conn = state.db();
                let mut stmt = conn.conn.prepare(sql)?;
                let mut rows = if let Some(platform) = platform {
                    stmt.query(params![guild, platform])?
                } else {
                    stmt.query(params![guild])?
                };
                let mut lines = Vec::new();
                while let Some(row) = rows.next()? {
                    let p: String = row.get(0)?;
                    let url: String = row.get(1)?;
                    let title: String = row.get(2)?;
                    lines.push(format!(
                        "**{p}** {} — {url}",
                        if title.is_empty() { "" } else { &title }
                    ));
                }
                if lines.is_empty() {
                    "Nenhum vídeo cadastrado.".into()
                } else {
                    lines.join("\n")
                }
            };
            reply(command, &ctx.http, text, true).await
        }
        _ => {
            reply(
                command,
                &ctx.http,
                "Use adicionar, remover ou listar.",
                true,
            )
            .await
        }
    }
}

async fn command_reel(
    ctx: &Context,
    state: &Arc<State>,
    command: &CommandInteraction,
    name: &str,
) -> BotResult<()> {
    let platform = if name == "reels" {
        "instagram"
    } else {
        "tiktok"
    };
    let guild = command_guild(command)?.to_string();
    let row: Option<(i64, String, String)> = {
        let conn = state.db();
        let mut stmt = conn.conn.prepare("SELECT id,url,title FROM reels WHERE guild_id=?1 AND platform=?2 ORDER BY RANDOM() LIMIT 1")?;
        stmt.query_row(params![guild, platform], |r| {
            Ok((r.get(0)?, r.get(1)?, r.get(2)?))
        })
        .optional()?
    };
    let Some((id, url, title)) = row else {
        return reply(
            command,
            &ctx.http,
            "Nenhum vídeo cadastrado para esta plataforma.",
            true,
        )
        .await;
    };
    let label = if title.is_empty() {
        platform.to_string()
    } else {
        title
    };
    command
        .create_response(
            &ctx.http,
            CreateInteractionResponse::Message(
                CreateInteractionResponseMessage::new()
                    .embed(
                        CreateEmbed::new()
                            .title(label)
                            .description(format!("[Abrir vídeo]({url})"))
                            .color(PURPLE),
                    )
                    .components(vec![CreateActionRow::Buttons(vec![
                        CreateButton::new(format!("reel:like:{id}"))
                            .label("Curtir")
                            .style(ButtonStyle::Secondary),
                        CreateButton::new(format!("reel:repost:{id}"))
                            .label("Repostar")
                            .style(ButtonStyle::Secondary),
                        CreateButton::new(format!("reel:share:{id}"))
                            .label("Compartilhar")
                            .style(ButtonStyle::Secondary),
                    ])]),
            ),
        )
        .await?;
    Ok(())
}

async fn command_afk(
    state: &Arc<State>,
    http: &Http,
    command: &CommandInteraction,
) -> BotResult<()> {
    let guild = command_guild(command)?.to_string();
    let mut profile =
        state
            .db()
            .profile(&guild, &command.user.id.to_string(), &command.user.name)?;
    profile.afk_message = option_string(command, "mensagem");
    state.db().save_profile(&profile)?;
    reply(
        command,
        http,
        if profile.afk_message.is_some() {
            format!("AFK ativado: {}", profile.afk_message.unwrap_or_default())
        } else {
            "Seu AFK foi removido.".into()
        },
        true,
    )
    .await
}

async fn command_fun(http: &Http, command: &CommandInteraction) -> BotResult<()> {
    match top_subcommand(command).as_deref() {
        Some("8ball") => {
            let answers = [
                "Sim.",
                "Não.",
                "Provavelmente.",
                "Melhor não contar com isso.",
                "Com certeza!",
            ];
            let answer = answers[rand::rng().random_range(0..answers.len())];
            embed_reply(
                command,
                http,
                CreateEmbed::new()
                    .title("8ball")
                    .description(answer)
                    .color(PURPLE),
                false,
            )
            .await
        }
        Some("ship") => {
            let user = option_user(command, "usuario").unwrap_or(command.user.id);
            let score = rand::rng().random_range(0..=100);
            embed_reply(
                command,
                http,
                CreateEmbed::new()
                    .title("Compatibilidade")
                    .description(format!(
                        "<@{}> + <@{}>\n💞 **{score}%**",
                        command.user.id, user
                    ))
                    .color(0xE91E63),
                false,
            )
            .await
        }
        Some("social") => {
            let user = option_user(command, "usuario").unwrap_or(command.user.id);
            let action = match option_string(command, "acao").as_deref() {
                Some("kiss") => "deu um beijo em",
                Some("pat") => "fez carinho em",
                Some("cuddle") => "se aconchegou com",
                _ => "deu um abraço em",
            };
            reply(
                command,
                http,
                format!("<@{}> {action} <@{}>.", command.user.id, user),
                false,
            )
            .await
        }
        _ => reply(command, http, "Subcomando inválido.", true).await,
    }
}

async fn command_poll(ctx: &Context, command: &CommandInteraction) -> BotResult<()> {
    let question = option_string(command, "pergunta").unwrap_or_default();
    let mut description = Vec::new();
    for (index, name) in ["opcao1", "opcao2", "opcao3"].iter().enumerate() {
        if let Some(value) = option_string(command, name) {
            description.push(format!("**{}.** {}", index + 1, value));
        }
    }
    let buttons = description
        .iter()
        .enumerate()
        .map(|(i, _)| {
            CreateButton::new(format!("poll:{i}"))
                .label(format!("Opção {}", i + 1))
                .style(ButtonStyle::Primary)
        })
        .collect();
    command
        .create_response(
            &ctx.http,
            CreateInteractionResponse::Message(
                CreateInteractionResponseMessage::new()
                    .embed(
                        CreateEmbed::new()
                            .title(question)
                            .description(description.join("\n"))
                            .color(PURPLE),
                    )
                    .components(vec![CreateActionRow::Buttons(buttons)]),
            ),
        )
        .await?;
    Ok(())
}

async fn command_remind(
    state: &Arc<State>,
    http: &Http,
    command: &CommandInteraction,
) -> BotResult<()> {
    let guild = command_guild(command)?.to_string();
    let duration = parse_duration(&option_string(command, "duracao").unwrap_or_default())
        .ok_or("Duração inválida; use 10m, 2h, 1d ou 1w.")?;
    let message = option_string(command, "mensagem").unwrap_or_default();
    state.db().conn.execute("INSERT INTO reminders(guild_id,user_id,channel_id,message,remind_at) VALUES(?1,?2,?3,?4,?5)", params![guild,command.user.id.to_string(),command.channel_id.to_string(),message,now()+duration])?;
    reply(
        command,
        http,
        format!("Lembrete criado para <t:{}:R>.", now() + duration),
        false,
    )
    .await
}

async fn command_coinflip(
    state: &Arc<State>,
    http: &Http,
    command: &CommandInteraction,
) -> BotResult<()> {
    let guild = command_guild(command)?.to_string();
    let bet = option_integer(command, "aposta").unwrap_or(0);
    if bet <= 0 {
        return reply(command, http, "A aposta deve ser positiva.", true).await;
    }
    let choice = option_string(command, "escolha")
        .unwrap_or_default()
        .to_lowercase();
    let mut profile =
        state
            .db()
            .profile(&guild, &command.user.id.to_string(), &command.user.name)?;
    if profile.wallet < bet {
        return reply(command, http, "Saldo insuficiente.", true).await;
    }
    let result = if rand::rng().random_bool(0.5) {
        "cara"
    } else {
        "coroa"
    };
    if choice == result {
        profile.wallet += bet;
    } else {
        profile.wallet -= bet;
    }
    state.db().save_profile(&profile)?;
    state.db().transaction(
        &guild,
        &command.user.id.to_string(),
        None,
        None,
        "game",
        bet,
        "Coinflip",
    )?;
    embed_reply(
        command,
        http,
        CreateEmbed::new()
            .title("Coinflip")
            .description(format!(
                "Resultado: **{result}**\n{}",
                if choice == result {
                    format!("Você ganhou {}!", format_currency(bet))
                } else {
                    format!("Você perdeu {}.", format_currency(bet))
                }
            ))
            .color(if choice == result { GREEN } else { RED }),
        false,
    )
    .await
}

async fn command_blackjack(
    state: &Arc<State>,
    http: &Http,
    command: &CommandInteraction,
) -> BotResult<()> {
    let guild = command_guild(command)?.to_string();
    let bet = option_integer(command, "aposta").unwrap_or(0);
    let mut profile =
        state
            .db()
            .profile(&guild, &command.user.id.to_string(), &command.user.name)?;
    if bet <= 0 || profile.wallet < bet {
        return reply(
            command,
            http,
            "Aposta inválida ou saldo insuficiente.",
            true,
        )
        .await;
    }
    let key = format!("{guild}:{}", command.user.id);
    if state
        .blackjack
        .lock()
        .expect("blackjack mutex poisoned")
        .contains_key(&key)
    {
        return reply(
            command,
            http,
            "Você já possui uma partida em andamento.",
            true,
        )
        .await;
    }
    profile.wallet -= bet;
    state.db().save_profile(&profile)?;
    state.db().transaction(
        &guild,
        &command.user.id.to_string(),
        None,
        None,
        "game",
        bet,
        "Blackjack",
    )?;
    let game = BlackjackGame {
        bet,
        player: vec![random_card(), random_card()],
        dealer: vec![random_card(), random_card()],
    };
    let description = blackjack_description(&game, false, "Escolha sua jogada.");
    state
        .blackjack
        .lock()
        .expect("blackjack mutex poisoned")
        .insert(key, game);
    command
        .create_response(
            http,
            CreateInteractionResponse::Message(
                CreateInteractionResponseMessage::new()
                    .embed(
                        CreateEmbed::new()
                            .title("Blackjack")
                            .description(description)
                            .color(BLUE),
                    )
                    .components(vec![blackjack_buttons()]),
            ),
        )
        .await?;
    Ok(())
}

fn random_card() -> u8 {
    rand::rng().random_range(1..=13)
}

fn card_label(card: u8) -> &'static str {
    match card {
        1 => "A",
        11 => "J",
        12 => "Q",
        13 => "K",
        _ => match card {
            2 => "2",
            3 => "3",
            4 => "4",
            5 => "5",
            6 => "6",
            7 => "7",
            8 => "8",
            9 => "9",
            _ => "10",
        },
    }
}

fn hand_score(cards: &[u8]) -> i64 {
    let mut score: i64 = cards
        .iter()
        .map(|card| {
            if *card == 1 {
                11
            } else if *card >= 10 {
                10
            } else {
                *card as i64
            }
        })
        .sum();
    let mut aces = cards.iter().filter(|card| **card == 1).count();
    while score > 21 && aces > 0 {
        score -= 10;
        aces -= 1;
    }
    score
}

fn blackjack_description(game: &BlackjackGame, reveal_dealer: bool, result: &str) -> String {
    let dealer = if reveal_dealer {
        game.dealer
            .iter()
            .map(|card| card_label(*card))
            .collect::<Vec<_>>()
            .join(" ")
    } else {
        format!("{} 🂠", card_label(game.dealer[0]))
    };
    format!(
        "**Dealer:** {dealer} ({})\n**Você:** {} ({})\n\n{result}",
        if reveal_dealer {
            hand_score(&game.dealer).to_string()
        } else {
            "?".into()
        },
        game.player
            .iter()
            .map(|card| card_label(*card))
            .collect::<Vec<_>>()
            .join(" "),
        hand_score(&game.player)
    )
}

fn blackjack_buttons() -> CreateActionRow {
    CreateActionRow::Buttons(vec![
        CreateButton::new("blackjack:hit")
            .label("Comprar")
            .style(ButtonStyle::Primary),
        CreateButton::new("blackjack:stand")
            .label("Parar")
            .style(ButtonStyle::Success),
    ])
}

fn blackjack_buttons_disabled(disabled: bool) -> CreateActionRow {
    CreateActionRow::Buttons(vec![
        CreateButton::new("blackjack:hit")
            .label("Comprar")
            .style(ButtonStyle::Primary)
            .disabled(disabled),
        CreateButton::new("blackjack:stand")
            .label("Parar")
            .style(ButtonStyle::Success)
            .disabled(disabled),
    ])
}

const SHOP_ITEMS: &[(&str, &str, i64, &str)] = &[
    ("coffee", "Café", 100, "Aumenta sua disposição."),
    ("cookie", "Cookie", 250, "Um cookie delicioso."),
    ("ticket", "Ticket", 1_000, "Um item colecionável."),
];

async fn command_economy(
    state: &Arc<State>,
    http: &Http,
    command: &CommandInteraction,
    name: &str,
) -> BotResult<()> {
    let guild = command_guild(command)?.to_string();
    let target = option_user(command, "usuario").unwrap_or(command.user.id);
    let target_name = if target == command.user.id {
        command.user.name.clone()
    } else {
        ctx_user_name(http, target)
            .await
            .unwrap_or_else(|| target.to_string())
    };
    let mut profile = state
        .db()
        .profile(&guild, &target.to_string(), &target_name)?;
    match name {
        "balance" => {
            embed_reply(
                command,
                http,
                CreateEmbed::new()
                    .title(format!("Saldo de {target_name}"))
                    .field("Carteira", format_currency(profile.wallet), true)
                    .field("Banco", format_currency(profile.bank), true)
                    .field(
                        "Total",
                        format_currency(profile.wallet + profile.bank),
                        true,
                    )
                    .color(GREEN),
                false,
            )
            .await
        }
        "inventory" => {
            let description = if profile.inventory.is_empty() {
                "Inventário vazio.".into()
            } else {
                profile
                    .inventory
                    .iter()
                    .map(|entry| {
                        let item = SHOP_ITEMS.iter().find(|item| item.0 == entry.item_id);
                        format!(
                            "{}: **{}**",
                            item.map(|v| v.1).unwrap_or(&entry.item_id),
                            entry.quantity
                        )
                    })
                    .collect::<Vec<_>>()
                    .join("\n")
            };
            embed_reply(
                command,
                http,
                CreateEmbed::new()
                    .title(format!("Inventário de {target_name}"))
                    .description(description)
                    .color(BLUE),
                false,
            )
            .await
        }
        "transactions" => {
            let lines = {
                let conn = state.db();
                let mut stmt = conn.conn.prepare("SELECT kind,amount,reason,created_at FROM transactions WHERE guild_id=?1 AND user_id=?2 ORDER BY created_at DESC LIMIT 10")?;
                let mut rows = stmt.query(params![guild, command.user.id.to_string()])?;
                let mut lines = Vec::new();
                while let Some(row) = rows.next()? {
                    let kind: String = row.get(0)?;
                    let amount: i64 = row.get(1)?;
                    let reason: String = row.get(2)?;
                    let at: i64 = row.get(3)?;
                    lines.push(format!(
                        "<t:{at}:d> **{kind}** {} — {reason}",
                        format_currency(amount)
                    ));
                }
                lines
            };
            embed_reply(
                command,
                http,
                CreateEmbed::new()
                    .title("Extrato")
                    .description(if lines.is_empty() {
                        "Nenhuma transação.".into()
                    } else {
                        lines.join("\n")
                    })
                    .color(BLUE),
                false,
            )
            .await
        }
        "daily" => {
            if now() - profile.last_daily_at < 86_400 {
                return reply(
                    command,
                    http,
                    format!(
                        "Você já recebeu sua recompensa. Tente novamente <t:{}:R>.",
                        profile.last_daily_at + 86_400
                    ),
                    true,
                )
                .await;
            }
            profile.last_daily_at = now();
            profile.wallet += 500;
            state.db().save_profile(&profile)?;
            state.db().transaction(
                &guild,
                &command.user.id.to_string(),
                None,
                None,
                "reward",
                500,
                "Recompensa diária",
            )?;
            embed_reply(
                command,
                http,
                CreateEmbed::new()
                    .title("Recompensa diária")
                    .description(format!("Você recebeu **{}**.", format_currency(500)))
                    .color(GREEN),
                false,
            )
            .await
        }
        "work" => {
            if now() - profile.last_work_at < 3_600 {
                return reply(
                    command,
                    http,
                    format!(
                        "Você já trabalhou. Tente novamente <t:{}:R>.",
                        profile.last_work_at + 3_600
                    ),
                    true,
                )
                .await;
            }
            let amount = rand::rng().random_range(100..=300);
            profile.last_work_at = now();
            profile.wallet += amount;
            state.db().save_profile(&profile)?;
            state.db().transaction(
                &guild,
                &command.user.id.to_string(),
                None,
                None,
                "credit",
                amount,
                "Trabalho",
            )?;
            reply(
                command,
                http,
                format!("Você trabalhou e ganhou **{}**.", format_currency(amount)),
                false,
            )
            .await
        }
        "pay" => {
            let recipient = option_user(command, "usuario").ok_or("Destinatário obrigatório")?;
            let amount = option_integer(command, "valor").unwrap_or(0);
            if recipient == command.user.id {
                return reply(
                    command,
                    http,
                    "Você não pode transferir para si mesmo.",
                    true,
                )
                .await;
            }
            if amount <= 0 || profile.wallet < amount {
                return reply(command, http, "Valor inválido ou saldo insuficiente.", true).await;
            }
            let mut destination =
                state
                    .db()
                    .profile(&guild, &recipient.to_string(), &recipient.to_string())?;
            profile.wallet -= amount;
            destination.wallet += amount;
            state.db().save_profile(&profile)?;
            state.db().save_profile(&destination)?;
            state.db().transaction(
                &guild,
                &command.user.id.to_string(),
                Some(&command.user.id.to_string()),
                Some(&recipient.to_string()),
                "transfer",
                amount,
                "Transferência",
            )?;
            reply(
                command,
                http,
                format!(
                    "Você enviou **{}** para <@{}>.",
                    format_currency(amount),
                    recipient
                ),
                false,
            )
            .await
        }
        "shop" => {
            if top_subcommand(command).as_deref() == Some("list")
                || top_subcommand(command).is_none()
            {
                let description = SHOP_ITEMS
                    .iter()
                    .map(|item| {
                        format!(
                            "**{}** — `{}` — {}\n{}",
                            item.1,
                            item.0,
                            format_currency(item.2),
                            item.3
                        )
                    })
                    .collect::<Vec<_>>()
                    .join("\n\n");
                return embed_reply(
                    command,
                    http,
                    CreateEmbed::new()
                        .title("Loja")
                        .description(description)
                        .color(0xF1C40F),
                    false,
                )
                .await;
            }
            let item_id = option_string(command, "item").unwrap_or_default();
            let Some(item) = SHOP_ITEMS.iter().find(|item| item.0 == item_id) else {
                return reply(command, http, "Item não encontrado.", true).await;
            };
            if profile.wallet < item.2 {
                return reply(command, http, "Saldo insuficiente.", true).await;
            }
            profile.wallet -= item.2;
            if let Some(entry) = profile
                .inventory
                .iter_mut()
                .find(|entry| entry.item_id == item.0)
            {
                entry.quantity += 1;
            } else {
                profile.inventory.push(InventoryItem {
                    item_id: item.0.into(),
                    quantity: 1,
                });
            }
            state.db().save_profile(&profile)?;
            state.db().transaction(
                &guild,
                &command.user.id.to_string(),
                None,
                None,
                "purchase",
                item.2,
                item.1,
            )?;
            reply(
                command,
                http,
                format!("Compra realizada: **{}**.", item.1),
                false,
            )
            .await
        }
        _ => reply(command, http, "Comando de economia inválido.", true).await,
    }
}

async fn command_add_balance(
    state: &Arc<State>,
    http: &Http,
    command: &CommandInteraction,
) -> BotResult<()> {
    let guild = command_guild(command)?.to_string();
    let target = option_user(command, "usuario").ok_or("Usuário obrigatório")?;
    let amount = option_integer(command, "valor").unwrap_or(0);
    if amount <= 0 {
        return reply(
            command,
            http,
            "O valor precisa ser um número positivo.",
            true,
        )
        .await;
    }

    let reason = option_string(command, "motivo")
        .filter(|value| !value.trim().is_empty())
        .unwrap_or_else(|| "Adição administrativa".into());
    let target_name = ctx_user_name(http, target)
        .await
        .unwrap_or_else(|| target.to_string());
    let target_id = target.to_string();
    let admin_id = command.user.id.to_string();
    let mut profile = state.db().profile(&guild, &target_id, &target_name)?;
    let Some(new_wallet) = profile.wallet.checked_add(amount) else {
        return reply(command, http, "O saldo máximo foi excedido.", true).await;
    };

    profile.wallet = new_wallet;
    state.db().save_profile(&profile)?;
    state.db().transaction(
        &guild,
        &target_id,
        Some(&admin_id),
        Some(&target_id),
        "admin_credit",
        amount,
        &reason,
    )?;

    embed_reply(
        command,
        http,
        CreateEmbed::new()
            .title("Saldo adicionado")
            .description(format!(
                "<@{target_id}> recebeu **{}** na carteira.",
                format_currency(amount)
            ))
            .field("Novo saldo", format_currency(profile.wallet), true)
            .field("Motivo", reason, true)
            .color(GREEN),
        false,
    )
    .await
}

async fn profile_image(
    state: &Arc<State>,
    guild: &str,
    user: &User,
    profile: &Profile,
) -> BotResult<Vec<u8>> {
    let rank = {
        let db = state.db();
        db.conn.query_row(
            "SELECT COUNT(*) FROM profiles WHERE guild_id=?1 AND total_xp>?2",
            params![guild, profile.total_xp],
            |row| row.get::<_, i64>(0),
        )? + 1
    };
    Ok(render_profile_card(
        &state.http,
        ProfileCardInput {
            username: user.name.clone(),
            avatar_url: user.face(),
            level: level_from_xp(profile.total_xp),
            total_xp: profile.total_xp,
            rank,
            background: profile.profile_background.clone(),
        },
    )
    .await?)
}

async fn ranking_image(
    state: &Arc<State>,
    http: &Http,
    guild: &str,
    period: &str,
    background: &str,
) -> BotResult<Vec<u8>> {
    let (field, period_label) = match period {
        "weekly" => ("weekly_xp", "Semanal"),
        "monthly" => ("monthly_xp", "Mensal"),
        _ => ("total_xp", "Total"),
    };
    let rows = {
        let db = state.db();
        let mut statement = db.conn.prepare(&format!(
            "SELECT username,user_id,{field} FROM profiles WHERE guild_id=?1 ORDER BY {field} DESC LIMIT 10"
        ))?;
        let mut result = Vec::new();
        let mut query = statement.query(params![guild])?;
        while let Some(row) = query.next()? {
            result.push((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, i64>(2)?,
            ));
        }
        result
    };

    let entries =
        futures::future::join_all(rows.into_iter().map(|(username, user_id, xp)| async move {
            let fallback_avatar = user_id
                .parse::<u64>()
                .map(|id| format!("https://cdn.discordapp.com/embed/avatars/{}.png", id % 5))
                .unwrap_or_default();
            if let Ok(id) = user_id.parse::<u64>() {
                if let Ok(user) = http.get_user(UserId::new(id)).await {
                    let avatar_url = user.face();
                    return RankingEntry {
                        username: user.name,
                        avatar_url,
                        xp,
                    };
                }
            }
            RankingEntry {
                username,
                avatar_url: fallback_avatar,
                xp,
            }
        }))
        .await;

    Ok(render_ranking_card(
        &state.http,
        RankingCardInput {
            period: period_label.to_string(),
            entries,
            background: background.to_string(),
        },
    )
    .await?)
}

async fn command_levels(
    state: &Arc<State>,
    http: &Http,
    command: &CommandInteraction,
    name: &str,
) -> BotResult<()> {
    let guild = command_guild(command)?.to_string();
    if matches!(name, "profile" | "ranking" | "leaderboard") {
        command.defer(http).await?;
    }
    let target = option_user(command, "usuario").unwrap_or(command.user.id);
    let user = if target == command.user.id {
        command.user.clone()
    } else {
        http.get_user(target).await?
    };
    let username = user.name.clone();
    let profile = state.db().profile(&guild, &target.to_string(), &username)?;
    match name {
        "level" => {
            let level = level_from_xp(profile.total_xp);
            let next = xp_for_level(level + 1);
            embed_reply(command, http, CreateEmbed::new().title(format!("Nível de {username}")).description(format!("Nível **{level}**\nXP total: **{}** / **{next}**\nXP semanal: **{}**\nXP mensal: **{}**", profile.total_xp, profile.weekly_xp, profile.monthly_xp)).color(PURPLE), false).await
        }
        "profile" => {
            let image = profile_image(state, &guild, &user, &profile).await?;
            let attachment = CreateAttachment::bytes(image, "profile.png");
            command
                .edit_response(
                    http,
                    EditInteractionResponse::new().new_attachment(attachment),
                )
                .await?;
            Ok(())
        }
        "ranking" | "leaderboard" => {
            let period = option_string(command, "periodo").unwrap_or_else(|| "total".into());
            let image =
                ranking_image(state, http, &guild, &period, &profile.profile_background).await?;
            let attachment = CreateAttachment::bytes(image, "ranking.png");
            command
                .edit_response(
                    http,
                    EditInteractionResponse::new().new_attachment(attachment),
                )
                .await?;
            Ok(())
        }
        _ => reply(command, http, "Comando de níveis inválido.", true).await,
    }
}

async fn command_profile_config(
    state: &Arc<State>,
    http: &Http,
    command: &CommandInteraction,
) -> BotResult<()> {
    let guild = command_guild(command)?.to_string();
    let theme = option_string(command, "tema").unwrap_or_else(|| "midnight".into());
    if !is_profile_theme(&theme) {
        return reply(command, http, "Esse tema não está disponível.", true).await;
    }

    let mut profile =
        state
            .db()
            .profile(&guild, &command.user.id.to_string(), &command.user.name)?;
    profile.profile_background = theme.clone();
    state.db().save_profile(&profile)?;
    reply(
        command,
        http,
        format!("Tema do perfil alterado para **{theme}**. Use `/profile` para visualizar."),
        true,
    )
    .await
}

async fn command_level_config(
    state: &Arc<State>,
    http: &Http,
    command: &CommandInteraction,
) -> BotResult<()> {
    let guild = command_guild(command)?.to_string();
    let mut settings = state.db().settings(&guild)?;
    settings.level_enabled = option_boolean(command, "ativado").unwrap_or(settings.level_enabled);
    if let Some(cooldown) = option_integer(command, "cooldown") {
        settings.xp_cooldown = cooldown.clamp(5, 3_600);
    }
    settings.level_channel = option_channel(command, "canal_anuncio")
        .map(|v| v.to_string())
        .or(settings.level_channel);
    state.db().save_settings(&guild, &settings)?;
    reply(
        command,
        http,
        format!(
            "Níveis {}.",
            if settings.level_enabled {
                "ativados"
            } else {
                "desativados"
            }
        ),
        true,
    )
    .await
}

async fn command_moderation(
    ctx: &Context,
    state: &Arc<State>,
    command: &CommandInteraction,
    name: &str,
) -> BotResult<()> {
    let guild = command_guild(command)?;
    let guild_string = guild.to_string();
    match name {
        "warnings" => {
            let target = option_user(command, "usuario").ok_or("Usuário obrigatório")?;
            let lines = {
                let conn = state.db();
                let mut stmt = conn.conn.prepare("SELECT reason,moderator_id,created_at FROM warnings WHERE guild_id=?1 AND target_id=?2 ORDER BY created_at DESC LIMIT 20")?;
                let mut rows = stmt.query(params![guild_string, target.to_string()])?;
                let mut lines = Vec::new();
                while let Some(row) = rows.next()? {
                    let reason: String = row.get(0)?;
                    let moderator: String = row.get(1)?;
                    let at: i64 = row.get(2)?;
                    lines.push(format!("<t:{at}:d> — {reason} (por <@{moderator}>)"));
                }
                lines
            };
            return embed_reply(
                command,
                &ctx.http,
                CreateEmbed::new()
                    .title("Advertências")
                    .description(if lines.is_empty() {
                        "Nenhuma advertência.".into()
                    } else {
                        lines.join("\n")
                    })
                    .color(0xF1C40F),
                true,
            )
            .await;
        }
        "warn" => {
            let target = option_user(command, "usuario").ok_or("Usuário obrigatório")?;
            let reason =
                option_string(command, "motivo").unwrap_or_else(|| "Sem motivo informado".into());
            let case = state.db().next_warning(
                &guild_string,
                &target.to_string(),
                &target.to_string(),
                &command.user.id.to_string(),
                &reason,
            )?;
            let _ = target.create_dm_channel(&ctx.http).await;
            let target_label = audit_user_label(&ctx.http, target).await;
            send_punishment_log(
                ctx,
                guild,
                "Advertencia",
                &target_label,
                &audit_actor_label(&command.user),
                &reason,
                Some(command.channel_id),
                &format!("Caso #{case}. Advertencia registrada no banco local."),
                0xF1C40F,
            )
            .await;
            return reply(
                command,
                &ctx.http,
                format!("Advertência aplicada. Caso #{case}."),
                false,
            )
            .await;
        }
        "kick" => {
            let target = option_user(command, "usuario").ok_or("Usuário obrigatório")?;
            let reason =
                option_string(command, "motivo").unwrap_or_else(|| "Sem motivo informado".into());
            guild.kick_with_reason(&ctx.http, target, &reason).await?;
            let target_label = audit_user_label(&ctx.http, target).await;
            send_punishment_log(
                ctx,
                guild,
                "Expulsao",
                &target_label,
                &audit_actor_label(&command.user),
                &reason,
                Some(command.channel_id),
                "Comando manual /kick.",
                RED,
            )
            .await;
            return reply(command, &ctx.http, "Membro expulso.", false).await;
        }
        "ban" => {
            let target = option_user(command, "usuario").ok_or("Usuário obrigatório")?;
            let reason =
                option_string(command, "motivo").unwrap_or_else(|| "Sem motivo informado".into());
            guild.ban_with_reason(&ctx.http, target, 0, &reason).await?;
            let target_label = audit_user_label(&ctx.http, target).await;
            send_punishment_log(
                ctx,
                guild,
                "Banimento",
                &target_label,
                &audit_actor_label(&command.user),
                &reason,
                Some(command.channel_id),
                "Comando manual /ban. Nenhuma mensagem adicional foi removida.",
                RED,
            )
            .await;
            return reply(command, &ctx.http, "Membro banido.", false).await;
        }
        "unban" => {
            let target = option_string(command, "usuario_id")
                .ok_or("ID obrigatório")?
                .parse::<u64>()?;
            let target = UserId::new(target);
            guild.unban(&ctx.http, target).await?;
            let target_label = audit_user_label(&ctx.http, target).await;
            send_punishment_log(
                ctx,
                guild,
                "Banimento removido",
                &target_label,
                &audit_actor_label(&command.user),
                "Remocao manual do banimento.",
                Some(command.channel_id),
                "Comando manual /unban.",
                BLUE,
            )
            .await;
            return reply(command, &ctx.http, "Banimento removido.", false).await;
        }
        "timeout" | "untimeout" => {
            let target = option_user(command, "usuario").ok_or("Usuário obrigatório")?;
            let mut member = guild.member(&ctx.http, target).await?;
            if name == "untimeout" {
                member
                    .edit(&ctx.http, EditMember::new().enable_communication())
                    .await?;
                let target_label = audit_user_label(&ctx.http, target).await;
                send_punishment_log(
                    ctx,
                    guild,
                    "Timeout removido",
                    &target_label,
                    &audit_actor_label(&command.user),
                    "Remocao manual do timeout.",
                    Some(command.channel_id),
                    "Comando manual /untimeout.",
                    BLUE,
                )
                .await;
                return reply(command, &ctx.http, "Timeout removido.", false).await;
            }
            let duration = parse_duration(&option_string(command, "duracao").unwrap_or_default())
                .ok_or("Duração inválida (máximo 28 dias).")?;
            let until_unix = now() + duration;
            let until =
                Timestamp::from_unix_timestamp(until_unix).map_err(|_| "timestamp inválido")?;
            member
                .disable_communication_until_datetime(&ctx.http, until)
                .await?;
            let target_label = audit_user_label(&ctx.http, target).await;
            send_punishment_log(
                ctx,
                guild,
                "Timeout",
                &target_label,
                &audit_actor_label(&command.user),
                "Timeout aplicado manualmente.",
                Some(command.channel_id),
                &format!(
                    "Comando manual /timeout. Duracao: {duration} segundos. Expira: <t:{until_unix}:F>."
                ),
                BLUE,
            )
            .await;
            return reply(command, &ctx.http, "Timeout aplicado.", false).await;
        }
        "clear" => {
            let quantity = option_integer(command, "quantidade")
                .unwrap_or(0)
                .clamp(1, 100) as u64;
            let messages = command
                .channel_id
                .messages(&ctx.http, GetMessages::new().limit(quantity as u8))
                .await?;
            let ids = messages
                .iter()
                .map(|message| message.id)
                .collect::<Vec<_>>();
            if !ids.is_empty() {
                command.channel_id.delete_messages(&ctx.http, ids).await?;
            }
            let deleted = quantity.min(messages.len() as u64);
            send_punishment_log(
                ctx,
                guild,
                "Mensagens removidas",
                &format!(
                    "Canal <#{}> (`{}`)",
                    command.channel_id, command.channel_id
                ),
                &audit_actor_label(&command.user),
                "Limpeza manual de mensagens.",
                Some(command.channel_id),
                &format!(
                    "Comando manual /clear. Quantidade solicitada: {quantity}. Quantidade removida: {deleted}."
                ),
                BLUE,
            )
            .await;
            return reply(
                command,
                &ctx.http,
                format!("{deleted} mensagens removidas."),
                true,
            )
            .await;
        }
        "lock" | "unlock" => {
            let channel = option_channel_or_current(command, "canal");
            let everyone = PermissionOverwriteType::Role(RoleId::new(guild.get()));
            if name == "lock" {
                channel
                    .create_permission(
                        &ctx.http,
                        PermissionOverwrite {
                            allow: Permissions::empty(),
                            deny: Permissions::SEND_MESSAGES,
                            kind: everyone,
                        },
                    )
                    .await?;
                send_punishment_log(
                    ctx,
                    guild,
                    "Canal bloqueado",
                    &format!("Canal <#{}> (`{}`)", channel, channel),
                    &audit_actor_label(&command.user),
                    "Bloqueio manual do canal.",
                    Some(command.channel_id),
                    &format!("Comando manual /lock. Canal afetado: <#{}>.", channel),
                    BLUE,
                )
                .await;
                return reply(command, &ctx.http, "Canal bloqueado.", true).await;
            }
            channel.delete_permission(&ctx.http, everyone).await?;
            send_punishment_log(
                ctx,
                guild,
                "Canal desbloqueado",
                &format!("Canal <#{}> (`{}`)", channel, channel),
                &audit_actor_label(&command.user),
                "Desbloqueio manual do canal.",
                Some(command.channel_id),
                &format!("Comando manual /unlock. Canal afetado: <#{}>.", channel),
                BLUE,
            )
            .await;
            return reply(command, &ctx.http, "Canal desbloqueado.", true).await;
        }
        _ => {}
    }
    Ok(())
}

async fn command_automod(
    state: &Arc<State>,
    http: &Http,
    command: &CommandInteraction,
) -> BotResult<()> {
    let guild = command_guild(command)?.to_string();
    let mut settings = state.db().settings(&guild)?;
    match top_subcommand(command).as_deref() {
        Some("config") => {
            settings.automod_enabled =
                option_boolean(command, "ativado").unwrap_or(settings.automod_enabled);
            if let Some(value) = option_integer(command, "spam_limite") {
                settings.spam_limit = value.clamp(3, 20);
            }
            if let Some(value) = option_integer(command, "mencoes_limite") {
                settings.mention_limit = value.clamp(2, 20);
            }
        }
        Some("palavra") => list_mutate(
            &mut settings.blocked_words,
            option_string(command, "acao").as_deref(),
            option_string(command, "valor").as_deref(),
        ),
        Some("dominio") => list_mutate(
            &mut settings.blocked_domains,
            option_string(command, "acao").as_deref(),
            option_string(command, "valor").as_deref(),
        ),
        Some("whitelist") => {
            let list = if option_string(command, "tipo").as_deref() == Some("role") {
                &mut settings.exempt_roles
            } else {
                &mut settings.exempt_channels
            };
            list_mutate(
                list,
                option_string(command, "acao").as_deref(),
                option_string(command, "id").as_deref(),
            );
        }
        Some("log") => {
            settings.automod_log_channel = option_channel(command, "canal").map(|v| v.to_string())
        }
        _ => {
            return reply(
                command,
                http,
                "Use config, palavra, dominio, whitelist ou log.",
                true,
            )
            .await
        }
    }
    state.db().save_settings(&guild, &settings)?;
    reply(command, http, "Configuração do automod atualizada.", true).await
}

async fn command_welcome(
    ctx: &Context,
    state: &Arc<State>,
    command: &CommandInteraction,
) -> BotResult<()> {
    let guild = command_guild(command)?.to_string();
    let mut settings = state.db().settings(&guild)?;
    match top_subcommand(command).as_deref() {
        Some("config") => {
            settings.welcome_enabled =
                option_boolean(command, "ativado").unwrap_or(settings.welcome_enabled);
            settings.welcome_channel = option_channel(command, "canal")
                .map(|v| v.to_string())
                .or(settings.welcome_channel);
            if let Some(message) = option_string(command, "mensagem") {
                settings.welcome_message = message;
            }
        }
        Some("disable") => settings.welcome_enabled = false,
        Some("test") => {
            let text = replace_placeholders(
                &settings.welcome_message,
                &[
                    ("mention", command.user.mention().to_string()),
                    ("server", guild.clone()),
                    ("memberCount", "".into()),
                ],
            );
            command.channel_id.say(&ctx.http, text).await?;
        }
        Some("color-role") => {
            reply(
                command,
                &ctx.http,
                "Cargos de cor são gerenciados pelo painel `/cores`.",
                true,
            )
            .await?
        }
        Some("panel") => {
            let channel = option_channel_or_current(command, "canal");
            channel
                .send_message(
                    &ctx.http,
                    CreateMessage::new().embed(
                        CreateEmbed::new()
                            .title("Cores")
                            .description("Escolha uma cor.")
                            .color(PURPLE),
                    ),
                )
                .await?;
        }
        _ => {
            return reply(
                command,
                &ctx.http,
                "Use config, disable, test, color-role ou panel.",
                true,
            )
            .await
        }
    }
    state.db().save_settings(&guild, &settings)?;
    reply(
        command,
        &ctx.http,
        "Configuração de boas-vindas atualizada.",
        true,
    )
    .await
}

fn list_mutate(list: &mut Vec<String>, action: Option<&str>, value: Option<&str>) {
    let Some(value) = value.map(str::trim).filter(|v| !v.is_empty()) else {
        return;
    };
    if action == Some("remove") {
        list.retain(|item| item != value);
    } else if !list.iter().any(|item| item == value) {
        list.push(value.to_lowercase());
    }
}

async fn require_verified(ctx: &Context, component: &ComponentInteraction) -> BotResult<bool> {
    let Some(guild_id) = component.guild_id else {
        component_reply(component, &ctx.http, "Use este painel em um servidor.").await?;
        return Ok(false);
    };
    let member = guild_id.member(&ctx.http, component.user.id).await?;
    if member.roles.contains(&RoleId::new(VERIFIED_ROLE_ID)) {
        return Ok(true);
    }
    component_reply(
        component,
        &ctx.http,
        "Você precisa estar verificado para receber cargos de cor ou tecnologia.",
    )
    .await?;
    Ok(false)
}

async fn update_technology_roles(
    ctx: &Context,
    component: &ComponentInteraction,
    selected_roles: &[RoleId],
) -> BotResult<()> {
    let Some(guild_id) = component.guild_id else {
        return component_reply(component, &ctx.http, "Use este painel em um servidor.").await;
    };
    let roles = guild_id.roles(&ctx.http).await?;
    if selected_roles
        .iter()
        .any(|role_id| !roles.contains_key(role_id))
    {
        return component_reply(
            component,
            &ctx.http,
            "Um ou mais cargos de tecnologia não estão mais disponíveis.",
        )
        .await;
    }

    let member = guild_id.member(&ctx.http, component.user.id).await?;
    let technology_roles = TECHNOLOGY_ROLE_IDS
        .iter()
        .map(|(_, role_id)| RoleId::new(*role_id))
        .collect::<Vec<_>>();
    for role_id in &technology_roles {
        if !selected_roles.contains(role_id) && member.roles.contains(role_id) {
            member.remove_role(&ctx.http, *role_id).await?;
        }
    }
    for role_id in selected_roles {
        if !member.roles.contains(role_id) {
            member.add_role(&ctx.http, *role_id).await?;
        }
    }
    component_reply(component, &ctx.http, "Cargos de tecnologia atualizados.").await
}

async fn handle_component(
    ctx: &Context,
    state: &Arc<State>,
    component: &ComponentInteraction,
) -> BotResult<()> {
    let id = component.data.custom_id.as_str();
    if id.starts_with("panel:verify:") {
        let Some(guild_id) = component.guild_id else {
            return component_reply(component, &ctx.http, "Use este painel em um servidor.").await;
        };
        let roles = guild_id.roles(&ctx.http).await?;
        let role = roles.get(&RoleId::new(VERIFIED_ROLE_ID));
        let Some(role) = role else {
            return component_reply(
                component,
                &ctx.http,
                format!("O cargo de verificado (`{VERIFIED_ROLE_ID}`) não foi encontrado."),
            )
            .await;
        };
        let member = guild_id.member(&ctx.http, component.user.id).await?;
        if member.roles.contains(&role.id) {
            return component_reply(component, &ctx.http, "Você já possui o cargo `Membro`.").await;
        }
        member.add_role(&ctx.http, role.id).await?;
        return component_reply(
            component,
            &ctx.http,
            "Verificação concluída. O cargo `Membro` foi entregue.",
        )
        .await;
    }
    if id == "panel:tech" {
        if !require_verified(ctx, component).await? {
            return Ok(());
        }
        let values = match &component.data.kind {
            ComponentInteractionDataKind::StringSelect { values } => values,
            _ => {
                return component_reply(component, &ctx.http, "Seleção de tecnologias inválida.")
                    .await;
            }
        };
        let mut selected_roles = Vec::with_capacity(values.len());
        for value in values {
            let role_id = value
                .parse::<u64>()
                .ok()
                .map(RoleId::new)
                .filter(|role_id| {
                    TECHNOLOGY_ROLE_IDS
                        .iter()
                        .any(|(_, known_id)| RoleId::new(*known_id) == *role_id)
                });
            let Some(role_id) = role_id else {
                return component_reply(component, &ctx.http, "Cargo de tecnologia inválido.")
                    .await;
            };
            selected_roles.push(role_id);
        }
        return update_technology_roles(ctx, component, &selected_roles).await;
    }
    if let Some(key) = id.strip_prefix("panel:tech:") {
        if !require_verified(ctx, component).await? {
            return Ok(());
        }
        let Some(role_id) =
            panel_technology(key).and_then(|technology| technology_role_id(technology.key))
        else {
            return component_reply(component, &ctx.http, "Tecnologia não encontrada.").await;
        };
        return update_technology_roles(ctx, component, &[role_id]).await;
    }
    if id == "panel:colors" {
        let Some(guild_id) = component.guild_id else {
            return component_reply(component, &ctx.http, "Use este painel em um servidor.").await;
        };
        if !require_verified(ctx, component).await? {
            return Ok(());
        }
        let roles = guild_id.roles(&ctx.http).await?;
        let selected_id = match &component.data.kind {
            ComponentInteractionDataKind::StringSelect { values } => values
                .first()
                .and_then(|value| value.parse::<u64>().ok())
                .map(RoleId::new),
            _ => None,
        };
        let color_roles = COLOR_ROLE_NAMES
            .iter()
            .filter_map(|name| {
                roles
                    .values()
                    .find(|role| role.name.eq_ignore_ascii_case(name))
                    .map(|role| role.id)
            })
            .collect::<Vec<_>>();
        let member = guild_id.member(&ctx.http, component.user.id).await?;
        for role_id in color_roles {
            if Some(role_id) != selected_id && member.roles.contains(&role_id) {
                let _ = member.remove_role(&ctx.http, role_id).await;
            }
        }
        if let Some(selected_id) = selected_id {
            if !roles.contains_key(&selected_id) {
                return component_reply(component, &ctx.http, "Essa cor não está mais disponível.")
                    .await;
            }
            member.add_role(&ctx.http, selected_id).await?;
            return component_reply(component, &ctx.http, "Sua cor foi atualizada.").await;
        }
        return component_reply(component, &ctx.http, "Suas cores foram removidas.").await;
    }
    if let Some(owner_id) = id.strip_prefix("help:category:") {
        let owner_id = owner_id.parse::<u64>().ok().map(UserId::new);
        if owner_id != Some(component.user.id) {
            return component_reply(
                component,
                &ctx.http,
                "Apenas o autor deste painel pode interagir com o menu de ajuda.",
            )
            .await;
        }

        let category = match &component.data.kind {
            ComponentInteractionDataKind::StringSelect { values } => {
                values.first().map(String::as_str).unwrap_or_default()
            }
            _ => "",
        };
        let can_see_admin_commands = component
            .member
            .as_ref()
            .and_then(|member| member.permissions)
            .map(|permissions| permissions.contains(Permissions::ADMINISTRATOR))
            .unwrap_or(false);
        let panel = help_category_panel(component.user.id, can_see_admin_commands, category);
        send_components_v2_interaction(&ctx.http, component.id, &component.token, 7, panel).await?;
        return Ok(());
    }
    if let Some(action) = id.strip_prefix("blackjack:") {
        let Some(guild_id) = component.guild_id else {
            return component_reply(component, &ctx.http, "Use este jogo em um servidor.").await;
        };
        let key = format!("{}:{}", guild_id, component.user.id);
        if action != "hit" && action != "stand" {
            return component_reply(component, &ctx.http, "Jogada inválida.").await;
        }
        let has_game = state
            .blackjack
            .lock()
            .expect("blackjack mutex poisoned")
            .contains_key(&key);
        if !has_game {
            return component_reply(component, &ctx.http, "Essa partida expirou.").await;
        }
        let outcome = {
            let mut games = state.blackjack.lock().expect("blackjack mutex poisoned");
            let game = games.get_mut(&key).expect("game disappeared");
            let mut settled = false;
            let result = if action == "hit" {
                game.player.push(random_card());
                if hand_score(&game.player) > 21 {
                    settled = true;
                    "lose"
                } else {
                    "playing"
                }
            } else if action == "stand" {
                while hand_score(&game.dealer) < 17 {
                    game.dealer.push(random_card());
                }
                settled = true;
                let player = hand_score(&game.player);
                let dealer = hand_score(&game.dealer);
                if player > dealer || dealer > 21 {
                    "win"
                } else if player == dealer {
                    "tie"
                } else {
                    "lose"
                }
            } else {
                unreachable!("action validated above");
            };
            let snapshot = game.clone();
            if settled {
                games.remove(&key);
            }
            (snapshot, result.to_string(), settled)
        };
        if outcome.2 && (outcome.1 == "win" || outcome.1 == "tie") {
            let mut profile = state.db().profile(
                &guild_id.to_string(),
                &component.user.id.to_string(),
                &component.user.name,
            )?;
            profile.wallet += if outcome.1 == "win" {
                outcome.0.bet * 2
            } else {
                outcome.0.bet
            };
            state.db().save_profile(&profile)?;
        }
        let text = match outcome.1.as_str() {
            "win" => format!(
                "Você venceu e recebeu {}.",
                format_currency(outcome.0.bet * 2)
            ),
            "tie" => "Empate: sua aposta foi devolvida.".into(),
            "lose" if hand_score(&outcome.0.player) > 21 => "Você perdeu: estourou 21.".into(),
            "lose" => "Você perdeu a rodada.".into(),
            _ => "Escolha sua jogada.".into(),
        };
        let color = if !outcome.2 {
            BLUE
        } else if outcome.1 == "win" {
            GREEN
        } else if outcome.1 == "lose" {
            RED
        } else {
            BLUE
        };
        component
            .create_response(
                &ctx.http,
                CreateInteractionResponse::UpdateMessage(
                    CreateInteractionResponseMessage::new()
                        .embed(
                            CreateEmbed::new()
                                .title("Blackjack")
                                .description(blackjack_description(&outcome.0, outcome.2, &text))
                                .color(color),
                        )
                        .components(vec![blackjack_buttons_disabled(outcome.2)]),
                ),
            )
            .await?;
        return Ok(());
    }
    if id == "verify:member" {
        let Some(guild_id) = component.guild_id else {
            return component_reply(component, &ctx.http, "Use este painel em um servidor.").await;
        };
        let roles = guild_id.roles(&ctx.http).await?;
        let role = roles.get(&RoleId::new(VERIFIED_ROLE_ID));
        if let Some(role) = role {
            guild_id
                .member(&ctx.http, component.user.id)
                .await?
                .add_role(&ctx.http, role.id)
                .await?;
            return component_reply(component, &ctx.http, "Você foi verificado!").await;
        }
        return component_reply(
            component,
            &ctx.http,
            format!("O cargo de verificado (`{VERIFIED_ROLE_ID}`) não foi encontrado."),
        )
        .await;
    }
    if let Some(technology) = id.strip_prefix("tech:") {
        if !require_verified(ctx, component).await? {
            return Ok(());
        }
        let Some(role_id) =
            panel_technology(technology).and_then(|technology| technology_role_id(technology.key))
        else {
            return component_reply(component, &ctx.http, "Tecnologia não encontrada.").await;
        };
        return update_technology_roles(ctx, component, &[role_id]).await;
    }
    if let Some(color) = id.strip_prefix("color:") {
        let Some(guild_id) = component.guild_id else {
            return component_reply(component, &ctx.http, "Use este painel em um servidor.").await;
        };
        if !require_verified(ctx, component).await? {
            return Ok(());
        }
        let roles = guild_id.roles(&ctx.http).await?;
        let Some(selected) = roles
            .values()
            .find(|role| role.name.eq_ignore_ascii_case(color))
        else {
            return component_reply(
                component,
                &ctx.http,
                format!("Crie o cargo **{color}** para habilitar esta cor."),
            )
            .await;
        };
        let member = guild_id.member(&ctx.http, component.user.id).await?;
        for name in [
            "Vermelho",
            "Branco",
            "Preto",
            "Vermelho Vinho",
            "Rosa",
            "Amarelo",
            "Verde",
            "Verde Escuro",
            "Azul",
            "Roxo",
            "Laranja",
            "Marrom",
        ] {
            if let Some(role) = roles
                .values()
                .find(|role| role.name.eq_ignore_ascii_case(name))
            {
                if role.id != selected.id && member.roles.contains(&role.id) {
                    let _ = member.remove_role(&ctx.http, role.id).await;
                }
            }
        }
        member.add_role(&ctx.http, selected.id).await?;
        return component_reply(
            component,
            &ctx.http,
            format!("Cargo **{color}** atribuído."),
        )
        .await;
    }
    if id.starts_with("poll:") {
        return component_reply(component, &ctx.http, "Voto registrado nesta enquete.").await;
    }
    if let Some(action) = id.strip_prefix("reel:") {
        return component_reply(
            component,
            &ctx.http,
            format!("Ação de reel registrada: {action}."),
        )
        .await;
    }
    if id == "ticket:open" {
        let Some(guild_id) = component.guild_id else {
            return component_reply(component, &ctx.http, "Use este painel em um servidor.").await;
        };
        let category = match &component.data.kind {
            ComponentInteractionDataKind::StringSelect { values } => {
                values.first().cloned().unwrap_or_else(|| "general".into())
            }
            _ => "general".into(),
        };
        let permissions = vec![
            PermissionOverwrite {
                allow: Permissions::empty(),
                deny: Permissions::VIEW_CHANNEL,
                kind: PermissionOverwriteType::Role(RoleId::new(guild_id.get())),
            },
            PermissionOverwrite {
                allow: Permissions::VIEW_CHANNEL
                    | Permissions::SEND_MESSAGES
                    | Permissions::READ_MESSAGE_HISTORY,
                deny: Permissions::empty(),
                kind: PermissionOverwriteType::Member(component.user.id),
            },
        ];
        let safe_name = component
            .user
            .name
            .to_lowercase()
            .chars()
            .filter(|c| c.is_ascii_alphanumeric() || *c == '-')
            .take(70)
            .collect::<String>();
        let channel = guild_id
            .create_channel(
                &ctx.http,
                CreateChannel::new(format!("ticket-{safe_name}"))
                    .topic(format!("Categoria: {category}"))
                    .permissions(permissions),
            )
            .await?;
        channel.id.send_message(&ctx.http, CreateMessage::new().embed(CreateEmbed::new().title("Ticket aberto").description("Explique sua solicitação. Um atendente poderá assumir este atendimento.").color(PURPLE)).components(vec![CreateActionRow::Buttons(vec![CreateButton::new("ticket:close").label("Encerrar").style(ButtonStyle::Danger)])])).await?;
        return component_reply(
            component,
            &ctx.http,
            format!("Ticket criado em {}.", channel.id.mention()),
        )
        .await;
    }
    if id == "ticket:close" {
        let channel = component.channel_id.to_channel(&ctx.http).await?;
        if let Some(channel) = channel.guild() {
            channel.delete(&ctx.http).await?;
        }
        return Ok(());
    }
    component_reply(component, &ctx.http, "Interação recebida.").await
}

async fn component_reply(
    component: &ComponentInteraction,
    http: &Http,
    content: impl Into<String>,
) -> BotResult<()> {
    component
        .create_response(
            http,
            CreateInteractionResponse::Message(
                CreateInteractionResponseMessage::new()
                    .content(content)
                    .ephemeral(true),
            ),
        )
        .await?;
    Ok(())
}

fn audit_actor_label(user: &User) -> String {
    format!("{} — {} (`{}`)", user.mention(), user.name, user.id)
}

async fn audit_user_label(http: &Http, user_id: UserId) -> String {
    match http.get_user(user_id).await {
        Ok(user) => audit_actor_label(&user),
        Err(_) => format!("<@{}> (`{}`)", user_id, user_id),
    }
}

fn audit_automated_actor(state: &Arc<State>) -> String {
    let bot_id = *state.bot_id.lock().expect("bot id mutex poisoned");
    bot_id.map_or_else(
        || "LarperBot (automatico)".to_string(),
        |id| format!("LarperBot (automatico) (`{id}`)"),
    )
}

async fn send_punishment_log(
    ctx: &Context,
    guild_id: GuildId,
    action: &str,
    target: &str,
    executor: &str,
    reason: &str,
    source_channel: Option<ChannelId>,
    details: &str,
    color: u32,
) {
    let source = source_channel.map_or_else(
        || "Nao informado (evento de auditoria)".to_string(),
        |channel| format!("<#{}> (`{}`)", channel, channel),
    );
    let embed = CreateEmbed::new()
        .title(format!("Log de punicao • {action}"))
        .description(format!(
            "**Resultado:** Aplicada\n**Servidor:** `{}`\n**Horario:** <t:{}:F>",
            guild_id,
            now()
        ))
        .field("Alvo", truncate(target, 1024), false)
        .field("Executor", truncate(executor, 1024), false)
        .field(
            "Motivo",
            truncate(
                if reason.trim().is_empty() {
                    "Sem motivo informado"
                } else {
                    reason
                },
                1024,
            ),
            false,
        )
        .field("Origem", source, false)
        .field(
            "Detalhes",
            truncate(
                if details.trim().is_empty() {
                    "Nenhum detalhe adicional."
                } else {
                    details
                },
                1024,
            ),
            false,
        )
        .color(color);

    if let Err(error) = ChannelId::new(PUNISHMENT_LOG_CHANNEL_ID)
        .send_message(&ctx.http, CreateMessage::new().embed(embed))
        .await
    {
        error!(
            "Falha ao enviar log de punicao para o canal {}: {error}",
            PUNISHMENT_LOG_CHANNEL_ID
        );
    }
}

async fn handle_autocomplete(ctx: &Context, interaction: &CommandInteraction) -> BotResult<()> {
    interaction
        .create_response(
            &ctx.http,
            CreateInteractionResponse::Autocomplete(
                CreateAutocompleteResponse::new()
                    .add_string_choice("USD", "USD")
                    .add_string_choice("BRL", "BRL")
                    .add_string_choice("EUR", "EUR"),
            ),
        )
        .await?;
    Ok(())
}

async fn handle_audit_log_entry(
    ctx: &Context,
    state: &Arc<State>,
    entry: AuditLogEntry,
    guild_id: GuildId,
) {
    let own_bot_action = state
        .bot_id
        .lock()
        .expect("bot id mutex poisoned")
        .as_ref()
        .is_some_and(|bot_id| *bot_id == entry.user_id);
    if own_bot_action {
        return;
    }

    let timeout_update = matches!(
        entry.changes.as_ref(),
        Some(changes)
            if changes.iter().any(|change| matches!(
                change,
                serenity::model::guild::audit_log::Change::CommunicationDisabledUntil { .. }
            ))
    );
    let (action, color, user_target) = match entry.action {
        serenity::model::guild::audit_log::Action::Member(
            serenity::model::guild::audit_log::MemberAction::Kick,
        ) => ("Expulsao registrada (auditoria)", RED, true),
        serenity::model::guild::audit_log::Action::Member(
            serenity::model::guild::audit_log::MemberAction::BanAdd,
        ) => ("Banimento registrado (auditoria)", RED, true),
        serenity::model::guild::audit_log::Action::Member(
            serenity::model::guild::audit_log::MemberAction::BanRemove,
        ) => ("Banimento removido (auditoria)", BLUE, true),
        serenity::model::guild::audit_log::Action::Member(
            serenity::model::guild::audit_log::MemberAction::Update,
        ) if timeout_update => ("Timeout alterado (auditoria)", BLUE, true),
        serenity::model::guild::audit_log::Action::Message(
            serenity::model::guild::audit_log::MessageAction::Delete,
        ) => ("Mensagem removida (auditoria)", 0xF1C40F, false),
        serenity::model::guild::audit_log::Action::Message(
            serenity::model::guild::audit_log::MessageAction::BulkDelete,
        ) => ("Mensagens removidas (auditoria)", 0xF1C40F, false),
        _ => return,
    };

    let target_label = if user_target {
        match entry.target_id {
            Some(target_id) => audit_user_label(&ctx.http, UserId::new(target_id.get())).await,
            None => "Usuario alvo nao informado".to_string(),
        }
    } else {
        entry.target_id.map_or_else(
            || "Mensagem alvo nao informada".to_string(),
            |target_id| format!("Mensagem (`{}`)", target_id.get()),
        )
    };
    let executor_label = audit_user_label(&ctx.http, entry.user_id).await;
    let reason = entry
        .reason
        .as_deref()
        .unwrap_or("Sem motivo informado na auditoria.");
    let details = format!(
        "ID da entrada: `{}`. Tipo de acao: `{}`. Opcoes: {:?}. Alteracoes: {:?}.",
        entry.id,
        entry.action.num(),
        entry.options,
        entry.changes
    );
    let source_channel = entry
        .options
        .as_ref()
        .and_then(|options| options.channel_id);
    send_punishment_log(
        ctx,
        guild_id,
        action,
        &target_label,
        &executor_label,
        reason,
        source_channel,
        &details,
        color,
    )
    .await;
}

async fn automod(ctx: &Context, state: &Arc<State>, message: &Message) -> bool {
    let Some(guild_id) = message.guild_id else {
        return false;
    };
    let settings = match state.db().settings(&guild_id.to_string()) {
        Ok(v) => v,
        Err(_) => return false,
    };
    if !settings.automod_enabled
        || settings
            .exempt_channels
            .iter()
            .any(|id| id == &message.channel_id.to_string())
    {
        return false;
    }
    let lower = message.content.to_lowercase();
    let blocked = settings
        .blocked_words
        .iter()
        .any(|word| lower.contains(word))
        || settings
            .blocked_domains
            .iter()
            .any(|domain| lower.contains(domain));
    let mentions = message.mentions.len() as i64 > settings.mention_limit;
    let letters: Vec<char> = message
        .content
        .chars()
        .filter(|c| c.is_alphabetic())
        .collect();
    let caps = letters.len() > 12
        && letters.iter().filter(|c| c.is_uppercase()).count() as i64 * 100 / letters.len() as i64
            >= settings.caps_percent;
    let key = format!("{}:{}", guild_id, message.author.id);
    let spam = {
        let mut map = state.spam.lock().expect("spam mutex poisoned");
        let entries = map.entry(key).or_default();
        let cutoff = Instant::now() - Duration::from_secs(settings.spam_window.max(1) as u64);
        entries.retain(|at| *at > cutoff);
        entries.push(Instant::now());
        entries.len() as i64 >= settings.spam_limit
    };
    if blocked || mentions || caps || spam {
        let deleted = message.delete(&ctx.http).await.is_ok();
        if !deleted {
            error!(
                "Falha ao remover mensagem moderada de {} no canal {} do servidor {}.",
                message.author.id, message.channel_id, guild_id
            );
            return true;
        }
        let mut triggers = Vec::new();
        if blocked {
            triggers.push("palavra ou dominio bloqueado");
        }
        if mentions {
            triggers.push("limite de mencoes excedido");
        }
        if caps {
            triggers.push("excesso de letras maiusculas");
        }
        if spam {
            triggers.push("limite de spam excedido");
        }
        let target_label = audit_user_label(&ctx.http, message.author.id).await;
        let details = format!(
            "Regras acionadas: {}. Conteudo: {}",
            triggers.join(", "),
            if message.content.is_empty() {
                "(indisponivel)"
            } else {
                message.content.as_str()
            }
        );
        let actor_label = audit_automated_actor(state);
        send_punishment_log(
            ctx,
            guild_id,
            "Mensagem removida (automod)",
            &target_label,
            &actor_label,
            "Mensagem removida automaticamente pelas regras de moderacao.",
            Some(message.channel_id),
            &details,
            0xF1C40F,
        )
        .await;
        if let Some(channel) = settings
            .automod_log_channel
            .and_then(|id| id.parse::<u64>().ok())
            .map(ChannelId::new)
        {
            let _ = channel
                .say(
                    &ctx.http,
                    format!("Mensagem moderada de {}.", message.author.mention()),
                )
                .await;
        }
        return true;
    }
    false
}

async fn handle_captcha_message(ctx: &Context, state: &Arc<State>, message: &Message) {
    let is_own_message = state
        .bot_id
        .lock()
        .expect("bot id mutex poisoned")
        .as_ref()
        .is_some_and(|bot_id| *bot_id == message.author.id);
    if is_own_message {
        return;
    }

    let Some(guild_id) = message.guild_id else {
        return;
    };
    match guild_id
        .ban_with_reason(
            &ctx.http,
            message.author.id,
            1,
            "Mensagem enviada no canal #captcha protegido contra spam.",
        )
        .await
    {
        Ok(()) => {
            let target_label = audit_user_label(&ctx.http, message.author.id).await;
            let actor_label = audit_automated_actor(state);
            send_punishment_log(
                ctx,
                guild_id,
                "Banimento automatico (captcha)",
                &target_label,
                &actor_label,
                "Mensagem enviada no canal #captcha protegido contra spam.",
                Some(message.channel_id),
                &format!(
                    "Mensagens removidas: ultimas 24 horas. Conteudo: {}",
                    if message.content.is_empty() {
                        "(indisponivel)"
                    } else {
                        message.content.as_str()
                    }
                ),
                RED,
            )
            .await;
            info!(
                "Usuário {} banido por enviar mensagem no canal #captcha do servidor {guild_id}.",
                message.author.id
            );
        }
        Err(error) => error!(
            "Falha ao banir {} por mensagem no canal #captcha do servidor {guild_id}: {error}",
            message.author.id
        ),
    }
}

fn level_role_level(name: &str) -> Option<i64> {
    let normalized = name.trim().to_lowercase();
    ["level", "nível", "nivel", "lvl"]
        .iter()
        .find_map(|prefix| normalized.strip_prefix(prefix))
        .map(|suffix| suffix.trim_start_matches([' ', '-', '_']))
        .filter(|suffix| {
            !suffix.is_empty() && suffix.chars().all(|character| character.is_ascii_digit())
        })
        .and_then(|suffix| suffix.parse::<i64>().ok())
}

async fn grant_level_role(
    ctx: &Context,
    guild_id: GuildId,
    user_id: UserId,
    level: i64,
) -> Option<RoleId> {
    let roles = match guild_id.roles(&ctx.http).await {
        Ok(roles) => roles,
        Err(error) => {
            debug!("Falha ao carregar cargos de level do servidor {guild_id}: {error}");
            return None;
        }
    };
    let role_id = roles
        .values()
        .find(|role| level_role_level(&role.name) == Some(level))
        .map(|role| role.id)?;
    let member = match guild_id.member(&ctx.http, user_id).await {
        Ok(member) => member,
        Err(error) => {
            debug!("Falha ao carregar membro para cargo de level no servidor {guild_id}: {error}");
            return None;
        }
    };
    if member.roles.contains(&role_id) {
        return None;
    }
    match member.add_role(&ctx.http, role_id).await {
        Ok(()) => Some(role_id),
        Err(error) => {
            debug!("Falha ao entregar cargo de level {role_id} no servidor {guild_id}: {error}");
            None
        }
    }
}

fn level_up_description(user_mention: &str, level: i64, earned_role: Option<RoleId>) -> String {
    earned_role
        .map(|role_id| {
            format!("Parabéns {user_mention}! Agora você é level {level} e ganhou <@&{role_id}>.")
        })
        .unwrap_or_else(|| format!("{user_mention} agora é level {level}!"))
}

async fn gain_xp(ctx: &Context, state: &Arc<State>, message: &Message) {
    let Some(guild_id) = message.guild_id else {
        return;
    };
    let settings = match state.db().settings(&guild_id.to_string()) {
        Ok(settings) => settings,
        Err(error) => {
            error!("Falha ao carregar configuração de XP do servidor {guild_id}: {error}");
            return;
        }
    };
    if !settings.level_enabled
        || settings
            .exempt_channels
            .iter()
            .any(|id| id == &message.channel_id.to_string())
    {
        return;
    }
    let guild = guild_id.to_string();
    let user_id = message.author.id.to_string();
    let mut profile = match state.db().profile(&guild, &user_id, &message.author.name) {
        Ok(profile) => profile,
        Err(error) => {
            error!(
                "Falha ao carregar perfil de XP de {} no servidor {guild}: {error}",
                message.author.name
            );
            return;
        }
    };
    if profile.last_xp_at > 0 && now() - profile.last_xp_at < settings.xp_cooldown.max(0) {
        return;
    }
    let minimum = settings.xp_min.max(1);
    let maximum = settings.xp_max.max(minimum);
    let amount = rand::rng().random_range(minimum..=maximum);
    let old_level = level_from_xp(profile.total_xp);
    profile.total_xp += amount;
    profile.weekly_xp += amount;
    profile.monthly_xp += amount;
    profile.last_xp_at = now();
    if let Err(error) = state.db().save_profile(&profile) {
        error!(
            "Falha ao salvar XP de {} no servidor {guild}: {error}",
            message.author.name
        );
        return;
    }
    debug!(
        "XP concedido: usuário={} servidor={guild} quantidade={amount} total={}",
        message.author.name, profile.total_xp
    );
    let new_level = level_from_xp(profile.total_xp);
    if new_level <= old_level {
        return;
    }
    let earned_role = grant_level_role(ctx, guild_id, message.author.id, new_level).await;
    if let Some(channel) = settings
        .level_channel
        .and_then(|id| id.parse::<u64>().ok())
        .map(ChannelId::new)
    {
        let user_mention = message.author.mention().to_string();
        let description = level_up_description(&user_mention, new_level, earned_role);
        let _ = channel
            .send_message(
                &ctx.http,
                CreateMessage::new().embed(CreateEmbed::new().description(description)),
            )
            .await;
    }
}

async fn reminder_loop(http: Arc<Http>, state: Arc<State>) {
    loop {
        sleep(Duration::from_secs(20)).await;
        let reminders = {
            let db = state.db();
            let mut result = Vec::new();
            if let Ok(mut statement) = db.conn.prepare("SELECT id,channel_id,message FROM reminders WHERE sent=0 AND remind_at<=?1 LIMIT 50") {
                if let Ok(mut rows) = statement.query(params![now()]) { while let Ok(Some(row)) = rows.next() { let id: i64 = row.get(0).unwrap_or_default(); let channel: String = row.get(1).unwrap_or_default(); let message: String = row.get(2).unwrap_or_default(); result.push((id, channel, message)); } }
            }
            for (id, _, _) in &result {
                let _ = db
                    .conn
                    .execute("UPDATE reminders SET sent=1 WHERE id=?1", params![id]);
            }
            result
        };
        for (_, channel, message) in reminders {
            if let Ok(id) = channel.parse::<u64>() {
                let _ = ChannelId::new(id)
                    .say(&http, format!("⏰ Lembrete: {message}"))
                    .await;
            }
        }
    }
}

async fn handle_prefix(
    ctx: &Context,
    state: &Arc<State>,
    message: &Message,
    name: &str,
    args: &[&str],
) -> BotResult<()> {
    let command = name.to_lowercase();
    match command.as_str() {
        "ping" => {
            message.channel_id.say(&ctx.http, "Pong! 🏓").await?;
        }
        "help" | "ajuda" | "h" => {
            let query = args
                .first()
                .map(|value| value.trim())
                .filter(|value| !value.is_empty());
            let can_see_admin_commands = message
                .member
                .as_ref()
                .and_then(|member| member.permissions)
                .map(|permissions| permissions.contains(Permissions::ADMINISTRATOR))
                .unwrap_or(false);
            let panel = query
                .map(|value| command_help_panel(value, message.author.id, can_see_admin_commands))
                .unwrap_or_else(|| {
                    help_home_panel(message.author.id, can_see_admin_commands, state)
                });
            send_components_v2_channel(&ctx.http, message.channel_id, panel, Some(message.id))
                .await?;
            return Ok(());
        }
        "balance" | "bal" => {
            if let Some(guild) = message.guild_id {
                let profile = state.db().profile(
                    &guild.to_string(),
                    &message.author.id.to_string(),
                    &message.author.name,
                )?;
                message
                    .channel_id
                    .say(
                        &ctx.http,
                        format!(
                            "Carteira: {} | Banco: {}",
                            format_currency(profile.wallet),
                            format_currency(profile.bank)
                        ),
                    )
                    .await?;
            }
        }
        "calc" => {
            let expression = args.join(" ");
            let result = calculate(&expression)
                .map(format_number)
                .unwrap_or_else(|e| format!("Erro: {e}"));
            message.channel_id.say(&ctx.http, result).await?;
        }
        _ => {}
    }
    Ok(())
}

fn top_subcommand(command: &CommandInteraction) -> Option<String> {
    command.data.options.iter().find_map(|option| {
        if matches!(
            option.value,
            CommandDataOptionValue::SubCommand(_) | CommandDataOptionValue::SubCommandGroup(_)
        ) {
            Some(option.name.clone())
        } else {
            None
        }
    })
}

async fn ctx_user_name(http: &Http, user_id: UserId) -> Option<String> {
    http.get_user(user_id).await.ok().map(|user| user.name)
}

fn parse_duration(input: &str) -> Option<i64> {
    let input = input.trim().to_lowercase();
    let (number, unit) = input.split_at(input.len().saturating_sub(1));
    let value = number.parse::<i64>().ok()?;
    let seconds = match unit {
        "s" => value,
        "m" => value * 60,
        "h" => value * 3_600,
        "d" => value * 86_400,
        "w" => value * 604_800,
        _ => return None,
    };
    (seconds > 0 && seconds <= 28 * 86_400).then_some(seconds)
}

fn format_currency(amount: i64) -> String {
    format!(
        "{} moedas",
        amount
            .to_string()
            .as_bytes()
            .rchunks(3)
            .rev()
            .map(|chunk| String::from_utf8_lossy(chunk))
            .collect::<Vec<_>>()
            .join(".")
    )
}
fn format_money(value: f64, currency: &str) -> String {
    format!("{value:.2} {currency}")
}
fn truncate(value: &str, max: usize) -> String {
    if value.chars().count() <= max {
        value.to_string()
    } else {
        format!(
            "{}…",
            value
                .chars()
                .take(max.saturating_sub(1))
                .collect::<String>()
        )
    }
}
fn parse_color(value: &str) -> u32 {
    u32::from_str_radix(value.trim_start_matches('#'), 16).unwrap_or(PURPLE)
}
fn level_from_xp(xp: i64) -> i64 {
    let mut level = 0;
    while xp_for_level(level + 1) <= xp && level < 10_000 {
        level += 1;
    }
    level
}
fn xp_for_level(level: i64) -> i64 {
    100 * level * level
}
fn format_number(value: f64) -> String {
    if value.fract() == 0.0 {
        format!("{value:.0}")
    } else {
        format!("{value:.6}")
            .trim_end_matches('0')
            .trim_end_matches('.')
            .to_string()
    }
}

fn calculate(input: &str) -> Result<f64, &'static str> {
    let mut values: Vec<f64> = Vec::new();
    let mut operators: Vec<char> = Vec::new();
    let mut index = 0;
    let chars: Vec<char> = input.chars().collect();
    let mut expecting_value = true;
    while index < chars.len() {
        let c = chars[index];
        if c.is_whitespace() {
            index += 1;
            continue;
        }
        if c.is_ascii_digit() || c == '.' {
            let start = index;
            while index < chars.len() && (chars[index].is_ascii_digit() || chars[index] == '.') {
                index += 1;
            }
            values.push(
                input[start
                    ..input
                        .char_indices()
                        .nth(index)
                        .map(|(i, _)| i)
                        .unwrap_or(input.len())]
                    .parse()
                    .map_err(|_| "número inválido")?,
            );
            expecting_value = false;
            continue;
        }
        if c == '(' {
            operators.push(c);
            expecting_value = true;
            index += 1;
            continue;
        }
        if c == ')' {
            while operators.last().copied().unwrap_or('(') != '(' {
                apply_operator(&mut values, &mut operators)?;
            }
            if operators.pop() != Some('(') {
                return Err("parênteses inválidos");
            }
            expecting_value = false;
            index += 1;
            continue;
        }
        if "+-*/%^".contains(c) {
            let operator = if expecting_value && (c == '+' || c == '-') {
                if c == '-' {
                    'u'
                } else {
                    'p'
                }
            } else {
                c
            };
            while let Some(&top) = operators.last() {
                if top == '('
                    || precedence(top) < precedence(operator)
                    || (right_associative(operator) && precedence(top) == precedence(operator))
                {
                    break;
                }
                apply_operator(&mut values, &mut operators)?;
            }
            operators.push(operator);
            expecting_value = true;
            index += 1;
            continue;
        }
        return Err("caractere não permitido");
    }
    while !operators.is_empty() {
        if operators.last() == Some(&'(') {
            return Err("parênteses inválidos");
        }
        apply_operator(&mut values, &mut operators)?;
    }
    if values.len() != 1 || !values[0].is_finite() {
        return Err("resultado inválido");
    }
    Ok(values[0])
}

fn precedence(operator: char) -> u8 {
    match operator {
        'u' | 'p' => 4,
        '^' => 3,
        '*' | '/' | '%' => 2,
        '+' | '-' => 1,
        _ => 0,
    }
}
fn right_associative(operator: char) -> bool {
    matches!(operator, '^' | 'u' | 'p')
}
fn apply_operator(values: &mut Vec<f64>, operators: &mut Vec<char>) -> Result<(), &'static str> {
    let Some(operator) = operators.pop() else {
        return Err("operador inválido");
    };
    if matches!(operator, 'u' | 'p') {
        let value = values.pop().ok_or("valor ausente")?;
        values.push(if operator == 'u' { -value } else { value });
        return Ok(());
    }
    let right = values.pop().ok_or("valor ausente")?;
    let left = values.pop().ok_or("valor ausente")?;
    let result = match operator {
        '+' => left + right,
        '-' => left - right,
        '*' => left * right,
        '/' if right != 0.0 => left / right,
        '%' if right != 0.0 => left % right,
        '^' => left.powf(right),
        _ => return Err("divisão por zero"),
    };
    if !result.is_finite() {
        return Err("resultado inválido");
    }
    values.push(result);
    Ok(())
}

#[tokio::main]
async fn main() -> BotResult<()> {
    dotenvy::dotenv().ok();
    tracing_subscriber::fmt()
        .with_env_filter(env::var("RUST_LOG").unwrap_or_else(|_| "info".into()))
        .init();
    let config = Config::from_env()?;
    let db = Db::open(&config.database_path)?;
    let mut intents = GatewayIntents::GUILDS
        | GatewayIntents::DIRECT_MESSAGES
        | GatewayIntents::GUILD_MESSAGES
        | GatewayIntents::GUILD_MODERATION;
    if config.prefix_commands || config.automod || config.levels {
        intents |= GatewayIntents::MESSAGE_CONTENT;
    }
    if config.member_events {
        intents |= GatewayIntents::GUILD_MEMBERS;
    }
    let state = Arc::new(State {
        config: config.clone(),
        db: Mutex::new(db),
        http: HttpClient::builder().user_agent("LarperBot/1.0").build()?,
        spam: Mutex::new(HashMap::new()),
        resume: Mutex::new(HashMap::new()),
        blackjack: Mutex::new(HashMap::new()),
        bot_id: Mutex::new(None),
    });

    if let Some(uri) = config.mongo_uri.as_deref() {
        let mut mongo_for_loop = None;
        let mut cache_loaded = false;
        match timeout(Duration::from_secs(20), MongoSync::connect(uri)).await {
            Ok(Ok(mongo)) => {
                cache_loaded = load_remote_cache(&mongo, &state, "carga inicial").await;
                if cache_loaded {
                    push_local_cache(&mongo, &state, "carga inicial").await;
                } else {
                    error!(
                        "Envio inicial ao MongoDB adiado para evitar sobrescrever dados remotos incompletos."
                    );
                }
                mongo_for_loop = Some(mongo);
                info!(
                    "Sincronização MongoDB ativada; próximo envio em {} minutos.",
                    SYNC_INTERVAL.as_secs() / 60
                );
            }
            Ok(Err(error)) => {
                error!(
                    "Não foi possível conectar ao MongoDB no start; usando apenas cache local: {error}"
                );
            }
            Err(_) => {
                error!("Timeout ao conectar ao MongoDB no start; usando apenas cache local.");
            }
        }
        tokio::spawn(mongo_sync_loop(
            uri.to_string(),
            Arc::clone(&state),
            mongo_for_loop,
            cache_loaded,
        ));
    } else {
        info!(
            "MONGO_URI não definido; usando SQLite local como cache persistente sem sincronização remota."
        );
    }

    let mut client = Client::builder(&config.token, intents)
        .event_handler(Handler {
            state: Arc::clone(&state),
        })
        .await?;

    let port = http_port()?;
    let listener = TcpListener::bind(("0.0.0.0", port)).await?;
    let mut health_server = tokio::spawn(run_health_server(listener));
    info!(
        "Iniciando LarperBot em Rust; cache local em {}; health check em 0.0.0.0:{port}",
        config.database_path,
    );

    tokio::select! {
        bot_result = client.start() => {
            health_server.abort();
            let _ = health_server.await;
            bot_result?;
            Ok(())
        }
        health_result = &mut health_server => {
            match health_result {
                Ok(Ok(())) => Err("O servidor HTTP de saúde encerrou inesperadamente.".into()),
                Ok(Err(error)) => Err(error),
                Err(error) => Err(error.into()),
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn health_server_answers_health_checks() {
        let listener = TcpListener::bind(("127.0.0.1", 0)).await.unwrap();
        let address = listener.local_addr().unwrap();
        let server = tokio::spawn(run_health_server(listener));

        let response = HttpClient::new()
            .get(format!("http://{address}/healthz"))
            .send()
            .await
            .unwrap();
        assert!(response.status().is_success());
        assert_eq!(response.text().await.unwrap(), "LarperBot online");

        server.abort();
        let _ = server.await;
    }

    #[test]
    fn help_panel_uses_components_v2_container_and_select_menu() {
        let panel = help_category_panel(UserId::new(42), false, "Informação");

        assert_eq!(panel["type"], 17);
        assert_eq!(panel["accent_color"], PURPLE);
        assert_eq!(panel["components"][0]["type"], 10);
        assert_eq!(panel["components"][1]["type"], 1);
        assert_eq!(panel["components"][1]["components"][0]["type"], 3);
        assert_eq!(
            panel["components"][1]["components"][0]["custom_id"],
            "help:category:42"
        );
    }

    #[test]
    fn help_hides_admin_commands_and_resolves_node_aliases() {
        let public_categories = help_categories(false);
        assert!(public_categories
            .iter()
            .all(|(category, _)| *category != "Moderação" && *category != "Boas-vindas"));
        assert!(help_categories(true)
            .iter()
            .any(|(category, _)| *category == "Moderação"));
        assert_eq!(
            find_help_command("bal").map(|command| command.name),
            Some("balance")
        );

        let details = command_help_panel("balance", UserId::new(42), false);
        assert!(details["components"][0]["content"]
            .as_str()
            .unwrap()
            .contains("`bal`"));
    }

    #[test]
    fn node_panel_flows_are_components_v2_panels() {
        let verify = verify_panel(RoleId::new(123));
        assert_eq!(verify["type"], 17);
        assert_eq!(
            verify["components"][1]["components"][0]["custom_id"],
            "panel:verify:123"
        );

        let technology = technology_panel(&[]);
        assert_eq!(technology["type"], 17);
        assert_eq!(technology["components"].as_array().unwrap().len(), 2);
        assert_eq!(technology["components"][1]["components"][0]["type"], 3);
        assert_eq!(
            technology["components"][1]["components"][0]["custom_id"],
            "panel:tech"
        );
        assert_eq!(
            technology["components"][1]["components"][0]["min_values"],
            0
        );
        assert_eq!(
            technology["components"][1]["components"][0]["max_values"],
            TECHNOLOGY_ROLE_IDS.len()
        );
        assert_eq!(
            technology["components"][1]["components"][0]["options"]
                .as_array()
                .unwrap()
                .len(),
            TECHNOLOGY_ROLE_IDS.len()
        );
        assert_eq!(
            technology_role_id("typescript"),
            Some(RoleId::new(1_548_767_613_000_351_834))
        );
        assert_eq!(
            technology_role_id("discord-js"),
            Some(RoleId::new(1_548_768_351_051_063_346))
        );

        let colors = colors_panel(Vec::new());
        assert_eq!(colors["type"], 17);
        assert_eq!(
            colors["components"][1]["components"][0]["custom_id"],
            "panel:colors"
        );
    }

    #[test]
    fn technology_panel_attaches_available_custom_emojis_by_name() {
        for (technology, emoji) in [
            ("html", "html"),
            ("ruby", "ruby"),
            ("css", "css"),
            ("svelte", "svelte"),
            ("sql", "sql"),
            ("discord-js", "djs"),
            ("docker", "docker"),
            ("python", "python"),
        ] {
            assert!(technology_emoji_names(technology).contains(&emoji));
        }

        let emojis = vec![
            serde_json::from_value::<Emoji>(json!({
                "id": "101",
                "name": "csharp",
                "available": true,
                "animated": false,
                "managed": false,
                "require_colons": true,
                "roles": [],
                "user": null,
            }))
            .unwrap(),
            serde_json::from_value::<Emoji>(json!({
                "id": "103",
                "name": "typescript",
                "available": true,
                "animated": false,
                "managed": false,
                "require_colons": true,
                "roles": [],
                "user": null,
            }))
            .unwrap(),
            serde_json::from_value::<Emoji>(json!({
                "id": "102",
                "name": "rust",
                "available": false,
                "animated": false,
                "managed": false,
                "require_colons": true,
                "roles": [],
                "user": null,
            }))
            .unwrap(),
        ];
        let panel = technology_panel(&emojis);
        let options = panel["components"][1]["components"][0]["options"]
            .as_array()
            .unwrap();
        let option = |role_id: u64| {
            options
                .iter()
                .find(|option| option["value"] == role_id.to_string())
                .unwrap()
        };

        assert_eq!(option(1_548_767_808_605_782_087)["emoji"]["id"], "101");
        assert_eq!(
            option(1_548_767_613_000_351_834)["emoji"]["name"],
            "typescript"
        );
        assert!(option(1_548_767_946_725_483_303)["emoji"].is_null());
    }

    #[test]
    fn level_up_descriptions_match_requested_messages() {
        assert_eq!(
            level_up_description("<@42>", 3, None),
            "<@42> agora é level 3!"
        );
        assert_eq!(
            level_up_description("<@42>", 3, Some(RoleId::new(99))),
            "Parabéns <@42>! Agora você é level 3 e ganhou <@&99>."
        );
        assert_eq!(level_role_level("Level 3"), Some(3));
        assert_eq!(level_role_level("Nível-4"), Some(4));
        assert_eq!(level_role_level("cargo level"), None);
    }

    #[test]
    fn captcha_warning_is_bilingual() {
        assert!(
            CAPTCHA_WARNING.contains("Não mande mensagens aqui ou você será retirado do servidor!")
        );
        assert!(CAPTCHA_WARNING
            .contains("Do not send messages here or you will be removed from the server!"));
    }

    #[test]
    fn duration_accepts_supported_units_and_limit() {
        assert_eq!(parse_duration("10m"), Some(600));
        assert_eq!(parse_duration("2h"), Some(7_200));
        assert_eq!(parse_duration("29d"), None);
        assert_eq!(parse_duration("abc"), None);
    }

    #[test]
    fn level_curve_matches_node_profile() {
        assert_eq!(level_from_xp(99), 0);
        assert_eq!(level_from_xp(100), 1);
        assert_eq!(level_from_xp(399), 1);
        assert_eq!(level_from_xp(400), 2);
        assert_eq!(xp_for_level(3), 900);
    }

    #[test]
    fn calculator_respects_precedence_and_unary_signs() {
        assert_eq!(calculate("(10 + 5) * 2").unwrap(), 30.0);
        assert_eq!(calculate("-2^2").unwrap(), 4.0);
        assert!(calculate("10 / 0").is_err());
        assert!(calculate("std::process::exit(1)").is_err());
    }

    #[test]
    fn sqlite_creates_and_updates_a_profile() {
        let db = Db::open(":memory:").unwrap();
        let mut profile = db.profile("guild", "user", "Gard").unwrap();
        profile.wallet = 500;
        profile.inventory.push(InventoryItem {
            item_id: "coffee".into(),
            quantity: 1,
        });
        db.save_profile(&profile).unwrap();
        let loaded = db.profile("guild", "user", "Gard").unwrap();
        assert_eq!(loaded.wallet, 500);
        assert_eq!(loaded.inventory[0].item_id, "coffee");
    }

    #[test]
    fn sqlite_cache_snapshot_round_trips_profile_data() {
        let source = Db::open(":memory:").unwrap();
        let mut profile = source.profile("guild", "user", "Gard").unwrap();
        profile.total_xp = 230;
        profile.weekly_xp = 120;
        profile.monthly_xp = 230;
        source.save_profile(&profile).unwrap();

        let snapshot = source.cache_snapshot().unwrap();
        let mut target = Db::open(":memory:").unwrap();
        assert_eq!(target.restore_cache(&snapshot).unwrap(), 1);

        let restored = target.profile("guild", "user", "Gard").unwrap();
        assert_eq!(restored.total_xp, 230);
        assert_eq!(restored.weekly_xp, 120);
        assert_eq!(restored.monthly_xp, 230);
    }
}
