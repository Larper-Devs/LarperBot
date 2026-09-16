use futures::TryStreamExt;
use mongodb::bson::{doc, Bson, DateTime, Document};
use mongodb::Client;
use std::collections::HashSet;
use std::env;
use std::time::Duration;

use crate::BotResult;

pub const SYNC_INTERVAL: Duration = Duration::from_secs(30 * 60);
const CACHE_COLLECTION: &str = "rust_cache";
const LEGACY_PROFILE_COLLECTION: &str = "userprofiles";

#[derive(Clone, Debug)]
pub struct CacheRecord {
    pub id: String,
    pub table: String,
    pub data: Document,
}

#[derive(Clone)]
pub struct MongoSync {
    client: Client,
    database: String,
}

impl MongoSync {
    pub async fn connect(uri: &str) -> BotResult<Self> {
        let client = Client::with_uri_str(uri).await?;
        let database = env::var("MONGO_DATABASE")
            .ok()
            .filter(|value| !value.trim().is_empty())
            .or_else(|| {
                client
                    .default_database()
                    .map(|database| database.name().to_string())
            })
            .unwrap_or_else(|| "larperbot".to_string());

        client
            .database(&database)
            .run_command(doc! { "ping": 1 })
            .await?;

        Ok(Self { client, database })
    }

    pub async fn load_snapshot(&self) -> BotResult<Vec<CacheRecord>> {
        let collection = self
            .client
            .database(&self.database)
            .collection::<Document>(CACHE_COLLECTION);
        let mut cursor = collection.find(doc! {}).await?;
        let mut records = Vec::new();
        let mut ids = HashSet::new();

        while let Some(mut document) = cursor.try_next().await? {
            let Some(id) = document
                .remove("_id")
                .and_then(|value| value.as_str().map(str::to_owned))
            else {
                continue;
            };
            let Some(table) = document
                .remove("table")
                .and_then(|value| value.as_str().map(str::to_owned))
            else {
                continue;
            };
            let Some(Bson::Document(data)) = document.remove("data") else {
                continue;
            };

            ids.insert(id.clone());
            records.push(CacheRecord { id, table, data });
        }

        self.load_legacy_profiles(&mut records, &ids).await?;
        Ok(records)
    }

    async fn load_legacy_profiles(
        &self,
        records: &mut Vec<CacheRecord>,
        known_ids: &HashSet<String>,
    ) -> BotResult<()> {
        let collection = self
            .client
            .database(&self.database)
            .collection::<Document>(LEGACY_PROFILE_COLLECTION);
        let mut cursor = collection.find(doc! {}).await?;

        while let Some(document) = cursor.try_next().await? {
            let Some(guild_id) = string_value(&document, "guildId") else {
                continue;
            };
            let Some(user_id) = string_value(&document, "userId") else {
                continue;
            };
            let id = format!("profiles:{guild_id}:{user_id}");
            if known_ids.contains(&id) {
                continue;
            }

            records.push(CacheRecord {
                id,
                table: "profiles".to_string(),
                data: legacy_profile_to_cache(&document, &guild_id, &user_id),
            });
        }

        Ok(())
    }

    pub async fn push_snapshot(&self, records: &[CacheRecord]) -> BotResult<usize> {
        let database = self.client.database(&self.database);
        let collection = database.collection::<Document>(CACHE_COLLECTION);
        let mut uploaded = 0;

        for record in records {
            collection
                .update_one(
                    doc! { "_id": &record.id },
                    doc! {
                        "$set": {
                            "table": &record.table,
                            "data": record.data.clone(),
                        }
                    },
                )
                .upsert(true)
                .await?;
            uploaded += 1;

            if record.table == "profiles" {
                self.push_legacy_profile(&database, record).await?;
            }
        }

        Ok(uploaded)
    }

    async fn push_legacy_profile(
        &self,
        database: &mongodb::Database,
        record: &CacheRecord,
    ) -> BotResult<()> {
        let Some(guild_id) = string_value(&record.data, "guild_id") else {
            return Ok(());
        };
        let Some(user_id) = string_value(&record.data, "user_id") else {
            return Ok(());
        };

        database
            .collection::<Document>(LEGACY_PROFILE_COLLECTION)
            .update_one(
                doc! { "guildId": &guild_id, "userId": &user_id },
                doc! { "$set": cache_profile_to_legacy(&record.data) },
            )
            .upsert(true)
            .await?;
        Ok(())
    }
}

fn legacy_profile_to_cache(document: &Document, guild_id: &str, user_id: &str) -> Document {
    let last_xp_at = timestamp_seconds(document.get("lastXpAt"));
    let last_daily_at = timestamp_seconds(document.get("lastDailyAt"));
    let last_work_at = timestamp_seconds(document.get("lastWorkAt"));

    doc! {
        "guild_id": guild_id,
        "user_id": user_id,
        "username": string_value(document, "username").unwrap_or_default(),
        "wallet": number_value(document, "wallet"),
        "bank": number_value(document, "bank"),
        "total_xp": number_value(document, "totalXp"),
        "weekly_xp": number_value(document, "weeklyXp"),
        "monthly_xp": number_value(document, "monthlyXp"),
        "week_key": string_value(document, "weekKey").unwrap_or_default(),
        "month_key": string_value(document, "monthKey").unwrap_or_default(),
        "last_xp_at": last_xp_at,
        "last_daily_at": last_daily_at,
        "last_work_at": last_work_at,
        "afk_message": optional_string_value(document, "afkMessage"),
        "profile_background": "midnight",
        "inventory": legacy_inventory_json(document.get("inventory")),
    }
}

fn cache_profile_to_legacy(data: &Document) -> Document {
    let last_xp_at = timestamp_bson(number_value(data, "last_xp_at"));
    let last_daily_at = timestamp_bson(number_value(data, "last_daily_at"));
    let last_work_at = timestamp_bson(number_value(data, "last_work_at"));

    doc! {
        "guildId": string_value(data, "guild_id").unwrap_or_default(),
        "userId": string_value(data, "user_id").unwrap_or_default(),
        "username": string_value(data, "username").unwrap_or_default(),
        "wallet": number_value(data, "wallet"),
        "bank": number_value(data, "bank"),
        "totalXp": number_value(data, "total_xp"),
        "weeklyXp": number_value(data, "weekly_xp"),
        "monthlyXp": number_value(data, "monthly_xp"),
        "weekKey": string_value(data, "week_key").unwrap_or_default(),
        "monthKey": string_value(data, "month_key").unwrap_or_default(),
        "lastXpAt": last_xp_at,
        "lastDailyAt": last_daily_at,
        "lastWorkAt": last_work_at,
        "afkMessage": optional_string_value(data, "afk_message"),
        "inventory": inventory_bson(data.get("inventory")),
    }
}

fn string_value(document: &Document, key: &str) -> Option<String> {
    match document.get(key) {
        Some(Bson::String(value)) => Some(value.clone()),
        Some(value) => value.as_str().map(str::to_owned),
        None => None,
    }
}

fn optional_string_value(document: &Document, key: &str) -> Bson {
    string_value(document, key)
        .map(Bson::String)
        .unwrap_or(Bson::Null)
}

fn number_value(document: &Document, key: &str) -> i64 {
    match document.get(key) {
        Some(Bson::Int32(value)) => i64::from(*value),
        Some(Bson::Int64(value)) => *value,
        Some(Bson::Double(value)) => *value as i64,
        Some(Bson::Decimal128(value)) => value.to_string().parse::<i64>().unwrap_or_default(),
        _ => 0,
    }
}

fn timestamp_seconds(value: Option<&Bson>) -> i64 {
    match value {
        Some(Bson::DateTime(value)) => value.timestamp_millis() / 1_000,
        Some(Bson::Int64(value)) => *value,
        Some(Bson::Int32(value)) => i64::from(*value),
        Some(Bson::Double(value)) => *value as i64,
        _ => 0,
    }
}

fn timestamp_bson(value: i64) -> Bson {
    if value > 0 {
        Bson::DateTime(DateTime::from_millis(value.saturating_mul(1_000)))
    } else {
        Bson::Null
    }
}

fn legacy_inventory_json(value: Option<&Bson>) -> String {
    let Some(Bson::Array(items)) = value else {
        return "[]".to_string();
    };

    let normalized = items
        .iter()
        .filter_map(|item| {
            let Bson::Document(item) = item else {
                return None;
            };
            Some(serde_json::json!({
                "item_id": string_value(item, "itemId")
                    .or_else(|| string_value(item, "item_id"))
                    .unwrap_or_default(),
                "quantity": number_value(item, "quantity"),
            }))
        })
        .collect::<Vec<_>>();

    serde_json::to_string(&normalized).unwrap_or_else(|_| "[]".to_string())
}

fn inventory_bson(value: Option<&Bson>) -> Bson {
    let Some(Bson::String(value)) = value else {
        return Bson::Array(Vec::new());
    };
    let Ok(items) = serde_json::from_str::<Vec<serde_json::Value>>(value) else {
        return Bson::Array(Vec::new());
    };

    items
        .iter()
        .map(|item| {
            let item_id = item
                .get("item_id")
                .or_else(|| item.get("itemId"))
                .and_then(serde_json::Value::as_str)
                .unwrap_or_default();
            let quantity = item
                .get("quantity")
                .and_then(serde_json::Value::as_i64)
                .unwrap_or_default();
            Bson::Document(doc! { "itemId": item_id, "quantity": quantity })
        })
        .collect::<Vec<_>>()
        .into()
}
