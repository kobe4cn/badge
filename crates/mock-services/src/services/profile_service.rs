//! Mock Profile 服务
//!
//! 模拟用户资料服务的 REST API，提供用户 CRUD 和会员等级管理功能。

use axum::{
    Json, Router,
    extract::{Path, State},
    http::StatusCode,
    routing::{get, post, put},
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

use crate::models::user::{MembershipLevel, MockUser};
use crate::store::MemoryStore;

/// Profile 服务状态
///
/// 包含用户数据的内存存储，支持高并发访问
pub struct ProfileServiceState {
    pub users: MemoryStore<MockUser>,
}

impl ProfileServiceState {
    pub fn new() -> Self {
        Self {
            users: MemoryStore::new(),
        }
    }
}

impl Default for ProfileServiceState {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// 请求/响应 DTO
// ============================================================================

/// 创建用户请求
#[derive(Debug, Deserialize)]
pub struct CreateUserRequest {
    /// 用户名，可选。不提供则随机生成
    pub username: Option<String>,
    pub email: Option<String>,
}

/// 更新用户资料请求
#[derive(Debug, Deserialize)]
pub struct UpdateProfileRequest {
    pub username: Option<String>,
    pub email: Option<String>,
    pub phone: Option<String>,
}

/// 升级会员等级请求
#[derive(Debug, Deserialize)]
pub struct UpgradeMembershipRequest {
    pub level: MembershipLevel,
}

/// 用户资料响应
#[derive(Debug, Serialize)]
pub struct ProfileResponse {
    pub user: MockUser,
}

/// 会员等级响应
#[derive(Debug, Serialize)]
pub struct MembershipResponse {
    pub user_id: String,
    pub level: MembershipLevel,
    pub total_spent: f64,
    /// 下一等级，Diamond 用户返回 None
    pub next_level: Option<MembershipLevel>,
    /// 距离下一等级还需消费的金额
    pub amount_to_next_level: Option<f64>,
}

/// 用户列表响应
#[derive(Debug, Serialize)]
pub struct UserListResponse {
    pub users: Vec<MockUser>,
    pub total: usize,
}

/// API 错误响应
#[derive(Debug, Serialize)]
pub struct ErrorResponse {
    pub error: String,
}

// ============================================================================
// 路由配置
// ============================================================================

/// 构建 Profile 服务路由
pub fn profile_routes() -> Router<Arc<ProfileServiceState>> {
    Router::new()
        .route("/users", get(list_users))
        .route("/users", post(create_user))
        .route("/users/{user_id}", get(get_profile))
        .route("/users/{user_id}", put(update_profile))
        .route("/users/{user_id}/membership", get(get_membership))
        .route("/users/{user_id}/membership", post(upgrade_membership))
}

// ============================================================================
// Handler 实现
// ============================================================================

/// 获取用户资料
async fn get_profile(
    State(state): State<Arc<ProfileServiceState>>,
    Path(user_id): Path<String>,
) -> Result<Json<ProfileResponse>, (StatusCode, Json<ErrorResponse>)> {
    tracing::info!(user_id = %user_id, "获取用户资料");

    state
        .users
        .get(&user_id)
        .map(|user| Json(ProfileResponse { user }))
        .ok_or_else(|| {
            tracing::warn!(user_id = %user_id, "用户不存在");
            (
                StatusCode::NOT_FOUND,
                Json(ErrorResponse {
                    error: format!("User {} not found", user_id),
                }),
            )
        })
}

/// 更新用户资料
async fn update_profile(
    State(state): State<Arc<ProfileServiceState>>,
    Path(user_id): Path<String>,
    Json(req): Json<UpdateProfileRequest>,
) -> Result<Json<ProfileResponse>, (StatusCode, Json<ErrorResponse>)> {
    tracing::info!(user_id = %user_id, "更新用户资料");

    let mut user = state.users.get(&user_id).ok_or_else(|| {
        tracing::warn!(user_id = %user_id, "用户不存在");
        (
            StatusCode::NOT_FOUND,
            Json(ErrorResponse {
                error: format!("User {} not found", user_id),
            }),
        )
    })?;

    // 仅更新请求中提供的字段
    if let Some(username) = req.username {
        user.username = username;
    }
    if let Some(email) = req.email {
        user.email = email;
    }
    if let Some(phone) = req.phone {
        user.phone = Some(phone);
    }

    state.users.insert(&user_id, user.clone());
    tracing::info!(user_id = %user_id, "用户资料更新成功");

    Ok(Json(ProfileResponse { user }))
}

/// 创建用户
///
/// 支持两种模式：
/// 1. 提供用户名/邮箱创建指定用户
/// 2. 不提供任何参数则生成随机用户
async fn create_user(
    State(state): State<Arc<ProfileServiceState>>,
    Json(req): Json<CreateUserRequest>,
) -> (StatusCode, Json<ProfileResponse>) {
    let mut user = MockUser::random();

    // 使用请求中的值覆盖随机生成的值
    if let Some(username) = req.username {
        user.username = username;
    }
    if let Some(email) = req.email {
        user.email = email;
    }

    let user_id = user.user_id.clone();
    state.users.insert(&user_id, user.clone());

    tracing::info!(
        user_id = %user_id,
        username = %user.username,
        "创建新用户"
    );

    (StatusCode::CREATED, Json(ProfileResponse { user }))
}

/// 列出所有用户
async fn list_users(State(state): State<Arc<ProfileServiceState>>) -> Json<UserListResponse> {
    let users = state.users.list();
    let total = users.len();

    tracing::info!(count = total, "列出所有用户");

    Json(UserListResponse { users, total })
}

/// 获取会员等级信息
async fn get_membership(
    State(state): State<Arc<ProfileServiceState>>,
    Path(user_id): Path<String>,
) -> Result<Json<MembershipResponse>, (StatusCode, Json<ErrorResponse>)> {
    tracing::info!(user_id = %user_id, "获取会员等级信息");

    let user = state.users.get(&user_id).ok_or_else(|| {
        tracing::warn!(user_id = %user_id, "用户不存在");
        (
            StatusCode::NOT_FOUND,
            Json(ErrorResponse {
                error: format!("User {} not found", user_id),
            }),
        )
    })?;

    let next_level = user.membership_level.next_level();
    let amount_to_next_level = user
        .membership_level
        .next_level_threshold()
        .map(|threshold| (threshold - user.total_spent).max(0.0));

    Ok(Json(MembershipResponse {
        user_id: user.user_id,
        level: user.membership_level,
        total_spent: user.total_spent,
        next_level,
        amount_to_next_level,
    }))
}

/// 升级会员等级
///
/// 允许管理员手动设置用户的会员等级，不受消费金额限制
async fn upgrade_membership(
    State(state): State<Arc<ProfileServiceState>>,
    Path(user_id): Path<String>,
    Json(req): Json<UpgradeMembershipRequest>,
) -> Result<Json<MembershipResponse>, (StatusCode, Json<ErrorResponse>)> {
    tracing::info!(
        user_id = %user_id,
        new_level = ?req.level,
        "升级会员等级"
    );

    let mut user = state.users.get(&user_id).ok_or_else(|| {
        tracing::warn!(user_id = %user_id, "用户不存在");
        (
            StatusCode::NOT_FOUND,
            Json(ErrorResponse {
                error: format!("User {} not found", user_id),
            }),
        )
    })?;

    user.membership_level = req.level;
    state.users.insert(&user_id, user.clone());

    let next_level = user.membership_level.next_level();
    let amount_to_next_level = user
        .membership_level
        .next_level_threshold()
        .map(|threshold| (threshold - user.total_spent).max(0.0));

    tracing::info!(
        user_id = %user_id,
        level = ?user.membership_level,
        "会员等级升级成功"
    );

    Ok(Json(MembershipResponse {
        user_id: user.user_id,
        level: user.membership_level,
        total_spent: user.total_spent,
        next_level,
        amount_to_next_level,
    }))
}

// ============================================================================
// 单元测试
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_state() -> Arc<ProfileServiceState> {
        Arc::new(ProfileServiceState::new())
    }

    #[test]
    fn test_create_user_random() {
        let state = create_test_state();

        // 模拟创建随机用户（不提供任何参数）
        let user = MockUser::random();
        state.users.insert(&user.user_id, user.clone());

        let retrieved = state.users.get(&user.user_id).unwrap();
        assert_eq!(retrieved.user_id, user.user_id);
        assert!(!retrieved.username.is_empty());
        assert!(retrieved.email.contains('@'));
    }

    #[test]
    fn test_get_profile() {
        let state = create_test_state();

        // 预先创建用户
        let user = MockUser::random_with_id("test-user-123");
        state.users.insert(&user.user_id, user.clone());

        // 验证能够获取
        let retrieved = state.users.get("test-user-123").unwrap();
        assert_eq!(retrieved.user_id, "test-user-123");

        // 验证不存在的用户返回 None
        assert!(state.users.get("non-existent").is_none());
    }

    #[test]
    fn test_update_profile() {
        let state = create_test_state();

        // 创建用户
        let mut user = MockUser::random_with_id("update-test-user");
        user.username = "original_name".to_string();
        user.email = "original@test.com".to_string();
        let user_id = user.user_id.clone();
        state.users.insert(&user_id, user);

        // 更新用户资料
        let mut updated = state.users.get("update-test-user").unwrap();
        updated.username = "new_name".to_string();
        updated.email = "new@test.com".to_string();
        updated.phone = Some("123-456-7890".to_string());
        state.users.insert("update-test-user", updated);

        // 验证更新结果
        let result = state.users.get("update-test-user").unwrap();
        assert_eq!(result.username, "new_name");
        assert_eq!(result.email, "new@test.com");
        assert_eq!(result.phone, Some("123-456-7890".to_string()));
    }

    #[test]
    fn test_membership_calculation() {
        // 测试会员等级计算
        assert_eq!(MembershipLevel::from_spent(0.0), MembershipLevel::Bronze);
        assert_eq!(MembershipLevel::from_spent(500.0), MembershipLevel::Bronze);
        assert_eq!(MembershipLevel::from_spent(1000.0), MembershipLevel::Silver);
        assert_eq!(MembershipLevel::from_spent(3000.0), MembershipLevel::Silver);
        assert_eq!(MembershipLevel::from_spent(5000.0), MembershipLevel::Gold);
        assert_eq!(MembershipLevel::from_spent(15000.0), MembershipLevel::Gold);
        assert_eq!(
            MembershipLevel::from_spent(20000.0),
            MembershipLevel::Platinum
        );
        assert_eq!(
            MembershipLevel::from_spent(40000.0),
            MembershipLevel::Platinum
        );
        assert_eq!(
            MembershipLevel::from_spent(50000.0),
            MembershipLevel::Diamond
        );
        assert_eq!(
            MembershipLevel::from_spent(100000.0),
            MembershipLevel::Diamond
        );

        // 测试下一等级阈值
        assert_eq!(MembershipLevel::Bronze.next_level_threshold(), Some(1000.0));
        assert_eq!(MembershipLevel::Silver.next_level_threshold(), Some(5000.0));
        assert_eq!(MembershipLevel::Gold.next_level_threshold(), Some(20000.0));
        assert_eq!(
            MembershipLevel::Platinum.next_level_threshold(),
            Some(50000.0)
        );
        assert_eq!(MembershipLevel::Diamond.next_level_threshold(), None);

        // 测试下一等级
        assert_eq!(
            MembershipLevel::Bronze.next_level(),
            Some(MembershipLevel::Silver)
        );
        assert_eq!(
            MembershipLevel::Silver.next_level(),
            Some(MembershipLevel::Gold)
        );
        assert_eq!(
            MembershipLevel::Gold.next_level(),
            Some(MembershipLevel::Platinum)
        );
        assert_eq!(
            MembershipLevel::Platinum.next_level(),
            Some(MembershipLevel::Diamond)
        );
        assert_eq!(MembershipLevel::Diamond.next_level(), None);
    }

    #[test]
    fn test_upgrade_membership() {
        let state = create_test_state();

        // 创建 Bronze 用户
        let mut user = MockUser::random_with_id("upgrade-test-user");
        user.membership_level = MembershipLevel::Bronze;
        user.total_spent = 500.0;
        let user_id = user.user_id.clone();
        state.users.insert(&user_id, user);

        // 升级到 Gold
        let mut updated = state.users.get("upgrade-test-user").unwrap();
        updated.membership_level = MembershipLevel::Gold;
        state.users.insert("upgrade-test-user", updated);

        // 验证升级结果
        let result = state.users.get("upgrade-test-user").unwrap();
        assert_eq!(result.membership_level, MembershipLevel::Gold);
        // 消费金额不变
        assert_eq!(result.total_spent, 500.0);
    }

    #[test]
    fn test_list_users() {
        let state = create_test_state();

        // 创建多个用户
        for i in 0..5 {
            let user = MockUser::random_with_id(&format!("list-test-user-{}", i));
            let user_id = user.user_id.clone();
            state.users.insert(&user_id, user);
        }

        let users = state.users.list();
        assert_eq!(users.len(), 5);
    }

    #[test]
    fn test_amount_to_next_level_calculation() {
        // 测试距离下一等级的金额计算
        let level = MembershipLevel::Bronze;
        let total_spent = 300.0;
        let threshold = level.next_level_threshold().unwrap();
        let amount_to_next = (threshold - total_spent).max(0.0);
        assert_eq!(amount_to_next, 700.0);

        // 消费超过阈值时返回 0
        let total_spent_over = 1200.0;
        let amount_to_next_over = (threshold - total_spent_over).max(0.0);
        assert_eq!(amount_to_next_over, 0.0);
    }
}
