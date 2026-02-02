//! 规则引擎与徽章管理 gRPC 客户端封装
//!
//! 在行为事件服务的基础上，增加 revoke_badge 方法支持退款/取消场景的徽章撤销。
//! 通过 TransactionRuleService trait 抽象 gRPC 调用，便于测试时注入 mock 实现。

use std::collections::BTreeMap;

use async_trait::async_trait;
use badge_proto::badge::badge_management_service_client::BadgeManagementServiceClient;
use badge_proto::badge::{GrantBadgeRequest, RevokeBadgeRequest};
use badge_proto::rule_engine::BatchEvaluateRequest;
use badge_proto::rule_engine::rule_engine_service_client::RuleEngineServiceClient;
use tonic::transport::Channel;
use tracing::{debug, info, warn};

use crate::error::TransactionError;

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

/// 徽章撤销结果
#[derive(Debug, Clone)]
pub struct RevokeResult {
    pub success: bool,
    pub message: String,
}

/// 规则与徽章 gRPC 客户端的抽象接口
///
/// 相比行为事件服务的 BadgeRuleService，额外提供 revoke_badge 方法，
/// 因为交易场景中退款/取消需要撤回已发放的徽章。
#[async_trait]
pub trait TransactionRuleService: Send + Sync {
    /// 批量评估规则，返回匹配的规则列表
    async fn evaluate_rules(
        &self,
        rule_ids: &[String],
        context: serde_json::Value,
    ) -> Result<Vec<RuleMatch>, TransactionError>;

    /// 发放徽章给用户
    async fn grant_badge(
        &self,
        user_id: &str,
        badge_id: i64,
        quantity: i32,
        source_ref: &str,
    ) -> Result<GrantResult, TransactionError>;

    /// 退款/取消时撤销已发放的徽章
    async fn revoke_badge(
        &self,
        user_id: &str,
        badge_id: i64,
        quantity: i32,
        reason: &str,
    ) -> Result<RevokeResult, TransactionError>;
}

// ---------------------------------------------------------------------------
// gRPC 客户端实现
// ---------------------------------------------------------------------------

/// 封装规则引擎和徽章管理 gRPC 调用
///
/// 持有两个 gRPC 客户端连接，统一错误转换和日志追踪。
/// Channel 内部带连接池，clone 是廉价操作，所以这里用 Clone 语义的客户端。
pub struct TransactionRuleClient {
    rule_engine: RuleEngineServiceClient<Channel>,
    badge_service: BadgeManagementServiceClient<Channel>,
}

impl TransactionRuleClient {
    /// 创建规则引擎和徽章管理 gRPC 客户端
    ///
    /// 使用懒连接模式，不会在启动时尝试建立连接。
    /// 连接将在首次 RPC 调用时按需建立，使服务可以独立启动。
    pub fn new(rule_engine_url: &str, badge_service_url: &str) -> Result<Self, TransactionError> {
        let rule_engine_channel = tonic::transport::Endpoint::from_shared(rule_engine_url.to_string())
            .map_err(|e| TransactionError::RuleEngineError(format!("无效的规则引擎 URL: {e}")))?
            .connect_lazy();

        let badge_service_channel =
            tonic::transport::Endpoint::from_shared(badge_service_url.to_string())
                .map_err(|e| {
                    TransactionError::BadgeGrantError(format!("无效的徽章管理服务 URL: {e}"))
                })?
                .connect_lazy();

        info!(
            rule_engine_url,
            badge_service_url, "gRPC 客户端已初始化（懒连接模式）"
        );

        Ok(Self {
            rule_engine: RuleEngineServiceClient::new(rule_engine_channel),
            badge_service: BadgeManagementServiceClient::new(badge_service_channel),
        })
    }
}

#[async_trait]
impl TransactionRuleService for TransactionRuleClient {
    async fn evaluate_rules(
        &self,
        rule_ids: &[String],
        context: serde_json::Value,
    ) -> Result<Vec<RuleMatch>, TransactionError> {
        let prost_struct = json_to_prost_struct(&context);

        let request = BatchEvaluateRequest {
            rule_ids: rule_ids.to_vec(),
            context: Some(prost_struct),
        };

        debug!(rule_count = rule_ids.len(), "调用规则引擎 BatchEvaluate");

        let mut client = self.rule_engine.clone();
        let response = client.batch_evaluate(request).await.map_err(|e| {
            TransactionError::RuleEngineError(format!("BatchEvaluate 调用失败: {e}"))
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
        badge_id: i64,
        quantity: i32,
        source_ref: &str,
    ) -> Result<GrantResult, TransactionError> {
        let request = GrantBadgeRequest {
            user_id: user_id.to_string(),
            badge_id: badge_id.to_string(),
            quantity,
            source_type: "event".to_string(),
            source_ref: source_ref.to_string(),
            operator: String::new(),
        };

        debug!(user_id, badge_id, quantity, source_ref, "调用 GrantBadge");

        let mut client = self.badge_service.clone();
        let response = client
            .grant_badge(request)
            .await
            .map_err(|e| TransactionError::BadgeGrantError(format!("GrantBadge 调用失败: {e}")))?;

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

    async fn revoke_badge(
        &self,
        user_id: &str,
        badge_id: i64,
        quantity: i32,
        reason: &str,
    ) -> Result<RevokeResult, TransactionError> {
        let request = RevokeBadgeRequest {
            user_id: user_id.to_string(),
            badge_id: badge_id.to_string(),
            quantity,
            reason: reason.to_string(),
            // 退款撤销由系统自动触发，operator 留空表示非人工操作
            operator: String::new(),
        };

        debug!(user_id, badge_id, quantity, reason, "调用 RevokeBadge");

        let mut client = self.badge_service.clone();
        let response = client.revoke_badge(request).await.map_err(|e| {
            TransactionError::BadgeRevokeError(format!("RevokeBadge 调用失败: {e}"))
        })?;

        let revoke_response = response.into_inner();

        if revoke_response.success {
            info!(user_id, badge_id, "徽章撤销成功");
        } else {
            warn!(
                user_id,
                badge_id,
                message = %revoke_response.message,
                "徽章撤销未成功"
            );
        }

        Ok(RevokeResult {
            success: revoke_response.success,
            message: revoke_response.message,
        })
    }
}

// ---------------------------------------------------------------------------
// JSON -> prost_types::Struct 转换
// ---------------------------------------------------------------------------

/// 将 serde_json::Value 转换为 prost_types::Struct
///
/// 规则引擎的 gRPC 接口要求 context 为 protobuf Struct 类型，
/// 而事件数据是 JSON 格式。此函数处理 JSON 到 protobuf 的完整类型映射。
pub fn json_to_prost_struct(json: &serde_json::Value) -> prost_types::Struct {
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

        assert_eq!(prost_struct.fields.len(), 5);

        let user_id = &prost_struct.fields["user_id"];
        assert!(matches!(
            &user_id.kind,
            Some(prost_types::value::Kind::StringValue(s)) if s == "user-001"
        ));

        let amount = &prost_struct.fields["amount"];
        assert!(matches!(
            &amount.kind,
            Some(prost_types::value::Kind::NumberValue(v)) if (*v - 100.5).abs() < f64::EPSILON
        ));

        let is_vip = &prost_struct.fields["is_vip"];
        assert!(matches!(
            &is_vip.kind,
            Some(prost_types::value::Kind::BoolValue(true))
        ));

        let tags = &prost_struct.fields["tags"];
        assert!(matches!(
            &tags.kind,
            Some(prost_types::value::Kind::ListValue(list)) if list.values.len() == 2
        ));

        let address = &prost_struct.fields["address"];
        assert!(matches!(
            &address.kind,
            Some(prost_types::value::Kind::NullValue(0))
        ));
    }

    #[test]
    fn test_json_to_prost_struct_non_object_wraps_in_value_key() {
        let json = serde_json::json!("just a string");
        let prost_struct = json_to_prost_struct(&json);

        assert_eq!(prost_struct.fields.len(), 1);
        assert!(prost_struct.fields.contains_key("value"));
    }
}
