//! 修复 NULL config 列

use sqlx::PgPool;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let database_url = std::env::var("DATABASE_URL")
        .unwrap_or_else(|_| "postgres://badge:badge_secret@localhost:5432/badge_db".to_string());

    let pool = PgPool::connect(&database_url).await?;

    // 更新 benefits 表中 NULL 的 config
    sqlx::query("UPDATE benefits SET config = '{}'::jsonb WHERE config IS NULL")
        .execute(&pool)
        .await?;
    println!("修复 benefits.config NULL 值");

    // 更新 badge_redemption_rules 表中 NULL 的 frequency_config
    sqlx::query("UPDATE badge_redemption_rules SET frequency_config = '{}'::jsonb WHERE frequency_config IS NULL")
        .execute(&pool)
        .await?;
    println!("修复 badge_redemption_rules.frequency_config NULL 值");

    println!("NULL 值修复完成！");
    Ok(())
}
