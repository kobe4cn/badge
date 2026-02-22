//! 规则引擎与徽章管理 gRPC 客户端封装
//!
//! 将规则引擎的 BatchEvaluate 和徽章管理的 GrantBadge 两个 gRPC 调用
//! 封装为统一接口，并通过 trait 抽象以支持单元测试中的 mock 注入。

use std::collections::BTreeMap;

use async_trait::async_trait;
use badge_proto::badge::GrantBadgeRequest;
use badge_proto::badge::badge_management_service_client::BadgeManagementServiceClient;
use badge_proto::rule_engine::BatchEvaluateRequest;
use badge_proto::rule_engine::rule_engine_service_client::RuleEngineServiceClient;
use tonic::transport::Channel;
use tracing::{debug, info, warn};

use crate::error::EngagementError;

// ---------------------------------------------------------------------------
// Trait 抽象 — 便于测试时替换为 mock 实现
// ---------------------------------------------------------------------------

/// 规则评估匹配结果
#[derive(Debug, Clone)]
pub struct RuleMatch {
    pub rule_id: String,
    pub rule_name: String,
    pub matched_conditions: Vec<String>,
}

/// 徽章发放结果
#[derive(Debug, Clone)]
pub struct GrantResult {
    pub success: bool,
    pub user_badge_id: String,
    pub message: String,
}

/// 规则与徽章 gRPC 客户端的抽象接口
///
/// 提取 trait 使得 processor 可以通过泛型注入依赖，
/// 测试时无需启动真实 gRPC 服务。
#[async_trait]
pub trait BadgeRuleService: Send + Sync {
    /// 批量评估规则，返回匹配的规则列表
    async fn evaluate_rules(
        &self,
        rule_ids: &[String],
        context: serde_json::Value,
    ) -> Result<Vec<RuleMatch>, EngagementError>;

    /// 发放徽章给用户
    async fn grant_badge(
        &self,
        user_id: &str,
        badge_id: &str,
        quantity: i32,
        source_ref: &str,
    ) -> Result<GrantResult, EngagementError>;
}

// ---------------------------------------------------------------------------
// gRPC 客户端实现
// ---------------------------------------------------------------------------

/// 封装规则引擎和徽章管理 gRPC 调用
///
/// 持有两个 gRPC 客户端连接，统一错误转换和日志追踪。
/// Channel 内部带连接池，clone 是廉价操作，所以这里用 Clone 语义的客户端。
pub struct BadgeRuleClient {
    rule_engine: RuleEngineServiceClient<Channel>,
    badge_service: BadgeManagementServiceClient<Channel>,
}

impl BadgeRuleClient {
    /// 创建规则引擎和徽章管理 gRPC 客户端
    ///
    /// 使用懒连接模式，不会在启动时尝试建立连接。
    /// 连接将在首次 RPC 调用时按需建立，使服务可以独立启动。
    /// `tls_config` 为 None 时使用明文连接，Some 时启用 TLS。
    pub fn new(
        rule_engine_url: &str,
        badge_service_url: &str,
        tls_config: Option<tonic::transport::ClientTlsConfig>,
    ) -> Result<Self, EngagementError> {
        let mut rule_engine_endpoint = tonic::transport::Endpoint::from_shared(rule_engine_url.to_string())
            .map_err(|e| EngagementError::RuleEngineError(format!("无效的规则引擎 URL: {e}")))?;
        if let Some(ref tls) = tls_config {
            rule_engine_endpoint = rule_engine_endpoint
                .tls_config(tls.clone())
                .map_err(|e| EngagementError::RuleEngineError(format!("规则引擎 TLS 配置失败: {e}")))?;
        }

        let mut badge_service_endpoint = tonic::transport::Endpoint::from_shared(badge_service_url.to_string())
            .map_err(|e| {
                EngagementError::BadgeGrantError(format!("无效的徽章管理服务 URL: {e}"))
            })?;
        if let Some(ref tls) = tls_config {
            badge_service_endpoint = badge_service_endpoint
                .tls_config(tls.clone())
                .map_err(|e| EngagementError::BadgeGrantError(format!("徽章服务 TLS 配置失败: {e}")))?;
        }

        let tls_status = if tls_config.is_some() { "TLS" } else { "plaintext" };
        info!(
            rule_engine_url,
            badge_service_url,
            tls_status,
            "gRPC 客户端已初始化（懒连接模式）"
        );

        Ok(Self {
            rule_engine: RuleEngineServiceClient::new(rule_engine_endpoint.connect_lazy()),
            badge_service: BadgeManagementServiceClient::new(badge_service_endpoint.connect_lazy()),
        })
    }
}

#[async_trait]
impl BadgeRuleService for BadgeRuleClient {
    async fn evaluate_rules(
        &self,
        rule_ids: &[String],
        context: serde_json::Value,
    ) -> Result<Vec<RuleMatch>, EngagementError> {
        let prost_struct = json_to_prost_struct(&context);

        let request = BatchEvaluateRequest {
            rule_ids: rule_ids.to_vec(),
            context: Some(prost_struct),
        };

        debug!(rule_count = rule_ids.len(), "调用规则引擎 BatchEvaluate");

        // clone 客户端避免 &mut self 限制（tonic 客户端 clone 很轻量）
        let mut client = self.rule_engine.clone();
        let response = client.batch_evaluate(request).await.map_err(|e| {
            EngagementError::RuleEngineError(format!("BatchEvaluate 调用失败: {e}"))
        })?;

        let batch_result = response.into_inner();

        let matches: Vec<RuleMatch> = batch_result
            .results
            .into_iter()
            .filter(|r| r.matched)
            .map(|r| RuleMatch {
                rule_id: r.rule_id,
                rule_name: r.rule_name,
                matched_conditions: r.matched_conditions,
            })
            .collect();

        info!(
            total_evaluated = rule_ids.len(),
            matched = matches.len(),
            evaluation_time_ms = batch_result.total_evaluation_time_ms,
            "规则评估完成"
        );

        Ok(matches)
    }

    async fn grant_badge(
        &self,
        user_id: &str,
        badge_id: &str,
        quantity: i32,
        source_ref: &str,
    ) -> Result<GrantResult, EngagementError> {
        let request = GrantBadgeRequest {
            user_id: user_id.to_string(),
            badge_id: badge_id.to_string(),
            quantity,
            source_type: "event".to_string(),
            source_ref: source_ref.to_string(),
            operator: String::new(),
        };

        debug!(
            user_id,
            badge_id, quantity, source_ref, "调用徽章管理服务 GrantBadge"
        );

        let mut client = self.badge_service.clone();
        let response = client
            .grant_badge(request)
            .await
            .map_err(|e| EngagementError::BadgeGrantError(format!("GrantBadge 调用失败: {e}")))?;

        let grant_response = response.into_inner();

        if grant_response.success {
            info!(
                user_id,
                badge_id,
                user_badge_id = %grant_response.user_badge_id,
                "徽章发放成功"
            );
        } else {
            warn!(
                user_id,
                badge_id,
                message = %grant_response.message,
                "徽章发放未成功"
            );
        }

        Ok(GrantResult {
            success: grant_response.success,
            user_badge_id: grant_response.user_badge_id,
            message: grant_response.message,
        })
    }
}

// ---------------------------------------------------------------------------
// JSON -> prost_types::Struct 转换
// ---------------------------------------------------------------------------

/// 将 serde_json::Value 转换为 prost_types::Struct
///
/// 规则引擎的 gRPC 接口要求 context 为 protobuf Struct 类型，
/// 而事件数据是 JSON 格式。此函数处理 JSON 到 protobuf 的完整类型映射：
/// - Object -> Struct
/// - Array -> ListValue
/// - String/Number/Bool/Null -> 对应的 protobuf Value kind
fn json_to_prost_struct(json: &serde_json::Value) -> prost_types::Struct {
    match json {
        serde_json::Value::Object(map) => {
            let fields = map
                .iter()
                .map(|(k, v)| (k.clone(), json_to_prost_value(v)))
                .collect::<BTreeMap<String, prost_types::Value>>();
            prost_types::Struct { fields }
        }
        // 非对象类型包装为只有 "value" 键的 Struct，保持接口一致性
        other => {
            let mut fields = BTreeMap::new();
            fields.insert("value".to_string(), json_to_prost_value(other));
            prost_types::Struct { fields }
        }
    }
}

/// 递归转换 JSON 值为 protobuf Value
fn json_to_prost_value(json: &serde_json::Value) -> prost_types::Value {
    let kind = match json {
        serde_json::Value::Null => prost_types::value::Kind::NullValue(0),
        serde_json::Value::Bool(b) => prost_types::value::Kind::BoolValue(*b),
        serde_json::Value::Number(n) => {
            // JSON Number 统一转为 f64，protobuf 只支持 double
            prost_types::value::Kind::NumberValue(n.as_f64().unwrap_or(0.0))
        }
        serde_json::Value::String(s) => prost_types::value::Kind::StringValue(s.clone()),
        serde_json::Value::Array(arr) => {
            let values = arr.iter().map(json_to_prost_value).collect();
            prost_types::value::Kind::ListValue(prost_types::ListValue { values })
        }
        serde_json::Value::Object(map) => {
            let fields = map
                .iter()
                .map(|(k, v)| (k.clone(), json_to_prost_value(v)))
                .collect::<BTreeMap<String, prost_types::Value>>();
            prost_types::value::Kind::StructValue(prost_types::Struct { fields })
        }
    };

    prost_types::Value { kind: Some(kind) }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_json_to_prost_struct_basic() {
        let json = serde_json::json!({
            "user_id": "user-001",
            "amount": 100.5,
            "is_vip": true,
            "tags": ["new", "active"],
            "address": null,
        });

        let prost_struct = json_to_prost_struct(&json);

        // 验证字段数量
        assert_eq!(prost_struct.fields.len(), 5);

        // 验证字符串字段
        let user_id = &prost_struct.fields["user_id"];
        assert!(matches!(
            &user_id.kind,
            Some(prost_types::value::Kind::StringValue(s)) if s == "user-001"
        ));

        // 验证数值字段
        let amount = &prost_struct.fields["amount"];
        assert!(matches!(
            &amount.kind,
            Some(prost_types::value::Kind::NumberValue(v)) if (*v - 100.5).abs() < f64::EPSILON
        ));

        // 验证布尔字段
        let is_vip = &prost_struct.fields["is_vip"];
        assert!(matches!(
            &is_vip.kind,
            Some(prost_types::value::Kind::BoolValue(true))
        ));

        // 验证数组字段
        let tags = &prost_struct.fields["tags"];
        assert!(matches!(
            &tags.kind,
            Some(prost_types::value::Kind::ListValue(list)) if list.values.len() == 2
        ));

        // 验证 null 字段
        let address = &prost_struct.fields["address"];
        assert!(matches!(
            &address.kind,
            Some(prost_types::value::Kind::NullValue(0))
        ));
    }

    #[test]
    fn test_json_to_prost_struct_nested_object() {
        let json = serde_json::json!({
            "user": {
                "name": "张三",
                "level": 5
            }
        });

        let prost_struct = json_to_prost_struct(&json);
        let user = &prost_struct.fields["user"];

        if let Some(prost_types::value::Kind::StructValue(inner)) = &user.kind {
            assert_eq!(inner.fields.len(), 2);
            assert!(matches!(
                &inner.fields["name"].kind,
                Some(prost_types::value::Kind::StringValue(s)) if s == "张三"
            ));
        } else {
            panic!("嵌套对象应转换为 StructValue");
        }
    }

    #[test]
    fn test_json_to_prost_struct_non_object_wraps_in_value_key() {
        let json = serde_json::json!("just a string");
        let prost_struct = json_to_prost_struct(&json);

        assert_eq!(prost_struct.fields.len(), 1);
        assert!(prost_struct.fields.contains_key("value"));
    }
}
