//! 通知配置 API 处理器
//!
//! 管理徽章/权益相关事件的通知规则和发送记录

use axum::{
    extract::{Path, Query, State},
    Json,
};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use tracing::info;
use validator::Validate;

use crate::{
    dto::{ApiResponse, PageResponse, PaginationParams},
    error::AdminError,
    state::AppState,
};

// ═══════════════════════════════════════════════════════════════════════════
// DTO 定义
// ═══════════════════════════════════════════════════════════════════════════

/// 通知配置 DTO
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct NotificationConfigDto {
    pub id: i64,
    pub badge_id: Option<i64>,
    pub badge_name: Option<String>,
    pub benefit_id: Option<i64>,
    pub benefit_name: Option<String>,
    pub trigger_type: String,
    pub channels: Vec<String>,
    pub template_id: Option<String>,
    pub advance_days: Option<i32>,
    pub retry_count: i32,
    pub retry_interval_seconds: i32,
    pub status: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// 通知任务 DTO
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct NotificationTaskDto {
    pub id: i64,
    pub user_id: String,
    pub trigger_type: String,
    pub channels: Vec<String>,
    pub template_id: Option<String>,
    pub status: String,
    pub retry_count: i32,
    pub max_retries: i32,
    pub last_error: Option<String>,
    pub completed_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
}

/// 创建通知配置请求
#[derive(Debug, Deserialize, Validate)]
#[serde(rename_all = "camelCase")]
pub struct CreateNotificationConfigRequest {
    pub badge_id: Option<i64>,
    pub benefit_id: Option<i64>,
    #[validate(length(min = 1, max = 20, message = "触发类型长度必须在1-20个字符之间"))]
    pub trigger_type: String,
    #[validate(length(min = 1, message = "至少选择一个通知渠道"))]
    pub channels: Vec<String>,
    pub template_id: Option<String>,
    pub advance_days: Option<i32>,
    #[validate(range(min = 0, max = 10, message = "重试次数必须在0-10之间"))]
    pub retry_count: Option<i32>,
    #[validate(range(min = 10, max = 3600, message = "重试间隔必须在10-3600秒之间"))]
    pub retry_interval_seconds: Option<i32>,
}

/// 更新通知配置请求
#[derive(Debug, Deserialize, Validate)]
#[serde(rename_all = "camelCase")]
pub struct UpdateNotificationConfigRequest {
    pub channels: Option<Vec<String>>,
    pub template_id: Option<String>,
    pub advance_days: Option<i32>,
    pub retry_count: Option<i32>,
    pub retry_interval_seconds: Option<i32>,
    pub status: Option<String>,
}

/// 通知任务查询过滤
#[derive(Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NotificationTaskFilter {
    pub user_id: Option<String>,
    pub trigger_type: Option<String>,
    pub status: Option<String>,
}

/// 测试通知请求
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TestNotificationRequest {
    pub user_id: String,
    pub channels: Vec<String>,
    pub template_id: Option<String>,
    pub template_params: Option<serde_json::Value>,
}

/// 测试通知结果
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TestNotificationResult {
    pub success: bool,
    pub channels_sent: Vec<String>,
    pub errors: Vec<String>,
}

// ═══════════════════════════════════════════════════════════════════════════
// 数据库行映射
// ═══════════════════════════════════════════════════════════════════════════

#[derive(FromRow)]
struct NotificationConfigRow {
    id: i64,
    badge_id: Option<i64>,
    badge_name: Option<String>,
    benefit_id: Option<i64>,
    benefit_name: Option<String>,
    trigger_type: String,
    channels: serde_json::Value,
    template_id: Option<String>,
    advance_days: Option<i32>,
    retry_count: i32,
    retry_interval_seconds: i32,
    status: String,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

impl From<NotificationConfigRow> for NotificationConfigDto {
    fn from(row: NotificationConfigRow) -> Self {
        let channels: Vec<String> = serde_json::from_value(row.channels).unwrap_or_default();
        Self {
            id: row.id,
            badge_id: row.badge_id,
            badge_name: row.badge_name,
            benefit_id: row.benefit_id,
            benefit_name: row.benefit_name,
            trigger_type: row.trigger_type,
            channels,
            template_id: row.template_id,
            advance_days: row.advance_days,
            retry_count: row.retry_count,
            retry_interval_seconds: row.retry_interval_seconds,
            status: row.status,
            created_at: row.created_at,
            updated_at: row.updated_at,
        }
    }
}

#[derive(FromRow)]
struct NotificationTaskRow {
    id: i64,
    user_id: String,
    trigger_type: String,
    channels: serde_json::Value,
    template_id: Option<String>,
    status: String,
    retry_count: i32,
    max_retries: i32,
    last_error: Option<String>,
    completed_at: Option<DateTime<Utc>>,
    created_at: DateTime<Utc>,
}

impl From<NotificationTaskRow> for NotificationTaskDto {
    fn from(row: NotificationTaskRow) -> Self {
        let channels: Vec<String> = serde_json::from_value(row.channels).unwrap_or_default();
        Self {
            id: row.id,
            user_id: row.user_id,
            trigger_type: row.trigger_type,
            channels,
            template_id: row.template_id,
            status: row.status,
            retry_count: row.retry_count,
            max_retries: row.max_retries,
            last_error: row.last_error,
            completed_at: row.completed_at,
            created_at: row.created_at,
        }
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// API 处理器
// ═══════════════════════════════════════════════════════════════════════════

const CONFIG_FULL_SQL: &str = r#"
    SELECT
        nc.id,
        nc.badge_id,
        b.name as badge_name,
        nc.benefit_id,
        be.name as benefit_name,
        nc.trigger_type,
        nc.channels,
        nc.template_id,
        nc.advance_days,
        nc.retry_count,
        nc.retry_interval_seconds,
        nc.status,
        nc.created_at,
        nc.updated_at
    FROM notification_configs nc
    LEFT JOIN badges b ON b.id = nc.badge_id
    LEFT JOIN benefits be ON be.id = nc.benefit_id
"#;

/// 获取通知配置列表
///
/// GET /api/admin/notification-configs
pub async fn list_notification_configs(
    State(state): State<AppState>,
    Query(pagination): Query<PaginationParams>,
) -> Result<Json<ApiResponse<PageResponse<NotificationConfigDto>>>, AdminError> {
    let offset = pagination.offset();
    let limit = pagination.limit();

    let total: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM notification_configs")
        .fetch_one(&state.pool)
        .await?;

    if total.0 == 0 {
        return Ok(Json(ApiResponse::success(PageResponse::empty(
            pagination.page,
            pagination.page_size,
        ))));
    }

    let sql = format!(
        "{} ORDER BY nc.created_at DESC LIMIT $1 OFFSET $2",
        CONFIG_FULL_SQL
    );

    let rows = sqlx::query_as::<_, NotificationConfigRow>(&sql)
        .bind(limit)
        .bind(offset)
        .fetch_all(&state.pool)
        .await?;

    let items: Vec<NotificationConfigDto> = rows.into_iter().map(Into::into).collect();
    let response = PageResponse::new(items, total.0, pagination.page, pagination.page_size);
    Ok(Json(ApiResponse::success(response)))
}

/// 获取通知配置详情
///
/// GET /api/admin/notification-configs/:id
pub async fn get_notification_config(
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> Result<Json<ApiResponse<NotificationConfigDto>>, AdminError> {
    let sql = format!("{} WHERE nc.id = $1", CONFIG_FULL_SQL);

    let row = sqlx::query_as::<_, NotificationConfigRow>(&sql)
        .bind(id)
        .fetch_optional(&state.pool)
        .await?
        .ok_or(AdminError::NotFound(format!("通知配置 {} 不存在", id)))?;

    Ok(Json(ApiResponse::success(row.into())))
}

/// 创建通知配置
///
/// POST /api/admin/notification-configs
pub async fn create_notification_config(
    State(state): State<AppState>,
    Json(req): Json<CreateNotificationConfigRequest>,
) -> Result<Json<ApiResponse<NotificationConfigDto>>, AdminError> {
    req.validate()?;

    // 至少关联一个徽章或权益
    if req.badge_id.is_none() && req.benefit_id.is_none() {
        return Err(AdminError::Validation(
            "必须关联一个徽章或权益".to_string(),
        ));
    }

    // 验证触发类型
    let valid_triggers = ["grant", "revoke", "expire", "expire_remind", "redeem"];
    if !valid_triggers.contains(&req.trigger_type.as_str()) {
        return Err(AdminError::Validation(format!(
            "无效的触发类型: {}，支持: {:?}",
            req.trigger_type, valid_triggers
        )));
    }

    // 验证通知渠道
    let valid_channels = ["app_push", "sms", "wechat", "email", "in_app"];
    for ch in &req.channels {
        if !valid_channels.contains(&ch.as_str()) {
            return Err(AdminError::Validation(format!(
                "无效的通知渠道: {}，支持: {:?}",
                ch, valid_channels
            )));
        }
    }

    let channels_json = serde_json::to_value(&req.channels)?;

    let row: (i64,) = sqlx::query_as(
        r#"
        INSERT INTO notification_configs
            (badge_id, benefit_id, trigger_type, channels, template_id, advance_days, retry_count, retry_interval_seconds)
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
        RETURNING id
        "#,
    )
    .bind(req.badge_id)
    .bind(req.benefit_id)
    .bind(&req.trigger_type)
    .bind(&channels_json)
    .bind(&req.template_id)
    .bind(req.advance_days)
    .bind(req.retry_count.unwrap_or(3))
    .bind(req.retry_interval_seconds.unwrap_or(60))
    .fetch_one(&state.pool)
    .await?;

    info!(config_id = row.0, trigger_type = %req.trigger_type, "通知配置已创建");

    // 回查完整数据
    let sql = format!("{} WHERE nc.id = $1", CONFIG_FULL_SQL);
    let config = sqlx::query_as::<_, NotificationConfigRow>(&sql)
        .bind(row.0)
        .fetch_one(&state.pool)
        .await?;

    Ok(Json(ApiResponse::success(config.into())))
}

/// 更新通知配置
///
/// PUT /api/admin/notification-configs/:id
pub async fn update_notification_config(
    State(state): State<AppState>,
    Path(id): Path<i64>,
    Json(req): Json<UpdateNotificationConfigRequest>,
) -> Result<Json<ApiResponse<NotificationConfigDto>>, AdminError> {
    req.validate()?;

    // 检查配置是否存在
    let exists: (bool,) =
        sqlx::query_as("SELECT EXISTS(SELECT 1 FROM notification_configs WHERE id = $1)")
            .bind(id)
            .fetch_one(&state.pool)
            .await?;

    if !exists.0 {
        return Err(AdminError::NotFound(format!("通知配置 {} 不存在", id)));
    }

    // 验证通知渠道
    if let Some(ref channels) = req.channels {
        let valid_channels = ["app_push", "sms", "wechat", "email", "in_app"];
        for ch in channels {
            if !valid_channels.contains(&ch.as_str()) {
                return Err(AdminError::Validation(format!(
                    "无效的通知渠道: {}",
                    ch
                )));
            }
        }
    }

    // 验证状态
    if let Some(ref status) = req.status {
        let valid_statuses = ["active", "inactive"];
        if !valid_statuses.contains(&status.as_str()) {
            return Err(AdminError::Validation(format!(
                "无效的状态: {}，支持: {:?}",
                status, valid_statuses
            )));
        }
    }

    let channels_json = req.channels.as_ref().map(|c| serde_json::to_value(c).ok()).flatten();

    sqlx::query(
        r#"
        UPDATE notification_configs
        SET
            channels = COALESCE($2, channels),
            template_id = COALESCE($3, template_id),
            advance_days = COALESCE($4, advance_days),
            retry_count = COALESCE($5, retry_count),
            retry_interval_seconds = COALESCE($6, retry_interval_seconds),
            status = COALESCE($7, status),
            updated_at = NOW()
        WHERE id = $1
        "#,
    )
    .bind(id)
    .bind(&channels_json)
    .bind(&req.template_id)
    .bind(req.advance_days)
    .bind(req.retry_count)
    .bind(req.retry_interval_seconds)
    .bind(&req.status)
    .execute(&state.pool)
    .await?;

    info!(config_id = id, "通知配置已更新");

    // 回查完整数据
    let sql = format!("{} WHERE nc.id = $1", CONFIG_FULL_SQL);
    let config = sqlx::query_as::<_, NotificationConfigRow>(&sql)
        .bind(id)
        .fetch_one(&state.pool)
        .await?;

    Ok(Json(ApiResponse::success(config.into())))
}

/// 删除通知配置
///
/// DELETE /api/admin/notification-configs/:id
pub async fn delete_notification_config(
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> Result<Json<ApiResponse<()>>, AdminError> {
    let result = sqlx::query("DELETE FROM notification_configs WHERE id = $1")
        .bind(id)
        .execute(&state.pool)
        .await?;

    if result.rows_affected() == 0 {
        return Err(AdminError::NotFound(format!("通知配置 {} 不存在", id)));
    }

    info!(config_id = id, "通知配置已删除");
    Ok(Json(ApiResponse::<()>::success_empty()))
}

/// 获取通知任务列表
///
/// GET /api/admin/notification-tasks
pub async fn list_notification_tasks(
    State(state): State<AppState>,
    Query(pagination): Query<PaginationParams>,
    Query(filter): Query<NotificationTaskFilter>,
) -> Result<Json<ApiResponse<PageResponse<NotificationTaskDto>>>, AdminError> {
    let offset = pagination.offset();
    let limit = pagination.limit();

    // 构建动态 WHERE 子句
    let mut conditions = Vec::new();
    let mut param_idx = 1;

    if filter.user_id.is_some() {
        conditions.push(format!("user_id = ${}", param_idx));
        param_idx += 1;
    }
    if filter.trigger_type.is_some() {
        conditions.push(format!("trigger_type = ${}", param_idx));
        param_idx += 1;
    }
    if filter.status.is_some() {
        conditions.push(format!("status = ${}", param_idx));
        param_idx += 1;
    }

    let where_clause = if conditions.is_empty() {
        String::new()
    } else {
        format!("WHERE {}", conditions.join(" AND "))
    };

    // 统计总数
    let count_sql = format!("SELECT COUNT(*) FROM notification_tasks {}", where_clause);
    let mut count_query = sqlx::query_as::<_, (i64,)>(&count_sql);

    if let Some(ref user_id) = filter.user_id {
        count_query = count_query.bind(user_id);
    }
    if let Some(ref trigger_type) = filter.trigger_type {
        count_query = count_query.bind(trigger_type);
    }
    if let Some(ref status) = filter.status {
        count_query = count_query.bind(status);
    }

    let total = count_query.fetch_one(&state.pool).await?;

    if total.0 == 0 {
        return Ok(Json(ApiResponse::success(PageResponse::empty(
            pagination.page,
            pagination.page_size,
        ))));
    }

    // 查询数据
    let data_sql = format!(
        r#"
        SELECT id, user_id, trigger_type, channels, template_id, status,
               retry_count, max_retries, last_error, completed_at, created_at
        FROM notification_tasks
        {}
        ORDER BY created_at DESC
        LIMIT ${} OFFSET ${}
        "#,
        where_clause,
        param_idx,
        param_idx + 1
    );

    let mut data_query = sqlx::query_as::<_, NotificationTaskRow>(&data_sql);

    if let Some(ref user_id) = filter.user_id {
        data_query = data_query.bind(user_id);
    }
    if let Some(ref trigger_type) = filter.trigger_type {
        data_query = data_query.bind(trigger_type);
    }
    if let Some(ref status) = filter.status {
        data_query = data_query.bind(status);
    }

    let rows = data_query
        .bind(limit)
        .bind(offset)
        .fetch_all(&state.pool)
        .await?;

    let items: Vec<NotificationTaskDto> = rows.into_iter().map(Into::into).collect();
    let response = PageResponse::new(items, total.0, pagination.page, pagination.page_size);
    Ok(Json(ApiResponse::success(response)))
}

/// 测试通知发送
///
/// POST /api/admin/notification-configs/test
pub async fn test_notification(
    State(_state): State<AppState>,
    Json(req): Json<TestNotificationRequest>,
) -> Result<Json<ApiResponse<TestNotificationResult>>, AdminError> {
    // TODO: 实际对接通知服务
    info!(
        user_id = %req.user_id,
        channels = ?req.channels,
        template_id = ?req.template_id,
        "测试通知发送"
    );

    // 模拟测试结果
    let result = TestNotificationResult {
        success: true,
        channels_sent: req.channels.clone(),
        errors: vec![],
    };

    Ok(Json(ApiResponse::success(result)))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_notification_config_dto_serialization() {
        let dto = NotificationConfigDto {
            id: 1,
            badge_id: Some(10),
            badge_name: Some("测试徽章".to_string()),
            benefit_id: None,
            benefit_name: None,
            trigger_type: "grant".to_string(),
            channels: vec!["app_push".to_string(), "in_app".to_string()],
            template_id: Some("tpl_001".to_string()),
            advance_days: None,
            retry_count: 3,
            retry_interval_seconds: 60,
            status: "active".to_string(),
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };

        let json = serde_json::to_string(&dto).unwrap();
        assert!(json.contains("\"triggerType\":\"grant\""));
        assert!(json.contains("\"channels\":[\"app_push\",\"in_app\"]"));
    }

    #[test]
    fn test_create_request_validation() {
        let valid = CreateNotificationConfigRequest {
            badge_id: Some(1),
            benefit_id: None,
            trigger_type: "grant".to_string(),
            channels: vec!["app_push".to_string()],
            template_id: None,
            advance_days: None,
            retry_count: Some(3),
            retry_interval_seconds: Some(60),
        };
        assert!(valid.validate().is_ok());
    }
}
