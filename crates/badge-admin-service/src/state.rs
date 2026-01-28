//! 应用状态定义
//!
//! 包含 Axum 路由共享的应用状态

use badge_shared::cache::Cache;
use sqlx::PgPool;
use std::sync::Arc;

/// Axum 应用共享状态
///
/// 包含数据库连接池和缓存客户端，通过 Arc 在 handler 间共享
#[derive(Clone)]
pub struct AppState {
    /// PostgreSQL 连接池
    pub pool: PgPool,
    /// Redis 缓存客户端
    pub cache: Arc<Cache>,
}

impl AppState {
    /// 创建新的应用状态
    pub fn new(pool: PgPool, cache: Arc<Cache>) -> Self {
        Self { pool, cache }
    }
}
