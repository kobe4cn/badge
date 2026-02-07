//! 取消管理 API 处理器
//!
//! 实现徽章的手动取消、批量取消和取消记录查询。
//! 取消操作涉及多表事务：user_badges 扣减 + badge_ledger + user_badge_logs。

use axum::{
    Json,
    extract::{Query, State},
};
use chrono::{DateTime, Utc};
use tracing::info;
use uuid::Uuid;
use validator::Validate;

use crate::{
    dto::{
        ApiResponse, AutoRevokeRequest, AutoRevokeScenario, BatchRevokeRequest, BatchTaskDto,
        GrantLogDto, GrantLogFilter, ManualRevokeRequest, PageResponse, PaginationParams,
    },
    error::AdminError,
    state::AppState,
};

/// 取消记录数据库查询结果
#[derive(sqlx::FromRow)]
struct RevokeLogRow {
    id: i64,
    user_id: String,
    badge_id: i64,
    badge_name: String,
    quantity: i32,
    source_type: String,
    source_id: Option<String>,
    reason: Option<String>,
    created_at: DateTime<Utc>,
}

impl From<RevokeLogRow> for GrantLogDto {
    fn from(row: RevokeLogRow) -> Self {
        Self {
            id: row.id,
            user_id: row.user_id,
            badge_id: row.badge_id,
            badge_name: row.badge_name,
            quantity: row.quantity,
            source_type: row.source_type,
            source_id: row.source_id,
            reason: row.reason,
            operator_id: None,
            operator_name: None,
            created_at: row.created_at,
        }
    }
}

/// 手动取消单个用户的徽章
///
/// POST /api/admin/revokes/manual
///
/// 前端通过 user_badge_id 指定要撤销的记录，后端反查 user_id 和 badge_id 执行撤销。
/// 在事务中完成：
/// 1. 通过 user_badge_id 查出 user_id、badge_id 和当前数量
/// 2. 扣减 user_badges 数量（归零时标记为 Revoked）
/// 3. 写入 badge_ledger（CANCEL + MANUAL）
/// 4. 写入 user_badge_logs（REVOKE）
pub async fn manual_revoke(
    State(state): State<AppState>,
    Json(req): Json<ManualRevokeRequest>,
) -> Result<Json<ApiResponse<serde_json::Value>>, AdminError> {
    req.validate()?;

    // 通过 user_badge_id 反查用户和徽章信息
    let ub_row: Option<(String, i64, i32)> = sqlx::query_as(
        "SELECT user_id, badge_id, quantity FROM user_badges WHERE id = $1",
    )
    .bind(req.user_badge_id)
    .fetch_optional(&state.pool)
    .await?;

    let (user_id, badge_id, current_qty) = ub_row.ok_or_else(|| {
        AdminError::NotFound(format!("用户徽章记录不存在: {}", req.user_badge_id))
    })?;

    // 查询徽章名称（用于响应和日志）
    let badge: Option<(i64, String)> = sqlx::query_as("SELECT id, name FROM badges WHERE id = $1")
        .bind(badge_id)
        .fetch_optional(&state.pool)
        .await?;

    let badge = badge.ok_or(AdminError::BadgeNotFound(badge_id))?;

    // 每次撤销一个
    let quantity = 1;

    if current_qty < quantity {
        return Err(AdminError::InsufficientUserBadge);
    }

    let source_ref_id = Uuid::new_v4().to_string();
    let now = Utc::now();
    let remaining = current_qty - quantity;

    let mut tx = state.pool.begin().await?;

    // 1. 扣减 user_badges，归零时标记为 revoked
    if remaining == 0 {
        sqlx::query(
            r#"
            UPDATE user_badges
            SET quantity = 0, status = 'REVOKED', updated_at = $2
            WHERE id = $1
            "#,
        )
        .bind(req.user_badge_id)
        .bind(now)
        .execute(&mut *tx)
        .await?;
    } else {
        sqlx::query(
            r#"
            UPDATE user_badges
            SET quantity = quantity - $2, updated_at = $3
            WHERE id = $1
            "#,
        )
        .bind(req.user_badge_id)
        .bind(quantity)
        .bind(now)
        .execute(&mut *tx)
        .await?;
    }

    // 2. 写入 badge_ledger（quantity 为负数表示扣减）
    sqlx::query(
        r#"
        INSERT INTO badge_ledger (user_id, badge_id, change_type, source_type, ref_id, quantity, balance_after, remark, created_at)
        VALUES ($1, $2, 'cancel', 'MANUAL', $3, $4, $5, $6, $7)
        "#,
    )
    .bind(&user_id)
    .bind(badge_id)
    .bind(&source_ref_id)
    .bind(-quantity) // 负数表示扣减
    .bind(remaining) // 扣减后的余额
    .bind(&req.reason)
    .bind(now)
    .execute(&mut *tx)
    .await?;

    // 3. 写入 user_badge_logs
    sqlx::query(
        r#"
        INSERT INTO user_badge_logs (user_id, badge_id, action, quantity, source_type, source_ref_id, remark, created_at)
        VALUES ($1, $2, 'revoke', $3, 'MANUAL', $4, $5, $6)
        "#,
    )
    .bind(&user_id)
    .bind(badge_id)
    .bind(quantity)
    .bind(&source_ref_id)
    .bind(&req.reason)
    .bind(now)
    .execute(&mut *tx)
    .await?;

    // 4. 扣减徽章已发放计数
    sqlx::query(
        "UPDATE badges SET issued_count = issued_count - $2, updated_at = $3 WHERE id = $1",
    )
    .bind(badge_id)
    .bind(quantity as i64)
    .bind(now)
    .execute(&mut *tx)
    .await?;

    tx.commit().await?;

    info!(
        user_badge_id = req.user_badge_id,
        user_id = %user_id,
        badge_id = badge_id,
        quantity = quantity,
        remaining = remaining,
        "Manual revoke completed"
    );

    let result = serde_json::json!({
        "userId": user_id,
        "badgeId": badge_id,
        "badgeName": badge.1,
        "quantity": quantity,
        "remaining": remaining,
        "sourceRefId": source_ref_id,
    });

    Ok(Json(ApiResponse::success(result)))
}

/// 批量取消徽章（异步任务）
///
/// POST /api/admin/revokes/batch
///
/// 与批量发放类似，创建异步任务后立即返回任务 ID。
pub async fn batch_revoke(
    State(state): State<AppState>,
    Json(req): Json<BatchRevokeRequest>,
) -> Result<Json<ApiResponse<BatchTaskDto>>, AdminError> {
    req.validate()?;

    // 验证徽章存在
    let badge_exists: (bool,) = sqlx::query_as("SELECT EXISTS(SELECT 1 FROM badges WHERE id = $1)")
        .bind(req.badge_id)
        .fetch_one(&state.pool)
        .await?;

    if !badge_exists.0 {
        return Err(AdminError::BadgeNotFound(req.badge_id));
    }

    let now = Utc::now();

    // 将业务参数序列化到 params 列，Worker 轮询到任务后从中解析 badge_id 和 reason
    let params = serde_json::json!({
        "badge_id": req.badge_id,
        "reason": &req.reason,
    });

    let row: (i64,) = sqlx::query_as(
        r#"
        INSERT INTO batch_tasks (task_type, file_url, params, status, progress, total_count, success_count, failure_count, created_by, created_at, updated_at)
        VALUES ('batch_revoke', $1, $2, 'pending', 0, 0, 0, 0, 'admin', $3, $3)
        RETURNING id
        "#,
    )
    .bind(&req.file_url)
    .bind(&params)
    .bind(now)
    .fetch_one(&state.pool)
    .await?;

    info!(
        task_id = row.0,
        badge_id = req.badge_id,
        "Batch revoke task created"
    );
    let task_dto = BatchTaskDto {
        id: row.0,
        task_type: "batch_revoke".to_string(),
        status: "pending".to_string(),
        total_count: 0,
        success_count: 0,
        failure_count: 0,
        progress: 0,
        file_url: Some(req.file_url),
        result_file_url: None,
        error_message: None,
        created_by: "admin".to_string(),
        created_at: now,
        updated_at: now,
    };

    Ok(Json(ApiResponse::success(task_dto)))
}

/// 取消记录查询（分页）
///
/// GET /api/admin/revokes
///
/// 查询 badge_ledger 中 change_type = 'cancel' 的记录
pub async fn list_revokes(
    State(state): State<AppState>,
    Query(pagination): Query<PaginationParams>,
    Query(filter): Query<GrantLogFilter>,
) -> Result<Json<ApiResponse<PageResponse<GrantLogDto>>>, AdminError> {
    let offset = pagination.offset();
    let limit = pagination.limit();

    let source_type_str = filter.source_type.map(|t| {
        serde_json::to_value(t)
            .ok()
            .and_then(|v| v.as_str().map(|s| s.to_lowercase()))
            .unwrap_or_default()
    });

    let total: (i64,) = sqlx::query_as(
        r#"
        SELECT COUNT(*)
        FROM badge_ledger l
        WHERE l.change_type = 'cancel'
          AND ($1::text IS NULL OR l.user_id = $1)
          AND ($2::bigint IS NULL OR l.badge_id = $2)
          AND ($3::text IS NULL OR l.source_type::text = $3)
          AND ($4::timestamptz IS NULL OR l.created_at >= $4)
          AND ($5::timestamptz IS NULL OR l.created_at <= $5)
        "#,
    )
    .bind(&filter.user_id)
    .bind(filter.badge_id)
    .bind(&source_type_str)
    .bind(filter.start_time)
    .bind(filter.end_time)
    .fetch_one(&state.pool)
    .await?;

    if total.0 == 0 {
        return Ok(Json(ApiResponse::success(PageResponse::empty(
            pagination.page,
            pagination.page_size,
        ))));
    }

    let rows = sqlx::query_as::<_, RevokeLogRow>(
        r#"
        SELECT
            l.id,
            l.user_id,
            l.badge_id,
            b.name as badge_name,
            l.quantity,
            l.source_type,
            l.ref_id as source_id,
            l.remark as reason,
            l.created_at
        FROM badge_ledger l
        JOIN badges b ON b.id = l.badge_id
        WHERE l.change_type = 'cancel'
          AND ($1::text IS NULL OR l.user_id = $1)
          AND ($2::bigint IS NULL OR l.badge_id = $2)
          AND ($3::text IS NULL OR l.source_type::text = $3)
          AND ($4::timestamptz IS NULL OR l.created_at >= $4)
          AND ($5::timestamptz IS NULL OR l.created_at <= $5)
        ORDER BY l.created_at DESC
        LIMIT $6 OFFSET $7
        "#,
    )
    .bind(&filter.user_id)
    .bind(filter.badge_id)
    .bind(&source_type_str)
    .bind(filter.start_time)
    .bind(filter.end_time)
    .bind(limit)
    .bind(offset)
    .fetch_all(&state.pool)
    .await?;

    let items: Vec<GrantLogDto> = rows.into_iter().map(Into::into).collect();
    let response = PageResponse::new(items, total.0, pagination.page, pagination.page_size);
    Ok(Json(ApiResponse::success(response)))
}

/// 自动取消结果
#[derive(Debug, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AutoRevokeResult {
    /// 用户 ID
    pub user_id: String,
    /// 撤销的徽章数量
    pub revoked_count: i32,
    /// 撤销的徽章详情
    pub revoked_badges: Vec<RevokedBadgeInfo>,
    /// 取消场景
    pub scenario: String,
}

/// 被撤销的徽章信息
#[derive(Debug, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RevokedBadgeInfo {
    pub badge_id: i64,
    pub badge_name: String,
    pub quantity: i32,
}

/// 自动取消徽章
///
/// POST /api/admin/revokes/auto
///
/// 用于账号注销、身份变更、条件不满足等自动触发的撤销场景。
/// 支持撤销指定用户的单个徽章或所有徽章。
pub async fn auto_revoke(
    State(state): State<AppState>,
    Json(req): Json<AutoRevokeRequest>,
) -> Result<Json<ApiResponse<AutoRevokeResult>>, AdminError> {
    use validator::Validate;
    req.validate()?;

    let now = Utc::now();
    let scenario_str = req.scenario.to_string();
    // 使用缩短的 source_type 以适应 VARCHAR(20) 字段限制
    let source_type = match req.scenario {
        AutoRevokeScenario::AccountDeletion => "AUTO_ACCT_DEL",
        AutoRevokeScenario::IdentityChange => "AUTO_ID_CHG",
        AutoRevokeScenario::ConditionUnmet => "AUTO_COND_UNMET",
        AutoRevokeScenario::Violation => "AUTO_VIOLATION",
        AutoRevokeScenario::SystemTriggered => "AUTO_SYSTEM",
    }
    .to_string();

    // 查询需要撤销的徽章
    let badges_to_revoke: Vec<(i64, i64, String, i32)> = if let Some(badge_id) = req.badge_id {
        // 撤销指定徽章
        sqlx::query_as(
            r#"
            SELECT ub.id, ub.badge_id, b.name, ub.quantity
            FROM user_badges ub
            JOIN badges b ON b.id = ub.badge_id
            WHERE ub.user_id = $1 AND ub.badge_id = $2 AND UPPER(ub.status::text) = 'ACTIVE' AND ub.quantity > 0
            "#,
        )
        .bind(&req.user_id)
        .bind(badge_id)
        .fetch_all(&state.pool)
        .await?
    } else {
        // 撤销所有徽章
        sqlx::query_as(
            r#"
            SELECT ub.id, ub.badge_id, b.name, ub.quantity
            FROM user_badges ub
            JOIN badges b ON b.id = ub.badge_id
            WHERE ub.user_id = $1 AND UPPER(ub.status::text) = 'ACTIVE' AND ub.quantity > 0
            "#,
        )
        .bind(&req.user_id)
        .fetch_all(&state.pool)
        .await?
    };

    if badges_to_revoke.is_empty() {
        return Ok(Json(ApiResponse::success(AutoRevokeResult {
            user_id: req.user_id,
            revoked_count: 0,
            revoked_badges: vec![],
            scenario: scenario_str,
        })));
    }

    let mut tx = state.pool.begin().await?;
    let mut revoked_badges = Vec::new();
    let mut total_revoked = 0;

    for (user_badge_id, badge_id, badge_name, quantity) in badges_to_revoke {
        let source_ref_id = req
            .ref_id
            .clone()
            .unwrap_or_else(|| Uuid::new_v4().to_string());

        // 1. 将 user_badges 状态改为 revoked
        sqlx::query(
            r#"
            UPDATE user_badges
            SET quantity = 0, status = 'REVOKED', updated_at = $2
            WHERE id = $1
            "#,
        )
        .bind(user_badge_id)
        .bind(now)
        .execute(&mut *tx)
        .await?;

        // 2. 写入 badge_ledger
        sqlx::query(
            r#"
            INSERT INTO badge_ledger (user_id, badge_id, change_type, source_type, ref_id, quantity, balance_after, remark, created_at)
            VALUES ($1, $2, 'cancel', $3, $4, $5, 0, $6, $7)
            "#,
        )
        .bind(&req.user_id)
        .bind(badge_id)
        .bind(&source_type)
        .bind(&source_ref_id)
        .bind(-quantity)
        .bind(&req.reason)
        .bind(now)
        .execute(&mut *tx)
        .await?;

        // 3. 写入 user_badge_logs
        sqlx::query(
            r#"
            INSERT INTO user_badge_logs (user_id, badge_id, action, quantity, source_type, source_ref_id, remark, created_at)
            VALUES ($1, $2, 'revoke', $3, $4, $5, $6, $7)
            "#,
        )
        .bind(&req.user_id)
        .bind(badge_id)
        .bind(quantity)
        .bind(&source_type)
        .bind(&source_ref_id)
        .bind(&req.reason)
        .bind(now)
        .execute(&mut *tx)
        .await?;

        // 4. 扣减徽章已发放计数
        sqlx::query(
            "UPDATE badges SET issued_count = issued_count - $2, updated_at = $3 WHERE id = $1",
        )
        .bind(badge_id)
        .bind(quantity as i64)
        .bind(now)
        .execute(&mut *tx)
        .await?;

        revoked_badges.push(RevokedBadgeInfo {
            badge_id,
            badge_name,
            quantity,
        });
        total_revoked += quantity;

        info!(
            user_id = %req.user_id,
            badge_id = badge_id,
            quantity = quantity,
            scenario = %scenario_str,
            "Auto revoke completed for badge"
        );
    }

    tx.commit().await?;

    info!(
        user_id = %req.user_id,
        scenario = %scenario_str,
        total_revoked = total_revoked,
        badge_count = revoked_badges.len(),
        "Auto revoke completed"
    );

    Ok(Json(ApiResponse::success(AutoRevokeResult {
        user_id: req.user_id,
        revoked_count: total_revoked,
        revoked_badges,
        scenario: scenario_str,
    })))
}

#[cfg(test)]
mod tests {
    use crate::dto::ManualRevokeRequest;
    use validator::Validate;

    #[test]
    fn test_manual_revoke_request_validation() {
        let valid = ManualRevokeRequest {
            user_badge_id: 1,
            reason: "违规处理".to_string(),
        };
        assert!(valid.validate().is_ok());

        // 原因为空应失败
        let invalid = ManualRevokeRequest {
            user_badge_id: 1,
            reason: "".to_string(),
        };
        assert!(invalid.validate().is_err());
    }

    #[test]
    fn test_batch_revoke_request_validation() {
        use crate::dto::BatchRevokeRequest;

        let valid = BatchRevokeRequest {
            badge_id: 1,
            file_url: "https://oss.example.com/users.csv".to_string(),
            reason: "批量清退".to_string(),
        };
        assert!(valid.validate().is_ok());

        // 无效URL
        let invalid = BatchRevokeRequest {
            badge_id: 1,
            file_url: "not-a-url".to_string(),
            reason: "批量清退".to_string(),
        };
        assert!(invalid.validate().is_err());
    }
}
