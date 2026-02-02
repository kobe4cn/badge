//! 修复 JSON 格式

use sqlx::PgPool;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let pool = PgPool::connect("postgres://badge:badge_secret@localhost:5432/badge_db").await?;

    // 更新 required_badges JSON 格式
    sqlx::query(
        r#"
        UPDATE badge_redemption_rules
        SET required_badges = '[{"badgeId": 1, "quantity": 1}]'::jsonb
        WHERE required_badges::text LIKE '%badge_id%'
    "#,
    )
    .execute(&pool)
    .await?;

    println!("JSON 格式修复完成");
    Ok(())
}
