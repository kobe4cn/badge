//! 批量任务 API 处理器
//!
//! 提供批量任务的创建、列表查询和详情/进度查询。
//! 批量任务用于处理批量发放、批量取消、数据导出等耗时操作，
//! 前端通过轮询 get_task 接口获取实时进度。

use axum::{
    Json,
    extract::{Path, Query, State},
};
use chrono::{DateTime, Utc};
use tracing::{info, instrument};
use validator::Validate;

use crate::{
    dto::{ApiResponse, BatchTaskDto, BatchTaskFilter, PageResponse, PaginationParams},
    error::AdminError,
    models::BatchTaskType,
    state::AppState,
};

/// 创建批量任务请求
#[derive(Debug, serde::Deserialize, Validate)]
#[serde(rename_all = "camelCase")]
pub struct CreateBatchTaskRequest {
    /// 任务类型：batch_grant / batch_revoke / data_export
    pub task_type: String,
    /// 输入文件地址（批量发放/取消场景需要）
    pub file_url: Option<String>,
    /// 额外参数，不同任务类型含义不同
    pub params: Option<serde_json::Value>,
}

/// 批量任务行（数据库查询结果）
#[derive(sqlx::FromRow)]
struct BatchTaskRow {
    id: i64,
    task_type: String,
    status: String,
    total_count: i32,
    success_count: i32,
    failure_count: i32,
    progress: i32,
    file_url: Option<String>,
    result_file_url: Option<String>,
    error_message: Option<String>,
    created_by: String,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

impl From<BatchTaskRow> for BatchTaskDto {
    fn from(row: BatchTaskRow) -> Self {
        Self {
            id: row.id,
            task_type: row.task_type,
            status: row.status,
            total_count: row.total_count,
            success_count: row.success_count,
            failure_count: row.failure_count,
            progress: row.progress,
            file_url: row.file_url,
            result_file_url: row.result_file_url,
            error_message: row.error_message,
            created_by: row.created_by,
            created_at: row.created_at,
            updated_at: row.updated_at,
        }
    }
}

/// 创建批量任务
///
/// POST /api/admin/tasks
///
/// 校验 task_type 合法性后插入 batch_tasks 表，
/// 初始状态为 pending，progress = 0。
/// 实际处理由后台异步执行，前端通过 get_task 轮询进度。
#[instrument(skip(state))]
pub async fn create_task(
    State(state): State<AppState>,
    Json(req): Json<CreateBatchTaskRequest>,
) -> Result<Json<ApiResponse<BatchTaskDto>>, AdminError> {
    req.validate()?;

    // task_type 必须是已知类型，防止写入无效数据
    let task_type = BatchTaskType::parse(&req.task_type).ok_or_else(|| {
        AdminError::Validation(format!(
            "不支持的任务类型: {}，支持: batch_grant, batch_revoke, data_export",
            req.task_type
        ))
    })?;

    let now = Utc::now();

    let row = sqlx::query_as::<_, BatchTaskRow>(
        r#"
        INSERT INTO batch_tasks (task_type, file_url, status, progress, total_count, success_count, failure_count, params, created_by, created_at, updated_at)
        VALUES ($1, $2, 'pending', 0, 0, 0, 0, $3, 'admin', $4, $4)
        RETURNING id, task_type, status, total_count, success_count, failure_count, progress, file_url, result_file_url, error_message, created_by, created_at, updated_at
        "#,
    )
    .bind(task_type.as_str())
    .bind(&req.file_url)
    .bind(&req.params)
    .bind(now)
    .fetch_one(&state.pool)
    .await?;

    info!(
        task_id = row.id,
        task_type = task_type.as_str(),
        "Batch task created"
    );

    Ok(Json(ApiResponse::success(row.into())))
}

/// 获取批量任务列表（分页 + 过滤）
///
/// GET /api/admin/tasks
///
/// 支持按 task_type、status、created_by 过滤
#[instrument(skip(state))]
pub async fn list_tasks(
    State(state): State<AppState>,
    Query(pagination): Query<PaginationParams>,
    Query(filter): Query<BatchTaskFilter>,
) -> Result<Json<ApiResponse<PageResponse<BatchTaskDto>>>, AdminError> {
    let offset = pagination.offset();
    let limit = pagination.limit();

    let total: (i64,) = sqlx::query_as(
        r#"
        SELECT COUNT(*)
        FROM batch_tasks
        WHERE ($1::text IS NULL OR task_type = $1)
          AND ($2::text IS NULL OR status = $2)
          AND ($3::text IS NULL OR created_by = $3)
        "#,
    )
    .bind(&filter.task_type)
    .bind(&filter.status)
    .bind(&filter.created_by)
    .fetch_one(&state.pool)
    .await?;

    if total.0 == 0 {
        return Ok(Json(ApiResponse::success(PageResponse::empty(
            pagination.page,
            pagination.page_size,
        ))));
    }

    let rows = sqlx::query_as::<_, BatchTaskRow>(
        r#"
        SELECT
            id, task_type, status, total_count, success_count, failure_count,
            progress, file_url, result_file_url, error_message, created_by,
            created_at, updated_at
        FROM batch_tasks
        WHERE ($1::text IS NULL OR task_type = $1)
          AND ($2::text IS NULL OR status = $2)
          AND ($3::text IS NULL OR created_by = $3)
        ORDER BY created_at DESC
        LIMIT $4 OFFSET $5
        "#,
    )
    .bind(&filter.task_type)
    .bind(&filter.status)
    .bind(&filter.created_by)
    .bind(limit)
    .bind(offset)
    .fetch_all(&state.pool)
    .await?;

    let items: Vec<BatchTaskDto> = rows.into_iter().map(Into::into).collect();
    let response = PageResponse::new(items, total.0, pagination.page, pagination.page_size);
    Ok(Json(ApiResponse::success(response)))
}

/// 获取批量任务详情/进度
///
/// GET /api/admin/tasks/:id
///
/// 前端轮询此接口获取任务执行进度和最终结果
#[instrument(skip(state))]
pub async fn get_task(
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> Result<Json<ApiResponse<BatchTaskDto>>, AdminError> {
    let row = sqlx::query_as::<_, BatchTaskRow>(
        r#"
        SELECT
            id, task_type, status, total_count, success_count, failure_count,
            progress, file_url, result_file_url, error_message, created_by,
            created_at, updated_at
        FROM batch_tasks
        WHERE id = $1
        "#,
    )
    .bind(id)
    .fetch_optional(&state.pool)
    .await?;

    let row = row.ok_or(AdminError::TaskNotFound(id))?;
    Ok(Json(ApiResponse::success(row.into())))
}

/// 取消批量任务
///
/// POST /api/admin/tasks/:id/cancel
///
/// 只有 pending 或 running 状态的任务可以取消
#[instrument(skip(state))]
pub async fn cancel_task(
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> Result<Json<ApiResponse<BatchTaskDto>>, AdminError> {
    // 检查任务状态
    let task: Option<(String,)> = sqlx::query_as("SELECT status FROM batch_tasks WHERE id = $1")
        .bind(id)
        .fetch_optional(&state.pool)
        .await?;

    let task = task.ok_or(AdminError::TaskNotFound(id))?;

    if task.0 != "pending" && task.0 != "running" {
        return Err(AdminError::Validation(format!(
            "只有待执行或执行中的任务可以取消，当前状态: {}",
            task.0
        )));
    }

    let now = Utc::now();
    let row = sqlx::query_as::<_, BatchTaskRow>(
        r#"
        UPDATE batch_tasks
        SET status = 'cancelled', error_message = '用户取消', updated_at = $2
        WHERE id = $1
        RETURNING id, task_type, status, total_count, success_count, failure_count,
                  progress, file_url, result_file_url, error_message, created_by,
                  created_at, updated_at
        "#,
    )
    .bind(id)
    .bind(now)
    .fetch_one(&state.pool)
    .await?;

    info!(task_id = id, "Batch task cancelled");
    Ok(Json(ApiResponse::success(row.into())))
}

/// 获取批量任务失败明细
///
/// GET /api/admin/tasks/:id/failures
///
/// 返回任务执行过程中失败的记录列表
#[instrument(skip(state))]
pub async fn get_task_failures(
    State(state): State<AppState>,
    Path(id): Path<i64>,
    Query(pagination): Query<PaginationParams>,
) -> Result<Json<ApiResponse<PageResponse<TaskFailureDto>>>, AdminError> {
    // 验证任务存在
    let exists: (bool,) = sqlx::query_as("SELECT EXISTS(SELECT 1 FROM batch_tasks WHERE id = $1)")
        .bind(id)
        .fetch_optional(&state.pool)
        .await?
        .unwrap_or((false,));

    if !exists.0 {
        return Err(AdminError::TaskNotFound(id));
    }

    let offset = pagination.offset();
    let limit = pagination.limit();

    // 查询总数
    let total: (i64,) = sqlx::query_as(
        "SELECT COUNT(*) FROM batch_task_failures WHERE task_id = $1",
    )
    .bind(id)
    .fetch_one(&state.pool)
    .await?;

    if total.0 == 0 {
        return Ok(Json(ApiResponse::success(PageResponse::empty(
            pagination.page,
            pagination.page_size,
        ))));
    }

    let rows = sqlx::query_as::<_, TaskFailureRow>(
        r#"
        SELECT id, task_id, row_number, user_id, error_code, error_message, created_at
        FROM batch_task_failures
        WHERE task_id = $1
        ORDER BY row_number ASC
        LIMIT $2 OFFSET $3
        "#,
    )
    .bind(id)
    .bind(limit)
    .bind(offset)
    .fetch_all(&state.pool)
    .await?;

    let items: Vec<TaskFailureDto> = rows.into_iter().map(Into::into).collect();
    let response = PageResponse::new(items, total.0, pagination.page, pagination.page_size);
    Ok(Json(ApiResponse::success(response)))
}

/// 下载批量任务结果
///
/// GET /api/admin/tasks/:id/result
///
/// 返回结果文件的下载 URL
#[instrument(skip(state))]
pub async fn get_task_result(
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> Result<Json<ApiResponse<TaskResultDto>>, AdminError> {
    let row: Option<(String, Option<String>)> = sqlx::query_as(
        "SELECT status, result_file_url FROM batch_tasks WHERE id = $1",
    )
    .bind(id)
    .fetch_optional(&state.pool)
    .await?;

    let row = row.ok_or(AdminError::TaskNotFound(id))?;

    if row.0 != "completed" {
        return Err(AdminError::Validation(format!(
            "任务未完成，当前状态: {}",
            row.0
        )));
    }

    let result = TaskResultDto {
        task_id: id,
        result_file_url: row.1,
    };

    Ok(Json(ApiResponse::success(result)))
}

/// 任务失败记录 DTO
#[derive(Debug, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TaskFailureDto {
    pub id: i64,
    pub task_id: i64,
    pub row_number: i32,
    pub user_id: Option<String>,
    pub error_code: String,
    pub error_message: String,
    pub created_at: DateTime<Utc>,
}

#[derive(sqlx::FromRow)]
struct TaskFailureRow {
    id: i64,
    task_id: i64,
    row_number: i32,
    user_id: Option<String>,
    error_code: String,
    error_message: String,
    created_at: DateTime<Utc>,
}

impl From<TaskFailureRow> for TaskFailureDto {
    fn from(row: TaskFailureRow) -> Self {
        Self {
            id: row.id,
            task_id: row.task_id,
            row_number: row.row_number,
            user_id: row.user_id,
            error_code: row.error_code,
            error_message: row.error_message,
            created_at: row.created_at,
        }
    }
}

/// 任务结果 DTO
#[derive(Debug, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TaskResultDto {
    pub task_id: i64,
    pub result_file_url: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dto::BatchTaskDto;

    #[test]
    fn test_create_batch_task_request_valid_types() {
        assert!(BatchTaskType::parse("batch_grant").is_some());
        assert!(BatchTaskType::parse("batch_revoke").is_some());
        assert!(BatchTaskType::parse("data_export").is_some());
        assert!(BatchTaskType::parse("unknown_type").is_none());
    }

    #[test]
    fn test_batch_task_row_to_dto() {
        let now = Utc::now();
        let row = BatchTaskRow {
            id: 1,
            task_type: "batch_grant".to_string(),
            status: "pending".to_string(),
            total_count: 0,
            success_count: 0,
            failure_count: 0,
            progress: 0,
            file_url: Some("https://oss.example.com/users.csv".to_string()),
            result_file_url: None,
            error_message: None,
            created_by: "admin".to_string(),
            created_at: now,
            updated_at: now,
        };

        let dto: BatchTaskDto = row.into();
        assert_eq!(dto.id, 1);
        assert_eq!(dto.task_type, "batch_grant");
        assert_eq!(dto.status, "pending");
        assert_eq!(dto.progress, 0);
        assert_eq!(
            dto.file_url,
            Some("https://oss.example.com/users.csv".to_string())
        );
        assert!(dto.result_file_url.is_none());
        assert!(dto.error_message.is_none());
    }

    #[test]
    fn test_batch_task_dto_serialization() {
        let dto = BatchTaskDto {
            id: 42,
            task_type: "data_export".to_string(),
            status: "completed".to_string(),
            total_count: 100,
            success_count: 95,
            failure_count: 5,
            progress: 100,
            file_url: None,
            result_file_url: Some("https://oss.example.com/result.csv".to_string()),
            error_message: None,
            created_by: "admin001".to_string(),
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };

        let json = serde_json::to_string(&dto).unwrap();
        assert!(json.contains("\"taskType\":\"data_export\""));
        assert!(json.contains("\"progress\":100"));
        assert!(json.contains("\"successCount\":95"));
        assert!(json.contains("\"failureCount\":5"));
    }

    #[test]
    fn test_batch_task_filter_default() {
        let filter = BatchTaskFilter::default();
        assert!(filter.task_type.is_none());
        assert!(filter.status.is_none());
        assert!(filter.created_by.is_none());
    }
}
