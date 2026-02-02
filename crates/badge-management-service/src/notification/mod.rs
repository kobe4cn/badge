//! 通知服务模块
//!
//! 提供徽章系统的通知发送功能，支持多渠道异步发送。
//!
//! ## 功能特性
//!
//! - **多渠道支持**：App Push、SMS、Email、微信等
//! - **模板引擎**：支持变量替换的通知模板
//! - **异步发送**：不阻塞主业务流程
//! - **部分失败容忍**：单渠道失败不影响其他渠道
//! - **发送记录**：通过 Kafka 发送通知事件供下游消费
//!
//! ## 使用示例
//!
//! ```ignore
//! use notification::{NotificationService, Notification, NotificationType};
//!
//! let service = NotificationService::new(producer, template_engine);
//!
//! // 发送徽章获取通知
//! let notification = Notification::badge_granted("user-123", badge_info);
//! service.send(notification).await?;
//! ```

pub mod channels;
pub mod sender;
pub mod service;
pub mod template;
pub mod types;

pub use channels::{
    AppPushChannel, EmailChannel, NotificationChannel, SmsChannel, WeChatChannel,
};
pub use sender::NotificationSender;
pub use service::NotificationService;
pub use template::TemplateEngine;
pub use types::{
    ChannelResult, Notification, NotificationBuilder, NotificationContext, NotificationResult,
    SendStatus,
};
