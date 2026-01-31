//! Handler 注册表
//!
//! 管理所有 BenefitHandler 实例，按权益类型索引，提供统一的查找接口。
//!
//! ## 设计说明
//!
//! 注册表采用 HashMap 存储 Handler 实例，以 BenefitType 为 key 进行路由。
//! 所有 Handler 通过 Arc 包装实现共享，支持跨线程安全访问。
//!
//! ## 使用示例
//!
//! ```ignore
//! use badge_management::benefit::registry::HandlerRegistry;
//! use badge_management::benefit::handlers::CouponHandler;
//! use badge_management::models::BenefitType;
//! use std::sync::Arc;
//!
//! // 创建注册表并注册 Handler
//! let mut registry = HandlerRegistry::new();
//! registry.register(Arc::new(CouponHandler::default()));
//!
//! // 获取 Handler
//! let handler = registry.get(BenefitType::Coupon).unwrap();
//! ```

use std::collections::HashMap;
use std::sync::Arc;

use tracing::{debug, info};

use crate::benefit::handler::BenefitHandler;
use crate::benefit::handlers::{CouponHandler, PhysicalHandler, PointsHandler};
use crate::models::BenefitType;

/// Handler 注册表
///
/// 集中管理所有权益处理器实例，根据权益类型路由到对应的 Handler。
/// 线程安全，可在多个服务实例间共享。
pub struct HandlerRegistry {
    handlers: HashMap<BenefitType, Arc<dyn BenefitHandler>>,
}

impl HandlerRegistry {
    /// 创建空的注册表
    pub fn new() -> Self {
        Self {
            handlers: HashMap::new(),
        }
    }

    /// 注册一个 Handler
    ///
    /// Handler 会根据其 `benefit_type()` 方法返回的类型进行索引。
    /// 如果已存在相同类型的 Handler，会被替换。
    pub fn register(&mut self, handler: Arc<dyn BenefitHandler>) -> &mut Self {
        let benefit_type = handler.benefit_type();
        debug!(
            benefit_type = ?benefit_type,
            description = handler.description(),
            "注册权益处理器"
        );
        self.handlers.insert(benefit_type, handler);
        self
    }

    /// 获取指定类型的 Handler
    ///
    /// 返回 None 表示该权益类型没有注册对应的 Handler
    pub fn get(&self, benefit_type: BenefitType) -> Option<Arc<dyn BenefitHandler>> {
        self.handlers.get(&benefit_type).cloned()
    }

    /// 检查是否已注册指定类型的 Handler
    pub fn contains(&self, benefit_type: BenefitType) -> bool {
        self.handlers.contains_key(&benefit_type)
    }

    /// 获取所有已注册的权益类型
    pub fn registered_types(&self) -> Vec<BenefitType> {
        self.handlers.keys().copied().collect()
    }

    /// 获取已注册的 Handler 数量
    pub fn len(&self) -> usize {
        self.handlers.len()
    }

    /// 检查注册表是否为空
    pub fn is_empty(&self) -> bool {
        self.handlers.is_empty()
    }

    /// 创建包含所有默认 Handler 的注册表
    ///
    /// 默认注册以下 Handler:
    /// - CouponHandler: 优惠券发放
    /// - PointsHandler: 积分发放
    /// - PhysicalHandler: 实物发放
    pub fn with_defaults() -> Self {
        let mut registry = Self::new();

        info!("初始化默认权益处理器");

        // 注册优惠券处理器
        registry.register(Arc::new(CouponHandler::default()));

        // 注册积分处理器
        registry.register(Arc::new(PointsHandler::default()));

        // 注册实物处理器
        registry.register(Arc::new(PhysicalHandler::default()));

        info!(
            handler_count = registry.len(),
            types = ?registry.registered_types(),
            "默认权益处理器初始化完成"
        );

        registry
    }

    /// 使用自定义配置创建注册表
    ///
    /// 允许为每个 Handler 指定不同的服务 URL 或配置
    pub fn with_config(config: RegistryConfig) -> Self {
        let mut registry = Self::new();

        info!("使用自定义配置初始化权益处理器");

        // 注册优惠券处理器
        registry.register(Arc::new(CouponHandler::new(&config.coupon_service_url)));

        // 注册积分处理器
        registry.register(Arc::new(PointsHandler::new(&config.points_service_url)));

        // 注册实物处理器
        registry.register(Arc::new(PhysicalHandler::new(
            &config.kafka_brokers,
            &config.physical_shipment_topic,
        )));

        info!(
            handler_count = registry.len(),
            "自定义权益处理器初始化完成"
        );

        registry
    }
}

impl Default for HandlerRegistry {
    fn default() -> Self {
        Self::with_defaults()
    }
}

/// 注册表配置
///
/// 用于自定义各 Handler 的外部服务配置
#[derive(Debug, Clone)]
pub struct RegistryConfig {
    /// 优惠券服务 URL
    pub coupon_service_url: String,
    /// 积分服务 URL
    pub points_service_url: String,
    /// Kafka broker 地址
    pub kafka_brokers: String,
    /// 实物发货消息 topic
    pub physical_shipment_topic: String,
}

impl Default for RegistryConfig {
    fn default() -> Self {
        Self {
            coupon_service_url: "http://coupon-service:8080".to_string(),
            points_service_url: "http://points-service:8080".to_string(),
            kafka_brokers: "localhost:9092".to_string(),
            physical_shipment_topic: "physical_shipment".to_string(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::benefit::dto::{BenefitGrantRequest, BenefitGrantResult};
    use crate::models::GrantStatus;
    use async_trait::async_trait;
    use serde_json::{json, Value};

    /// 测试用的 Mock Handler
    struct MockHandler {
        benefit_type: BenefitType,
    }

    impl MockHandler {
        fn new(benefit_type: BenefitType) -> Self {
            Self { benefit_type }
        }
    }

    #[async_trait]
    impl BenefitHandler for MockHandler {
        fn benefit_type(&self) -> BenefitType {
            self.benefit_type
        }

        async fn grant(
            &self,
            request: BenefitGrantRequest,
        ) -> crate::error::Result<BenefitGrantResult> {
            Ok(BenefitGrantResult::success(request.grant_no))
        }

        async fn query_status(&self, _grant_no: &str) -> crate::error::Result<GrantStatus> {
            Ok(GrantStatus::Success)
        }

        fn validate_config(&self, _config: &Value) -> crate::error::Result<()> {
            Ok(())
        }
    }

    #[test]
    fn test_registry_new() {
        let registry = HandlerRegistry::new();
        assert!(registry.is_empty());
        assert_eq!(registry.len(), 0);
    }

    #[test]
    fn test_registry_register_and_get() {
        let mut registry = HandlerRegistry::new();

        let handler = Arc::new(MockHandler::new(BenefitType::Coupon));
        registry.register(handler);

        assert!(!registry.is_empty());
        assert_eq!(registry.len(), 1);
        assert!(registry.contains(BenefitType::Coupon));

        let retrieved = registry.get(BenefitType::Coupon);
        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap().benefit_type(), BenefitType::Coupon);
    }

    #[test]
    fn test_registry_get_not_found() {
        let registry = HandlerRegistry::new();
        let result = registry.get(BenefitType::Coupon);
        assert!(result.is_none());
    }

    #[test]
    fn test_registry_register_multiple() {
        let mut registry = HandlerRegistry::new();

        registry
            .register(Arc::new(MockHandler::new(BenefitType::Coupon)))
            .register(Arc::new(MockHandler::new(BenefitType::Points)))
            .register(Arc::new(MockHandler::new(BenefitType::Physical)));

        assert_eq!(registry.len(), 3);
        assert!(registry.contains(BenefitType::Coupon));
        assert!(registry.contains(BenefitType::Points));
        assert!(registry.contains(BenefitType::Physical));
    }

    #[test]
    fn test_registry_register_replace() {
        let mut registry = HandlerRegistry::new();

        // 注册第一个 Coupon Handler
        registry.register(Arc::new(MockHandler::new(BenefitType::Coupon)));
        assert_eq!(registry.len(), 1);

        // 注册第二个 Coupon Handler（应该替换）
        registry.register(Arc::new(MockHandler::new(BenefitType::Coupon)));
        assert_eq!(registry.len(), 1);
    }

    #[test]
    fn test_registry_registered_types() {
        let mut registry = HandlerRegistry::new();
        registry
            .register(Arc::new(MockHandler::new(BenefitType::Coupon)))
            .register(Arc::new(MockHandler::new(BenefitType::Points)));

        let types = registry.registered_types();
        assert_eq!(types.len(), 2);
        assert!(types.contains(&BenefitType::Coupon));
        assert!(types.contains(&BenefitType::Points));
    }

    #[test]
    fn test_registry_with_defaults() {
        let registry = HandlerRegistry::with_defaults();

        // 应该包含 3 个默认 Handler
        assert_eq!(registry.len(), 3);
        assert!(registry.contains(BenefitType::Coupon));
        assert!(registry.contains(BenefitType::Points));
        assert!(registry.contains(BenefitType::Physical));

        // 验证各 Handler 的类型
        assert_eq!(
            registry.get(BenefitType::Coupon).unwrap().benefit_type(),
            BenefitType::Coupon
        );
        assert_eq!(
            registry.get(BenefitType::Points).unwrap().benefit_type(),
            BenefitType::Points
        );
        assert_eq!(
            registry.get(BenefitType::Physical).unwrap().benefit_type(),
            BenefitType::Physical
        );
    }

    #[test]
    fn test_registry_default() {
        let registry = HandlerRegistry::default();
        assert_eq!(registry.len(), 3);
    }

    #[test]
    fn test_registry_with_config() {
        let config = RegistryConfig {
            coupon_service_url: "http://custom-coupon:8080".to_string(),
            points_service_url: "http://custom-points:8080".to_string(),
            kafka_brokers: "kafka:9092".to_string(),
            physical_shipment_topic: "custom_shipment".to_string(),
        };

        let registry = HandlerRegistry::with_config(config);
        assert_eq!(registry.len(), 3);
    }

    #[test]
    fn test_registry_config_default() {
        let config = RegistryConfig::default();
        assert_eq!(config.coupon_service_url, "http://coupon-service:8080");
        assert_eq!(config.points_service_url, "http://points-service:8080");
        assert_eq!(config.kafka_brokers, "localhost:9092");
        assert_eq!(config.physical_shipment_topic, "physical_shipment");
    }

    #[tokio::test]
    async fn test_registry_handler_grant() {
        let registry = HandlerRegistry::with_defaults();

        let handler = registry.get(BenefitType::Coupon).unwrap();
        let request = BenefitGrantRequest::new(
            "test-grant",
            "user-123",
            1,
            json!({"coupon_template_id": "tpl-001"}),
        );

        let result = handler.grant(request).await.unwrap();
        assert!(result.is_success());
    }
}
