//! 权益处理器 Trait 定义
//!
//! 提供权益发放的统一抽象接口，支持不同权益类型的多态实现

use async_trait::async_trait;
use serde_json::Value;

use crate::error::{BadgeError, Result};
use crate::models::{BenefitType, GrantStatus};

use super::dto::{BenefitGrantRequest, BenefitGrantResult, BenefitRevokeResult};

/// 权益处理器 Trait
///
/// 定义权益发放的统一接口，每种权益类型（优惠券、积分、数字资产等）
/// 都需要实现此 trait。通过 Handler 注册表统一管理，实现发放逻辑的解耦。
///
/// # 设计说明
///
/// - `grant`: 核心发放方法，必须实现
/// - `query_status`: 查询发放状态，用于异步发放场景的状态追踪
/// - `revoke`: 撤销权益，提供默认实现返回不支持错误
/// - `validate_config`: 配置校验，在发放前验证配置的合法性
///
/// # 示例
///
/// ```ignore
/// struct CouponHandler {
///     coupon_client: CouponServiceClient,
/// }
///
/// #[async_trait]
/// impl BenefitHandler for CouponHandler {
///     fn benefit_type(&self) -> BenefitType {
///         BenefitType::Coupon
///     }
///
///     async fn grant(&self, request: BenefitGrantRequest) -> Result<BenefitGrantResult> {
///         // 调用优惠券服务发放
///         let coupon = self.coupon_client.issue(&request).await?;
///         Ok(BenefitGrantResult::success(request.grant_no)
///             .with_external_ref(coupon.id))
///     }
///
///     async fn query_status(&self, grant_id: &str) -> Result<GrantStatus> {
///         // 优惠券是同步发放，直接返回成功
///         Ok(GrantStatus::Success)
///     }
///
///     fn validate_config(&self, config: &Value) -> Result<()> {
///         // 验证必须包含 coupon_template_id
///         if config.get("coupon_template_id").is_none() {
///             return Err(BadgeError::Validation("缺少 coupon_template_id".into()));
///         }
///         Ok(())
///     }
/// }
/// ```
#[async_trait]
pub trait BenefitHandler: Send + Sync {
    /// 返回此 Handler 处理的权益类型
    ///
    /// 用于在注册表中标识和查找对应的 Handler
    fn benefit_type(&self) -> BenefitType;

    /// 发放权益
    ///
    /// 执行实际的权益发放操作。对于同步类型（如优惠券、积分），
    /// 应在此方法内完成发放并返回成功状态；对于异步类型（如数字资产），
    /// 可以先返回 Processing 状态，后续通过 `query_status` 查询最终结果。
    ///
    /// # 参数
    /// - `request`: 发放请求，包含用户、权益配置等信息
    ///
    /// # 返回
    /// - `Ok(BenefitGrantResult)`: 发放结果，包含状态和相关信息
    /// - `Err(BadgeError)`: 发放过程中的错误
    ///
    /// # 幂等性
    /// 实现应保证幂等性：相同的 `grant_no` 多次调用应返回相同结果
    async fn grant(&self, request: BenefitGrantRequest) -> Result<BenefitGrantResult>;

    /// 查询发放状态
    ///
    /// 用于异步发放场景，查询指定发放流水的当前状态。
    /// 同步发放的 Handler 可以简单返回 Success。
    ///
    /// # 参数
    /// - `grant_no`: 发放流水号
    ///
    /// # 返回
    /// - `Ok(GrantStatus)`: 当前发放状态
    /// - `Err(BadgeError)`: 查询过程中的错误
    async fn query_status(&self, grant_no: &str) -> Result<GrantStatus>;

    /// 撤销权益
    ///
    /// 回收已发放的权益。并非所有权益类型都支持撤销，
    /// 默认实现返回不支持错误。
    ///
    /// # 参数
    /// - `grant_no`: 发放流水号，标识要撤销的发放记录
    ///
    /// # 返回
    /// - `Ok(BenefitRevokeResult)`: 撤销结果
    /// - `Err(BadgeError)`: 撤销过程中的错误
    ///
    /// # 默认行为
    /// 返回 `BadgeError::Internal` 表示此权益类型不支持撤销
    async fn revoke(&self, grant_no: &str) -> Result<BenefitRevokeResult> {
        let _ = grant_no;
        Err(BadgeError::Internal(format!(
            "权益类型 {:?} 不支持撤销操作",
            self.benefit_type()
        )))
    }

    /// 验证权益配置
    ///
    /// 在发放前验证配置的合法性，避免无效配置导致发放失败。
    /// 建议在兑换规则保存时调用此方法进行预校验。
    ///
    /// # 参数
    /// - `config`: 权益配置 JSON
    ///
    /// # 返回
    /// - `Ok(())`: 配置有效
    /// - `Err(BadgeError)`: 配置无效，包含具体原因
    fn validate_config(&self, config: &Value) -> Result<()>;

    /// 获取 Handler 的描述信息（用于日志和监控）
    fn description(&self) -> &'static str {
        "Generic Benefit Handler"
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use std::sync::atomic::{AtomicU32, Ordering};
    use std::sync::Arc;

    /// 测试用的 Mock Handler
    struct MockCouponHandler {
        call_count: AtomicU32,
    }

    impl MockCouponHandler {
        fn new() -> Self {
            Self {
                call_count: AtomicU32::new(0),
            }
        }

        fn get_call_count(&self) -> u32 {
            self.call_count.load(Ordering::SeqCst)
        }
    }

    #[async_trait]
    impl BenefitHandler for MockCouponHandler {
        fn benefit_type(&self) -> BenefitType {
            BenefitType::Coupon
        }

        async fn grant(&self, request: BenefitGrantRequest) -> Result<BenefitGrantResult> {
            self.call_count.fetch_add(1, Ordering::SeqCst);

            // 模拟配置校验
            if request.benefit_config.get("coupon_template_id").is_none() {
                return Ok(BenefitGrantResult::failed(
                    request.grant_no,
                    "缺少 coupon_template_id",
                ));
            }

            Ok(BenefitGrantResult::success(request.grant_no)
                .with_external_ref("coupon-mock-001")
                .with_payload(json!({"coupon_code": "MOCK123"})))
        }

        async fn query_status(&self, _grant_no: &str) -> Result<GrantStatus> {
            Ok(GrantStatus::Success)
        }

        async fn revoke(&self, grant_no: &str) -> Result<BenefitRevokeResult> {
            Ok(BenefitRevokeResult::success(grant_no))
        }

        fn validate_config(&self, config: &Value) -> Result<()> {
            if config.get("coupon_template_id").is_none() {
                return Err(BadgeError::Validation("缺少 coupon_template_id".into()));
            }
            Ok(())
        }

        fn description(&self) -> &'static str {
            "Mock Coupon Handler for Testing"
        }
    }

    /// 测试不支持撤销的 Handler
    struct MockPhysicalHandler;

    #[async_trait]
    impl BenefitHandler for MockPhysicalHandler {
        fn benefit_type(&self) -> BenefitType {
            BenefitType::Physical
        }

        async fn grant(&self, request: BenefitGrantRequest) -> Result<BenefitGrantResult> {
            Ok(BenefitGrantResult::processing(request.grant_no)
                .with_message("实物奖品已提交发货"))
        }

        async fn query_status(&self, _grant_no: &str) -> Result<GrantStatus> {
            Ok(GrantStatus::Processing)
        }

        fn validate_config(&self, config: &Value) -> Result<()> {
            if config.get("sku_id").is_none() {
                return Err(BadgeError::Validation("缺少 sku_id".into()));
            }
            Ok(())
        }
    }

    #[tokio::test]
    async fn test_handler_benefit_type() {
        let handler = MockCouponHandler::new();
        assert_eq!(handler.benefit_type(), BenefitType::Coupon);

        let physical_handler = MockPhysicalHandler;
        assert_eq!(physical_handler.benefit_type(), BenefitType::Physical);
    }

    #[tokio::test]
    async fn test_handler_grant_success() {
        let handler = MockCouponHandler::new();
        let request = BenefitGrantRequest::new(
            "grant-001",
            "user-123",
            1,
            json!({"coupon_template_id": "tpl-001"}),
        );

        let result = handler.grant(request).await.unwrap();

        assert!(result.is_success());
        assert_eq!(result.grant_no, "grant-001");
        assert_eq!(result.external_ref, Some("coupon-mock-001".to_string()));
        assert_eq!(handler.get_call_count(), 1);
    }

    #[tokio::test]
    async fn test_handler_grant_invalid_config() {
        let handler = MockCouponHandler::new();
        let request = BenefitGrantRequest::new(
            "grant-002",
            "user-123",
            1,
            json!({}), // 缺少 coupon_template_id
        );

        let result = handler.grant(request).await.unwrap();

        assert!(!result.is_success());
        assert_eq!(result.status, GrantStatus::Failed);
    }

    #[tokio::test]
    async fn test_handler_query_status() {
        let handler = MockCouponHandler::new();
        let status = handler.query_status("grant-001").await.unwrap();
        assert_eq!(status, GrantStatus::Success);
    }

    #[tokio::test]
    async fn test_handler_revoke_supported() {
        let handler = MockCouponHandler::new();
        let result = handler.revoke("grant-001").await.unwrap();

        assert!(result.success);
        assert_eq!(result.grant_no, "grant-001");
    }

    #[tokio::test]
    async fn test_handler_revoke_not_supported() {
        let handler = MockPhysicalHandler;
        let result = handler.revoke("grant-001").await;

        // 默认实现返回不支持错误
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.to_string().contains("不支持撤销"));
    }

    #[tokio::test]
    async fn test_handler_validate_config_valid() {
        let handler = MockCouponHandler::new();
        let config = json!({"coupon_template_id": "tpl-001"});
        assert!(handler.validate_config(&config).is_ok());
    }

    #[tokio::test]
    async fn test_handler_validate_config_invalid() {
        let handler = MockCouponHandler::new();
        let config = json!({});
        let result = handler.validate_config(&config);

        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(matches!(err, BadgeError::Validation(_)));
    }

    #[tokio::test]
    async fn test_handler_description() {
        let handler = MockCouponHandler::new();
        assert_eq!(handler.description(), "Mock Coupon Handler for Testing");

        let physical_handler = MockPhysicalHandler;
        assert_eq!(physical_handler.description(), "Generic Benefit Handler");
    }

    #[tokio::test]
    async fn test_handler_as_trait_object() {
        // 验证 Handler 可以作为 trait object 使用
        let handler: Arc<dyn BenefitHandler> = Arc::new(MockCouponHandler::new());

        let request = BenefitGrantRequest::new(
            "grant-003",
            "user-123",
            1,
            json!({"coupon_template_id": "tpl-001"}),
        );

        let result = handler.grant(request).await.unwrap();
        assert!(result.is_success());
    }
}
