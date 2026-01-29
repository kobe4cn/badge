//! 模拟用户模型
//!
//! 用于测试和开发环境的用户数据结构，支持随机生成以模拟真实业务场景。

use chrono::{DateTime, Utc};
use fake::Fake;
use fake::faker::internet::en::*;
use fake::faker::phone_number::en::*;
use rand::Rng;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// 模拟用户
///
/// 包含用户的基本信息和会员等级，用于模拟外部用户系统
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MockUser {
    pub user_id: String,
    pub username: String,
    pub email: String,
    pub phone: Option<String>,
    pub membership_level: MembershipLevel,
    pub registration_date: DateTime<Utc>,
    pub total_spent: f64,
    pub order_count: i32,
}

/// 会员等级
///
/// 五级会员体系，从青铜到钻石，与消费金额相关联
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum MembershipLevel {
    Bronze,
    Silver,
    Gold,
    Platinum,
    Diamond,
}

impl MockUser {
    /// 生成随机用户
    ///
    /// 用户的会员等级基于消费金额自动确定，模拟真实会员升级逻辑
    pub fn random() -> Self {
        let mut rng = rand::thread_rng();

        // 消费金额决定会员等级
        let total_spent = rng.gen_range(0.0..100000.0);
        let membership_level = MembershipLevel::from_spent(total_spent);

        // 注册时间在过去 1-5 年内
        let days_ago = rng.gen_range(30..1825);
        let registration_date = Utc::now() - chrono::Duration::days(days_ago);

        // 订单数量与消费金额正相关
        let avg_order_value = 200.0;
        let base_order_count = (total_spent / avg_order_value) as i32;
        let order_count = base_order_count.max(1);

        // 70% 的用户提供手机号
        let phone = if rng.gen_bool(0.7) {
            Some(PhoneNumber().fake())
        } else {
            None
        };

        Self {
            user_id: format!("USR-{}", Uuid::new_v4()),
            username: Username().fake(),
            email: SafeEmail().fake(),
            phone,
            membership_level,
            registration_date,
            total_spent,
            order_count,
        }
    }

    /// 使用指定 ID 生成随机用户
    pub fn random_with_id(user_id: &str) -> Self {
        let mut user = Self::random();
        user.user_id = user_id.to_string();
        user
    }
}

impl MembershipLevel {
    /// 根据消费金额确定会员等级
    ///
    /// 等级阈值设计参考常见电商平台：
    /// - Bronze: 0-1000
    /// - Silver: 1000-5000
    /// - Gold: 5000-20000
    /// - Platinum: 20000-50000
    /// - Diamond: 50000+
    pub fn from_spent(amount: f64) -> Self {
        match amount {
            x if x >= 50000.0 => Self::Diamond,
            x if x >= 20000.0 => Self::Platinum,
            x if x >= 5000.0 => Self::Gold,
            x if x >= 1000.0 => Self::Silver,
            _ => Self::Bronze,
        }
    }

    /// 获取等级名称
    pub fn name(&self) -> &'static str {
        match self {
            Self::Bronze => "Bronze",
            Self::Silver => "Silver",
            Self::Gold => "Gold",
            Self::Platinum => "Platinum",
            Self::Diamond => "Diamond",
        }
    }

    /// 获取下一个会员等级
    ///
    /// Diamond 已是最高等级，返回 None
    pub fn next_level(&self) -> Option<Self> {
        match self {
            Self::Bronze => Some(Self::Silver),
            Self::Silver => Some(Self::Gold),
            Self::Gold => Some(Self::Platinum),
            Self::Platinum => Some(Self::Diamond),
            Self::Diamond => None,
        }
    }

    /// 获取下一等级所需的消费金额门槛
    ///
    /// 返回达到下一等级需要的最低消费金额，Diamond 返回 None
    pub fn next_level_threshold(&self) -> Option<f64> {
        match self {
            Self::Bronze => Some(1000.0),
            Self::Silver => Some(5000.0),
            Self::Gold => Some(20000.0),
            Self::Platinum => Some(50000.0),
            Self::Diamond => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_user_random() {
        let user = MockUser::random();

        assert!(user.user_id.starts_with("USR-"));
        assert!(!user.username.is_empty());
        assert!(user.email.contains('@'));
        assert!(user.total_spent >= 0.0);
        assert!(user.order_count >= 1);
    }

    #[test]
    fn test_user_random_with_id() {
        let user_id = "custom-user-id";
        let user = MockUser::random_with_id(user_id);

        assert_eq!(user.user_id, user_id);
    }

    #[test]
    fn test_membership_level_from_spent() {
        assert_eq!(MembershipLevel::from_spent(0.0), MembershipLevel::Bronze);
        assert_eq!(MembershipLevel::from_spent(999.0), MembershipLevel::Bronze);
        assert_eq!(MembershipLevel::from_spent(1000.0), MembershipLevel::Silver);
        assert_eq!(MembershipLevel::from_spent(5000.0), MembershipLevel::Gold);
        assert_eq!(
            MembershipLevel::from_spent(20000.0),
            MembershipLevel::Platinum
        );
        assert_eq!(
            MembershipLevel::from_spent(50000.0),
            MembershipLevel::Diamond
        );
        assert_eq!(
            MembershipLevel::from_spent(100000.0),
            MembershipLevel::Diamond
        );
    }

    #[test]
    fn test_membership_level_serialization() {
        let level = MembershipLevel::Gold;
        let json = serde_json::to_string(&level).unwrap();
        let deserialized: MembershipLevel = serde_json::from_str(&json).unwrap();
        assert_eq!(level, deserialized);
    }
}
