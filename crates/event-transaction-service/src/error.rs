//! 订单事件服务专用错误类型
//!
//! 在共享库 BadgeError 基础上定义本服务特有的错误变体，
//! 除了与行为事件服务相同的"已处理/不支持/规则引擎/徽章发放"外，
//! 额外增加"徽章撤销失败"变体，用于退款/取消场景的错误区分。

use badge_shared::error::BadgeError;

/// 订单事件处理错误
#[derive(Debug, thiserror::Error)]
pub enum TransactionError {
    /// Kafka 重复投递时通过 Redis 幂等键识别出已处理的事件，直接跳过
    #[error("事件已处理: {event_id}")]
    AlreadyProcessed { event_id: String },

    /// 本服务只处理交易类事件，收到其他类型说明路由配置有误
    #[error("不支持的事件类型: {event_type}")]
    UnsupportedEventType { event_type: String },

    /// 规则引擎 gRPC 调用失败（网络、超时或服务端错误）
    #[error("规则引擎调用失败: {0}")]
    RuleEngineError(String),

    /// 徽章管理 gRPC 调用失败
    #[error("徽章发放失败: {0}")]
    BadgeGrantError(String),

    /// 退款/取消时撤销徽章的 gRPC 调用失败
    #[error("徽章撤销失败: {0}")]
    BadgeRevokeError(String),

    /// 透传共享库错误，避免在每个 match 分支手动转换
    #[error(transparent)]
    Shared(#[from] BadgeError),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_display() {
        let err = TransactionError::AlreadyProcessed {
            event_id: "evt-001".to_string(),
        };
        assert_eq!(err.to_string(), "事件已处理: evt-001");

        let err = TransactionError::UnsupportedEventType {
            event_type: "CHECK_IN".to_string(),
        };
        assert_eq!(err.to_string(), "不支持的事件类型: CHECK_IN");

        let err = TransactionError::RuleEngineError("连接超时".to_string());
        assert_eq!(err.to_string(), "规则引擎调用失败: 连接超时");

        let err = TransactionError::BadgeGrantError("库存不足".to_string());
        assert_eq!(err.to_string(), "徽章发放失败: 库存不足");

        let err = TransactionError::BadgeRevokeError("用户不存在".to_string());
        assert_eq!(err.to_string(), "徽章撤销失败: 用户不存在");

        let shared_err = BadgeError::Kafka("broker 不可达".to_string());
        let err = TransactionError::Shared(shared_err);
        assert_eq!(err.to_string(), "Kafka 错误: broker 不可达");
    }
}
