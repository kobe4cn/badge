//! 徽章管理后台服务（B端）
//!
//! 提供徽章配置、发放管理、统计报表等 REST API。

use std::sync::Arc;

use badge_admin_service::{routes, state::AppState};
use badge_shared::{cache::Cache, config::AppConfig, database::Database};
use tokio::net::TcpListener;
use tower_http::trace::TraceLayer;
use tracing::info;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt::init();

    let config = AppConfig::load("badge-admin-service").unwrap_or_default();
    info!("Starting badge-admin-service on {}", config.server_addr());

    // 初始化基础设施
    let db = Database::connect(&config.database).await?;
    let cache = Arc::new(Cache::new(&config.redis)?);

    let state = AppState::new(db.pool().clone(), cache);

    let app = routes::api_routes()
        .with_state(state)
        .layer(TraceLayer::new_for_http());

    let listener = TcpListener::bind(config.server_addr()).await?;
    info!("Listening on {}", config.server_addr());

    axum::serve(listener, app).await?;

    Ok(())
}
