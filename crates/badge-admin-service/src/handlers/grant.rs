//! 发放管理 API 处理器
//!
//! 实现徽章的手动发放、批量发放和发放记录查询。
//! 发放操作涉及多表事务：user_badges + badge_ledger + user_badge_logs。

use axum::{
    Extension, Json,
    extract::{Multipart, Path, Query, State},
    http::{HeaderMap, HeaderValue, StatusCode, header},
    response::IntoResponse,
};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use tracing::info;
use uuid::Uuid;
use validator::Validate;

use crate::{
    auth::Claims,
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
    // 数据库 DEFAULT 和 Worker 均使用小写 status，此处保持一致
    sqlx::query(
        r#"
        INSERT INTO user_badges (user_id, badge_id, quantity, status, first_acquired_at, source_type, created_at, updated_at)
        VALUES ($1, $2, $3, 'active', $4, 'MANUAL', $4, $4)
        ON CONFLICT (user_id, badge_id)
        DO UPDATE SET
            quantity = user_badges.quantity + $3,
            status = 'active',
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
        VALUES ($1, $2, 'grant', $3, 'MANUAL', $4, $5, $6)
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
    Extension(claims): Extension<Claims>,
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

    // 将业务参数序列化到 params 列，Worker 轮询到任务后从中解析 badge_id 和 reason
    let params = serde_json::json!({
        "badge_id": req.badge_id,
        "reason": &req.reason,
    });

    let created_by = &claims.username;

    let row: (i64,) = sqlx::query_as(
        r#"
        INSERT INTO batch_tasks (task_type, file_url, params, status, progress, total_count, success_count, failure_count, created_by, created_at, updated_at)
        VALUES ('batch_grant', $1, $2, 'pending', 0, 0, 0, 0, $4, $3, $3)
        RETURNING id
        "#,
    )
    .bind(&req.file_url)
    .bind(&params)
    .bind(now)
    .bind(created_by)
    .fetch_one(&state.pool)
    .await?;

    info!(
        task_id = row.0,
        badge_id = req.badge_id,
        created_by = %created_by,
        "Batch grant task created"
    );
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
        created_by: created_by.clone(),
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
            .and_then(|v| v.as_str().map(|s| s.to_string()))
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
            .and_then(|v| v.as_str().map(|s| s.to_string()))
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
///
/// userIds 存储在 Redis（通过 csvRefKey 引用），避免大列表在 HTTP 请求中传输。
/// 前端提交批量任务时只需传 csvRefKey，后端从 Redis 解析出用户列表。
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CsvUploadResult {
    /// Redis 中存储 userIds 的引用键，后续创建任务时传此键即可
    csv_ref_key: String,
    /// 有效用户数量
    valid_count: usize,
    /// 无效行号列表
    invalid_rows: Vec<usize>,
    /// 总行数
    total_rows: usize,
}

/// 上传用户 CSV 文件
///
/// POST /api/admin/grants/upload-csv
///
/// 接收 multipart/form-data 上传的 CSV 文件，解析并返回预览结果。
/// 前端通过 FormData 上传 file 字段，后端提取文件内容进行解析。
pub async fn upload_user_csv(
    State(state): State<AppState>,
    mut multipart: Multipart,
) -> Result<Json<ApiResponse<CsvUploadResult>>, AdminError> {
    let mut csv_content = String::new();

    while let Some(field) = multipart
        .next_field()
        .await
        .map_err(|e| AdminError::Validation(format!("解析上传文件失败: {}", e)))?
    {
        // 接受 name="file" 或 name="content" 的字段
        let name = field.name().unwrap_or("").to_string();
        if name == "file" || name == "content" {
            csv_content = field
                .text()
                .await
                .map_err(|e| AdminError::Validation(format!("读取文件内容失败: {}", e)))?;
            break;
        }
    }

    if csv_content.is_empty() {
        return Err(AdminError::Validation(
            "未上传文件或文件内容为空".to_string(),
        ));
    }

    let csv_content = &csv_content;

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

    let mut total_rows = 0usize;
    let mut user_ids = Vec::new();
    let mut invalid_rows = Vec::new();

    for line in lines {
        if line.trim().is_empty() {
            continue;
        }
        total_rows += 1;
        let cols: Vec<&str> = line.split(',').collect();
        if let Some(uid) = cols.get(user_id_col) {
            let uid = uid.trim();
            if !uid.is_empty() {
                user_ids.push(uid.to_string());
            } else {
                invalid_rows.push(total_rows);
            }
        } else {
            invalid_rows.push(total_rows);
        }
    }

    let valid_count = user_ids.len();

    // 将 userIds 存入 Redis（30 分钟 TTL），避免大列表随 HTTP 请求传输
    let csv_ref_key = format!("csv_ref:{}", Uuid::new_v4());
    state
        .cache
        .set(
            &csv_ref_key,
            &user_ids,
            std::time::Duration::from_secs(30 * 60),
        )
        .await
        .map_err(|e| AdminError::Internal(format!("Redis 存储 CSV 结果失败: {}", e)))?;

    Ok(Json(ApiResponse::success(CsvUploadResult {
        csv_ref_key,
        valid_count,
        invalid_rows,
        total_rows,
    })))
}

/// 用户筛选预览请求
///
/// 前端在批量发放前调用此接口预估影响范围，
/// 避免盲目发放导致不可预期的大规模变更
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UserFilterRequest {
    pub badge_id: Option<i64>,
    pub min_quantity: Option<i32>,
    pub status: Option<String>,
    /// 预览列表的页码，默认第 1 页
    #[serde(default = "default_preview_page")]
    pub page: i64,
    /// 预览列表每页条数，默认 20
    #[serde(default = "default_preview_page_size")]
    pub page_size: i64,
}

fn default_preview_page() -> i64 {
    1
}

fn default_preview_page_size() -> i64 {
    20
}

/// 用户筛选预览结果中的单条用户信息
#[derive(Debug, Serialize, sqlx::FromRow)]
#[serde(rename_all = "camelCase")]
struct FilteredUserRow {
    user_id: String,
    badge_id: i64,
    quantity: i32,
    status: String,
    first_acquired_at: DateTime<Utc>,
}

/// 预览用户筛选结果
///
/// POST /api/admin/grants/preview-filter
///
/// 批量发放前的"干跑"接口：根据筛选条件查询 user_badges 表，
/// 返回匹配的用户列表和总数，帮助运营人员确认影响范围后再执行实际发放。
pub async fn preview_user_filter(
    State(state): State<AppState>,
    Json(req): Json<UserFilterRequest>,
) -> Result<Json<ApiResponse<serde_json::Value>>, AdminError> {
    let page = req.page.max(1);
    // 预览场景不需要返回大量数据，硬性限制每页最多 100 条
    let page_size = req.page_size.clamp(1, 100);
    let offset = (page - 1) * page_size;

    // 先查总数，用于前端展示影响范围和分页控件
    let (total,): (i64,) = sqlx::query_as(
        r#"
        SELECT COUNT(*)
        FROM user_badges ub
        WHERE ($1::bigint IS NULL OR ub.badge_id = $1)
          AND ($2::int IS NULL OR ub.quantity >= $2)
          AND ($3::text IS NULL OR ub.status = $3)
        "#,
    )
    .bind(req.badge_id)
    .bind(req.min_quantity)
    .bind(&req.status)
    .fetch_one(&state.pool)
    .await?;

    // 总数为零时提前返回，省去一次无意义的分页查询
    if total == 0 {
        let result = serde_json::json!({
            "total": 0,
            "page": page,
            "pageSize": page_size,
            "totalPages": 0,
            "users": []
        });
        return Ok(Json(ApiResponse::success(result)));
    }

    let rows = sqlx::query_as::<_, FilteredUserRow>(
        r#"
        SELECT ub.user_id, ub.badge_id, ub.quantity, ub.status, ub.first_acquired_at
        FROM user_badges ub
        WHERE ($1::bigint IS NULL OR ub.badge_id = $1)
          AND ($2::int IS NULL OR ub.quantity >= $2)
          AND ($3::text IS NULL OR ub.status = $3)
        ORDER BY ub.first_acquired_at DESC
        LIMIT $4 OFFSET $5
        "#,
    )
    .bind(req.badge_id)
    .bind(req.min_quantity)
    .bind(&req.status)
    .bind(page_size)
    .bind(offset)
    .fetch_all(&state.pool)
    .await?;

    let total_pages = if page_size > 0 {
        (total + page_size - 1) / page_size
    } else {
        0
    };

    let result = serde_json::json!({
        "total": total,
        "page": page,
        "pageSize": page_size,
        "totalPages": total_pages,
        "users": rows
    });

    Ok(Json(ApiResponse::success(result)))
}

#[cfg(test)]
mod tests {
    use crate::dto::{ManualGrantRequest, RecipientType};
    use validator::Validate;

    #[test]
    fn test_manual_grant_request_validation() {
        let valid = ManualGrantRequest {
            user_id: "user001".to_string(),
            badge_id: 1,
            quantity: 1,
            reason: "活动奖励".to_string(),
            recipient_type: RecipientType::default(),
            actual_user_id: None,
        };
        assert!(valid.validate().is_ok());

        // 数量超限
        let invalid = ManualGrantRequest {
            user_id: "user001".to_string(),
            badge_id: 1,
            quantity: 101,
            reason: "活动奖励".to_string(),
            recipient_type: RecipientType::default(),
            actual_user_id: None,
        };
        assert!(invalid.validate().is_err());

        // 空原因
        let invalid_reason = ManualGrantRequest {
            user_id: "user001".to_string(),
            badge_id: 1,
            quantity: 1,
            reason: "".to_string(),
            recipient_type: RecipientType::default(),
            actual_user_id: None,
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
