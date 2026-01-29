//! 操作日志 API 处理器
//!
//! 提供操作日志查询，支持按模块、操作、操作人、时间范围等维度过滤。
//! 所有管理后台的变更操作（CRUD/发放/取消等）都会记录到操作日志中。

use axum::{
    Json,
    extract::{Query, State},
};
use chrono::{DateTime, Utc};
use tracing::instrument;

use crate::{
    dto::{ApiResponse, OperationLogDto, OperationLogFilter, PageResponse, PaginationParams},
    error::AdminError,
    state::AppState,
};

/// 操作日志行
#[derive(sqlx::FromRow)]
struct OperationLogRow {
    id: i64,
    operator_id: String,
    operator_name: Option<String>,
    module: String,
    action: String,
    target_type: Option<String>,
    target_id: Option<String>,
    before_data: Option<serde_json::Value>,
    after_data: Option<serde_json::Value>,
    ip_address: Option<String>,
    created_at: DateTime<Utc>,
}

impl From<OperationLogRow> for OperationLogDto {
    fn from(row: OperationLogRow) -> Self {
        Self {
            id: row.id,
            operator_id: row.operator_id,
            operator_name: row.operator_name,
            module: row.module,
            action: row.action,
            target_type: row.target_type,
            target_id: row.target_id,
            before_data: row.before_data,
            after_data: row.after_data,
            ip_address: row.ip_address,
            created_at: row.created_at,
        }
    }
}

/// 查询操作日志（分页 + 过滤）
///
/// GET /api/admin/logs
///
/// 支持的过滤条件：
/// - operator_id: 操作人 ID
/// - module: 操作模块（badge/rule/grant/revoke 等）
/// - action: 操作动作（create/update/delete 等）
/// - target_type / target_id: 操作目标
/// - start_time / end_time: 时间范围
#[instrument(skip(state))]
pub async fn list_logs(
    State(state): State<AppState>,
    Query(pagination): Query<PaginationParams>,
    Query(filter): Query<OperationLogFilter>,
) -> Result<Json<ApiResponse<PageResponse<OperationLogDto>>>, AdminError> {
    let offset = pagination.offset();
    let limit = pagination.limit();

    let total: (i64,) = sqlx::query_as(
        r#"
        SELECT COUNT(*)
        FROM operation_logs
        WHERE ($1::text IS NULL OR operator_id = $1)
          AND ($2::text IS NULL OR module = $2)
          AND ($3::text IS NULL OR action = $3)
          AND ($4::text IS NULL OR target_type = $4)
          AND ($5::text IS NULL OR target_id = $5)
          AND ($6::timestamptz IS NULL OR created_at >= $6)
          AND ($7::timestamptz IS NULL OR created_at <= $7)
        "#,
    )
    .bind(&filter.operator_id)
    .bind(&filter.module)
    .bind(&filter.action)
    .bind(&filter.target_type)
    .bind(&filter.target_id)
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

    let rows = sqlx::query_as::<_, OperationLogRow>(
        r#"
        SELECT
            id,
            operator_id,
            operator_name,
            module,
            action,
            target_type,
            target_id,
            before_data,
            after_data,
            ip_address,
            created_at
        FROM operation_logs
        WHERE ($1::text IS NULL OR operator_id = $1)
          AND ($2::text IS NULL OR module = $2)
          AND ($3::text IS NULL OR action = $3)
          AND ($4::text IS NULL OR target_type = $4)
          AND ($5::text IS NULL OR target_id = $5)
          AND ($6::timestamptz IS NULL OR created_at >= $6)
          AND ($7::timestamptz IS NULL OR created_at <= $7)
        ORDER BY created_at DESC
        LIMIT $8 OFFSET $9
        "#,
    )
    .bind(&filter.operator_id)
    .bind(&filter.module)
    .bind(&filter.action)
    .bind(&filter.target_type)
    .bind(&filter.target_id)
    .bind(filter.start_time)
    .bind(filter.end_time)
    .bind(limit)
    .bind(offset)
    .fetch_all(&state.pool)
    .await?;

    let items: Vec<OperationLogDto> = rows.into_iter().map(Into::into).collect();
    let response = PageResponse::new(items, total.0, pagination.page, pagination.page_size);
    Ok(Json(ApiResponse::success(response)))
}

#[cfg(test)]
mod tests {
    use crate::dto::{OperationLogDto, OperationLogFilter};

    #[test]
    fn test_operation_log_dto_serialization() {
        let dto = OperationLogDto {
            id: 1,
            operator_id: "admin001".to_string(),
            operator_name: Some("管理员".to_string()),
            module: "badge".to_string(),
            action: "create".to_string(),
            target_type: Some("badge".to_string()),
            target_id: Some("42".to_string()),
            before_data: None,
            after_data: Some(serde_json::json!({"name": "新徽章"})),
            ip_address: Some("192.168.1.1".to_string()),
            created_at: chrono::Utc::now(),
        };

        let json = serde_json::to_string(&dto).unwrap();
        assert!(json.contains("\"operatorId\":\"admin001\""));
        assert!(json.contains("\"module\":\"badge\""));
        assert!(json.contains("\"targetId\":\"42\""));
    }

    #[test]
    fn test_operation_log_filter_default() {
        let filter = OperationLogFilter::default();
        assert!(filter.operator_id.is_none());
        assert!(filter.module.is_none());
        assert!(filter.action.is_none());
        assert!(filter.target_type.is_none());
        assert!(filter.target_id.is_none());
        assert!(filter.start_time.is_none());
        assert!(filter.end_time.is_none());
    }
}
