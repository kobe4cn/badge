//! Mock 权益服务
//!
//! 模拟外部权益发放服务，用于测试权益同步功能。

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

/// 权益发放记录
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BenefitGrantRecord {
    pub grant_id: String,
    pub user_id: String,
    pub benefit_type: String,
    pub benefit_code: String,
    pub value: serde_json::Value,
    pub status: String,
    pub granted_at: chrono::DateTime<chrono::Utc>,
    pub expires_at: Option<chrono::DateTime<chrono::Utc>>,
    pub external_ref: Option<String>,
}

/// 发放权益请求
#[derive(Debug, Deserialize)]
pub struct GrantBenefitRequest {
    pub user_id: String,
    pub benefit_type: String,
    pub benefit_code: String,
    pub value: serde_json::Value,
    #[serde(default)]
    pub validity_days: Option<i32>,
    #[serde(default)]
    pub idempotency_key: Option<String>,
}

/// 权益服务状态
#[derive(Default)]
pub struct BenefitServiceState {
    /// 发放的权益记录
    grants: RwLock<HashMap<String, BenefitGrantRecord>>,
    /// 用户权益映射
    user_benefits: RwLock<HashMap<String, Vec<String>>>,
    /// 幂等性缓存
    idempotency_cache: RwLock<HashMap<String, String>>,
    /// 是否模拟失败
    simulate_failure: RwLock<bool>,
    /// 失败的权益类型
    failing_types: RwLock<Vec<String>>,
    /// 库存管理
    stock: RwLock<HashMap<String, i32>>,
}

impl BenefitServiceState {
    pub fn new() -> Self {
        Self::default()
    }

    /// 设置模拟失败
    pub async fn set_simulate_failure(&self, should_fail: bool) {
        *self.simulate_failure.write().await = should_fail;
    }

    /// 设置失败的权益类型
    pub async fn set_failing_types(&self, types: Vec<String>) {
        *self.failing_types.write().await = types;
    }

    /// 设置库存
    pub async fn set_stock(&self, benefit_code: &str, count: i32) {
        self.stock
            .write()
            .await
            .insert(benefit_code.to_string(), count);
    }

    /// 获取用户的所有权益
    pub async fn get_user_benefits(&self, user_id: &str) -> Vec<BenefitGrantRecord> {
        let user_benefits = self.user_benefits.read().await;
        let grants = self.grants.read().await;

        user_benefits
            .get(user_id)
            .map(|ids| {
                ids.iter()
                    .filter_map(|id| grants.get(id).cloned())
                    .collect()
            })
            .unwrap_or_default()
    }

    /// 清空所有权益
    pub async fn clear(&self) {
        self.grants.write().await.clear();
        self.user_benefits.write().await.clear();
        self.idempotency_cache.write().await.clear();
        self.stock.write().await.clear();
    }
}

/// 创建权益服务路由
pub fn benefit_routes() -> Router<Arc<BenefitServiceState>> {
    Router::new()
        .route("/benefits/grant", post(grant_benefit))
        .route("/benefits/:id", get(get_benefit))
        .route("/users/:user_id/benefits", get(get_user_benefits))
        .route("/benefits/:code/stock", get(get_stock))
        .route("/admin/benefits/clear", post(clear_benefits))
        .route("/admin/benefits/simulate-failure", post(set_failure))
        .route("/admin/benefits/stock", post(set_stock))
}

/// 发放权益
async fn grant_benefit(
    State(state): State<Arc<BenefitServiceState>>,
    Json(req): Json<GrantBenefitRequest>,
) -> impl IntoResponse {
    // 检查幂等性：如果请求携带幂等键且已存在对应记录，直接返回已有结果
    if let Some(ref key) = req.idempotency_key
        && let Some(existing_id) = state.idempotency_cache.read().await.get(key)
        && let Some(record) = state.grants.read().await.get(existing_id)
    {
        return (StatusCode::OK, Json(serde_json::to_value(record).unwrap()));
    }

    // 检查是否模拟失败
    if *state.simulate_failure.read().await {
        return (
            StatusCode::SERVICE_UNAVAILABLE,
            Json(serde_json::json!({
                "error": "Service temporarily unavailable"
            })),
        );
    }

    // 检查权益类型是否会失败
    if state.failing_types.read().await.contains(&req.benefit_type) {
        return (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({
                "error": format!("Benefit type {} is not available", req.benefit_type)
            })),
        );
    }

    // 检查库存
    {
        let mut stock = state.stock.write().await;
        if let Some(count) = stock.get_mut(&req.benefit_code) {
            if *count <= 0 {
                return (
                    StatusCode::CONFLICT,
                    Json(serde_json::json!({
                        "error": "Out of stock"
                    })),
                );
            }
            *count -= 1;
        }
    }

    let grant_id = uuid::Uuid::now_v7().to_string();
    let external_ref = format!("EXT-{}", uuid::Uuid::new_v4());

    let expires_at = req
        .validity_days
        .map(|days| chrono::Utc::now() + chrono::Duration::days(days as i64));

    let record = BenefitGrantRecord {
        grant_id: grant_id.clone(),
        user_id: req.user_id.clone(),
        benefit_type: req.benefit_type,
        benefit_code: req.benefit_code,
        value: req.value,
        status: "granted".to_string(),
        granted_at: chrono::Utc::now(),
        expires_at,
        external_ref: Some(external_ref),
    };

    // 存储权益
    state
        .grants
        .write()
        .await
        .insert(grant_id.clone(), record.clone());

    // 更新用户权益映射
    state
        .user_benefits
        .write()
        .await
        .entry(req.user_id)
        .or_default()
        .push(grant_id.clone());

    // 存储幂等性缓存
    if let Some(key) = req.idempotency_key {
        state.idempotency_cache.write().await.insert(key, grant_id);
    }

    (StatusCode::OK, Json(serde_json::to_value(record).unwrap()))
}

/// 获取权益详情
async fn get_benefit(
    State(state): State<Arc<BenefitServiceState>>,
    Path(id): Path<String>,
) -> impl IntoResponse {
    match state.grants.read().await.get(&id) {
        Some(record) => (StatusCode::OK, Json(serde_json::to_value(record).unwrap())),
        None => (
            StatusCode::NOT_FOUND,
            Json(serde_json::json!({ "error": "Benefit grant not found" })),
        ),
    }
}

/// 获取用户权益列表
async fn get_user_benefits(
    State(state): State<Arc<BenefitServiceState>>,
    Path(user_id): Path<String>,
) -> impl IntoResponse {
    let benefits = state.get_user_benefits(&user_id).await;
    Json(benefits)
}

/// 获取库存
async fn get_stock(
    State(state): State<Arc<BenefitServiceState>>,
    Path(code): Path<String>,
) -> impl IntoResponse {
    let stock = state.stock.read().await.get(&code).copied().unwrap_or(-1);
    Json(serde_json::json!({ "code": code, "stock": stock }))
}

/// 清空权益（测试用）
async fn clear_benefits(State(state): State<Arc<BenefitServiceState>>) -> impl IntoResponse {
    state.clear().await;
    StatusCode::OK
}

/// 设置模拟失败（测试用）
async fn set_failure(
    State(state): State<Arc<BenefitServiceState>>,
    Json(req): Json<serde_json::Value>,
) -> impl IntoResponse {
    if let Some(should_fail) = req.get("simulate_failure").and_then(|v| v.as_bool()) {
        state.set_simulate_failure(should_fail).await;
    }
    if let Some(types) = req.get("failing_types").and_then(|v| v.as_array()) {
        let types: Vec<String> = types
            .iter()
            .filter_map(|v| v.as_str().map(String::from))
            .collect();
        state.set_failing_types(types).await;
    }
    StatusCode::OK
}

/// 设置库存（测试用）
async fn set_stock(
    State(state): State<Arc<BenefitServiceState>>,
    Json(req): Json<serde_json::Value>,
) -> impl IntoResponse {
    if let (Some(code), Some(count)) = (
        req.get("code").and_then(|v| v.as_str()),
        req.get("count").and_then(|v| v.as_i64()),
    ) {
        state.set_stock(code, count as i32).await;
    }
    StatusCode::OK
}
