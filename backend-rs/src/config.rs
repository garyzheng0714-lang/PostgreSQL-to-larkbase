//! 应用配置，从环境变量加载（对应 Python `config.py`）。

use std::env;

/// 全局配置。所有字段都有合理默认值，可被同名大写环境变量覆盖。
#[derive(Debug, Clone)]
pub struct Config {
    /// 飞书请求验签密钥。默认 `testBase` 即开发模式。
    pub secret_key: String,
    /// 前端配置页 URL（写入 meta.json，会追加防 CDN 缓存的 `?v=`）。
    pub frontend_url: String,
    /// 服务监听地址。
    pub bind_addr: String,
    /// PostgreSQL 连接超时（秒）。
    pub pg_connect_timeout: u64,
    /// PostgreSQL 查询超时（秒）。
    pub pg_query_timeout: u64,
    /// 单次同步最大行数（企业版 50000）。
    pub max_row_limit: i64,
    /// 前端辅助接口 API key。为空时 helper 路由 fail-closed（除非 dev 模式）。
    pub helper_api_key: String,
    /// 单配置连接池最大连接数。
    pub pool_max_size: usize,
    /// 连接池空闲回收超时（秒）。
    pub pool_idle_timeout: u64,
    /// 多配置池缓存上限（超出按 LRU 淘汰）。
    pub pool_max_pools: usize,
}

fn env_or(key: &str, default: &str) -> String {
    env::var(key).unwrap_or_else(|_| default.to_string())
}

fn env_parse<T: std::str::FromStr>(key: &str, default: T) -> T {
    env::var(key).ok().and_then(|v| v.parse().ok()).unwrap_or(default)
}

impl Config {
    /// 从进程环境变量构建配置。
    pub fn from_env() -> Self {
        Self {
            secret_key: env_or("SECRET_KEY", "testBase"),
            frontend_url: env_or("FRONTEND_URL", "http://localhost:5173"),
            bind_addr: env_or("BIND_ADDR", "0.0.0.0:8000"),
            pg_connect_timeout: env_parse("PG_CONNECT_TIMEOUT", 5),
            pg_query_timeout: env_parse("PG_QUERY_TIMEOUT", 15),
            max_row_limit: env_parse("MAX_ROW_LIMIT", 50_000),
            helper_api_key: env_or("HELPER_API_KEY", ""),
            pool_max_size: env_parse("POOL_MAX_SIZE", 5),
            pool_idle_timeout: env_parse("POOL_IDLE_TIMEOUT", 300),
            pool_max_pools: env_parse("POOL_MAX_POOLS", 20),
        }
    }

    /// 开发模式：密钥未配置（仍为占位 `testBase`）。
    pub fn is_dev_mode(&self) -> bool {
        self.secret_key == "testBase"
    }
}
