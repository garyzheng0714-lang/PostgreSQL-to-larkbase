//! 连接池管理器：多配置缓存 + TTL 回收 + LRU 淘汰（对应 Python `pool.py`）。
//!
//! ⚠️ deadpool 本身只管「单配置连接池」；多配置缓存/TTL/LRU 由本结构自建。
//! PoolKey 含安全配置 hash（密码/证书），避免改密码后复用旧池（设计 §6.2）。
//! 连接 options 统一设 UTC 时区、statement_timeout、default_transaction_read_only，
//! 把查询超时与只读护栏下沉到 PG（设计 §4.7/§5）。

use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};

use deadpool_postgres::{Manager, ManagerConfig, Object, Pool, RecyclingMethod};
use md5::{Digest, Md5};
use tokio::sync::Mutex;
use tokio_postgres::config::SslMode;

use super::tls::build_tls;
use crate::config::Config;
use crate::protocol::request::DatasourceConfig;
use crate::protocol::ConnectorError;

struct Entry {
    pool: Pool,
    last_used: Instant,
}

/// 多配置连接池管理器。
pub struct PoolManager {
    pools: Mutex<HashMap<String, Entry>>,
    max_pools: usize,
    idle_timeout: Duration,
    max_size: usize,
}

impl PoolManager {
    pub fn new(cfg: &Config) -> Arc<Self> {
        Arc::new(Self {
            pools: Mutex::new(HashMap::new()),
            max_pools: cfg.pool_max_pools,
            idle_timeout: Duration::from_secs(cfg.pool_idle_timeout),
            max_size: cfg.pool_max_size,
        })
    }

    /// 池键：含连接标识 + 安全配置 hash（密码/证书），仅打印脱敏前缀。
    fn pool_key(c: &DatasourceConfig) -> String {
        let mut h = Md5::new();
        h.update(c.password.as_bytes());
        h.update(c.ssl_root_cert.as_deref().unwrap_or("").as_bytes());
        h.update(c.ssl_cert.as_deref().unwrap_or("").as_bytes());
        h.update(c.ssl_key.as_deref().unwrap_or("").as_bytes());
        let sec = hex::encode(h.finalize());
        format!(
            "{}:{}:{}:{}:{}:{}",
            c.host, c.port, c.username, c.database, c.ssl_mode, &sec[..12]
        )
    }

    fn ssl_mode(mode: &str) -> SslMode {
        // disable → 明文；其余一律强制 TLS（require，不回落明文），与 Python oracle
        // 把 allow/prefer 视为 require 一致，避免意外明文连接。
        match mode {
            "disable" => SslMode::Disable,
            _ => SslMode::Require,
        }
    }

    fn build_pool(
        &self,
        c: &DatasourceConfig,
        connect_timeout: u64,
        query_timeout: u64,
    ) -> Result<Pool, ConnectorError> {
        let mut pg = tokio_postgres::Config::new();
        pg.host(&c.host)
            .port(c.port)
            .user(&c.username)
            .password(&c.password)
            .dbname(&c.database)
            .connect_timeout(Duration::from_secs(connect_timeout))
            .ssl_mode(Self::ssl_mode(&c.ssl_mode));
        let stmt_ms = query_timeout.saturating_mul(1000);
        pg.options(format!(
            "-c timezone=UTC -c statement_timeout={stmt_ms} -c default_transaction_read_only=on"
        ));

        let tls = build_tls(c)?;
        let mgr = Manager::from_config(
            pg,
            tls,
            ManagerConfig {
                recycling_method: RecyclingMethod::Fast,
            },
        );
        Pool::builder(mgr)
            .max_size(self.max_size)
            .build()
            .map_err(|e| ConnectorError::ConnectionFailed(format!("pool build: {e}")))
    }

    /// 取一个连接（按需建池，命中则复用并 touch）。
    pub async fn acquire(&self, c: &DatasourceConfig) -> Result<Object, ConnectorError> {
        let key = Self::pool_key(c);
        let connect_timeout = c.connect_timeout.unwrap_or(5);
        let query_timeout = c.query_timeout.unwrap_or(15);

        let pool = {
            let mut map = self.pools.lock().await;
            if let Some(e) = map.get_mut(&key) {
                e.last_used = Instant::now();
                e.pool.clone()
            } else {
                if map.len() >= self.max_pools {
                    if let Some(oldest) = map
                        .iter()
                        .min_by_key(|(_, e)| e.last_used)
                        .map(|(k, _)| k.clone())
                    {
                        if let Some(e) = map.remove(&oldest) {
                            e.pool.close();
                            // 不打印含密码 hash 的 key
                            tracing::info!("evicted oldest connection pool (LRU)");
                        }
                    }
                }
                let pool = self.build_pool(c, connect_timeout, query_timeout)?;
                map.insert(
                    key.clone(),
                    Entry {
                        pool: pool.clone(),
                        last_used: Instant::now(),
                    },
                );
                tracing::info!(host = %c.host, db = %c.database, "created connection pool");
                pool
            }
        };

        // 池等待超时：池耗尽时不无限挂起，超时映射为 QueryTimeout（协议 200）。
        let wait = Duration::from_secs(connect_timeout.saturating_add(2));
        match tokio::time::timeout(wait, pool.get()).await {
            Ok(Ok(obj)) => Ok(obj),
            Ok(Err(e)) => Err(ConnectorError::ConnectionFailed(format!("acquire connection: {e}"))),
            Err(_) => Err(ConnectorError::QueryTimeout("connection pool wait timeout".into())),
        }
    }

    /// 回收空闲超 TTL 的池。
    pub async fn cleanup_idle(&self) {
        let mut map = self.pools.lock().await;
        let now = Instant::now();
        let stale: Vec<String> = map
            .iter()
            .filter(|(_, e)| now.duration_since(e.last_used) > self.idle_timeout)
            .map(|(k, _)| k.clone())
            .collect();
        for k in stale {
            if let Some(e) = map.remove(&k) {
                e.pool.close();
            }
        }
    }

    /// 关闭所有池（优雅退出）。
    pub async fn close_all(&self) {
        let mut map = self.pools.lock().await;
        for (_, e) in map.drain() {
            e.pool.close();
        }
    }

    /// 启动后台清理循环（每 60s）。
    pub fn spawn_cleanup(self: &Arc<Self>) {
        let me = self.clone();
        tokio::spawn(async move {
            let mut tick = tokio::time::interval(Duration::from_secs(60));
            loop {
                tick.tick().await;
                me.cleanup_idle().await;
            }
        });
    }

    /// 当前缓存的池数量（可观测）。
    pub async fn pool_count(&self) -> usize {
        self.pools.lock().await.len()
    }
}
