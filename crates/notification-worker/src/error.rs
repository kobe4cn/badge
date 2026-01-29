//! 通知服务错误类型
//!
//! 定义通知发送、模板渲染和消息反序列化等场景的错误分类，
//! 便于上层根据错误类型决定重试或丢弃策略。

use thiserror::Error;

#[derive(Debug, Error)]
pub enum NotificationError {
    #[error("通知发送失败: 渠道={channel}, 原因={reason}")]
    SendFailed { channel: String, reason: String },

    #[error("通知模板未找到: {template_id}")]
    TemplateNotFound { template_id: String },

    #[error("通知反序列化失败: {0}")]
    DeserializationFailed(String),

    #[error(transparent)]
    Shared(#[from] badge_shared::error::BadgeError),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_display() {
        let send_err = NotificationError::SendFailed {
            channel: "SMS".to_string(),
            reason: "网络超时".to_string(),
        };
        assert_eq!(
            send_err.to_string(),
            "通知发送失败: 渠道=SMS, 原因=网络超时"
        );

        let template_err = NotificationError::TemplateNotFound {
            template_id: "tpl-001".to_string(),
        };
        assert_eq!(template_err.to_string(), "通知模板未找到: tpl-001");

        let deser_err = NotificationError::DeserializationFailed("invalid JSON".to_string());
        assert_eq!(deser_err.to_string(), "通知反序列化失败: invalid JSON");
    }
}
