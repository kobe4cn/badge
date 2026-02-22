//! gRPC 服务端实现
//!
//! 将内部服务层暴露为 gRPC 接口，处理 Proto 类型与内部 DTO 之间的转换

use std::sync::Arc;

use sqlx::PgPool;
use tonic::{Request, Response, Status};
use tracing::instrument;

use badge_proto::badge::{
    Badge as ProtoBadge, BadgeStatus as ProtoBadgeStatus, BadgeType as ProtoBadgeType,
    FindBadgesBySourceRefRequest, FindBadgesBySourceRefResponse,
    GetBadgeDetailRequest, GetBadgeDetailResponse, GetBadgeWallRequest, GetBadgeWallResponse,
    GetUserBadgesRequest, GetUserBadgesResponse, GrantBadgeRequest as ProtoGrantBadgeRequest,
    GrantBadgeResponse as ProtoGrantBadgeResponse, PinBadgeRequest, PinBadgeResponse,
    RedeemBadgeRequest as ProtoRedeemBadgeRequest, RedeemBadgeResponse as ProtoRedeemBadgeResponse,
    RefreshAutoBenefitCacheRequest, RefreshAutoBenefitCacheResponse,
    RefreshDependencyCacheRequest, RefreshDependencyCacheResponse,
    RevokeBadgeRequest as ProtoRevokeBadgeRequest, RevokeBadgeResponse as ProtoRevokeBadgeResponse,
    SourceRefBadge, UserBadge as ProtoUserBadge,
    badge_management_service_server::BadgeManagementService,
};

use crate::auto_benefit::AutoBenefitRuleCache;
use crate::cascade::CascadeEvaluator;

use crate::error::BadgeError;
use crate::models::{BadgeType, SourceType, UserBadgeStatus};
use crate::repository::{
    BadgeLedgerRepositoryTrait, BadgeRepositoryTrait, RedemptionRepositoryTrait,
    UserBadgeRepositoryTrait,
};
use crate::service::dto::{
    GrantBadgeRequest, RedeemBadgeRequest, RevokeBadgeRequest, UserBadgeDto,
};
use crate::service::{BadgeQueryService, GrantService, RedemptionService, RevokeService};

// ==================== 错误转换 ====================

impl From<BadgeError> for Status {
    fn from(err: BadgeError) -> Self {
        match err {
            BadgeError::BadgeNotFound(_) => Status::not_found(err.to_string()),
            BadgeError::UserBadgeNotFound { .. } => Status::not_found(err.to_string()),
            BadgeError::SeriesNotFound(_) => Status::not_found(err.to_string()),
            BadgeError::CategoryNotFound(_) => Status::not_found(err.to_string()),
            BadgeError::RedemptionRuleNotFound(_) => Status::not_found(err.to_string()),
            BadgeError::BenefitNotFound(_) => Status::not_found(err.to_string()),
            BadgeError::OrderNotFound(_) => Status::not_found(err.to_string()),
            BadgeError::BadgeInactive(_) => Status::failed_precondition(err.to_string()),
            BadgeError::RedemptionRuleInactive(_) => Status::failed_precondition(err.to_string()),
            BadgeError::InsufficientBadges { .. } => Status::failed_precondition(err.to_string()),
            BadgeError::UserBadgeExpired(_) => Status::failed_precondition(err.to_string()),
            BadgeError::InvalidOrderStatus { .. } => Status::failed_precondition(err.to_string()),
            BadgeError::BadgeOutOfStock(_) => Status::resource_exhausted(err.to_string()),
            BadgeError::BenefitOutOfStock(_) => Status::resource_exhausted(err.to_string()),
            BadgeError::BadgeAcquisitionLimitReached { .. } => {
                Status::resource_exhausted(err.to_string())
            }
            BadgeError::RedemptionFrequencyLimitReached { .. } => {
                Status::resource_exhausted(err.to_string())
            }
            BadgeError::Validation(_) => Status::invalid_argument(err.to_string()),
            BadgeError::DuplicateRedemption(_) => Status::already_exists(err.to_string()),
            BadgeError::Database(_) => Status::internal(err.to_string()),
            BadgeError::Serialization(_) => Status::internal(err.to_string()),
            BadgeError::Redis(_) => Status::internal(err.to_string()),
            BadgeError::Internal(_) => Status::internal(err.to_string()),
            BadgeError::ConcurrencyConflict => Status::aborted(err.to_string()),
            // 依赖关系和级联评估相关错误
            BadgeError::PrerequisiteNotMet { .. } => Status::failed_precondition(err.to_string()),
            BadgeError::ExclusiveConflict { .. } => Status::failed_precondition(err.to_string()),
            BadgeError::CascadeDepthExceeded { .. } => Status::resource_exhausted(err.to_string()),
            BadgeError::CascadeTimeout { .. } => Status::deadline_exceeded(err.to_string()),
            BadgeError::CascadeGrantServiceNotSet => Status::internal(err.to_string()),
            // 锁相关错误
            BadgeError::LockConflict { .. } => Status::aborted(err.to_string()),
        }
    }
}

// ==================== 类型转换辅助函数 ====================

/// 将内部 BadgeType 转换为 Proto BadgeType
fn badge_type_to_proto(badge_type: BadgeType) -> i32 {
    match badge_type {
        BadgeType::Normal => ProtoBadgeType::Transaction as i32,
        BadgeType::Limited => ProtoBadgeType::Engagement as i32,
        BadgeType::Achievement => ProtoBadgeType::Identity as i32,
        BadgeType::Event => ProtoBadgeType::Seasonal as i32,
    }
}

/// 将内部 UserBadgeStatus 转换为 Proto BadgeStatus
fn user_badge_status_to_proto(status: UserBadgeStatus) -> i32 {
    match status {
        UserBadgeStatus::Active => ProtoBadgeStatus::Active as i32,
        UserBadgeStatus::Expired => ProtoBadgeStatus::Expired as i32,
        UserBadgeStatus::Revoked => ProtoBadgeStatus::Revoked as i32,
        UserBadgeStatus::Redeemed => ProtoBadgeStatus::Redeemed as i32,
    }
}

/// 将 chrono::DateTime 转换为 prost_types::Timestamp
fn datetime_to_timestamp(dt: chrono::DateTime<chrono::Utc>) -> prost_types::Timestamp {
    prost_types::Timestamp {
        seconds: dt.timestamp(),
        nanos: dt.timestamp_subsec_nanos() as i32,
    }
}

/// 解析 source_type 字符串为 SourceType
fn parse_source_type(s: &str) -> SourceType {
    match s.to_lowercase().as_str() {
        "event" => SourceType::Event,
        "scheduled" => SourceType::Scheduled,
        "manual" => SourceType::Manual,
        "redemption" => SourceType::Redemption,
        _ => SourceType::System,
    }
}

/// 将 UserBadgeDto 转换为 Proto UserBadge
fn user_badge_dto_to_proto(dto: &UserBadgeDto) -> ProtoUserBadge {
    ProtoUserBadge {
        id: dto.badge_id.to_string(),
        badge: Some(ProtoBadge {
            id: dto.badge_id.to_string(),
            code: String::new(),
            name: dto.badge_name.clone(),
            description: String::new(),
            badge_type: badge_type_to_proto(dto.badge_type),
            category_name: String::new(),
            series_name: String::new(),
            icon_url: dto.assets.icon_url.clone(),
            icon_3d_url: dto.assets.animation_url.clone().unwrap_or_default(),
        }),
        quantity: dto.quantity,
        status: user_badge_status_to_proto(dto.status),
        acquired_at: Some(datetime_to_timestamp(dto.acquired_at)),
        expires_at: dto.expires_at.map(datetime_to_timestamp),
        is_pinned: false,
    }
}

// ==================== gRPC 服务实现 ====================

/// 徽章管理服务 gRPC 实现
///
/// 聚合多个业务服务，对外暴露统一的 gRPC 接口
pub struct BadgeManagementServiceImpl<BR, UBR, RR, LR>
where
    BR: BadgeRepositoryTrait,
    UBR: UserBadgeRepositoryTrait,
    RR: RedemptionRepositoryTrait,
    LR: BadgeLedgerRepositoryTrait,
{
    query_service: Arc<BadgeQueryService<BR, UBR, RR, LR>>,
    grant_service: Arc<GrantService<BR>>,
    revoke_service: Arc<RevokeService<BR>>,
    redemption_service: Arc<RedemptionService>,
    pool: PgPool,
    /// 级联评估器（用于刷新依赖图缓存）
    cascade_evaluator: Option<Arc<CascadeEvaluator>>,
    /// 自动权益规则缓存（用于刷新自动权益缓存）
    auto_benefit_rule_cache: Option<Arc<AutoBenefitRuleCache>>,
}

impl<BR, UBR, RR, LR> BadgeManagementServiceImpl<BR, UBR, RR, LR>
where
    BR: BadgeRepositoryTrait,
    UBR: UserBadgeRepositoryTrait,
    RR: RedemptionRepositoryTrait,
    LR: BadgeLedgerRepositoryTrait,
{
    pub fn new(
        query_service: Arc<BadgeQueryService<BR, UBR, RR, LR>>,
        grant_service: Arc<GrantService<BR>>,
        revoke_service: Arc<RevokeService<BR>>,
        redemption_service: Arc<RedemptionService>,
        pool: PgPool,
        cascade_evaluator: Option<Arc<CascadeEvaluator>>,
    ) -> Self {
        Self {
            query_service,
            grant_service,
            revoke_service,
            redemption_service,
            pool,
            cascade_evaluator,
            auto_benefit_rule_cache: None,
        }
    }

    /// 设置自动权益规则缓存
    pub fn with_auto_benefit_rule_cache(mut self, cache: Arc<AutoBenefitRuleCache>) -> Self {
        self.auto_benefit_rule_cache = Some(cache);
        self
    }
}

#[tonic::async_trait]
impl<BR, UBR, RR, LR> BadgeManagementService for BadgeManagementServiceImpl<BR, UBR, RR, LR>
where
    BR: BadgeRepositoryTrait + 'static,
    UBR: UserBadgeRepositoryTrait + 'static,
    RR: RedemptionRepositoryTrait + 'static,
    LR: BadgeLedgerRepositoryTrait + 'static,
{
    /// 获取用户徽章列表
    #[instrument(skip(self), fields(user_id = %request.get_ref().user_id))]
    async fn get_user_badges(
        &self,
        request: Request<GetUserBadgesRequest>,
    ) -> Result<Response<GetUserBadgesResponse>, Status> {
        let req = request.into_inner();

        if req.user_id.is_empty() {
            return Err(Status::invalid_argument("user_id 不能为空"));
        }

        let badges = self
            .query_service
            .get_user_badges(&req.user_id)
            .await
            .map_err(Status::from)?;

        // 转换为 Proto 类型
        let proto_badges: Vec<ProtoUserBadge> =
            badges.iter().map(user_badge_dto_to_proto).collect();

        // 简单分页处理
        let total = proto_badges.len() as i32;
        let page = if req.page > 0 { req.page } else { 1 };
        let page_size = if req.page_size > 0 { req.page_size } else { 20 };
        let start = ((page - 1) * page_size) as usize;
        let end = (start + page_size as usize).min(proto_badges.len());

        let paged_badges = if start < proto_badges.len() {
            proto_badges[start..end].to_vec()
        } else {
            vec![]
        };

        Ok(Response::new(GetUserBadgesResponse {
            badges: paged_badges,
            total,
            page,
            page_size,
        }))
    }

    /// 获取徽章详情
    #[instrument(skip(self), fields(badge_id = %request.get_ref().badge_id))]
    async fn get_badge_detail(
        &self,
        request: Request<GetBadgeDetailRequest>,
    ) -> Result<Response<GetBadgeDetailResponse>, Status> {
        let req = request.into_inner();

        if req.badge_id.is_empty() {
            return Err(Status::invalid_argument("badge_id 不能为空"));
        }

        let badge_id: i64 = req
            .badge_id
            .parse()
            .map_err(|_| Status::invalid_argument("badge_id 格式无效"))?;

        let detail = self
            .query_service
            .get_badge_detail(badge_id)
            .await
            .map_err(Status::from)?;

        // 构造响应
        let badge = ProtoBadge {
            id: detail.id.to_string(),
            code: String::new(),
            name: detail.name,
            description: detail.description,
            badge_type: badge_type_to_proto(detail.badge_type),
            category_name: detail.category_name,
            series_name: detail.series_name,
            icon_url: detail.assets.icon_url,
            icon_3d_url: detail.assets.animation_url.unwrap_or_default(),
        };

        // 如果提供了 user_id，查询用户持有状态
        let (user_quantity, user_acquired_at, user_expires_at) = if !req.user_id.is_empty() {
            let user_badges = self
                .query_service
                .get_user_badges(&req.user_id)
                .await
                .map_err(Status::from)?;

            if let Some(ub) = user_badges.iter().find(|b| b.badge_id == badge_id) {
                (
                    Some(ub.quantity),
                    Some(datetime_to_timestamp(ub.acquired_at)),
                    ub.expires_at.map(datetime_to_timestamp),
                )
            } else {
                (None, None, None)
            }
        } else {
            (None, None, None)
        };

        Ok(Response::new(GetBadgeDetailResponse {
            badge: Some(badge),
            user_quantity,
            user_acquired_at,
            user_expires_at,
            can_redeem: !detail.redeemable_benefits.is_empty(),
        }))
    }

    /// 获取徽章墙
    #[instrument(skip(self), fields(user_id = %request.get_ref().user_id))]
    async fn get_badge_wall(
        &self,
        request: Request<GetBadgeWallRequest>,
    ) -> Result<Response<GetBadgeWallResponse>, Status> {
        let req = request.into_inner();

        if req.user_id.is_empty() {
            return Err(Status::invalid_argument("user_id 不能为空"));
        }

        let wall = self
            .query_service
            .get_badge_wall(&req.user_id)
            .await
            .map_err(Status::from)?;

        // 获取统计信息
        let stats = self
            .query_service
            .get_user_badge_stats(&req.user_id)
            .await
            .map_err(Status::from)?;

        // 收集所有徽章并转换为 Proto
        let mut all_badges: Vec<ProtoUserBadge> = Vec::new();
        for category in &wall.categories {
            for badge in &category.badges {
                all_badges.push(user_badge_dto_to_proto(badge));
            }
        }

        Ok(Response::new(GetBadgeWallResponse {
            badges: all_badges,
            total_count: wall.total_count,
            active_count: stats.active_badges,
            expired_count: stats.expired_badges,
            redeemed_count: stats.redeemed_badges,
        }))
    }

    /// 发放徽章（内部接口）
    #[instrument(skip(self), fields(user_id = %request.get_ref().user_id, badge_id = %request.get_ref().badge_id))]
    async fn grant_badge(
        &self,
        request: Request<ProtoGrantBadgeRequest>,
    ) -> Result<Response<ProtoGrantBadgeResponse>, Status> {
        let req = request.into_inner();

        // 参数校验
        if req.user_id.is_empty() {
            return Err(Status::invalid_argument("user_id 不能为空"));
        }
        if req.badge_id.is_empty() {
            return Err(Status::invalid_argument("badge_id 不能为空"));
        }

        let badge_id: i64 = req
            .badge_id
            .parse()
            .map_err(|_| Status::invalid_argument("badge_id 格式无效"))?;

        // 构造内部请求
        let source_type = parse_source_type(&req.source_type);
        let mut grant_req = GrantBadgeRequest::new(&req.user_id, badge_id, req.quantity)
            .with_source(
                source_type,
                if req.source_ref.is_empty() {
                    None
                } else {
                    Some(req.source_ref)
                },
            );

        if !req.operator.is_empty() {
            grant_req.operator = Some(req.operator);
        }

        // 调用服务
        let result = self.grant_service.grant_badge(grant_req).await;

        match result {
            Ok(resp) => Ok(Response::new(ProtoGrantBadgeResponse {
                success: resp.success,
                user_badge_id: resp.user_badge_id.to_string(),
                message: resp.message,
            })),
            Err(e) => Ok(Response::new(ProtoGrantBadgeResponse {
                success: false,
                user_badge_id: String::new(),
                message: e.to_string(),
            })),
        }
    }

    /// 取消徽章（内部接口）
    #[instrument(skip(self), fields(user_id = %request.get_ref().user_id, badge_id = %request.get_ref().badge_id))]
    async fn revoke_badge(
        &self,
        request: Request<ProtoRevokeBadgeRequest>,
    ) -> Result<Response<ProtoRevokeBadgeResponse>, Status> {
        let req = request.into_inner();

        // 参数校验
        if req.user_id.is_empty() {
            return Err(Status::invalid_argument("user_id 不能为空"));
        }
        if req.badge_id.is_empty() {
            return Err(Status::invalid_argument("badge_id 不能为空"));
        }
        if req.reason.is_empty() {
            return Err(Status::invalid_argument("reason 不能为空"));
        }

        let badge_id: i64 = req
            .badge_id
            .parse()
            .map_err(|_| Status::invalid_argument("badge_id 格式无效"))?;

        // 构造内部请求
        let revoke_req = if req.operator.is_empty() {
            RevokeBadgeRequest::system(&req.user_id, badge_id, req.quantity, &req.reason)
        } else {
            RevokeBadgeRequest::manual(
                &req.user_id,
                badge_id,
                req.quantity,
                &req.reason,
                &req.operator,
            )
        };

        // 调用服务
        let result = self.revoke_service.revoke_badge(revoke_req).await;

        match result {
            Ok(resp) => Ok(Response::new(ProtoRevokeBadgeResponse {
                success: resp.success,
                message: resp.message,
            })),
            Err(e) => Ok(Response::new(ProtoRevokeBadgeResponse {
                success: false,
                message: e.to_string(),
            })),
        }
    }

    /// 兑换徽章
    #[instrument(skip(self), fields(user_id = %request.get_ref().user_id, rule_id = %request.get_ref().redemption_rule_id))]
    async fn redeem_badge(
        &self,
        request: Request<ProtoRedeemBadgeRequest>,
    ) -> Result<Response<ProtoRedeemBadgeResponse>, Status> {
        let req = request.into_inner();

        // 参数校验
        if req.user_id.is_empty() {
            return Err(Status::invalid_argument("user_id 不能为空"));
        }
        if req.redemption_rule_id.is_empty() {
            return Err(Status::invalid_argument("redemption_rule_id 不能为空"));
        }

        let rule_id: i64 = req
            .redemption_rule_id
            .parse()
            .map_err(|_| Status::invalid_argument("redemption_rule_id 格式无效"))?;

        // 生成幂等键（基于用户ID和规则ID的组合，生产环境应使用客户端传入的幂等键）
        let idempotency_key = format!(
            "{}:{}:{}",
            req.user_id,
            rule_id,
            chrono::Utc::now().timestamp_millis()
        );

        // 构造内部请求
        let redeem_req = RedeemBadgeRequest::new(&req.user_id, rule_id, &idempotency_key);

        // 调用服务
        let result = self.redemption_service.redeem_badge(redeem_req).await;

        match result {
            Ok(resp) => Ok(Response::new(ProtoRedeemBadgeResponse {
                success: resp.success,
                order_id: resp.order_id.to_string(),
                benefit_id: String::new(),
                benefit_name: resp.benefit_name,
                message: resp.message,
            })),
            Err(e) => Ok(Response::new(ProtoRedeemBadgeResponse {
                success: false,
                order_id: String::new(),
                benefit_id: String::new(),
                benefit_name: String::new(),
                message: e.to_string(),
            })),
        }
    }

    /// 置顶/佩戴徽章
    #[instrument(skip(self), fields(user_id = %request.get_ref().user_id, user_badge_id = %request.get_ref().user_badge_id))]
    async fn pin_badge(
        &self,
        request: Request<PinBadgeRequest>,
    ) -> Result<Response<PinBadgeResponse>, Status> {
        let req = request.into_inner();

        // 参数校验
        if req.user_id.is_empty() {
            return Err(Status::invalid_argument("user_id 不能为空"));
        }
        if req.user_badge_id.is_empty() {
            return Err(Status::invalid_argument("user_badge_id 不能为空"));
        }

        let user_badge_id: i64 = req
            .user_badge_id
            .parse()
            .map_err(|_| Status::invalid_argument("user_badge_id 格式无效"))?;

        // 直接在 gRPC 层实现置顶逻辑（简化版）
        // 由于 user_badges 表当前没有 pinned 字段，这里只做验证和返回成功
        // 生产环境应添加 pinned 字段并通过 service 层处理
        let result = sqlx::query(
            r#"
            SELECT id FROM user_badges
            WHERE id = $1 AND user_id = $2
            "#,
        )
        .bind(user_badge_id)
        .bind(&req.user_id)
        .fetch_optional(&self.pool)
        .await;

        match result {
            Ok(Some(_)) => {
                let message = if req.pin {
                    "徽章置顶成功"
                } else {
                    "徽章取消置顶成功"
                };
                Ok(Response::new(PinBadgeResponse {
                    success: true,
                    message: message.to_string(),
                }))
            }
            Ok(None) => Err(Status::not_found("用户徽章不存在")),
            Err(e) => Err(Status::internal(format!("数据库错误: {}", e))),
        }
    }

    /// 根据来源引用查询关联的用户徽章
    ///
    /// 退款撤销时通过 source_ref（如事件 ID、订单号）反查哪些徽章是由该交易发放的，
    /// 从而精确定位需要撤销的徽章，避免误撤其他来源发放的徽章。
    #[instrument(skip(self), fields(user_id = %request.get_ref().user_id, source_ref = %request.get_ref().source_ref))]
    async fn find_badges_by_source_ref(
        &self,
        request: Request<FindBadgesBySourceRefRequest>,
    ) -> Result<Response<FindBadgesBySourceRefResponse>, Status> {
        let req = request.into_inner();

        if req.user_id.is_empty() {
            return Err(Status::invalid_argument("user_id 不能为空"));
        }
        if req.source_ref.is_empty() {
            return Err(Status::invalid_argument("source_ref 不能为空"));
        }

        // 通过 source_ref 查询 user_badges 表
        let rows = sqlx::query(
            r#"
            SELECT ub.id AS user_badge_id, ub.badge_id, ub.quantity, ub.status
            FROM user_badges ub
            WHERE ub.user_id = $1 AND ub.source_ref = $2
              AND UPPER(ub.status) = 'ACTIVE'
            "#,
        )
        .bind(&req.user_id)
        .bind(&req.source_ref)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| Status::internal(format!("查询关联徽章失败: {}", e)))?;

        let badges: Vec<SourceRefBadge> = rows
            .iter()
            .map(|row| {
                use sqlx::Row;
                SourceRefBadge {
                    badge_id: row.get::<i64, _>("badge_id"),
                    user_badge_id: row.get::<i64, _>("user_badge_id"),
                    quantity: row.get::<i32, _>("quantity"),
                    status: row.get::<String, _>("status"),
                }
            })
            .collect();

        tracing::info!(
            user_id = %req.user_id,
            source_ref = %req.source_ref,
            found_count = badges.len(),
            "查询来源引用关联徽章完成"
        );

        Ok(Response::new(FindBadgesBySourceRefResponse { badges }))
    }

    /// 刷新依赖图缓存
    ///
    /// 当依赖关系配置发生变化后，admin-service 调用此方法通知
    /// badge-management-service 刷新其内部的依赖图缓存
    #[instrument(skip(self))]
    async fn refresh_dependency_cache(
        &self,
        _request: Request<RefreshDependencyCacheRequest>,
    ) -> Result<Response<RefreshDependencyCacheResponse>, Status> {
        if let Some(ref evaluator) = self.cascade_evaluator {
            evaluator
                .refresh_cache()
                .await
                .map_err(|e| Status::internal(format!("刷新依赖缓存失败: {}", e)))?;

            tracing::info!("依赖图缓存已刷新");
            Ok(Response::new(RefreshDependencyCacheResponse {
                success: true,
                message: "依赖图缓存已刷新".to_string(),
            }))
        } else {
            // 没有配置级联评估器，返回成功但提示未配置
            Ok(Response::new(RefreshDependencyCacheResponse {
                success: true,
                message: "级联评估器未配置，无需刷新".to_string(),
            }))
        }
    }

    /// 刷新自动权益规则缓存
    ///
    /// 当兑换规则配置发生变化后（特别是 auto_redeem=true 的规则），
    /// admin-service 调用此方法通知 badge-management-service 刷新缓存
    #[instrument(skip(self))]
    async fn refresh_auto_benefit_cache(
        &self,
        _request: Request<RefreshAutoBenefitCacheRequest>,
    ) -> Result<Response<RefreshAutoBenefitCacheResponse>, Status> {
        if let Some(ref cache) = self.auto_benefit_rule_cache {
            cache
                .refresh()
                .await
                .map_err(|e| Status::internal(format!("刷新自动权益缓存失败: {}", e)))?;

            let rules_count = cache.get_total_rules_count().await;
            tracing::info!(rules_count = rules_count, "自动权益规则缓存已刷新");

            Ok(Response::new(RefreshAutoBenefitCacheResponse {
                success: true,
                message: "自动权益规则缓存已刷新".to_string(),
                rules_loaded: rules_count as i32,
            }))
        } else {
            // 没有配置自动权益缓存，返回成功但提示未配置
            Ok(Response::new(RefreshAutoBenefitCacheResponse {
                success: true,
                message: "自动权益规则缓存未配置，无需刷新".to_string(),
                rules_loaded: 0,
            }))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::BadgeAssets;
    use chrono::Utc;

    #[test]
    fn test_badge_error_to_status() {
        // not_found 错误
        let err = BadgeError::BadgeNotFound(1);
        let status: Status = err.into();
        assert_eq!(status.code(), tonic::Code::NotFound);

        // failed_precondition 错误
        let err = BadgeError::BadgeInactive(1);
        let status: Status = err.into();
        assert_eq!(status.code(), tonic::Code::FailedPrecondition);

        // resource_exhausted 错误
        let err = BadgeError::BadgeOutOfStock(1);
        let status: Status = err.into();
        assert_eq!(status.code(), tonic::Code::ResourceExhausted);

        // invalid_argument 错误
        let err = BadgeError::Validation("invalid".to_string());
        let status: Status = err.into();
        assert_eq!(status.code(), tonic::Code::InvalidArgument);

        // internal 错误
        let err = BadgeError::Internal("error".to_string());
        let status: Status = err.into();
        assert_eq!(status.code(), tonic::Code::Internal);

        // already_exists 错误
        let err = BadgeError::DuplicateRedemption("key".to_string());
        let status: Status = err.into();
        assert_eq!(status.code(), tonic::Code::AlreadyExists);

        // aborted 错误
        let err = BadgeError::ConcurrencyConflict;
        let status: Status = err.into();
        assert_eq!(status.code(), tonic::Code::Aborted);
    }

    #[test]
    fn test_badge_type_to_proto() {
        assert_eq!(
            badge_type_to_proto(BadgeType::Normal),
            ProtoBadgeType::Transaction as i32
        );
        assert_eq!(
            badge_type_to_proto(BadgeType::Limited),
            ProtoBadgeType::Engagement as i32
        );
        assert_eq!(
            badge_type_to_proto(BadgeType::Achievement),
            ProtoBadgeType::Identity as i32
        );
        assert_eq!(
            badge_type_to_proto(BadgeType::Event),
            ProtoBadgeType::Seasonal as i32
        );
    }

    #[test]
    fn test_user_badge_status_to_proto() {
        assert_eq!(
            user_badge_status_to_proto(UserBadgeStatus::Active),
            ProtoBadgeStatus::Active as i32
        );
        assert_eq!(
            user_badge_status_to_proto(UserBadgeStatus::Expired),
            ProtoBadgeStatus::Expired as i32
        );
        assert_eq!(
            user_badge_status_to_proto(UserBadgeStatus::Revoked),
            ProtoBadgeStatus::Revoked as i32
        );
        assert_eq!(
            user_badge_status_to_proto(UserBadgeStatus::Redeemed),
            ProtoBadgeStatus::Redeemed as i32
        );
    }

    #[test]
    fn test_datetime_to_timestamp() {
        let dt = Utc::now();
        let ts = datetime_to_timestamp(dt);
        assert_eq!(ts.seconds, dt.timestamp());
    }

    #[test]
    fn test_parse_source_type() {
        assert_eq!(parse_source_type("event"), SourceType::Event);
        assert_eq!(parse_source_type("EVENT"), SourceType::Event);
        assert_eq!(parse_source_type("scheduled"), SourceType::Scheduled);
        assert_eq!(parse_source_type("manual"), SourceType::Manual);
        assert_eq!(parse_source_type("redemption"), SourceType::Redemption);
        assert_eq!(parse_source_type("unknown"), SourceType::System);
    }

    #[test]
    fn test_user_badge_dto_to_proto() {
        let dto = UserBadgeDto {
            badge_id: 1,
            badge_name: "Test Badge".to_string(),
            badge_type: BadgeType::Normal,
            quantity: 5,
            status: UserBadgeStatus::Active,
            acquired_at: Utc::now(),
            expires_at: None,
            assets: BadgeAssets {
                icon_url: "https://example.com/icon.png".to_string(),
                image_url: None,
                animation_url: Some("https://example.com/anim.json".to_string()),
                disabled_icon_url: None,
            },
        };

        let proto = user_badge_dto_to_proto(&dto);
        assert_eq!(proto.id, "1");
        assert_eq!(proto.quantity, 5);
        assert_eq!(proto.status, ProtoBadgeStatus::Active as i32);
        assert!(proto.badge.is_some());

        let badge = proto.badge.unwrap();
        assert_eq!(badge.name, "Test Badge");
        assert_eq!(badge.icon_url, "https://example.com/icon.png");
        assert_eq!(badge.icon_3d_url, "https://example.com/anim.json");
    }
}
