//! 规则模板管理 API
//!
//! 提供规则模板的查询、预览和从模板创建规则等功能。
//! 模板允许运营人员通过参数化配置快速创建规则，降低规则配置的复杂度。

use axum::{
    Json,
    extract::{Path, Query, State},
};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use tracing::info;

use crate::{dto::ApiResponse, error::AdminError, state::AppState};

/// 模板列表查询参数
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TemplateListParams {
    /// 按分类筛选（basic/advanced/industry）
    pub category: Option<String>,
    /// 是否只返回启用的模板，默认为 true
    pub enabled_only: Option<bool>,
}

/// 模板 DTO
///
/// 用于 API 响应的模板数据传输对象
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TemplateDto {
    pub id: i64,
    pub code: String,
    pub name: String,
    pub description: Option<String>,
    pub category: String,
    pub subcategory: Option<String>,
    pub template_json: Value,
    pub parameters: Value,
    pub version: String,
    pub is_system: bool,
    pub enabled: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// 模板数据库行
#[derive(sqlx::FromRow)]
struct TemplateRow {
    id: i64,
    code: String,
    name: String,
    description: Option<String>,
    category: String,
    subcategory: Option<String>,
    template_json: Value,
    parameters: Value,
    version: String,
    is_system: bool,
    enabled: bool,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

impl From<TemplateRow> for TemplateDto {
    fn from(row: TemplateRow) -> Self {
        Self {
            id: row.id,
            code: row.code,
            name: row.name,
            description: row.description,
            category: row.category,
            subcategory: row.subcategory,
            template_json: row.template_json,
            parameters: row.parameters,
            version: row.version,
            is_system: row.is_system,
            enabled: row.enabled,
            created_at: row.created_at,
            updated_at: row.updated_at,
        }
    }
}

/// 模板列表响应
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TemplateListResponse {
    pub items: Vec<TemplateDto>,
    pub total: usize,
}

/// 预览请求
#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PreviewRequest {
    /// 模板参数值映射
    pub params: HashMap<String, Value>,
}

/// 预览响应
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PreviewResponse {
    /// 编译后的完整规则 JSON
    pub rule_json: Value,
}

/// 从模板创建规则请求
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateRuleFromTemplateRequest {
    /// 模板代码
    pub template_code: String,
    /// 关联的徽章 ID
    pub badge_id: i64,
    /// 模板参数值
    pub params: HashMap<String, Value>,
    /// 是否启用规则，默认为 true
    pub enabled: Option<bool>,
}

/// 从模板创建规则响应
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateRuleFromTemplateResponse {
    pub id: i64,
    pub badge_id: i64,
    pub template_code: String,
    pub template_version: String,
    pub rule_json: Value,
    pub enabled: bool,
}

/// 获取模板列表
///
/// GET /api/admin/templates
///
/// 支持按分类筛选和启用状态筛选。返回的模板按分类和名称排序。
pub async fn list_templates(
    State(state): State<AppState>,
    Query(params): Query<TemplateListParams>,
) -> Result<Json<ApiResponse<TemplateListResponse>>, AdminError> {
    let enabled_only = params.enabled_only.unwrap_or(true);

    // 构建查询，使用参数化查询防止 SQL 注入
    let rows = match (&params.category, enabled_only) {
        (Some(category), true) => {
            sqlx::query_as::<_, TemplateRow>(
                r#"SELECT id, code, name, description, category, subcategory,
                          template_json, parameters, version, is_system, enabled,
                          created_at, updated_at
                   FROM rule_templates
                   WHERE enabled = TRUE AND category = $1
                   ORDER BY category, subcategory, name"#,
            )
            .bind(category)
            .fetch_all(&state.pool)
            .await?
        }
        (Some(category), false) => {
            sqlx::query_as::<_, TemplateRow>(
                r#"SELECT id, code, name, description, category, subcategory,
                          template_json, parameters, version, is_system, enabled,
                          created_at, updated_at
                   FROM rule_templates
                   WHERE category = $1
                   ORDER BY category, subcategory, name"#,
            )
            .bind(category)
            .fetch_all(&state.pool)
            .await?
        }
        (None, true) => {
            sqlx::query_as::<_, TemplateRow>(
                r#"SELECT id, code, name, description, category, subcategory,
                          template_json, parameters, version, is_system, enabled,
                          created_at, updated_at
                   FROM rule_templates
                   WHERE enabled = TRUE
                   ORDER BY category, subcategory, name"#,
            )
            .fetch_all(&state.pool)
            .await?
        }
        (None, false) => {
            sqlx::query_as::<_, TemplateRow>(
                r#"SELECT id, code, name, description, category, subcategory,
                          template_json, parameters, version, is_system, enabled,
                          created_at, updated_at
                   FROM rule_templates
                   ORDER BY category, subcategory, name"#,
            )
            .fetch_all(&state.pool)
            .await?
        }
    };

    let items: Vec<TemplateDto> = rows.into_iter().map(Into::into).collect();
    let total = items.len();

    Ok(Json(ApiResponse::success(TemplateListResponse {
        items,
        total,
    })))
}

/// 获取模板详情
///
/// GET /api/admin/templates/:code
///
/// 通过模板代码获取单个模板的完整信息
pub async fn get_template(
    State(state): State<AppState>,
    Path(code): Path<String>,
) -> Result<Json<ApiResponse<TemplateDto>>, AdminError> {
    let row = sqlx::query_as::<_, TemplateRow>(
        r#"SELECT id, code, name, description, category, subcategory,
                  template_json, parameters, version, is_system, enabled,
                  created_at, updated_at
           FROM rule_templates WHERE code = $1"#,
    )
    .bind(&code)
    .fetch_optional(&state.pool)
    .await?
    .ok_or_else(|| AdminError::Validation(format!("模板 {} 不存在", code)))?;

    Ok(Json(ApiResponse::success(row.into())))
}

/// 预览模板编译结果
///
/// POST /api/admin/templates/:code/preview
///
/// 使用给定的参数预览模板编译后的规则 JSON，用于运营人员在创建规则前验证配置
pub async fn preview_template(
    State(state): State<AppState>,
    Path(code): Path<String>,
    Json(req): Json<PreviewRequest>,
) -> Result<Json<ApiResponse<PreviewResponse>>, AdminError> {
    use rule_engine::template::{ParameterDef, RuleTemplate, TemplateCategory, TemplateCompiler};

    // 获取模板
    let row = sqlx::query_as::<_, TemplateRow>(
        r#"SELECT id, code, name, description, category, subcategory,
                  template_json, parameters, version, is_system, enabled,
                  created_at, updated_at
           FROM rule_templates WHERE code = $1"#,
    )
    .bind(&code)
    .fetch_optional(&state.pool)
    .await?
    .ok_or_else(|| AdminError::Validation(format!("模板 {} 不存在", code)))?;

    // 将数据库行转换为规则引擎的模板结构
    let category = match row.category.as_str() {
        "basic" => TemplateCategory::Basic,
        "advanced" => TemplateCategory::Advanced,
        "industry" => TemplateCategory::Industry,
        _ => TemplateCategory::Basic,
    };

    let parameters: Vec<ParameterDef> = serde_json::from_value(row.parameters).unwrap_or_default();

    let template = RuleTemplate {
        id: row.id,
        code: row.code,
        name: row.name,
        description: row.description,
        category,
        subcategory: row.subcategory,
        template_json: row.template_json,
        parameters,
        version: row.version,
        is_system: row.is_system,
        enabled: row.enabled,
        created_at: row.created_at,
        updated_at: row.updated_at,
    };

    // 编译模板
    let compiler = TemplateCompiler::new();
    let rule_json = compiler
        .compile(&template, &req.params)
        .map_err(|e| AdminError::Validation(e.to_string()))?;

    Ok(Json(ApiResponse::success(PreviewResponse { rule_json })))
}

/// 从模板创建规则
///
/// POST /api/admin/rules/from-template
///
/// 通过模板和参数创建新的徽章规则。创建的规则会记录模板来源和版本，便于后续追踪模板更新。
pub async fn create_rule_from_template(
    State(state): State<AppState>,
    Json(req): Json<CreateRuleFromTemplateRequest>,
) -> Result<Json<ApiResponse<CreateRuleFromTemplateResponse>>, AdminError> {
    use rule_engine::template::{ParameterDef, RuleTemplate, TemplateCategory, TemplateCompiler};

    // 1. 验证徽章存在
    let badge_exists: (bool,) = sqlx::query_as("SELECT EXISTS(SELECT 1 FROM badges WHERE id = $1)")
        .bind(req.badge_id)
        .fetch_one(&state.pool)
        .await?;

    if !badge_exists.0 {
        return Err(AdminError::BadgeNotFound(req.badge_id));
    }

    // 2. 获取模板
    let row = sqlx::query_as::<_, TemplateRow>(
        r#"SELECT id, code, name, description, category, subcategory,
                  template_json, parameters, version, is_system, enabled,
                  created_at, updated_at
           FROM rule_templates WHERE code = $1 AND enabled = TRUE"#,
    )
    .bind(&req.template_code)
    .fetch_optional(&state.pool)
    .await?
    .ok_or_else(|| AdminError::Validation(format!("模板 {} 不存在或未启用", req.template_code)))?;

    let template_id = row.id;
    let version = row.version.clone();
    let template_code = row.code.clone();

    // 转换为规则引擎的模板结构
    let category = match row.category.as_str() {
        "basic" => TemplateCategory::Basic,
        "advanced" => TemplateCategory::Advanced,
        "industry" => TemplateCategory::Industry,
        _ => TemplateCategory::Basic,
    };

    let parameters: Vec<ParameterDef> = serde_json::from_value(row.parameters).unwrap_or_default();

    let template = RuleTemplate {
        id: row.id,
        code: row.code,
        name: row.name,
        description: row.description,
        category,
        subcategory: row.subcategory,
        template_json: row.template_json,
        parameters,
        version: row.version,
        is_system: row.is_system,
        enabled: row.enabled,
        created_at: row.created_at,
        updated_at: row.updated_at,
    };

    // 3. 编译规则
    let compiler = TemplateCompiler::new();
    let rule_json = compiler
        .compile(&template, &req.params)
        .map_err(|e| AdminError::Validation(e.to_string()))?;

    // 4. 创建规则记录
    let enabled = req.enabled.unwrap_or(true);
    let params_json = serde_json::to_value(&req.params).unwrap_or_default();

    let rule_row: (i64,) = sqlx::query_as(
        r#"INSERT INTO badge_rules
           (badge_id, rule_json, template_id, template_version, template_params, enabled)
           VALUES ($1, $2, $3, $4, $5, $6)
           RETURNING id"#,
    )
    .bind(req.badge_id)
    .bind(&rule_json)
    .bind(template_id)
    .bind(&version)
    .bind(&params_json)
    .bind(enabled)
    .fetch_one(&state.pool)
    .await?;

    info!(
        rule_id = rule_row.0,
        badge_id = req.badge_id,
        template_code = %template_code,
        template_version = %version,
        "Rule created from template"
    );

    Ok(Json(ApiResponse::success(CreateRuleFromTemplateResponse {
        id: rule_row.0,
        badge_id: req.badge_id,
        template_code,
        template_version: version,
        rule_json,
        enabled,
    })))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_template_list_params_default() {
        let json_str = "{}";
        let params: TemplateListParams = serde_json::from_str(json_str).unwrap();
        assert!(params.category.is_none());
        assert!(params.enabled_only.is_none());
    }

    #[test]
    fn test_template_list_params_with_values() {
        let json_str = r#"{"category": "basic", "enabledOnly": true}"#;
        let params: TemplateListParams = serde_json::from_str(json_str).unwrap();
        assert_eq!(params.category, Some("basic".to_string()));
        assert_eq!(params.enabled_only, Some(true));
    }

    #[test]
    fn test_preview_request_serialization() {
        let mut params = HashMap::new();
        params.insert("amount".to_string(), serde_json::json!(500));
        params.insert("days".to_string(), serde_json::json!(30));

        let req = PreviewRequest { params };
        let json = serde_json::to_string(&req).unwrap();
        assert!(json.contains("amount"));
        assert!(json.contains("500"));
    }

    #[test]
    fn test_create_rule_from_template_request() {
        let json_str = r#"{
            "templateCode": "purchase_gte",
            "badgeId": 123,
            "params": {"amount": 1000},
            "enabled": false
        }"#;

        let req: CreateRuleFromTemplateRequest = serde_json::from_str(json_str).unwrap();
        assert_eq!(req.template_code, "purchase_gte");
        assert_eq!(req.badge_id, 123);
        assert_eq!(req.enabled, Some(false));
        assert_eq!(req.params.get("amount"), Some(&serde_json::json!(1000)));
    }

    #[test]
    fn test_template_dto_serialization() {
        let now = Utc::now();
        let dto = TemplateDto {
            id: 1,
            code: "test".to_string(),
            name: "测试模板".to_string(),
            description: Some("描述".to_string()),
            category: "basic".to_string(),
            subcategory: None,
            template_json: serde_json::json!({"type": "condition"}),
            parameters: serde_json::json!([]),
            version: "1.0".to_string(),
            is_system: false,
            enabled: true,
            created_at: now,
            updated_at: now,
        };

        let json = serde_json::to_string(&dto).unwrap();
        // 验证使用了 camelCase
        assert!(json.contains("templateJson"));
        assert!(json.contains("isSystem"));
        assert!(json.contains("createdAt"));
    }
}
