//! 规则配置 API 处理器
//!
//! 实现徽章获取规则的 CRUD、发布和测试操作。
//! 规则与规则引擎配合，定义用户获取徽章的自动触发条件。

use axum::{
    Json,
    extract::{Path, Query, State},
};
use badge_proto::rule_engine::{
    self, ConditionNode, GroupNode, Operator as ProtoOperator,
    LogicalOperator as ProtoLogicalOperator, Rule as ProtoRule, RuleNode as ProtoRuleNode,
    TestRuleRequest as ProtoTestRuleRequest,
};
use chrono::{DateTime, Utc};
use serde_json::Value;
use tracing::{info, warn};
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
    event_type: String,
    rule_code: String,
    name: Option<String>,
    description: Option<String>,
    rule_json: Value,
    start_time: Option<DateTime<Utc>>,
    end_time: Option<DateTime<Utc>>,
    max_count_per_user: Option<i32>,
    global_quota: Option<i32>,
    global_granted: i32,
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
            event_type: row.event_type,
            rule_code: row.rule_code,
            name: row.name,
            description: row.description,
            rule_json: row.rule_json,
            start_time: row.start_time,
            end_time: row.end_time,
            max_count_per_user: row.max_count_per_user,
            global_quota: row.global_quota,
            global_granted: row.global_granted,
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
        r.event_type,
        r.rule_code,
        r.name,
        r.description,
        r.rule_json,
        r.start_time,
        r.end_time,
        r.max_count_per_user,
        r.global_quota,
        COALESCE(r.global_granted, 0) as global_granted,
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
        INSERT INTO badge_rules (badge_id, rule_code, event_type, name, description, rule_json, start_time, end_time, max_count_per_user, global_quota, enabled)
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, false)
        RETURNING id
        "#,
    )
    .bind(req.badge_id)
    .bind(&req.rule_code)
    .bind(&req.event_type)
    .bind(&req.name)
    .bind(&req.description)
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
            event_type = COALESCE($2, event_type),
            rule_code = COALESCE($3, rule_code),
            name = COALESCE($4, name),
            description = COALESCE($5, description),
            global_quota = COALESCE($6, global_quota),
            rule_json = COALESCE($7, rule_json),
            start_time = COALESCE($8, start_time),
            end_time = COALESCE($9, end_time),
            max_count_per_user = COALESCE($10, max_count_per_user),
            enabled = COALESCE($11, enabled),
            updated_at = NOW()
        WHERE id = $1
        "#,
    )
    .bind(id)
    .bind(&req.event_type)
    .bind(&req.rule_code)
    .bind(&req.name)
    .bind(&req.description)
    .bind(req.global_quota)
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

// ─── JSON → Proto 转换工具 ───────────────────────────────────────────
//
// 数据库中 rule_json 使用 serde_json::Value 存储，格式与 unified-rule-engine
// 的 Rule 模型一致（参见 crates/unified-rule-engine/src/models.rs）。
// 以下函数将 JSON 格式转为 gRPC Proto 消息，避免在 handler 层引入
// rule_engine 内部模型的编译依赖。

/// 将 rule_json（数据库中的 JSON 规则定义）转为 gRPC Proto Rule 消息
fn json_to_proto_rule(
    rule_json: &Value,
    id: &str,
    name: &str,
) -> Result<ProtoRule, AdminError> {
    let root = json_to_proto_rule_node(rule_json)?;

    Ok(ProtoRule {
        id: id.to_string(),
        name: name.to_string(),
        version: "1.0".to_string(),
        root: Some(root),
        created_at: None,
        updated_at: None,
    })
}

/// 递归转换规则节点：JSON 中 `type: "condition"` 或 `type: "group"`
fn json_to_proto_rule_node(value: &Value) -> Result<ProtoRuleNode, AdminError> {
    let obj = value
        .as_object()
        .ok_or_else(|| AdminError::InvalidRuleJson("规则节点必须是 JSON 对象".to_string()))?;

    let node_type = obj
        .get("type")
        .and_then(|v| v.as_str())
        .ok_or_else(|| AdminError::InvalidRuleJson("规则节点缺少 type 字段".to_string()))?;

    match node_type {
        "condition" => {
            let field = obj
                .get("field")
                .and_then(|v| v.as_str())
                .unwrap_or_default()
                .to_string();

            let operator_str = obj
                .get("operator")
                .and_then(|v| v.as_str())
                .unwrap_or_default();

            let operator = str_to_proto_operator(operator_str)?;

            let proto_value = obj.get("value").map(json_value_to_proto_value);

            Ok(ProtoRuleNode {
                node: Some(rule_engine::rule_node::Node::Condition(ConditionNode {
                    field,
                    operator: operator.into(),
                    value: proto_value,
                })),
            })
        }
        "group" => {
            let operator_str = obj
                .get("operator")
                .and_then(|v| v.as_str())
                .unwrap_or_default();

            let logical_op = str_to_proto_logical_operator(operator_str)?;

            let children = obj
                .get("children")
                .and_then(|v| v.as_array())
                .ok_or_else(|| {
                    AdminError::InvalidRuleJson("group 节点缺少 children 数组".to_string())
                })?;

            let proto_children: Result<Vec<ProtoRuleNode>, AdminError> =
                children.iter().map(json_to_proto_rule_node).collect();

            Ok(ProtoRuleNode {
                node: Some(rule_engine::rule_node::Node::Group(GroupNode {
                    operator: logical_op.into(),
                    children: proto_children?,
                })),
            })
        }
        other => Err(AdminError::InvalidRuleJson(format!(
            "未知的规则节点类型: {}，期望 condition 或 group",
            other
        ))),
    }
}

/// 操作符字符串（JSON 中的 snake_case）映射到 Proto 枚举
fn str_to_proto_operator(s: &str) -> Result<ProtoOperator, AdminError> {
    match s {
        "eq" => Ok(ProtoOperator::Eq),
        "neq" => Ok(ProtoOperator::Neq),
        "gt" => Ok(ProtoOperator::Gt),
        "gte" => Ok(ProtoOperator::Gte),
        "lt" => Ok(ProtoOperator::Lt),
        "lte" => Ok(ProtoOperator::Lte),
        "between" => Ok(ProtoOperator::Between),
        "in" => Ok(ProtoOperator::In),
        "not_in" => Ok(ProtoOperator::NotIn),
        "contains" => Ok(ProtoOperator::Contains),
        "starts_with" => Ok(ProtoOperator::StartsWith),
        "ends_with" => Ok(ProtoOperator::EndsWith),
        "regex" => Ok(ProtoOperator::Regex),
        "is_empty" => Ok(ProtoOperator::IsEmpty),
        "is_not_empty" => Ok(ProtoOperator::IsNotEmpty),
        "contains_any" => Ok(ProtoOperator::ContainsAny),
        "contains_all" => Ok(ProtoOperator::ContainsAll),
        "before" => Ok(ProtoOperator::Before),
        "after" => Ok(ProtoOperator::After),
        other => Err(AdminError::InvalidRuleJson(format!(
            "未知的操作符: {}",
            other
        ))),
    }
}

/// 逻辑操作符字符串映射到 Proto 枚举
fn str_to_proto_logical_operator(s: &str) -> Result<ProtoLogicalOperator, AdminError> {
    // 兼容大写（Proto 惯例）和大写全称（JSON 存储格式）
    match s.to_uppercase().as_str() {
        "AND" => Ok(ProtoLogicalOperator::And),
        "OR" => Ok(ProtoLogicalOperator::Or),
        other => Err(AdminError::InvalidRuleJson(format!(
            "未知的逻辑操作符: {}，期望 AND 或 OR",
            other
        ))),
    }
}

/// serde_json::Value → prost_types::Value
fn json_value_to_proto_value(value: &Value) -> prost_types::Value {
    match value {
        Value::Null => prost_types::Value {
            kind: Some(prost_types::value::Kind::NullValue(0)),
        },
        Value::Bool(b) => prost_types::Value {
            kind: Some(prost_types::value::Kind::BoolValue(*b)),
        },
        Value::Number(n) => prost_types::Value {
            kind: Some(prost_types::value::Kind::NumberValue(
                n.as_f64().unwrap_or(0.0),
            )),
        },
        Value::String(s) => prost_types::Value {
            kind: Some(prost_types::value::Kind::StringValue(s.clone())),
        },
        Value::Array(arr) => {
            let values = arr.iter().map(json_value_to_proto_value).collect();
            prost_types::Value {
                kind: Some(prost_types::value::Kind::ListValue(
                    prost_types::ListValue { values },
                )),
            }
        }
        Value::Object(map) => {
            let fields = map
                .iter()
                .map(|(k, v)| (k.clone(), json_value_to_proto_value(v)))
                .collect();
            prost_types::Value {
                kind: Some(prost_types::value::Kind::StructValue(prost_types::Struct {
                    fields,
                })),
            }
        }
    }
}

/// serde_json::Value（必须是 Object）→ prost_types::Struct
fn json_value_to_proto_struct(value: &Value) -> Result<prost_types::Struct, AdminError> {
    let map = value
        .as_object()
        .ok_or_else(|| AdminError::Validation("上下文必须是 JSON 对象".to_string()))?;

    let fields = map
        .iter()
        .map(|(k, v)| (k.clone(), json_value_to_proto_value(v)))
        .collect();

    Ok(prost_types::Struct { fields })
}

/// 测试规则
///
/// POST /api/admin/rules/:id/test
///
/// 从数据库读取已有规则定义，通过 gRPC 调用规则引擎进行真实评估。
/// 调用方需提供 context 数据作为评估上下文。
pub async fn test_rule(
    State(state): State<AppState>,
    Path(id): Path<i64>,
    Json(test_data): Json<serde_json::Value>,
) -> Result<Json<ApiResponse<serde_json::Value>>, AdminError> {
    let rule = fetch_rule_by_id(&state.pool, id).await?;

    // 将数据库中的 rule_json 转为 Proto Rule，供 gRPC 调用
    let proto_rule = json_to_proto_rule(&rule.rule_json, &id.to_string(), &format!("rule-{}", id))?;

    // 从请求体提取上下文（可选），不存在则使用空上下文
    let context = test_data
        .get("context")
        .cloned()
        .or_else(|| {
            // 兼容直接传递上下文对象的情况（请求体本身就是上下文）
            if test_data.is_object() && !test_data.as_object().map_or(true, |m| m.is_empty()) {
                Some(test_data.clone())
            } else {
                None
            }
        });

    let proto_context = context
        .as_ref()
        .map(json_value_to_proto_struct)
        .transpose()?;

    let grpc_request = ProtoTestRuleRequest {
        rule: Some(proto_rule),
        context: proto_context,
    };

    // 通过 gRPC 调用规则引擎，若客户端不可用则返回明确错误
    let guard = state.rule_engine_client.read().await;
    let Some(client) = guard.as_ref() else {
        return Err(AdminError::Internal(
            "规则引擎服务不可用，请检查 RULE_ENGINE_GRPC_ADDR 配置".to_string(),
        ));
    };

    let mut client = client.clone();
    drop(guard);

    let response = client
        .test_rule(tonic::Request::new(grpc_request))
        .await
        .map_err(|e| {
            warn!(error = %e, rule_id = id, "规则引擎 gRPC 调用失败");
            AdminError::Internal(format!("规则引擎调用失败: {}", e.message()))
        })?;

    let resp = response.into_inner();
    let result = serde_json::json!({
        "matched": resp.matched,
        "matchedConditions": resp.matched_conditions,
        "evaluationTrace": resp.evaluation_trace,
        "evaluationTimeMs": resp.evaluation_time_ms
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
/// 接收规则 JSON 和可选上下文，通过 gRPC 调用规则引擎进行真实评估。
/// 用于在保存规则前预览其匹配行为，规则不会被持久化到数据库。
pub async fn test_rule_definition(
    State(state): State<AppState>,
    Json(req): Json<TestRuleDefinitionRequest>,
) -> Result<Json<ApiResponse<Value>>, AdminError> {
    if req.rule_json.is_null() {
        return Err(AdminError::InvalidRuleJson("规则内容不能为空".to_string()));
    }

    // 临时规则使用 UUID 标识，不与数据库中的规则关联
    let temp_id = uuid::Uuid::new_v4().to_string();
    let proto_rule = json_to_proto_rule(&req.rule_json, &temp_id, "test-rule")?;

    let proto_context = req
        .context
        .as_ref()
        .map(json_value_to_proto_struct)
        .transpose()?;

    let grpc_request = ProtoTestRuleRequest {
        rule: Some(proto_rule),
        context: proto_context,
    };

    let guard = state.rule_engine_client.read().await;
    let Some(client) = guard.as_ref() else {
        return Err(AdminError::Internal(
            "规则引擎服务不可用，请检查 RULE_ENGINE_GRPC_ADDR 配置".to_string(),
        ));
    };

    let mut client = client.clone();
    drop(guard);

    let response = client
        .test_rule(tonic::Request::new(grpc_request))
        .await
        .map_err(|e| {
            warn!(error = %e, "规则引擎 gRPC 调用失败（测试规则定义）");
            AdminError::Internal(format!("规则引擎调用失败: {}", e.message()))
        })?;

    let resp = response.into_inner();
    let result = serde_json::json!({
        "matched": resp.matched,
        "matchedConditions": resp.matched_conditions,
        "evaluationTrace": resp.evaluation_trace,
        "evaluationTimeMs": resp.evaluation_time_ms
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
            description: Some("这是一个测试规则".to_string()),
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
            event_type: None,
            rule_code: None,
            name: Some("更新后的规则名称".to_string()),
            description: Some("更新后的描述".to_string()),
            global_quota: None,
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
            event_type: "purchase".to_string(),
            rule_code: "test_rule_001".to_string(),
            name: Some("测试规则名称".to_string()),
            description: Some("测试规则描述".to_string()),
            rule_json: serde_json::json!({"type": "event"}),
            start_time: None,
            end_time: None,
            max_count_per_user: Some(3),
            global_quota: None,
            global_granted: 0,
            enabled: false,
            created_at: now,
            updated_at: now,
        };

        let dto: RuleDto = row.into();
        assert_eq!(dto.id, 1);
        assert_eq!(dto.badge_id, 10);
        assert_eq!(dto.badge_name, "测试徽章");
        assert_eq!(dto.name, Some("测试规则名称".to_string()));
        assert_eq!(dto.description, Some("测试规则描述".to_string()));
        assert_eq!(dto.max_count_per_user, Some(3));
        assert!(!dto.enabled);
    }
}
