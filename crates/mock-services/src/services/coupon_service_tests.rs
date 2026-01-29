//! coupon_service 单元测试

use super::coupon_service::*;
use crate::models::{CouponStatus, CouponType, MockCoupon};
use axum::{
    Router,
    body::Body,
    http::{Request, StatusCode},
};
use chrono::{Duration, Utc};
use std::sync::Arc;
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
        discount_value: 10.0,
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
    assert!((resp.discount_applied - 20.0).abs() < 0.01);

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
        min_order_amount: 200.0,
        status: CouponStatus::Active,
        issued_at: Utc::now(),
        expires_at: Utc::now() + Duration::days(30),
    };
    let coupon_id = coupon.coupon_id.clone();
    state.coupons.insert(&coupon_id, coupon);

    let app = create_test_app_with_state(state);

    let req_body = serde_json::json!({
        "order_id": "ORDER-002",
        "order_amount": 100.0
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
        expires_at: Utc::now() - Duration::days(1),
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
