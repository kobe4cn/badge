//! 发放管理 API 处理器
//!
//! 实现徽章的手动发放、批量发放和发放记录查询。
//! 发放操作涉及多表事务：user_badges + badge_ledger + user_badge_logs。

use axum::{
    extract::{Query, State},
    Json,
};
use chrono::{DateTime, Utc};
use tracing::info;
use uuid::Uuid;
use validator::Validate;

use crate::{
    dto::{
        ApiResponse, BatchTaskDto, GrantLogDto, GrantLogFilter, ManualGrantRequest,
        BatchGrantRequest, PageResponse, PaginationParams,
    },
    error::AdminError,
    state::AppState,
    SourceType,
};

/// 发放记录数据库查询结果
#[derive(sqlx::FromRow)]
struct GrantLogRow {
    id: i64,
    user_id: String,
    badge_id: i64,
    badge_name: String,
    quantity: i32,
    source_type: SourceType,
    source_id: Option<String>,
    reason: Option<String>,
    created_at: DateTime<Utc>,
}

impl From<GrantLogRow> for GrantLogDto {
    fn from(row: GrantLogRow) -> Self {
        Self {
            id: row.id,
            user_id: row.user_id,
            badge_id: row.badge_id,
            badge_name: row.badge_name,
            quantity: row.quantity,
            source_type: row.source_type,
            source_id: row.source_id,
            reason: row.reason,
            // 手动发放暂不追踪操作人信息，后续可通过 JWT 中间件注入
            operator_id: None,
            operator_name: None,
            created_at: row.created_at,
        }
    }
}

/// 手动发放单个徽章
///
/// POST /api/admin/grants/manual
///
/// 在事务中完成三步操作：
/// 1. 插入/更新 user_badges 数量
/// 2. 写入 badge_ledger 记录（ACQUIRE + MANUAL）
/// 3. 写入 user_badge_logs 日志（GRANT）
pub async fn manual_grant(
    State(state): State<AppState>,
    Json(req): Json<ManualGrantRequest>,
) -> Result<Json<ApiResponse<serde_json::Value>>, AdminError> {
    req.validate()?;

    // 检查徽章存在且状态为可发放
    let badge: Option<(i64, String, Option<i64>, i64)> = sqlx::query_as(
        "SELECT id, name, max_supply, issued_count FROM badges WHERE id = $1",
    )
    .bind(req.badge_id)
    .fetch_optional(&state.pool)
    .await?;

    let badge = badge.ok_or(AdminError::BadgeNotFound(req.badge_id))?;

    // 库存检查：有限量时确认剩余足够
    if let Some(max_supply) = badge.2
        && badge.3 + req.quantity as i64 > max_supply
    {
        return Err(AdminError::InsufficientStock);
    }

    let source_ref_id = Uuid::new_v4().to_string();
    let now = Utc::now();

    // 事务保证三表一致性
    let mut tx = state.pool.begin().await?;

    // 1. 插入或更新 user_badges
    sqlx::query(
        r#"
        INSERT INTO user_badges (user_id, badge_id, quantity, status, first_acquired_at, last_acquired_at)
        VALUES ($1, $2, $3, 'active', $4, $4)
        ON CONFLICT (user_id, badge_id)
        DO UPDATE SET
            quantity = user_badges.quantity + $3,
            status = 'active',
            last_acquired_at = $4,
            updated_at = $4
        "#,
    )
    .bind(&req.user_id)
    .bind(req.badge_id)
    .bind(req.quantity)
    .bind(now)
    .execute(&mut *tx)
    .await?;

    // 2. 写入 badge_ledger
    sqlx::query(
        r#"
        INSERT INTO badge_ledger (user_id, badge_id, change_type, source_type, source_ref_id, quantity, remark, created_at)
        VALUES ($1, $2, 'acquire', 'manual', $3, $4, $5, $6)
        "#,
    )
    .bind(&req.user_id)
    .bind(req.badge_id)
    .bind(&source_ref_id)
    .bind(req.quantity)
    .bind(&req.reason)
    .bind(now)
    .execute(&mut *tx)
    .await?;

    // 3. 写入 user_badge_logs
    sqlx::query(
        r#"
        INSERT INTO user_badge_logs (user_id, badge_id, action, quantity, source_type, source_ref_id, remark, created_at)
        VALUES ($1, $2, 'grant', $3, 'manual', $4, $5, $6)
        "#,
    )
    .bind(&req.user_id)
    .bind(req.badge_id)
    .bind(req.quantity)
    .bind(&source_ref_id)
    .bind(&req.reason)
    .bind(now)
    .execute(&mut *tx)
    .await?;

    // 4. 累加徽章已发放计数
    sqlx::query("UPDATE badges SET issued_count = issued_count + $2, updated_at = $3 WHERE id = $1")
        .bind(req.badge_id)
        .bind(req.quantity as i64)
        .bind(now)
        .execute(&mut *tx)
        .await?;

    tx.commit().await?;

    info!(
        user_id = %req.user_id,
        badge_id = req.badge_id,
        quantity = req.quantity,
        "Manual grant completed"
    );

    let result = serde_json::json!({
        "userId": req.user_id,
        "badgeId": req.badge_id,
        "badgeName": badge.1,
        "quantity": req.quantity,
        "sourceRefId": source_ref_id,
    });

    Ok(Json(ApiResponse::success(result)))
}

/// 批量发放徽章（异步任务）
///
/// POST /api/admin/grants/batch
///
/// 创建批量任务记录后立即返回任务 ID，
/// 实际处理由后台异步执行（简化实现）。
pub async fn batch_grant(
    State(state): State<AppState>,
    Json(req): Json<BatchGrantRequest>,
) -> Result<Json<ApiResponse<BatchTaskDto>>, AdminError> {
    req.validate()?;

    // 验证徽章存在
    let badge_exists: (bool,) =
        sqlx::query_as("SELECT EXISTS(SELECT 1 FROM badges WHERE id = $1)")
            .bind(req.badge_id)
            .fetch_one(&state.pool)
            .await?;

    if !badge_exists.0 {
        return Err(AdminError::BadgeNotFound(req.badge_id));
    }

    let now = Utc::now();

    // 创建批量任务记录
    let row: (i64,) = sqlx::query_as(
        r#"
        INSERT INTO batch_tasks (task_type, file_url, status, progress, total_count, success_count, failure_count, created_by, created_at, updated_at)
        VALUES ('batch_grant', $1, 'pending', 0, 0, 0, 0, 'admin', $2, $2)
        RETURNING id
        "#,
    )
    .bind(&req.file_url)
    .bind(now)
    .fetch_one(&state.pool)
    .await?;

    info!(task_id = row.0, badge_id = req.badge_id, "Batch grant task created");

    // TODO: 发送消息到任务队列触发异步处理
    let task_dto = BatchTaskDto {
        id: row.0,
        task_type: "batch_grant".to_string(),
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

/// 发放记录查询（分页）
///
/// GET /api/admin/grants
///
/// 查询 badge_ledger 中 change_type = 'acquire' 的记录
pub async fn list_grants(
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

    // 查询总数
    let total: (i64,) = sqlx::query_as(
        r#"
        SELECT COUNT(*)
        FROM badge_ledger l
        WHERE l.change_type = 'acquire'
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

    let rows = sqlx::query_as::<_, GrantLogRow>(
        r#"
        SELECT
            l.id,
            l.user_id,
            l.badge_id,
            b.name as badge_name,
            l.quantity,
            l.source_type,
            l.source_ref_id as source_id,
            l.remark as reason,
            l.created_at
        FROM badge_ledger l
        JOIN badges b ON b.id = l.badge_id
        WHERE l.change_type = 'acquire'
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

#[cfg(test)]
mod tests {
    use crate::dto::ManualGrantRequest;
    use validator::Validate;

    #[test]
    fn test_manual_grant_request_validation() {
        let valid = ManualGrantRequest {
            user_id: "user001".to_string(),
            badge_id: 1,
            quantity: 1,
            reason: "活动奖励".to_string(),
        };
        assert!(valid.validate().is_ok());

        // 数量超限
        let invalid = ManualGrantRequest {
            user_id: "user001".to_string(),
            badge_id: 1,
            quantity: 101,
            reason: "活动奖励".to_string(),
        };
        assert!(invalid.validate().is_err());

        // 空原因
        let invalid_reason = ManualGrantRequest {
            user_id: "user001".to_string(),
            badge_id: 1,
            quantity: 1,
            reason: "".to_string(),
        };
        assert!(invalid_reason.validate().is_err());
    }

    #[test]
    fn test_batch_grant_request_validation() {
        use crate::dto::BatchGrantRequest;

        let valid = BatchGrantRequest {
            badge_id: 1,
            file_url: "https://oss.example.com/users.csv".to_string(),
            reason: "批量活动奖励".to_string(),
        };
        assert!(valid.validate().is_ok());

        // 无效 URL
        let invalid_url = BatchGrantRequest {
            badge_id: 1,
            file_url: "not-a-url".to_string(),
            reason: "批量活动奖励".to_string(),
        };
        assert!(invalid_url.validate().is_err());
    }
}
