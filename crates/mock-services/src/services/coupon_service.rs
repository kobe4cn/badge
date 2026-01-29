//! Mock 优惠券服务
//!
//! 提供优惠券发放、查询、核销等 REST API 端点，用于测试和开发环境。

use axum::{
    Json, Router,
    extract::{Path, Query, State},
    http::StatusCode,
    routing::{get, post},
};
use chrono::{Duration, Utc};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use uuid::Uuid;

use crate::models::{CouponStatus, CouponType, MockCoupon};
use crate::store::MemoryStore;

/// Mock Coupon 服务状态
#[derive(Default)]
pub struct CouponServiceState {
    pub coupons: MemoryStore<MockCoupon>,
}

// ============================================================================
// 请求/响应 DTO
// ============================================================================

/// 发放优惠券请求
#[derive(Debug, Deserialize)]
pub struct IssueCouponRequest {
    pub user_id: String,
    pub coupon_type: CouponType,
    pub discount_value: f64,
    pub min_order_amount: f64,
    /// 有效期天数
    pub valid_days: i64,
}

/// 批量发放优惠券请求
#[derive(Debug, Deserialize)]
pub struct BatchIssueCouponRequest {
    pub user_ids: Vec<String>,
    pub coupon_type: CouponType,
    pub discount_value: f64,
    pub min_order_amount: f64,
    pub valid_days: i64,
}

/// 核销优惠券请求
#[derive(Debug, Deserialize)]
pub struct RedeemCouponRequest {
    pub order_id: String,
    pub order_amount: f64,
}

/// 查询优惠券列表参数
#[derive(Debug, Deserialize)]
pub struct ListCouponsQuery {
    pub status: Option<CouponStatus>,
}

/// 优惠券响应
#[derive(Debug, Serialize, Deserialize)]
pub struct CouponResponse {
    pub coupon: MockCoupon,
}

/// 优惠券列表响应
#[derive(Debug, Serialize, Deserialize)]
pub struct CouponListResponse {
    pub coupons: Vec<MockCoupon>,
    pub total: usize,
}

/// 批量发放响应
#[derive(Debug, Serialize, Deserialize)]
pub struct BatchIssueResponse {
    pub issued_count: usize,
    pub coupon_ids: Vec<String>,
}

/// 核销响应
#[derive(Debug, Serialize, Deserialize)]
pub struct RedeemResponse {
    pub success: bool,
    pub discount_applied: f64,
    pub message: String,
}

/// 错误响应
#[derive(Debug, Serialize, Deserialize)]
pub struct ErrorResponse {
    pub error: String,
}

// ============================================================================
// 路由配置
// ============================================================================

/// 构建优惠券服务路由
pub fn coupon_routes() -> Router<Arc<CouponServiceState>> {
    Router::new()
        .route("/coupons/{coupon_id}", get(get_coupon))
        .route("/coupons", post(issue_coupon))
        .route("/coupons/{coupon_id}/redeem", post(redeem_coupon))
        .route("/users/{user_id}/coupons", get(list_user_coupons))
        .route("/coupons/batch", post(batch_issue_coupons))
}

// ============================================================================
// 端点处理函数
// ============================================================================

/// 获取优惠券详情
#[tracing::instrument(skip(state))]
async fn get_coupon(
    State(state): State<Arc<CouponServiceState>>,
    Path(coupon_id): Path<String>,
) -> Result<Json<CouponResponse>, (StatusCode, Json<ErrorResponse>)> {
    tracing::info!("获取优惠券详情: {}", coupon_id);

    state.coupons.get(&coupon_id).map_or_else(
        || {
            Err((
                StatusCode::NOT_FOUND,
                Json(ErrorResponse {
                    error: format!("优惠券不存在: {}", coupon_id),
                }),
            ))
        },
        |coupon| Ok(Json(CouponResponse { coupon })),
    )
}

/// 发放优惠券
#[tracing::instrument(skip(state))]
async fn issue_coupon(
    State(state): State<Arc<CouponServiceState>>,
    Json(req): Json<IssueCouponRequest>,
) -> (StatusCode, Json<CouponResponse>) {
    tracing::info!("发放优惠券给用户: {}", req.user_id);

    let coupon = create_coupon(&req.user_id, &req);
    state.coupons.insert(&coupon.coupon_id, coupon.clone());

    tracing::info!("优惠券发放成功: {}", coupon.coupon_id);
    (StatusCode::CREATED, Json(CouponResponse { coupon }))
}

/// 核销优惠券
#[tracing::instrument(skip(state))]
async fn redeem_coupon(
    State(state): State<Arc<CouponServiceState>>,
    Path(coupon_id): Path<String>,
    Json(req): Json<RedeemCouponRequest>,
) -> Result<Json<RedeemResponse>, (StatusCode, Json<ErrorResponse>)> {
    tracing::info!("核销优惠券: {}, 订单: {}", coupon_id, req.order_id);

    let Some(mut coupon) = state.coupons.get(&coupon_id) else {
        return Err((
            StatusCode::NOT_FOUND,
            Json(ErrorResponse {
                error: format!("优惠券不存在: {}", coupon_id),
            }),
        ));
    };

    // 检查优惠券状态
    if coupon.status != CouponStatus::Active {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse {
                error: format!("优惠券状态无效: {:?}", coupon.status),
            }),
        ));
    }

    // 检查是否过期
    if coupon.expires_at < Utc::now() {
        // 更新状态为已过期
        coupon.status = CouponStatus::Expired;
        state.coupons.insert(&coupon_id, coupon);
        return Err((
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse {
                error: "优惠券已过期".to_string(),
            }),
        ));
    }

    // 检查订单金额是否满足最低消费
    if req.order_amount < coupon.min_order_amount {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse {
                error: format!("订单金额不足，最低消费: {:.2}", coupon.min_order_amount),
            }),
        ));
    }

    // 计算折扣金额
    let discount_applied = coupon.calculate_discount(req.order_amount);

    // 更新优惠券状态为已使用
    coupon.status = CouponStatus::Used;
    state.coupons.insert(&coupon_id, coupon);

    tracing::info!(
        "优惠券核销成功: {}, 折扣金额: {:.2}",
        coupon_id,
        discount_applied
    );

    Ok(Json(RedeemResponse {
        success: true,
        discount_applied,
        message: format!("优惠券核销成功，已减免 {:.2} 元", discount_applied),
    }))
}

/// 获取用户的优惠券列表
#[tracing::instrument(skip(state))]
async fn list_user_coupons(
    State(state): State<Arc<CouponServiceState>>,
    Path(user_id): Path<String>,
    Query(query): Query<ListCouponsQuery>,
) -> Json<CouponListResponse> {
    tracing::info!(
        "获取用户优惠券列表: {}, 状态筛选: {:?}",
        user_id,
        query.status
    );

    let coupons = state.coupons.list_by(|c| {
        // 必须匹配用户 ID
        if c.user_id != user_id {
            return false;
        }
        // 如果指定了状态筛选，则必须匹配
        query.status.is_none_or(|s| c.status == s)
    });

    let total = coupons.len();
    Json(CouponListResponse { coupons, total })
}

/// 批量发放优惠券
#[tracing::instrument(skip(state))]
async fn batch_issue_coupons(
    State(state): State<Arc<CouponServiceState>>,
    Json(req): Json<BatchIssueCouponRequest>,
) -> (StatusCode, Json<BatchIssueResponse>) {
    tracing::info!("批量发放优惠券给 {} 个用户", req.user_ids.len());

    let issue_req = IssueCouponRequest {
        user_id: String::new(), // 占位，实际在循环中设置
        coupon_type: req.coupon_type,
        discount_value: req.discount_value,
        min_order_amount: req.min_order_amount,
        valid_days: req.valid_days,
    };

    let mut coupon_ids = Vec::with_capacity(req.user_ids.len());

    for user_id in &req.user_ids {
        let coupon = create_coupon(user_id, &issue_req);
        let coupon_id = coupon.coupon_id.clone();
        coupon_ids.push(coupon_id.clone());
        state.coupons.insert(&coupon_id, coupon);
    }

    let issued_count = coupon_ids.len();
    tracing::info!("批量发放完成，共发放 {} 张优惠券", issued_count);

    (
        StatusCode::CREATED,
        Json(BatchIssueResponse {
            issued_count,
            coupon_ids,
        }),
    )
}

// ============================================================================
// 辅助函数
// ============================================================================

/// 根据请求参数创建优惠券
fn create_coupon(user_id: &str, req: &IssueCouponRequest) -> MockCoupon {
    let now = Utc::now();
    MockCoupon {
        coupon_id: format!("CPN-{}", Uuid::new_v4()),
        user_id: user_id.to_string(),
        coupon_type: req.coupon_type,
        discount_value: req.discount_value,
        min_order_amount: req.min_order_amount,
        status: CouponStatus::Active,
        issued_at: now,
        expires_at: now + Duration::days(req.valid_days),
    }
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

    fn create_test_app() -> Router {
        let state = Arc::new(CouponServiceState::default());
        coupon_routes().with_state(state)
    }

    fn create_test_app_with_state(state: Arc<CouponServiceState>) -> Router {
        coupon_routes().with_state(state)
    }

    #[tokio::test]
    async fn test_issue_coupon() {
        let app = create_test_app();

        let req_body = serde_json::json!({
            "user_id": "user-123",
            "coupon_type": "Percentage",
            "discount_value": 10.0,
            "min_order_amount": 100.0,
            "valid_days": 30
        });

        let response = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/coupons")
                    .header("content-type", "application/json")
                    .body(Body::from(serde_json::to_string(&req_body).unwrap()))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::CREATED);

        let body = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        let resp: CouponResponse = serde_json::from_slice(&body).unwrap();

        assert_eq!(resp.coupon.user_id, "user-123");
        assert_eq!(resp.coupon.coupon_type, CouponType::Percentage);
        assert_eq!(resp.coupon.status, CouponStatus::Active);
    }

    #[tokio::test]
    async fn test_get_coupon() {
        let state = Arc::new(CouponServiceState::default());
        let coupon = MockCoupon {
            coupon_id: "CPN-TEST-001".to_string(),
            user_id: "user-123".to_string(),
            coupon_type: CouponType::FixedAmount,
            discount_value: 50.0,
            min_order_amount: 200.0,
            status: CouponStatus::Active,
            issued_at: Utc::now(),
            expires_at: Utc::now() + Duration::days(30),
        };
        let coupon_id = coupon.coupon_id.clone();
        state.coupons.insert(&coupon_id, coupon);

        let app = create_test_app_with_state(state);

        let response = app
            .oneshot(
                Request::builder()
                    .method("GET")
                    .uri("/coupons/CPN-TEST-001")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);

        let body = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        let resp: CouponResponse = serde_json::from_slice(&body).unwrap();

        assert_eq!(resp.coupon.coupon_id, "CPN-TEST-001");
        assert_eq!(resp.coupon.discount_value, 50.0);
    }

    #[tokio::test]
    async fn test_list_user_coupons() {
        let state = Arc::new(CouponServiceState::default());

        // 为同一用户添加多张优惠券
        for i in 0..3 {
            let coupon = MockCoupon {
                coupon_id: format!("CPN-{}", i),
                user_id: "user-456".to_string(),
                coupon_type: CouponType::Percentage,
                discount_value: 10.0,
                min_order_amount: 100.0,
                status: if i == 0 {
                    CouponStatus::Used
                } else {
                    CouponStatus::Active
                },
                issued_at: Utc::now(),
                expires_at: Utc::now() + Duration::days(30),
            };
            let coupon_id = coupon.coupon_id.clone();
            state.coupons.insert(&coupon_id, coupon);
        }

        let app = create_test_app_with_state(state);

        let response = app
            .oneshot(
                Request::builder()
                    .method("GET")
                    .uri("/users/user-456/coupons")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);

        let body = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        let resp: CouponListResponse = serde_json::from_slice(&body).unwrap();

        assert_eq!(resp.total, 3);
    }

    #[tokio::test]
    async fn test_redeem_coupon_success() {
        let state = Arc::new(CouponServiceState::default());
        let coupon = MockCoupon {
            coupon_id: "CPN-REDEEM-001".to_string(),
            user_id: "user-123".to_string(),
            coupon_type: CouponType::Percentage,
            discount_value: 10.0, // 10% 折扣
            min_order_amount: 100.0,
            status: CouponStatus::Active,
            issued_at: Utc::now(),
            expires_at: Utc::now() + Duration::days(30),
        };
        let coupon_id = coupon.coupon_id.clone();
        state.coupons.insert(&coupon_id, coupon);

        let app = create_test_app_with_state(state.clone());

        let req_body = serde_json::json!({
            "order_id": "ORDER-001",
            "order_amount": 200.0
        });

        let response = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/coupons/CPN-REDEEM-001/redeem")
                    .header("content-type", "application/json")
                    .body(Body::from(serde_json::to_string(&req_body).unwrap()))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);

        let body = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        let resp: RedeemResponse = serde_json::from_slice(&body).unwrap();

        assert!(resp.success);
        // 200 * 10% = 20
        assert!((resp.discount_applied - 20.0).abs() < 0.01);

        // 验证优惠券状态已更新为 Used
        let updated_coupon = state.coupons.get("CPN-REDEEM-001").unwrap();
        assert_eq!(updated_coupon.status, CouponStatus::Used);
    }

    #[tokio::test]
    async fn test_redeem_coupon_insufficient_amount() {
        let state = Arc::new(CouponServiceState::default());
        let coupon = MockCoupon {
            coupon_id: "CPN-REDEEM-002".to_string(),
            user_id: "user-123".to_string(),
            coupon_type: CouponType::FixedAmount,
            discount_value: 50.0,
            min_order_amount: 200.0, // 最低消费 200 元
            status: CouponStatus::Active,
            issued_at: Utc::now(),
            expires_at: Utc::now() + Duration::days(30),
        };
        let coupon_id = coupon.coupon_id.clone();
        state.coupons.insert(&coupon_id, coupon);

        let app = create_test_app_with_state(state);

        let req_body = serde_json::json!({
            "order_id": "ORDER-002",
            "order_amount": 100.0  // 未达到最低消费
        });

        let response = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/coupons/CPN-REDEEM-002/redeem")
                    .header("content-type", "application/json")
                    .body(Body::from(serde_json::to_string(&req_body).unwrap()))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::BAD_REQUEST);

        let body = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        let resp: ErrorResponse = serde_json::from_slice(&body).unwrap();

        assert!(resp.error.contains("订单金额不足"));
    }

    #[tokio::test]
    async fn test_redeem_expired_coupon() {
        let state = Arc::new(CouponServiceState::default());
        let coupon = MockCoupon {
            coupon_id: "CPN-EXPIRED-001".to_string(),
            user_id: "user-123".to_string(),
            coupon_type: CouponType::Percentage,
            discount_value: 10.0,
            min_order_amount: 100.0,
            status: CouponStatus::Active,
            issued_at: Utc::now() - Duration::days(60),
            expires_at: Utc::now() - Duration::days(1), // 已过期
        };
        let coupon_id = coupon.coupon_id.clone();
        state.coupons.insert(&coupon_id, coupon);

        let app = create_test_app_with_state(state);

        let req_body = serde_json::json!({
            "order_id": "ORDER-003",
            "order_amount": 200.0
        });

        let response = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/coupons/CPN-EXPIRED-001/redeem")
                    .header("content-type", "application/json")
                    .body(Body::from(serde_json::to_string(&req_body).unwrap()))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::BAD_REQUEST);

        let body = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        let resp: ErrorResponse = serde_json::from_slice(&body).unwrap();

        assert!(resp.error.contains("已过期"));
    }

    #[tokio::test]
    async fn test_batch_issue() {
        let app = create_test_app();

        let req_body = serde_json::json!({
            "user_ids": ["user-1", "user-2", "user-3"],
            "coupon_type": "FreeShipping",
            "discount_value": 15.0,
            "min_order_amount": 50.0,
            "valid_days": 7
        });

        let response = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/coupons/batch")
                    .header("content-type", "application/json")
                    .body(Body::from(serde_json::to_string(&req_body).unwrap()))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::CREATED);

        let body = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        let resp: BatchIssueResponse = serde_json::from_slice(&body).unwrap();

        assert_eq!(resp.issued_count, 3);
        assert_eq!(resp.coupon_ids.len(), 3);
    }
}
