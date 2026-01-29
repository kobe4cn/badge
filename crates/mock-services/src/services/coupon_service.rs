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
///
/// 核销流程：
/// 1. 检查优惠券是否存在
/// 2. 检查状态是否为 Active
/// 3. 检查是否已过期
/// 4. 检查订单金额是否满足最低消费
/// 5. 计算折扣并更新状态
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

    if coupon.status != CouponStatus::Active {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse {
                error: format!("优惠券状态无效: {:?}", coupon.status),
            }),
        ));
    }

    if coupon.expires_at < Utc::now() {
        coupon.status = CouponStatus::Expired;
        state.coupons.insert(&coupon_id, coupon);
        return Err((
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse {
                error: "优惠券已过期".to_string(),
            }),
        ));
    }

    if req.order_amount < coupon.min_order_amount {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse {
                error: format!("订单金额不足，最低消费: {:.2}", coupon.min_order_amount),
            }),
        ));
    }

    let discount_applied = coupon.calculate_discount(req.order_amount);
    coupon.status = CouponStatus::Used;
    state.coupons.insert(&coupon_id, coupon);

    tracing::info!(
        "优惠券核销成功: {}, 折扣: {:.2}",
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
    tracing::info!("获取用户优惠券: {}, 状态: {:?}", user_id, query.status);

    let coupons = state.coupons.list_by(|c| {
        if c.user_id != user_id {
            return false;
        }
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
        user_id: String::new(),
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
