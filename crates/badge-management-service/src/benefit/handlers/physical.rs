//! 实物奖品权益处理器
//!
//! 异步发放实物奖品，通过 Kafka 消息触发后续物流流程

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use tracing::{debug, error, info, instrument, warn};
use uuid::Uuid;

use crate::benefit::dto::{BenefitGrantRequest, BenefitGrantResult};
use crate::benefit::handler::BenefitHandler;
use crate::error::{BadgeError, Result};
use crate::models::{BenefitType, GrantStatus};

/// 收货地址信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShippingAddress {
    /// 收货人姓名
    pub recipient_name: String,
    /// 收货人手机号
    pub phone: String,
    /// 省份
    pub province: String,
    /// 城市
    pub city: String,
    /// 区/县
    pub district: String,
    /// 详细地址
    pub address: String,
    /// 邮编（可选）
    pub postal_code: Option<String>,
}

/// 实物配置结构
///
/// 配置需要包含 `sku_id` 和收货地址信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PhysicalConfig {
    /// SKU ID（必填）
    pub sku_id: String,
    /// SKU 名称（可选，用于显示）
    pub sku_name: Option<String>,
    /// 发放数量（默认 1）
    #[serde(default = "default_quantity")]
    pub quantity: i32,
    /// 收货地址（可以在配置中预设，也可以通过 metadata 传递）
    pub shipping_address: Option<ShippingAddress>,
}

fn default_quantity() -> i32 {
    1
}

/// Kafka 发货消息
///
/// 发送到物流系统的消息格式
#[derive(Debug, Clone, Serialize, Deserialize)]
struct PhysicalShipmentMessage {
    /// 消息 ID
    message_id: String,
    /// 发放流水号
    grant_no: String,
    /// 用户 ID
    user_id: String,
    /// SKU ID
    sku_id: String,
    /// SKU 名称
    sku_name: Option<String>,
    /// 数量
    quantity: i32,
    /// 收货地址
    shipping_address: ShippingAddress,
    /// 创建时间
    created_at: String,
}

/// 实物处理器
///
/// 异步发放实物奖品。发放时只记录请求并发送 Kafka 消息，
/// 实际物流状态通过回调或轮询更新。不支持撤销操作。
pub struct PhysicalHandler {
    /// Kafka broker 地址（实际实现时使用）
    #[allow(dead_code)]
    kafka_brokers: String,
    /// 发货消息 topic
    #[allow(dead_code)]
    shipment_topic: String,
}

impl PhysicalHandler {
    /// 创建实物处理器
    pub fn new(kafka_brokers: impl Into<String>, shipment_topic: impl Into<String>) -> Self {
        Self {
            kafka_brokers: kafka_brokers.into(),
            shipment_topic: shipment_topic.into(),
        }
    }
}

impl Default for PhysicalHandler {
    fn default() -> Self {
        Self::new("localhost:9092", "physical_shipment")
    }
}

impl PhysicalHandler {

    /// 解析实物配置
    fn parse_config(&self, config: &Value) -> Result<PhysicalConfig> {
        serde_json::from_value(config.clone()).map_err(|e| {
            BadgeError::Validation(format!("实物配置解析失败: {}", e))
        })
    }

    /// 从 metadata 中提取收货地址
    ///
    /// 如果配置中没有地址，尝试从 metadata 中获取
    fn extract_address(
        &self,
        config: &PhysicalConfig,
        metadata: Option<&Value>,
    ) -> Result<ShippingAddress> {
        // 优先使用配置中的地址
        if let Some(addr) = &config.shipping_address {
            return Ok(addr.clone());
        }

        // 尝试从 metadata 中提取
        if let Some(meta) = metadata
            && let Some(addr_value) = meta.get("shipping_address")
        {
            return serde_json::from_value(addr_value.clone()).map_err(|e| {
                BadgeError::Validation(format!("metadata 中收货地址解析失败: {}", e))
            });
        }

        Err(BadgeError::Validation(
            "缺少收货地址信息，请在配置或 metadata 中提供 shipping_address".into(),
        ))
    }

    /// 发送 Kafka 消息（stub 实现）
    ///
    /// TODO: 替换为实际的 Kafka producer 调用
    async fn send_shipment_message(&self, message: &PhysicalShipmentMessage) -> Result<()> {
        debug!(
            message_id = %message.message_id,
            grant_no = %message.grant_no,
            sku_id = %message.sku_id,
            topic = %self.shipment_topic,
            "模拟发送 Kafka 发货消息"
        );

        // 模拟消息发送成功
        // 实际实现需要：
        // 1. 序列化消息
        // 2. 发送到 Kafka topic
        // 3. 等待确认（可选同步/异步）
        Ok(())
    }
}

#[async_trait]
impl BenefitHandler for PhysicalHandler {
    fn benefit_type(&self) -> BenefitType {
        BenefitType::Physical
    }

    #[instrument(
        skip(self, request),
        fields(
            grant_no = %request.grant_no,
            user_id = %request.user_id,
            benefit_type = "physical"
        )
    )]
    async fn grant(&self, request: BenefitGrantRequest) -> Result<BenefitGrantResult> {
        // 解析配置
        let config = match self.parse_config(&request.benefit_config) {
            Ok(c) => c,
            Err(e) => {
                error!(error = %e, "实物配置解析失败");
                return Ok(BenefitGrantResult::failed(&request.grant_no, e.to_string()));
            }
        };

        // 提取收货地址
        let shipping_address =
            match self.extract_address(&config, request.metadata.as_ref()) {
                Ok(addr) => addr,
                Err(e) => {
                    error!(error = %e, "收货地址获取失败");
                    return Ok(BenefitGrantResult::failed(&request.grant_no, e.to_string()));
                }
            };

        info!(
            sku_id = %config.sku_id,
            quantity = config.quantity,
            recipient = %shipping_address.recipient_name,
            "开始处理实物发放"
        );

        // 构建发货消息
        let message_id = Uuid::new_v4().to_string();
        let message = PhysicalShipmentMessage {
            message_id: message_id.clone(),
            grant_no: request.grant_no.clone(),
            user_id: request.user_id.clone(),
            sku_id: config.sku_id.clone(),
            sku_name: config.sku_name.clone(),
            quantity: config.quantity,
            shipping_address: shipping_address.clone(),
            created_at: chrono::Utc::now().to_rfc3339(),
        };

        // 发送 Kafka 消息
        match self.send_shipment_message(&message).await {
            Ok(()) => {
                info!(
                    message_id = %message_id,
                    "实物发货消息已发送"
                );

                // 构建处理中结果（异步发放）
                let result = BenefitGrantResult::processing(&request.grant_no)
                    .with_external_ref(&message_id)
                    .with_message("发货请求已提交，等待物流处理")
                    .with_payload(serde_json::json!({
                        "message_id": message_id,
                        "sku_id": config.sku_id,
                        "sku_name": config.sku_name,
                        "quantity": config.quantity,
                        "recipient_name": shipping_address.recipient_name,
                        "shipping_city": format!("{} {}", shipping_address.province, shipping_address.city),
                    }));

                Ok(result)
            }
            Err(e) => {
                error!(error = %e, "发货消息发送失败");
                Ok(BenefitGrantResult::failed(&request.grant_no, e.to_string()))
            }
        }
    }

    async fn query_status(&self, grant_no: &str) -> Result<GrantStatus> {
        // 实物是异步发放，状态需要从物流系统查询
        // TODO: 实现实际的状态查询逻辑
        warn!(
            grant_no = %grant_no,
            "实物发放状态查询暂未实现，返回 Processing"
        );
        Ok(GrantStatus::Processing)
    }

    // 使用默认的 revoke 实现（返回不支持错误）
    // 实物奖品一旦发货无法撤销

    fn validate_config(&self, config: &Value) -> Result<()> {
        // 验证必要字段存在
        let physical_config = self.parse_config(config)?;

        if physical_config.sku_id.is_empty() {
            return Err(BadgeError::Validation("sku_id 不能为空".into()));
        }

        if physical_config.quantity <= 0 {
            return Err(BadgeError::Validation("quantity 必须大于 0".into()));
        }

        // 收货地址可以在配置中或运行时通过 metadata 提供
        // 这里只做基本校验
        if let Some(addr) = &physical_config.shipping_address {
            self.validate_address(addr)?;
        }

        Ok(())
    }

    fn description(&self) -> &'static str {
        "Physical Handler - 实物异步发放（Kafka）"
    }
}

impl PhysicalHandler {
    /// 验证收货地址
    fn validate_address(&self, address: &ShippingAddress) -> Result<()> {
        if address.recipient_name.is_empty() {
            return Err(BadgeError::Validation(
                "收货人姓名 (recipient_name) 不能为空".into(),
            ));
        }

        if address.phone.is_empty() {
            return Err(BadgeError::Validation(
                "收货人手机号 (phone) 不能为空".into(),
            ));
        }

        // 简单的手机号格式校验（中国大陆 11 位）
        if !address.phone.chars().all(|c| c.is_ascii_digit()) || address.phone.len() != 11 {
            return Err(BadgeError::Validation(
                "手机号格式不正确，应为 11 位数字".into(),
            ));
        }

        if address.province.is_empty() || address.city.is_empty() || address.district.is_empty() {
            return Err(BadgeError::Validation(
                "省/市/区 不能为空".into(),
            ));
        }

        if address.address.is_empty() {
            return Err(BadgeError::Validation("详细地址不能为空".into()));
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn create_handler() -> PhysicalHandler {
        PhysicalHandler::default()
    }

    fn create_valid_address() -> Value {
        json!({
            "recipient_name": "张三",
            "phone": "13800138000",
            "province": "北京市",
            "city": "北京市",
            "district": "朝阳区",
            "address": "某某街道 123 号"
        })
    }

    #[test]
    fn test_parse_config_success() {
        let handler = create_handler();
        let config = json!({
            "sku_id": "SKU001",
            "sku_name": "限量徽章",
            "quantity": 1,
            "shipping_address": create_valid_address()
        });

        let parsed = handler.parse_config(&config).unwrap();
        assert_eq!(parsed.sku_id, "SKU001");
        assert_eq!(parsed.sku_name, Some("限量徽章".to_string()));
        assert_eq!(parsed.quantity, 1);
        assert!(parsed.shipping_address.is_some());
    }

    #[test]
    fn test_parse_config_default_quantity() {
        let handler = create_handler();
        let config = json!({
            "sku_id": "SKU001"
        });

        let parsed = handler.parse_config(&config).unwrap();
        assert_eq!(parsed.quantity, 1);
    }

    #[test]
    fn test_validate_config_success() {
        let handler = create_handler();
        let config = json!({
            "sku_id": "SKU001",
            "shipping_address": create_valid_address()
        });

        assert!(handler.validate_config(&config).is_ok());
    }

    #[test]
    fn test_validate_config_missing_sku_id() {
        let handler = create_handler();
        let config = json!({});

        let result = handler.validate_config(&config);
        assert!(result.is_err());
    }

    #[test]
    fn test_validate_config_empty_sku_id() {
        let handler = create_handler();
        let config = json!({
            "sku_id": ""
        });

        let result = handler.validate_config(&config);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("sku_id"));
    }

    #[test]
    fn test_validate_config_invalid_quantity() {
        let handler = create_handler();
        let config = json!({
            "sku_id": "SKU001",
            "quantity": 0
        });

        let result = handler.validate_config(&config);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("quantity"));
    }

    #[test]
    fn test_validate_address_success() {
        let handler = create_handler();
        let address = ShippingAddress {
            recipient_name: "张三".to_string(),
            phone: "13800138000".to_string(),
            province: "北京市".to_string(),
            city: "北京市".to_string(),
            district: "朝阳区".to_string(),
            address: "某某街道 123 号".to_string(),
            postal_code: Some("100000".to_string()),
        };

        assert!(handler.validate_address(&address).is_ok());
    }

    #[test]
    fn test_validate_address_empty_recipient() {
        let handler = create_handler();
        let address = ShippingAddress {
            recipient_name: "".to_string(),
            phone: "13800138000".to_string(),
            province: "北京市".to_string(),
            city: "北京市".to_string(),
            district: "朝阳区".to_string(),
            address: "某某街道 123 号".to_string(),
            postal_code: None,
        };

        let result = handler.validate_address(&address);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("recipient_name"));
    }

    #[test]
    fn test_validate_address_invalid_phone() {
        let handler = create_handler();
        let address = ShippingAddress {
            recipient_name: "张三".to_string(),
            phone: "1234567".to_string(), // 不是 11 位
            province: "北京市".to_string(),
            city: "北京市".to_string(),
            district: "朝阳区".to_string(),
            address: "某某街道 123 号".to_string(),
            postal_code: None,
        };

        let result = handler.validate_address(&address);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("手机号"));
    }

    #[tokio::test]
    async fn test_grant_success() {
        let handler = create_handler();
        let request = BenefitGrantRequest::new(
            "grant-001",
            "user-123",
            1,
            json!({
                "sku_id": "SKU001",
                "sku_name": "限量徽章",
                "shipping_address": create_valid_address()
            }),
        );

        let result = handler.grant(request).await.unwrap();

        // 实物是异步发放，返回 Processing 状态
        assert!(result.is_processing());
        assert_eq!(result.grant_no, "grant-001");
        assert!(result.external_ref.is_some()); // message_id
        assert!(result.message.is_some());
        assert!(result.payload.is_some());

        // 验证 payload
        let payload = result.payload.unwrap();
        assert!(payload.get("message_id").is_some());
        assert_eq!(payload.get("sku_id").unwrap(), "SKU001");
    }

    #[tokio::test]
    async fn test_grant_with_address_in_metadata() {
        let handler = create_handler();
        let request = BenefitGrantRequest::new(
            "grant-002",
            "user-123",
            1,
            json!({
                "sku_id": "SKU001"
            }),
        )
        .with_metadata(json!({
            "shipping_address": create_valid_address()
        }));

        let result = handler.grant(request).await.unwrap();

        // 应该能从 metadata 获取地址，成功处理
        assert!(result.is_processing());
    }

    #[tokio::test]
    async fn test_grant_missing_address() {
        let handler = create_handler();
        let request = BenefitGrantRequest::new(
            "grant-003",
            "user-123",
            1,
            json!({
                "sku_id": "SKU001"
                // 没有收货地址
            }),
        );

        let result = handler.grant(request).await.unwrap();

        // 缺少收货地址，应该失败
        assert!(!result.is_success());
        assert!(!result.is_processing());
        assert_eq!(result.status, GrantStatus::Failed);
        assert!(result.message.is_some());
        assert!(result.message.unwrap().contains("收货地址"));
    }

    #[tokio::test]
    async fn test_query_status() {
        let handler = create_handler();
        let status = handler.query_status("grant-001").await.unwrap();

        // 实物是异步发放，状态应为 Processing
        assert_eq!(status, GrantStatus::Processing);
    }

    #[tokio::test]
    async fn test_revoke_not_supported() {
        let handler = create_handler();
        let result = handler.revoke("grant-001").await;

        // 实物不支持撤销
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("不支持撤销"));
    }

    #[test]
    fn test_benefit_type() {
        let handler = create_handler();
        assert_eq!(handler.benefit_type(), BenefitType::Physical);
    }

    #[test]
    fn test_description() {
        let handler = create_handler();
        assert!(handler.description().contains("Physical"));
        assert!(handler.description().contains("Kafka"));
    }
}
