//! 运行迁移脚本

use sqlx::PgPool;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let database_url = std::env::var("DATABASE_URL")
        .unwrap_or_else(|_| "postgres://badge:badge_secret@localhost:5432/badge_db".to_string());

    let pool = PgPool::connect(&database_url).await?;

    // 创建 user_badge_logs 表
    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS user_badge_logs (
            id BIGSERIAL PRIMARY KEY,
            user_badge_id BIGINT REFERENCES user_badges(id),
            user_id VARCHAR(100) NOT NULL,
            badge_id BIGINT NOT NULL REFERENCES badges(id),
            action VARCHAR(20) NOT NULL,
            reason TEXT,
            operator VARCHAR(100),
            quantity INT NOT NULL DEFAULT 1,
            source_type VARCHAR(20) NOT NULL,
            source_ref_id VARCHAR(200),
            remark TEXT,
            created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
        )
    "#,
    )
    .execute(&pool)
    .await?;

    println!("表 user_badge_logs 创建成功");

    // 创建索引
    sqlx::query("CREATE INDEX IF NOT EXISTS idx_user_badge_logs_user ON user_badge_logs(user_id)")
        .execute(&pool)
        .await?;
    sqlx::query(
        "CREATE INDEX IF NOT EXISTS idx_user_badge_logs_badge ON user_badge_logs(badge_id)",
    )
    .execute(&pool)
    .await?;
    sqlx::query("CREATE INDEX IF NOT EXISTS idx_user_badge_logs_user_badge ON user_badge_logs(user_badge_id)")
        .execute(&pool).await?;
    sqlx::query("CREATE INDEX IF NOT EXISTS idx_user_badge_logs_action ON user_badge_logs(action)")
        .execute(&pool)
        .await?;
    sqlx::query(
        "CREATE INDEX IF NOT EXISTS idx_user_badge_logs_time ON user_badge_logs(created_at)",
    )
    .execute(&pool)
    .await?;

    println!("索引创建成功");

    Ok(())
}
