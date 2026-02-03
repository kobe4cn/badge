//! 发放管理 API 处理器
//!
//! 实现徽章的手动发放、批量发放和发放记录查询。
//! 发放操作涉及多表事务：user_badges + badge_ledger + user_badge_logs。

use axum::{
    Json,
    extract::{Path, Query, State},
    http::{HeaderMap, HeaderValue, StatusCode, header},
    response::IntoResponse,
};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use tracing::info;
use uuid::Uuid;
use validator::Validate;

use crate::{
    dto::{
        ApiResponse, BatchGrantRequest, BatchTaskDto, GrantLogDto, GrantLogFilter,
        ManualGrantRequest, PageResponse, PaginationParams,
    },
    error::AdminError,
    state::AppState,
};

/// 发放记录数据库查询结果
///
/// source_type 使用 String 而非 SourceType 枚举，以兼容数据库中可能存在的不同大小写格式
#[derive(sqlx::FromRow)]
struct GrantLogRow {
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
    let badge: Option<(i64, String, Option<i64>, i64)> =
        sqlx::query_as("SELECT id, name, max_supply, issued_count FROM badges WHERE id = $1")
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
    // 注意：UserBadgeStatus 枚举使用 SCREAMING_SNAKE_CASE，必须使用大写 'ACTIVE'
    sqlx::query(
        r#"
        INSERT INTO user_badges (user_id, badge_id, quantity, status, first_acquired_at, source_type, created_at, updated_at)
        VALUES ($1, $2, $3, 'ACTIVE', $4, 'MANUAL', $4, $4)
        ON CONFLICT (user_id, badge_id)
        DO UPDATE SET
            quantity = user_badges.quantity + $3,
            status = 'ACTIVE',
            updated_at = $4
        "#,
    )
    .bind(&req.user_id)
    .bind(req.badge_id)
    .bind(req.quantity)
    .bind(now)
    .execute(&mut *tx)
    .await?;

    // 2. 写入 badge_ledger（需要计算 balance_after）
    let balance_row: (i32,) = sqlx::query_as(
        "SELECT COALESCE(SUM(quantity), 0)::INT FROM badge_ledger WHERE user_id = $1 AND badge_id = $2",
    )
    .bind(&req.user_id)
    .bind(req.badge_id)
    .fetch_one(&mut *tx)
    .await?;
    let balance_after = balance_row.0 + req.quantity;

    sqlx::query(
        r#"
        INSERT INTO badge_ledger (user_id, badge_id, change_type, source_type, ref_id, quantity, balance_after, remark, created_at)
        VALUES ($1, $2, 'acquire', 'MANUAL', $3, $4, $5, $6, $7)
        "#,
    )
    .bind(&req.user_id)
    .bind(req.badge_id)
    .bind(&source_ref_id)
    .bind(req.quantity)
    .bind(balance_after)
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
    sqlx::query(
        "UPDATE badges SET issued_count = issued_count + $2, updated_at = $3 WHERE id = $1",
    )
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
    let badge_exists: (bool,) = sqlx::query_as("SELECT EXISTS(SELECT 1 FROM badges WHERE id = $1)")
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

    info!(
        task_id = row.0,
        badge_id = req.badge_id,
        "Batch grant task created"
    );

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
            l.ref_id as source_id,
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

/// 发放日志列表查询（分页）
///
/// GET /api/admin/grants/logs
///
/// 与 list_grants 共享查询逻辑，作为日志视图的独立入口，
/// 便于后续日志视图与发放记录视图独立演进。
pub async fn list_grant_logs(
    state: State<AppState>,
    pagination: Query<PaginationParams>,
    filter: Query<GrantLogFilter>,
) -> Result<Json<ApiResponse<PageResponse<GrantLogDto>>>, AdminError> {
    list_grants(state, pagination, filter).await
}

/// 发放日志详情
///
/// GET /api/admin/grants/logs/:id
///
/// 查询 badge_ledger 中单条 acquire 记录，关联徽章名称返回完整信息
pub async fn get_grant_log_detail(
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> Result<Json<ApiResponse<GrantLogDto>>, AdminError> {
    let row = sqlx::query_as::<_, GrantLogRow>(
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
        WHERE l.id = $1 AND l.change_type = 'acquire'
        "#,
    )
    .bind(id)
    .fetch_optional(&state.pool)
    .await?;

    let row = row.ok_or(AdminError::NotFound(format!("发放日志不存在: {id}")))?;
    Ok(Json(ApiResponse::success(row.into())))
}

/// 发放记录列表查询（分页）
///
/// GET /api/admin/grants/records
///
/// 当前与 list_grant_logs 共享同一查询逻辑，
/// 保留独立入口以便后续 records 视图增加不同的字段或聚合。
pub async fn list_grant_records(
    state: State<AppState>,
    pagination: Query<PaginationParams>,
    filter: Query<GrantLogFilter>,
) -> Result<Json<ApiResponse<PageResponse<GrantLogDto>>>, AdminError> {
    list_grants(state, pagination, filter).await
}

/// 导出发放日志为 CSV
///
/// GET /api/admin/grants/logs/export
///
/// 按过滤条件查询所有匹配记录（不分页），生成 CSV 文件流式返回。
/// 使用 10000 条硬上限防止超大导出导致 OOM。
pub async fn export_grant_logs(
    State(state): State<AppState>,
    Query(filter): Query<GrantLogFilter>,
) -> Result<impl IntoResponse, AdminError> {
    let source_type_str = filter.source_type.map(|t| {
        serde_json::to_value(t)
            .ok()
            .and_then(|v| v.as_str().map(|s| s.to_lowercase()))
            .unwrap_or_default()
    });

    let rows = sqlx::query_as::<_, GrantLogRow>(
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
        WHERE l.change_type = 'acquire'
          AND ($1::text IS NULL OR l.user_id = $1)
          AND ($2::bigint IS NULL OR l.badge_id = $2)
          AND ($3::text IS NULL OR l.source_type::text = $3)
          AND ($4::timestamptz IS NULL OR l.created_at >= $4)
          AND ($5::timestamptz IS NULL OR l.created_at <= $5)
        ORDER BY l.created_at DESC
        LIMIT 10000
        "#,
    )
    .bind(&filter.user_id)
    .bind(filter.badge_id)
    .bind(&source_type_str)
    .bind(filter.start_time)
    .bind(filter.end_time)
    .fetch_all(&state.pool)
    .await?;

    // 构建 CSV 内容
    let mut csv = String::from("id,user_id,badge_name,action,source_type,created_at\n");
    for row in &rows {
        let source_type_display = serde_json::to_value(&row.source_type)
            .ok()
            .and_then(|v| v.as_str().map(|s| s.to_string()))
            .unwrap_or_default();

        csv.push_str(&format!(
            "{},{},{},grant,{},{}\n",
            row.id,
            row.user_id,
            escape_csv_field(&row.badge_name),
            source_type_display,
            row.created_at.format("%Y-%m-%d %H:%M:%S"),
        ));
    }

    let mut headers = HeaderMap::new();
    headers.insert(
        header::CONTENT_TYPE,
        HeaderValue::from_static("text/csv; charset=utf-8"),
    );
    headers.insert(
        header::CONTENT_DISPOSITION,
        HeaderValue::from_static("attachment; filename=\"grant_logs.csv\""),
    );

    Ok((StatusCode::OK, headers, csv))
}

/// 对 CSV 字段进行转义
///
/// 当字段包含逗号、双引号或换行符时，用双引号包裹并转义内部双引号
fn escape_csv_field(field: &str) -> String {
    if field.contains(',') || field.contains('"') || field.contains('\n') {
        format!("\"{}\"", field.replace('"', "\"\""))
    } else {
        field.to_string()
    }
}

/// CSV 上传解析结果
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CsvUploadResult {
    total: usize,
    valid: usize,
    invalid: usize,
    preview: Vec<String>,
}

/// 上传用户 CSV 文件
///
/// POST /api/admin/grants/upload-csv
///
/// 接收 CSV 文本内容（JSON body），解析并返回预览结果。
/// 简化方案：前端读取文件后以 JSON 发送 CSV 内容，避免 multipart 依赖。
pub async fn upload_user_csv(
    State(_state): State<AppState>,
    Json(body): Json<serde_json::Value>,
) -> Result<Json<ApiResponse<CsvUploadResult>>, AdminError> {
    let csv_content = body
        .get("content")
        .and_then(|v| v.as_str())
        .ok_or_else(|| AdminError::Validation("缺少 content 字段".to_string()))?;

    let mut lines = csv_content.lines();

    // 解析表头，查找 user_id 列
    let header_line = lines
        .next()
        .ok_or_else(|| AdminError::Validation("CSV 文件为空".to_string()))?;

    let headers: Vec<&str> = header_line.split(',').map(|h| h.trim()).collect();
    let user_id_col = headers
        .iter()
        .position(|h| h.eq_ignore_ascii_case("user_id"))
        .ok_or_else(|| AdminError::Validation("CSV 缺少 user_id 列".to_string()))?;

    let mut total = 0usize;
    let mut valid = 0usize;
    let mut invalid = 0usize;
    let mut preview = Vec::new();

    for line in lines {
        if line.trim().is_empty() {
            continue;
        }
        total += 1;
        let cols: Vec<&str> = line.split(',').collect();
        if let Some(uid) = cols.get(user_id_col) {
            let uid = uid.trim();
            if !uid.is_empty() {
                valid += 1;
                if preview.len() < 10 {
                    preview.push(uid.to_string());
                }
            } else {
                invalid += 1;
            }
        } else {
            invalid += 1;
        }
    }

    Ok(Json(ApiResponse::success(CsvUploadResult {
        total,
        valid,
        invalid,
        preview,
    })))
}

/// 用户筛选预览请求
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
#[allow(dead_code)]
pub struct UserFilterRequest {
    badge_id: Option<i64>,
    min_quantity: Option<i32>,
    status: Option<String>,
}

/// 预览用户筛选结果
///
/// POST /api/admin/grants/preview-filter
///
/// 根据筛选条件返回匹配用户数量和预览列表。
/// 当前为占位实现，后续可接入用户查询系统。
pub async fn preview_user_filter(
    State(_state): State<AppState>,
    Json(_req): Json<UserFilterRequest>,
) -> Result<Json<ApiResponse<serde_json::Value>>, AdminError> {
    let result = serde_json::json!({
        "total": 0,
        "users": []
    });
    Ok(Json(ApiResponse::success(result)))
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

    #[test]
    fn test_escape_csv_field() {
        use super::escape_csv_field;

        // 普通字段无需转义
        assert_eq!(escape_csv_field("hello"), "hello");

        // 包含逗号需要用双引号包裹
        assert_eq!(escape_csv_field("hello,world"), "\"hello,world\"");

        // 包含双引号需要转义
        assert_eq!(escape_csv_field("say \"hi\""), "\"say \"\"hi\"\"\"");

        // 包含换行符
        assert_eq!(escape_csv_field("line1\nline2"), "\"line1\nline2\"");
    }
}
