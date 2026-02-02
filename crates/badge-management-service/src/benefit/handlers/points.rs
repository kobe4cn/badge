//! 积分权益处理器
//!
//! 同步发放积分，调用外部积分系统 API

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use tracing::{debug, error, info, instrument, warn};
use uuid::Uuid;

use crate::benefit::dto::{BenefitGrantRequest, BenefitGrantResult, BenefitRevokeResult};
use crate::benefit::handler::BenefitHandler;
use crate::error::{BadgeError, Result};
use crate::models::{BenefitType, GrantStatus};

/// 积分配置结构
///
/// 配置需要包含 `point_amount`，可选 `point_type`（默认为 "general"）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PointsConfig {
    /// 发放积分数量（必填）
    pub point_amount: i64,
    /// 积分类型（可选，默认 "general"）
    #[serde(default = "default_point_type")]
    pub point_type: String,
    /// 积分备注（可选）
    pub remark: Option<String>,
    /// 积分有效期天数（可选，不填则永久有效）
    pub validity_days: Option<i32>,
}

fn default_point_type() -> String {
    "general".to_string()
}

/// 积分发放响应（模拟外部系统返回）
#[derive(Debug, Clone, Serialize, Deserialize)]
struct PointsGrantResponse {
    transaction_id: String,
    point_amount: i64,
    point_type: String,
    balance_after: i64,
}

/// 积分处理器
///
/// 通过调用外部积分服务同步发放积分。
/// 支持撤销操作（积分回收）。
pub struct PointsHandler {
    /// 积分服务 base URL（实际实现时使用）
    #[allow(dead_code)]
    points_service_url: String,
}

impl PointsHandler {
    /// 创建积分处理器
    pub fn new(points_service_url: impl Into<String>) -> Self {
        Self {
            points_service_url: points_service_url.into(),
        }
    }
}

impl Default for PointsHandler {
    fn default() -> Self {
        Self::new("http://points-service:8080")
    }
}

impl PointsHandler {
    /// 解析积分配置
    fn parse_config(&self, config: &Value) -> Result<PointsConfig> {
        serde_json::from_value(config.clone())
            .map_err(|e| BadgeError::Validation(format!("积分配置解析失败: {}", e)))
    }

    /// 调用外部积分服务发放（stub 实现）
    ///
    /// TODO: 替换为实际的积分服务 SDK 调用
    async fn grant_points(
        &self,
        user_id: &str,
        config: &PointsConfig,
        grant_no: &str,
    ) -> Result<PointsGrantResponse> {
        let transaction_id = Uuid::new_v4().to_string();

        debug!(
            user_id = %user_id,
            point_amount = config.point_amount,
            point_type = %config.point_type,
            grant_no = %grant_no,
            transaction_id = %transaction_id,
            "模拟发放积分"
        );

        // 模拟发放后的余额（实际应由积分服务返回）
        let mock_balance_after = config.point_amount + 1000;

        Ok(PointsGrantResponse {
            transaction_id,
            point_amount: config.point_amount,
            point_type: config.point_type.clone(),
            balance_after: mock_balance_after,
        })
    }

    /// 调用外部积分服务撤销（stub 实现）
    ///
    /// TODO: 替换为实际的积分服务 SDK 调用
    async fn revoke_points(&self, transaction_id: &str, amount: i64) -> Result<()> {
        debug!(
            transaction_id = %transaction_id,
            amount = amount,
            "模拟撤销积分"
        );
        // 模拟撤销成功
        Ok(())
    }
}

#[async_trait]
impl BenefitHandler for PointsHandler {
    fn benefit_type(&self) -> BenefitType {
        BenefitType::Points
    }

    #[instrument(
        skip(self, request),
        fields(
            grant_no = %request.grant_no,
            user_id = %request.user_id,
            benefit_type = "points"
        )
    )]
    async fn grant(&self, request: BenefitGrantRequest) -> Result<BenefitGrantResult> {
        // 解析配置
        let config = match self.parse_config(&request.benefit_config) {
            Ok(c) => c,
            Err(e) => {
                error!(error = %e, "积分配置解析失败");
                return Ok(BenefitGrantResult::failed(&request.grant_no, e.to_string()));
            }
        };

        info!(
            point_amount = config.point_amount,
            point_type = %config.point_type,
            "开始发放积分"
        );

        // 调用外部服务发放积分
        match self
            .grant_points(&request.user_id, &config, &request.grant_no)
            .await
        {
            Ok(response) => {
                info!(
                    transaction_id = %response.transaction_id,
                    balance_after = response.balance_after,
                    "积分发放成功"
                );

                // 构建成功结果
                let result = BenefitGrantResult::success(&request.grant_no)
                    .with_granted_now()
                    .with_external_ref(&response.transaction_id)
                    .with_payload(serde_json::json!({
                        "transaction_id": response.transaction_id,
                        "point_amount": response.point_amount,
                        "point_type": response.point_type,
                        "balance_after": response.balance_after,
                    }));

                Ok(result)
            }
            Err(e) => {
                error!(error = %e, "积分发放失败");
                Ok(BenefitGrantResult::failed(&request.grant_no, e.to_string()))
            }
        }
    }

    async fn query_status(&self, _grant_no: &str) -> Result<GrantStatus> {
        // 积分是同步发放，查询时直接返回成功
        // 实际实现可以查询外部系统确认状态
        Ok(GrantStatus::Success)
    }

    async fn revoke(&self, grant_no: &str) -> Result<BenefitRevokeResult> {
        info!(grant_no = %grant_no, "撤销积分");

        // TODO: 实际实现需要：
        // 1. 从 benefit_grants 表查询 payload（包含 point_amount）
        // 2. 使用查询到的金额调用积分系统撤销 API
        // 当前为 stub 实现，使用 0 作为占位符
        warn!("revoke() 当前为 stub 实现，生产环境需要从数据库查询发放金额");
        match self.revoke_points(grant_no, 0).await {
            Ok(()) => Ok(BenefitRevokeResult::success(grant_no)),
            Err(e) => Ok(BenefitRevokeResult::failed(grant_no, e.to_string())),
        }
    }

    fn validate_config(&self, config: &Value) -> Result<()> {
        // 验证必要字段存在
        let points_config = self.parse_config(config)?;

        if points_config.point_amount <= 0 {
            return Err(BadgeError::Validation("point_amount 必须大于 0".into()));
        }

        if points_config.point_type.is_empty() {
            return Err(BadgeError::Validation("point_type 不能为空".into()));
        }

        if let Some(days) = points_config.validity_days
            && days <= 0
        {
            return Err(BadgeError::Validation("validity_days 必须大于 0".into()));
        }

        Ok(())
    }

    fn description(&self) -> &'static str {
        "Points Handler - 积分同步发放"
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn create_handler() -> PointsHandler {
        PointsHandler::default()
    }

    #[test]
    fn test_parse_config_success() {
        let handler = create_handler();
        let config = json!({
            "point_amount": 100,
            "point_type": "bonus",
            "remark": "活动奖励"
        });

        let parsed = handler.parse_config(&config).unwrap();
        assert_eq!(parsed.point_amount, 100);
        assert_eq!(parsed.point_type, "bonus");
        assert_eq!(parsed.remark, Some("活动奖励".to_string()));
    }

    #[test]
    fn test_parse_config_default_point_type() {
        let handler = create_handler();
        let config = json!({
            "point_amount": 50
        });

        let parsed = handler.parse_config(&config).unwrap();
        assert_eq!(parsed.point_type, "general");
    }

    #[test]
    fn test_validate_config_success() {
        let handler = create_handler();
        let config = json!({
            "point_amount": 100
        });

        assert!(handler.validate_config(&config).is_ok());
    }

    #[test]
    fn test_validate_config_missing_amount() {
        let handler = create_handler();
        let config = json!({});

        let result = handler.validate_config(&config);
        assert!(result.is_err());
    }

    #[test]
    fn test_validate_config_invalid_amount() {
        let handler = create_handler();
        let config = json!({
            "point_amount": 0
        });

        let result = handler.validate_config(&config);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("point_amount"));
    }

    #[test]
    fn test_validate_config_negative_amount() {
        let handler = create_handler();
        let config = json!({
            "point_amount": -100
        });

        let result = handler.validate_config(&config);
        assert!(result.is_err());
    }

    #[test]
    fn test_validate_config_empty_point_type() {
        let handler = create_handler();
        let config = json!({
            "point_amount": 100,
            "point_type": ""
        });

        let result = handler.validate_config(&config);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("point_type"));
    }

    #[test]
    fn test_validate_config_invalid_validity_days() {
        let handler = create_handler();
        let config = json!({
            "point_amount": 100,
            "validity_days": 0
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
                "point_amount": 100,
                "point_type": "bonus"
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
        assert!(payload.get("transaction_id").is_some());
        assert_eq!(payload.get("point_amount").unwrap(), 100);
        assert_eq!(payload.get("point_type").unwrap(), "bonus");
        assert!(payload.get("balance_after").is_some());
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
        assert_eq!(handler.benefit_type(), BenefitType::Points);
    }

    #[test]
    fn test_description() {
        let handler = create_handler();
        assert!(handler.description().contains("Points"));
    }
}
