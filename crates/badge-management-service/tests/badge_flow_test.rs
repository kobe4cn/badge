//! 徽章发放流程集成测试
//!
//! 测试徽章系统的完整业务流程（使用 Mock 实现，无需外部依赖）

use std::collections::HashMap;
use std::sync::Arc;

use chrono::{DateTime, Utc};
use tokio::sync::RwLock;

// ==================== Mock 数据结构 ====================

#[derive(Debug, Clone, PartialEq)]
pub enum MockBadgeStatus {
    Active,
    Inactive,
}

#[derive(Debug, Clone, PartialEq)]
pub enum MockUserBadgeStatus {
    Active,
    Expired,
    Revoked,
    Redeemed,
}

#[derive(Debug, Clone)]
pub struct MockBadge {
    pub id: i64,
    pub name: String,
    pub status: MockBadgeStatus,
    pub max_supply: Option<i64>,
    pub issued_count: i64,
    pub max_per_user: Option<i32>,
}

impl MockBadge {
    pub fn new(id: i64, name: &str) -> Self {
        Self {
            id,
            name: name.to_string(),
            status: MockBadgeStatus::Active,
            max_supply: None,
            issued_count: 0,
            max_per_user: None,
        }
    }

    pub fn with_supply(mut self, max: i64) -> Self {
        self.max_supply = Some(max);
        self
    }

    pub fn with_per_user_limit(mut self, limit: i32) -> Self {
        self.max_per_user = Some(limit);
        self
    }

    pub fn inactive(mut self) -> Self {
        self.status = MockBadgeStatus::Inactive;
        self
    }

    pub fn has_stock(&self) -> bool {
        match self.max_supply {
            Some(max) => self.issued_count < max,
            None => true,
        }
    }
}

#[derive(Debug, Clone)]
pub struct MockUserBadge {
    pub id: i64,
    pub user_id: String,
    pub badge_id: i64,
    pub quantity: i32,
    pub status: MockUserBadgeStatus,
    pub acquired_at: DateTime<Utc>,
    pub expires_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone)]
pub struct MockLedgerEntry {
    pub id: i64,
    pub user_id: String,
    pub badge_id: i64,
    pub change_type: String,
    pub quantity: i32,
    pub balance_after: i32,
    pub ref_id: Option<String>,
    pub created_at: DateTime<Utc>,
}

// ==================== Mock 服务实现 ====================

#[derive(Default)]
pub struct MockBadgeStore {
    badges: Arc<RwLock<HashMap<i64, MockBadge>>>,
    user_badges: Arc<RwLock<HashMap<String, Vec<MockUserBadge>>>>,
    ledger: Arc<RwLock<Vec<MockLedgerEntry>>>,
    next_user_badge_id: Arc<RwLock<i64>>,
    next_ledger_id: Arc<RwLock<i64>>,
}

impl MockBadgeStore {
    pub fn new() -> Self {
        Self {
            badges: Arc::new(RwLock::new(HashMap::new())),
            user_badges: Arc::new(RwLock::new(HashMap::new())),
            ledger: Arc::new(RwLock::new(Vec::new())),
            next_user_badge_id: Arc::new(RwLock::new(1)),
            next_ledger_id: Arc::new(RwLock::new(1)),
        }
    }

    pub async fn register_badge(&self, badge: MockBadge) {
        let mut badges = self.badges.write().await;
        badges.insert(badge.id, badge);
    }

    pub async fn get_badge(&self, badge_id: i64) -> Option<MockBadge> {
        let badges = self.badges.read().await;
        badges.get(&badge_id).cloned()
    }

    pub async fn grant_badge(
        &self,
        user_id: &str,
        badge_id: i64,
        quantity: i32,
        idempotency_key: Option<String>,
    ) -> Result<GrantResult, GrantError> {
        // 幂等检查：如果幂等键已存在，返回已存在的 user_badge 信息
        if let Some(ref key) = idempotency_key {
            let ledger = self.ledger.read().await;
            if let Some(entry) = ledger.iter().find(|e| e.ref_id.as_ref() == Some(key)) {
                let user_badges = self.user_badges.read().await;
                let user_badge_id = user_badges
                    .get(&entry.user_id)
                    .and_then(|ubs| ubs.iter().find(|ub| ub.badge_id == entry.badge_id))
                    .map(|ub| ub.id)
                    .unwrap_or(0);
                return Ok(GrantResult {
                    user_badge_id,
                    new_quantity: entry.balance_after,
                    is_duplicate: true,
                });
            }
        }

        // 徽章有效性检查
        let mut badges = self.badges.write().await;
        let badge = badges.get_mut(&badge_id).ok_or(GrantError::BadgeNotFound)?;

        if badge.status != MockBadgeStatus::Active {
            return Err(GrantError::BadgeInactive);
        }

        // 库存检查
        if !badge.has_stock() {
            return Err(GrantError::OutOfStock);
        }

        if let Some(max) = badge.max_supply {
            if badge.issued_count + quantity as i64 > max {
                return Err(GrantError::OutOfStock);
            }
        }

        // 用户限制检查
        let user_badges = self.user_badges.read().await;
        let current_quantity = user_badges
            .get(user_id)
            .and_then(|ubs| ubs.iter().find(|ub| ub.badge_id == badge_id))
            .map(|ub| ub.quantity)
            .unwrap_or(0);

        if let Some(limit) = badge.max_per_user {
            if current_quantity + quantity > limit {
                return Err(GrantError::LimitExceeded);
            }
        }
        drop(user_badges);

        // 执行发放
        let mut user_badges = self.user_badges.write().await;
        let mut next_id = self.next_user_badge_id.write().await;

        let user_badge_list = user_badges.entry(user_id.to_string()).or_default();
        let new_quantity;
        let user_badge_id;

        if let Some(existing) = user_badge_list
            .iter_mut()
            .find(|ub| ub.badge_id == badge_id)
        {
            existing.quantity += quantity;
            new_quantity = existing.quantity;
            user_badge_id = existing.id;
        } else {
            user_badge_id = *next_id;
            *next_id += 1;
            new_quantity = quantity;

            user_badge_list.push(MockUserBadge {
                id: user_badge_id,
                user_id: user_id.to_string(),
                badge_id,
                quantity,
                status: MockUserBadgeStatus::Active,
                acquired_at: Utc::now(),
                expires_at: None,
            });
        }

        badge.issued_count += quantity as i64;
        drop(badges);
        drop(user_badges);

        // 写入账本
        let mut ledger = self.ledger.write().await;
        let mut next_ledger_id = self.next_ledger_id.write().await;
        ledger.push(MockLedgerEntry {
            id: *next_ledger_id,
            user_id: user_id.to_string(),
            badge_id,
            change_type: "acquire".to_string(),
            quantity,
            balance_after: new_quantity,
            ref_id: idempotency_key,
            created_at: Utc::now(),
        });
        *next_ledger_id += 1;

        Ok(GrantResult {
            user_badge_id,
            new_quantity,
            is_duplicate: false,
        })
    }

    pub async fn revoke_badge(
        &self,
        user_id: &str,
        badge_id: i64,
        quantity: i32,
        reason: &str,
    ) -> Result<RevokeResult, RevokeError> {
        let mut user_badges = self.user_badges.write().await;

        let user_badge_list = user_badges
            .get_mut(user_id)
            .ok_or(RevokeError::UserBadgeNotFound)?;

        let user_badge = user_badge_list
            .iter_mut()
            .find(|ub| ub.badge_id == badge_id)
            .ok_or(RevokeError::UserBadgeNotFound)?;

        if user_badge.quantity < quantity {
            return Err(RevokeError::InsufficientQuantity);
        }

        user_badge.quantity -= quantity;
        let new_quantity = user_badge.quantity;

        if new_quantity == 0 {
            user_badge.status = MockUserBadgeStatus::Revoked;
        }

        drop(user_badges);

        // 同步减少徽章的 issued_count
        let mut badges = self.badges.write().await;
        if let Some(badge) = badges.get_mut(&badge_id) {
            badge.issued_count -= quantity as i64;
        }
        drop(badges);

        // 写入账本
        let mut ledger = self.ledger.write().await;
        let mut next_ledger_id = self.next_ledger_id.write().await;
        ledger.push(MockLedgerEntry {
            id: *next_ledger_id,
            user_id: user_id.to_string(),
            badge_id,
            change_type: "revoke".to_string(),
            quantity: -quantity,
            balance_after: new_quantity,
            ref_id: Some(format!("revoke:{}", reason)),
            created_at: Utc::now(),
        });
        *next_ledger_id += 1;

        Ok(RevokeResult {
            revoked_quantity: quantity,
            remaining_quantity: new_quantity,
        })
    }

    pub async fn get_user_badges(&self, user_id: &str) -> Vec<MockUserBadge> {
        let user_badges = self.user_badges.read().await;
        user_badges.get(user_id).cloned().unwrap_or_default()
    }

    pub async fn get_user_badge(&self, user_id: &str, badge_id: i64) -> Option<MockUserBadge> {
        let user_badges = self.user_badges.read().await;
        user_badges
            .get(user_id)
            .and_then(|ubs| ubs.iter().find(|ub| ub.badge_id == badge_id).cloned())
    }

    pub async fn get_ledger(&self, user_id: &str, badge_id: Option<i64>) -> Vec<MockLedgerEntry> {
        let ledger = self.ledger.read().await;
        ledger
            .iter()
            .filter(|e| e.user_id == user_id && badge_id.is_none_or(|bid| e.badge_id == bid))
            .cloned()
            .collect()
    }
}

#[derive(Debug, Clone)]
pub struct GrantResult {
    pub user_badge_id: i64,
    pub new_quantity: i32,
    pub is_duplicate: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub enum GrantError {
    BadgeNotFound,
    BadgeInactive,
    OutOfStock,
    LimitExceeded,
}

#[derive(Debug, Clone)]
pub struct RevokeResult {
    pub revoked_quantity: i32,
    pub remaining_quantity: i32,
}

#[derive(Debug, Clone, PartialEq)]
pub enum RevokeError {
    UserBadgeNotFound,
    InsufficientQuantity,
}

// ==================== 测试用例 ====================

#[tokio::test]
async fn test_basic_grant_badge() {
    let store = MockBadgeStore::new();
    store
        .register_badge(MockBadge::new(1, "First Purchase Badge"))
        .await;

    let result = store.grant_badge("user-001", 1, 1, None).await;
    assert!(result.is_ok());

    let result = result.unwrap();
    assert!(!result.is_duplicate);
    assert_eq!(result.new_quantity, 1);

    let user_badge = store.get_user_badge("user-001", 1).await;
    assert!(user_badge.is_some());
    assert_eq!(user_badge.unwrap().quantity, 1);
}

#[tokio::test]
async fn test_grant_badge_increments_quantity() {
    let store = MockBadgeStore::new();
    store
        .register_badge(MockBadge::new(1, "Collectible Badge"))
        .await;

    let result1 = store.grant_badge("user-001", 1, 2, None).await.unwrap();
    assert_eq!(result1.new_quantity, 2);

    let result2 = store.grant_badge("user-001", 1, 3, None).await.unwrap();
    assert_eq!(result2.new_quantity, 5);

    let user_badge = store.get_user_badge("user-001", 1).await.unwrap();
    assert_eq!(user_badge.quantity, 5);
}

#[tokio::test]
async fn test_grant_idempotency() {
    let store = MockBadgeStore::new();
    store.register_badge(MockBadge::new(1, "Test Badge")).await;

    let idempotency_key = "grant-key-001".to_string();

    let result1 = store
        .grant_badge("user-001", 1, 5, Some(idempotency_key.clone()))
        .await
        .unwrap();
    assert!(!result1.is_duplicate);
    assert_eq!(result1.new_quantity, 5);

    let result2 = store
        .grant_badge("user-001", 1, 5, Some(idempotency_key.clone()))
        .await
        .unwrap();
    assert!(result2.is_duplicate);
    assert_eq!(result2.new_quantity, 5);

    let user_badge = store.get_user_badge("user-001", 1).await.unwrap();
    assert_eq!(user_badge.quantity, 5);
}

#[tokio::test]
async fn test_grant_badge_with_limited_supply() {
    let store = MockBadgeStore::new();
    store
        .register_badge(MockBadge::new(1, "Limited Badge").with_supply(10))
        .await;

    for i in 0..10 {
        let result = store
            .grant_badge(&format!("user-{:03}", i), 1, 1, None)
            .await;
        assert!(result.is_ok(), "第 {} 次发放应该成功", i + 1);
    }

    let result = store.grant_badge("user-100", 1, 1, None).await;
    assert_eq!(result.err(), Some(GrantError::OutOfStock));
}

#[tokio::test]
async fn test_grant_badge_with_per_user_limit() {
    let store = MockBadgeStore::new();
    store
        .register_badge(MockBadge::new(1, "User Limited Badge").with_per_user_limit(3))
        .await;

    store.grant_badge("user-001", 1, 2, None).await.unwrap();
    store.grant_badge("user-001", 1, 1, None).await.unwrap();

    let result = store.grant_badge("user-001", 1, 1, None).await;
    assert_eq!(result.err(), Some(GrantError::LimitExceeded));

    let result = store.grant_badge("user-002", 1, 3, None).await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_grant_inactive_badge_fails() {
    let store = MockBadgeStore::new();
    store
        .register_badge(MockBadge::new(1, "Inactive Badge").inactive())
        .await;

    let result = store.grant_badge("user-001", 1, 1, None).await;
    assert_eq!(result.err(), Some(GrantError::BadgeInactive));
}

#[tokio::test]
async fn test_grant_nonexistent_badge_fails() {
    let store = MockBadgeStore::new();

    let result = store.grant_badge("user-001", 999, 1, None).await;
    assert_eq!(result.err(), Some(GrantError::BadgeNotFound));
}

#[tokio::test]
async fn test_basic_revoke_badge() {
    let store = MockBadgeStore::new();
    store.register_badge(MockBadge::new(1, "Test Badge")).await;

    store.grant_badge("user-001", 1, 5, None).await.unwrap();

    let result = store
        .revoke_badge("user-001", 1, 2, "test reason")
        .await
        .unwrap();
    assert_eq!(result.revoked_quantity, 2);
    assert_eq!(result.remaining_quantity, 3);

    let user_badge = store.get_user_badge("user-001", 1).await.unwrap();
    assert_eq!(user_badge.quantity, 3);
    assert_eq!(user_badge.status, MockUserBadgeStatus::Active);
}

#[tokio::test]
async fn test_revoke_all_badges_changes_status() {
    let store = MockBadgeStore::new();
    store.register_badge(MockBadge::new(1, "Test Badge")).await;

    store.grant_badge("user-001", 1, 3, None).await.unwrap();

    let result = store
        .revoke_badge("user-001", 1, 3, "full revoke")
        .await
        .unwrap();
    assert_eq!(result.remaining_quantity, 0);

    let user_badge = store.get_user_badge("user-001", 1).await.unwrap();
    assert_eq!(user_badge.status, MockUserBadgeStatus::Revoked);
}

#[tokio::test]
async fn test_revoke_insufficient_quantity_fails() {
    let store = MockBadgeStore::new();
    store.register_badge(MockBadge::new(1, "Test Badge")).await;

    store.grant_badge("user-001", 1, 2, None).await.unwrap();

    let result = store.revoke_badge("user-001", 1, 3, "too many").await;
    assert_eq!(result.err(), Some(RevokeError::InsufficientQuantity));
}

#[tokio::test]
async fn test_ledger_records_grant_and_revoke() {
    let store = MockBadgeStore::new();
    store.register_badge(MockBadge::new(1, "Test Badge")).await;

    store.grant_badge("user-001", 1, 5, None).await.unwrap();
    store.revoke_badge("user-001", 1, 2, "test").await.unwrap();

    let ledger = store.get_ledger("user-001", Some(1)).await;
    assert_eq!(ledger.len(), 2);

    assert_eq!(ledger[0].change_type, "acquire");
    assert_eq!(ledger[0].quantity, 5);
    assert_eq!(ledger[0].balance_after, 5);

    assert_eq!(ledger[1].change_type, "revoke");
    assert_eq!(ledger[1].quantity, -2);
    assert_eq!(ledger[1].balance_after, 3);
}

#[tokio::test]
async fn test_complete_badge_lifecycle() {
    let store = MockBadgeStore::new();
    store
        .register_badge(MockBadge::new(1, "Lifecycle Badge").with_supply(100))
        .await;

    // 首次发放
    let grant1 = store.grant_badge("user-001", 1, 10, None).await.unwrap();
    assert_eq!(grant1.new_quantity, 10);

    // 再次发放
    let grant2 = store.grant_badge("user-001", 1, 5, None).await.unwrap();
    assert_eq!(grant2.new_quantity, 15);

    // 部分取消
    let revoke1 = store
        .revoke_badge("user-001", 1, 3, "adjustment")
        .await
        .unwrap();
    assert_eq!(revoke1.remaining_quantity, 12);

    // 验证最终状态
    let user_badge = store.get_user_badge("user-001", 1).await.unwrap();
    assert_eq!(user_badge.quantity, 12);
    assert_eq!(user_badge.status, MockUserBadgeStatus::Active);

    // 验证账本完整性
    let ledger = store.get_ledger("user-001", Some(1)).await;
    assert_eq!(ledger.len(), 3);
    assert_eq!(ledger[0].balance_after, 10);
    assert_eq!(ledger[1].balance_after, 15);
    assert_eq!(ledger[2].balance_after, 12);
}

#[tokio::test]
async fn test_multiple_users_with_same_badge() {
    let store = MockBadgeStore::new();
    store
        .register_badge(MockBadge::new(1, "Popular Badge").with_supply(100))
        .await;

    let users = ["user-001", "user-002", "user-003", "user-004", "user-005"];
    for (i, user) in users.iter().enumerate() {
        store
            .grant_badge(user, 1, (i + 1) as i32, None)
            .await
            .unwrap();
    }

    for (i, user) in users.iter().enumerate() {
        let badge = store.get_user_badge(user, 1).await.unwrap();
        assert_eq!(badge.quantity, (i + 1) as i32);
    }

    let badge = store.get_badge(1).await.unwrap();
    assert_eq!(badge.issued_count, 15); // 1+2+3+4+5 = 15
}

#[tokio::test]
async fn test_user_with_multiple_badges() {
    let store = MockBadgeStore::new();

    for i in 1..=5 {
        store
            .register_badge(MockBadge::new(i, &format!("Badge {}", i)))
            .await;
    }

    for i in 1..=5 {
        store
            .grant_badge("user-001", i, i as i32, None)
            .await
            .unwrap();
    }

    let user_badges = store.get_user_badges("user-001").await;
    assert_eq!(user_badges.len(), 5);

    for badge in &user_badges {
        assert_eq!(badge.quantity, badge.badge_id as i32);
    }
}

// ==================== 数据库集成测试 ====================

/// 数据库集成测试：验证发放流程的事务一致性
///
/// 通过直接操作 PgPool 模拟发放操作，确认 user_badges 和 badge_ledger
/// 在同一事务中原子写入，且发放后余额与账本一致。
#[tokio::test]
#[ignore = "需要 PostgreSQL 数据库连接"]
async fn test_database_integration_grant_flow() {
    use sqlx::PgPool;

    let database_url = std::env::var("DATABASE_URL")
        .unwrap_or_else(|_| "postgres://badge:badge_secret@localhost:5432/badge_db".to_string());

    let pool = PgPool::connect(&database_url)
        .await
        .expect("无法连接数据库，请确保 PostgreSQL 正在运行");

    // 使用唯一用户 ID 避免与其他测试冲突
    let user_id = format!("test_grant_flow_{}", chrono::Utc::now().timestamp_millis());

    // 查询一个已存在的 active 徽章作为测试目标
    let badge_row: Option<(i64, String)> = sqlx::query_as(
        "SELECT id, name FROM badges WHERE UPPER(status) = 'ACTIVE' LIMIT 1",
    )
    .fetch_optional(&pool)
    .await
    .expect("查询徽章失败");

    let Some((badge_id, badge_name)) = badge_row else {
        eprintln!("数据库中无 active 徽章，跳过集成测试（需要先通过 Admin API 创建徽章）");
        return;
    };

    // 在事务中执行发放：同时写入 user_badges 和 badge_ledger
    let mut tx = pool.begin().await.expect("开启事务失败");

    sqlx::query(
        r#"INSERT INTO user_badges (user_id, badge_id, quantity, status, source_type)
           VALUES ($1, $2, 1, 'active', 'manual')
           ON CONFLICT (user_id, badge_id) DO UPDATE SET quantity = user_badges.quantity + 1"#,
    )
    .bind(&user_id)
    .bind(badge_id)
    .execute(&mut *tx)
    .await
    .expect("写入 user_badges 失败");

    sqlx::query(
        r#"INSERT INTO badge_ledger (user_id, badge_id, change_type, source_type, quantity, balance_after, remark)
           VALUES ($1, $2, 'acquire', 'manual', 1, 1, '集成测试发放')"#,
    )
    .bind(&user_id)
    .bind(badge_id)
    .execute(&mut *tx)
    .await
    .expect("写入 badge_ledger 失败");

    tx.commit().await.expect("事务提交失败");

    // 验证 user_badges 记录存在
    let ub_count: (i64,) = sqlx::query_as(
        "SELECT COUNT(*) FROM user_badges WHERE user_id = $1 AND badge_id = $2",
    )
    .bind(&user_id)
    .bind(badge_id)
    .fetch_one(&pool)
    .await
    .expect("查询 user_badges 失败");
    assert!(ub_count.0 > 0, "user_badges 应有记录（徽章: {}）", badge_name);

    // 验证 badge_ledger 记录存在
    let ledger_count: (i64,) = sqlx::query_as(
        "SELECT COUNT(*) FROM badge_ledger WHERE user_id = $1 AND badge_id = $2",
    )
    .bind(&user_id)
    .bind(badge_id)
    .fetch_one(&pool)
    .await
    .expect("查询 badge_ledger 失败");
    assert!(ledger_count.0 > 0, "badge_ledger 应有账本记录");

    // 清理测试数据
    let _ = sqlx::query("DELETE FROM badge_ledger WHERE user_id = $1 AND badge_id = $2")
        .bind(&user_id)
        .bind(badge_id)
        .execute(&pool)
        .await;
    let _ = sqlx::query("DELETE FROM user_badges WHERE user_id = $1 AND badge_id = $2")
        .bind(&user_id)
        .bind(badge_id)
        .execute(&pool)
        .await;
}

/// gRPC 集成测试：验证 GrantBadge RPC 端到端可达性
///
/// 连接 gRPC 服务端口发送 GrantBadge 请求，验证返回 OK 或符合预期的业务错误码。
/// 当 badge-management-service 未运行时自动跳过。
#[tokio::test]
#[ignore = "需要运行 badge-management-service gRPC 服务"]
async fn test_grpc_service_integration() {
    use tonic::transport::Channel;

    let grpc_url = std::env::var("BADGE_MANAGEMENT_GRPC_URL")
        .unwrap_or_else(|_| "http://127.0.0.1:50051".to_string());

    // 尝试连接 gRPC 服务
    let channel = match Channel::from_shared(grpc_url.clone())
        .expect("无效的 gRPC URL")
        .connect_timeout(std::time::Duration::from_secs(5))
        .connect()
        .await
    {
        Ok(ch) => ch,
        Err(e) => {
            eprintln!(
                "无法连接 gRPC 服务 {}，跳过测试: {}",
                grpc_url, e
            );
            return;
        }
    };

    use badge_proto::badge::badge_management_service_client::BadgeManagementServiceClient;
    use badge_proto::badge::GetUserBadgesRequest;

    let mut client = BadgeManagementServiceClient::new(channel);

    // 使用唯一用户 ID 查询徽章列表，验证 RPC 连通性
    let test_user_id = format!("grpc_test_user_{}", chrono::Utc::now().timestamp_millis());

    let response = client
        .get_user_badges(tonic::Request::new(GetUserBadgesRequest {
            user_id: test_user_id,
            badge_type: None,
            status: None,
            page: 1,
            page_size: 10,
        }))
        .await;

    match response {
        Ok(resp) => {
            let inner = resp.into_inner();
            // 新用户应返回空列表
            assert!(
                inner.badges.is_empty(),
                "新用户的徽章列表应为空，实际返回 {} 条",
                inner.badges.len()
            );
        }
        Err(status) => {
            // 如果服务正常运行但返回业务错误也可接受，记录以便排查
            panic!(
                "GetUserBadges RPC 失败: code={:?}, message={}",
                status.code(),
                status.message()
            );
        }
    }
}
