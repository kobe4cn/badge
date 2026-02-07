//! GrantService 集成测试
//!
//! 使用真实 PostgreSQL 和 Redis 测试 GrantService 的完整发放流程。
//! GrantService 内部通过 sqlx::query 直接操作数据库（幂等检查、前置条件、互斥组等），
//! 无法通过纯 mock 覆盖，因此需要集成测试。
//!
//! ## 运行方式
//!
//! ```bash
//! DATABASE_URL=postgres://... REDIS_URL=redis://... \
//!   cargo test --test grant_service_test -- --ignored
//! ```

use badge_management::error::BadgeError;
use badge_management::repository::BadgeRepository;
use badge_management::service::dto::GrantBadgeRequest;
use badge_management::service::GrantService;
use badge_management::SourceType;
use badge_shared::cache::Cache;
use badge_shared::config::RedisConfig;
use sqlx::PgPool;
use std::sync::Arc;

// ==================== 辅助函数 ====================

/// 从环境变量读取数据库 URL，未设置则 panic
fn database_url() -> String {
    std::env::var("DATABASE_URL").expect("DATABASE_URL must be set for integration tests")
}

fn redis_url() -> String {
    std::env::var("REDIS_URL").unwrap_or_else(|_| "redis://localhost:6379".to_string())
}

/// 创建 GrantService 实例（使用真实 BadgeRepository + Cache）
async fn setup_grant_service(pool: &PgPool) -> GrantService<BadgeRepository> {
    let badge_repo = Arc::new(BadgeRepository::new(pool.clone()));
    let redis_config = RedisConfig {
        url: redis_url(),
        pool_size: 2,
    };
    let cache = Arc::new(Cache::new(&redis_config).expect("Redis connection failed"));
    GrantService::new(badge_repo, cache, pool.clone())
}

/// 插入测试用分类和系列（幂等，已存在则跳过）
///
/// 所有测试徽章共享同一分类和系列，减少重复 SQL
async fn ensure_category_and_series(pool: &PgPool) -> (i64, i64) {
    let cat_id: i64 = sqlx::query_scalar(
        r#"
        INSERT INTO badge_categories (id, name, status, sort_order)
        VALUES (99900, 'IntegTest Category', 'active', 0)
        ON CONFLICT (id) DO UPDATE SET name = EXCLUDED.name
        RETURNING id
        "#,
    )
    .fetch_one(pool)
    .await
    .expect("插入测试分类失败");

    let series_id: i64 = sqlx::query_scalar(
        r#"
        INSERT INTO badge_series (id, category_id, name, status, sort_order)
        VALUES (99900, 99900, 'IntegTest Series', 'active', 0)
        ON CONFLICT (id) DO UPDATE SET name = EXCLUDED.name
        RETURNING id
        "#,
    )
    .fetch_one(pool)
    .await
    .expect("插入测试系列失败");

    (cat_id, series_id)
}

/// 插入一个测试徽章
///
/// 使用 ON CONFLICT 保证幂等：相同 badge_id 多次调用不报错
async fn seed_test_badge(
    pool: &PgPool,
    badge_id: i64,
    name: &str,
    status: &str,
    max_supply: Option<i64>,
    issued_count: i64,
) {
    ensure_category_and_series(pool).await;

    sqlx::query(
        r#"
        INSERT INTO badges (id, series_id, badge_type, name, status, assets, validity_config,
                            max_supply, issued_count, sort_order)
        VALUES ($1, 99900, 'normal', $2, $3,
                '{"iconUrl":"https://test.com/icon.png"}',
                '{"validityType":"PERMANENT"}',
                $4, $5, 0)
        ON CONFLICT (id) DO UPDATE SET
            name = EXCLUDED.name,
            status = EXCLUDED.status,
            max_supply = EXCLUDED.max_supply,
            issued_count = EXCLUDED.issued_count
        "#,
    )
    .bind(badge_id)
    .bind(name)
    .bind(status)
    .bind(max_supply)
    .bind(issued_count)
    .execute(pool)
    .await
    .expect("插入测试徽章失败");
}

/// 为徽章插入获取规则（含 max_count_per_user 限制）
async fn seed_badge_rule(pool: &PgPool, badge_id: i64, max_count_per_user: Option<i32>) {
    sqlx::query(
        r#"
        INSERT INTO badge_rules (badge_id, rule_json, max_count_per_user, enabled)
        VALUES ($1, '{}', $2, true)
        "#,
    )
    .bind(badge_id)
    .bind(max_count_per_user)
    .execute(pool)
    .await
    .expect("插入徽章规则失败");
}

/// 插入前置条件依赖
///
/// 表示获得 badge_id 需要先持有 depends_on_badge_id 的 required_quantity 个
async fn seed_prerequisite(
    pool: &PgPool,
    badge_id: i64,
    depends_on_badge_id: i64,
    required_quantity: i32,
    group_id: &str,
) {
    sqlx::query(
        r#"
        INSERT INTO badge_dependencies
            (badge_id, depends_on_badge_id, dependency_type, required_quantity,
             dependency_group_id, enabled)
        VALUES ($1, $2, 'prerequisite', $3, $4, true)
        ON CONFLICT (badge_id, depends_on_badge_id, dependency_group_id) DO NOTHING
        "#,
    )
    .bind(badge_id)
    .bind(depends_on_badge_id)
    .bind(required_quantity)
    .bind(group_id)
    .execute(pool)
    .await
    .expect("插入前置条件失败");
}

/// 插入互斥组依赖
///
/// badge_id 和 other_badge_id 属于同一互斥组，用户只能持有其中一个
async fn seed_exclusive_group(
    pool: &PgPool,
    badge_id: i64,
    other_badge_id: i64,
    group_id: &str,
) {
    // 互斥关系双向插入：两个徽章都需要声明所属互斥组
    for (bid, dep) in [(badge_id, other_badge_id), (other_badge_id, badge_id)] {
        sqlx::query(
            r#"
            INSERT INTO badge_dependencies
                (badge_id, depends_on_badge_id, dependency_type, required_quantity,
                 exclusive_group_id, dependency_group_id, enabled)
            VALUES ($1, $2, 'exclusive', 1, $3, $4, true)
            ON CONFLICT (badge_id, depends_on_badge_id, dependency_group_id) DO NOTHING
            "#,
        )
        .bind(bid)
        .bind(dep)
        .bind(group_id)
        .bind(format!("exclusive_{}", group_id))
        .execute(pool)
        .await
        .expect("插入互斥组失败");
    }
}

/// 给用户直接插入一条 user_badges 记录（跳过业务逻辑，用于准备前置数据）
async fn seed_user_badge(pool: &PgPool, user_id: &str, badge_id: i64, quantity: i32) {
    sqlx::query(
        r#"
        INSERT INTO user_badges (user_id, badge_id, quantity, status, source_type)
        VALUES ($1, $2, $3, 'ACTIVE', 'MANUAL')
        ON CONFLICT (user_id, badge_id) DO UPDATE SET quantity = $3, status = 'ACTIVE'
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
///
/// 只清理当前测试使用的特定 badge_id 和 user_id，避免影响其他测试
async fn cleanup_test_data(pool: &PgPool, badge_ids: &[i64], user_ids: &[&str]) {
    // user_badge_logs 引用 user_badges.id，需先删
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

    // 清理规则和依赖
    for bid in badge_ids {
        sqlx::query("DELETE FROM badge_rules WHERE badge_id = $1")
            .bind(bid)
            .execute(pool)
            .await
            .ok();

        sqlx::query("DELETE FROM badge_dependencies WHERE badge_id = $1 OR depends_on_badge_id = $1")
            .bind(bid)
            .execute(pool)
            .await
            .ok();
    }
}

/// 构建发放请求的快捷方法
fn grant_request(user_id: &str, badge_id: i64, quantity: i32) -> GrantBadgeRequest {
    GrantBadgeRequest::new(user_id, badge_id, quantity)
        .with_source(SourceType::Manual, None)
}

// ==================== 测试用例 ====================

/// 正常发放：验证 user_badges 和 badge_ledger 记录正确创建
#[tokio::test]
#[ignore = "需要 PostgreSQL 和 Redis"]
async fn test_grant_badge_success() {
    let pool = PgPool::connect(&database_url()).await.unwrap();
    let badge_id = 90001;
    let user_id = "integ_grant_success_001";

    cleanup_test_data(&pool, &[badge_id], &[user_id]).await;
    seed_test_badge(&pool, badge_id, "Grant Success Badge", "active", Some(1000), 0).await;

    let svc = setup_grant_service(&pool).await;
    let resp = svc.grant_badge(grant_request(user_id, badge_id, 1)).await;

    assert!(resp.is_ok(), "发放应成功: {:?}", resp.err());
    let resp = resp.unwrap();
    assert!(resp.success);
    assert_eq!(resp.new_quantity, 1);
    assert!(resp.user_badge_id > 0);

    // 验证 user_badges 记录
    let ub_qty: Option<(i32,)> = sqlx::query_as(
        "SELECT quantity FROM user_badges WHERE user_id = $1 AND badge_id = $2",
    )
    .bind(user_id)
    .bind(badge_id)
    .fetch_optional(&pool)
    .await
    .unwrap();
    assert_eq!(ub_qty.unwrap().0, 1, "user_badges 数量应为 1");

    // 验证 badge_ledger 记录
    let ledger_count: (i64,) = sqlx::query_as(
        "SELECT COUNT(*) FROM badge_ledger WHERE user_id = $1 AND badge_id = $2 AND change_type = 'ACQUIRE'",
    )
    .bind(user_id)
    .bind(badge_id)
    .fetch_one(&pool)
    .await
    .unwrap();
    assert!(ledger_count.0 > 0, "badge_ledger 应有 acquire 记录");

    cleanup_test_data(&pool, &[badge_id], &[user_id]).await;
}

/// 增量发放：已有记录时 quantity 应累加
#[tokio::test]
#[ignore = "需要 PostgreSQL 和 Redis"]
async fn test_grant_badge_incremental() {
    let pool = PgPool::connect(&database_url()).await.unwrap();
    let badge_id = 90002;
    let user_id = "integ_grant_incr_001";

    cleanup_test_data(&pool, &[badge_id], &[user_id]).await;
    seed_test_badge(&pool, badge_id, "Incremental Badge", "active", None, 0).await;

    let svc = setup_grant_service(&pool).await;

    let r1 = svc.grant_badge(grant_request(user_id, badge_id, 3)).await.unwrap();
    assert_eq!(r1.new_quantity, 3);

    let r2 = svc.grant_badge(grant_request(user_id, badge_id, 2)).await.unwrap();
    assert_eq!(r2.new_quantity, 5, "第二次发放后 quantity 应累加到 5");

    // 确认数据库中 quantity 一致
    let ub_qty: (i32,) = sqlx::query_as(
        "SELECT quantity FROM user_badges WHERE user_id = $1 AND badge_id = $2",
    )
    .bind(user_id)
    .bind(badge_id)
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(ub_qty.0, 5);

    cleanup_test_data(&pool, &[badge_id], &[user_id]).await;
}

/// 幂等发放：相同 idempotency_key 第二次返回已存在消息
#[tokio::test]
#[ignore = "需要 PostgreSQL 和 Redis"]
async fn test_grant_idempotent_same_key() {
    let pool = PgPool::connect(&database_url()).await.unwrap();
    let badge_id = 90003;
    let user_id = "integ_grant_idem_001";

    cleanup_test_data(&pool, &[badge_id], &[user_id]).await;
    seed_test_badge(&pool, badge_id, "Idempotent Badge", "active", None, 0).await;

    let svc = setup_grant_service(&pool).await;

    let req = GrantBadgeRequest::new(user_id, badge_id, 2)
        .with_idempotency_key("idem-key-90003")
        .with_source(SourceType::Manual, None);

    let r1 = svc.grant_badge(req.clone()).await.unwrap();
    assert!(r1.success);
    assert_eq!(r1.new_quantity, 2);

    // 同 key 再次发放，应返回已存在的记录而非重复写入
    let r2 = svc.grant_badge(req).await.unwrap();
    assert!(r2.success);
    assert!(
        r2.message.contains("已存在"),
        "幂等返回的 message 应包含「已存在」, 实际: {}",
        r2.message
    );

    // 确认 quantity 未增加
    let ub_qty: (i32,) = sqlx::query_as(
        "SELECT quantity FROM user_badges WHERE user_id = $1 AND badge_id = $2",
    )
    .bind(user_id)
    .bind(badge_id)
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(ub_qty.0, 2, "幂等不应导致 quantity 增加");

    cleanup_test_data(&pool, &[badge_id], &[user_id]).await;
}

/// 不同 idempotency_key 应产生新的发放记录
#[tokio::test]
#[ignore = "需要 PostgreSQL 和 Redis"]
async fn test_grant_different_key() {
    let pool = PgPool::connect(&database_url()).await.unwrap();
    let badge_id = 90004;
    let user_id = "integ_grant_diffkey_001";

    cleanup_test_data(&pool, &[badge_id], &[user_id]).await;
    seed_test_badge(&pool, badge_id, "DiffKey Badge", "active", None, 0).await;

    let svc = setup_grant_service(&pool).await;

    let r1 = svc
        .grant_badge(
            GrantBadgeRequest::new(user_id, badge_id, 1)
                .with_idempotency_key("key-a-90004")
                .with_source(SourceType::Manual, None),
        )
        .await
        .unwrap();
    assert_eq!(r1.new_quantity, 1);

    let r2 = svc
        .grant_badge(
            GrantBadgeRequest::new(user_id, badge_id, 1)
                .with_idempotency_key("key-b-90004")
                .with_source(SourceType::Manual, None),
        )
        .await
        .unwrap();
    assert_eq!(r2.new_quantity, 2, "不同 key 应累加发放");

    cleanup_test_data(&pool, &[badge_id], &[user_id]).await;
}

/// 徽章不存在时返回 BadgeNotFound
#[tokio::test]
#[ignore = "需要 PostgreSQL 和 Redis"]
async fn test_grant_badge_not_found() {
    let pool = PgPool::connect(&database_url()).await.unwrap();
    let user_id = "integ_grant_notfound_001";
    // 使用一个肯定不存在的 badge_id
    let nonexistent_badge_id = 999999;

    let svc = setup_grant_service(&pool).await;
    let result = svc
        .grant_badge(grant_request(user_id, nonexistent_badge_id, 1))
        .await;

    assert!(result.is_err());
    assert!(
        matches!(result.unwrap_err(), BadgeError::BadgeNotFound(id) if id == nonexistent_badge_id),
        "应返回 BadgeNotFound"
    );
}

/// 徽章非 Active 状态时返回 BadgeInactive
#[tokio::test]
#[ignore = "需要 PostgreSQL 和 Redis"]
async fn test_grant_badge_inactive() {
    let pool = PgPool::connect(&database_url()).await.unwrap();
    let badge_id = 90006;
    let user_id = "integ_grant_inactive_001";

    cleanup_test_data(&pool, &[badge_id], &[user_id]).await;
    seed_test_badge(&pool, badge_id, "Inactive Badge", "draft", None, 0).await;

    let svc = setup_grant_service(&pool).await;
    let result = svc.grant_badge(grant_request(user_id, badge_id, 1)).await;

    assert!(result.is_err());
    assert!(
        matches!(result.unwrap_err(), BadgeError::BadgeInactive(id) if id == badge_id),
        "非 active 状态应返回 BadgeInactive"
    );

    cleanup_test_data(&pool, &[badge_id], &[user_id]).await;
}

/// 库存耗尽时返回 BadgeOutOfStock
#[tokio::test]
#[ignore = "需要 PostgreSQL 和 Redis"]
async fn test_grant_out_of_stock() {
    let pool = PgPool::connect(&database_url()).await.unwrap();
    let badge_id = 90007;
    let user_id = "integ_grant_oos_001";

    cleanup_test_data(&pool, &[badge_id], &[user_id]).await;
    // max_supply=10, issued_count=10 => 库存已满
    seed_test_badge(&pool, badge_id, "OOS Badge", "active", Some(10), 10).await;

    let svc = setup_grant_service(&pool).await;
    let result = svc.grant_badge(grant_request(user_id, badge_id, 1)).await;

    assert!(result.is_err());
    assert!(
        matches!(result.unwrap_err(), BadgeError::BadgeOutOfStock(id) if id == badge_id),
        "库存为 0 应返回 BadgeOutOfStock"
    );

    cleanup_test_data(&pool, &[badge_id], &[user_id]).await;
}

/// max_supply = None（不限量）正常发放
#[tokio::test]
#[ignore = "需要 PostgreSQL 和 Redis"]
async fn test_grant_unlimited_stock() {
    let pool = PgPool::connect(&database_url()).await.unwrap();
    let badge_id = 90008;
    let user_id = "integ_grant_unlimited_001";

    cleanup_test_data(&pool, &[badge_id], &[user_id]).await;
    seed_test_badge(&pool, badge_id, "Unlimited Badge", "active", None, 0).await;

    let svc = setup_grant_service(&pool).await;
    let result = svc
        .grant_badge(grant_request(user_id, badge_id, 100))
        .await;

    assert!(result.is_ok(), "不限量应正常发放: {:?}", result.err());
    assert_eq!(result.unwrap().new_quantity, 100);

    cleanup_test_data(&pool, &[badge_id], &[user_id]).await;
}

/// 达到 max_count_per_user 限制时返回 BadgeAcquisitionLimitReached
#[tokio::test]
#[ignore = "需要 PostgreSQL 和 Redis"]
async fn test_grant_user_limit_reached() {
    let pool = PgPool::connect(&database_url()).await.unwrap();
    let badge_id = 90009;
    let user_id = "integ_grant_userlimit_001";

    cleanup_test_data(&pool, &[badge_id], &[user_id]).await;
    seed_test_badge(&pool, badge_id, "UserLimit Badge", "active", None, 0).await;
    seed_badge_rule(&pool, badge_id, Some(3)).await;

    let svc = setup_grant_service(&pool).await;

    // 先发 3 个，达到上限
    let r1 = svc.grant_badge(grant_request(user_id, badge_id, 3)).await;
    assert!(r1.is_ok());

    // 再发 1 个，应触发限制
    let r2 = svc.grant_badge(grant_request(user_id, badge_id, 1)).await;
    assert!(r2.is_err());
    assert!(
        matches!(
            r2.unwrap_err(),
            BadgeError::BadgeAcquisitionLimitReached { badge_id: bid, limit: 3 } if bid == badge_id
        ),
        "超限应返回 BadgeAcquisitionLimitReached"
    );

    cleanup_test_data(&pool, &[badge_id], &[user_id]).await;
}

/// 无 max_count_per_user 限制时可无限发放
#[tokio::test]
#[ignore = "需要 PostgreSQL 和 Redis"]
async fn test_grant_no_user_limit() {
    let pool = PgPool::connect(&database_url()).await.unwrap();
    let badge_id = 90010;
    let user_id = "integ_grant_nolimit_001";

    cleanup_test_data(&pool, &[badge_id], &[user_id]).await;
    seed_test_badge(&pool, badge_id, "NoLimit Badge", "active", None, 0).await;
    // 不插入 badge_rules，则无 per-user 限制

    let svc = setup_grant_service(&pool).await;

    let r1 = svc
        .grant_badge(grant_request(user_id, badge_id, 50))
        .await;
    assert!(r1.is_ok());

    let r2 = svc
        .grant_badge(grant_request(user_id, badge_id, 50))
        .await;
    assert!(r2.is_ok());
    assert_eq!(r2.unwrap().new_quantity, 100, "无限制应持续累加");

    cleanup_test_data(&pool, &[badge_id], &[user_id]).await;
}

/// 缺少前置徽章时返回 PrerequisiteNotMet
#[tokio::test]
#[ignore = "需要 PostgreSQL 和 Redis"]
async fn test_grant_prerequisite_not_met() {
    let pool = PgPool::connect(&database_url()).await.unwrap();
    let prereq_badge_id = 90011;
    let target_badge_id = 90012;
    let user_id = "integ_grant_prereq_fail_001";

    cleanup_test_data(
        &pool,
        &[prereq_badge_id, target_badge_id],
        &[user_id],
    )
    .await;

    seed_test_badge(&pool, prereq_badge_id, "Prereq Badge", "active", None, 0).await;
    seed_test_badge(&pool, target_badge_id, "Target Badge", "active", None, 0).await;
    seed_prerequisite(&pool, target_badge_id, prereq_badge_id, 1, "grp_90012").await;

    let svc = setup_grant_service(&pool).await;

    // 未持有前置徽章，直接发放目标徽章
    let result = svc
        .grant_badge(grant_request(user_id, target_badge_id, 1))
        .await;

    assert!(result.is_err());
    match result.unwrap_err() {
        BadgeError::PrerequisiteNotMet { badge_id, missing } => {
            assert_eq!(badge_id, target_badge_id);
            assert!(
                missing.contains(&prereq_badge_id),
                "missing 应包含前置徽章 ID"
            );
        }
        other => panic!("应返回 PrerequisiteNotMet，实际: {:?}", other),
    }

    cleanup_test_data(
        &pool,
        &[prereq_badge_id, target_badge_id],
        &[user_id],
    )
    .await;
}

/// 满足前置条件后成功发放
#[tokio::test]
#[ignore = "需要 PostgreSQL 和 Redis"]
async fn test_grant_prerequisite_met() {
    let pool = PgPool::connect(&database_url()).await.unwrap();
    let prereq_badge_id = 90013;
    let target_badge_id = 90014;
    let user_id = "integ_grant_prereq_ok_001";

    cleanup_test_data(
        &pool,
        &[prereq_badge_id, target_badge_id],
        &[user_id],
    )
    .await;

    seed_test_badge(&pool, prereq_badge_id, "Prereq2 Badge", "active", None, 0).await;
    seed_test_badge(&pool, target_badge_id, "Target2 Badge", "active", None, 0).await;
    seed_prerequisite(&pool, target_badge_id, prereq_badge_id, 1, "grp_90014").await;

    // 先给用户持有前置徽章
    seed_user_badge(&pool, user_id, prereq_badge_id, 1).await;

    let svc = setup_grant_service(&pool).await;
    let result = svc
        .grant_badge(grant_request(user_id, target_badge_id, 1))
        .await;

    assert!(
        result.is_ok(),
        "满足前置条件后应发放成功: {:?}",
        result.err()
    );

    cleanup_test_data(
        &pool,
        &[prereq_badge_id, target_badge_id],
        &[user_id],
    )
    .await;
}

/// 已持有互斥徽章时返回 ExclusiveConflict
#[tokio::test]
#[ignore = "需要 PostgreSQL 和 Redis"]
async fn test_grant_exclusive_conflict() {
    let pool = PgPool::connect(&database_url()).await.unwrap();
    let badge_a = 90015;
    let badge_b = 90016;
    let user_id = "integ_grant_excl_001";

    cleanup_test_data(&pool, &[badge_a, badge_b], &[user_id]).await;

    seed_test_badge(&pool, badge_a, "Exclusive A", "active", None, 0).await;
    seed_test_badge(&pool, badge_b, "Exclusive B", "active", None, 0).await;
    seed_exclusive_group(&pool, badge_a, badge_b, "excl_grp_9001").await;

    // 先给用户持有 badge_a
    seed_user_badge(&pool, user_id, badge_a, 1).await;

    let svc = setup_grant_service(&pool).await;

    // 再尝试发放互斥的 badge_b
    let result = svc
        .grant_badge(grant_request(user_id, badge_b, 1))
        .await;

    assert!(result.is_err());
    match result.unwrap_err() {
        BadgeError::ExclusiveConflict {
            target,
            conflicting,
        } => {
            assert_eq!(target, badge_b);
            assert_eq!(conflicting, badge_a);
        }
        other => panic!("应返回 ExclusiveConflict，实际: {:?}", other),
    }

    cleanup_test_data(&pool, &[badge_a, badge_b], &[user_id]).await;
}

/// Cascade 来源跳过前置条件和互斥检查
#[tokio::test]
#[ignore = "需要 PostgreSQL 和 Redis"]
async fn test_grant_cascade_skips_checks() {
    let pool = PgPool::connect(&database_url()).await.unwrap();
    let prereq_badge_id = 90017;
    let target_badge_id = 90018;
    let excl_badge_id = 90019;
    let user_id = "integ_grant_cascade_001";

    cleanup_test_data(
        &pool,
        &[prereq_badge_id, target_badge_id, excl_badge_id],
        &[user_id],
    )
    .await;

    seed_test_badge(&pool, prereq_badge_id, "CascPrereq", "active", None, 0).await;
    seed_test_badge(&pool, target_badge_id, "CascTarget", "active", None, 0).await;
    seed_test_badge(&pool, excl_badge_id, "CascExcl", "active", None, 0).await;

    // 设置前置条件和互斥组
    seed_prerequisite(&pool, target_badge_id, prereq_badge_id, 1, "grp_90018").await;
    seed_exclusive_group(&pool, target_badge_id, excl_badge_id, "excl_grp_9002").await;

    // 给用户持有互斥徽章（正常发放时会被拦截）
    seed_user_badge(&pool, user_id, excl_badge_id, 1).await;

    let svc = setup_grant_service(&pool).await;

    // Cascade 来源应跳过前置和互斥检查
    let req = GrantBadgeRequest {
        user_id: user_id.to_string(),
        badge_id: target_badge_id,
        quantity: 1,
        source_type: SourceType::Cascade,
        source_ref_id: Some("triggered_by_90017".to_string()),
        idempotency_key: None,
        reason: Some("级联测试".to_string()),
        operator: None,
    };

    let result = svc.grant_badge(req).await;
    assert!(
        result.is_ok(),
        "Cascade 来源应绕过前置/互斥检查: {:?}",
        result.err()
    );

    cleanup_test_data(
        &pool,
        &[prereq_badge_id, target_badge_id, excl_badge_id],
        &[user_id],
    )
    .await;
}

/// 批量发放：部分失败时统计正确
#[tokio::test]
#[ignore = "需要 PostgreSQL 和 Redis"]
async fn test_batch_grant_partial_failure() {
    let pool = PgPool::connect(&database_url()).await.unwrap();
    let good_badge = 90020;
    let bad_badge = 90021; // inactive，发放会失败
    let user_a = "integ_batch_001";
    let user_b = "integ_batch_002";

    cleanup_test_data(&pool, &[good_badge, bad_badge], &[user_a, user_b]).await;

    seed_test_badge(&pool, good_badge, "Good Badge", "active", None, 0).await;
    seed_test_badge(&pool, bad_badge, "Bad Badge", "draft", None, 0).await;

    let svc = setup_grant_service(&pool).await;

    let requests = vec![
        grant_request(user_a, good_badge, 1),   // 应成功
        grant_request(user_b, bad_badge, 1),     // 应失败（draft 状态）
        grant_request(user_a, good_badge, 2),    // 应成功（增量）
    ];

    let batch_resp = svc.batch_grant_badges(requests).await.unwrap();

    assert_eq!(batch_resp.total, 3);
    assert_eq!(batch_resp.success_count, 2, "应有 2 个成功");
    assert_eq!(batch_resp.failed_count, 1, "应有 1 个失败");

    // 验证各结果
    assert!(batch_resp.results[0].success, "第 1 个请求应成功");
    assert!(!batch_resp.results[1].success, "第 2 个请求应失败");
    assert!(
        batch_resp.results[1].error.is_some(),
        "失败请求应有错误信息"
    );
    assert!(batch_resp.results[2].success, "第 3 个请求应成功");

    cleanup_test_data(&pool, &[good_badge, bad_badge], &[user_a, user_b]).await;
}
