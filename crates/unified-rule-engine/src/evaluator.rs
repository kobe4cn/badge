//! 条件评估器
//!
//! 实现各种操作符的评估逻辑，支持多种数据类型的比较。

use crate::error::{Result, RuleError};
use crate::operators::Operator;
use chrono::{DateTime, NaiveDate, Utc};
use regex::Regex;
use serde_json::Value;

/// 条件评估器
pub struct ConditionEvaluator;

impl ConditionEvaluator {
    /// 评估条件
    ///
    /// # Arguments
    /// * `field_value` - 从上下文中获取的字段值
    /// * `operator` - 操作符
    /// * `expected_value` - 规则中定义的期望值
    pub fn evaluate(
        field_value: Option<&Value>,
        operator: Operator,
        expected_value: &Value,
    ) -> Result<bool> {
        // 空值检查操作符需要特殊处理，因为它们的语义就是检查值是否存在
        match operator {
            Operator::IsEmpty => return Ok(Self::is_empty(field_value)),
            Operator::IsNotEmpty => return Ok(!Self::is_empty(field_value)),
            _ => {}
        }

        // 字段不存在时，大多数操作返回 false
        let field_value = match field_value {
            Some(v) => v,
            None => return Ok(false),
        };

        match operator {
            Operator::Eq => Self::eq(field_value, expected_value),
            Operator::Neq => Self::eq(field_value, expected_value).map(|r| !r),
            Operator::Gt => Self::compare(field_value, expected_value, |a, b| a > b),
            Operator::Gte => Self::compare(field_value, expected_value, |a, b| a >= b),
            Operator::Lt => Self::compare(field_value, expected_value, |a, b| a < b),
            Operator::Lte => Self::compare(field_value, expected_value, |a, b| a <= b),
            Operator::Between => Self::between(field_value, expected_value),
            Operator::In => Self::in_list(field_value, expected_value),
            Operator::NotIn => Self::in_list(field_value, expected_value).map(|r| !r),
            Operator::Contains => Self::contains(field_value, expected_value),
            Operator::ContainsAny => Self::contains_any(field_value, expected_value),
            Operator::ContainsAll => Self::contains_all(field_value, expected_value),
            Operator::StartsWith => Self::starts_with(field_value, expected_value),
            Operator::EndsWith => Self::ends_with(field_value, expected_value),
            Operator::Regex => Self::regex_match(field_value, expected_value),
            Operator::Before => Self::time_compare(field_value, expected_value, |a, b| a < b),
            Operator::After => Self::time_compare(field_value, expected_value, |a, b| a > b),
            Operator::IsEmpty | Operator::IsNotEmpty => unreachable!(),
        }
    }

    /// 判断值是否为空
    fn is_empty(value: Option<&Value>) -> bool {
        match value {
            None => true,
            Some(Value::Null) => true,
            Some(Value::String(s)) => s.is_empty(),
            Some(Value::Array(arr)) => arr.is_empty(),
            Some(Value::Object(obj)) => obj.is_empty(),
            _ => false,
        }
    }

    /// 相等比较
    fn eq(field: &Value, expected: &Value) -> Result<bool> {
        // 数值比较需要统一转为浮点数，避免整数和浮点数比较失败（如 100 == 100.0）
        if let (Some(f1), Some(f2)) = (Self::as_f64(field), Self::as_f64(expected)) {
            return Ok((f1 - f2).abs() < f64::EPSILON);
        }

        // 其他类型直接比较
        Ok(field == expected)
    }

    /// 数值比较
    fn compare<F>(field: &Value, expected: &Value, cmp: F) -> Result<bool>
    where
        F: Fn(f64, f64) -> bool,
    {
        let field_num = Self::as_f64(field).ok_or_else(|| RuleError::TypeMismatch {
            expected: "number".to_string(),
            actual: Self::type_name(field).to_string(),
        })?;

        let expected_num = Self::as_f64(expected).ok_or_else(|| RuleError::TypeMismatch {
            expected: "number".to_string(),
            actual: Self::type_name(expected).to_string(),
        })?;

        Ok(cmp(field_num, expected_num))
    }

    /// 范围比较 (between)
    /// expected 应为 [min, max] 数组
    fn between(field: &Value, expected: &Value) -> Result<bool> {
        let arr = expected
            .as_array()
            .ok_or_else(|| RuleError::TypeMismatch {
                expected: "array [min, max]".to_string(),
                actual: Self::type_name(expected).to_string(),
            })?;

        if arr.len() != 2 {
            return Err(RuleError::ParseError(
                "between 操作符需要 [min, max] 数组".to_string(),
            ));
        }

        let field_num = Self::as_f64(field).ok_or_else(|| RuleError::TypeMismatch {
            expected: "number".to_string(),
            actual: Self::type_name(field).to_string(),
        })?;

        let min = Self::as_f64(&arr[0]).ok_or_else(|| RuleError::TypeMismatch {
            expected: "number".to_string(),
            actual: Self::type_name(&arr[0]).to_string(),
        })?;

        let max = Self::as_f64(&arr[1]).ok_or_else(|| RuleError::TypeMismatch {
            expected: "number".to_string(),
            actual: Self::type_name(&arr[1]).to_string(),
        })?;

        Ok(field_num >= min && field_num <= max)
    }

    /// 列表包含检查 (in)
    fn in_list(field: &Value, expected: &Value) -> Result<bool> {
        let arr = expected
            .as_array()
            .ok_or_else(|| RuleError::TypeMismatch {
                expected: "array".to_string(),
                actual: Self::type_name(expected).to_string(),
            })?;

        for item in arr {
            if Self::eq(field, item)? {
                return Ok(true);
            }
        }

        Ok(false)
    }

    /// 字符串/数组包含检查
    fn contains(field: &Value, expected: &Value) -> Result<bool> {
        match field {
            Value::String(s) => {
                let substr = expected.as_str().ok_or_else(|| RuleError::TypeMismatch {
                    expected: "string".to_string(),
                    actual: Self::type_name(expected).to_string(),
                })?;
                Ok(s.contains(substr))
            }
            Value::Array(arr) => {
                for item in arr {
                    if Self::eq(item, expected)? {
                        return Ok(true);
                    }
                }
                Ok(false)
            }
            _ => Err(RuleError::TypeMismatch {
                expected: "string or array".to_string(),
                actual: Self::type_name(field).to_string(),
            }),
        }
    }

    /// 数组包含任意一个 (contains_any)
    fn contains_any(field: &Value, expected: &Value) -> Result<bool> {
        let field_arr = field.as_array().ok_or_else(|| RuleError::TypeMismatch {
            expected: "array".to_string(),
            actual: Self::type_name(field).to_string(),
        })?;

        let expected_arr = expected.as_array().ok_or_else(|| RuleError::TypeMismatch {
            expected: "array".to_string(),
            actual: Self::type_name(expected).to_string(),
        })?;

        for expected_item in expected_arr {
            for field_item in field_arr {
                if Self::eq(field_item, expected_item)? {
                    return Ok(true);
                }
            }
        }

        Ok(false)
    }

    /// 数组包含全部 (contains_all)
    fn contains_all(field: &Value, expected: &Value) -> Result<bool> {
        let field_arr = field.as_array().ok_or_else(|| RuleError::TypeMismatch {
            expected: "array".to_string(),
            actual: Self::type_name(field).to_string(),
        })?;

        let expected_arr = expected.as_array().ok_or_else(|| RuleError::TypeMismatch {
            expected: "array".to_string(),
            actual: Self::type_name(expected).to_string(),
        })?;

        for expected_item in expected_arr {
            let mut found = false;
            for field_item in field_arr {
                if Self::eq(field_item, expected_item)? {
                    found = true;
                    break;
                }
            }
            if !found {
                return Ok(false);
            }
        }

        Ok(true)
    }

    /// 字符串前缀检查
    fn starts_with(field: &Value, expected: &Value) -> Result<bool> {
        let s = field.as_str().ok_or_else(|| RuleError::TypeMismatch {
            expected: "string".to_string(),
            actual: Self::type_name(field).to_string(),
        })?;

        let prefix = expected.as_str().ok_or_else(|| RuleError::TypeMismatch {
            expected: "string".to_string(),
            actual: Self::type_name(expected).to_string(),
        })?;

        Ok(s.starts_with(prefix))
    }

    /// 字符串后缀检查
    fn ends_with(field: &Value, expected: &Value) -> Result<bool> {
        let s = field.as_str().ok_or_else(|| RuleError::TypeMismatch {
            expected: "string".to_string(),
            actual: Self::type_name(field).to_string(),
        })?;

        let suffix = expected.as_str().ok_or_else(|| RuleError::TypeMismatch {
            expected: "string".to_string(),
            actual: Self::type_name(expected).to_string(),
        })?;

        Ok(s.ends_with(suffix))
    }

    /// 正则表达式匹配
    fn regex_match(field: &Value, expected: &Value) -> Result<bool> {
        let s = field.as_str().ok_or_else(|| RuleError::TypeMismatch {
            expected: "string".to_string(),
            actual: Self::type_name(field).to_string(),
        })?;

        let pattern = expected.as_str().ok_or_else(|| RuleError::TypeMismatch {
            expected: "string (regex pattern)".to_string(),
            actual: Self::type_name(expected).to_string(),
        })?;

        // 编译正则表达式（生产环境应使用 LRU 缓存避免重复编译）
        let regex = Regex::new(pattern).map_err(|e| {
            RuleError::ParseError(format!("无效的正则表达式 '{}': {}", pattern, e))
        })?;

        Ok(regex.is_match(s))
    }

    /// 时间比较
    fn time_compare<F>(field: &Value, expected: &Value, cmp: F) -> Result<bool>
    where
        F: Fn(DateTime<Utc>, DateTime<Utc>) -> bool,
    {
        let field_time = Self::parse_datetime(field)?;
        let expected_time = Self::parse_datetime(expected)?;

        Ok(cmp(field_time, expected_time))
    }

    /// 解析日期时间
    fn parse_datetime(value: &Value) -> Result<DateTime<Utc>> {
        let s = value.as_str().ok_or_else(|| RuleError::TypeMismatch {
            expected: "datetime string".to_string(),
            actual: Self::type_name(value).to_string(),
        })?;

        // 尝试解析 ISO 8601 格式
        if let Ok(dt) = DateTime::parse_from_rfc3339(s) {
            return Ok(dt.with_timezone(&Utc));
        }

        // 尝试解析纯日期格式
        if let Ok(date) = NaiveDate::parse_from_str(s, "%Y-%m-%d") {
            return Ok(date.and_hms_opt(0, 0, 0).unwrap().and_utc());
        }

        Err(RuleError::ParseError(format!("无法解析日期时间: '{}'", s)))
    }

    /// 尝试将 Value 转换为 f64
    fn as_f64(value: &Value) -> Option<f64> {
        match value {
            Value::Number(n) => n.as_f64(),
            Value::String(s) => s.parse().ok(),
            _ => None,
        }
    }

    /// 获取值的类型名称
    fn type_name(value: &Value) -> &'static str {
        match value {
            Value::Null => "null",
            Value::Bool(_) => "boolean",
            Value::Number(_) => "number",
            Value::String(_) => "string",
            Value::Array(_) => "array",
            Value::Object(_) => "object",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_eq_numbers() {
        assert!(ConditionEvaluator::evaluate(
            Some(&json!(100)),
            Operator::Eq,
            &json!(100)
        )
        .unwrap());

        assert!(ConditionEvaluator::evaluate(
            Some(&json!(100.0)),
            Operator::Eq,
            &json!(100)
        )
        .unwrap());
    }

    #[test]
    fn test_eq_strings() {
        assert!(ConditionEvaluator::evaluate(
            Some(&json!("hello")),
            Operator::Eq,
            &json!("hello")
        )
        .unwrap());

        assert!(!ConditionEvaluator::evaluate(
            Some(&json!("hello")),
            Operator::Eq,
            &json!("world")
        )
        .unwrap());
    }

    #[test]
    fn test_numeric_comparisons() {
        assert!(ConditionEvaluator::evaluate(Some(&json!(100)), Operator::Gt, &json!(50)).unwrap());
        assert!(
            ConditionEvaluator::evaluate(Some(&json!(100)), Operator::Gte, &json!(100)).unwrap()
        );
        assert!(ConditionEvaluator::evaluate(Some(&json!(50)), Operator::Lt, &json!(100)).unwrap());
        assert!(
            ConditionEvaluator::evaluate(Some(&json!(100)), Operator::Lte, &json!(100)).unwrap()
        );
    }

    #[test]
    fn test_between() {
        assert!(
            ConditionEvaluator::evaluate(Some(&json!(50)), Operator::Between, &json!([0, 100]))
                .unwrap()
        );
        assert!(
            !ConditionEvaluator::evaluate(Some(&json!(150)), Operator::Between, &json!([0, 100]))
                .unwrap()
        );
    }

    #[test]
    fn test_in_list() {
        assert!(ConditionEvaluator::evaluate(
            Some(&json!("a")),
            Operator::In,
            &json!(["a", "b", "c"])
        )
        .unwrap());

        assert!(!ConditionEvaluator::evaluate(
            Some(&json!("d")),
            Operator::In,
            &json!(["a", "b", "c"])
        )
        .unwrap());
    }

    #[test]
    fn test_contains_string() {
        assert!(ConditionEvaluator::evaluate(
            Some(&json!("hello world")),
            Operator::Contains,
            &json!("world")
        )
        .unwrap());
    }

    #[test]
    fn test_contains_array() {
        assert!(ConditionEvaluator::evaluate(
            Some(&json!(["a", "b", "c"])),
            Operator::Contains,
            &json!("b")
        )
        .unwrap());
    }

    #[test]
    fn test_contains_any() {
        assert!(ConditionEvaluator::evaluate(
            Some(&json!(["a", "b", "c"])),
            Operator::ContainsAny,
            &json!(["b", "d"])
        )
        .unwrap());

        assert!(!ConditionEvaluator::evaluate(
            Some(&json!(["a", "b", "c"])),
            Operator::ContainsAny,
            &json!(["x", "y"])
        )
        .unwrap());
    }

    #[test]
    fn test_contains_all() {
        assert!(ConditionEvaluator::evaluate(
            Some(&json!(["a", "b", "c"])),
            Operator::ContainsAll,
            &json!(["a", "b"])
        )
        .unwrap());

        assert!(!ConditionEvaluator::evaluate(
            Some(&json!(["a", "b", "c"])),
            Operator::ContainsAll,
            &json!(["a", "d"])
        )
        .unwrap());
    }

    #[test]
    fn test_starts_with() {
        assert!(ConditionEvaluator::evaluate(
            Some(&json!("hello world")),
            Operator::StartsWith,
            &json!("hello")
        )
        .unwrap());
    }

    #[test]
    fn test_ends_with() {
        assert!(ConditionEvaluator::evaluate(
            Some(&json!("hello world")),
            Operator::EndsWith,
            &json!("world")
        )
        .unwrap());
    }

    #[test]
    fn test_regex() {
        assert!(ConditionEvaluator::evaluate(
            Some(&json!("user@example.com")),
            Operator::Regex,
            &json!(r"^[\w.-]+@[\w.-]+\.\w+$")
        )
        .unwrap());
    }

    #[test]
    fn test_is_empty() {
        assert!(ConditionEvaluator::evaluate(None, Operator::IsEmpty, &json!(null)).unwrap());
        assert!(
            ConditionEvaluator::evaluate(Some(&json!(null)), Operator::IsEmpty, &json!(null))
                .unwrap()
        );
        assert!(
            ConditionEvaluator::evaluate(Some(&json!("")), Operator::IsEmpty, &json!(null)).unwrap()
        );
        assert!(
            ConditionEvaluator::evaluate(Some(&json!([])), Operator::IsEmpty, &json!(null)).unwrap()
        );
        assert!(
            !ConditionEvaluator::evaluate(Some(&json!("hello")), Operator::IsEmpty, &json!(null))
                .unwrap()
        );
    }

    #[test]
    fn test_time_comparison() {
        assert!(ConditionEvaluator::evaluate(
            Some(&json!("2024-01-15T10:00:00Z")),
            Operator::Before,
            &json!("2024-01-20T10:00:00Z")
        )
        .unwrap());

        assert!(ConditionEvaluator::evaluate(
            Some(&json!("2024-01-20T10:00:00Z")),
            Operator::After,
            &json!("2024-01-15T10:00:00Z")
        )
        .unwrap());
    }

    #[test]
    fn test_missing_field() {
        assert!(!ConditionEvaluator::evaluate(None, Operator::Eq, &json!("test")).unwrap());
    }
}
