//! 权益发放集成测试
//!
//! 测试权益发放的完整业务流程，包括：
//! - 优惠券发放与撤销
//! - 积分发放
//! - 实物异步发放
//! - BenefitService 完整流程

use badge_management::benefit::{
    BenefitGrantRequest, BenefitHandler, BenefitService, CouponHandler, GrantBenefitRequest,
    HandlerRegistry, PhysicalHandler, PointsHandler,
};
use badge_management::models::{BenefitType, GrantStatus, RevokeReason};
use serde_json::json;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

/// 测试用的流水号生成器，确保每个测试用例的流水号唯一
static TEST_GRANT_NO: AtomicU64 = AtomicU64::new(1000);

fn next_grant_no() -> String {
    let id = TEST_GRANT_NO.fetch_add(1, Ordering::Relaxed);
    format!("TEST-GRANT-{:06}", id)
}

/// 创建有效的收货地址 JSON
fn create_valid_address() -> serde_json::Value {
    json!({
        "recipient_name": "张三",
        "phone": "13800138000",
        "province": "北京市",
        "city": "北京市",
        "district": "朝阳区",
        "address": "某某街道 123 号"
    })
}

/// 创建使用默认 Handler 的 BenefitService
fn create_service() -> BenefitService {
    BenefitService::with_defaults()
}

/// 创建自定义注册表的 BenefitService
fn create_service_with_registry(registry: HandlerRegistry) -> BenefitService {
    BenefitService::new(Arc::new(registry))
}

// ============================================================================
// 优惠券发放测试
// ============================================================================

mod coupon_integration {
    use super::*;

    /// 测试优惠券发放成功场景
    ///
    /// 验证：
    /// 1. 提供正确的配置后，优惠券能成功发放
    /// 2. 返回结果包含预期的字段（external_ref、payload、granted_at）
    /// 3. 状态为 Success
    #[tokio::test]
    async fn test_coupon_grant_success() {
        let service = create_service();
        let grant_no = next_grant_no();

        let request = GrantBenefitRequest::new(
            "user-coupon-001",
            BenefitType::Coupon,
            1,
            json!({
                "coupon_template_id": "tpl-summer-2024",
                "quantity": 1,
                "validity_days": 30
            }),
        )
        .with_grant_no(&grant_no);

        let response = service.grant_benefit(request).await.unwrap();

        // 验证发放成功
        assert!(response.is_success(), "优惠券发放应成功");
        assert_eq!(response.grant_no, grant_no);
        assert_eq!(response.benefit_type, BenefitType::Coupon);

        // 验证返回数据完整性
        assert!(
            response.external_ref.is_some(),
            "应返回外部系统引用（coupon_id）"
        );
        assert!(
            response.granted_at.is_some(),
            "应记录发放时间"
        );
        assert!(
            response.payload.is_some(),
            "应返回 payload 数据"
        );

        // 验证 payload 包含优惠券信息
        let payload = response.payload.unwrap();
        assert!(
            payload.get("coupon_id").is_some(),
            "payload 应包含 coupon_id"
        );
        assert!(
            payload.get("coupon_code").is_some(),
            "payload 应包含 coupon_code"
        );
    }

    /// 测试优惠券发放失败场景（配置缺失）
    ///
    /// 验证：缺少必要字段时返回 Failed 状态而非抛出异常
    #[tokio::test]
    async fn test_coupon_grant_invalid_config() {
        let service = create_service();
        let grant_no = next_grant_no();

        let request = GrantBenefitRequest::new(
            "user-coupon-002",
            BenefitType::Coupon,
            1,
            json!({}), // 缺少 coupon_template_id
        )
        .with_grant_no(&grant_no);

        let response = service.grant_benefit(request).await.unwrap();

        // 验证发放失败
        assert!(!response.is_success(), "缺少必要配置应导致发放失败");
        assert_eq!(response.status, GrantStatus::Failed);
        assert!(
            response.error_message.is_some(),
            "失败时应返回错误消息"
        );
    }

    /// 测试优惠券撤销成功场景
    ///
    /// 验证：
    /// 1. 已发放的优惠券可以被成功撤销
    /// 2. 撤销后状态变为 Revoked
    /// 3. 再次查询状态应为 Revoked
    #[tokio::test]
    async fn test_coupon_revoke() {
        let service = create_service();
        let grant_no = next_grant_no();

        // 先发放优惠券
        let request = GrantBenefitRequest::new(
            "user-coupon-003",
            BenefitType::Coupon,
            1,
            json!({
                "coupon_template_id": "tpl-revoke-test"
            }),
        )
        .with_grant_no(&grant_no);

        let grant_response = service.grant_benefit(request).await.unwrap();
        assert!(grant_response.is_success(), "发放应成功");

        // 撤销优惠券
        let revoke_result = service
            .revoke_grant(&grant_no, RevokeReason::UserRequest)
            .await
            .unwrap();

        // 验证撤销成功
        assert!(revoke_result.success, "撤销应成功");
        assert_eq!(revoke_result.reason, RevokeReason::UserRequest);
        assert!(revoke_result.revoked_at.is_some(), "应记录撤销时间");

        // 验证状态已更新
        let status = service.query_grant_status(&grant_no).await.unwrap();
        assert_eq!(status, GrantStatus::Revoked, "状态应为已撤销");
    }

    /// 测试重复撤销场景
    ///
    /// 验证：已撤销的优惠券不能再次撤销
    #[tokio::test]
    async fn test_coupon_double_revoke() {
        let service = create_service();
        let grant_no = next_grant_no();

        // 发放并撤销
        let request = GrantBenefitRequest::new(
            "user-coupon-004",
            BenefitType::Coupon,
            1,
            json!({
                "coupon_template_id": "tpl-double-revoke"
            }),
        )
        .with_grant_no(&grant_no);

        service.grant_benefit(request).await.unwrap();
        service
            .revoke_grant(&grant_no, RevokeReason::UserRequest)
            .await
            .unwrap();

        // 尝试再次撤销
        let second_revoke = service
            .revoke_grant(&grant_no, RevokeReason::SystemError)
            .await
            .unwrap();

        // 验证第二次撤销失败
        assert!(!second_revoke.success, "重复撤销应失败");
        assert!(
            second_revoke.message.unwrap().contains("已撤销"),
            "错误消息应说明已撤销"
        );
    }
}

// ============================================================================
// 积分发放测试
// ============================================================================

mod points_integration {
    use super::*;

    /// 测试积分发放成功场景
    ///
    /// 验证：
    /// 1. 提供正确的积分配置后成功发放
    /// 2. 返回结果包含交易 ID 和余额信息
    #[tokio::test]
    async fn test_points_grant_success() {
        let service = create_service();
        let grant_no = next_grant_no();

        let request = GrantBenefitRequest::new(
            "user-points-001",
            BenefitType::Points,
            1,
            json!({
                "point_amount": 500,
                "point_type": "bonus",
                "remark": "活动奖励积分"
            }),
        )
        .with_grant_no(&grant_no);

        let response = service.grant_benefit(request).await.unwrap();

        // 验证发放成功
        assert!(response.is_success(), "积分发放应成功");
        assert_eq!(response.grant_no, grant_no);
        assert_eq!(response.benefit_type, BenefitType::Points);

        // 验证返回数据
        assert!(
            response.external_ref.is_some(),
            "应返回积分交易 ID"
        );
        assert!(response.payload.is_some(), "应返回 payload");

        // 验证 payload 包含积分信息
        let payload = response.payload.unwrap();
        assert_eq!(
            payload.get("point_amount").unwrap(),
            500,
            "应返回发放的积分数量"
        );
        assert_eq!(
            payload.get("point_type").unwrap(),
            "bonus",
            "应返回积分类型"
        );
        assert!(
            payload.get("balance_after").is_some(),
            "应返回发放后余额"
        );
    }

    /// 测试积分发放使用默认积分类型
    ///
    /// 验证：不指定 point_type 时使用默认值 "general"
    #[tokio::test]
    async fn test_points_grant_default_type() {
        let service = create_service();
        let grant_no = next_grant_no();

        let request = GrantBenefitRequest::new(
            "user-points-002",
            BenefitType::Points,
            1,
            json!({
                "point_amount": 100
                // 不指定 point_type，使用默认值
            }),
        )
        .with_grant_no(&grant_no);

        let response = service.grant_benefit(request).await.unwrap();

        assert!(response.is_success());
        let payload = response.payload.unwrap();
        assert_eq!(
            payload.get("point_type").unwrap(),
            "general",
            "默认积分类型应为 general"
        );
    }

    /// 测试积分配置预校验（无效金额）
    ///
    /// 验证：使用 validate_config 可以在发放前检测无效配置
    /// 注意：当前 stub 实现的 grant 方法不会校验金额有效性，
    /// 生产环境应在发放前调用 validate_config 进行预校验
    #[test]
    fn test_points_validate_invalid_amount() {
        let service = create_service();

        // 零金额应校验失败
        let zero_amount = json!({
            "point_amount": 0
        });
        assert!(
            service.validate_config(BenefitType::Points, &zero_amount).is_err(),
            "零金额应校验失败"
        );

        // 负金额应校验失败
        let negative_amount = json!({
            "point_amount": -100
        });
        assert!(
            service.validate_config(BenefitType::Points, &negative_amount).is_err(),
            "负金额应校验失败"
        );

        // 正金额应校验通过
        let valid_amount = json!({
            "point_amount": 100
        });
        assert!(
            service.validate_config(BenefitType::Points, &valid_amount).is_ok(),
            "有效金额应校验通过"
        );
    }

    /// 测试积分配置缺失必要字段
    ///
    /// 验证：缺少 point_amount 字段时发放失败
    #[tokio::test]
    async fn test_points_grant_missing_amount() {
        let service = create_service();
        let grant_no = next_grant_no();

        let request = GrantBenefitRequest::new(
            "user-points-003",
            BenefitType::Points,
            1,
            json!({}), // 缺少 point_amount
        )
        .with_grant_no(&grant_no);

        let response = service.grant_benefit(request).await.unwrap();

        // 配置解析失败，返回 Failed 状态
        assert!(!response.is_success(), "缺少必要字段应导致发放失败");
        assert_eq!(response.status, GrantStatus::Failed);
    }

    /// 测试积分撤销
    ///
    /// 验证：积分支持撤销操作
    #[tokio::test]
    async fn test_points_revoke() {
        let service = create_service();
        let grant_no = next_grant_no();

        // 先发放积分
        let request = GrantBenefitRequest::new(
            "user-points-004",
            BenefitType::Points,
            1,
            json!({
                "point_amount": 200,
                "point_type": "activity"
            }),
        )
        .with_grant_no(&grant_no);

        service.grant_benefit(request).await.unwrap();

        // 撤销积分
        let revoke_result = service
            .revoke_grant(&grant_no, RevokeReason::OrderRefund)
            .await
            .unwrap();

        assert!(revoke_result.success, "积分撤销应成功");
        assert_eq!(revoke_result.reason, RevokeReason::OrderRefund);
    }
}

// ============================================================================
// 实物发放测试
// ============================================================================

mod physical_integration {
    use super::*;

    /// 测试实物异步发放场景
    ///
    /// 验证：
    /// 1. 实物发放返回 Processing 状态（异步处理）
    /// 2. 返回消息 ID 用于追踪物流
    /// 3. 状态查询返回 Processing
    #[tokio::test]
    async fn test_physical_async_grant() {
        let service = create_service();
        let grant_no = next_grant_no();

        let request = GrantBenefitRequest::new(
            "user-physical-001",
            BenefitType::Physical,
            1,
            json!({
                "sku_id": "SKU-BADGE-001",
                "sku_name": "限量版徽章实物",
                "quantity": 1,
                "shipping_address": create_valid_address()
            }),
        )
        .with_grant_no(&grant_no);

        let response = service.grant_benefit(request).await.unwrap();

        // 验证异步发放状态
        assert!(
            response.is_processing(),
            "实物发放应返回 Processing 状态"
        );
        assert_eq!(response.status, GrantStatus::Processing);
        assert_eq!(response.benefit_type, BenefitType::Physical);

        // 验证返回消息 ID
        assert!(
            response.external_ref.is_some(),
            "应返回 Kafka 消息 ID"
        );

        // 验证 payload
        let payload = response.payload.unwrap();
        assert_eq!(payload.get("sku_id").unwrap(), "SKU-BADGE-001");

        // 验证状态查询
        let status = service.query_grant_status(&grant_no).await.unwrap();
        assert_eq!(
            status,
            GrantStatus::Processing,
            "状态查询应返回 Processing"
        );
    }

    /// 测试实物发放地址从 metadata 获取
    ///
    /// 验证：收货地址可以通过 metadata 传递而非配置
    #[tokio::test]
    async fn test_physical_address_from_metadata() {
        let service = create_service();
        let grant_no = next_grant_no();

        let request = GrantBenefitRequest::new(
            "user-physical-002",
            BenefitType::Physical,
            1,
            json!({
                "sku_id": "SKU-BADGE-002"
                // 配置中不包含地址
            }),
        )
        .with_grant_no(&grant_no)
        .with_metadata(json!({
            "shipping_address": create_valid_address()
        }));

        let response = service.grant_benefit(request).await.unwrap();

        // 验证从 metadata 获取地址后成功处理
        assert!(
            response.is_processing(),
            "从 metadata 获取地址后应成功处理"
        );
    }

    /// 测试实物发放缺少地址场景
    ///
    /// 验证：缺少收货地址时发放失败
    #[tokio::test]
    async fn test_physical_missing_address() {
        let service = create_service();
        let grant_no = next_grant_no();

        let request = GrantBenefitRequest::new(
            "user-physical-003",
            BenefitType::Physical,
            1,
            json!({
                "sku_id": "SKU-BADGE-003"
                // 没有收货地址
            }),
        )
        .with_grant_no(&grant_no);

        let response = service.grant_benefit(request).await.unwrap();

        // 验证失败
        assert!(!response.is_success(), "缺少地址应导致发放失败");
        assert!(!response.is_processing());
        assert_eq!(response.status, GrantStatus::Failed);
        assert!(
            response.error_message.unwrap().contains("收货地址"),
            "错误消息应说明缺少收货地址"
        );
    }

    /// 测试实物不支持撤销
    ///
    /// 验证：实物发放后不能撤销（状态为 Processing 也不允许）
    #[tokio::test]
    async fn test_physical_revoke_not_allowed() {
        let service = create_service();
        let grant_no = next_grant_no();

        // 先发放实物
        let request = GrantBenefitRequest::new(
            "user-physical-004",
            BenefitType::Physical,
            1,
            json!({
                "sku_id": "SKU-BADGE-004",
                "shipping_address": create_valid_address()
            }),
        )
        .with_grant_no(&grant_no);

        service.grant_benefit(request).await.unwrap();

        // 尝试撤销
        let revoke_result = service
            .revoke_grant(&grant_no, RevokeReason::UserRequest)
            .await
            .unwrap();

        // 验证撤销失败（Processing 状态不允许撤销）
        assert!(!revoke_result.success, "实物发放不应支持撤销");
    }
}

// ============================================================================
// BenefitService 完整流程测试
// ============================================================================

mod benefit_service_flow {
    use super::*;

    /// 测试完整的权益发放流程
    ///
    /// 验证 BenefitService 的以下能力：
    /// 1. 自动生成流水号
    /// 2. 幂等性控制
    /// 3. 状态查询
    /// 4. 配置验证
    #[tokio::test]
    async fn test_benefit_service_flow() {
        let service = create_service();

        // 1. 测试自动生成流水号
        let request1 = GrantBenefitRequest::new(
            "user-flow-001",
            BenefitType::Coupon,
            1,
            json!({
                "coupon_template_id": "tpl-flow-test"
            }),
        );
        // 不指定 grant_no，应自动生成

        let response1 = service.grant_benefit(request1).await.unwrap();
        assert!(response1.is_success());
        assert!(
            response1.grant_no.starts_with("BG"),
            "自动生成的流水号应以 BG 开头"
        );
        assert_eq!(
            response1.grant_no.len(),
            16,
            "流水号长度应为 16"
        );

        // 2. 测试幂等性
        let grant_no = next_grant_no();
        let request2a = GrantBenefitRequest::new(
            "user-flow-002",
            BenefitType::Coupon,
            1,
            json!({
                "coupon_template_id": "tpl-idempotent"
            }),
        )
        .with_grant_no(&grant_no);

        let response2a = service.grant_benefit(request2a).await.unwrap();
        assert!(response2a.is_success());

        // 使用相同 grant_no 再次发放
        let request2b = GrantBenefitRequest::new(
            "user-flow-002",
            BenefitType::Coupon,
            1,
            json!({
                "coupon_template_id": "tpl-idempotent"
            }),
        )
        .with_grant_no(&grant_no);

        let response2b = service.grant_benefit(request2b).await.unwrap();
        // 应返回已存在的记录，而非重复发放
        assert_eq!(response2b.grant_no, grant_no);
        assert!(
            response2b.error_message.is_some(),
            "重复请求应有提示"
        );
        assert!(
            response2b.error_message.unwrap().contains("重复"),
            "提示应说明是重复请求"
        );

        // 3. 测试状态查询（不存在的记录）
        let query_result = service.query_grant_status("non-existent-grant").await;
        assert!(query_result.is_err(), "查询不存在的记录应返回错误");

        // 4. 测试配置验证
        let valid_coupon_config = json!({
            "coupon_template_id": "tpl-valid"
        });
        assert!(
            service
                .validate_config(BenefitType::Coupon, &valid_coupon_config)
                .is_ok(),
            "有效配置应通过验证"
        );

        let invalid_coupon_config = json!({});
        assert!(
            service
                .validate_config(BenefitType::Coupon, &invalid_coupon_config)
                .is_err(),
            "无效配置应验证失败"
        );
    }

    /// 测试支持的权益类型
    ///
    /// 验证默认注册表包含预期的 Handler
    #[tokio::test]
    async fn test_supported_benefit_types() {
        let service = create_service();

        let types = service.supported_types();
        assert_eq!(types.len(), 3, "默认应支持 3 种权益类型");

        assert!(service.supports(BenefitType::Coupon), "应支持优惠券");
        assert!(service.supports(BenefitType::Points), "应支持积分");
        assert!(service.supports(BenefitType::Physical), "应支持实物");

        // 未注册的类型
        assert!(
            !service.supports(BenefitType::DigitalAsset),
            "默认不应支持数字资产"
        );
        assert!(
            !service.supports(BenefitType::Membership),
            "默认不应支持会员权益"
        );
    }

    /// 测试批量状态查询
    #[tokio::test]
    async fn test_batch_status_query() {
        let service = create_service();

        // 发放多个权益
        let mut grant_nos = Vec::new();
        for i in 0..3 {
            let grant_no = next_grant_no();
            let request = GrantBenefitRequest::new(
                format!("user-batch-{}", i),
                BenefitType::Coupon,
                1,
                json!({
                    "coupon_template_id": format!("tpl-batch-{}", i)
                }),
            )
            .with_grant_no(&grant_no);

            service.grant_benefit(request).await.unwrap();
            grant_nos.push(grant_no);
        }

        // 批量查询
        let statuses = service.query_grant_statuses(&grant_nos).await.unwrap();

        assert_eq!(statuses.len(), 3, "应返回 3 个状态");
        for (grant_no, status) in statuses {
            assert!(
                grant_nos.contains(&grant_no),
                "返回的流水号应在请求列表中"
            );
            assert_eq!(status, GrantStatus::Success, "所有发放应成功");
        }
    }

    /// 测试带元数据的发放
    #[tokio::test]
    async fn test_grant_with_metadata() {
        let service = create_service();
        let grant_no = next_grant_no();

        let request = GrantBenefitRequest::new(
            "user-metadata-001",
            BenefitType::Coupon,
            1,
            json!({
                "coupon_template_id": "tpl-metadata"
            }),
        )
        .with_grant_no(&grant_no)
        .with_redemption_order(12345)
        .with_metadata(json!({
            "source": "promotion",
            "campaign_id": "summer-2024"
        }));

        let response = service.grant_benefit(request).await.unwrap();

        assert!(response.is_success());
    }

    /// 测试不同撤销原因
    #[tokio::test]
    async fn test_revoke_with_different_reasons() {
        let service = create_service();

        let reasons = vec![
            RevokeReason::UserRequest,
            RevokeReason::OrderRefund,
            RevokeReason::Expiration,
            RevokeReason::Violation,
            RevokeReason::SystemError,
        ];

        for (i, reason) in reasons.iter().enumerate() {
            let grant_no = next_grant_no();

            // 发放
            let request = GrantBenefitRequest::new(
                format!("user-revoke-reason-{}", i),
                BenefitType::Coupon,
                1,
                json!({
                    "coupon_template_id": format!("tpl-reason-{}", i)
                }),
            )
            .with_grant_no(&grant_no);

            service.grant_benefit(request).await.unwrap();

            // 撤销
            let result = service.revoke_grant(&grant_no, *reason).await.unwrap();

            assert!(result.success, "撤销原因 {:?} 应成功", reason);
            assert_eq!(result.reason, *reason);
        }
    }
}

// ============================================================================
// Handler 直接测试
// ============================================================================

mod handler_direct_tests {
    use super::*;

    /// 直接测试 CouponHandler
    #[tokio::test]
    async fn test_coupon_handler_direct() {
        let handler = CouponHandler::default();

        assert_eq!(handler.benefit_type(), BenefitType::Coupon);
        assert!(handler.description().contains("Coupon"));

        // 测试发放
        let request = BenefitGrantRequest::new(
            next_grant_no(),
            "user-direct-001",
            1,
            json!({
                "coupon_template_id": "tpl-direct"
            }),
        );

        let result = handler.grant(request).await.unwrap();
        assert!(result.is_success());

        // 测试状态查询
        let status = handler.query_status("any").await.unwrap();
        assert_eq!(status, GrantStatus::Success);

        // 测试撤销
        let revoke_result = handler.revoke("any").await.unwrap();
        assert!(revoke_result.success);
    }

    /// 直接测试 PointsHandler
    #[tokio::test]
    async fn test_points_handler_direct() {
        let handler = PointsHandler::default();

        assert_eq!(handler.benefit_type(), BenefitType::Points);
        assert!(handler.description().contains("Points"));

        // 测试发放
        let request = BenefitGrantRequest::new(
            next_grant_no(),
            "user-direct-002",
            1,
            json!({
                "point_amount": 100
            }),
        );

        let result = handler.grant(request).await.unwrap();
        assert!(result.is_success());
    }

    /// 直接测试 PhysicalHandler
    #[tokio::test]
    async fn test_physical_handler_direct() {
        let handler = PhysicalHandler::default();

        assert_eq!(handler.benefit_type(), BenefitType::Physical);
        assert!(handler.description().contains("Physical"));

        // 测试发放
        let request = BenefitGrantRequest::new(
            next_grant_no(),
            "user-direct-003",
            1,
            json!({
                "sku_id": "SKU-DIRECT",
                "shipping_address": create_valid_address()
            }),
        );

        let result = handler.grant(request).await.unwrap();
        assert!(result.is_processing());

        // 测试状态查询
        let status = handler.query_status("any").await.unwrap();
        assert_eq!(status, GrantStatus::Processing);

        // 测试撤销（应失败）
        let revoke_result = handler.revoke("any").await;
        assert!(revoke_result.is_err(), "实物 Handler 不应支持撤销");
    }

    /// 测试 Handler 作为 trait object 使用
    #[tokio::test]
    async fn test_handler_as_trait_object() {
        let handlers: Vec<Arc<dyn BenefitHandler>> = vec![
            Arc::new(CouponHandler::default()),
            Arc::new(PointsHandler::default()),
            Arc::new(PhysicalHandler::default()),
        ];

        for handler in handlers {
            // 所有 Handler 都应能正常调用 benefit_type
            let _ = handler.benefit_type();
            // 所有 Handler 都应有描述
            assert!(!handler.description().is_empty());
        }
    }
}

// ============================================================================
// HandlerRegistry 测试
// ============================================================================

mod registry_tests {
    use super::*;

    /// 测试默认注册表
    #[test]
    fn test_default_registry() {
        let registry = HandlerRegistry::with_defaults();

        assert!(registry.contains(BenefitType::Coupon));
        assert!(registry.contains(BenefitType::Points));
        assert!(registry.contains(BenefitType::Physical));
        assert!(!registry.contains(BenefitType::DigitalAsset));

        let types = registry.registered_types();
        assert_eq!(types.len(), 3);
    }

    /// 测试自定义注册表
    #[test]
    fn test_custom_registry() {
        let mut registry = HandlerRegistry::new();

        // 初始为空
        assert!(!registry.contains(BenefitType::Coupon));

        // 注册 CouponHandler
        registry.register(Arc::new(CouponHandler::default()));

        assert!(registry.contains(BenefitType::Coupon));
        assert!(!registry.contains(BenefitType::Points));

        // 获取 Handler
        let handler = registry.get(BenefitType::Coupon);
        assert!(handler.is_some());
        assert_eq!(handler.unwrap().benefit_type(), BenefitType::Coupon);
    }

    /// 测试使用自定义注册表创建服务
    #[tokio::test]
    async fn test_service_with_custom_registry() {
        let mut registry = HandlerRegistry::new();
        registry.register(Arc::new(CouponHandler::default()));
        // 只注册 CouponHandler

        let service = create_service_with_registry(registry);

        // 应只支持 Coupon
        assert!(service.supports(BenefitType::Coupon));
        assert!(!service.supports(BenefitType::Points));
        assert!(!service.supports(BenefitType::Physical));

        // 发放优惠券应成功
        let request = GrantBenefitRequest::new(
            "user-custom-001",
            BenefitType::Coupon,
            1,
            json!({
                "coupon_template_id": "tpl-custom"
            }),
        )
        .with_grant_no(next_grant_no());

        let response = service.grant_benefit(request).await.unwrap();
        assert!(response.is_success());

        // 发放积分应失败（未注册 Handler）
        let points_request = GrantBenefitRequest::new(
            "user-custom-002",
            BenefitType::Points,
            1,
            json!({
                "point_amount": 100
            }),
        )
        .with_grant_no(next_grant_no());

        let points_result = service.grant_benefit(points_request).await;
        assert!(points_result.is_err(), "未注册的 Handler 应返回错误");
    }
}

// ============================================================================
// 边界条件测试
// ============================================================================

mod edge_cases {
    use super::*;

    /// 测试空用户 ID
    #[tokio::test]
    async fn test_empty_user_id() {
        let service = create_service();

        let request = GrantBenefitRequest::new(
            "", // 空用户 ID
            BenefitType::Coupon,
            1,
            json!({
                "coupon_template_id": "tpl-empty-user"
            }),
        )
        .with_grant_no(next_grant_no());

        // 当前实现不校验用户 ID，由上游保证
        let response = service.grant_benefit(request).await.unwrap();
        assert!(response.is_success());
    }

    /// 测试大量发放数量
    #[tokio::test]
    async fn test_large_quantity() {
        let service = create_service();

        let request = GrantBenefitRequest::new(
            "user-large-qty",
            BenefitType::Coupon,
            1,
            json!({
                "coupon_template_id": "tpl-large-qty",
                "quantity": 100 // 大量发放
            }),
        )
        .with_grant_no(next_grant_no());

        let response = service.grant_benefit(request).await.unwrap();
        assert!(response.is_success());
    }

    /// 测试大额积分
    #[tokio::test]
    async fn test_large_points_amount() {
        let service = create_service();

        let request = GrantBenefitRequest::new(
            "user-large-points",
            BenefitType::Points,
            1,
            json!({
                "point_amount": 1_000_000 // 百万积分
            }),
        )
        .with_grant_no(next_grant_no());

        let response = service.grant_benefit(request).await.unwrap();
        assert!(response.is_success());
    }

    /// 测试复杂 metadata
    #[tokio::test]
    async fn test_complex_metadata() {
        let service = create_service();

        let request = GrantBenefitRequest::new(
            "user-complex-meta",
            BenefitType::Coupon,
            1,
            json!({
                "coupon_template_id": "tpl-complex"
            }),
        )
        .with_grant_no(next_grant_no())
        .with_metadata(json!({
            "source": "api",
            "nested": {
                "level1": {
                    "level2": {
                        "value": 123
                    }
                }
            },
            "array": [1, 2, 3, "four", {"five": 5}],
            "unicode": "中文测试 🎉"
        }));

        let response = service.grant_benefit(request).await.unwrap();
        assert!(response.is_success());
    }

    /// 测试撤销不存在的记录
    #[tokio::test]
    async fn test_revoke_non_existent() {
        let service = create_service();

        let result = service
            .revoke_grant("non-existent-grant", RevokeReason::UserRequest)
            .await
            .unwrap();

        assert!(!result.success, "撤销不存在的记录应失败");
        assert!(
            result.message.unwrap().contains("不存在"),
            "错误消息应说明记录不存在"
        );
    }
}
