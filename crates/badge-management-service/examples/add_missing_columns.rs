//! 添加缺失的数据库列

use sqlx::PgPool;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let database_url = std::env::var("DATABASE_URL")
        .unwrap_or_else(|_| "postgres://badge:badge_secret@localhost:5432/badge_db".to_string());

    let pool = PgPool::connect(&database_url).await?;

    // 为 redemption_orders 添加缺失的列
    sqlx::query("ALTER TABLE redemption_orders ADD COLUMN IF NOT EXISTS benefit_result JSONB")
        .execute(&pool)
        .await?;
    println!("添加 benefit_result 列");

    sqlx::query("ALTER TABLE redemption_orders ADD COLUMN IF NOT EXISTS idempotency_key VARCHAR(200)")
        .execute(&pool)
        .await?;
    println!("添加 idempotency_key 列");

    // 为 idempotency_key 创建唯一索引
    sqlx::query("CREATE UNIQUE INDEX IF NOT EXISTS idx_redemption_orders_idempotency ON redemption_orders(idempotency_key) WHERE idempotency_key IS NOT NULL")
        .execute(&pool)
        .await?;
    println!("创建 idempotency_key 索引");

    // 为 badge_redemption_rules 添加缺失列
    sqlx::query("ALTER TABLE badge_redemption_rules ADD COLUMN IF NOT EXISTS frequency_config JSONB DEFAULT '{}'::jsonb")
        .execute(&pool)
        .await?;
    println!("添加 frequency_config 列");

    sqlx::query("ALTER TABLE badge_redemption_rules ADD COLUMN IF NOT EXISTS start_time TIMESTAMPTZ")
        .execute(&pool)
        .await?;
    println!("添加 start_time 列");

    sqlx::query("ALTER TABLE badge_redemption_rules ADD COLUMN IF NOT EXISTS end_time TIMESTAMPTZ")
        .execute(&pool)
        .await?;
    println!("添加 end_time 列");

    sqlx::query("ALTER TABLE badge_redemption_rules ADD COLUMN IF NOT EXISTS enabled BOOLEAN DEFAULT true")
        .execute(&pool)
        .await?;
    println!("添加 enabled 列");

    // benefits 表添加缺失列
    sqlx::query("ALTER TABLE benefits ADD COLUMN IF NOT EXISTS icon_url TEXT")
        .execute(&pool)
        .await?;
    sqlx::query("ALTER TABLE benefits ADD COLUMN IF NOT EXISTS redeemed_count BIGINT DEFAULT 0")
        .execute(&pool)
        .await?;
    sqlx::query("ALTER TABLE benefits ADD COLUMN IF NOT EXISTS enabled BOOLEAN DEFAULT true")
        .execute(&pool)
        .await?;
    sqlx::query("ALTER TABLE benefits ADD COLUMN IF NOT EXISTS config JSONB DEFAULT '{}'::jsonb")
        .execute(&pool)
        .await?;
    println!("添加 benefits 缺失列");

    println!("数据库列添加完成！");
    Ok(())
}
