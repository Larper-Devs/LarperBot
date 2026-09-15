#![allow(clippy::too_many_arguments)]

use rand::Rng;
use reqwest::Client as HttpClient;
use rusqlite::{params, Connection, OptionalExtension};
use serde::{Deserialize, Serialize};
use serenity::all::*;
use serenity::async_trait;
use std::collections::HashMap;
use std::env;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};
use tokio::time::sleep;
use tracing::{error, info};

type BotResult<T> = Result<T, Box<dyn std::error::Error + Send + Sync>>;

const PURPLE: u32 = 0x8B5CF6;
const GREEN: u32 = 0x2ECC71;
const RED: u32 = 0xED4245;
const BLUE: u32 = 0x5865F2;

#[derive(Clone)]
struct Config {
    token: String,
    prefix: String,
    prefix_commands: bool,
    automod: bool,
    levels: bool,
    member_events: bool,
    database_path: String,
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
            levels: env_bool("ENABLE_LEVELS"),
            member_events: env_bool("ENABLE_MEMBER_EVENTS"),
            database_path: env::var("DATABASE_PATH")
                .unwrap_or_else(|_| "./larperbot.sqlite".into()),
        })
    }
}

fn env_bool(name: &str) -> bool {
    env::var(name)
        .map(|v| v.eq_ignore_ascii_case("true"))
        .unwrap_or(false)
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
            level_enabled: false,
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
}

struct State {
    config: Config,
    db: Mutex<Db>,
    http: HttpClient,
    spam: Mutex<HashMap<String, Vec<Instant>>>,
    resume: Mutex<HashMap<UserId, (Instant, u8)>>,
    blackjack: Mutex<HashMap<String, BlackjackGame>>,
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

    async fn guild_member_addition(&self, ctx: Context, member: Member) {
        if !self.state.config.member_events {
            return;
        }
        let Ok(settings) = self.state.db().settings(&member.guild_id.to_string()) else {
            return;
        };
        if !settings.welcome_enabled {
            return;
        }
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
            let _ = channel.say(&ctx.http, content).await;
        }
        if let Some(role) = settings
            .auto_role
            .and_then(|id| id.parse::<u64>().ok())
            .map(RoleId::new)
        {
            let _ = member.add_role(&ctx.http, role).await;
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
    let c = vec![
        CreateCommand::new("help")
            .description("Lista os comandos do bot.")
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
                .description("Publica o painel de verificação.")
                .add_option(opt(
                    CommandOptionType::Channel,
                    "canal",
                    "Canal do painel.",
                    false,
                )),
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
            .description("Exibe seu perfil.")
            .add_option(opt(CommandOptionType::User, "usuario", "Usuário.", false)),
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
        "help" => command_help(ctx, command).await?,
        "ping" => reply(command, &ctx.http, "Pong! 🏓", false).await?,
        "avatar" => command_avatar(ctx, command).await?,
        "userinfo" => command_userinfo(ctx, command).await?,
        "serverinfo" => command_serverinfo(ctx, command).await?,
        "calc" => command_calc(&ctx.http, command).await?,
        "devex" => command_devex(state, &ctx.http, command).await?,
        "resume" => command_resume(state, &ctx.http, command).await?,
        "verify" | "captcha" => command_verify(ctx, command).await?,
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

async fn command_help(ctx: &Context, command: &CommandInteraction) -> BotResult<()> {
    let requested = option_string(command, "comando");
    if let Some(name) = requested {
        let known = [
            "help",
            "ping",
            "avatar",
            "userinfo",
            "serverinfo",
            "calc",
            "devex",
            "resume",
            "verify",
            "captcha",
            "tech",
            "cores",
            "ticket",
            "reels",
            "tkk",
            "feed",
            "novidades",
            "regras",
            "afk",
            "fun",
            "poll",
            "remind",
            "coinflip",
            "blackjack",
            "balance",
            "daily",
            "work",
            "pay",
            "shop",
            "inventory",
            "transactions",
            "addsaldo",
            "level",
            "leaderboard",
            "ranking",
            "profile",
            "level-config",
            "warn",
            "warnings",
            "timeout",
            "untimeout",
            "kick",
            "ban",
            "unban",
            "clear",
            "lock",
            "unlock",
            "automod",
            "welcome",
        ]
        .contains(&name.as_str());
        return reply(
            command,
            &ctx.http,
            if known {
                format!("`/{name}` está disponível. Use as opções exibidas pelo Discord.")
            } else {
                "Comando não encontrado.".into()
            },
            true,
        )
        .await;
    }
    let text = "**Informação**\n`help` `ping` `avatar` `userinfo` `serverinfo`\n\n**Utilidades**\n`calc` `devex` `resume` `verify` `captcha` `tech` `cores` `ticket` `reels` `tkk` `feed` `novidades` `regras`\n\n**Convivência e jogos**\n`afk` `fun` `poll` `remind` `coinflip` `blackjack`\n\n**Economia e níveis**\n`balance` `daily` `work` `pay` `shop` `inventory` `transactions` `level` `leaderboard` `ranking` `profile`\n\n**Administração**\n`addsaldo` `level-config` `warn` `warnings` `timeout` `untimeout` `kick` `ban` `unban` `clear` `lock` `unlock` `automod` `welcome`";
    embed_reply(
        command,
        &ctx.http,
        CreateEmbed::new()
            .title("LarperBot — ajuda")
            .description(text)
            .color(PURPLE),
        false,
    )
    .await
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
    let channel = option_channel_or_current(command, "canal");
    channel
        .send_message(
            &ctx.http,
            CreateMessage::new()
                .embed(
                    CreateEmbed::new()
                        .title("Verificação")
                        .description("Clique para receber o cargo de membro.")
                        .color(PURPLE),
                )
                .components(vec![CreateActionRow::Buttons(vec![CreateButton::new(
                    "verify:member",
                )
                .label("Verificar")
                .style(ButtonStyle::Success)])]),
        )
        .await?;
    reply(command, &ctx.http, "Painel publicado.", true).await
}

async fn command_tech(ctx: &Context, command: &CommandInteraction) -> BotResult<()> {
    let channel = option_channel_or_current(command, "canal");
    let names = [
        "Rust",
        "TypeScript",
        "JavaScript",
        "Python",
        "C#",
        "C++",
        "Java",
        "Go",
        "Kotlin",
        "Swift",
        "Dart",
        "PHP",
        "Ruby",
        "Lua",
        "SQL",
        "HTML",
        "CSS",
        "React",
        "Vue",
        "Svelte",
        "Node.js",
        "Docker",
        "Git",
        "Linux",
        "Godot",
    ];
    let rows = names
        .chunks(5)
        .map(|chunk| {
            CreateActionRow::Buttons(
                chunk
                    .iter()
                    .map(|name| {
                        CreateButton::new(format!("tech:{name}"))
                            .label(*name)
                            .style(ButtonStyle::Secondary)
                    })
                    .collect(),
            )
        })
        .collect();
    channel
        .send_message(
            &ctx.http,
            CreateMessage::new()
                .embed(
                    CreateEmbed::new()
                        .title("Tecnologias")
                        .description("Escolha uma tecnologia para ver uma descrição.")
                        .color(PURPLE),
                )
                .components(rows),
        )
        .await?;
    reply(command, &ctx.http, "Painel publicado.", true).await
}

async fn command_cores(ctx: &Context, command: &CommandInteraction) -> BotResult<()> {
    let channel = option_channel_or_current(command, "canal");
    let names = [
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
    let rows = names
        .chunks(5)
        .map(|chunk| {
            CreateActionRow::Buttons(
                chunk
                    .iter()
                    .map(|name| {
                        CreateButton::new(format!("color:{name}"))
                            .label(*name)
                            .style(ButtonStyle::Secondary)
                    })
                    .collect(),
            )
        })
        .collect();
    channel
        .send_message(
            &ctx.http,
            CreateMessage::new()
                .embed(
                    CreateEmbed::new()
                        .title("Cores")
                        .description("Escolha uma cor para receber o cargo correspondente.")
                        .color(PURPLE),
                )
                .components(rows),
        )
        .await?;
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

async fn command_levels(
    state: &Arc<State>,
    http: &Http,
    command: &CommandInteraction,
    name: &str,
) -> BotResult<()> {
    let guild = command_guild(command)?.to_string();
    let target = option_user(command, "usuario").unwrap_or(command.user.id);
    let username = if target == command.user.id {
        command.user.name.clone()
    } else {
        ctx_user_name(http, target)
            .await
            .unwrap_or_else(|| target.to_string())
    };
    let profile = state.db().profile(&guild, &target.to_string(), &username)?;
    match name {
        "level" => {
            let level = level_from_xp(profile.total_xp);
            let next = xp_for_level(level + 1);
            embed_reply(command, http, CreateEmbed::new().title(format!("Nível de {username}")).description(format!("Nível **{level}**\nXP total: **{}** / **{next}**\nXP semanal: **{}**\nXP mensal: **{}**", profile.total_xp, profile.weekly_xp, profile.monthly_xp)).color(PURPLE), false).await
        }
        "profile" => {
            let background = profile.profile_background.clone();
            command
                .create_response(
                    http,
                    CreateInteractionResponse::Message(
                        CreateInteractionResponseMessage::new()
                            .embed(
                                CreateEmbed::new()
                                    .title(format!("Perfil de {username}"))
                                    .description(format!(
                                        "Nível **{}**\nXP total: **{}**\nFundo: `{background}`",
                                        level_from_xp(profile.total_xp),
                                        profile.total_xp
                                    ))
                                    .color(PURPLE),
                            )
                            .components(vec![CreateActionRow::Buttons(vec![
                                CreateButton::new("profile:bg:midnight")
                                    .label("Midnight")
                                    .style(ButtonStyle::Secondary),
                                CreateButton::new("profile:bg:aurora")
                                    .label("Aurora")
                                    .style(ButtonStyle::Secondary),
                                CreateButton::new("profile:bg:sunset")
                                    .label("Sunset")
                                    .style(ButtonStyle::Secondary),
                                CreateButton::new("profile:bg:cyber")
                                    .label("Cyber Grid")
                                    .style(ButtonStyle::Secondary),
                            ])]),
                    ),
                )
                .await?;
            Ok(())
        }
        "ranking" => {
            let description = format!(
                "XP total de **{username}**: **{}**\nNível: **{}**",
                profile.total_xp,
                level_from_xp(profile.total_xp)
            );
            embed_reply(
                command,
                http,
                CreateEmbed::new()
                    .title("Ranking visual")
                    .description(description)
                    .color(PURPLE),
                false,
            )
            .await
        }
        "leaderboard" => {
            let period = option_string(command, "periodo").unwrap_or_else(|| "total".into());
            let field = match period.as_str() {
                "weekly" => "weekly_xp",
                "monthly" => "monthly_xp",
                _ => "total_xp",
            };
            let lines = {
                let conn = state.db();
                let mut stmt = conn.conn.prepare(&format!("SELECT username,user_id,{field} FROM profiles WHERE guild_id=?1 ORDER BY {field} DESC LIMIT 10"))?;
                let mut rows = stmt.query(params![guild])?;
                let mut lines = Vec::new();
                let mut position = 1;
                while let Some(row) = rows.next()? {
                    let user: String = row.get(0)?;
                    let id: String = row.get(1)?;
                    let xp: i64 = row.get(2)?;
                    lines.push(format!(
                        "**{}.** {} (<@{}>) — {} XP",
                        position, user, id, xp
                    ));
                    position += 1;
                }
                lines
            };
            embed_reply(
                command,
                http,
                CreateEmbed::new()
                    .title("Ranking")
                    .description(if lines.is_empty() {
                        "Ainda não há dados.".into()
                    } else {
                        lines.join("\n")
                    })
                    .color(PURPLE),
                false,
            )
            .await
        }
        _ => reply(command, http, "Comando de níveis inválido.", true).await,
    }
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
            return reply(command, &ctx.http, "Membro expulso.", false).await;
        }
        "ban" => {
            let target = option_user(command, "usuario").ok_or("Usuário obrigatório")?;
            let reason =
                option_string(command, "motivo").unwrap_or_else(|| "Sem motivo informado".into());
            guild.ban_with_reason(&ctx.http, target, 0, &reason).await?;
            return reply(command, &ctx.http, "Membro banido.", false).await;
        }
        "unban" => {
            let target = option_string(command, "usuario_id")
                .ok_or("ID obrigatório")?
                .parse::<u64>()?;
            guild.unban(&ctx.http, UserId::new(target)).await?;
            return reply(command, &ctx.http, "Banimento removido.", false).await;
        }
        "timeout" | "untimeout" => {
            let target = option_user(command, "usuario").ok_or("Usuário obrigatório")?;
            let mut member = guild.member(&ctx.http, target).await?;
            if name == "untimeout" {
                member
                    .edit(&ctx.http, EditMember::new().enable_communication())
                    .await?;
                return reply(command, &ctx.http, "Timeout removido.", false).await;
            }
            let duration = parse_duration(&option_string(command, "duracao").unwrap_or_default())
                .ok_or("Duração inválida (máximo 28 dias).")?;
            let until = Timestamp::from_unix_timestamp(now() + duration)
                .map_err(|_| "timestamp inválido")?;
            member
                .disable_communication_until_datetime(&ctx.http, until)
                .await?;
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
            return reply(
                command,
                &ctx.http,
                format!(
                    "{} mensagens removidas.",
                    quantity.min(messages.len() as u64)
                ),
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
                return reply(command, &ctx.http, "Canal bloqueado.", true).await;
            }
            channel.delete_permission(&ctx.http, everyone).await?;
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

async fn handle_component(
    ctx: &Context,
    state: &Arc<State>,
    component: &ComponentInteraction,
) -> BotResult<()> {
    let id = component.data.custom_id.as_str();
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
        let role = roles.values().find(|role| {
            role.name.eq_ignore_ascii_case("Membro") || role.name.eq_ignore_ascii_case("verificado")
        });
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
            "Crie um cargo chamado `Membro` ou `verificado` para habilitar a verificação.",
        )
        .await;
    }
    if let Some(technology) = id.strip_prefix("tech:") {
        return component_reply(
            component,
            &ctx.http,
            format!("Tecnologia selecionada: **{technology}**."),
        )
        .await;
    }
    if let Some(color) = id.strip_prefix("color:") {
        let Some(guild_id) = component.guild_id else {
            return component_reply(component, &ctx.http, "Use este painel em um servidor.").await;
        };
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
    if let Some(background) = id.strip_prefix("profile:bg:") {
        let Some(guild_id) = component.guild_id else {
            return component_reply(component, &ctx.http, "Use este perfil em um servidor.").await;
        };
        let mut profile = state.db().profile(
            &guild_id.to_string(),
            &component.user.id.to_string(),
            &component.user.name,
        )?;
        profile.profile_background = background.to_string();
        state.db().save_profile(&profile)?;
        return component_reply(
            component,
            &ctx.http,
            format!("Fundo alterado para **{background}**."),
        )
        .await;
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
        let _ = message.delete(&ctx.http).await;
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

async fn gain_xp(ctx: &Context, state: &Arc<State>, message: &Message) {
    let Some(guild_id) = message.guild_id else {
        return;
    };
    let Ok(settings) = state.db().settings(&guild_id.to_string()) else {
        return;
    };
    if !settings.level_enabled
        || settings
            .exempt_channels
            .iter()
            .any(|id| id == &message.channel_id.to_string())
    {
        return;
    }
    let Ok(mut profile) = state.db().profile(
        &guild_id.to_string(),
        &message.author.id.to_string(),
        &message.author.name,
    ) else {
        return;
    };
    if now() - profile.last_xp_at < settings.xp_cooldown {
        return;
    }
    let amount =
        rand::rng().random_range(settings.xp_min.max(1)..=settings.xp_max.max(settings.xp_min));
    let old_level = level_from_xp(profile.total_xp);
    profile.total_xp += amount;
    profile.weekly_xp += amount;
    profile.monthly_xp += amount;
    profile.last_xp_at = now();
    let _ = state.db().save_profile(&profile);
    let new_level = level_from_xp(profile.total_xp);
    if new_level > old_level {
        if let Some(channel) = settings
            .level_channel
            .and_then(|id| id.parse::<u64>().ok())
            .map(ChannelId::new)
        {
            let _ = channel
                .say(
                    &ctx.http,
                    format!(
                        "Parabéns {}, você alcançou o nível **{new_level}**!",
                        message.author.mention()
                    ),
                )
                .await;
        }
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
        "help" => {
            message
                .channel_id
                .say(
                    &ctx.http,
                    "Use os slash commands do LarperBot; `/help` lista todas as funções.",
                )
                .await?;
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
    100 * level * level + 100 * level
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
    let mut intents = GatewayIntents::GUILDS | GatewayIntents::DIRECT_MESSAGES;
    if config.prefix_commands || config.automod || config.levels {
        intents |= GatewayIntents::GUILD_MESSAGES | GatewayIntents::MESSAGE_CONTENT;
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
    });
    let mut client = Client::builder(&config.token, intents)
        .event_handler(Handler { state })
        .await?;
    info!(
        "Iniciando LarperBot em Rust; banco SQLite em {}",
        config.database_path
    );
    client.start().await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn duration_accepts_supported_units_and_limit() {
        assert_eq!(parse_duration("10m"), Some(600));
        assert_eq!(parse_duration("2h"), Some(7_200));
        assert_eq!(parse_duration("29d"), None);
        assert_eq!(parse_duration("abc"), None);
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
}
