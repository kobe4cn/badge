//! Mock 订单服务
//!
//! 提供模拟订单的 REST API，用于开发和测试环境。
//! 使用 Axum 框架实现 RESTful 接口，比 gRPC 更灵活。

use axum::{
    Json, Router,
    extract::{Path, Query, State},
    http::StatusCode,
    routing::{get, post},
};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tracing::{info, warn};
use uuid::Uuid;

use crate::models::{MockOrder, OrderItem, OrderStatus};
use crate::store::MemoryStore;

/// 订单服务状态
///
/// 持有订单内存存储的共享引用，供所有路由处理器使用
#[derive(Clone)]
pub struct OrderServiceState {
    pub orders: MemoryStore<MockOrder>,
}

impl OrderServiceState {
    /// 创建新的订单服务状态
    pub fn new() -> Self {
        Self {
            orders: MemoryStore::new(),
        }
    }
}

impl Default for OrderServiceState {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// 请求/响应 DTO
// ============================================================================

/// 创建订单请求
#[derive(Debug, Deserialize)]
pub struct CreateOrderRequest {
    pub user_id: String,
    pub items: Vec<OrderItemRequest>,
}

/// 订单项请求
#[derive(Debug, Deserialize)]
pub struct OrderItemRequest {
    pub product_id: String,
    pub product_name: String,
    pub quantity: i32,
    pub unit_price: f64,
    pub category: String,
}

/// 更新订单状态请求
#[derive(Debug, Deserialize)]
pub struct UpdateStatusRequest {
    pub status: OrderStatus,
}

/// 订单列表查询参数
#[derive(Debug, Deserialize)]
pub struct ListOrdersQuery {
    pub page: Option<u32>,
    pub page_size: Option<u32>,
}

/// 单个订单响应
#[derive(Debug, Serialize, Deserialize)]
pub struct OrderResponse {
    pub order: MockOrder,
}

/// 订单列表响应
#[derive(Debug, Serialize, Deserialize)]
pub struct OrderListResponse {
    pub orders: Vec<MockOrder>,
    pub total: usize,
}

// ============================================================================
// 路由定义
// ============================================================================

/// 构建订单服务路由
///
/// 包含订单 CRUD 和按用户查询的端点
pub fn order_routes() -> Router<Arc<OrderServiceState>> {
    Router::new()
        .route("/orders/{order_id}", get(get_order))
        .route("/orders", get(list_orders))
        .route("/orders", post(create_order))
        .route("/orders/{order_id}/status", post(update_order_status))
        .route("/users/{user_id}/orders", get(list_user_orders))
}

// ============================================================================
// 路由处理器
// ============================================================================

/// 获取订单详情
///
/// GET /orders/:order_id
async fn get_order(
    State(state): State<Arc<OrderServiceState>>,
    Path(order_id): Path<String>,
) -> Result<Json<OrderResponse>, StatusCode> {
    info!(order_id = %order_id, "获取订单详情");

    state
        .orders
        .get(&order_id)
        .map(|order| Json(OrderResponse { order }))
        .ok_or_else(|| {
            warn!(order_id = %order_id, "订单不存在");
            StatusCode::NOT_FOUND
        })
}

/// 列出所有订单（支持分页）
///
/// GET /orders?page=1&page_size=10
async fn list_orders(
    State(state): State<Arc<OrderServiceState>>,
    Query(query): Query<ListOrdersQuery>,
) -> Json<OrderListResponse> {
    let page = query.page.unwrap_or(1).max(1) as usize;
    let page_size = query.page_size.unwrap_or(10).min(100) as usize;

    let all_orders = state.orders.list();
    let total = all_orders.len();

    // 分页处理：跳过前 (page-1)*page_size 条记录
    let orders: Vec<MockOrder> = all_orders
        .into_iter()
        .skip((page - 1) * page_size)
        .take(page_size)
        .collect();

    info!(page, page_size, total, returned = orders.len(), "列出订单");

    Json(OrderListResponse { orders, total })
}

/// 创建订单
///
/// POST /orders
async fn create_order(
    State(state): State<Arc<OrderServiceState>>,
    Json(req): Json<CreateOrderRequest>,
) -> (StatusCode, Json<OrderResponse>) {
    // 转换请求中的订单项
    let items: Vec<OrderItem> = req
        .items
        .into_iter()
        .map(|item| OrderItem {
            product_id: item.product_id,
            product_name: item.product_name,
            quantity: item.quantity,
            unit_price: item.unit_price,
            category: item.category,
        })
        .collect();

    // 计算订单总金额
    let total_amount: f64 = items
        .iter()
        .map(|item| item.unit_price * item.quantity as f64)
        .sum();

    let now = Utc::now();
    let order = MockOrder {
        order_id: format!("ORD-{}", Uuid::new_v4()),
        user_id: req.user_id.clone(),
        order_status: OrderStatus::Pending,
        total_amount,
        currency: "CNY".to_string(),
        items,
        created_at: now,
        updated_at: now,
    };

    info!(
        order_id = %order.order_id,
        user_id = %order.user_id,
        total_amount = order.total_amount,
        "创建订单"
    );

    state.orders.insert(&order.order_id, order.clone());

    (StatusCode::CREATED, Json(OrderResponse { order }))
}

/// 更新订单状态
///
/// POST /orders/:order_id/status
async fn update_order_status(
    State(state): State<Arc<OrderServiceState>>,
    Path(order_id): Path<String>,
    Json(req): Json<UpdateStatusRequest>,
) -> Result<Json<OrderResponse>, StatusCode> {
    // 获取现有订单
    let mut order = state.orders.get(&order_id).ok_or_else(|| {
        warn!(order_id = %order_id, "订单不存在，无法更新状态");
        StatusCode::NOT_FOUND
    })?;

    info!(
        order_id = %order_id,
        old_status = ?order.order_status,
        new_status = ?req.status,
        "更新订单状态"
    );

    // 更新状态和时间戳
    order.order_status = req.status;
    order.updated_at = Utc::now();

    // 保存更新后的订单
    state.orders.insert(&order_id, order.clone());

    Ok(Json(OrderResponse { order }))
}

/// 获取用户的订单列表
///
/// GET /users/:user_id/orders
async fn list_user_orders(
    State(state): State<Arc<OrderServiceState>>,
    Path(user_id): Path<String>,
    Query(query): Query<ListOrdersQuery>,
) -> Json<OrderListResponse> {
    let page = query.page.unwrap_or(1).max(1) as usize;
    let page_size = query.page_size.unwrap_or(10).min(100) as usize;

    // 筛选指定用户的订单
    let user_orders = state.orders.list_by(|order| order.user_id == user_id);
    let total = user_orders.len();

    // 分页处理
    let orders: Vec<MockOrder> = user_orders
        .into_iter()
        .skip((page - 1) * page_size)
        .take(page_size)
        .collect();

    info!(
        user_id = %user_id,
        page,
        page_size,
        total,
        returned = orders.len(),
        "列出用户订单"
    );

    Json(OrderListResponse { orders, total })
}

// ============================================================================
// 单元测试
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use axum::{
        body::Body,
        http::{Request, StatusCode},
    };
    use tower::ServiceExt;

    /// 创建测试用的应用实例
    fn create_test_app() -> (Router, Arc<OrderServiceState>) {
        let state = Arc::new(OrderServiceState::new());
        let app = order_routes().with_state(state.clone());
        (app, state)
    }

    #[tokio::test]
    async fn test_create_order() {
        let (app, _state) = create_test_app();

        let request_body = serde_json::json!({
            "user_id": "user-123",
            "items": [
                {
                    "product_id": "prod-1",
                    "product_name": "Test Product",
                    "quantity": 2,
                    "unit_price": 99.99,
                    "category": "Electronics"
                }
            ]
        });

        let response = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/orders")
                    .header("Content-Type", "application/json")
                    .body(Body::from(serde_json::to_string(&request_body).unwrap()))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::CREATED);

        let body = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        let resp: OrderResponse = serde_json::from_slice(&body).unwrap();

        assert!(resp.order.order_id.starts_with("ORD-"));
        assert_eq!(resp.order.user_id, "user-123");
        assert_eq!(resp.order.order_status, OrderStatus::Pending);
        assert!((resp.order.total_amount - 199.98).abs() < 0.01);
    }

    #[tokio::test]
    async fn test_get_order() {
        let (_app, state) = create_test_app();

        // 先创建一个订单
        let order = MockOrder::random("user-456");
        let order_id = order.order_id.clone();
        state.orders.insert(&order_id, order);

        // 重新创建 app 以便使用相同的 state
        let app = order_routes().with_state(state);

        let response = app
            .oneshot(
                Request::builder()
                    .method("GET")
                    .uri(format!("/orders/{}", order_id))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);

        let body = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        let resp: OrderResponse = serde_json::from_slice(&body).unwrap();

        assert_eq!(resp.order.order_id, order_id);
        assert_eq!(resp.order.user_id, "user-456");
    }

    #[tokio::test]
    async fn test_get_order_not_found() {
        let (app, _state) = create_test_app();

        let response = app
            .oneshot(
                Request::builder()
                    .method("GET")
                    .uri("/orders/non-existent-order")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::NOT_FOUND);
    }

    #[tokio::test]
    async fn test_list_user_orders() {
        let state = Arc::new(OrderServiceState::new());

        // 创建多个订单，分属不同用户
        for i in 0..5 {
            let user_id = if i < 3 { "user-a" } else { "user-b" };
            let order = MockOrder::random(user_id);
            let order_id = order.order_id.clone();
            state.orders.insert(&order_id, order);
        }

        let app = order_routes().with_state(state);

        let response = app
            .oneshot(
                Request::builder()
                    .method("GET")
                    .uri("/users/user-a/orders")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);

        let body = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        let resp: OrderListResponse = serde_json::from_slice(&body).unwrap();

        assert_eq!(resp.total, 3);
        assert_eq!(resp.orders.len(), 3);
        assert!(resp.orders.iter().all(|o| o.user_id == "user-a"));
    }

    #[tokio::test]
    async fn test_update_order_status() {
        let state = Arc::new(OrderServiceState::new());

        // 创建一个待处理订单
        let order = MockOrder::random("user-789");
        let order_id = order.order_id.clone();
        state.orders.insert(&order_id, order);

        let app = order_routes().with_state(state);

        let request_body = serde_json::json!({
            "status": "Paid"
        });

        let response = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri(format!("/orders/{}/status", order_id))
                    .header("Content-Type", "application/json")
                    .body(Body::from(serde_json::to_string(&request_body).unwrap()))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);

        let body = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        let resp: OrderResponse = serde_json::from_slice(&body).unwrap();

        assert_eq!(resp.order.order_status, OrderStatus::Paid);
    }
}
