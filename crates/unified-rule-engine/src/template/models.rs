//! 规则模板数据模型
//!
//! 定义规则模板和参数的核心结构体，支持模板的序列化/反序列化

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;

/// 模板参数定义
///
/// 描述规则模板中可配置的参数，包括类型约束、默认值和验证规则
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ParameterDef {
    /// 参数名称，用于在模板 JSON 中引用
    pub name: String,
    /// 参数类型，决定了前端渲染和后端验证逻辑
    #[serde(rename = "type")]
    pub param_type: ParameterType,
    /// 用户可见的参数标签
    pub label: String,
    /// 参数的详细描述
    #[serde(default)]
    pub description: Option<String>,
    /// 参数默认值，当用户未提供时使用
    #[serde(default)]
    pub default: Option<Value>,
    /// 是否为必填参数
    #[serde(default)]
    pub required: bool,
    /// 数值类型参数的最小值约束
    #[serde(default)]
    pub min: Option<f64>,
    /// 数值类型参数的最大值约束
    #[serde(default)]
    pub max: Option<f64>,
    /// 枚举类型参数的可选值列表
    #[serde(default)]
    pub options: Option<Vec<ParameterOption>>,
}

/// 参数类型枚举
///
/// 决定了参数值的验证规则和前端输入组件类型
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum ParameterType {
    /// 字符串类型
    String,
    /// 数值类型，支持整数和浮点数
    Number,
    /// 布尔类型
    Boolean,
    /// 日期时间类型
    Date,
    /// 数组类型，元素类型由上下文决定
    Array,
    /// 枚举类型，必须配合 options 字段使用
    Enum,
}

/// 枚举参数的选项定义
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParameterOption {
    /// 选项的实际值
    pub value: Value,
    /// 选项的显示标签
    pub label: String,
}

/// 模板分类
///
/// 用于组织和过滤模板，便于用户快速找到所需模板
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum TemplateCategory {
    /// 基础模板，适用于常见场景
    Basic,
    /// 高级模板，提供更复杂的规则逻辑
    Advanced,
    /// 行业模板，针对特定行业的预设规则
    Industry,
}

impl std::fmt::Display for TemplateCategory {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Basic => write!(f, "basic"),
            Self::Advanced => write!(f, "advanced"),
            Self::Industry => write!(f, "industry"),
        }
    }
}

/// 规则模板
///
/// 包含可参数化的规则定义，用户通过填充参数即可生成具体规则
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RuleTemplate {
    /// 模板唯一标识符
    pub id: i64,
    /// 模板编码，用于程序引用
    pub code: String,
    /// 模板名称
    pub name: String,
    /// 模板描述
    #[serde(default)]
    pub description: Option<String>,
    /// 模板分类
    pub category: TemplateCategory,
    /// 子分类，用于更细粒度的分类
    #[serde(default)]
    pub subcategory: Option<String>,
    /// 模板规则定义，包含参数占位符
    pub template_json: Value,
    /// 模板参数定义列表
    pub parameters: Vec<ParameterDef>,
    /// 模板版本号
    pub version: String,
    /// 是否为系统内置模板（内置模板不可删除）
    #[serde(default)]
    pub is_system: bool,
    /// 是否启用
    #[serde(default = "default_enabled")]
    pub enabled: bool,
    /// 创建时间
    #[serde(default = "Utc::now")]
    pub created_at: DateTime<Utc>,
    /// 更新时间
    #[serde(default = "Utc::now")]
    pub updated_at: DateTime<Utc>,
}

fn default_enabled() -> bool {
    true
}

impl RuleTemplate {
    /// 检查模板是否有效
    ///
    /// 模板需要同时满足：启用状态、编码非空、模板 JSON 非空
    pub fn is_valid(&self) -> bool {
        self.enabled && !self.code.is_empty() && !self.template_json.is_null()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_parameter_def_serialization() {
        let param = ParameterDef {
            name: "amount".into(),
            param_type: ParameterType::Number,
            label: "金额".into(),
            description: None,
            default: Some(json!(100)),
            required: true,
            min: Some(0.0),
            max: Some(10000.0),
            options: None,
        };

        let json = serde_json::to_string(&param).unwrap();
        assert!(json.contains("\"type\":\"number\""));
        assert!(json.contains("\"required\":true"));
    }

    #[test]
    fn test_parameter_def_deserialization() {
        let json_str = r#"{
            "name": "threshold",
            "type": "number",
            "label": "阈值",
            "required": true,
            "min": 0,
            "max": 100
        }"#;

        let param: ParameterDef = serde_json::from_str(json_str).unwrap();
        assert_eq!(param.name, "threshold");
        assert_eq!(param.param_type, ParameterType::Number);
        assert!(param.required);
        assert_eq!(param.min, Some(0.0));
        assert_eq!(param.max, Some(100.0));
    }

    #[test]
    fn test_template_category_display() {
        assert_eq!(TemplateCategory::Basic.to_string(), "basic");
        assert_eq!(TemplateCategory::Advanced.to_string(), "advanced");
        assert_eq!(TemplateCategory::Industry.to_string(), "industry");
    }

    #[test]
    fn test_template_category_serialization() {
        let category = TemplateCategory::Industry;
        let json = serde_json::to_string(&category).unwrap();
        assert_eq!(json, "\"industry\"");

        let deserialized: TemplateCategory = serde_json::from_str("\"basic\"").unwrap();
        assert_eq!(deserialized, TemplateCategory::Basic);
    }

    #[test]
    fn test_rule_template_is_valid() {
        let template = RuleTemplate {
            id: 1,
            code: "test_template".into(),
            name: "测试模板".into(),
            description: None,
            category: TemplateCategory::Basic,
            subcategory: None,
            template_json: json!({"condition": "test"}),
            parameters: vec![],
            version: "1.0.0".into(),
            is_system: false,
            enabled: true,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };

        assert!(template.is_valid());

        // 测试禁用模板
        let disabled_template = RuleTemplate {
            enabled: false,
            ..template.clone()
        };
        assert!(!disabled_template.is_valid());

        // 测试空编码
        let empty_code_template = RuleTemplate {
            code: "".into(),
            ..template.clone()
        };
        assert!(!empty_code_template.is_valid());

        // 测试空 JSON
        let null_json_template = RuleTemplate {
            template_json: Value::Null,
            ..template
        };
        assert!(!null_json_template.is_valid());
    }

    #[test]
    fn test_parameter_option_serialization() {
        let option = ParameterOption {
            value: json!("option1"),
            label: "选项一".into(),
        };

        let json = serde_json::to_string(&option).unwrap();
        assert!(json.contains("\"value\":\"option1\""));
        assert!(json.contains("\"label\":\"选项一\""));
    }

    #[test]
    fn test_parameter_with_enum_options() {
        let param = ParameterDef {
            name: "status".into(),
            param_type: ParameterType::Enum,
            label: "状态".into(),
            description: Some("选择状态".into()),
            default: Some(json!("active")),
            required: true,
            min: None,
            max: None,
            options: Some(vec![
                ParameterOption {
                    value: json!("active"),
                    label: "活跃".into(),
                },
                ParameterOption {
                    value: json!("inactive"),
                    label: "非活跃".into(),
                },
            ]),
        };

        let json = serde_json::to_string(&param).unwrap();
        assert!(json.contains("\"type\":\"enum\""));
        assert!(json.contains("\"options\""));
    }
}
