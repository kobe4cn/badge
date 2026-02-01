//! Mock 通知服务
//!
//! 模拟通知发送服务，用于测试通知功能。

use axum::{
    Json, Router,
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
    routing::{get, post},
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

/// 通知记录
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NotificationRecord {
    pub id: String,
    pub user_id: String,
    pub notification_type: String,
    pub title: String,
    pub body: String,
    pub channels: Vec<String>,
    pub status: String,
    pub sent_at: chrono::DateTime<chrono::Utc>,
    pub metadata: serde_json::Value,
}

/// 发送通知请求
#[derive(Debug, Deserialize)]
pub struct SendNotificationRequest {
    pub user_id: String,
    pub notification_type: String,
    pub title: String,
    pub body: String,
    pub channels: Vec<String>,
    #[serde(default)]
    pub metadata: serde_json::Value,
}

/// 通知服务状态
#[derive(Default)]
pub struct NotificationServiceState {
    /// 发送的通知记录
    notifications: RwLock<HashMap<String, NotificationRecord>>,
    /// 用户通知映射
    user_notifications: RwLock<HashMap<String, Vec<String>>>,
    /// 是否模拟失败
    simulate_failure: RwLock<bool>,
    /// 失败的渠道
    failing_channels: RwLock<Vec<String>>,
}

impl NotificationServiceState {
    pub fn new() -> Self {
        Self::default()
    }

    /// 设置模拟失败
    pub async fn set_simulate_failure(&self, should_fail: bool) {
        *self.simulate_failure.write().await = should_fail;
    }

    /// 设置失败的渠道
    pub async fn set_failing_channels(&self, channels: Vec<String>) {
        *self.failing_channels.write().await = channels;
    }

    /// 获取用户的所有通知
    pub async fn get_user_notifications(&self, user_id: &str) -> Vec<NotificationRecord> {
        let user_notifs = self.user_notifications.read().await;
        let notifications = self.notifications.read().await;

        user_notifs
            .get(user_id)
            .map(|ids| {
                ids.iter()
                    .filter_map(|id| notifications.get(id).cloned())
                    .collect()
            })
            .unwrap_or_default()
    }

    /// 清空所有通知
    pub async fn clear(&self) {
        self.notifications.write().await.clear();
        self.user_notifications.write().await.clear();
    }
}

/// 创建通知服务路由
pub fn notification_routes() -> Router<Arc<NotificationServiceState>> {
    Router::new()
        .route("/notifications", post(send_notification))
        .route("/notifications/:id", get(get_notification))
        .route("/users/:user_id/notifications", get(get_user_notifications))
        .route("/admin/notifications/clear", post(clear_notifications))
        .route("/admin/notifications/simulate-failure", post(set_failure))
}

/// 发送通知
async fn send_notification(
    State(state): State<Arc<NotificationServiceState>>,
    Json(req): Json<SendNotificationRequest>,
) -> impl IntoResponse {
    // 检查是否模拟失败
    if *state.simulate_failure.read().await {
        return (
            StatusCode::SERVICE_UNAVAILABLE,
            Json(serde_json::json!({
                "error": "Service temporarily unavailable"
            })),
        );
    }

    let failing_channels = state.failing_channels.read().await;
    let successful_channels: Vec<String> = req
        .channels
        .iter()
        .filter(|c| !failing_channels.contains(c))
        .cloned()
        .collect();

    let id = uuid::Uuid::now_v7().to_string();
    let record = NotificationRecord {
        id: id.clone(),
        user_id: req.user_id.clone(),
        notification_type: req.notification_type,
        title: req.title,
        body: req.body,
        channels: successful_channels.clone(),
        status: if successful_channels.is_empty() {
            "failed".to_string()
        } else if successful_channels.len() < req.channels.len() {
            "partial".to_string()
        } else {
            "sent".to_string()
        },
        sent_at: chrono::Utc::now(),
        metadata: req.metadata,
    };

    // 存储通知
    state
        .notifications
        .write()
        .await
        .insert(id.clone(), record.clone());

    // 更新用户通知映射
    state
        .user_notifications
        .write()
        .await
        .entry(req.user_id)
        .or_default()
        .push(id);

    (StatusCode::OK, Json(serde_json::to_value(record).unwrap()))
}

/// 获取通知详情
async fn get_notification(
    State(state): State<Arc<NotificationServiceState>>,
    Path(id): Path<String>,
) -> impl IntoResponse {
    match state.notifications.read().await.get(&id) {
        Some(record) => (StatusCode::OK, Json(serde_json::to_value(record).unwrap())),
        None => (
            StatusCode::NOT_FOUND,
            Json(serde_json::json!({ "error": "Notification not found" })),
        ),
    }
}

/// 获取用户通知列表
async fn get_user_notifications(
    State(state): State<Arc<NotificationServiceState>>,
    Path(user_id): Path<String>,
) -> impl IntoResponse {
    let notifications = state.get_user_notifications(&user_id).await;
    Json(notifications)
}

/// 清空通知（测试用）
async fn clear_notifications(
    State(state): State<Arc<NotificationServiceState>>,
) -> impl IntoResponse {
    state.clear().await;
    StatusCode::OK
}

/// 设置模拟失败（测试用）
async fn set_failure(
    State(state): State<Arc<NotificationServiceState>>,
    Json(req): Json<serde_json::Value>,
) -> impl IntoResponse {
    if let Some(should_fail) = req.get("simulate_failure").and_then(|v| v.as_bool()) {
        state.set_simulate_failure(should_fail).await;
    }
    if let Some(channels) = req.get("failing_channels").and_then(|v| v.as_array()) {
        let channels: Vec<String> = channels
            .iter()
            .filter_map(|v| v.as_str().map(String::from))
            .collect();
        state.set_failing_channels(channels).await;
    }
    StatusCode::OK
}
