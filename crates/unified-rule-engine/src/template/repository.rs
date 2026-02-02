//! 模板仓储层
//!
//! 提供规则模板的数据库访问功能，支持模板的增删改查操作。
//! 系统内置模板（is_system=true）受保护，不允许删除和更新。

use sqlx::{PgPool, Row};

use super::models::{ParameterDef, RuleTemplate, TemplateCategory};

/// 模板仓储
///
/// 封装所有与规则模板相关的数据库操作，提供类型安全的 API
pub struct TemplateRepository {
    pool: PgPool,
}

impl TemplateRepository {
    /// 创建新的模板仓储实例
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// 获取所有模板，可按分类过滤
    ///
    /// # Arguments
    /// * `category` - 可选的模板分类过滤条件
    /// * `enabled_only` - 是否只返回已启用的模板
    ///
    /// # Returns
    /// 按分类、子分类、名称排序的模板列表
    pub async fn list(
        &self,
        category: Option<&TemplateCategory>,
        enabled_only: bool,
    ) -> Result<Vec<RuleTemplate>, sqlx::Error> {
        // 动态构建 SQL 查询
        // 由于 sqlx 运行时查询的限制，使用字符串拼接方式构建条件
        let mut sql = String::from(
            r#"SELECT id, code, name, description, category, subcategory,
                      template_json, parameters, version, is_system, enabled,
                      created_at, updated_at
               FROM rule_templates
               WHERE 1=1"#,
        );

        if enabled_only {
            sql.push_str(" AND enabled = TRUE");
        }

        sql.push_str(" ORDER BY category, subcategory, name");

        let rows = sqlx::query(&sql).fetch_all(&self.pool).await?;

        // 在应用层进行分类过滤
        // 这样可以避免复杂的动态参数绑定，同时保持类型安全
        let templates: Vec<RuleTemplate> = rows
            .iter()
            .filter_map(|row| self.map_row(row).ok())
            .filter(|t| category.is_none() || Some(&t.category) == category)
            .collect();

        Ok(templates)
    }

    /// 根据代码获取模板
    ///
    /// 模板代码是业务层面的唯一标识符，用于程序引用
    pub async fn get_by_code(&self, code: &str) -> Result<Option<RuleTemplate>, sqlx::Error> {
        let row = sqlx::query(
            r#"SELECT id, code, name, description, category, subcategory,
                      template_json, parameters, version, is_system, enabled,
                      created_at, updated_at
               FROM rule_templates
               WHERE code = $1"#,
        )
        .bind(code)
        .fetch_optional(&self.pool)
        .await?;

        Ok(row.and_then(|r| self.map_row(&r).ok()))
    }

    /// 根据 ID 获取模板
    pub async fn get_by_id(&self, id: i64) -> Result<Option<RuleTemplate>, sqlx::Error> {
        let row = sqlx::query(
            r#"SELECT id, code, name, description, category, subcategory,
                      template_json, parameters, version, is_system, enabled,
                      created_at, updated_at
               FROM rule_templates
               WHERE id = $1"#,
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await?;

        Ok(row.and_then(|r| self.map_row(&r).ok()))
    }

    /// 创建模板
    ///
    /// # Returns
    /// 新创建模板的 ID
    pub async fn create(&self, template: &RuleTemplate) -> Result<i64, sqlx::Error> {
        let row = sqlx::query(
            r#"INSERT INTO rule_templates
               (code, name, description, category, subcategory,
                template_json, parameters, version, is_system, enabled)
               VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)
               RETURNING id"#,
        )
        .bind(&template.code)
        .bind(&template.name)
        .bind(&template.description)
        .bind(template.category.to_string())
        .bind(&template.subcategory)
        .bind(&template.template_json)
        .bind(serde_json::to_value(&template.parameters).unwrap_or_default())
        .bind(&template.version)
        .bind(template.is_system)
        .bind(template.enabled)
        .fetch_one(&self.pool)
        .await?;

        Ok(row.get("id"))
    }

    /// 更新模板
    ///
    /// 系统内置模板（is_system=true）不允许更新，以保护核心业务逻辑
    ///
    /// # Returns
    /// 是否成功更新（false 表示模板不存在或为系统模板）
    pub async fn update(&self, id: i64, template: &RuleTemplate) -> Result<bool, sqlx::Error> {
        let result = sqlx::query(
            r#"UPDATE rule_templates
               SET name = $2, description = $3, category = $4, subcategory = $5,
                   template_json = $6, parameters = $7, version = $8, enabled = $9,
                   updated_at = NOW()
               WHERE id = $1 AND is_system = FALSE"#,
        )
        .bind(id)
        .bind(&template.name)
        .bind(&template.description)
        .bind(template.category.to_string())
        .bind(&template.subcategory)
        .bind(&template.template_json)
        .bind(serde_json::to_value(&template.parameters).unwrap_or_default())
        .bind(&template.version)
        .bind(template.enabled)
        .execute(&self.pool)
        .await?;

        Ok(result.rows_affected() > 0)
    }

    /// 删除模板
    ///
    /// 系统内置模板（is_system=true）不允许删除
    ///
    /// # Returns
    /// 是否成功删除（false 表示模板不存在或为系统模板）
    pub async fn delete(&self, id: i64) -> Result<bool, sqlx::Error> {
        let result = sqlx::query("DELETE FROM rule_templates WHERE id = $1 AND is_system = FALSE")
            .bind(id)
            .execute(&self.pool)
            .await?;

        Ok(result.rows_affected() > 0)
    }

    /// 启用/禁用模板
    ///
    /// 此操作对系统模板也生效，允许临时禁用系统模板
    ///
    /// # Returns
    /// 是否成功更新
    pub async fn set_enabled(&self, id: i64, enabled: bool) -> Result<bool, sqlx::Error> {
        let result =
            sqlx::query("UPDATE rule_templates SET enabled = $2, updated_at = NOW() WHERE id = $1")
                .bind(id)
                .bind(enabled)
                .execute(&self.pool)
                .await?;

        Ok(result.rows_affected() > 0)
    }

    /// 检查模板代码是否已存在
    ///
    /// 用于创建模板前的唯一性校验
    pub async fn exists_by_code(&self, code: &str) -> Result<bool, sqlx::Error> {
        let row =
            sqlx::query("SELECT EXISTS(SELECT 1 FROM rule_templates WHERE code = $1) AS exists")
                .bind(code)
                .fetch_one(&self.pool)
                .await?;

        Ok(row.get("exists"))
    }

    /// 批量获取模板
    ///
    /// 根据多个模板代码一次性获取，减少数据库往返
    pub async fn get_by_codes(&self, codes: &[String]) -> Result<Vec<RuleTemplate>, sqlx::Error> {
        if codes.is_empty() {
            return Ok(Vec::new());
        }

        let rows = sqlx::query(
            r#"SELECT id, code, name, description, category, subcategory,
                      template_json, parameters, version, is_system, enabled,
                      created_at, updated_at
               FROM rule_templates
               WHERE code = ANY($1)"#,
        )
        .bind(codes)
        .fetch_all(&self.pool)
        .await?;

        let templates: Vec<RuleTemplate> = rows
            .iter()
            .filter_map(|row| self.map_row(row).ok())
            .collect();

        Ok(templates)
    }

    /// 从数据库行映射到 RuleTemplate 结构体
    ///
    /// 处理枚举类型和 JSON 字段的反序列化
    fn map_row(&self, row: &sqlx::postgres::PgRow) -> Result<RuleTemplate, sqlx::Error> {
        let category_str: String = row.get("category");
        let category = match category_str.as_str() {
            "basic" => TemplateCategory::Basic,
            "advanced" => TemplateCategory::Advanced,
            "industry" => TemplateCategory::Industry,
            // 未知分类默认为 Basic，保证向后兼容
            _ => TemplateCategory::Basic,
        };

        let parameters_json: serde_json::Value = row.get("parameters");
        let parameters: Vec<ParameterDef> =
            serde_json::from_value(parameters_json).unwrap_or_default();

        Ok(RuleTemplate {
            id: row.get("id"),
            code: row.get("code"),
            name: row.get("name"),
            description: row.get("description"),
            category,
            subcategory: row.get("subcategory"),
            template_json: row.get("template_json"),
            parameters,
            version: row.get("version"),
            is_system: row.get("is_system"),
            enabled: row.get("enabled"),
            created_at: row.get("created_at"),
            updated_at: row.get("updated_at"),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;
    use serde_json::json;

    /// 测试辅助函数：创建测试用的 RuleTemplate
    fn create_test_template(code: &str, name: &str) -> RuleTemplate {
        RuleTemplate {
            id: 0,
            code: code.to_string(),
            name: name.to_string(),
            description: Some("测试模板".to_string()),
            category: TemplateCategory::Basic,
            subcategory: None,
            template_json: json!({"condition": "test"}),
            parameters: vec![],
            version: "1.0.0".to_string(),
            is_system: false,
            enabled: true,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        }
    }

    #[test]
    fn test_create_test_template() {
        let template = create_test_template("test_code", "测试名称");
        assert_eq!(template.code, "test_code");
        assert_eq!(template.name, "测试名称");
        assert!(template.enabled);
        assert!(!template.is_system);
    }
}
