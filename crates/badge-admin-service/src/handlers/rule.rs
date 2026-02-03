//! 规则配置 API 处理器
//!
//! 实现徽章获取规则的 CRUD、发布和测试操作。
//! 规则与规则引擎配合，定义用户获取徽章的自动触发条件。

use axum::{
    Json,
    extract::{Path, Query, State},
};
use chrono::{DateTime, Utc};
use serde_json::Value;
use tracing::info;
use validator::Validate;

use crate::{
    dto::{
        ApiResponse, CreateRuleRequest, PageResponse, PaginationParams, RuleDto,
        TestRuleDefinitionRequest, UpdateRuleRequest,
    },
    error::AdminError,
    state::AppState,
};

/// 规则数据库查询结果（关联徽章名称）
#[derive(sqlx::FromRow)]
struct RuleFullRow {
    id: i64,
    badge_id: i64,
    badge_name: String,
    rule_json: Value,
    start_time: Option<DateTime<Utc>>,
    end_time: Option<DateTime<Utc>>,
    max_count_per_user: Option<i32>,
    enabled: bool,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

impl From<RuleFullRow> for RuleDto {
    fn from(row: RuleFullRow) -> Self {
        Self {
            id: row.id,
            badge_id: row.badge_id,
            badge_name: row.badge_name,
            rule_json: row.rule_json,
            start_time: row.start_time,
            end_time: row.end_time,
            max_count_per_user: row.max_count_per_user,
            enabled: row.enabled,
            created_at: row.created_at,
            updated_at: row.updated_at,
        }
    }
}

/// 规则完整信息的查询 SQL（复用于详情/更新后回查）
const RULE_FULL_SQL: &str = r#"
    SELECT
        r.id,
        r.badge_id,
        b.name as badge_name,
        r.rule_json,
        r.start_time,
        r.end_time,
        r.max_count_per_user,
        r.enabled,
        r.created_at,
        r.updated_at
    FROM badge_rules r
    JOIN badges b ON b.id = r.badge_id
"#;

/// 通过 ID 查询规则完整信息
async fn fetch_rule_by_id(pool: &sqlx::PgPool, id: i64) -> Result<RuleDto, AdminError> {
    let sql = format!("{} WHERE r.id = $1", RULE_FULL_SQL);

    let row = sqlx::query_as::<_, RuleFullRow>(&sql)
        .bind(id)
        .fetch_optional(pool)
        .await?
        .ok_or(AdminError::RuleNotFound(id))?;

    Ok(row.into())
}

/// 创建规则
///
/// POST /api/admin/rules
pub async fn create_rule(
    State(state): State<AppState>,
    Json(req): Json<CreateRuleRequest>,
) -> Result<Json<ApiResponse<RuleDto>>, AdminError> {
    req.validate()?;

    // 验证关联徽章存在
    let badge_exists: (bool,) = sqlx::query_as("SELECT EXISTS(SELECT 1 FROM badges WHERE id = $1)")
        .bind(req.badge_id)
        .fetch_one(&state.pool)
        .await?;

    if !badge_exists.0 {
        return Err(AdminError::BadgeNotFound(req.badge_id));
    }

    // 校验 rule_json 不能为空对象
    if req.rule_json.is_null() {
        return Err(AdminError::InvalidRuleJson("规则内容不能为空".to_string()));
    }

    // 验证 event_type 存在于 event_types 表中
    let event_type_exists: (bool,) =
        sqlx::query_as("SELECT EXISTS(SELECT 1 FROM event_types WHERE code = $1 AND enabled = true)")
            .bind(&req.event_type)
            .fetch_one(&state.pool)
            .await?;

    if !event_type_exists.0 {
        return Err(AdminError::Validation(format!(
            "事件类型 '{}' 不存在或已禁用",
            req.event_type
        )));
    }

    // 新建规则默认禁用，需要单独发布
    let row: (i64,) = sqlx::query_as(
        r#"
        INSERT INTO badge_rules (badge_id, rule_code, event_type, rule_json, start_time, end_time, max_count_per_user, global_quota, enabled)
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8, false)
        RETURNING id
        "#,
    )
    .bind(req.badge_id)
    .bind(&req.rule_code)
    .bind(&req.event_type)
    .bind(&req.rule_json)
    .bind(req.start_time)
    .bind(req.end_time)
    .bind(req.max_count_per_user)
    .bind(req.global_quota)
    .fetch_one(&state.pool)
    .await?;

    info!(rule_id = row.0, badge_id = req.badge_id, "Rule created");

    let dto = fetch_rule_by_id(&state.pool, row.0).await?;
    Ok(Json(ApiResponse::success(dto)))
}

/// 获取规则列表（分页）
///
/// GET /api/admin/rules
pub async fn list_rules(
    State(state): State<AppState>,
    Query(pagination): Query<PaginationParams>,
) -> Result<Json<ApiResponse<PageResponse<RuleDto>>>, AdminError> {
    let offset = pagination.offset();
    let limit = pagination.limit();

    let total: (i64,) =
        sqlx::query_as("SELECT COUNT(*) FROM badge_rules r JOIN badges b ON b.id = r.badge_id")
            .fetch_one(&state.pool)
            .await?;

    if total.0 == 0 {
        return Ok(Json(ApiResponse::success(PageResponse::empty(
            pagination.page,
            pagination.page_size,
        ))));
    }

    let sql = format!(
        "{} ORDER BY r.created_at DESC LIMIT $1 OFFSET $2",
        RULE_FULL_SQL
    );

    let rows = sqlx::query_as::<_, RuleFullRow>(&sql)
        .bind(limit)
        .bind(offset)
        .fetch_all(&state.pool)
        .await?;

    let items: Vec<RuleDto> = rows.into_iter().map(Into::into).collect();
    let response = PageResponse::new(items, total.0, pagination.page, pagination.page_size);
    Ok(Json(ApiResponse::success(response)))
}

/// 获取规则详情
///
/// GET /api/admin/rules/:id
pub async fn get_rule(
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> Result<Json<ApiResponse<RuleDto>>, AdminError> {
    let dto = fetch_rule_by_id(&state.pool, id).await?;
    Ok(Json(ApiResponse::success(dto)))
}

/// 更新规则
///
/// PUT /api/admin/rules/:id
pub async fn update_rule(
    State(state): State<AppState>,
    Path(id): Path<i64>,
    Json(req): Json<UpdateRuleRequest>,
) -> Result<Json<ApiResponse<RuleDto>>, AdminError> {
    req.validate()?;

    let exists: (bool,) = sqlx::query_as("SELECT EXISTS(SELECT 1 FROM badge_rules WHERE id = $1)")
        .bind(id)
        .fetch_one(&state.pool)
        .await?;

    if !exists.0 {
        return Err(AdminError::RuleNotFound(id));
    }

    // 校验 rule_json（如提供）
    if let Some(ref rule_json) = req.rule_json
        && rule_json.is_null()
    {
        return Err(AdminError::InvalidRuleJson("规则内容不能为空".to_string()));
    }

    sqlx::query(
        r#"
        UPDATE badge_rules
        SET
            rule_json = COALESCE($2, rule_json),
            start_time = COALESCE($3, start_time),
            end_time = COALESCE($4, end_time),
            max_count_per_user = COALESCE($5, max_count_per_user),
            enabled = COALESCE($6, enabled),
            updated_at = NOW()
        WHERE id = $1
        "#,
    )
    .bind(id)
    .bind(&req.rule_json)
    .bind(req.start_time)
    .bind(req.end_time)
    .bind(req.max_count_per_user)
    .bind(req.enabled)
    .execute(&state.pool)
    .await?;

    info!(rule_id = id, "Rule updated");

    let dto = fetch_rule_by_id(&state.pool, id).await?;
    Ok(Json(ApiResponse::success(dto)))
}

/// 删除规则
///
/// DELETE /api/admin/rules/:id
///
/// 仅允许删除禁用状态的规则，已启用的规则应先禁用再删除
pub async fn delete_rule(
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> Result<Json<ApiResponse<()>>, AdminError> {
    let rule: Option<(bool,)> = sqlx::query_as("SELECT enabled FROM badge_rules WHERE id = $1")
        .bind(id)
        .fetch_optional(&state.pool)
        .await?;

    let rule = rule.ok_or(AdminError::RuleNotFound(id))?;

    // 防止误删正在生效的规则
    if rule.0 {
        return Err(AdminError::Validation(
            "启用中的规则不能删除，请先禁用".to_string(),
        ));
    }

    sqlx::query("DELETE FROM badge_rules WHERE id = $1")
        .bind(id)
        .execute(&state.pool)
        .await?;

    info!(rule_id = id, "Rule deleted");

    Ok(Json(ApiResponse::<()>::success_empty()))
}

/// 发布（启用）规则
///
/// POST /api/admin/rules/:id/publish
///
/// 将规则状态从禁用切换为启用，启用后规则引擎会自动匹配事件
pub async fn publish_rule(
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> Result<Json<ApiResponse<RuleDto>>, AdminError> {
    let rule: Option<(bool,)> = sqlx::query_as("SELECT enabled FROM badge_rules WHERE id = $1")
        .bind(id)
        .fetch_optional(&state.pool)
        .await?;

    let rule = rule.ok_or(AdminError::RuleNotFound(id))?;

    if rule.0 {
        return Err(AdminError::Validation("规则已处于启用状态".to_string()));
    }

    sqlx::query("UPDATE badge_rules SET enabled = true, updated_at = NOW() WHERE id = $1")
        .bind(id)
        .execute(&state.pool)
        .await?;

    info!(rule_id = id, "Rule published (enabled)");

    let dto = fetch_rule_by_id(&state.pool, id).await?;
    Ok(Json(ApiResponse::success(dto)))
}

/// 测试规则
///
/// POST /api/admin/rules/:id/test
///
/// 用 mock 数据验证规则是否按预期匹配。
/// 后续会对接 rule-engine gRPC 服务进行真实评估。
pub async fn test_rule(
    State(state): State<AppState>,
    Path(id): Path<i64>,
    Json(_test_data): Json<serde_json::Value>,
) -> Result<Json<ApiResponse<serde_json::Value>>, AdminError> {
    // 确保规则存在
    let _rule = fetch_rule_by_id(&state.pool, id).await?;

    // TODO: 对接 rule-engine gRPC 服务进行真实规则评估
    let result = serde_json::json!({
        "matched": true,
        "matchedConditions": ["event.type == PURCHASE", "order.amount >= 500"],
        "evaluationTimeMs": 2
    });

    Ok(Json(ApiResponse::success(result)))
}

/// 禁用规则
///
/// POST /api/admin/rules/:id/disable
///
/// 将规则状态从启用切换为禁用，禁用后规则引擎不再匹配该规则
pub async fn disable_rule(
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> Result<Json<ApiResponse<RuleDto>>, AdminError> {
    let rule: Option<(bool,)> = sqlx::query_as("SELECT enabled FROM badge_rules WHERE id = $1")
        .bind(id)
        .fetch_optional(&state.pool)
        .await?;

    let rule = rule.ok_or(AdminError::RuleNotFound(id))?;

    if !rule.0 {
        return Err(AdminError::Validation("规则已处于禁用状态".to_string()));
    }

    sqlx::query("UPDATE badge_rules SET enabled = false, updated_at = NOW() WHERE id = $1")
        .bind(id)
        .execute(&state.pool)
        .await?;

    info!(rule_id = id, "Rule disabled");

    let dto = fetch_rule_by_id(&state.pool, id).await?;
    Ok(Json(ApiResponse::success(dto)))
}

/// 测试规则定义（不持久化）
///
/// POST /api/admin/rules/test
///
/// 接收规则 JSON 和可选上下文，返回模拟评估结果。
/// 用于在保存规则前预览其匹配行为。
/// 后续会对接 rule-engine gRPC 服务进行真实评估。
pub async fn test_rule_definition(
    State(_state): State<AppState>,
    Json(req): Json<TestRuleDefinitionRequest>,
) -> Result<Json<ApiResponse<Value>>, AdminError> {
    if req.rule_json.is_null() {
        return Err(AdminError::InvalidRuleJson("规则内容不能为空".to_string()));
    }

    // TODO: 对接 rule-engine gRPC 服务进行真实规则评估
    let result = serde_json::json!({
        "matched": true,
        "matchedConditions": ["mock condition 1", "mock condition 2"],
        "evaluationTimeMs": 1,
        "context": req.context,
    });

    Ok(Json(ApiResponse::success(result)))
}

#[cfg(test)]
mod tests {
    use super::*;
    use validator::Validate;

    #[test]
    fn test_create_rule_request_validation() {
        let valid = CreateRuleRequest {
            badge_id: 1,
            rule_code: "test_rule_001".to_string(),
            name: "测试规则".to_string(),
            event_type: "purchase".to_string(),
            rule_json: serde_json::json!({"type": "event", "conditions": []}),
            start_time: None,
            end_time: None,
            max_count_per_user: Some(5),
            global_quota: None,
        };
        assert!(valid.validate().is_ok());
    }

    #[test]
    fn test_update_rule_request_validation() {
        let valid = UpdateRuleRequest {
            rule_json: Some(serde_json::json!({"type": "event"})),
            start_time: None,
            end_time: None,
            max_count_per_user: None,
            enabled: Some(true),
        };
        assert!(valid.validate().is_ok());
    }

    #[test]
    fn test_rule_full_row_conversion() {
        let now = Utc::now();
        let row = RuleFullRow {
            id: 1,
            badge_id: 10,
            badge_name: "测试徽章".to_string(),
            rule_json: serde_json::json!({"type": "event"}),
            start_time: None,
            end_time: None,
            max_count_per_user: Some(3),
            enabled: false,
            created_at: now,
            updated_at: now,
        };

        let dto: RuleDto = row.into();
        assert_eq!(dto.id, 1);
        assert_eq!(dto.badge_id, 10);
        assert_eq!(dto.badge_name, "测试徽章");
        assert_eq!(dto.max_count_per_user, Some(3));
        assert!(!dto.enabled);
    }
}
