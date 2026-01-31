//! 权益发放数据传输对象
//!
//! 定义 BenefitHandler trait 使用的请求和响应结构

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::models::GrantStatus;

/// 权益发放请求
///
/// 包含发放权益所需的全部信息，由兑换服务构造后传递给具体的 Handler
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BenefitGrantRequest {
    /// 发放流水号，全局唯一，用于幂等控制和追踪
    pub grant_no: String,
    /// 目标用户 ID
    pub user_id: String,
    /// 权益定义 ID
    pub benefit_id: i64,
    /// 权益配置（JSON 格式，不同权益类型有不同的配置结构）
    ///
    /// 示例：
    /// - Coupon: {"coupon_template_id": "xxx", "quantity": 1}
    /// - Points: {"amount": 100}
    /// - DigitalAsset: {"asset_id": "xxx", "metadata": {...}}
    pub benefit_config: Value,
    /// 关联的兑换订单 ID（可选，手动发放时为空）
    #[serde(skip_serializing_if = "Option::is_none")]
    pub redemption_order_id: Option<i64>,
    /// 扩展元数据，用于传递额外的上下文信息
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<Value>,
}

impl BenefitGrantRequest {
    /// 创建权益发放请求
    pub fn new(
        grant_no: impl Into<String>,
        user_id: impl Into<String>,
        benefit_id: i64,
        benefit_config: Value,
    ) -> Self {
        Self {
            grant_no: grant_no.into(),
            user_id: user_id.into(),
            benefit_id,
            benefit_config,
            redemption_order_id: None,
            metadata: None,
        }
    }

    /// 设置关联的兑换订单 ID
    pub fn with_redemption_order(mut self, order_id: i64) -> Self {
        self.redemption_order_id = Some(order_id);
        self
    }

    /// 设置扩展元数据
    pub fn with_metadata(mut self, metadata: Value) -> Self {
        self.metadata = Some(metadata);
        self
    }
}

/// 权益发放结果
///
/// Handler 完成发放后返回的结果，包含发放状态和相关信息
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BenefitGrantResult {
    /// 发放流水号（与请求中的 grant_no 一致）
    pub grant_no: String,
    /// 发放状态
    pub status: GrantStatus,
    /// 外部系统的关联引用（如优惠券 ID、积分流水号等）
    #[serde(skip_serializing_if = "Option::is_none")]
    pub external_ref: Option<String>,
    /// 发放结果的额外数据（不同权益类型返回不同结构）
    ///
    /// 示例：
    /// - Coupon: {"coupon_id": "xxx", "coupon_code": "ABC123"}
    /// - Points: {"transaction_id": "xxx", "balance": 1000}
    #[serde(skip_serializing_if = "Option::is_none")]
    pub payload: Option<Value>,
    /// 实际发放时间（异步发放时可能与请求时间不同）
    #[serde(skip_serializing_if = "Option::is_none")]
    pub granted_at: Option<DateTime<Utc>>,
    /// 权益过期时间
    #[serde(skip_serializing_if = "Option::is_none")]
    pub expires_at: Option<DateTime<Utc>>,
    /// 结果描述信息
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
}

impl BenefitGrantResult {
    /// 创建成功的发放结果
    ///
    /// 注意：granted_at 默认为 None，由 Handler 实现者根据实际发放完成时间设置。
    /// 同步发放场景可使用 `with_granted_now()` 快捷设置当前时间。
    pub fn success(grant_no: impl Into<String>) -> Self {
        Self {
            grant_no: grant_no.into(),
            status: GrantStatus::Success,
            external_ref: None,
            payload: None,
            granted_at: None, // 留给 Handler 根据实际完成时间设置
            expires_at: None,
            message: None,
        }
    }

    /// 设置发放时间为当前时间（同步发放场景的快捷方法）
    pub fn with_granted_now(mut self) -> Self {
        self.granted_at = Some(Utc::now());
        self
    }

    /// 创建处理中的发放结果（用于异步发放场景）
    pub fn processing(grant_no: impl Into<String>) -> Self {
        Self {
            grant_no: grant_no.into(),
            status: GrantStatus::Processing,
            external_ref: None,
            payload: None,
            granted_at: None,
            expires_at: None,
            message: None,
        }
    }

    /// 创建失败的发放结果
    pub fn failed(grant_no: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            grant_no: grant_no.into(),
            status: GrantStatus::Failed,
            external_ref: None,
            payload: None,
            granted_at: None,
            expires_at: None,
            message: Some(message.into()),
        }
    }

    /// 设置外部引用
    pub fn with_external_ref(mut self, external_ref: impl Into<String>) -> Self {
        self.external_ref = Some(external_ref.into());
        self
    }

    /// 设置结果载荷
    pub fn with_payload(mut self, payload: Value) -> Self {
        self.payload = Some(payload);
        self
    }

    /// 设置过期时间
    pub fn with_expires_at(mut self, expires_at: DateTime<Utc>) -> Self {
        self.expires_at = Some(expires_at);
        self
    }

    /// 设置消息
    pub fn with_message(mut self, message: impl Into<String>) -> Self {
        self.message = Some(message.into());
        self
    }

    /// 检查发放是否成功
    pub fn is_success(&self) -> bool {
        self.status == GrantStatus::Success
    }

    /// 检查发放是否仍在处理中
    pub fn is_processing(&self) -> bool {
        self.status == GrantStatus::Processing
    }
}

/// 权益撤销结果
///
/// Handler 完成撤销后返回的结果
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BenefitRevokeResult {
    /// 发放流水号（标识被撤销的发放记录）
    pub grant_no: String,
    /// 是否成功撤销
    pub success: bool,
    /// 实际撤销时间
    #[serde(skip_serializing_if = "Option::is_none")]
    pub revoked_at: Option<DateTime<Utc>>,
    /// 结果描述信息
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
}

impl BenefitRevokeResult {
    /// 创建成功的撤销结果
    pub fn success(grant_no: impl Into<String>) -> Self {
        Self {
            grant_no: grant_no.into(),
            success: true,
            revoked_at: Some(Utc::now()),
            message: None,
        }
    }

    /// 创建失败的撤销结果
    pub fn failed(grant_no: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            grant_no: grant_no.into(),
            success: false,
            revoked_at: None,
            message: Some(message.into()),
        }
    }

    /// 设置消息
    pub fn with_message(mut self, message: impl Into<String>) -> Self {
        self.message = Some(message.into());
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_benefit_grant_request_new() {
        let config = json!({
            "coupon_template_id": "tpl-001",
            "quantity": 1
        });

        let request = BenefitGrantRequest::new("grant-001", "user-123", 1, config.clone())
            .with_redemption_order(100)
            .with_metadata(json!({"source": "promotion"}));

        assert_eq!(request.grant_no, "grant-001");
        assert_eq!(request.user_id, "user-123");
        assert_eq!(request.benefit_id, 1);
        assert_eq!(request.benefit_config, config);
        assert_eq!(request.redemption_order_id, Some(100));
        assert!(request.metadata.is_some());
    }

    #[test]
    fn test_benefit_grant_result_success() {
        let result = BenefitGrantResult::success("grant-001")
            .with_granted_now()
            .with_external_ref("coupon-abc")
            .with_payload(json!({"coupon_code": "ABC123"}))
            .with_message("发放成功");

        assert!(result.is_success());
        assert!(!result.is_processing());
        assert_eq!(result.grant_no, "grant-001");
        assert_eq!(result.external_ref, Some("coupon-abc".to_string()));
        assert!(result.granted_at.is_some());
    }

    #[test]
    fn test_benefit_grant_result_processing() {
        let result = BenefitGrantResult::processing("grant-002");

        assert!(!result.is_success());
        assert!(result.is_processing());
        assert_eq!(result.status, GrantStatus::Processing);
    }

    #[test]
    fn test_benefit_grant_result_failed() {
        let result = BenefitGrantResult::failed("grant-003", "库存不足");

        assert!(!result.is_success());
        assert_eq!(result.status, GrantStatus::Failed);
        assert_eq!(result.message, Some("库存不足".to_string()));
    }

    #[test]
    fn test_benefit_revoke_result_success() {
        let result = BenefitRevokeResult::success("grant-001").with_message("撤销成功");

        assert!(result.success);
        assert!(result.revoked_at.is_some());
        assert_eq!(result.message, Some("撤销成功".to_string()));
    }

    #[test]
    fn test_benefit_revoke_result_failed() {
        let result = BenefitRevokeResult::failed("grant-002", "权益已使用，无法撤销");

        assert!(!result.success);
        assert!(result.revoked_at.is_none());
        assert_eq!(result.message, Some("权益已使用，无法撤销".to_string()));
    }

    #[test]
    fn test_serialization() {
        let result = BenefitGrantResult::success("grant-001")
            .with_external_ref("ref-001")
            .with_payload(json!({"key": "value"}));

        let json = serde_json::to_value(&result).unwrap();
        assert_eq!(json["grantNo"], "grant-001");
        assert_eq!(json["status"], "SUCCESS");
        assert_eq!(json["externalRef"], "ref-001");
    }
}
