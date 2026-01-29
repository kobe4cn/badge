//! 预定义场景集合
//!
//! 提供一组开箱即用的标准测试场景，覆盖常见的用户行为流程。
//! 这些场景可直接用于功能测试、演示和压力测试。

use super::scenario::{Scenario, ScenarioStep};

/// 预定义场景集合
///
/// 提供常见业务场景的预配置定义，无需手动构建。
/// 每个场景模拟一种典型的用户行为模式。
pub struct PredefinedScenarios;

impl PredefinedScenarios {
    /// 新用户首购场景
    ///
    /// 模拟新用户注册后的首次购物流程：
    /// 1. 浏览首页
    /// 2. 浏览商品详情
    /// 3. 完成首次购买
    /// 4. 完成首日签到
    ///
    /// 此场景可触发「首次购物」和「新用户签到」等徽章。
    pub fn first_purchase() -> Scenario {
        let user_id = "new-user-001".to_string();

        Scenario {
            name: "first_purchase".to_string(),
            description: "新用户首购场景：浏览 -> 首次购买 -> 签到".to_string(),
            steps: vec![
                ScenarioStep::PageView {
                    user_id: user_id.clone(),
                    page_url: "https://shop.example.com/".to_string(),
                    delay_ms: Some(100),
                },
                ScenarioStep::PageView {
                    user_id: user_id.clone(),
                    page_url: "https://shop.example.com/product/detail/123".to_string(),
                    delay_ms: Some(200),
                },
                ScenarioStep::Purchase {
                    user_id: user_id.clone(),
                    amount: 99.00,
                    delay_ms: Some(100),
                },
                ScenarioStep::CheckIn {
                    user_id,
                    consecutive_days: 1,
                    delay_ms: None,
                },
            ],
        }
    }

    /// VIP 升级场景
    ///
    /// 模拟用户通过连续大额购买升级为 VIP：
    /// 5 次购买，每次金额递增，模拟消费能力增长。
    ///
    /// 此场景可触发「累计消费」「VIP 升级」等徽章。
    pub fn vip_upgrade() -> Scenario {
        let user_id = "vip-user-001".to_string();
        let amounts = [500.0, 800.0, 1200.0, 1500.0, 2000.0];

        let steps = amounts
            .iter()
            .map(|&amount| ScenarioStep::Purchase {
                user_id: user_id.clone(),
                amount,
                delay_ms: Some(100),
            })
            .collect();

        Scenario {
            name: "vip_upgrade".to_string(),
            description: "VIP 升级场景：连续 5 次大额购买".to_string(),
            steps,
        }
    }

    /// 连续签到场景
    ///
    /// 模拟用户连续 7 天签到，使用 Repeat 步骤简化定义。
    /// 每次签到的连续天数递增（1, 2, 3, ..., 7）。
    ///
    /// 此场景可触发「连续签到 3 天」「连续签到 7 天」等徽章。
    pub fn consecutive_checkin() -> Scenario {
        let user_id = "checkin-user-001".to_string();

        // 手动构建 7 天签到，因为每天的 consecutive_days 不同
        let steps: Vec<ScenarioStep> = (1..=7)
            .map(|day| ScenarioStep::CheckIn {
                user_id: user_id.clone(),
                consecutive_days: day,
                delay_ms: Some(50),
            })
            .collect();

        Scenario {
            name: "consecutive_checkin".to_string(),
            description: "连续签到场景：7 天连续签到".to_string(),
            steps,
        }
    }

    /// 退款场景
    ///
    /// 模拟完整的购买-退款流程：
    /// 1. 用户购买商品
    /// 2. 等待一段时间（模拟收货、使用）
    /// 3. 申请退款
    ///
    /// 此场景用于测试徽章回收逻辑，确保因购买获得的徽章在退款后被正确撤销。
    pub fn refund_flow() -> Scenario {
        let user_id = "refund-user-001".to_string();
        let order_id = "ORD-REFUND-001".to_string();

        Scenario {
            name: "refund_flow".to_string(),
            description: "退款场景：购买 -> 等待 -> 退款".to_string(),
            steps: vec![
                ScenarioStep::Purchase {
                    user_id: user_id.clone(),
                    amount: 299.00,
                    delay_ms: None,
                },
                ScenarioStep::Wait { duration_ms: 500 },
                ScenarioStep::Refund {
                    user_id,
                    order_id,
                    amount: 299.00,
                    badge_ids: vec![1, 2], // 假设购买触发了 ID 为 1 和 2 的徽章
                    delay_ms: None,
                },
            ],
        }
    }

    /// 活跃用户场景
    ///
    /// 模拟高活跃度用户的多样化行为：
    /// - 签到
    /// - 浏览多个页面
    /// - 购买
    /// - 分享到多个平台
    /// - 再次签到
    ///
    /// 此场景覆盖多种事件类型，适合全面测试事件处理管道。
    pub fn active_user() -> Scenario {
        let user_id = "active-user-001".to_string();

        Scenario {
            name: "active_user".to_string(),
            description: "活跃用户场景：签到、浏览、购买、分享等多种行为".to_string(),
            steps: vec![
                // 早晨签到
                ScenarioStep::CheckIn {
                    user_id: user_id.clone(),
                    consecutive_days: 5,
                    delay_ms: Some(50),
                },
                // 浏览首页
                ScenarioStep::PageView {
                    user_id: user_id.clone(),
                    page_url: "https://shop.example.com/".to_string(),
                    delay_ms: Some(50),
                },
                // 浏览促销活动
                ScenarioStep::PageView {
                    user_id: user_id.clone(),
                    page_url: "https://shop.example.com/promotion/spring-sale".to_string(),
                    delay_ms: Some(50),
                },
                // 浏览商品详情
                ScenarioStep::PageView {
                    user_id: user_id.clone(),
                    page_url: "https://shop.example.com/product/detail/456".to_string(),
                    delay_ms: Some(50),
                },
                // 购买
                ScenarioStep::Purchase {
                    user_id: user_id.clone(),
                    amount: 199.00,
                    delay_ms: Some(100),
                },
                // 分享到微信
                ScenarioStep::Share {
                    user_id: user_id.clone(),
                    platform: "wechat".to_string(),
                    delay_ms: Some(50),
                },
                // 分享到微博
                ScenarioStep::Share {
                    user_id: user_id.clone(),
                    platform: "weibo".to_string(),
                    delay_ms: Some(50),
                },
                // 晚上再次签到（用于测试同日重复签到处理）
                ScenarioStep::CheckIn {
                    user_id,
                    consecutive_days: 5,
                    delay_ms: None,
                },
            ],
        }
    }

    /// 社交达人场景
    ///
    /// 模拟用户频繁分享行为，覆盖多个社交平台。
    /// 此场景可触发「分享达人」「全平台分享」等徽章。
    pub fn social_butterfly() -> Scenario {
        let user_id = "social-user-001".to_string();
        let platforms = ["wechat", "weibo", "qq", "douyin", "xiaohongshu"];

        let steps: Vec<ScenarioStep> = platforms
            .iter()
            .map(|&platform| ScenarioStep::Share {
                user_id: user_id.clone(),
                platform: platform.to_string(),
                delay_ms: Some(100),
            })
            .collect();

        Scenario {
            name: "social_butterfly".to_string(),
            description: "社交达人场景：分享到所有社交平台".to_string(),
            steps,
        }
    }

    /// 获取所有预定义场景
    ///
    /// 返回全部预定义场景的列表，便于批量测试或生成场景目录。
    pub fn all() -> Vec<Scenario> {
        vec![
            Self::first_purchase(),
            Self::vip_upgrade(),
            Self::consecutive_checkin(),
            Self::refund_flow(),
            Self::active_user(),
            Self::social_butterfly(),
        ]
    }

    /// 根据名称获取场景
    ///
    /// 支持通过场景名称动态查找，便于命令行工具按名称执行场景。
    pub fn get(name: &str) -> Option<Scenario> {
        match name {
            "first_purchase" => Some(Self::first_purchase()),
            "vip_upgrade" => Some(Self::vip_upgrade()),
            "consecutive_checkin" => Some(Self::consecutive_checkin()),
            "refund_flow" => Some(Self::refund_flow()),
            "active_user" => Some(Self::active_user()),
            "social_butterfly" => Some(Self::social_butterfly()),
            _ => None,
        }
    }

    /// 获取所有场景名称
    ///
    /// 用于命令行帮助信息或自动补全。
    pub fn names() -> Vec<&'static str> {
        vec![
            "first_purchase",
            "vip_upgrade",
            "consecutive_checkin",
            "refund_flow",
            "active_user",
            "social_butterfly",
        ]
    }
}

// ---------------------------------------------------------------------------
// 单元测试
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::scenarios::scenario::ScenarioStep;

    #[test]
    fn test_first_purchase_scenario() {
        let scenario = PredefinedScenarios::first_purchase();

        assert_eq!(scenario.name, "first_purchase");
        assert!(!scenario.description.is_empty());
        assert_eq!(scenario.steps.len(), 4);

        // 验证步骤类型顺序
        assert!(matches!(&scenario.steps[0], ScenarioStep::PageView { .. }));
        assert!(matches!(&scenario.steps[1], ScenarioStep::PageView { .. }));
        assert!(matches!(&scenario.steps[2], ScenarioStep::Purchase { .. }));
        assert!(matches!(&scenario.steps[3], ScenarioStep::CheckIn { .. }));
    }

    #[test]
    fn test_vip_upgrade_scenario() {
        let scenario = PredefinedScenarios::vip_upgrade();

        assert_eq!(scenario.name, "vip_upgrade");
        assert_eq!(scenario.steps.len(), 5);

        // 验证所有步骤都是购买
        for step in &scenario.steps {
            assert!(matches!(step, ScenarioStep::Purchase { .. }));
        }

        // 验证金额递增
        let amounts: Vec<f64> = scenario
            .steps
            .iter()
            .filter_map(|s| {
                if let ScenarioStep::Purchase { amount, .. } = s {
                    Some(*amount)
                } else {
                    None
                }
            })
            .collect();

        assert_eq!(amounts, vec![500.0, 800.0, 1200.0, 1500.0, 2000.0]);
    }

    #[test]
    fn test_consecutive_checkin_scenario() {
        let scenario = PredefinedScenarios::consecutive_checkin();

        assert_eq!(scenario.name, "consecutive_checkin");
        assert_eq!(scenario.steps.len(), 7);

        // 验证连续天数递增 1-7
        for (i, step) in scenario.steps.iter().enumerate() {
            match step {
                ScenarioStep::CheckIn {
                    consecutive_days, ..
                } => {
                    assert_eq!(*consecutive_days, (i + 1) as i32);
                }
                _ => panic!("预期所有步骤都是 CheckIn"),
            }
        }
    }

    #[test]
    fn test_refund_flow_scenario() {
        let scenario = PredefinedScenarios::refund_flow();

        assert_eq!(scenario.name, "refund_flow");
        assert_eq!(scenario.steps.len(), 3);

        assert!(matches!(&scenario.steps[0], ScenarioStep::Purchase { .. }));
        assert!(matches!(&scenario.steps[1], ScenarioStep::Wait { .. }));
        assert!(matches!(&scenario.steps[2], ScenarioStep::Refund { .. }));

        // 验证退款金额与购买金额一致
        if let (
            ScenarioStep::Purchase {
                amount: purchase_amount,
                ..
            },
            ScenarioStep::Refund {
                amount: refund_amount,
                ..
            },
        ) = (&scenario.steps[0], &scenario.steps[2])
        {
            assert!((purchase_amount - refund_amount).abs() < f64::EPSILON);
        } else {
            panic!("步骤类型不匹配");
        }
    }

    #[test]
    fn test_active_user_scenario() {
        let scenario = PredefinedScenarios::active_user();

        assert_eq!(scenario.name, "active_user");
        assert_eq!(scenario.steps.len(), 8);

        // 统计各类型步骤数量
        let checkin_count = scenario
            .steps
            .iter()
            .filter(|s| matches!(s, ScenarioStep::CheckIn { .. }))
            .count();
        let page_view_count = scenario
            .steps
            .iter()
            .filter(|s| matches!(s, ScenarioStep::PageView { .. }))
            .count();
        let purchase_count = scenario
            .steps
            .iter()
            .filter(|s| matches!(s, ScenarioStep::Purchase { .. }))
            .count();
        let share_count = scenario
            .steps
            .iter()
            .filter(|s| matches!(s, ScenarioStep::Share { .. }))
            .count();

        assert_eq!(checkin_count, 2);
        assert_eq!(page_view_count, 3);
        assert_eq!(purchase_count, 1);
        assert_eq!(share_count, 2);
    }

    #[test]
    fn test_predefined_scenarios_all() {
        let all = PredefinedScenarios::all();

        assert_eq!(all.len(), 6);

        let names: Vec<&str> = all.iter().map(|s| s.name.as_str()).collect();
        assert!(names.contains(&"first_purchase"));
        assert!(names.contains(&"vip_upgrade"));
        assert!(names.contains(&"consecutive_checkin"));
        assert!(names.contains(&"refund_flow"));
        assert!(names.contains(&"active_user"));
        assert!(names.contains(&"social_butterfly"));
    }

    #[test]
    fn test_get_predefined_scenario() {
        // 存在的场景
        assert!(PredefinedScenarios::get("first_purchase").is_some());
        assert!(PredefinedScenarios::get("vip_upgrade").is_some());
        assert!(PredefinedScenarios::get("consecutive_checkin").is_some());
        assert!(PredefinedScenarios::get("refund_flow").is_some());
        assert!(PredefinedScenarios::get("active_user").is_some());
        assert!(PredefinedScenarios::get("social_butterfly").is_some());

        // 不存在的场景
        assert!(PredefinedScenarios::get("non_existent").is_none());
        assert!(PredefinedScenarios::get("").is_none());
    }

    #[test]
    fn test_scenario_names() {
        let names = PredefinedScenarios::names();

        assert_eq!(names.len(), 6);
        assert!(names.contains(&"first_purchase"));
        assert!(names.contains(&"social_butterfly"));
    }

    #[test]
    fn test_social_butterfly_scenario() {
        let scenario = PredefinedScenarios::social_butterfly();

        assert_eq!(scenario.name, "social_butterfly");
        assert_eq!(scenario.steps.len(), 5);

        // 验证覆盖所有平台
        let platforms: Vec<&str> = scenario
            .steps
            .iter()
            .filter_map(|s| {
                if let ScenarioStep::Share { platform, .. } = s {
                    Some(platform.as_str())
                } else {
                    None
                }
            })
            .collect();

        assert!(platforms.contains(&"wechat"));
        assert!(platforms.contains(&"weibo"));
        assert!(platforms.contains(&"qq"));
        assert!(platforms.contains(&"douyin"));
        assert!(platforms.contains(&"xiaohongshu"));
    }
}
