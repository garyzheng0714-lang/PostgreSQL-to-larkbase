//! 进程入口：初始化日志、安装 TLS provider、装配状态、启动服务、优雅关闭。

use std::sync::Arc;

use fbif_databridge::adapter::postgres::PostgresAdapter;
use fbif_databridge::adapter::registry::Registry;
use fbif_databridge::config::Config;
use fbif_databridge::metadata_cache::MetadataCache;
use fbif_databridge::server::{self, AppState};
use metrics_exporter_prometheus::PrometheusBuilder;
use tokio::signal;
use tracing_subscriber::EnvFilter;

#[tokio::main]
async fn main() {
    // 结构化 JSON 日志；级别由 RUST_LOG 控制，默认 info。
    tracing_subscriber::fmt()
        .json()
        .with_env_filter(
            EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")),
        )
        .init();

    // 安装 rustls 默认加密 provider（ring）。
    let _ = rustls::crypto::ring::default_provider().install_default();

    // 安装 Prometheus 指标记录器。
    let metrics = PrometheusBuilder::new()
        .install_recorder()
        .expect("install prometheus recorder");

    let cfg = Config::from_env();
    if !cfg.is_dev_mode() && cfg.secret_key.is_empty() {
        tracing::warn!("SECRET_KEY 未配置，生产环境请设置强随机值");
    }

    // 装配适配器注册表（PostgreSQL）+ 启动连接池清理循环。
    let pg = PostgresAdapter::new(&cfg);
    pg.pools().spawn_cleanup();
    let mut registry = Registry::new();
    registry.register(Arc::new(pg));

    let bind_addr = cfg.bind_addr.clone();
    let state = AppState {
        cfg,
        registry: Arc::new(registry),
        metrics,
        metadata_cache: Arc::new(MetadataCache::new(std::time::Duration::from_secs(300), 512)),
    };
    let app = server::build_router(state);

    let listener = match tokio::net::TcpListener::bind(&bind_addr).await {
        Ok(l) => l,
        Err(e) => {
            tracing::error!("无法绑定 {bind_addr}: {e}");
            std::process::exit(1);
        }
    };
    tracing::info!("FBIF DataBridge listening on {bind_addr}");

    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal())
        .await
        .unwrap_or_else(|e| tracing::error!("server error: {e}"));
}

/// 等待 SIGTERM / Ctrl-C，触发优雅关闭。
async fn shutdown_signal() {
    let ctrl_c = async {
        let _ = signal::ctrl_c().await;
    };

    #[cfg(unix)]
    let terminate = async {
        if let Ok(mut sig) = signal::unix::signal(signal::unix::SignalKind::terminate()) {
            sig.recv().await;
        }
    };
    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => {},
        _ = terminate => {},
    }
    tracing::info!("收到关闭信号，正在优雅退出…");
}
