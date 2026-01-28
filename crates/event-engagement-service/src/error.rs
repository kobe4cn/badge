//! 行为事件服务专用错误类型
//!
//! 在共享库 BadgeError 基础上定义本服务特有的错误变体，
//! 使上层可以精确区分"已处理/不支持/规则引擎/徽章发放"等不同失败原因，
//! 而无需在共享库中为每个服务追加变体。

use badge_shared::error::BadgeError;

/// 行为事件处理错误
#[derive(Debug, thiserror::Error)]
pub enum EngagementError {
    /// Kafka 重复投递时通过 Redis 幂等键识别出已处理的事件，直接跳过
    #[error("事件已处理: {event_id}")]
    AlreadyProcessed { event_id: String },

    /// 本服务只处理行为类事件，收到其他类型说明路由配置有误
    #[error("不支持的事件类型: {event_type}")]
    UnsupportedEventType { event_type: String },

    /// 规则引擎 gRPC 调用失败（网络、超时或服务端错误）
    #[error("规则引擎调用失败: {0}")]
    RuleEngineError(String),

    /// 徽章管理 gRPC 调用失败
    #[error("徽章发放失败: {0}")]
    BadgeGrantError(String),

    /// 透传共享库错误，避免在每个 match 分支手动转换
    #[error(transparent)]
    Shared(#[from] BadgeError),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_display() {
        let err = EngagementError::AlreadyProcessed {
            event_id: "evt-001".to_string(),
        };
        assert_eq!(err.to_string(), "事件已处理: evt-001");

        let err = EngagementError::UnsupportedEventType {
            event_type: "PURCHASE".to_string(),
        };
        assert_eq!(err.to_string(), "不支持的事件类型: PURCHASE");

        let err = EngagementError::RuleEngineError("连接超时".to_string());
        assert_eq!(err.to_string(), "规则引擎调用失败: 连接超时");

        let err = EngagementError::BadgeGrantError("库存不足".to_string());
        assert_eq!(err.to_string(), "徽章发放失败: 库存不足");

        let shared_err = BadgeError::Kafka("broker 不可达".to_string());
        let err = EngagementError::Shared(shared_err);
        assert_eq!(err.to_string(), "Kafka 错误: broker 不可达");
    }
}
