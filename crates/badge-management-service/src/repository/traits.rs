//! 仓储 Trait 定义
//!
//! 定义仓储接口，便于服务层依赖抽象而非具体实现，支持 mock 测试

use async_trait::async_trait;

use crate::error::Result;
use crate::models::{
    Badge, BadgeCategory, BadgeLedger, BadgeRedemptionRule, BadgeRule, BadgeSeries, Benefit,
    OrderStatus, RedemptionDetail, RedemptionOrder, UserBadge, UserBadgeStatus,
};

/// 徽章仓储接口
#[cfg_attr(test, mockall::automock)]
#[async_trait]
pub trait BadgeRepositoryTrait: Send + Sync {
    // 徽章分类
    async fn get_category(&self, id: i64) -> Result<Option<BadgeCategory>>;
    async fn get_categories_by_ids(&self, ids: &[i64]) -> Result<Vec<BadgeCategory>>;
    async fn list_categories(&self) -> Result<Vec<BadgeCategory>>;

    // 徽章系列
    async fn get_series(&self, id: i64) -> Result<Option<BadgeSeries>>;
    async fn get_series_by_ids(&self, ids: &[i64]) -> Result<Vec<BadgeSeries>>;
    async fn list_series_by_category(&self, category_id: i64) -> Result<Vec<BadgeSeries>>;

    // 徽章
    async fn get_badge(&self, id: i64) -> Result<Option<Badge>>;
    async fn get_badges_by_ids(&self, ids: &[i64]) -> Result<Vec<Badge>>;
    async fn list_badges_by_series(&self, series_id: i64) -> Result<Vec<Badge>>;
    async fn list_active_badges(&self) -> Result<Vec<Badge>>;

    // 规则
    async fn get_badge_rules(&self, badge_id: i64) -> Result<Vec<BadgeRule>>;

    // 库存
    async fn increment_issued_count(&self, badge_id: i64, delta: i64) -> Result<()>;
}

/// 用户徽章仓储接口
#[cfg_attr(test, mockall::automock)]
#[async_trait]
pub trait UserBadgeRepositoryTrait: Send + Sync {
    async fn get_user_badge(&self, user_id: &str, badge_id: i64) -> Result<Option<UserBadge>>;
    async fn get_user_badge_by_id(&self, id: i64) -> Result<Option<UserBadge>>;
    async fn list_user_badges(&self, user_id: &str) -> Result<Vec<UserBadge>>;
    async fn list_user_badges_by_status(
        &self,
        user_id: &str,
        status: UserBadgeStatus,
    ) -> Result<Vec<UserBadge>>;
    async fn create_user_badge(&self, badge: &UserBadge) -> Result<i64>;
    async fn update_user_badge(&self, badge: &UserBadge) -> Result<()>;
    async fn update_user_badge_quantity(&self, id: i64, delta: i32) -> Result<()>;
}

/// 徽章账本仓储接口
#[cfg_attr(test, mockall::automock)]
#[async_trait]
pub trait BadgeLedgerRepositoryTrait: Send + Sync {
    async fn create(&self, ledger: &BadgeLedger) -> Result<i64>;
    async fn list_by_user(&self, user_id: &str, limit: i64) -> Result<Vec<BadgeLedger>>;
    async fn list_by_user_badge(&self, user_id: &str, badge_id: i64) -> Result<Vec<BadgeLedger>>;
    async fn get_balance(&self, user_id: &str, badge_id: i64) -> Result<i32>;
    async fn get_all_balances(&self, user_id: &str) -> Result<Vec<(i64, i32)>>;
}

/// 兑换仓储接口
#[cfg_attr(test, mockall::automock)]
#[async_trait]
pub trait RedemptionRepositoryTrait: Send + Sync {
    // 权益
    async fn get_benefit(&self, id: i64) -> Result<Option<Benefit>>;
    async fn list_benefits(&self) -> Result<Vec<Benefit>>;
    async fn increment_redeemed_count(&self, benefit_id: i64, delta: i64) -> Result<()>;

    // 兑换规则
    async fn get_redemption_rule(&self, id: i64) -> Result<Option<BadgeRedemptionRule>>;
    async fn list_rules_by_badge(&self, badge_id: i64) -> Result<Vec<BadgeRedemptionRule>>;
    async fn list_active_rules(&self) -> Result<Vec<BadgeRedemptionRule>>;

    // 兑换订单
    async fn create_order(&self, order: &RedemptionOrder) -> Result<i64>;
    async fn get_order(&self, id: i64) -> Result<Option<RedemptionOrder>>;
    async fn get_order_by_no(&self, order_no: &str) -> Result<Option<RedemptionOrder>>;
    async fn get_order_by_idempotency_key(
        &self,
        idempotency_key: &str,
    ) -> Result<Option<RedemptionOrder>>;
    async fn update_order_status(
        &self,
        id: i64,
        status: OrderStatus,
        failure_reason: Option<String>,
    ) -> Result<()>;
    async fn update_order_benefit_result(
        &self,
        id: i64,
        benefit_result: &serde_json::Value,
    ) -> Result<()>;
    async fn list_orders_by_user(&self, user_id: &str, limit: i64) -> Result<Vec<RedemptionOrder>>;
    async fn count_user_redemptions(
        &self,
        user_id: &str,
        rule_id: i64,
        since: Option<chrono::DateTime<chrono::Utc>>,
    ) -> Result<i64>;

    // 兑换明细
    async fn create_detail(&self, detail: &RedemptionDetail) -> Result<i64>;
    async fn list_details_by_order(&self, order_id: i64) -> Result<Vec<RedemptionDetail>>;
}
