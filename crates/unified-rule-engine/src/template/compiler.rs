//! 模板编译器
//!
//! 将规则模板与参数值结合，生成完整的规则 JSON。
//! 核心功能是替换模板中的 `${param}` 占位符，同时保留参数的原始类型。

use regex::Regex;
use serde_json::Value;
use std::collections::HashMap;

use super::models::{ParameterDef, ParameterType, RuleTemplate};

/// 模板编译错误
#[derive(Debug, thiserror::Error)]
pub enum CompileError {
    #[error("缺少必填参数: {0}")]
    MissingParameter(String),

    #[error("参数 {name} 超出范围: 期望 [{min:?}, {max:?}]")]
    ParamOutOfRange {
        name: String,
        min: Option<f64>,
        max: Option<f64>,
    },

    #[error("参数类型错误: {name} 期望 {expected}, 实际 {actual}")]
    TypeMismatch {
        name: String,
        expected: String,
        actual: String,
    },

    #[error("无效的模板: {0}")]
    InvalidTemplate(String),
}

/// 模板编译器
///
/// 负责将模板 JSON 中的占位符替换为实际参数值，支持嵌套结构和类型保留
pub struct TemplateCompiler {
    /// 匹配 ${paramName} 格式的占位符
    placeholder_regex: Regex,
}

impl TemplateCompiler {
    pub fn new() -> Self {
        Self {
            placeholder_regex: Regex::new(r#"\$\{([a-zA-Z_][a-zA-Z0-9_]*)\}"#).unwrap(),
        }
    }

    /// 从模板和参数编译出完整规则 JSON
    ///
    /// 编译过程：
    /// 1. 验证必填参数是否提供
    /// 2. 合并用户参数与默认值
    /// 3. 递归替换模板中的占位符
    pub fn compile(
        &self,
        template: &RuleTemplate,
        params: &HashMap<String, Value>,
    ) -> Result<Value, CompileError> {
        self.validate_params(&template.parameters, params)?;
        let merged_params = self.merge_with_defaults(&template.parameters, params);
        let compiled = self.replace_placeholders(&template.template_json, &merged_params)?;
        Ok(compiled)
    }

    /// 验证所有参数是否满足定义的约束
    fn validate_params(
        &self,
        definitions: &[ParameterDef],
        params: &HashMap<String, Value>,
    ) -> Result<(), CompileError> {
        for def in definitions {
            // 必填参数必须提供值或有默认值
            if def.required && !params.contains_key(&def.name) && def.default.is_none() {
                return Err(CompileError::MissingParameter(def.name.clone()));
            }

            if let Some(value) = params.get(&def.name) {
                self.validate_param_value(def, value)?;
            }
        }
        Ok(())
    }

    /// 验证单个参数值是否符合定义的类型和范围约束
    fn validate_param_value(&self, def: &ParameterDef, value: &Value) -> Result<(), CompileError> {
        let type_ok = match def.param_type {
            ParameterType::String => value.is_string(),
            ParameterType::Number => value.is_number(),
            ParameterType::Boolean => value.is_boolean(),
            ParameterType::Array => value.is_array(),
            ParameterType::Date => value.is_string(),
            ParameterType::Enum => {
                // 枚举类型需检查值是否在选项列表中
                if let Some(options) = &def.options {
                    options.iter().any(|o| &o.value == value)
                } else {
                    true
                }
            }
        };

        if !type_ok && def.param_type != ParameterType::Enum {
            return Err(CompileError::TypeMismatch {
                name: def.name.clone(),
                expected: format!("{:?}", def.param_type),
                actual: value_type_name(value),
            });
        }

        // 数值范围检查
        if let Some(v) = value.as_f64() {
            if let Some(min) = def.min
                && v < min
            {
                return Err(CompileError::ParamOutOfRange {
                    name: def.name.clone(),
                    min: Some(min),
                    max: def.max,
                });
            }
            if let Some(max) = def.max
                && v > max
            {
                return Err(CompileError::ParamOutOfRange {
                    name: def.name.clone(),
                    min: def.min,
                    max: Some(max),
                });
            }
        }

        Ok(())
    }

    /// 将用户提供的参数与模板默认值合并
    fn merge_with_defaults(
        &self,
        definitions: &[ParameterDef],
        params: &HashMap<String, Value>,
    ) -> HashMap<String, Value> {
        let mut merged = params.clone();
        for def in definitions {
            if !merged.contains_key(&def.name)
                && let Some(default) = &def.default
            {
                merged.insert(def.name.clone(), default.clone());
            }
        }
        merged
    }

    /// 递归替换模板中的占位符
    ///
    /// 对于纯占位符字符串（如 "${amount}"），直接返回参数值以保留原始类型；
    /// 对于包含占位符的混合字符串（如 "total: ${amount}"），进行字符串替换
    fn replace_placeholders(
        &self,
        template: &Value,
        params: &HashMap<String, Value>,
    ) -> Result<Value, CompileError> {
        match template {
            Value::String(s) => {
                // 检查是否是纯占位符，以便保留原始类型
                if let Some(caps) = self.placeholder_regex.captures(s)
                    && caps.get(0).map(|m| m.as_str()) == Some(s.as_str())
                {
                    let param_name = &caps[1];
                    return params.get(param_name).cloned().ok_or_else(|| {
                        CompileError::MissingParameter(param_name.to_string())
                    });
                }

                // 混合字符串中的占位符替换
                let result = self
                    .placeholder_regex
                    .replace_all(s, |caps: &regex::Captures| {
                        let param_name = &caps[1];
                        params
                            .get(param_name)
                            .map(|v| match v {
                                Value::String(s) => s.clone(),
                                _ => v.to_string(),
                            })
                            .unwrap_or_else(|| caps[0].to_string())
                    });
                Ok(Value::String(result.into_owned()))
            }
            Value::Array(arr) => {
                let compiled: Result<Vec<Value>, _> = arr
                    .iter()
                    .map(|v| self.replace_placeholders(v, params))
                    .collect();
                Ok(Value::Array(compiled?))
            }
            Value::Object(obj) => {
                let mut compiled = serde_json::Map::new();
                for (k, v) in obj {
                    compiled.insert(k.clone(), self.replace_placeholders(v, params)?);
                }
                Ok(Value::Object(compiled))
            }
            _ => Ok(template.clone()),
        }
    }
}

impl Default for TemplateCompiler {
    fn default() -> Self {
        Self::new()
    }
}

fn value_type_name(v: &Value) -> String {
    match v {
        Value::Null => "null".into(),
        Value::Bool(_) => "boolean".into(),
        Value::Number(_) => "number".into(),
        Value::String(_) => "string".into(),
        Value::Array(_) => "array".into(),
        Value::Object(_) => "object".into(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn create_test_template() -> RuleTemplate {
        RuleTemplate {
            id: 1,
            code: "test_amount".into(),
            name: "测试模板".into(),
            description: None,
            category: super::super::models::TemplateCategory::Basic,
            subcategory: None,
            template_json: json!({
                "root": {
                    "type": "condition",
                    "field": "order.amount",
                    "operator": "gte",
                    "value": "${amount}"
                }
            }),
            parameters: vec![ParameterDef {
                name: "amount".into(),
                param_type: ParameterType::Number,
                label: "金额".into(),
                description: None,
                default: Some(json!(100)),
                required: true,
                min: Some(0.0),
                max: Some(10000.0),
                options: None,
            }],
            version: "1.0".into(),
            is_system: false,
            enabled: true,
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
        }
    }

    #[test]
    fn test_compile_with_params() {
        let compiler = TemplateCompiler::new();
        let template = create_test_template();
        let params: HashMap<String, Value> = [("amount".to_string(), json!(500))].into();

        let result = compiler.compile(&template, &params).unwrap();

        assert_eq!(result["root"]["value"], json!(500));
    }

    #[test]
    fn test_compile_with_default() {
        let compiler = TemplateCompiler::new();
        let template = create_test_template();
        let params: HashMap<String, Value> = HashMap::new();

        let result = compiler.compile(&template, &params).unwrap();

        assert_eq!(result["root"]["value"], json!(100));
    }

    #[test]
    fn test_missing_required_param() {
        let compiler = TemplateCompiler::new();
        let mut template = create_test_template();
        template.parameters[0].default = None;
        let params: HashMap<String, Value> = HashMap::new();

        let result = compiler.compile(&template, &params);

        assert!(matches!(result, Err(CompileError::MissingParameter(_))));
    }

    #[test]
    fn test_param_out_of_range() {
        let compiler = TemplateCompiler::new();
        let template = create_test_template();
        let params: HashMap<String, Value> = [("amount".to_string(), json!(20000))].into();

        let result = compiler.compile(&template, &params);

        assert!(matches!(result, Err(CompileError::ParamOutOfRange { .. })));
    }

    #[test]
    fn test_nested_placeholders() {
        let compiler = TemplateCompiler::new();
        let mut template = create_test_template();
        template.template_json = json!({
            "root": {
                "type": "group",
                "operator": "AND",
                "children": [
                    {"type": "condition", "field": "event.type", "operator": "eq", "value": "${event_type}"},
                    {"type": "condition", "field": "order.amount", "operator": "gte", "value": "${amount}"}
                ]
            }
        });
        template.parameters.push(ParameterDef {
            name: "event_type".into(),
            param_type: ParameterType::String,
            label: "事件类型".into(),
            description: None,
            default: None,
            required: true,
            min: None,
            max: None,
            options: None,
        });

        let params: HashMap<String, Value> = [
            ("amount".to_string(), json!(500)),
            ("event_type".to_string(), json!("purchase")),
        ]
        .into();

        let result = compiler.compile(&template, &params).unwrap();

        assert_eq!(result["root"]["children"][0]["value"], json!("purchase"));
        assert_eq!(result["root"]["children"][1]["value"], json!(500));
    }

    #[test]
    fn test_type_mismatch() {
        let compiler = TemplateCompiler::new();
        let template = create_test_template();
        let params: HashMap<String, Value> = [("amount".to_string(), json!("not a number"))].into();

        let result = compiler.compile(&template, &params);

        assert!(matches!(result, Err(CompileError::TypeMismatch { .. })));
    }

    #[test]
    fn test_mixed_string_placeholder() {
        let compiler = TemplateCompiler::new();
        let mut template = create_test_template();
        template.template_json = json!({
            "message": "订单金额: ${amount} 元"
        });

        let params: HashMap<String, Value> = [("amount".to_string(), json!(500))].into();

        let result = compiler.compile(&template, &params).unwrap();

        assert_eq!(result["message"], json!("订单金额: 500 元"));
    }

    #[test]
    fn test_boolean_param() {
        let compiler = TemplateCompiler::new();
        let mut template = create_test_template();
        template.template_json = json!({
            "enabled": "${is_active}"
        });
        template.parameters = vec![ParameterDef {
            name: "is_active".into(),
            param_type: ParameterType::Boolean,
            label: "是否激活".into(),
            description: None,
            default: Some(json!(true)),
            required: false,
            min: None,
            max: None,
            options: None,
        }];

        let params: HashMap<String, Value> = [("is_active".to_string(), json!(false))].into();

        let result = compiler.compile(&template, &params).unwrap();

        assert_eq!(result["enabled"], json!(false));
    }

    #[test]
    fn test_array_param() {
        let compiler = TemplateCompiler::new();
        let mut template = create_test_template();
        template.template_json = json!({
            "tags": "${tag_list}"
        });
        template.parameters = vec![ParameterDef {
            name: "tag_list".into(),
            param_type: ParameterType::Array,
            label: "标签列表".into(),
            description: None,
            default: None,
            required: true,
            min: None,
            max: None,
            options: None,
        }];

        let params: HashMap<String, Value> =
            [("tag_list".to_string(), json!(["vip", "new_user"]))].into();

        let result = compiler.compile(&template, &params).unwrap();

        assert_eq!(result["tags"], json!(["vip", "new_user"]));
    }
}
