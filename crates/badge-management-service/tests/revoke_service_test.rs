//! RevokeService 集成测试
//!
//! 使用真实 PostgreSQL 和 Redis 测试 RevokeService 的完整取消/撤销流程。
//! RevokeService 内部通过 sqlx::query 直接操作数据库（行级锁、事务扣减、状态变更），
//! 无法通过纯 mock 覆盖，因此需要集成测试。
//!
//! ## 运行方式
//!
//! ```bash
//! DATABASE_URL=postgres://... REDIS_URL=redis://... \
//!   cargo test --test revoke_service_test -- --ignored
//! ```

use badge_management::error::BadgeError;
use badge_management::repository::BadgeRepository;
use badge_management::service::dto::{BadgeGrantCondition, RefundEvent, RevokeBadgeRequest};
use badge_management::service::RevokeService;
use badge_shared::cache::Cache;
use badge_shared::config::RedisConfig;
use chrono::Utc;
use sqlx::PgPool;
use std::sync::Arc;

// ==================== 辅助函数 ====================

fn database_url() -> String {
    std::env::var("DATABASE_URL").expect("DATABASE_URL must be set for integration tests")
}

fn redis_url() -> String {
    std::env::var("REDIS_URL").unwrap_or_else(|_| "redis://localhost:6379".to_string())
}

/// 创建 RevokeService 实例
///
/// RevokeService 构造函数需要 (Arc<Cache>, PgPool, Arc<BadgeRepository>)，
/// 与 GrantService 的参数顺序不同
async fn setup_revoke_service(pool: &PgPool) -> RevokeService<BadgeRepository> {
    let badge_repo = Arc::new(BadgeRepository::new(pool.clone()));
    let redis_config = RedisConfig {
        url: redis_url(),
        pool_size: 2,
    };
    let cache = Arc::new(Cache::new(&redis_config).expect("Redis connection failed"));
    RevokeService::new(cache, pool.clone(), badge_repo)
}

/// 插入测试用分类和系列（幂等）
async fn ensure_category_and_series(pool: &PgPool) {
    sqlx::query(
        r#"
        INSERT INTO badge_categories (id, name, status, sort_order)
        VALUES (99900, 'IntegTest Category', 'active', 0)
        ON CONFLICT (id) DO UPDATE SET name = EXCLUDED.name
        "#,
    )
    .execute(pool)
    .await
    .expect("插入测试分类失败");

    sqlx::query(
        r#"
        INSERT INTO badge_series (id, category_id, name, status, sort_order)
        VALUES (99900, 99900, 'IntegTest Series', 'active', 0)
        ON CONFLICT (id) DO UPDATE SET name = EXCLUDED.name
        "#,
    )
    .execute(pool)
    .await
    .expect("插入测试系列失败");
}

/// 插入一个测试徽章定义
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
        ON CONFLICT (id) DO UPDATE SET name = EXCLUDED.name, status = 'active'
        "#,
    )
    .bind(badge_id)
    .bind(name)
    .execute(pool)
    .await
    .expect("插入测试徽章失败");
}

/// 直接向 user_badges 表插入记录，模拟用户已持有徽章
///
/// 取消操作的前提是用户已持有该徽章，所以需要先插入持有记录。
/// 使用 ON CONFLICT 保证幂等。
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

/// 将用户徽章状态设为 Revoked，用于测试对已取消状态的二次取消
async fn set_user_badge_revoked(pool: &PgPool, user_id: &str, badge_id: i64) {
    sqlx::query(
        r#"
        UPDATE user_badges SET status = 'revoked', quantity = 0
        WHERE user_id = $1 AND badge_id = $2
        "#,
    )
    .bind(user_id)
    .bind(badge_id)
    .execute(pool)
    .await
    .expect("设置用户徽章为已取消失败");
}

/// 查询用户徽章的当前数量
async fn get_user_badge_quantity(pool: &PgPool, user_id: &str, badge_id: i64) -> Option<i32> {
    let result: Option<(i32,)> = sqlx::query_as(
        "SELECT quantity FROM user_badges WHERE user_id = $1 AND badge_id = $2",
    )
    .bind(user_id)
    .bind(badge_id)
    .fetch_optional(pool)
    .await
    .unwrap();
    result.map(|r| r.0)
}

/// 查询用户徽章的当前状态
async fn get_user_badge_status(pool: &PgPool, user_id: &str, badge_id: i64) -> Option<String> {
    let result: Option<(String,)> = sqlx::query_as(
        "SELECT status::text FROM user_badges WHERE user_id = $1 AND badge_id = $2",
    )
    .bind(user_id)
    .bind(badge_id)
    .fetch_optional(pool)
    .await
    .unwrap();
    result.map(|r| r.0)
}

/// 清理测试数据，按外键依赖顺序删除
async fn cleanup_test_data(pool: &PgPool, badge_ids: &[i64], user_ids: &[&str]) {
    for uid in user_ids {
        sqlx::query("DELETE FROM user_badge_logs WHERE user_id = $1")
            .bind(uid)
            .execute(pool)
            .await
            .ok();
    }

    for uid in user_ids {
        sqlx::query("DELETE FROM badge_ledger WHERE user_id = $1")
            .bind(uid)
            .execute(pool)
            .await
            .ok();
    }

    for uid in user_ids {
        sqlx::query("DELETE FROM user_badges WHERE user_id = $1")
            .bind(uid)
            .execute(pool)
            .await
            .ok();
    }

    for bid in badge_ids {
        sqlx::query("DELETE FROM badge_rules WHERE badge_id = $1")
            .bind(bid)
            .execute(pool)
            .await
            .ok();

        sqlx::query(
            "DELETE FROM badge_dependencies WHERE badge_id = $1 OR depends_on_badge_id = $1",
        )
        .bind(bid)
        .execute(pool)
        .await
        .ok();
    }
}

/// 清理 Redis 中的退款幂等标记
async fn cleanup_refund_key(cache: &Cache, event_id: &str) {
    let key = format!("refund:processed:{}", event_id);
    let _ = cache.delete(&key).await;
}

// ==================== 测试用例 ====================

/// 正常取消：持有 10 个，取消 10 个，验证数量归零
#[tokio::test]
#[ignore = "需要 PostgreSQL 和 Redis"]
async fn test_revoke_badge_success() {
    let pool = PgPool::connect(&database_url()).await.unwrap();
    let badge_id = 91001;
    let user_id = "integ_revoke_success_001";

    cleanup_test_data(&pool, &[badge_id], &[user_id]).await;
    seed_test_badge(&pool, badge_id, "Revoke Success Badge").await;
    seed_user_badge(&pool, user_id, badge_id, 10).await;

    let svc = setup_revoke_service(&pool).await;
    let req = RevokeBadgeRequest::manual(user_id, badge_id, 10, "集成测试取消", "test_admin");
    let resp = svc.revoke_badge(req).await;

    assert!(resp.is_ok(), "取消应成功: {:?}", resp.err());
    let resp = resp.unwrap();
    assert!(resp.success);
    assert_eq!(resp.remaining_quantity, 0, "取消全部后剩余应为 0");

    // 验证数据库中数量确实被扣减
    let qty = get_user_badge_quantity(&pool, user_id, badge_id).await;
    assert_eq!(qty, Some(0), "数据库中数量应为 0");

    cleanup_test_data(&pool, &[badge_id], &[user_id]).await;
}

/// 部分取消：持有 10 个取消 3 个，验证剩余 7 个
#[tokio::test]
#[ignore = "需要 PostgreSQL 和 Redis"]
async fn test_revoke_badge_partial() {
    let pool = PgPool::connect(&database_url()).await.unwrap();
    let badge_id = 91002;
    let user_id = "integ_revoke_partial_001";

    cleanup_test_data(&pool, &[badge_id], &[user_id]).await;
    seed_test_badge(&pool, badge_id, "Revoke Partial Badge").await;
    seed_user_badge(&pool, user_id, badge_id, 10).await;

    let svc = setup_revoke_service(&pool).await;
    let req = RevokeBadgeRequest::manual(user_id, badge_id, 3, "部分取消测试", "test_admin");
    let resp = svc.revoke_badge(req).await;

    assert!(resp.is_ok(), "部分取消应成功: {:?}", resp.err());
    let resp = resp.unwrap();
    assert!(resp.success);
    assert_eq!(resp.remaining_quantity, 7, "取消 3 个后应剩余 7 个");

    let qty = get_user_badge_quantity(&pool, user_id, badge_id).await;
    assert_eq!(qty, Some(7), "数据库中数量应为 7");

    cleanup_test_data(&pool, &[badge_id], &[user_id]).await;
}

/// 取消至零时状态变为 Revoked
///
/// 业务规则：当用户持有的徽章数量扣减到 0 时，
/// 状态应自动变为 REVOKED 而非保持 ACTIVE
#[tokio::test]
#[ignore = "需要 PostgreSQL 和 Redis"]
async fn test_revoke_to_zero_sets_revoked() {
    let pool = PgPool::connect(&database_url()).await.unwrap();
    let badge_id = 91003;
    let user_id = "integ_revoke_zero_001";

    cleanup_test_data(&pool, &[badge_id], &[user_id]).await;
    seed_test_badge(&pool, badge_id, "Revoke Zero Badge").await;
    seed_user_badge(&pool, user_id, badge_id, 5).await;

    let svc = setup_revoke_service(&pool).await;
    let req = RevokeBadgeRequest::manual(user_id, badge_id, 5, "全部取消", "test_admin");
    let resp = svc.revoke_badge(req).await.unwrap();

    assert_eq!(resp.remaining_quantity, 0);

    // 验证状态变为 REVOKED
    let status = get_user_badge_status(&pool, user_id, badge_id).await;
    assert_eq!(
        status.as_deref(),
        Some("REVOKED"),
        "数量归零后状态应变为 REVOKED"
    );

    cleanup_test_data(&pool, &[badge_id], &[user_id]).await;
}

/// 用户未持有该徽章时返回 UserBadgeNotFound
#[tokio::test]
#[ignore = "需要 PostgreSQL 和 Redis"]
async fn test_revoke_not_found() {
    let pool = PgPool::connect(&database_url()).await.unwrap();
    let badge_id = 91004;
    let user_id = "integ_revoke_notfound_001";

    cleanup_test_data(&pool, &[badge_id], &[user_id]).await;
    seed_test_badge(&pool, badge_id, "Revoke NotFound Badge").await;
    // 不插入 user_badges 记录，模拟用户未持有

    let svc = setup_revoke_service(&pool).await;
    let req = RevokeBadgeRequest::manual(user_id, badge_id, 1, "不存在测试", "test_admin");
    let result = svc.revoke_badge(req).await;

    assert!(result.is_err());
    match result.unwrap_err() {
        BadgeError::UserBadgeNotFound {
            user_id: uid,
            badge_id: bid,
        } => {
            assert_eq!(uid, user_id);
            assert_eq!(bid, badge_id);
        }
        other => panic!("应返回 UserBadgeNotFound，实际: {:?}", other),
    }

    cleanup_test_data(&pool, &[badge_id], &[user_id]).await;
}

/// 余额不足时返回 InsufficientBadges
///
/// 用户持有 2 个但尝试取消 5 个
#[tokio::test]
#[ignore = "需要 PostgreSQL 和 Redis"]
async fn test_revoke_insufficient_balance() {
    let pool = PgPool::connect(&database_url()).await.unwrap();
    let badge_id = 91005;
    let user_id = "integ_revoke_insuff_001";

    cleanup_test_data(&pool, &[badge_id], &[user_id]).await;
    seed_test_badge(&pool, badge_id, "Revoke Insufficient Badge").await;
    seed_user_badge(&pool, user_id, badge_id, 2).await;

    let svc = setup_revoke_service(&pool).await;
    let req = RevokeBadgeRequest::manual(user_id, badge_id, 5, "超额取消测试", "test_admin");
    let result = svc.revoke_badge(req).await;

    assert!(result.is_err());
    match result.unwrap_err() {
        BadgeError::InsufficientBadges {
            required,
            available,
        } => {
            assert_eq!(required, 5);
            assert_eq!(available, 2);
        }
        other => panic!("应返回 InsufficientBadges，实际: {:?}", other),
    }

    // 验证数量未被修改
    let qty = get_user_badge_quantity(&pool, user_id, badge_id).await;
    assert_eq!(qty, Some(2), "失败的取消不应改变数量");

    cleanup_test_data(&pool, &[badge_id], &[user_id]).await;
}

/// 已处于 Revoked 状态的徽章不允许再次取消
///
/// 业务规则：只有 ACTIVE 状态的徽章才能被取消，
/// 对已取消的徽章再次操作应返回 Validation 错误
#[tokio::test]
#[ignore = "需要 PostgreSQL 和 Redis"]
async fn test_revoke_already_revoked() {
    let pool = PgPool::connect(&database_url()).await.unwrap();
    let badge_id = 91006;
    let user_id = "integ_revoke_already_001";

    cleanup_test_data(&pool, &[badge_id], &[user_id]).await;
    seed_test_badge(&pool, badge_id, "Already Revoked Badge").await;
    seed_user_badge(&pool, user_id, badge_id, 0).await;
    // 手动将状态设为 REVOKED
    set_user_badge_revoked(&pool, user_id, badge_id).await;

    let svc = setup_revoke_service(&pool).await;
    let req = RevokeBadgeRequest::manual(user_id, badge_id, 1, "重复取消测试", "test_admin");
    let result = svc.revoke_badge(req).await;

    assert!(result.is_err());
    match result.unwrap_err() {
        BadgeError::Validation(msg) => {
            assert!(
                msg.contains("状态不允许取消"),
                "错误消息应包含状态说明，实际: {}",
                msg
            );
        }
        other => panic!("应返回 Validation 错误，实际: {:?}", other),
    }

    cleanup_test_data(&pool, &[badge_id], &[user_id]).await;
}

/// 全额退款：所有相关徽章都应被撤销
#[tokio::test]
#[ignore = "需要 PostgreSQL 和 Redis"]
async fn test_handle_full_refund_revokes_all() {
    let pool = PgPool::connect(&database_url()).await.unwrap();
    let badge_id = 91007;
    let user_id = "integ_refund_full_001";

    cleanup_test_data(&pool, &[badge_id], &[user_id]).await;
    seed_test_badge(&pool, badge_id, "Full Refund Badge").await;
    seed_user_badge(&pool, user_id, badge_id, 1).await;

    let svc = setup_revoke_service(&pool).await;

    let event = RefundEvent {
        event_id: "evt-full-91007".to_string(),
        user_id: user_id.to_string(),
        original_order_id: "order-91007".to_string(),
        refund_order_id: "refund-91007".to_string(),
        original_amount: 50000,
        refund_amount: 50000, // 全额退款
        remaining_amount: 0,
        reason: Some("全额退款测试".to_string()),
        badge_ids_to_revoke: None,
        timestamp: Utc::now(),
    };

    let conditions = vec![BadgeGrantCondition {
        rule_id: 1,
        badge_id,
        badge_name: "Full Refund Badge".to_string(),
        amount_threshold: Some(30000),
        event_type: "purchase".to_string(),
    }];

    let result = svc.handle_refund(&event, &conditions).await;

    assert!(result.is_ok(), "退款处理应成功: {:?}", result.err());
    let result = result.unwrap();
    assert!(result.success);
    assert_eq!(result.revoked_badges.len(), 1, "全额退款应撤销 1 个徽章");
    assert_eq!(result.retained_badges.len(), 0, "不应保留任何徽章");

    // 验证数据库中徽章已被扣减
    let qty = get_user_badge_quantity(&pool, user_id, badge_id).await;
    assert_eq!(qty, Some(0), "全额退款后数量应为 0");

    cleanup_test_data(&pool, &[badge_id], &[user_id]).await;
}

/// 部分退款但仍满足发放条件：保留徽章
///
/// 原金额 80000，退款 20000，剩余 60000 >= 阈值 50000，应保留
#[tokio::test]
#[ignore = "需要 PostgreSQL 和 Redis"]
async fn test_handle_partial_refund_keeps_badge() {
    let pool = PgPool::connect(&database_url()).await.unwrap();
    let badge_id = 91008;
    let user_id = "integ_refund_keep_001";

    cleanup_test_data(&pool, &[badge_id], &[user_id]).await;
    seed_test_badge(&pool, badge_id, "Partial Keep Badge").await;
    seed_user_badge(&pool, user_id, badge_id, 1).await;

    let svc = setup_revoke_service(&pool).await;

    let event = RefundEvent {
        event_id: "evt-keep-91008".to_string(),
        user_id: user_id.to_string(),
        original_order_id: "order-91008".to_string(),
        refund_order_id: "refund-91008".to_string(),
        original_amount: 80000,
        refund_amount: 20000,   // 退 200 元
        remaining_amount: 60000, // 剩 600 元
        reason: Some("部分退款保留测试".to_string()),
        badge_ids_to_revoke: None,
        timestamp: Utc::now(),
    };

    // 阈值 50000，退款后剩余 60000 >= 50000，应保留
    let conditions = vec![BadgeGrantCondition {
        rule_id: 2,
        badge_id,
        badge_name: "Partial Keep Badge".to_string(),
        amount_threshold: Some(50000),
        event_type: "purchase".to_string(),
    }];

    let result = svc.handle_refund(&event, &conditions).await.unwrap();

    assert!(result.success);
    assert_eq!(result.revoked_badges.len(), 0, "满足条件应保留，不撤销");
    assert_eq!(result.retained_badges.len(), 1, "应有 1 个保留记录");

    // 验证数据库中徽章未被扣减
    let qty = get_user_badge_quantity(&pool, user_id, badge_id).await;
    assert_eq!(qty, Some(1), "保留场景中数量应不变");

    cleanup_test_data(&pool, &[badge_id], &[user_id]).await;
}

/// 部分退款后不满足发放条件：撤销徽章
///
/// 原金额 80000，退款 50000，剩余 30000 < 阈值 50000，应撤销
#[tokio::test]
#[ignore = "需要 PostgreSQL 和 Redis"]
async fn test_handle_partial_refund_revokes_badge() {
    let pool = PgPool::connect(&database_url()).await.unwrap();
    let badge_id = 91009;
    let user_id = "integ_refund_revoke_001";

    cleanup_test_data(&pool, &[badge_id], &[user_id]).await;
    seed_test_badge(&pool, badge_id, "Partial Revoke Badge").await;
    seed_user_badge(&pool, user_id, badge_id, 1).await;

    let svc = setup_revoke_service(&pool).await;

    let event = RefundEvent {
        event_id: "evt-revoke-91009".to_string(),
        user_id: user_id.to_string(),
        original_order_id: "order-91009".to_string(),
        refund_order_id: "refund-91009".to_string(),
        original_amount: 80000,
        refund_amount: 50000,   // 退 500 元
        remaining_amount: 30000, // 剩 300 元
        reason: Some("部分退款撤销测试".to_string()),
        badge_ids_to_revoke: None,
        timestamp: Utc::now(),
    };

    // 阈值 50000，退款后剩余 30000 < 50000，应撤销
    let conditions = vec![BadgeGrantCondition {
        rule_id: 3,
        badge_id,
        badge_name: "Partial Revoke Badge".to_string(),
        amount_threshold: Some(50000),
        event_type: "purchase".to_string(),
    }];

    let result = svc.handle_refund(&event, &conditions).await.unwrap();

    assert!(result.success);
    assert_eq!(result.revoked_badges.len(), 1, "不满足条件应撤销");
    assert_eq!(result.retained_badges.len(), 0, "不应保留");

    let qty = get_user_badge_quantity(&pool, user_id, badge_id).await;
    assert_eq!(qty, Some(0), "撤销后数量应为 0");

    cleanup_test_data(&pool, &[badge_id], &[user_id]).await;
}

/// 退款幂等：相同 event_id 标记后不应重复处理
///
/// 通过 is_refund_processed + mark_refund_processed 实现幂等，
/// 验证已标记的事件确实被识别为"已处理"
#[tokio::test]
#[ignore = "需要 PostgreSQL 和 Redis"]
async fn test_refund_idempotent() {
    let pool = PgPool::connect(&database_url()).await.unwrap();
    let badge_id = 91010;
    let user_id = "integ_refund_idem_001";
    let event_id = "evt-idem-91010";

    cleanup_test_data(&pool, &[badge_id], &[user_id]).await;
    seed_test_badge(&pool, badge_id, "Idempotent Refund Badge").await;
    seed_user_badge(&pool, user_id, badge_id, 1).await;

    let svc = setup_revoke_service(&pool).await;

    // 清理可能残留的幂等标记
    let redis_config = RedisConfig {
        url: redis_url(),
        pool_size: 2,
    };
    let cache = Cache::new(&redis_config).expect("Redis connection failed");
    cleanup_refund_key(&cache, event_id).await;

    // 首次检查：未处理
    let processed = svc.is_refund_processed(event_id).await.unwrap();
    assert!(!processed, "首次检查应返回未处理");

    // 标记为已处理
    svc.mark_refund_processed(event_id).await.unwrap();

    // 二次检查：已处理
    let processed = svc.is_refund_processed(event_id).await.unwrap();
    assert!(processed, "标记后应返回已处理");

    // 模拟业务调用方使用幂等检查跳过重复退款
    // （实际业务代码应在 handle_refund 前先调用 is_refund_processed）
    let qty = get_user_badge_quantity(&pool, user_id, badge_id).await;
    assert_eq!(qty, Some(1), "幂等跳过后数量应不变");

    // 清理
    cleanup_refund_key(&cache, event_id).await;
    cleanup_test_data(&pool, &[badge_id], &[user_id]).await;
}

/// 批量取消：部分失败时统计正确
///
/// 三个请求中一个用户未持有徽章，应该只有该请求失败
#[tokio::test]
#[ignore = "需要 PostgreSQL 和 Redis"]
async fn test_batch_revoke_partial_failure() {
    let pool = PgPool::connect(&database_url()).await.unwrap();
    let badge_a = 91011;
    let badge_b = 91012;
    let user_a = "integ_batch_revoke_001";
    let user_b = "integ_batch_revoke_002";

    cleanup_test_data(&pool, &[badge_a, badge_b], &[user_a, user_b]).await;
    seed_test_badge(&pool, badge_a, "Batch Revoke A").await;
    seed_test_badge(&pool, badge_b, "Batch Revoke B").await;

    // user_a 持有 badge_a，user_b 不持有 badge_b
    seed_user_badge(&pool, user_a, badge_a, 5).await;

    let svc = setup_revoke_service(&pool).await;

    let requests = vec![
        // 应成功：user_a 持有 badge_a 5 个，取消 2 个
        RevokeBadgeRequest::manual(user_a, badge_a, 2, "批量取消A", "admin"),
        // 应失败：user_b 未持有 badge_b
        RevokeBadgeRequest::manual(user_b, badge_b, 1, "批量取消B", "admin"),
        // 应成功：user_a 再取消 1 个
        RevokeBadgeRequest::manual(user_a, badge_a, 1, "批量取消A2", "admin"),
    ];

    let batch_resp = svc.batch_revoke_badges(requests).await.unwrap();

    assert_eq!(batch_resp.total, 3);
    assert_eq!(batch_resp.success_count, 2, "应有 2 个成功");
    assert_eq!(batch_resp.failed_count, 1, "应有 1 个失败");

    assert!(batch_resp.results[0].success, "第 1 个请求应成功");
    assert!(!batch_resp.results[1].success, "第 2 个请求应失败");
    assert!(
        batch_resp.results[1].error.is_some(),
        "失败请求应有错误信息"
    );
    assert!(batch_resp.results[2].success, "第 3 个请求应成功");

    // 验证 user_a 的最终数量：5 - 2 - 1 = 2
    let qty = get_user_badge_quantity(&pool, user_a, badge_a).await;
    assert_eq!(qty, Some(2), "批量取消后 user_a 应剩余 2 个");

    cleanup_test_data(&pool, &[badge_a, badge_b], &[user_a, user_b]).await;
}

/// 批量取消：全部成功
#[tokio::test]
#[ignore = "需要 PostgreSQL 和 Redis"]
async fn test_batch_revoke_all_success() {
    let pool = PgPool::connect(&database_url()).await.unwrap();
    let badge_a = 91013;
    let badge_b = 91014;
    let user_a = "integ_batch_all_001";
    let user_b = "integ_batch_all_002";

    cleanup_test_data(&pool, &[badge_a, badge_b], &[user_a, user_b]).await;
    seed_test_badge(&pool, badge_a, "Batch All A").await;
    seed_test_badge(&pool, badge_b, "Batch All B").await;

    seed_user_badge(&pool, user_a, badge_a, 10).await;
    seed_user_badge(&pool, user_b, badge_b, 3).await;

    let svc = setup_revoke_service(&pool).await;

    let requests = vec![
        RevokeBadgeRequest::manual(user_a, badge_a, 5, "批量全成功A", "admin"),
        RevokeBadgeRequest::manual(user_b, badge_b, 3, "批量全成功B", "admin"),
    ];

    let batch_resp = svc.batch_revoke_badges(requests).await.unwrap();

    assert_eq!(batch_resp.total, 2);
    assert_eq!(batch_resp.success_count, 2, "所有请求应成功");
    assert_eq!(batch_resp.failed_count, 0, "不应有失败");

    // 验证剩余数量
    assert!(batch_resp.results[0].success);
    assert_eq!(batch_resp.results[0].remaining_quantity, Some(5));
    assert!(batch_resp.results[1].success);
    assert_eq!(batch_resp.results[1].remaining_quantity, Some(0));

    let qty_a = get_user_badge_quantity(&pool, user_a, badge_a).await;
    let qty_b = get_user_badge_quantity(&pool, user_b, badge_b).await;
    assert_eq!(qty_a, Some(5), "user_a 应剩余 5 个");
    assert_eq!(qty_b, Some(0), "user_b 应剩余 0 个");

    cleanup_test_data(&pool, &[badge_a, badge_b], &[user_a, user_b]).await;
}
