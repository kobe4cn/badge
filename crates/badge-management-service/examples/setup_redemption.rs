//! 设置兑换测试数据

use sqlx::PgPool;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let database_url = std::env::var("DATABASE_URL")
        .unwrap_or_else(|_| "postgres://badge:badge_secret@localhost:5432/badge_db".to_string());

    let pool = PgPool::connect(&database_url).await?;

    // 1. 创建权益
    let benefit_id: (i64,) = sqlx::query_as(
        r#"
        INSERT INTO benefits (code, name, description, benefit_type, total_stock, remaining_stock, status)
        VALUES ('TEST_COUPON', '测试优惠券', '用于测试的优惠券权益', 'coupon', 100, 100, 'active')
        ON CONFLICT (code) DO UPDATE SET name = EXCLUDED.name
        RETURNING id
        "#,
    )
    .fetch_one(&pool)
    .await?;
    println!("权益 ID: {}", benefit_id.0);

    // 2. 创建兑换规则：1个 badge 1 可以兑换优惠券
    let rule_id: Option<(i64,)> = sqlx::query_as(
        r#"
        INSERT INTO badge_redemption_rules (name, description, benefit_id, required_badges, status)
        VALUES (
            '新人注册兑换优惠券',
            '用新人注册徽章兑换优惠券',
            $1,
            '[{"badgeId": 1, "quantity": 1}]'::jsonb,
            'active'
        )
        ON CONFLICT DO NOTHING
        RETURNING id
        "#,
    )
    .bind(benefit_id.0)
    .fetch_optional(&pool)
    .await?;

    match rule_id {
        Some((id,)) => println!("兑换规则 ID: {}", id),
        None => {
            let existing: (i64,) = sqlx::query_as(
                "SELECT id FROM badge_redemption_rules WHERE name = '新人注册兑换优惠券'"
            )
            .fetch_one(&pool)
            .await?;
            println!("兑换规则已存在, ID: {}", existing.0);
        }
    }

    println!("兑换测试数据设置完成！");
    Ok(())
}
