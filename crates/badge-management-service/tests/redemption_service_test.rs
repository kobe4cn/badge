//! RedemptionService 集成测试
//!
//! 使用真实 PostgreSQL 和 Redis 测试兑换服务的完整业务流程。
//! RedemptionService 内部通过 sqlx 直接操作数据库（13 步事务流程），
//! 无法通过纯 mock 覆盖，因此需要集成测试。
//!
//! ## 运行方式
//!
//! ```bash
//! DATABASE_URL=postgres://... REDIS_URL=redis://... \
//!   cargo test --test redemption_service_test -- --ignored
//! ```

use badge_management::error::BadgeError;
use badge_management::repository::RedemptionRepository;
use badge_management::service::dto::RedeemBadgeRequest;
use badge_management::service::RedemptionService;
use badge_shared::cache::Cache;
use badge_shared::config::RedisConfig;
use chrono::{Duration, Utc};
use serde_json::json;
use sqlx::PgPool;
use std::sync::Arc;

// ==================== 辅助函数 ====================

fn database_url() -> String {
    std::env::var("DATABASE_URL").expect("DATABASE_URL must be set for integration tests")
}

fn redis_url() -> String {
    std::env::var("REDIS_URL").unwrap_or_else(|_| "redis://localhost:6379".to_string())
}

/// 构建 RedemptionService 实例（使用真实 RedemptionRepository + Cache）
async fn setup_redemption_service(pool: &PgPool) -> RedemptionService {
    let redemption_repo = Arc::new(RedemptionRepository::new(pool.clone()));
    let redis_config = RedisConfig {
        url: redis_url(),
        pool_size: 2,
    };
    let cache = Arc::new(Cache::new(&redis_config).expect("Redis connection failed"));
    RedemptionService::new(redemption_repo, cache, pool.clone())
}

/// 插入测试用分类和系列（幂等）
async fn ensure_category_and_series(pool: &PgPool) {
    sqlx::query(
        r#"
        INSERT INTO badge_categories (id, name, status, sort_order)
        VALUES (99900, 'RedemptionTest Category', 'active', 0)
        ON CONFLICT (id) DO UPDATE SET name = EXCLUDED.name
        "#,
    )
    .execute(pool)
    .await
    .expect("插入测试分类失败");

    sqlx::query(
        r#"
        INSERT INTO badge_series (id, category_id, name, status, sort_order)
        VALUES (99900, 99900, 'RedemptionTest Series', 'active', 0)
        ON CONFLICT (id) DO UPDATE SET name = EXCLUDED.name
        "#,
    )
    .execute(pool)
    .await
    .expect("插入测试系列失败");
}

/// 插入测试徽章
async fn seed_test_badge(pool: &PgPool, badge_id: i64, name: &str) {
    ensure_category_and_series(pool).await;

    sqlx::query(
        r#"
        INSERT INTO badges (id, series_id, badge_type, name, status, assets, validity_config,
                            max_supply, issued_count, sort_order)
        VALUES ($1, 99900, 'normal', $2, 'active',
                '{"iconUrl":"https://test.com/icon.png"}',
                '{"validityType":"PERMANENT"}',
                NULL, 0, 0)
        ON CONFLICT (id) DO UPDATE SET name = EXCLUDED.name
        "#,
    )
    .bind(badge_id)
    .bind(name)
    .execute(pool)
    .await
    .expect("插入测试徽章失败");
}

/// 插入测试权益
///
/// remaining_stock 为 None 表示不限量
async fn seed_test_benefit(
    pool: &PgPool,
    benefit_id: i64,
    code: &str,
    name: &str,
    total_stock: Option<i64>,
    remaining_stock: Option<i64>,
    enabled: bool,
) {
    sqlx::query(
        r#"
        INSERT INTO benefits (id, code, name, benefit_type, total_stock, remaining_stock,
                              status, config, redeemed_count, enabled)
        VALUES ($1, $2, $3, 'coupon', $4, $5, 'active',
                '{"couponId":"test-coupon"}', 0, $6)
        ON CONFLICT (id) DO UPDATE SET
            code = EXCLUDED.code,
            name = EXCLUDED.name,
            total_stock = EXCLUDED.total_stock,
            remaining_stock = EXCLUDED.remaining_stock,
            redeemed_count = 0,
            enabled = EXCLUDED.enabled
        "#,
    )
    .bind(benefit_id)
    .bind(code)
    .bind(name)
    .bind(total_stock)
    .bind(remaining_stock)
    .bind(enabled)
    .execute(pool)
    .await
    .expect("插入测试权益失败");
}

/// 插入兑换规则
///
/// required_badges 格式示例：`[{"badgeId": 92001, "quantity": 2}]`
async fn seed_redemption_rule(
    pool: &PgPool,
    rule_id: i64,
    name: &str,
    benefit_id: i64,
    required_badges: serde_json::Value,
    enabled: bool,
    start_time: Option<chrono::DateTime<Utc>>,
    end_time: Option<chrono::DateTime<Utc>>,
) {
    sqlx::query(
        r#"
        INSERT INTO badge_redemption_rules
            (id, name, benefit_id, required_badges, frequency_config, enabled, start_time, end_time, status)
        VALUES ($1, $2, $3, $4, '{}', $5, $6, $7, 'active')
        ON CONFLICT (id) DO UPDATE SET
            name = EXCLUDED.name,
            benefit_id = EXCLUDED.benefit_id,
            required_badges = EXCLUDED.required_badges,
            enabled = EXCLUDED.enabled,
            start_time = EXCLUDED.start_time,
            end_time = EXCLUDED.end_time
        "#,
    )
    .bind(rule_id)
    .bind(name)
    .bind(benefit_id)
    .bind(required_badges)
    .bind(enabled)
    .bind(start_time)
    .bind(end_time)
    .execute(pool)
    .await
    .expect("插入兑换规则失败");
}

/// 给用户直接插入一条 user_badges 记录（跳过业务逻辑，用于准备前置数据）
async fn seed_user_badge(pool: &PgPool, user_id: &str, badge_id: i64, quantity: i32) {
    sqlx::query(
        r#"
        INSERT INTO user_badges (user_id, badge_id, quantity, status, source_type)
        VALUES ($1, $2, $3, 'active', 'MANUAL')
        ON CONFLICT (user_id, badge_id) DO UPDATE SET quantity = $3, status = 'active'
        "#,
    )
    .bind(user_id)
    .bind(badge_id)
    .bind(quantity)
    .execute(pool)
    .await
    .expect("插入用户徽章失败");
}

/// 清理测试数据，按外键依赖顺序删除
async fn cleanup_test_data(pool: &PgPool, user_ids: &[&str], rule_ids: &[i64], benefit_ids: &[i64], badge_ids: &[i64]) {
    // 1. 先删除兑换明细（引用 redemption_orders 和 user_badges）
    for uid in user_ids {
        sqlx::query(
            "DELETE FROM redemption_details WHERE order_id IN (SELECT id FROM redemption_orders WHERE user_id = $1)",
        )
        .bind(uid)
        .execute(pool)
        .await
        .ok();
    }

    // 2. 删除兑换订单
    for uid in user_ids {
        sqlx::query("DELETE FROM redemption_orders WHERE user_id = $1")
            .bind(uid)
            .execute(pool)
            .await
            .ok();
    }

    // 3. 删除用户徽章日志
    for uid in user_ids {
        sqlx::query("DELETE FROM user_badge_logs WHERE user_id = $1")
            .bind(uid)
            .execute(pool)
            .await
            .ok();
    }

    // 4. 删除账本流水
    for uid in user_ids {
        sqlx::query("DELETE FROM badge_ledger WHERE user_id = $1")
            .bind(uid)
            .execute(pool)
            .await
            .ok();
    }

    // 5. 删除用户徽章
    for uid in user_ids {
        sqlx::query("DELETE FROM user_badges WHERE user_id = $1")
            .bind(uid)
            .execute(pool)
            .await
            .ok();
    }

    // 6. 删除兑换规则（引用 benefits）
    for rid in rule_ids {
        sqlx::query("DELETE FROM badge_redemption_rules WHERE id = $1")
            .bind(rid)
            .execute(pool)
            .await
            .ok();
    }

    // 7. 删除 benefit_grants（引用 benefits）
    for bid in benefit_ids {
        sqlx::query("DELETE FROM benefit_grants WHERE benefit_id = $1")
            .bind(bid)
            .execute(pool)
            .await
            .ok();
    }

    // 8. 删除权益
    for bid in benefit_ids {
        sqlx::query("DELETE FROM benefits WHERE id = $1")
            .bind(bid)
            .execute(pool)
            .await
            .ok();
    }

    // 9. 删除徽章
    for bid in badge_ids {
        sqlx::query("DELETE FROM badges WHERE id = $1")
            .bind(bid)
            .execute(pool)
            .await
            .ok();
    }
}

/// 查询用户徽章的当前数量
async fn get_user_badge_quantity(pool: &PgPool, user_id: &str, badge_id: i64) -> Option<i32> {
    sqlx::query_scalar::<_, i32>(
        "SELECT quantity FROM user_badges WHERE user_id = $1 AND badge_id = $2",
    )
    .bind(user_id)
    .bind(badge_id)
    .fetch_optional(pool)
    .await
    .expect("查询用户徽章数量失败")
}

/// 查询用户徽章的当前状态
async fn get_user_badge_status(pool: &PgPool, user_id: &str, badge_id: i64) -> Option<String> {
    sqlx::query_scalar::<_, String>(
        "SELECT status FROM user_badges WHERE user_id = $1 AND badge_id = $2",
    )
    .bind(user_id)
    .bind(badge_id)
    .fetch_optional(pool)
    .await
    .expect("查询用户徽章状态失败")
}

// ==================== 测试用例 ====================

/// 单徽章兑换成功：验证订单创建、徽章扣减、账本流水等完整流程
#[tokio::test]
#[ignore = "需要 PostgreSQL 和 Redis"]
async fn test_redeem_badge_success() {
    let pool = PgPool::connect(&database_url()).await.unwrap();
    let badge_id = 92001;
    let benefit_id = 92001;
    let rule_id = 92001;
    let user_id = "integ_redeem_success_001";

    cleanup_test_data(&pool, &[user_id], &[rule_id], &[benefit_id], &[badge_id]).await;

    // 准备测试数据
    seed_test_badge(&pool, badge_id, "Redeem Success Badge").await;
    seed_test_benefit(&pool, benefit_id, "BEN_92001", "测试优惠券", Some(100), Some(100), true).await;
    seed_redemption_rule(
        &pool, rule_id, "单徽章兑换规则", benefit_id,
        json!([{"badgeId": badge_id, "quantity": 2}]),
        true, None, None,
    ).await;
    seed_user_badge(&pool, user_id, badge_id, 5).await;

    let svc = setup_redemption_service(&pool).await;
    let request = RedeemBadgeRequest::new(user_id, rule_id, "idem-redeem-92001");
    let resp = svc.redeem_badge(request).await;

    assert!(resp.is_ok(), "兑换应成功: {:?}", resp.err());
    let resp = resp.unwrap();
    assert!(resp.success);
    assert!(!resp.order_no.is_empty());
    assert_eq!(resp.benefit_name, "测试优惠券");

    // 验证徽章扣减：5 - 2 = 3
    let qty = get_user_badge_quantity(&pool, user_id, badge_id).await;
    assert_eq!(qty, Some(3), "兑换后徽章数量应为 3");

    // 验证账本有 REDEEM_OUT 流水
    let ledger_count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM badge_ledger WHERE user_id = $1 AND badge_id = $2 AND change_type = 'REDEEM_OUT'",
    )
    .bind(user_id)
    .bind(badge_id)
    .fetch_one(&pool)
    .await
    .unwrap();
    assert!(ledger_count > 0, "应有 REDEEM_OUT 账本记录");

    cleanup_test_data(&pool, &[user_id], &[rule_id], &[benefit_id], &[badge_id]).await;
}

/// 多种徽章组合兑换：规则要求多种徽章同时消耗
#[tokio::test]
#[ignore = "需要 PostgreSQL 和 Redis"]
async fn test_redeem_multi_badge() {
    let pool = PgPool::connect(&database_url()).await.unwrap();
    let badge_a = 92002;
    let badge_b = 92003;
    let benefit_id = 92002;
    let rule_id = 92002;
    let user_id = "integ_redeem_multi_001";

    cleanup_test_data(&pool, &[user_id], &[rule_id], &[benefit_id], &[badge_a, badge_b]).await;

    seed_test_badge(&pool, badge_a, "Multi Badge A").await;
    seed_test_badge(&pool, badge_b, "Multi Badge B").await;
    seed_test_benefit(&pool, benefit_id, "BEN_92002", "组合权益", Some(50), Some(50), true).await;
    seed_redemption_rule(
        &pool, rule_id, "多徽章组合兑换", benefit_id,
        json!([{"badgeId": badge_a, "quantity": 1}, {"badgeId": badge_b, "quantity": 2}]),
        true, None, None,
    ).await;
    // 给用户持有两种徽章
    seed_user_badge(&pool, user_id, badge_a, 3).await;
    seed_user_badge(&pool, user_id, badge_b, 5).await;

    let svc = setup_redemption_service(&pool).await;
    let request = RedeemBadgeRequest::new(user_id, rule_id, "idem-redeem-multi-92002");
    let resp = svc.redeem_badge(request).await;

    assert!(resp.is_ok(), "多徽章兑换应成功: {:?}", resp.err());
    let resp = resp.unwrap();
    assert!(resp.success);

    // 验证两种徽章分别扣减
    let qty_a = get_user_badge_quantity(&pool, user_id, badge_a).await;
    assert_eq!(qty_a, Some(2), "Badge A 应从 3 扣减到 2");

    let qty_b = get_user_badge_quantity(&pool, user_id, badge_b).await;
    assert_eq!(qty_b, Some(3), "Badge B 应从 5 扣减到 3");

    cleanup_test_data(&pool, &[user_id], &[rule_id], &[benefit_id], &[badge_a, badge_b]).await;
}

/// 幂等测试：相同 idempotency_key 重复请求应返回已存在的订单
#[tokio::test]
#[ignore = "需要 PostgreSQL 和 Redis"]
async fn test_redeem_idempotent() {
    let pool = PgPool::connect(&database_url()).await.unwrap();
    let badge_id = 92004;
    let benefit_id = 92004;
    let rule_id = 92004;
    let user_id = "integ_redeem_idem_001";

    cleanup_test_data(&pool, &[user_id], &[rule_id], &[benefit_id], &[badge_id]).await;

    seed_test_badge(&pool, badge_id, "Idempotent Badge").await;
    seed_test_benefit(&pool, benefit_id, "BEN_92004", "幂等权益", Some(100), Some(100), true).await;
    seed_redemption_rule(
        &pool, rule_id, "幂等测试规则", benefit_id,
        json!([{"badgeId": badge_id, "quantity": 1}]),
        true, None, None,
    ).await;
    seed_user_badge(&pool, user_id, badge_id, 10).await;

    let svc = setup_redemption_service(&pool).await;
    let idem_key = "idem-redeem-dup-92004";

    // 第一次兑换
    let r1 = svc.redeem_badge(RedeemBadgeRequest::new(user_id, rule_id, idem_key)).await.unwrap();
    assert!(r1.success);
    let first_order_id = r1.order_id;

    // 第二次相同 key 兑换，应返回已存在的订单而非重复扣减
    let r2 = svc.redeem_badge(RedeemBadgeRequest::new(user_id, rule_id, idem_key)).await.unwrap();
    assert!(r2.success);
    assert_eq!(r2.order_id, first_order_id, "幂等请求应返回相同的订单 ID");
    assert!(
        r2.message.contains("已存在"),
        "幂等返回的 message 应包含「已存在」，实际: {}",
        r2.message,
    );

    // 验证徽章只被扣减一次：10 - 1 = 9
    let qty = get_user_badge_quantity(&pool, user_id, badge_id).await;
    assert_eq!(qty, Some(9), "幂等请求不应导致重复扣减");

    cleanup_test_data(&pool, &[user_id], &[rule_id], &[benefit_id], &[badge_id]).await;
}

/// 规则不存在：使用不存在的 rule_id 应返回 RedemptionRuleNotFound
#[tokio::test]
#[ignore = "需要 PostgreSQL 和 Redis"]
async fn test_redeem_rule_not_found() {
    let pool = PgPool::connect(&database_url()).await.unwrap();
    let user_id = "integ_redeem_norule_001";
    let nonexistent_rule_id = 999999;

    let svc = setup_redemption_service(&pool).await;
    let result = svc
        .redeem_badge(RedeemBadgeRequest::new(user_id, nonexistent_rule_id, "idem-norule-92005"))
        .await;

    assert!(result.is_err());
    assert!(
        matches!(result.unwrap_err(), BadgeError::RedemptionRuleNotFound(id) if id == nonexistent_rule_id),
        "应返回 RedemptionRuleNotFound",
    );
}

/// 规则未启用：enabled=false 的规则应返回 RedemptionRuleInactive
#[tokio::test]
#[ignore = "需要 PostgreSQL 和 Redis"]
async fn test_redeem_rule_inactive() {
    let pool = PgPool::connect(&database_url()).await.unwrap();
    let badge_id = 92006;
    let benefit_id = 92006;
    let rule_id = 92006;
    let user_id = "integ_redeem_inactive_001";

    cleanup_test_data(&pool, &[user_id], &[rule_id], &[benefit_id], &[badge_id]).await;

    seed_test_badge(&pool, badge_id, "Inactive Rule Badge").await;
    seed_test_benefit(&pool, benefit_id, "BEN_92006", "禁用权益", Some(100), Some(100), true).await;
    // enabled=false：规则被禁用
    seed_redemption_rule(
        &pool, rule_id, "禁用规则", benefit_id,
        json!([{"badgeId": badge_id, "quantity": 1}]),
        false, None, None,
    ).await;

    let svc = setup_redemption_service(&pool).await;
    let result = svc
        .redeem_badge(RedeemBadgeRequest::new(user_id, rule_id, "idem-inactive-92006"))
        .await;

    assert!(result.is_err());
    assert!(
        matches!(result.unwrap_err(), BadgeError::RedemptionRuleInactive(id) if id == rule_id),
        "禁用规则应返回 RedemptionRuleInactive",
    );

    cleanup_test_data(&pool, &[user_id], &[rule_id], &[benefit_id], &[badge_id]).await;
}

/// 规则过期：超出 end_time 的规则应返回 RedemptionRuleInactive
#[tokio::test]
#[ignore = "需要 PostgreSQL 和 Redis"]
async fn test_redeem_rule_expired() {
    let pool = PgPool::connect(&database_url()).await.unwrap();
    let badge_id = 92007;
    let benefit_id = 92007;
    let rule_id = 92007;
    let user_id = "integ_redeem_expired_001";

    cleanup_test_data(&pool, &[user_id], &[rule_id], &[benefit_id], &[badge_id]).await;

    seed_test_badge(&pool, badge_id, "Expired Rule Badge").await;
    seed_test_benefit(&pool, benefit_id, "BEN_92007", "过期权益", Some(100), Some(100), true).await;
    // end_time 设为昨天，规则已过期
    let yesterday = Utc::now() - Duration::days(1);
    seed_redemption_rule(
        &pool, rule_id, "已过期规则", benefit_id,
        json!([{"badgeId": badge_id, "quantity": 1}]),
        true, None, Some(yesterday),
    ).await;

    let svc = setup_redemption_service(&pool).await;
    let result = svc
        .redeem_badge(RedeemBadgeRequest::new(user_id, rule_id, "idem-expired-92007"))
        .await;

    assert!(result.is_err());
    // is_active() 在 enabled=true 但超出时间范围时返回 false，触发 RedemptionRuleInactive
    assert!(
        matches!(result.unwrap_err(), BadgeError::RedemptionRuleInactive(id) if id == rule_id),
        "过期规则应返回 RedemptionRuleInactive",
    );

    cleanup_test_data(&pool, &[user_id], &[rule_id], &[benefit_id], &[badge_id]).await;
}

/// 权益库存不足：remaining_stock=0 应返回 BenefitOutOfStock
#[tokio::test]
#[ignore = "需要 PostgreSQL 和 Redis"]
async fn test_redeem_benefit_out_of_stock() {
    let pool = PgPool::connect(&database_url()).await.unwrap();
    let badge_id = 92008;
    let benefit_id = 92008;
    let rule_id = 92008;
    let user_id = "integ_redeem_oos_001";

    cleanup_test_data(&pool, &[user_id], &[rule_id], &[benefit_id], &[badge_id]).await;

    seed_test_badge(&pool, badge_id, "OOS Benefit Badge").await;
    // remaining_stock=0：库存已耗尽
    seed_test_benefit(&pool, benefit_id, "BEN_92008", "无库存权益", Some(10), Some(0), true).await;
    seed_redemption_rule(
        &pool, rule_id, "无库存兑换规则", benefit_id,
        json!([{"badgeId": badge_id, "quantity": 1}]),
        true, None, None,
    ).await;
    seed_user_badge(&pool, user_id, badge_id, 5).await;

    let svc = setup_redemption_service(&pool).await;
    let result = svc
        .redeem_badge(RedeemBadgeRequest::new(user_id, rule_id, "idem-oos-92008"))
        .await;

    assert!(result.is_err());
    assert!(
        matches!(result.unwrap_err(), BadgeError::BenefitOutOfStock(id) if id == benefit_id),
        "库存为 0 应返回 BenefitOutOfStock",
    );

    // 验证徽章未被扣减（兑换在校验阶段就失败了）
    let qty = get_user_badge_quantity(&pool, user_id, badge_id).await;
    assert_eq!(qty, Some(5), "兑换失败时徽章不应被扣减");

    cleanup_test_data(&pool, &[user_id], &[rule_id], &[benefit_id], &[badge_id]).await;
}

/// 徽章余额不足：用户持有数量少于规则要求时应返回 InsufficientBadges
#[tokio::test]
#[ignore = "需要 PostgreSQL 和 Redis"]
async fn test_redeem_insufficient_badges() {
    let pool = PgPool::connect(&database_url()).await.unwrap();
    let badge_id = 92009;
    let benefit_id = 92009;
    let rule_id = 92009;
    let user_id = "integ_redeem_insuf_001";

    cleanup_test_data(&pool, &[user_id], &[rule_id], &[benefit_id], &[badge_id]).await;

    seed_test_badge(&pool, badge_id, "Insufficient Badge").await;
    seed_test_benefit(&pool, benefit_id, "BEN_92009", "余额不足权益", Some(100), Some(100), true).await;
    // 规则要求 3 个徽章
    seed_redemption_rule(
        &pool, rule_id, "需要3个徽章", benefit_id,
        json!([{"badgeId": badge_id, "quantity": 3}]),
        true, None, None,
    ).await;
    // 用户只有 2 个
    seed_user_badge(&pool, user_id, badge_id, 2).await;

    let svc = setup_redemption_service(&pool).await;
    let result = svc
        .redeem_badge(RedeemBadgeRequest::new(user_id, rule_id, "idem-insuf-92009"))
        .await;

    assert!(result.is_err());
    match result.unwrap_err() {
        BadgeError::InsufficientBadges { required, available } => {
            assert_eq!(required, 3);
            assert_eq!(available, 2);
        }
        other => panic!("应返回 InsufficientBadges，实际: {:?}", other),
    }

    cleanup_test_data(&pool, &[user_id], &[rule_id], &[benefit_id], &[badge_id]).await;
}

/// 兑换后徽章归零：数量正好用完时状态应变为 REDEEMED
#[tokio::test]
#[ignore = "需要 PostgreSQL 和 Redis"]
async fn test_redeem_badge_to_zero_sets_redeemed() {
    let pool = PgPool::connect(&database_url()).await.unwrap();
    let badge_id = 92010;
    let benefit_id = 92010;
    let rule_id = 92010;
    let user_id = "integ_redeem_zero_001";

    cleanup_test_data(&pool, &[user_id], &[rule_id], &[benefit_id], &[badge_id]).await;

    seed_test_badge(&pool, badge_id, "Zero Out Badge").await;
    seed_test_benefit(&pool, benefit_id, "BEN_92010", "归零权益", Some(100), Some(100), true).await;
    // 规则要求恰好 3 个
    seed_redemption_rule(
        &pool, rule_id, "精确扣减规则", benefit_id,
        json!([{"badgeId": badge_id, "quantity": 3}]),
        true, None, None,
    ).await;
    // 用户恰好有 3 个，兑换后归零
    seed_user_badge(&pool, user_id, badge_id, 3).await;

    let svc = setup_redemption_service(&pool).await;
    let resp = svc
        .redeem_badge(RedeemBadgeRequest::new(user_id, rule_id, "idem-zero-92010"))
        .await;

    assert!(resp.is_ok(), "兑换应成功: {:?}", resp.err());

    // 验证数量归零
    let qty = get_user_badge_quantity(&pool, user_id, badge_id).await;
    assert_eq!(qty, Some(0), "徽章数量应为 0");

    // 验证状态变为 REDEEMED（业务逻辑中 new_quantity==0 时更新状态）
    let status = get_user_badge_status(&pool, user_id, badge_id).await;
    assert_eq!(
        status.as_deref(),
        Some("REDEEMED"),
        "归零后状态应为 REDEEMED，实际: {:?}",
        status,
    );

    cleanup_test_data(&pool, &[user_id], &[rule_id], &[benefit_id], &[badge_id]).await;
}

/// 查询兑换历史：先执行兑换，再通过 get_user_redemptions 查询历史记录
#[tokio::test]
#[ignore = "需要 PostgreSQL 和 Redis"]
async fn test_get_user_redemptions() {
    let pool = PgPool::connect(&database_url()).await.unwrap();
    let badge_id = 92011;
    let benefit_id = 92011;
    let rule_id = 92011;
    let user_id = "integ_redeem_history_001";

    cleanup_test_data(&pool, &[user_id], &[rule_id], &[benefit_id], &[badge_id]).await;

    seed_test_badge(&pool, badge_id, "History Badge").await;
    seed_test_benefit(&pool, benefit_id, "BEN_92011", "历史权益", Some(100), Some(100), true).await;
    seed_redemption_rule(
        &pool, rule_id, "历史查询规则", benefit_id,
        json!([{"badgeId": badge_id, "quantity": 1}]),
        true, None, None,
    ).await;
    // 给足够的徽章用于两次兑换
    seed_user_badge(&pool, user_id, badge_id, 10).await;

    let svc = setup_redemption_service(&pool).await;

    // 执行两次兑换（不同的幂等键）
    let r1 = svc.redeem_badge(RedeemBadgeRequest::new(user_id, rule_id, "idem-hist-a-92011")).await;
    assert!(r1.is_ok(), "第一次兑换应成功: {:?}", r1.err());

    let r2 = svc.redeem_badge(RedeemBadgeRequest::new(user_id, rule_id, "idem-hist-b-92011")).await;
    assert!(r2.is_ok(), "第二次兑换应成功: {:?}", r2.err());

    // 查询兑换历史
    let history = svc.get_user_redemptions(user_id, 10).await;
    assert!(history.is_ok(), "查询历史应成功: {:?}", history.err());

    let history = history.unwrap();
    assert!(history.len() >= 2, "应至少有 2 条兑换记录，实际: {}", history.len());

    // 验证历史记录包含必要信息
    for record in &history {
        assert!(!record.order_no.is_empty(), "订单号不应为空");
        assert_eq!(record.benefit_name, "历史权益");
        // 每条记录应包含消耗的徽章明细
        assert!(!record.consumed_badges.is_empty(), "应包含消耗的徽章明细");
        assert_eq!(record.consumed_badges[0].badge_id, badge_id);
        assert_eq!(record.consumed_badges[0].quantity, 1);
    }

    cleanup_test_data(&pool, &[user_id], &[rule_id], &[benefit_id], &[badge_id]).await;
}
