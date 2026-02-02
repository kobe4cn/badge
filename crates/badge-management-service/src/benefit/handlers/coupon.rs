//! 优惠券权益处理器
//!
//! 同步发放优惠券，调用外部优惠券系统 API

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use tracing::{debug, error, info, instrument, warn};
use uuid::Uuid;

use crate::benefit::dto::{BenefitGrantRequest, BenefitGrantResult, BenefitRevokeResult};
use crate::benefit::handler::BenefitHandler;
use crate::error::{BadgeError, Result};
use crate::models::{BenefitType, GrantStatus};

/// 优惠券配置结构
///
/// 配置需要包含 `coupon_template_id`，可选 `quantity`（默认为 1）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CouponConfig {
    /// 优惠券模板 ID（必填）
    pub coupon_template_id: String,
    /// 发放数量（默认 1）
    #[serde(default = "default_quantity")]
    pub quantity: i32,
    /// 有效期天数（可选，不填则使用模板默认值）
    pub validity_days: Option<i32>,
}

fn default_quantity() -> i32 {
    1
}

/// 优惠券发放响应（模拟外部系统返回）
#[derive(Debug, Clone, Serialize, Deserialize)]
struct CouponIssueResponse {
    coupon_id: String,
    coupon_code: String,
    template_id: String,
    expires_at: Option<String>,
}

/// 优惠券处理器
///
/// 通过调用外部优惠券服务同步发放优惠券。
/// 支持撤销操作（优惠券回收）。
pub struct CouponHandler {
    /// 优惠券服务 base URL（实际实现时使用）
    #[allow(dead_code)]
    coupon_service_url: String,
}

impl CouponHandler {
    /// 创建优惠券处理器
    pub fn new(coupon_service_url: impl Into<String>) -> Self {
        Self {
            coupon_service_url: coupon_service_url.into(),
        }
    }
}

impl Default for CouponHandler {
    fn default() -> Self {
        Self::new("http://coupon-service:8080")
    }
}

impl CouponHandler {
    /// 解析优惠券配置
    fn parse_config(&self, config: &Value) -> Result<CouponConfig> {
        serde_json::from_value(config.clone())
            .map_err(|e| BadgeError::Validation(format!("优惠券配置解析失败: {}", e)))
    }

    /// 调用外部优惠券服务发放（stub 实现）
    ///
    /// TODO: 替换为实际的优惠券服务 SDK 调用
    async fn issue_coupon(
        &self,
        user_id: &str,
        config: &CouponConfig,
        grant_no: &str,
    ) -> Result<CouponIssueResponse> {
        // 生成模拟的优惠券数据
        let coupon_id = Uuid::new_v4().to_string();
        let coupon_code = format!("CPN{}", &coupon_id[..8].to_uppercase());

        debug!(
            user_id = %user_id,
            template_id = %config.coupon_template_id,
            grant_no = %grant_no,
            coupon_id = %coupon_id,
            "模拟发放优惠券"
        );

        // 模拟过期时间（如果配置了有效期）
        let expires_at = config.validity_days.map(|days| {
            let expires = chrono::Utc::now() + chrono::Duration::days(days as i64);
            expires.to_rfc3339()
        });

        Ok(CouponIssueResponse {
            coupon_id,
            coupon_code,
            template_id: config.coupon_template_id.clone(),
            expires_at,
        })
    }

    /// 调用外部优惠券服务撤销（stub 实现）
    ///
    /// TODO: 替换为实际的优惠券服务 SDK 调用
    async fn revoke_coupon(&self, coupon_id: &str) -> Result<()> {
        debug!(coupon_id = %coupon_id, "模拟撤销优惠券");
        // 模拟撤销成功
        Ok(())
    }
}

#[async_trait]
impl BenefitHandler for CouponHandler {
    fn benefit_type(&self) -> BenefitType {
        BenefitType::Coupon
    }

    #[instrument(
        skip(self, request),
        fields(
            grant_no = %request.grant_no,
            user_id = %request.user_id,
            benefit_type = "coupon"
        )
    )]
    async fn grant(&self, request: BenefitGrantRequest) -> Result<BenefitGrantResult> {
        // 解析配置
        let config = match self.parse_config(&request.benefit_config) {
            Ok(c) => c,
            Err(e) => {
                error!(error = %e, "优惠券配置解析失败");
                return Ok(BenefitGrantResult::failed(&request.grant_no, e.to_string()));
            }
        };

        info!(
            template_id = %config.coupon_template_id,
            quantity = config.quantity,
            "开始发放优惠券"
        );

        // 调用外部服务发放优惠券
        match self
            .issue_coupon(&request.user_id, &config, &request.grant_no)
            .await
        {
            Ok(response) => {
                info!(
                    coupon_id = %response.coupon_id,
                    coupon_code = %response.coupon_code,
                    "优惠券发放成功"
                );

                // 构建成功结果
                let mut result = BenefitGrantResult::success(&request.grant_no)
                    .with_granted_now()
                    .with_external_ref(&response.coupon_id)
                    .with_payload(serde_json::json!({
                        "coupon_id": response.coupon_id,
                        "coupon_code": response.coupon_code,
                        "template_id": response.template_id,
                    }));

                // 设置过期时间
                if let Some(expires_str) = &response.expires_at
                    && let Ok(expires) = chrono::DateTime::parse_from_rfc3339(expires_str)
                {
                    result = result.with_expires_at(expires.with_timezone(&chrono::Utc));
                }

                Ok(result)
            }
            Err(e) => {
                error!(error = %e, "优惠券发放失败");
                Ok(BenefitGrantResult::failed(&request.grant_no, e.to_string()))
            }
        }
    }

    async fn query_status(&self, _grant_no: &str) -> Result<GrantStatus> {
        // 优惠券是同步发放，查询时直接返回成功
        // 实际实现可以查询外部系统确认状态
        Ok(GrantStatus::Success)
    }

    async fn revoke(&self, grant_no: &str) -> Result<BenefitRevokeResult> {
        info!(grant_no = %grant_no, "撤销优惠券");

        // TODO: 实际实现需要：
        // 1. 从 benefit_grants 表查询 external_ref（coupon_id）
        // 2. 使用查询到的 coupon_id 调用优惠券系统撤销 API
        // 当前为 stub 实现，使用 grant_no 模拟调用
        warn!("revoke() 当前为 stub 实现，生产环境需要从数据库查询 coupon_id");
        match self.revoke_coupon(grant_no).await {
            Ok(()) => Ok(BenefitRevokeResult::success(grant_no)),
            Err(e) => Ok(BenefitRevokeResult::failed(grant_no, e.to_string())),
        }
    }

    fn validate_config(&self, config: &Value) -> Result<()> {
        // 验证必要字段存在
        let coupon_config = self.parse_config(config)?;

        if coupon_config.coupon_template_id.is_empty() {
            return Err(BadgeError::Validation("coupon_template_id 不能为空".into()));
        }

        if coupon_config.quantity <= 0 {
            return Err(BadgeError::Validation("quantity 必须大于 0".into()));
        }

        if let Some(days) = coupon_config.validity_days
            && days <= 0
        {
            return Err(BadgeError::Validation("validity_days 必须大于 0".into()));
        }

        Ok(())
    }

    fn description(&self) -> &'static str {
        "Coupon Handler - 优惠券同步发放"
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn create_handler() -> CouponHandler {
        CouponHandler::default()
    }

    #[test]
    fn test_parse_config_success() {
        let handler = create_handler();
        let config = json!({
            "coupon_template_id": "tpl-001",
            "quantity": 2,
            "validity_days": 30
        });

        let parsed = handler.parse_config(&config).unwrap();
        assert_eq!(parsed.coupon_template_id, "tpl-001");
        assert_eq!(parsed.quantity, 2);
        assert_eq!(parsed.validity_days, Some(30));
    }

    #[test]
    fn test_parse_config_default_quantity() {
        let handler = create_handler();
        let config = json!({
            "coupon_template_id": "tpl-001"
        });

        let parsed = handler.parse_config(&config).unwrap();
        assert_eq!(parsed.quantity, 1);
    }

    #[test]
    fn test_validate_config_success() {
        let handler = create_handler();
        let config = json!({
            "coupon_template_id": "tpl-001",
            "quantity": 1
        });

        assert!(handler.validate_config(&config).is_ok());
    }

    #[test]
    fn test_validate_config_missing_template_id() {
        let handler = create_handler();
        let config = json!({});

        let result = handler.validate_config(&config);
        assert!(result.is_err());
    }

    #[test]
    fn test_validate_config_empty_template_id() {
        let handler = create_handler();
        let config = json!({
            "coupon_template_id": ""
        });

        let result = handler.validate_config(&config);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("不能为空"));
    }

    #[test]
    fn test_validate_config_invalid_quantity() {
        let handler = create_handler();
        let config = json!({
            "coupon_template_id": "tpl-001",
            "quantity": 0
        });

        let result = handler.validate_config(&config);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("quantity"));
    }

    #[test]
    fn test_validate_config_invalid_validity_days() {
        let handler = create_handler();
        let config = json!({
            "coupon_template_id": "tpl-001",
            "validity_days": -1
        });

        let result = handler.validate_config(&config);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("validity_days"));
    }

    #[tokio::test]
    async fn test_grant_success() {
        let handler = create_handler();
        let request = BenefitGrantRequest::new(
            "grant-001",
            "user-123",
            1,
            json!({
                "coupon_template_id": "tpl-001",
                "quantity": 1
            }),
        );

        let result = handler.grant(request).await.unwrap();

        assert!(result.is_success());
        assert_eq!(result.grant_no, "grant-001");
        assert!(result.external_ref.is_some());
        assert!(result.payload.is_some());
        assert!(result.granted_at.is_some());

        // 验证 payload 包含预期字段
        let payload = result.payload.unwrap();
        assert!(payload.get("coupon_id").is_some());
        assert!(payload.get("coupon_code").is_some());
    }

    #[tokio::test]
    async fn test_grant_invalid_config() {
        let handler = create_handler();
        let request = BenefitGrantRequest::new(
            "grant-002",
            "user-123",
            1,
            json!({}), // 缺少必要字段
        );

        let result = handler.grant(request).await.unwrap();

        assert!(!result.is_success());
        assert_eq!(result.status, GrantStatus::Failed);
    }

    #[tokio::test]
    async fn test_query_status() {
        let handler = create_handler();
        let status = handler.query_status("grant-001").await.unwrap();

        assert_eq!(status, GrantStatus::Success);
    }

    #[tokio::test]
    async fn test_revoke() {
        let handler = create_handler();
        let result = handler.revoke("grant-001").await.unwrap();

        assert!(result.success);
        assert_eq!(result.grant_no, "grant-001");
    }

    #[test]
    fn test_benefit_type() {
        let handler = create_handler();
        assert_eq!(handler.benefit_type(), BenefitType::Coupon);
    }

    #[test]
    fn test_description() {
        let handler = create_handler();
        assert!(handler.description().contains("Coupon"));
    }
}
