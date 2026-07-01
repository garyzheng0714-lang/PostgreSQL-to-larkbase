//! 短 TTL 元数据缓存：避免每个 records 分页重复查询 information_schema 和主键。

use std::collections::HashMap;
use std::time::{Duration, Instant};

use md5::{Digest, Md5};
use tokio::sync::Mutex;

use crate::adapter::ColumnInfo;
use crate::protocol::request::DatasourceConfig;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct MetadataCacheKey(String);

#[derive(Debug, Clone)]
pub struct CachedMetadata {
    pub columns: Vec<ColumnInfo>,
    pub pk_columns: Vec<String>,
}

struct CacheEntry {
    value: CachedMetadata,
    inserted_at: Instant,
}

pub struct MetadataCache {
    entries: Mutex<HashMap<MetadataCacheKey, CacheEntry>>,
    ttl: Duration,
    max_entries: usize,
}

impl MetadataCache {
    pub fn new(ttl: Duration, max_entries: usize) -> Self {
        Self {
            entries: Mutex::new(HashMap::new()),
            ttl,
            max_entries,
        }
    }

    pub async fn get(&self, key: &MetadataCacheKey) -> Option<CachedMetadata> {
        let mut entries = self.entries.lock().await;
        match entries.get(key) {
            Some(entry) if entry.inserted_at.elapsed() <= self.ttl => Some(entry.value.clone()),
            Some(_) => {
                entries.remove(key);
                None
            }
            None => None,
        }
    }

    pub async fn insert(&self, key: MetadataCacheKey, value: CachedMetadata) {
        let mut entries = self.entries.lock().await;
        if entries.len() >= self.max_entries {
            if let Some(oldest) = entries
                .iter()
                .min_by_key(|(_, entry)| entry.inserted_at)
                .map(|(k, _)| k.clone())
            {
                entries.remove(&oldest);
            }
        }
        entries.insert(
            key,
            CacheEntry {
                value,
                inserted_at: Instant::now(),
            },
        );
    }
}

fn security_hash(c: &DatasourceConfig) -> String {
    let mut h = Md5::new();
    h.update(c.password.as_bytes());
    h.update(c.ssl_root_cert.as_deref().unwrap_or("").as_bytes());
    h.update(c.ssl_cert.as_deref().unwrap_or("").as_bytes());
    h.update(c.ssl_key.as_deref().unwrap_or("").as_bytes());
    hex::encode(h.finalize())
}

pub fn metadata_cache_key(c: &DatasourceConfig) -> MetadataCacheKey {
    let sec = security_hash(c);
    let custom_sql = c.custom_sql.as_deref().unwrap_or("");
    MetadataCacheKey(format!(
        "{}:{}:{}:{}:{}:{}:{}:{}:{}",
        c.host,
        c.port,
        c.username,
        c.database,
        c.ssl_mode,
        &sec[..12],
        c.mode,
        c.schema_name,
        c.table_name.as_deref().unwrap_or(custom_sql)
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cfg(password: &str) -> DatasourceConfig {
        DatasourceConfig {
            host: "db.example.com".into(),
            username: "u".into(),
            password: password.into(),
            database: "d".into(),
            table_name: Some("articles".into()),
            ..Default::default()
        }
    }

    #[test]
    fn key_does_not_contain_plaintext_password() {
        let key = metadata_cache_key(&cfg("super-secret"));
        assert!(!key.0.contains("super-secret"));
    }

    #[test]
    fn key_changes_when_security_material_changes() {
        assert_ne!(
            metadata_cache_key(&cfg("one")),
            metadata_cache_key(&cfg("two"))
        );
    }
}
