//! 规则模板系统集成测试
//!
//! 测试模板编译和规则评估的完整工作流，确保：
//! 1. 模板编译器能正确替换参数占位符
//! 2. 编译后的规则可以被规则引擎正确评估
//! 3. 参数验证能够捕获各类错误

use rule_engine::template::{
    CompileError, ParameterDef, ParameterType, RuleTemplate, TemplateCategory, TemplateCompiler,
};
use rule_engine::{EvaluationContext, RuleCompiler, RuleExecutor};
use serde_json::{json, Value};
use std::collections::HashMap;

// ==================== 辅助函数 ====================

/// 创建消费满额模板
///
/// 该模板用于判断用户的消费金额是否达到指定阈值，是最常见的规则类型之一
fn create_purchase_gte_template() -> RuleTemplate {
    RuleTemplate {
        id: 1,
        code: "purchase_gte".into(),
        name: "消费满额".into(),
        description: Some("判断用户消费金额是否达到指定阈值".into()),
        category: TemplateCategory::Basic,
        subcategory: None,
        template_json: json!({
            "root": {
                "type": "condition",
                "field": "order.amount",
                "operator": "gte",
                "value": "${threshold}"
            }
        }),
        parameters: vec![ParameterDef {
            name: "threshold".into(),
            param_type: ParameterType::Number,
            label: "消费金额阈值".into(),
            description: Some("用户需要达到的最低消费金额".into()),
            default: Some(json!(100)),
            required: true,
            min: Some(0.0),
            max: Some(100000.0),
            options: None,
        }],
        version: "1.0".into(),
        is_system: true,
        enabled: true,
        created_at: chrono::Utc::now(),
        updated_at: chrono::Utc::now(),
    }
}

/// 创建复合条件模板（事件类型 + 消费金额）
///
/// 用于测试嵌套参数替换和多条件组合
fn create_composite_template() -> RuleTemplate {
    RuleTemplate {
        id: 2,
        code: "event_purchase_composite".into(),
        name: "事件消费复合规则".into(),
        description: Some("同时检查事件类型和消费金额".into()),
        category: TemplateCategory::Advanced,
        subcategory: None,
        template_json: json!({
            "root": {
                "type": "group",
                "operator": "AND",
                "children": [
                    {
                        "type": "condition",
                        "field": "event.type",
                        "operator": "eq",
                        "value": "${event_type}"
                    },
                    {
                        "type": "condition",
                        "field": "order.amount",
                        "operator": "gte",
                        "value": "${amount}"
                    }
                ]
            }
        }),
        parameters: vec![
            ParameterDef {
                name: "event_type".into(),
                param_type: ParameterType::String,
                label: "事件类型".into(),
                description: None,
                default: None,
                required: true,
                min: None,
                max: None,
                options: None,
            },
            ParameterDef {
                name: "amount".into(),
                param_type: ParameterType::Number,
                label: "消费金额".into(),
                description: None,
                default: Some(json!(500)),
                required: true,
                min: Some(0.0),
                max: Some(50000.0),
                options: None,
            },
        ],
        version: "1.0".into(),
        is_system: true,
        enabled: true,
        created_at: chrono::Utc::now(),
        updated_at: chrono::Utc::now(),
    }
}

/// 创建无默认值的必填参数模板
fn create_required_no_default_template() -> RuleTemplate {
    RuleTemplate {
        id: 3,
        code: "required_test".into(),
        name: "必填参数测试".into(),
        description: None,
        category: TemplateCategory::Basic,
        subcategory: None,
        template_json: json!({
            "root": {
                "type": "condition",
                "field": "user.id",
                "operator": "eq",
                "value": "${user_id}"
            }
        }),
        parameters: vec![ParameterDef {
            name: "user_id".into(),
            param_type: ParameterType::String,
            label: "用户ID".into(),
            description: None,
            default: None,
            required: true,
            min: None,
            max: None,
            options: None,
        }],
        version: "1.0".into(),
        is_system: false,
        enabled: true,
        created_at: chrono::Utc::now(),
        updated_at: chrono::Utc::now(),
    }
}

/// 创建测试上下文：模拟一个购买事件
fn create_test_context() -> EvaluationContext {
    EvaluationContext::new(json!({
        "event": {
            "type": "PURCHASE",
            "timestamp": "2024-01-15T10:00:00Z"
        },
        "order": {
            "id": "order-12345",
            "amount": 1500,
            "currency": "CNY"
        },
        "user": {
            "id": "user-67890",
            "is_vip": true
        }
    }))
}

// ==================== 模板编译测试 ====================

#[test]
fn test_compile_purchase_gte_template() {
    // 验证消费满额模板能正确编译，参数值被正确替换
    let template_compiler = TemplateCompiler::new();
    let template = create_purchase_gte_template();
    let params: HashMap<String, Value> = [("threshold".to_string(), json!(1000))].into();

    let compiled_json = template_compiler.compile(&template, &params).unwrap();

    // 验证编译结果中 ${threshold} 被替换为 1000
    assert_eq!(compiled_json["root"]["value"], json!(1000));
    assert_eq!(compiled_json["root"]["operator"], json!("gte"));
    assert_eq!(compiled_json["root"]["field"], json!("order.amount"));
}

#[test]
fn test_compile_with_default_value() {
    // 验证当不提供参数值时，编译器使用默认值
    let template_compiler = TemplateCompiler::new();
    let template = create_purchase_gte_template();
    let params: HashMap<String, Value> = HashMap::new();

    let compiled_json = template_compiler.compile(&template, &params).unwrap();

    // 未提供 threshold，应使用默认值 100
    assert_eq!(compiled_json["root"]["value"], json!(100));
}

#[test]
fn test_compile_nested_placeholders() {
    // 验证复合模板中的多个占位符都能被正确替换
    let template_compiler = TemplateCompiler::new();
    let template = create_composite_template();
    let params: HashMap<String, Value> = [
        ("event_type".to_string(), json!("PURCHASE")),
        ("amount".to_string(), json!(2000)),
    ]
    .into();

    let compiled_json = template_compiler.compile(&template, &params).unwrap();

    // 验证两个占位符都被替换
    let children = compiled_json["root"]["children"].as_array().unwrap();
    assert_eq!(children[0]["value"], json!("PURCHASE"));
    assert_eq!(children[1]["value"], json!(2000));
}

#[test]
fn test_compile_preserves_type() {
    // 验证编译器保留参数的原始类型（数值不会变成字符串）
    let template_compiler = TemplateCompiler::new();
    let template = create_purchase_gte_template();
    let params: HashMap<String, Value> = [("threshold".to_string(), json!(500))].into();

    let compiled_json = template_compiler.compile(&template, &params).unwrap();

    // 值应该是数值类型，而不是字符串 "500"
    assert!(compiled_json["root"]["value"].is_number());
    assert_eq!(compiled_json["root"]["value"].as_i64(), Some(500));
}

// ==================== 编译 + 评估集成测试 ====================

#[test]
fn test_compile_and_evaluate_match() {
    // 完整流程测试：模板编译 -> 规则编译 -> 评估（匹配情况）
    let template_compiler = TemplateCompiler::new();
    let template = create_purchase_gte_template();
    let params: HashMap<String, Value> = [("threshold".to_string(), json!(1000))].into();

    // 步骤1: 编译模板
    let compiled_template = template_compiler.compile(&template, &params).unwrap();

    // 步骤2: 构建完整的规则 JSON
    let rule_json = json!({
        "id": "rule-from-template",
        "name": "消费满1000",
        "version": "1.0",
        "root": compiled_template["root"]
    });

    // 步骤3: 使用规则编译器编译
    let mut rule_compiler = RuleCompiler::new();
    let compiled_rule = rule_compiler
        .compile_from_json(&rule_json.to_string())
        .unwrap();

    // 步骤4: 执行评估
    let executor = RuleExecutor::new();
    let context = create_test_context(); // order.amount = 1500

    let result = executor.execute(&compiled_rule, &context).unwrap();

    // 1500 >= 1000，应该匹配
    assert!(result.matched);
}

#[test]
fn test_compile_and_evaluate_no_match() {
    // 完整流程测试：模板编译 -> 规则编译 -> 评估（不匹配情况）
    let template_compiler = TemplateCompiler::new();
    let template = create_purchase_gte_template();
    let params: HashMap<String, Value> = [("threshold".to_string(), json!(2000))].into();

    let compiled_template = template_compiler.compile(&template, &params).unwrap();

    let rule_json = json!({
        "id": "rule-from-template",
        "name": "消费满2000",
        "version": "1.0",
        "root": compiled_template["root"]
    });

    let mut rule_compiler = RuleCompiler::new();
    let compiled_rule = rule_compiler
        .compile_from_json(&rule_json.to_string())
        .unwrap();

    let executor = RuleExecutor::new();
    let context = create_test_context(); // order.amount = 1500

    let result = executor.execute(&compiled_rule, &context).unwrap();

    // 1500 < 2000，不应该匹配
    assert!(!result.matched);
}

#[test]
fn test_compile_and_evaluate_composite() {
    // 测试复合规则：事件类型 + 消费金额
    let template_compiler = TemplateCompiler::new();
    let template = create_composite_template();
    let params: HashMap<String, Value> = [
        ("event_type".to_string(), json!("PURCHASE")),
        ("amount".to_string(), json!(1000)),
    ]
    .into();

    let compiled_template = template_compiler.compile(&template, &params).unwrap();

    let rule_json = json!({
        "id": "composite-rule",
        "name": "购买且消费满1000",
        "version": "1.0",
        "root": compiled_template["root"]
    });

    let mut rule_compiler = RuleCompiler::new();
    let compiled_rule = rule_compiler
        .compile_from_json(&rule_json.to_string())
        .unwrap();

    let executor = RuleExecutor::new().with_trace();
    let context = create_test_context(); // event.type = PURCHASE, order.amount = 1500

    let result = executor.execute(&compiled_rule, &context).unwrap();

    // event.type == PURCHASE AND order.amount >= 1000，应该匹配
    assert!(result.matched);
    assert_eq!(result.matched_conditions.len(), 2);
}

#[test]
fn test_compile_and_evaluate_composite_no_match() {
    // 测试复合规则不匹配的情况：事件类型不对
    let template_compiler = TemplateCompiler::new();
    let template = create_composite_template();
    let params: HashMap<String, Value> = [
        ("event_type".to_string(), json!("REFUND")), // 不匹配
        ("amount".to_string(), json!(1000)),
    ]
    .into();

    let compiled_template = template_compiler.compile(&template, &params).unwrap();

    let rule_json = json!({
        "id": "composite-rule",
        "name": "退款且消费满1000",
        "version": "1.0",
        "root": compiled_template["root"]
    });

    let mut rule_compiler = RuleCompiler::new();
    let compiled_rule = rule_compiler
        .compile_from_json(&rule_json.to_string())
        .unwrap();

    let executor = RuleExecutor::new();
    let context = create_test_context(); // event.type = PURCHASE

    let result = executor.execute(&compiled_rule, &context).unwrap();

    // event.type != REFUND，AND 短路，不匹配
    assert!(!result.matched);
}

// ==================== 参数验证测试 ====================

#[test]
fn test_missing_required_parameter() {
    // 验证缺少必填参数时返回正确的错误
    let template_compiler = TemplateCompiler::new();
    let template = create_required_no_default_template();
    let params: HashMap<String, Value> = HashMap::new(); // 未提供 user_id

    let result = template_compiler.compile(&template, &params);

    assert!(result.is_err());
    let err = result.unwrap_err();
    assert!(matches!(err, CompileError::MissingParameter(_)));

    // 验证错误信息包含参数名
    let err_msg = err.to_string();
    assert!(err_msg.contains("user_id"));
}

#[test]
fn test_parameter_out_of_range_too_high() {
    // 验证参数值超过最大值时返回错误
    let template_compiler = TemplateCompiler::new();
    let template = create_purchase_gte_template();
    let params: HashMap<String, Value> =
        [("threshold".to_string(), json!(200000))].into(); // 超过 max=100000

    let result = template_compiler.compile(&template, &params);

    assert!(result.is_err());
    let err = result.unwrap_err();
    assert!(matches!(err, CompileError::ParamOutOfRange { .. }));
}

#[test]
fn test_parameter_out_of_range_too_low() {
    // 验证参数值低于最小值时返回错误
    let template_compiler = TemplateCompiler::new();
    let template = create_purchase_gte_template();
    let params: HashMap<String, Value> =
        [("threshold".to_string(), json!(-100))].into(); // 低于 min=0

    let result = template_compiler.compile(&template, &params);

    assert!(result.is_err());
    let err = result.unwrap_err();
    assert!(matches!(err, CompileError::ParamOutOfRange { .. }));
}

#[test]
fn test_parameter_type_mismatch() {
    // 验证参数类型不匹配时返回错误
    let template_compiler = TemplateCompiler::new();
    let template = create_purchase_gte_template();
    let params: HashMap<String, Value> =
        [("threshold".to_string(), json!("not a number"))].into(); // 应该是数值

    let result = template_compiler.compile(&template, &params);

    assert!(result.is_err());
    let err = result.unwrap_err();
    assert!(matches!(err, CompileError::TypeMismatch { .. }));
}

// ==================== 边界情况测试 ====================

#[test]
fn test_parameter_at_boundary() {
    // 验证边界值被接受
    let template_compiler = TemplateCompiler::new();
    let template = create_purchase_gte_template();

    // 测试最小边界值
    let params_min: HashMap<String, Value> = [("threshold".to_string(), json!(0))].into();
    let result = template_compiler.compile(&template, &params_min);
    assert!(result.is_ok());

    // 测试最大边界值
    let params_max: HashMap<String, Value> = [("threshold".to_string(), json!(100000))].into();
    let result = template_compiler.compile(&template, &params_max);
    assert!(result.is_ok());
}

#[test]
fn test_extra_parameters_ignored() {
    // 验证额外的参数被忽略，不影响编译
    let template_compiler = TemplateCompiler::new();
    let template = create_purchase_gte_template();
    let params: HashMap<String, Value> = [
        ("threshold".to_string(), json!(1000)),
        ("unknown_param".to_string(), json!("should be ignored")),
    ]
    .into();

    let result = template_compiler.compile(&template, &params);

    assert!(result.is_ok());
    let compiled = result.unwrap();
    assert_eq!(compiled["root"]["value"], json!(1000));
}

#[test]
fn test_float_parameter() {
    // 验证浮点数参数被正确处理
    let template_compiler = TemplateCompiler::new();
    let template = create_purchase_gte_template();
    let params: HashMap<String, Value> = [("threshold".to_string(), json!(1500.50))].into();

    let compiled = template_compiler.compile(&template, &params).unwrap();

    assert!(compiled["root"]["value"].is_number());
    assert_eq!(compiled["root"]["value"].as_f64(), Some(1500.50));
}

// ==================== 多轮编译测试 ====================

#[test]
fn test_multiple_compilations() {
    // 验证同一模板可以用不同参数多次编译
    let template_compiler = TemplateCompiler::new();
    let template = create_purchase_gte_template();

    // 第一次编译
    let params1: HashMap<String, Value> = [("threshold".to_string(), json!(500))].into();
    let compiled1 = template_compiler.compile(&template, &params1).unwrap();

    // 第二次编译
    let params2: HashMap<String, Value> = [("threshold".to_string(), json!(1000))].into();
    let compiled2 = template_compiler.compile(&template, &params2).unwrap();

    // 验证两次编译结果不同
    assert_eq!(compiled1["root"]["value"], json!(500));
    assert_eq!(compiled2["root"]["value"], json!(1000));
}

#[test]
fn test_template_reusability() {
    // 完整流程测试：同一模板生成的不同规则能正确评估
    let template_compiler = TemplateCompiler::new();
    let template = create_purchase_gte_template();
    let executor = RuleExecutor::new();
    let context = create_test_context(); // order.amount = 1500

    // 规则1: threshold = 1000（应匹配）
    let params1: HashMap<String, Value> = [("threshold".to_string(), json!(1000))].into();
    let compiled1 = template_compiler.compile(&template, &params1).unwrap();
    let rule1 = json!({
        "id": "rule-1000",
        "name": "满1000",
        "version": "1.0",
        "root": compiled1["root"]
    });
    let mut rule_compiler = RuleCompiler::new();
    let compiled_rule1 = rule_compiler.compile_from_json(&rule1.to_string()).unwrap();
    let result1 = executor.execute(&compiled_rule1, &context).unwrap();

    // 规则2: threshold = 2000（不应匹配）
    let params2: HashMap<String, Value> = [("threshold".to_string(), json!(2000))].into();
    let compiled2 = template_compiler.compile(&template, &params2).unwrap();
    let rule2 = json!({
        "id": "rule-2000",
        "name": "满2000",
        "version": "1.0",
        "root": compiled2["root"]
    });
    let compiled_rule2 = rule_compiler.compile_from_json(&rule2.to_string()).unwrap();
    let result2 = executor.execute(&compiled_rule2, &context).unwrap();

    assert!(result1.matched);
    assert!(!result2.matched);
}
