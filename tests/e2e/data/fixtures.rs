//! 测试数据 Fixtures
//!
//! 预定义的测试数据，用于快速创建测试场景。

use super::super::helpers::{
    CreateBadgeRequest, CreateBenefitRequest, CreateCategoryRequest, CreateRuleRequest,
    CreateSeriesRequest,
};
use serde_json::json;

/// 测试分类
pub struct TestCategories;

impl TestCategories {
    /// 成就分类
    pub fn achievement() -> CreateCategoryRequest {
        CreateCategoryRequest {
            name: "Test成就徽章".to_string(),
            description: Some("测试用成就分类".to_string()),
            icon_url: Some("https://example.com/achievement.png".to_string()),
            parent_id: None,
        }
    }

    /// 活动分类
    pub fn event() -> CreateCategoryRequest {
        CreateCategoryRequest {
            name: "Test活动徽章".to_string(),
            description: Some("测试用活动分类".to_string()),
            icon_url: Some("https://example.com/event.png".to_string()),
            parent_id: None,
        }
    }

    /// 消费分类
    pub fn consumption() -> CreateCategoryRequest {
        CreateCategoryRequest {
            name: "Test消费徽章".to_string(),
            description: Some("测试用消费分类".to_string()),
            icon_url: Some("https://example.com/consumption.png".to_string()),
            parent_id: None,
        }
    }
}

/// 测试系列
pub struct TestSeries;

impl TestSeries {
    /// 新手系列
    pub fn newcomer(category_id: i64) -> CreateSeriesRequest {
        CreateSeriesRequest {
            category_id,
            name: "Test新手之路".to_string(),
            description: Some("测试用新手系列".to_string()),
            theme: Some("green".to_string()),
        }
    }

    /// 消费系列
    pub fn spending(category_id: i64) -> CreateSeriesRequest {
        CreateSeriesRequest {
            category_id,
            name: "Test消费达人".to_string(),
            description: Some("测试用消费系列".to_string()),
            theme: Some("gold".to_string()),
        }
    }

    /// 签到系列
    pub fn checkin(category_id: i64) -> CreateSeriesRequest {
        CreateSeriesRequest {
            category_id,
            name: "Test签到之星".to_string(),
            description: Some("测试用签到系列".to_string()),
            theme: Some("blue".to_string()),
        }
    }
}

/// 测试徽章
pub struct TestBadges;

impl TestBadges {
    /// 首次购买徽章
    pub fn first_purchase(series_id: i64) -> CreateBadgeRequest {
        CreateBadgeRequest {
            series_id,
            name: "Test首次购买".to_string(),
            description: Some("完成首次购买获得".to_string()),
            badge_type: "normal".to_string(),
            icon_url: Some("https://example.com/first_purchase.png".to_string()),
            max_supply: None,
        }
    }

    /// 累计消费 1000 徽章
    pub fn spending_1000(series_id: i64) -> CreateBadgeRequest {
        CreateBadgeRequest {
            series_id,
            name: "Test千元达人".to_string(),
            description: Some("累计消费满 1000 元获得".to_string()),
            badge_type: "achievement".to_string(),
            icon_url: Some("https://example.com/spending_1000.png".to_string()),
            max_supply: None,
        }
    }

    /// 累计消费 5000 徽章
    pub fn spending_5000(series_id: i64) -> CreateBadgeRequest {
        CreateBadgeRequest {
            series_id,
            name: "Test五千达人".to_string(),
            description: Some("累计消费满 5000 元获得".to_string()),
            badge_type: "achievement".to_string(),
            icon_url: Some("https://example.com/spending_5000.png".to_string()),
            max_supply: None,
        }
    }

    /// 限量徽章
    pub fn limited_edition(series_id: i64, supply: i64) -> CreateBadgeRequest {
        CreateBadgeRequest {
            series_id,
            name: "Test限量珍藏".to_string(),
            description: Some("限量版徽章".to_string()),
            badge_type: "limited".to_string(),
            icon_url: Some("https://example.com/limited.png".to_string()),
            max_supply: Some(supply),
        }
    }

    /// 签到徽章
    pub fn checkin_7days(series_id: i64) -> CreateBadgeRequest {
        CreateBadgeRequest {
            series_id,
            name: "Test周签到".to_string(),
            description: Some("连续签到 7 天获得".to_string()),
            badge_type: "normal".to_string(),
            icon_url: Some("https://example.com/checkin_7.png".to_string()),
            max_supply: None,
        }
    }

    /// 分享徽章
    pub fn share_master(series_id: i64) -> CreateBadgeRequest {
        CreateBadgeRequest {
            series_id,
            name: "Test分享达人".to_string(),
            description: Some("分享 3 次获得".to_string()),
            badge_type: "normal".to_string(),
            icon_url: Some("https://example.com/share.png".to_string()),
            max_supply: None,
        }
    }
}

/// 测试规则
pub struct TestRules;

impl TestRules {
    /// 首次购买规则
    pub fn first_purchase(badge_id: i64) -> CreateRuleRequest {
        CreateRuleRequest {
            badge_id,
            rule_code: format!("test_first_purchase_{}", badge_id),
            name: "Test首次购买规则".to_string(),
            event_type: "purchase".to_string(),
            rule_json: json!({
                "type": "condition",
                "field": "purchase_count",
                "operator": "eq",
                "value": 1
            }),
            start_time: None,
            end_time: None,
            max_count_per_user: Some(1),
            global_quota: None,
        }
    }

    /// 累计消费规则
    pub fn total_spending(badge_id: i64, amount: i64) -> CreateRuleRequest {
        CreateRuleRequest {
            badge_id,
            rule_code: format!("test_spending_{}_{}", amount, badge_id),
            name: format!("Test累计消费{}规则", amount),
            event_type: "purchase".to_string(),
            rule_json: json!({
                "type": "condition",
                "field": "total_amount",
                "operator": "gte",
                "value": amount
            }),
            start_time: None,
            end_time: None,
            max_count_per_user: Some(1),
            global_quota: None,
        }
    }

    /// 单笔消费规则
    pub fn single_order(badge_id: i64, amount: i64) -> CreateRuleRequest {
        CreateRuleRequest {
            badge_id,
            rule_code: format!("test_single_order_{}_{}", amount, badge_id),
            name: format!("Test单笔消费{}规则", amount),
            event_type: "purchase".to_string(),
            rule_json: json!({
                "type": "condition",
                "field": "order.amount",
                "operator": "gte",
                "value": amount
            }),
            start_time: None,
            end_time: None,
            max_count_per_user: None,
            global_quota: None,
        }
    }

    /// 连续签到规则
    pub fn consecutive_checkin(badge_id: i64, days: i32) -> CreateRuleRequest {
        CreateRuleRequest {
            badge_id,
            rule_code: format!("test_checkin_{}_{}", days, badge_id),
            name: format!("Test连续签到{}天规则", days),
            event_type: "checkin".to_string(),
            rule_json: json!({
                "type": "condition",
                "field": "consecutive_days",
                "operator": "gte",
                "value": days
            }),
            start_time: None,
            end_time: None,
            max_count_per_user: Some(1),
            global_quota: None,
        }
    }

    /// 分享次数规则
    pub fn share_count(badge_id: i64, count: i32) -> CreateRuleRequest {
        CreateRuleRequest {
            badge_id,
            rule_code: format!("test_share_{}_{}", count, badge_id),
            name: format!("Test分享{}次规则", count),
            event_type: "share".to_string(),
            rule_json: json!({
                "type": "condition",
                "field": "share_count",
                "operator": "gte",
                "value": count
            }),
            start_time: None,
            end_time: None,
            max_count_per_user: Some(1),
            global_quota: None,
        }
    }

    /// 组合规则 (AND)
    pub fn combined_and(badge_id: i64, conditions: Vec<serde_json::Value>) -> CreateRuleRequest {
        CreateRuleRequest {
            badge_id,
            rule_code: format!("test_combined_and_{}", badge_id),
            name: "Test组合规则(AND)".to_string(),
            event_type: "purchase".to_string(),
            rule_json: json!({
                "type": "group",
                "operator": "AND",
                "children": conditions
            }),
            start_time: None,
            end_time: None,
            max_count_per_user: Some(1),
            global_quota: None,
        }
    }

    /// 带配额限制的规则
    pub fn with_quota(badge_id: i64, quota: i32) -> CreateRuleRequest {
        CreateRuleRequest {
            badge_id,
            rule_code: format!("test_quota_{}_{}", quota, badge_id),
            name: "Test限量规则".to_string(),
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
            global_quota: Some(quota),
        }
    }
}

/// 测试权益
pub struct TestBenefits;

impl TestBenefits {
    /// 积分权益
    pub fn points(amount: i32) -> CreateBenefitRequest {
        CreateBenefitRequest {
            name: format!("Test{}积分", amount),
            benefit_type: "points".to_string(),
            config: json!({
                "points_amount": amount
            }),
        }
    }

    /// 优惠券权益
    pub fn coupon(template_id: &str, validity_days: i32) -> CreateBenefitRequest {
        CreateBenefitRequest {
            name: "Test优惠券".to_string(),
            benefit_type: "coupon".to_string(),
            config: json!({
                "coupon_template_id": template_id,
                "validity_days": validity_days
            }),
        }
    }

    /// 实物权益
    pub fn physical(sku_id: &str) -> CreateBenefitRequest {
        CreateBenefitRequest {
            name: "Test实物奖品".to_string(),
            benefit_type: "physical".to_string(),
            config: json!({
                "sku_id": sku_id,
                "shipping_required": true
            }),
        }
    }
}

/// 完整测试场景数据
pub struct TestScenario {
    pub category: CreateCategoryRequest,
    pub series: CreateSeriesRequest,
    pub badges: Vec<CreateBadgeRequest>,
    pub rules: Vec<CreateRuleRequest>,
    pub benefits: Vec<CreateBenefitRequest>,
}

impl TestScenario {
    /// 消费升级场景
    pub fn spending_upgrade() -> Self {
        // 这个场景需要在创建后填充 ID
        Self {
            category: TestCategories::consumption(),
            series: CreateSeriesRequest {
                category_id: 0, // 需要后续填充
                name: "Test消费升级".to_string(),
                description: Some("消费升级测试场景".to_string()),
                theme: Some("gold".to_string()),
            },
            badges: vec![], // 需要后续填充
            rules: vec![],  // 需要后续填充
            benefits: vec![TestBenefits::points(100), TestBenefits::points(500)],
        }
    }
}
