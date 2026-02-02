//! 会员视图 API 处理器
//!
//! 提供以用户为中心的徽章查看能力：
//! 用户徽章列表、兑换记录、统计汇总、账本流水。

use axum::{
    Json,
    extract::{Path, Query, State},
};
use chrono::{DateTime, Utc};
use tracing::instrument;

use serde::Deserialize;

use crate::{
    dto::{
        ApiResponse, PageResponse, PaginationParams, UserBadgeAdminDto, UserLedgerDto,
        UserRedemptionDto, UserStatsDto,
    },
    error::AdminError,
    state::AppState,
};

/// 用户搜索查询参数
#[derive(Debug, Deserialize)]
pub struct UserSearchParams {
    /// 搜索关键词（用户 ID 模糊匹配）
    pub keyword: String,
}

/// 用户简要信息 DTO
#[derive(Debug, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UserDto {
    pub user_id: String,
    pub nickname: Option<String>,
    pub phone: Option<String>,
    pub badge_count: i64,
}

// ---------------------------------------------------------------------------
// 用户搜索
// ---------------------------------------------------------------------------

/// 用户搜索结果行
#[derive(sqlx::FromRow)]
struct UserSearchRow {
    user_id: String,
    badge_count: i64,
}

/// 搜索用户
///
/// GET /api/admin/users/search
///
/// 在 user_badges 表中搜索用户 ID，返回匹配的用户列表
/// 注：完整用户信息（昵称、手机号等）需从用户中心服务获取
#[instrument(skip(state))]
pub async fn search_users(
    State(state): State<AppState>,
    Query(params): Query<UserSearchParams>,
) -> Result<Json<ApiResponse<Vec<UserDto>>>, AdminError> {
    let keyword = format!("%{}%", params.keyword);

    let rows = sqlx::query_as::<_, UserSearchRow>(
        r#"
        SELECT
            user_id,
            COUNT(*) as badge_count
        FROM user_badges
        WHERE user_id ILIKE $1
        GROUP BY user_id
        ORDER BY badge_count DESC
        LIMIT 20
        "#,
    )
    .bind(&keyword)
    .fetch_all(&state.pool)
    .await?;

    let users: Vec<UserDto> = rows
        .into_iter()
        .map(|row| UserDto {
            user_id: row.user_id,
            // 昵称和手机号需从用户中心服务获取，此处暂时返回空
            nickname: None,
            phone: None,
            badge_count: row.badge_count,
        })
        .collect();

    Ok(Json(ApiResponse::success(users)))
}

/// 用户详情 DTO
#[derive(Debug, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UserDetailDto {
    pub user_id: String,
    pub nickname: Option<String>,
    pub phone: Option<String>,
    pub total_badges: i64,
    pub active_badges: i64,
    pub total_redeemed: i64,
}

/// 获取用户详情
///
/// GET /api/admin/users/:id
///
/// 返回用户基本信息和徽章统计
#[instrument(skip(state))]
pub async fn get_user_detail(
    State(state): State<AppState>,
    Path(user_id): Path<String>,
) -> Result<Json<ApiResponse<UserDetailDto>>, AdminError> {
    // 注意：UserBadgeStatus 枚举使用 UPPER() 确保大小写不敏感
    let stats: Option<(i64, i64, i64)> = sqlx::query_as(
        r#"
        SELECT
            COUNT(*) as total_badges,
            COUNT(*) FILTER (WHERE UPPER(status) = 'ACTIVE') as active_badges,
            COALESCE((
                SELECT COUNT(*) FROM redemption_orders WHERE user_id = $1
            ), 0) as total_redeemed
        FROM user_badges
        WHERE user_id = $1
        "#,
    )
    .bind(&user_id)
    .fetch_optional(&state.pool)
    .await?;

    let (total_badges, active_badges, total_redeemed) = stats.unwrap_or((0, 0, 0));

    let dto = UserDetailDto {
        user_id,
        // 昵称和手机号需从用户中心服务获取
        nickname: None,
        phone: None,
        total_badges,
        active_badges,
        total_redeemed,
    };

    Ok(Json(ApiResponse::success(dto)))
}

// ---------------------------------------------------------------------------
// 用户徽章列表
// ---------------------------------------------------------------------------

/// 用户徽章行
#[derive(sqlx::FromRow)]
struct UserBadgeRow {
    badge_id: i64,
    badge_name: String,
    badge_type: String,
    quantity: i32,
    status: String,
    acquired_at: DateTime<Utc>,
    expires_at: Option<DateTime<Utc>>,
}

/// 查询用户持有的所有徽章
///
/// GET /api/admin/users/:id/badges
///
/// 关联 user_badges 和 badges 表，返回徽章基本信息、
/// 持有数量、状态和获取/过期时间。
#[instrument(skip(state))]
pub async fn get_user_badges(
    State(state): State<AppState>,
    Path(user_id): Path<String>,
    Query(pagination): Query<PaginationParams>,
) -> Result<Json<ApiResponse<PageResponse<UserBadgeAdminDto>>>, AdminError> {
    let offset = pagination.offset();
    let limit = pagination.limit();

    let total: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM user_badges WHERE user_id = $1")
        .bind(&user_id)
        .fetch_one(&state.pool)
        .await?;

    if total.0 == 0 {
        return Ok(Json(ApiResponse::success(PageResponse::empty(
            pagination.page,
            pagination.page_size,
        ))));
    }

    let rows = sqlx::query_as::<_, UserBadgeRow>(
        r#"
        SELECT
            ub.badge_id,
            b.name as badge_name,
            b.badge_type::text as badge_type,
            ub.quantity,
            ub.status::text as status,
            ub.first_acquired_at as acquired_at,
            ub.expires_at
        FROM user_badges ub
        JOIN badges b ON b.id = ub.badge_id
        WHERE ub.user_id = $1
        ORDER BY ub.first_acquired_at DESC
        LIMIT $2 OFFSET $3
        "#,
    )
    .bind(&user_id)
    .bind(limit)
    .bind(offset)
    .fetch_all(&state.pool)
    .await?;

    let items: Vec<UserBadgeAdminDto> = rows
        .into_iter()
        .map(|row| UserBadgeAdminDto {
            badge_id: row.badge_id,
            badge_name: row.badge_name,
            badge_type: row.badge_type,
            quantity: row.quantity,
            status: row.status,
            acquired_at: row.acquired_at,
            expires_at: row.expires_at,
        })
        .collect();

    let response = PageResponse::new(items, total.0, pagination.page, pagination.page_size);
    Ok(Json(ApiResponse::success(response)))
}

// ---------------------------------------------------------------------------
// 用户兑换记录
// ---------------------------------------------------------------------------

/// 兑换记录行
#[derive(sqlx::FromRow)]
struct RedemptionRow {
    order_id: i64,
    order_no: String,
    benefit_name: String,
    status: String,
    created_at: DateTime<Utc>,
}

/// 查询用户兑换订单记录
///
/// GET /api/admin/users/:id/redemptions
///
/// 从 redemption_orders 表获取该用户的兑换历史，
/// 关联 benefits 表获取权益名称。
#[instrument(skip(state))]
pub async fn get_user_redemptions(
    State(state): State<AppState>,
    Path(user_id): Path<String>,
    Query(pagination): Query<PaginationParams>,
) -> Result<Json<ApiResponse<PageResponse<UserRedemptionDto>>>, AdminError> {
    let offset = pagination.offset();
    let limit = pagination.limit();

    let total: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM redemption_orders WHERE user_id = $1")
        .bind(&user_id)
        .fetch_one(&state.pool)
        .await?;

    if total.0 == 0 {
        return Ok(Json(ApiResponse::success(PageResponse::empty(
            pagination.page,
            pagination.page_size,
        ))));
    }

    let rows = sqlx::query_as::<_, RedemptionRow>(
        r#"
        SELECT
            ro.id as order_id,
            ro.order_no,
            COALESCE(bf.name, '未知权益') as benefit_name,
            ro.status::text as status,
            ro.created_at
        FROM redemption_orders ro
        LEFT JOIN benefits bf ON bf.id = ro.benefit_id
        WHERE ro.user_id = $1
        ORDER BY ro.created_at DESC
        LIMIT $2 OFFSET $3
        "#,
    )
    .bind(&user_id)
    .bind(limit)
    .bind(offset)
    .fetch_all(&state.pool)
    .await?;

    let items: Vec<UserRedemptionDto> = rows
        .into_iter()
        .map(|row| UserRedemptionDto {
            order_id: row.order_id,
            order_no: row.order_no,
            benefit_name: row.benefit_name,
            status: row.status,
            created_at: row.created_at,
        })
        .collect();

    let response = PageResponse::new(items, total.0, pagination.page, pagination.page_size);
    Ok(Json(ApiResponse::success(response)))
}

// ---------------------------------------------------------------------------
// 用户统计
// ---------------------------------------------------------------------------

/// 用户统计行
#[derive(sqlx::FromRow)]
struct UserStatsRow {
    total_badges: i64,
    active_badges: i64,
    expired_badges: i64,
    total_redeemed: i64,
}

/// 查询用户徽章统计汇总
///
/// GET /api/admin/users/:id/stats
///
/// 聚合用户的徽章持有状况（总数/活跃/过期）和兑换次数。
#[instrument(skip(state))]
pub async fn get_user_stats(
    State(state): State<AppState>,
    Path(user_id): Path<String>,
) -> Result<Json<ApiResponse<UserStatsDto>>, AdminError> {
    // 注意：UserBadgeStatus 枚举使用 SCREAMING_SNAKE_CASE
    let stats = sqlx::query_as::<_, UserStatsRow>(
        r#"
        SELECT
            COUNT(*) as total_badges,
            COUNT(*) FILTER (WHERE UPPER(status) = 'ACTIVE') as active_badges,
            COUNT(*) FILTER (WHERE UPPER(status) = 'EXPIRED') as expired_badges,
            COALESCE((
                SELECT COUNT(*) FROM redemption_orders WHERE user_id = $1
            ), 0) as total_redeemed
        FROM user_badges
        WHERE user_id = $1
        "#,
    )
    .bind(&user_id)
    .fetch_one(&state.pool)
    .await?;

    let dto = UserStatsDto {
        user_id,
        total_badges: stats.total_badges,
        active_badges: stats.active_badges,
        expired_badges: stats.expired_badges,
        total_redeemed: stats.total_redeemed,
    };

    Ok(Json(ApiResponse::success(dto)))
}

// ---------------------------------------------------------------------------
// 用户账本流水
// ---------------------------------------------------------------------------

/// 账本流水行
#[derive(sqlx::FromRow)]
struct LedgerRow {
    id: i64,
    badge_id: i64,
    badge_name: String,
    change_type: String,
    source_type: String,
    quantity: i32,
    remark: Option<String>,
    created_at: DateTime<Utc>,
}

/// 查询用户账本流水
///
/// GET /api/admin/users/:id/ledger
///
/// 返回用户在 badge_ledger 中的所有变动记录，
/// 包括获取、取消、兑换等所有类型。
#[instrument(skip(state))]
pub async fn get_user_ledger(
    State(state): State<AppState>,
    Path(user_id): Path<String>,
    Query(pagination): Query<PaginationParams>,
) -> Result<Json<ApiResponse<PageResponse<UserLedgerDto>>>, AdminError> {
    let offset = pagination.offset();
    let limit = pagination.limit();

    let total: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM badge_ledger WHERE user_id = $1")
        .bind(&user_id)
        .fetch_one(&state.pool)
        .await?;

    if total.0 == 0 {
        return Ok(Json(ApiResponse::success(PageResponse::empty(
            pagination.page,
            pagination.page_size,
        ))));
    }

    let rows = sqlx::query_as::<_, LedgerRow>(
        r#"
        SELECT
            bl.id,
            bl.badge_id,
            b.name as badge_name,
            bl.change_type::text as change_type,
            bl.source_type::text as source_type,
            bl.quantity,
            bl.remark,
            bl.created_at
        FROM badge_ledger bl
        JOIN badges b ON b.id = bl.badge_id
        WHERE bl.user_id = $1
        ORDER BY bl.created_at DESC
        LIMIT $2 OFFSET $3
        "#,
    )
    .bind(&user_id)
    .bind(limit)
    .bind(offset)
    .fetch_all(&state.pool)
    .await?;

    let items: Vec<UserLedgerDto> = rows
        .into_iter()
        .map(|row| UserLedgerDto {
            id: row.id,
            badge_id: row.badge_id,
            badge_name: row.badge_name,
            change_type: row.change_type,
            source_type: row.source_type,
            quantity: row.quantity,
            remark: row.remark,
            created_at: row.created_at,
        })
        .collect();

    let response = PageResponse::new(items, total.0, pagination.page, pagination.page_size);
    Ok(Json(ApiResponse::success(response)))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dto::{UserBadgeAdminDto, UserStatsDto};

    #[test]
    fn test_user_badge_admin_dto_serialization() {
        let dto = UserBadgeAdminDto {
            badge_id: 1,
            badge_name: "勋章".to_string(),
            badge_type: "normal".to_string(),
            quantity: 3,
            status: "active".to_string(),
            acquired_at: Utc::now(),
            expires_at: None,
        };

        let json = serde_json::to_string(&dto).unwrap();
        assert!(json.contains("\"badgeId\":1"));
        assert!(json.contains("\"quantity\":3"));
        assert!(json.contains("\"expiresAt\":null"));
    }

    #[test]
    fn test_user_stats_dto_serialization() {
        let dto = UserStatsDto {
            user_id: "user001".to_string(),
            total_badges: 10,
            active_badges: 8,
            expired_badges: 2,
            total_redeemed: 5,
        };

        let json = serde_json::to_string(&dto).unwrap();
        assert!(json.contains("\"userId\":\"user001\""));
        assert!(json.contains("\"activeBadges\":8"));
        assert!(json.contains("\"totalRedeemed\":5"));
    }

    #[test]
    fn test_user_redemption_dto_serialization() {
        let dto = UserRedemptionDto {
            order_id: 100,
            order_no: "ORD2025010001".to_string(),
            benefit_name: "优惠券".to_string(),
            status: "completed".to_string(),
            created_at: Utc::now(),
        };

        let json = serde_json::to_string(&dto).unwrap();
        assert!(json.contains("\"orderId\":100"));
        assert!(json.contains("\"orderNo\":\"ORD2025010001\""));
    }

    #[test]
    fn test_user_ledger_dto_serialization() {
        let dto = UserLedgerDto {
            id: 1,
            badge_id: 5,
            badge_name: "新手徽章".to_string(),
            change_type: "acquire".to_string(),
            source_type: "manual".to_string(),
            quantity: 1,
            remark: Some("手动发放".to_string()),
            created_at: Utc::now(),
        };

        let json = serde_json::to_string(&dto).unwrap();
        assert!(json.contains("\"changeType\":\"acquire\""));
        assert!(json.contains("\"sourceType\":\"manual\""));
    }
}
