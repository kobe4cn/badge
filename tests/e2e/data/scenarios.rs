//! 业务场景数据
//!
//! 预定义的完整业务场景，用于端到端测试。

use super::fixtures::*;
#[allow(unused_imports)]
use super::generators::*;
use crate::helpers::*;
use serde_json::json;

/// 场景构建器
///
/// 用于在测试中快速构建完整的业务场景。
pub struct ScenarioBuilder<'a> {
    api: &'a ApiClient,
}

impl<'a> ScenarioBuilder<'a> {
    pub fn new(api: &'a ApiClient) -> Self {
        Self { api }
    }

    /// 构建消费升级场景
    ///
    /// 创建三个消费阶梯徽章：500元、1000元、5000元
    pub async fn spending_upgrade(&self) -> anyhow::Result<SpendingUpgradeScenario> {
        // 1. 创建分类
        let category = self
            .api
            .create_category(&TestCategories::consumption())
            .await?;

        // 2. 创建系列
        let series = self
            .api
            .create_series(&TestSeries::spending(category.id))
            .await?;

        // 3. 创建三个消费徽章
        let badge_500 = self
            .api
            .create_badge(&CreateBadgeRequest {
                series_id: series.id,
                name: "Test500元达人".to_string(),
                description: Some("累计消费满 500 元".to_string()),
                badge_type: "achievement".to_string(),
                icon_url: None,
                max_supply: None,
            })
            .await?;

        let badge_1000 = self
            .api
            .create_badge(&CreateBadgeRequest {
                series_id: series.id,
                name: "Test1000元达人".to_string(),
                description: Some("累计消费满 1000 元".to_string()),
                badge_type: "achievement".to_string(),
                icon_url: None,
                max_supply: None,
            })
            .await?;

        let badge_5000 = self
            .api
            .create_badge(&CreateBadgeRequest {
                series_id: series.id,
                name: "Test5000元达人".to_string(),
                description: Some("累计消费满 5000 元".to_string()),
                badge_type: "achievement".to_string(),
                icon_url: None,
                max_supply: None,
            })
            .await?;

        // 4. 创建规则
        let rule_500 = self
            .api
            .create_rule(&TestRules::total_spending(badge_500.id, 500))
            .await?;

        let rule_1000 = self
            .api
            .create_rule(&TestRules::total_spending(badge_1000.id, 1000))
            .await?;

        let rule_5000 = self
            .api
            .create_rule(&TestRules::total_spending(badge_5000.id, 5000))
            .await?;

        // 5. 上线徽章
        self.api.update_badge_status(badge_500.id, "active").await?;
        self.api
            .update_badge_status(badge_1000.id, "active")
            .await?;
        self.api
            .update_badge_status(badge_5000.id, "active")
            .await?;

        Ok(SpendingUpgradeScenario {
            category,
            series,
            badge_500,
            badge_1000,
            badge_5000,
            rule_500,
            rule_1000,
            rule_5000,
        })
    }

    /// 构建签到场景
    pub async fn checkin(&self) -> anyhow::Result<CheckinScenario> {
        let category = self
            .api
            .create_category(&TestCategories::achievement())
            .await?;
        let series = self
            .api
            .create_series(&TestSeries::checkin(category.id))
            .await?;

        let badge_7days = self
            .api
            .create_badge(&TestBadges::checkin_7days(series.id))
            .await?;

        let rule = self
            .api
            .create_rule(&TestRules::consecutive_checkin(badge_7days.id, 7))
            .await?;

        self.api
            .update_badge_status(badge_7days.id, "active")
            .await?;

        Ok(CheckinScenario {
            category,
            series,
            badge_7days,
            rule,
        })
    }

    /// 构建限量抢兑场景
    pub async fn limited_redemption(
        &self,
        supply: i64,
    ) -> anyhow::Result<LimitedRedemptionScenario> {
        let category = self.api.create_category(&TestCategories::event()).await?;
        let series = self
            .api
            .create_series(&CreateSeriesRequest {
                category_id: category.id,
                name: "Test限量活动".to_string(),
                description: Some("限量活动测试".to_string()),
                theme: Some("red".to_string()),
            })
            .await?;

        let badge = self
            .api
            .create_badge(&TestBadges::limited_edition(series.id, supply))
            .await?;

        let rule = self
            .api
            .create_rule(&TestRules::with_quota(badge.id, supply as i32))
            .await?;

        self.api.update_badge_status(badge.id, "active").await?;

        Ok(LimitedRedemptionScenario {
            category,
            series,
            badge,
            rule,
            supply,
        })
    }

    /// 构建级联触发场景
    ///
    /// 创建 A -> B -> C 的级联关系
    pub async fn cascade_chain(&self) -> anyhow::Result<CascadeChainScenario> {
        let category = self
            .api
            .create_category(&TestCategories::achievement())
            .await?;
        let series = self
            .api
            .create_series(&CreateSeriesRequest {
                category_id: category.id,
                name: "Test级联系列".to_string(),
                description: Some("级联测试".to_string()),
                theme: Some("purple".to_string()),
            })
            .await?;

        // 创建三个徽章
        let badge_a = self
            .api
            .create_badge(&CreateBadgeRequest {
                series_id: series.id,
                name: "Test徽章A".to_string(),
                description: Some("入门徽章".to_string()),
                badge_type: "normal".to_string(),
                icon_url: None,
                max_supply: None,
            })
            .await?;

        let badge_b = self
            .api
            .create_badge(&CreateBadgeRequest {
                series_id: series.id,
                name: "Test徽章B".to_string(),
                description: Some("进阶徽章，依赖 A".to_string()),
                badge_type: "normal".to_string(),
                icon_url: None,
                max_supply: None,
            })
            .await?;

        let badge_c = self
            .api
            .create_badge(&CreateBadgeRequest {
                series_id: series.id,
                name: "Test徽章C".to_string(),
                description: Some("高级徽章，依赖 B".to_string()),
                badge_type: "achievement".to_string(),
                icon_url: None,
                max_supply: None,
            })
            .await?;

        // 创建规则
        let rule_a = self
            .api
            .create_rule(&CreateRuleRequest {
                badge_id: badge_a.id,
                rule_code: format!("test_cascade_a_{}", badge_a.id),
                name: "Test徽章A规则".to_string(),
                event_type: "purchase".to_string(),
                rule_json: json!({
                    "type": "condition",
                    "field": "order.amount",
                    "operator": "gte",
                    "value": 100
                }),
                start_time: None,
                end_time: None,
                max_count_per_user: Some(1),
                global_quota: None,
            })
            .await?;

        // B 依赖 A（需要通过依赖配置 API）
        // C 依赖 B

        // 上线徽章
        self.api.update_badge_status(badge_a.id, "active").await?;
        self.api.update_badge_status(badge_b.id, "active").await?;
        self.api.update_badge_status(badge_c.id, "active").await?;

        Ok(CascadeChainScenario {
            category,
            series,
            badge_a,
            badge_b,
            badge_c,
            rule_a,
        })
    }

    /// 构建权益发放场景
    pub async fn benefit_grant(&self) -> anyhow::Result<BenefitGrantScenario> {
        let category = self
            .api
            .create_category(&TestCategories::consumption())
            .await?;
        let series = self
            .api
            .create_series(&TestSeries::spending(category.id))
            .await?;

        let badge = self
            .api
            .create_badge(&TestBadges::first_purchase(series.id))
            .await?;

        let rule = self
            .api
            .create_rule(&TestRules::first_purchase(badge.id))
            .await?;

        // 创建权益
        let benefit_points = self.api.create_benefit(&TestBenefits::points(100)).await?;
        let benefit_coupon = self
            .api
            .create_benefit(&TestBenefits::coupon("TPL_001", 30))
            .await?;

        // TODO: 关联徽章和权益（需要对应 API）

        self.api.update_badge_status(badge.id, "active").await?;

        Ok(BenefitGrantScenario {
            category,
            series,
            badge,
            rule,
            benefit_points,
            benefit_coupon,
        })
    }
}

// ========== 场景数据结构 ==========

/// 消费升级场景
pub struct SpendingUpgradeScenario {
    pub category: CategoryResponse,
    pub series: SeriesResponse,
    pub badge_500: BadgeResponse,
    pub badge_1000: BadgeResponse,
    pub badge_5000: BadgeResponse,
    pub rule_500: RuleResponse,
    pub rule_1000: RuleResponse,
    pub rule_5000: RuleResponse,
}

/// 签到场景
pub struct CheckinScenario {
    pub category: CategoryResponse,
    pub series: SeriesResponse,
    pub badge_7days: BadgeResponse,
    pub rule: RuleResponse,
}

/// 限量抢兑场景
pub struct LimitedRedemptionScenario {
    pub category: CategoryResponse,
    pub series: SeriesResponse,
    pub badge: BadgeResponse,
    pub rule: RuleResponse,
    pub supply: i64,
}

/// 级联触发场景
pub struct CascadeChainScenario {
    pub category: CategoryResponse,
    pub series: SeriesResponse,
    pub badge_a: BadgeResponse,
    pub badge_b: BadgeResponse,
    pub badge_c: BadgeResponse,
    pub rule_a: RuleResponse,
}

/// 权益发放场景
pub struct BenefitGrantScenario {
    pub category: CategoryResponse,
    pub series: SeriesResponse,
    pub badge: BadgeResponse,
    pub rule: RuleResponse,
    pub benefit_points: BenefitResponse,
    pub benefit_coupon: BenefitResponse,
}
