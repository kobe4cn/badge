//! 兑换管理 API 处理器
//!
//! 实现兑换规则管理和兑换操作的 HTTP 接口

use axum::{
    Extension, Json,
    extract::{Path, Query, State},
    http::HeaderMap,
};
use crate::middleware::AuditContext;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use tracing::info;
use uuid::Uuid;
use validator::Validate;

use badge_management::service::dto::RedeemBadgeRequest;

use crate::{
    dto::{ApiResponse, PageResponse, PaginationParams},
    error::AdminError,
    state::AppState,
};

// ==================== DTO 定义 ====================

/// 有效期类型
#[derive(Debug, Clone, Default, Serialize, Deserialize, sqlx::Type)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
#[sqlx(type_name = "varchar", rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ValidityType {
    /// 固定时间段（使用 start_time/end_time）
    #[default]
    Fixed,
    /// 相对于徽章获取时间
    Relative,
}

/// 兑换规则响应 DTO
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RedemptionRuleDto {
    pub id: i64,
    pub name: String,
    pub description: Option<String>,
    pub benefit_id: i64,
    pub benefit_name: String,
    pub required_badges: Vec<RequiredBadgeDto>,
    pub frequency_config: FrequencyConfigDto,
    /// 有效期类型：FIXED-固定时间段，RELATIVE-相对徽章获取时间
    pub validity_type: ValidityType,
    /// 相对有效天数（validity_type=RELATIVE 时使用）
    pub relative_days: Option<i32>,
    pub start_time: Option<DateTime<Utc>>,
    pub end_time: Option<DateTime<Utc>>,
    pub enabled: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// 所需徽章 DTO
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RequiredBadgeDto {
    pub badge_id: i64,
    pub badge_name: String,
    pub quantity: i32,
}

/// 频率配置 DTO
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FrequencyConfigDto {
    pub max_per_user: Option<i32>,
    pub max_per_day: Option<i32>,
    pub max_per_week: Option<i32>,
    pub max_per_month: Option<i32>,
    pub max_per_year: Option<i32>,
}

/// 创建兑换规则请求
#[derive(Debug, Deserialize, Validate)]
#[serde(rename_all = "camelCase")]
pub struct CreateRedemptionRuleRequest {
    #[validate(length(min = 1, max = 100, message = "规则名称长度必须在1-100个字符之间"))]
    pub name: String,
    pub description: Option<String>,
    pub benefit_id: i64,
    #[validate(length(min = 1, message = "至少需要一个徽章"))]
    pub required_badges: Vec<RequiredBadgeInput>,
    pub frequency_config: Option<FrequencyConfigDto>,
    /// 有效期类型：FIXED-固定时间段，RELATIVE-相对徽章获取时间
    #[serde(default)]
    pub validity_type: ValidityType,
    /// 相对有效天数（validity_type=RELATIVE 时使用）
    pub relative_days: Option<i32>,
    pub start_time: Option<DateTime<Utc>>,
    pub end_time: Option<DateTime<Utc>>,
    /// 是否自动兑换：满足徽章条件时自动发放权益
    #[serde(default)]
    pub auto_redeem: bool,
}

/// 所需徽章输入
#[derive(Debug, Clone, Serialize, Deserialize, Validate)]
#[serde(rename_all = "camelCase")]
pub struct RequiredBadgeInput {
    pub badge_id: i64,
    #[validate(range(min = 1, message = "数量必须大于0"))]
    pub quantity: i32,
}

/// 更新兑换规则请求
#[derive(Debug, Deserialize, Validate)]
#[serde(rename_all = "camelCase")]
pub struct UpdateRedemptionRuleRequest {
    #[validate(length(min = 1, max = 100, message = "规则名称长度必须在1-100个字符之间"))]
    pub name: Option<String>,
    pub description: Option<String>,
    pub required_badges: Option<Vec<RequiredBadgeInput>>,
    pub frequency_config: Option<FrequencyConfigDto>,
    /// 有效期类型：FIXED-固定时间段，RELATIVE-相对徽章获取时间
    pub validity_type: Option<ValidityType>,
    /// 相对有效天数（validity_type=RELATIVE 时使用）
    pub relative_days: Option<i32>,
    pub start_time: Option<DateTime<Utc>>,
    pub end_time: Option<DateTime<Utc>>,
    pub enabled: Option<bool>,
}

/// 执行兑换请求
#[derive(Debug, Deserialize, Validate)]
#[serde(rename_all = "camelCase")]
pub struct RedeemRequest {
    #[validate(length(min = 1, message = "用户ID不能为空"))]
    pub user_id: String,
    pub rule_id: i64,
    /// 幂等键（可选，不提供时自动生成）
    pub idempotency_key: Option<String>,
}

/// 兑换响应 DTO
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RedeemResponseDto {
    pub success: bool,
    pub order_id: i64,
    pub order_no: String,
    pub benefit_name: String,
    pub message: String,
}

/// 兑换订单 DTO
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RedemptionOrderDto {
    pub id: i64,
    pub order_no: String,
    pub user_id: String,
    pub rule_id: i64,
    pub rule_name: String,
    pub benefit_id: i64,
    pub benefit_name: String,
    pub status: String,
    pub failure_reason: Option<String>,
    pub consumed_badges: Vec<ConsumedBadgeDto>,
    pub created_at: DateTime<Utc>,
}

/// 消耗的徽章 DTO
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ConsumedBadgeDto {
    pub badge_id: i64,
    pub badge_name: String,
    pub quantity: i32,
}

/// 订单查询过滤
#[derive(Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OrderQueryFilter {
    pub user_id: Option<String>,
    pub status: Option<String>,
    pub rule_id: Option<i64>,
}

// ==================== 数据库查询结构 ====================

#[derive(sqlx::FromRow)]
struct RedemptionRuleRow {
    id: i64,
    name: String,
    description: Option<String>,
    benefit_id: i64,
    benefit_name: String,
    required_badges: Value,
    frequency_config: Value,
    validity_type: Option<String>,
    relative_days: Option<i32>,
    start_time: Option<DateTime<Utc>>,
    end_time: Option<DateTime<Utc>>,
    enabled: bool,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

#[derive(sqlx::FromRow)]
struct RedemptionOrderRow {
    id: i64,
    order_no: String,
    user_id: String,
    rule_id: i64,
    rule_name: String,
    benefit_id: i64,
    benefit_name: String,
    status: String,
    failure_reason: Option<String>,
    created_at: DateTime<Utc>,
}

#[derive(sqlx::FromRow)]
struct RedemptionDetailRow {
    badge_id: i64,
    badge_name: String,
    quantity: i32,
}

// ==================== API 处理器 ====================

/// 创建兑换规则
///
/// POST /api/admin/redemption/rules
pub async fn create_redemption_rule(
    State(state): State<AppState>,
    Json(req): Json<CreateRedemptionRuleRequest>,
) -> Result<Json<ApiResponse<RedemptionRuleDto>>, AdminError> {
    req.validate()?;

    // 检查权益是否存在
    let benefit_exists: (bool,) =
        sqlx::query_as("SELECT EXISTS(SELECT 1 FROM benefits WHERE id = $1)")
            .bind(req.benefit_id)
            .fetch_one(&state.pool)
            .await?;

    if !benefit_exists.0 {
        return Err(AdminError::BenefitNotFound(req.benefit_id));
    }

    // 检查所需徽章是否都存在
    for badge in &req.required_badges {
        let badge_exists: (bool,) =
            sqlx::query_as("SELECT EXISTS(SELECT 1 FROM badges WHERE id = $1)")
                .bind(badge.badge_id)
                .fetch_one(&state.pool)
                .await?;

        if !badge_exists.0 {
            return Err(AdminError::BadgeNotFound(badge.badge_id));
        }
    }

    // 序列化所需徽章和频率配置
    let required_badges_json = serde_json::to_value(&req.required_badges)
        .map_err(|e| AdminError::Internal(e.to_string()))?;
    let frequency_config_json =
        serde_json::to_value(req.frequency_config.unwrap_or_default())
            .map_err(|e| AdminError::Internal(e.to_string()))?;

    // 序列化有效期类型
    let validity_type_str = match req.validity_type {
        ValidityType::Fixed => "FIXED",
        ValidityType::Relative => "RELATIVE",
    };

    let row: (i64,) = sqlx::query_as(
        r#"
        INSERT INTO badge_redemption_rules (name, description, benefit_id, required_badges,
                                           frequency_config, validity_type, relative_days,
                                           start_time, end_time, enabled, auto_redeem)
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, true, $10)
        RETURNING id
        "#,
    )
    .bind(&req.name)
    .bind(&req.description)
    .bind(req.benefit_id)
    .bind(&required_badges_json)
    .bind(&frequency_config_json)
    .bind(validity_type_str)
    .bind(req.relative_days)
    .bind(req.start_time)
    .bind(req.end_time)
    .bind(req.auto_redeem)
    .fetch_one(&state.pool)
    .await?;

    info!(rule_id = row.0, name = %req.name, "Redemption rule created");

    let dto = fetch_rule_by_id(&state.pool, row.0).await?;
    Ok(Json(ApiResponse::success(dto)))
}

/// 获取兑换规则列表
///
/// GET /api/admin/redemption/rules
pub async fn list_redemption_rules(
    State(state): State<AppState>,
    Query(pagination): Query<PaginationParams>,
) -> Result<Json<ApiResponse<PageResponse<RedemptionRuleDto>>>, AdminError> {
    let offset = pagination.offset();
    let limit = pagination.limit();

    // 查询总数
    let total: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM badge_redemption_rules")
        .fetch_one(&state.pool)
        .await?;

    if total.0 == 0 {
        return Ok(Json(ApiResponse::success(PageResponse::empty(
            pagination.page,
            pagination.page_size,
        ))));
    }

    // 查询列表
    let rows = sqlx::query_as::<_, RedemptionRuleRow>(
        r#"
        SELECT
            r.id, r.name, r.description, r.benefit_id,
            b.name as benefit_name,
            r.required_badges, r.frequency_config,
            r.validity_type, r.relative_days,
            r.start_time, r.end_time, r.enabled,
            r.created_at, r.updated_at
        FROM badge_redemption_rules r
        JOIN benefits b ON b.id = r.benefit_id
        ORDER BY r.created_at DESC
        LIMIT $1 OFFSET $2
        "#,
    )
    .bind(limit)
    .bind(offset)
    .fetch_all(&state.pool)
    .await?;

    let mut items = Vec::with_capacity(rows.len());
    for row in rows {
        items.push(row_to_dto(&state.pool, row).await?);
    }

    let response = PageResponse::new(items, total.0, pagination.page, pagination.page_size);
    Ok(Json(ApiResponse::success(response)))
}

/// 获取兑换规则详情
///
/// GET /api/admin/redemption/rules/:id
pub async fn get_redemption_rule(
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> Result<Json<ApiResponse<RedemptionRuleDto>>, AdminError> {
    let dto = fetch_rule_by_id(&state.pool, id).await?;
    Ok(Json(ApiResponse::success(dto)))
}

/// 更新兑换规则
///
/// PUT /api/admin/redemption/rules/:id
pub async fn update_redemption_rule(
    State(state): State<AppState>,
    Path(id): Path<i64>,
    Extension(audit_ctx): Extension<AuditContext>,
    Json(req): Json<UpdateRedemptionRuleRequest>,
) -> Result<Json<ApiResponse<RedemptionRuleDto>>, AdminError> {
    req.validate()?;

    // 检查规则是否存在
    let exists: (bool,) =
        sqlx::query_as("SELECT EXISTS(SELECT 1 FROM badge_redemption_rules WHERE id = $1)")
            .bind(id)
            .fetch_one(&state.pool)
            .await?;

    if !exists.0 {
        return Err(AdminError::RuleNotFound(id));
    }

    // 如果更新所需徽章，检查徽章是否存在
    if let Some(ref badges) = req.required_badges {
        for badge in badges {
            let badge_exists: (bool,) =
                sqlx::query_as("SELECT EXISTS(SELECT 1 FROM badges WHERE id = $1)")
                    .bind(badge.badge_id)
                    .fetch_one(&state.pool)
                    .await?;

            if !badge_exists.0 {
                return Err(AdminError::BadgeNotFound(badge.badge_id));
            }
        }
    }

    let required_badges_json = req
        .required_badges
        .as_ref()
        .and_then(|b| serde_json::to_value(b).ok());
    let frequency_config_json = req
        .frequency_config
        .as_ref()
        .and_then(|f| serde_json::to_value(f).ok());
    let validity_type_str: Option<&str> = req.validity_type.as_ref().map(|v| match v {
        ValidityType::Fixed => "FIXED",
        ValidityType::Relative => "RELATIVE",
    });

    // 审计快照：记录变更前状态
    audit_ctx.snapshot(&state.pool, "redemption_rules", id).await;

    sqlx::query(
        r#"
        UPDATE badge_redemption_rules
        SET
            name = COALESCE($2, name),
            description = COALESCE($3, description),
            required_badges = COALESCE($4, required_badges),
            frequency_config = COALESCE($5, frequency_config),
            validity_type = COALESCE($6, validity_type),
            relative_days = COALESCE($7, relative_days),
            start_time = COALESCE($8, start_time),
            end_time = COALESCE($9, end_time),
            enabled = COALESCE($10, enabled),
            updated_at = NOW()
        WHERE id = $1
        "#,
    )
    .bind(id)
    .bind(&req.name)
    .bind(&req.description)
    .bind(&required_badges_json)
    .bind(&frequency_config_json)
    .bind(validity_type_str)
    .bind(req.relative_days)
    .bind(req.start_time)
    .bind(req.end_time)
    .bind(req.enabled)
    .execute(&state.pool)
    .await?;

    info!(rule_id = id, "Redemption rule updated");

    let dto = fetch_rule_by_id(&state.pool, id).await?;
    Ok(Json(ApiResponse::success(dto)))
}

/// 删除兑换规则
///
/// DELETE /api/admin/redemption/rules/:id
pub async fn delete_redemption_rule(
    State(state): State<AppState>,
    Path(id): Path<i64>,
    Extension(audit_ctx): Extension<AuditContext>,
) -> Result<Json<ApiResponse<()>>, AdminError> {
    // 检查是否有关联订单
    let has_orders: (bool,) = sqlx::query_as(
        "SELECT EXISTS(SELECT 1 FROM redemption_orders WHERE redemption_rule_id = $1)",
    )
    .bind(id)
    .fetch_one(&state.pool)
    .await?;

    if has_orders.0 {
        return Err(AdminError::Validation(
            "该规则已有兑换订单，无法删除".to_string(),
        ));
    }

    // 审计快照：记录变更前状态
    audit_ctx.snapshot(&state.pool, "redemption_rules", id).await;

    let result = sqlx::query("DELETE FROM badge_redemption_rules WHERE id = $1")
        .bind(id)
        .execute(&state.pool)
        .await?;

    if result.rows_affected() == 0 {
        return Err(AdminError::RuleNotFound(id));
    }

    info!(rule_id = id, "Redemption rule deleted");

    Ok(Json(ApiResponse::<()>::success_empty()))
}

/// 执行兑换
///
/// POST /api/admin/redemption/redeem
pub async fn redeem(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(req): Json<RedeemRequest>,
) -> Result<Json<ApiResponse<RedeemResponseDto>>, AdminError> {
    req.validate()?;

    // 优先使用请求体中的 idempotency_key，其次从 Idempotency-Key HTTP 头读取
    let idempotency_key = req
        .idempotency_key
        .or_else(|| {
            headers
                .get("Idempotency-Key")
                .and_then(|v| v.to_str().ok())
                .map(|s| s.to_string())
        })
        .unwrap_or_else(|| Uuid::new_v4().to_string());

    // 检查幂等性
    let existing_order: Option<(i64, String, i64)> = sqlx::query_as(
        r#"
        SELECT id, order_no, benefit_id
        FROM redemption_orders
        WHERE idempotency_key = $1
        "#,
    )
    .bind(&idempotency_key)
    .fetch_optional(&state.pool)
    .await?;

    if let Some((order_id, order_no, benefit_id)) = existing_order {
        let benefit_name: String = sqlx::query_scalar("SELECT name FROM benefits WHERE id = $1")
            .bind(benefit_id)
            .fetch_one(&state.pool)
            .await?;

        return Ok(Json(ApiResponse::success(RedeemResponseDto {
            success: true,
            order_id,
            order_no,
            benefit_name,
            message: "幂等请求，返回已存在的订单".to_string(),
        })));
    }

    // 通过 RedemptionService 执行兑换
    if let Some(ref redemption_service) = state.redemption_service {
        let request = RedeemBadgeRequest::new(&req.user_id, req.rule_id, &idempotency_key);

        let response = redemption_service.redeem_badge(request).await?;

        Ok(Json(ApiResponse::success(RedeemResponseDto {
            success: response.success,
            order_id: response.order_id,
            order_no: response.order_no,
            benefit_name: response.benefit_name,
            message: response.message,
        })))
    } else {
        Err(AdminError::Internal(
            "RedemptionService 未配置".to_string(),
        ))
    }
}

/// 获取单个兑换订单详情
///
/// GET /api/admin/redemption/orders/:order_no
///
/// 通过订单号精确查询，同时关联 redemption_details 获取消耗的徽章信息
pub async fn get_redemption_order(
    State(state): State<AppState>,
    Path(order_no): Path<String>,
) -> Result<Json<ApiResponse<RedemptionOrderDto>>, AdminError> {
    let row = sqlx::query_as::<_, RedemptionOrderRow>(
        r#"
        SELECT
            o.id, o.order_no, o.user_id,
            o.redemption_rule_id as rule_id,
            r.name as rule_name,
            o.benefit_id,
            b.name as benefit_name,
            o.status::text as status,
            o.failure_reason,
            o.created_at
        FROM redemption_orders o
        JOIN badge_redemption_rules r ON r.id = o.redemption_rule_id
        JOIN benefits b ON b.id = o.benefit_id
        WHERE o.order_no = $1
        "#,
    )
    .bind(&order_no)
    .fetch_optional(&state.pool)
    .await?
    .ok_or_else(|| AdminError::NotFound(format!("兑换订单不存在: {}", order_no)))?;

    let consumed_badges = fetch_order_details(&state.pool, row.id).await?;

    let dto = RedemptionOrderDto {
        id: row.id,
        order_no: row.order_no,
        user_id: row.user_id,
        rule_id: row.rule_id,
        rule_name: row.rule_name,
        benefit_id: row.benefit_id,
        benefit_name: row.benefit_name,
        status: row.status,
        failure_reason: row.failure_reason,
        consumed_badges,
        created_at: row.created_at,
    };

    Ok(Json(ApiResponse::success(dto)))
}

/// 获取兑换订单列表
///
/// GET /api/admin/redemption/orders
pub async fn list_redemption_orders(
    State(state): State<AppState>,
    Query(pagination): Query<PaginationParams>,
    Query(filter): Query<OrderQueryFilter>,
) -> Result<Json<ApiResponse<PageResponse<RedemptionOrderDto>>>, AdminError> {
    let offset = pagination.offset();
    let limit = pagination.limit();

    // 查询总数
    let total: (i64,) = sqlx::query_as(
        r#"
        SELECT COUNT(*)
        FROM redemption_orders o
        WHERE ($1::text IS NULL OR o.user_id = $1)
          AND ($2::text IS NULL OR o.status = $2)
          AND ($3::bigint IS NULL OR o.redemption_rule_id = $3)
        "#,
    )
    .bind(&filter.user_id)
    .bind(&filter.status)
    .bind(filter.rule_id)
    .fetch_one(&state.pool)
    .await?;

    if total.0 == 0 {
        return Ok(Json(ApiResponse::success(PageResponse::empty(
            pagination.page,
            pagination.page_size,
        ))));
    }

    // 查询列表
    let rows = sqlx::query_as::<_, RedemptionOrderRow>(
        r#"
        SELECT
            o.id, o.order_no, o.user_id,
            o.redemption_rule_id as rule_id,
            r.name as rule_name,
            o.benefit_id,
            b.name as benefit_name,
            o.status::text as status,
            o.failure_reason,
            o.created_at
        FROM redemption_orders o
        JOIN badge_redemption_rules r ON r.id = o.redemption_rule_id
        JOIN benefits b ON b.id = o.benefit_id
        WHERE ($1::text IS NULL OR o.user_id = $1)
          AND ($2::text IS NULL OR o.status::text = $2)
          AND ($3::bigint IS NULL OR o.redemption_rule_id = $3)
        ORDER BY o.created_at DESC
        LIMIT $4 OFFSET $5
        "#,
    )
    .bind(&filter.user_id)
    .bind(&filter.status)
    .bind(filter.rule_id)
    .bind(limit)
    .bind(offset)
    .fetch_all(&state.pool)
    .await?;

    let mut items = Vec::with_capacity(rows.len());
    for row in rows {
        let consumed_badges = fetch_order_details(&state.pool, row.id).await?;
        items.push(RedemptionOrderDto {
            id: row.id,
            order_no: row.order_no,
            user_id: row.user_id,
            rule_id: row.rule_id,
            rule_name: row.rule_name,
            benefit_id: row.benefit_id,
            benefit_name: row.benefit_name,
            status: row.status,
            failure_reason: row.failure_reason,
            consumed_badges,
            created_at: row.created_at,
        });
    }

    let response = PageResponse::new(items, total.0, pagination.page, pagination.page_size);
    Ok(Json(ApiResponse::success(response)))
}

/// 获取用户兑换历史
///
/// GET /api/admin/users/:user_id/redemption-history
pub async fn get_user_redemption_history(
    State(state): State<AppState>,
    Path(user_id): Path<String>,
    Query(pagination): Query<PaginationParams>,
) -> Result<Json<ApiResponse<PageResponse<RedemptionOrderDto>>>, AdminError> {
    let offset = pagination.offset();
    let limit = pagination.limit();

    // 查询总数
    let total: (i64,) = sqlx::query_as(
        "SELECT COUNT(*) FROM redemption_orders WHERE user_id = $1",
    )
    .bind(&user_id)
    .fetch_one(&state.pool)
    .await?;

    if total.0 == 0 {
        return Ok(Json(ApiResponse::success(PageResponse::empty(
            pagination.page,
            pagination.page_size,
        ))));
    }

    // 查询列表
    let rows = sqlx::query_as::<_, RedemptionOrderRow>(
        r#"
        SELECT
            o.id, o.order_no, o.user_id,
            o.redemption_rule_id as rule_id,
            r.name as rule_name,
            o.benefit_id,
            b.name as benefit_name,
            o.status::text as status,
            o.failure_reason,
            o.created_at
        FROM redemption_orders o
        JOIN badge_redemption_rules r ON r.id = o.redemption_rule_id
        JOIN benefits b ON b.id = o.benefit_id
        WHERE o.user_id = $1
        ORDER BY o.created_at DESC
        LIMIT $2 OFFSET $3
        "#,
    )
    .bind(&user_id)
    .bind(limit)
    .bind(offset)
    .fetch_all(&state.pool)
    .await?;

    let mut items = Vec::with_capacity(rows.len());
    for row in rows {
        let consumed_badges = fetch_order_details(&state.pool, row.id).await?;
        items.push(RedemptionOrderDto {
            id: row.id,
            order_no: row.order_no,
            user_id: row.user_id,
            rule_id: row.rule_id,
            rule_name: row.rule_name,
            benefit_id: row.benefit_id,
            benefit_name: row.benefit_name,
            status: row.status,
            failure_reason: row.failure_reason,
            consumed_badges,
            created_at: row.created_at,
        });
    }

    let response = PageResponse::new(items, total.0, pagination.page, pagination.page_size);
    Ok(Json(ApiResponse::success(response)))
}

// ==================== 辅助函数 ====================

async fn fetch_rule_by_id(pool: &sqlx::PgPool, id: i64) -> Result<RedemptionRuleDto, AdminError> {
    let row = sqlx::query_as::<_, RedemptionRuleRow>(
        r#"
        SELECT
            r.id, r.name, r.description, r.benefit_id,
            b.name as benefit_name,
            r.required_badges, r.frequency_config,
            r.validity_type, r.relative_days,
            r.start_time, r.end_time, r.enabled,
            r.created_at, r.updated_at
        FROM badge_redemption_rules r
        JOIN benefits b ON b.id = r.benefit_id
        WHERE r.id = $1
        "#,
    )
    .bind(id)
    .fetch_optional(pool)
    .await?
    .ok_or(AdminError::RuleNotFound(id))?;

    row_to_dto(pool, row).await
}

async fn row_to_dto(
    pool: &sqlx::PgPool,
    row: RedemptionRuleRow,
) -> Result<RedemptionRuleDto, AdminError> {
    // 解析所需徽章并查询徽章名称
    let required_badges_raw: Vec<RequiredBadgeInput> =
        serde_json::from_value(row.required_badges).unwrap_or_default();

    let mut required_badges = Vec::with_capacity(required_badges_raw.len());
    for badge in required_badges_raw {
        let badge_name: Option<String> =
            sqlx::query_scalar("SELECT name FROM badges WHERE id = $1")
                .bind(badge.badge_id)
                .fetch_optional(pool)
                .await?;

        required_badges.push(RequiredBadgeDto {
            badge_id: badge.badge_id,
            badge_name: badge_name.unwrap_or_else(|| "未知徽章".to_string()),
            quantity: badge.quantity,
        });
    }

    let frequency_config: FrequencyConfigDto =
        serde_json::from_value(row.frequency_config).unwrap_or_default();

    // 解析有效期类型
    let validity_type = match row.validity_type.as_deref() {
        Some("RELATIVE") => ValidityType::Relative,
        _ => ValidityType::Fixed,
    };

    Ok(RedemptionRuleDto {
        id: row.id,
        name: row.name,
        description: row.description,
        benefit_id: row.benefit_id,
        benefit_name: row.benefit_name,
        required_badges,
        frequency_config,
        validity_type,
        relative_days: row.relative_days,
        start_time: row.start_time,
        end_time: row.end_time,
        enabled: row.enabled,
        created_at: row.created_at,
        updated_at: row.updated_at,
    })
}

async fn fetch_order_details(
    pool: &sqlx::PgPool,
    order_id: i64,
) -> Result<Vec<ConsumedBadgeDto>, AdminError> {
    let rows = sqlx::query_as::<_, RedemptionDetailRow>(
        r#"
        SELECT
            d.badge_id,
            b.name as badge_name,
            d.quantity
        FROM redemption_details d
        JOIN badges b ON b.id = d.badge_id
        WHERE d.order_id = $1
        "#,
    )
    .bind(order_id)
    .fetch_all(pool)
    .await?;

    Ok(rows
        .into_iter()
        .map(|r| ConsumedBadgeDto {
            badge_id: r.badge_id,
            badge_name: r.badge_name,
            quantity: r.quantity,
        })
        .collect())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_frequency_config_default() {
        let config = FrequencyConfigDto::default();
        assert!(config.max_per_user.is_none());
        assert!(config.max_per_day.is_none());
    }

    #[test]
    fn test_required_badge_input_validation() {
        let valid = RequiredBadgeInput {
            badge_id: 1,
            quantity: 1,
        };
        assert!(valid.validate().is_ok());

        let invalid = RequiredBadgeInput {
            badge_id: 1,
            quantity: 0,
        };
        assert!(invalid.validate().is_err());
    }
}
