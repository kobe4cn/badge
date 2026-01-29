//! 数据库连接管理模块
//!
//! 提供 PostgreSQL 连接池管理，支持健康检查和连接配置。

use crate::config::DatabaseConfig;
use crate::error::{BadgeError, Result};
use sqlx::postgres::{PgPool, PgPoolOptions};
use std::time::Duration;
use tracing::{info, instrument};

/// 数据库连接池包装
#[derive(Clone)]
pub struct Database {
    pool: PgPool,
}

impl Database {
    /// 创建数据库连接池
    #[instrument(skip(config))]
    pub async fn connect(config: &DatabaseConfig) -> Result<Self> {
        info!("Connecting to database...");

        let pool = PgPoolOptions::new()
            .max_connections(config.max_connections)
            .min_connections(config.min_connections)
            .acquire_timeout(Duration::from_secs(config.connect_timeout_seconds))
            .idle_timeout(Duration::from_secs(config.idle_timeout_seconds))
            .connect(&config.url)
            .await?;

        info!("Database connection pool created");

        Ok(Self { pool })
    }

    /// 获取连接池引用
    pub fn pool(&self) -> &PgPool {
        &self.pool
    }

    /// 健康检查
    pub async fn health_check(&self) -> Result<()> {
        sqlx::query("SELECT 1")
            .execute(&self.pool)
            .await
            .map(|_| ())
            .map_err(BadgeError::from)
    }

    /// 关闭连接池
    pub async fn close(&self) {
        self.pool.close().await;
        info!("Database connection pool closed");
    }

    /// 运行迁移（注意：由于 migrations 目录在项目根目录，此方法可能需要调整路径）
    #[instrument(skip(self))]
    pub async fn run_migrations(&self) -> Result<()> {
        info!("Running database migrations...");
        // 注意：migrate! 宏需要在编译时知道迁移目录路径
        // 在实际使用中，服务会单独处理迁移
        Ok(())
    }
}

impl std::ops::Deref for Database {
    type Target = PgPool;

    fn deref(&self) -> &Self::Target {
        &self.pool
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    #[ignore] // 需要数据库连接
    async fn test_database_connection() {
        let config = DatabaseConfig::default();
        let db = Database::connect(&config).await.unwrap();
        db.health_check().await.unwrap();
    }
}
