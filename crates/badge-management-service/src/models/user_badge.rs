//! 用户徽章相关实体定义
//!
//! 包含用户持有徽章、操作日志、账本流水等

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use super::enums::{ChangeType, LogAction, SourceType, UserBadgeStatus};

/// 用户徽章
///
/// 记录用户持有的徽章实例，支持同一徽章多数量持有
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
#[serde(rename_all = "camelCase")]
pub struct UserBadge {
    pub id: i64,
    /// 用户 ID
    pub user_id: String,
    /// 徽章定义 ID
    pub badge_id: i64,
    /// 徽章状态
    pub status: UserBadgeStatus,
    /// 持有数量（支持同一徽章多次获取）
    pub quantity: i32,
    /// 获取时间
    pub acquired_at: DateTime<Utc>,
    /// 过期时间（null 表示永久有效）
    #[sqlx(default)]
    pub expires_at: Option<DateTime<Utc>>,
    /// 发放来源
    pub source_type: SourceType,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl UserBadge {
    /// 检查徽章是否已过期
    pub fn is_expired(&self, now: DateTime<Utc>) -> bool {
        self.expires_at.is_some_and(|t| now > t)
    }

    /// 检查徽章是否有效可用
    pub fn is_valid(&self, now: DateTime<Utc>) -> bool {
        self.status == UserBadgeStatus::Active && !self.is_expired(now)
    }

    /// 获取可用数量（考虑状态）
    pub fn available_quantity(&self, now: DateTime<Utc>) -> i32 {
        if self.is_valid(now) { self.quantity } else { 0 }
    }
}

/// 用户徽章操作日志
///
/// 记录徽章的发放、取消、兑换等操作，用于审计追踪
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
#[serde(rename_all = "camelCase")]
pub struct UserBadgeLog {
    pub id: i64,
    /// 关联的用户徽章 ID
    pub user_badge_id: i64,
    /// 用户 ID（冗余存储，便于查询）
    pub user_id: String,
    /// 徽章 ID（冗余存储）
    pub badge_id: i64,
    /// 操作动作
    pub action: LogAction,
    /// 操作原因/备注
    #[sqlx(default)]
    pub reason: Option<String>,
    /// 操作人（系统操作时为 "SYSTEM"）
    #[sqlx(default)]
    pub operator: Option<String>,
    /// 操作涉及的数量
    pub quantity: i32,
    /// 操作来源
    pub source_type: SourceType,
    /// 关联的业务 ID（如事件 ID、订单 ID）
    #[sqlx(default)]
    pub source_ref_id: Option<String>,
    pub created_at: DateTime<Utc>,
}

/// 徽章账本（流水）
///
/// 采用复式记账思想，记录徽章数量的每一次变动
/// 每条记录包含变动类型、数量和变动后余额，确保数据一致性可追溯
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
#[serde(rename_all = "camelCase")]
pub struct BadgeLedger {
    pub id: i64,
    /// 用户 ID
    pub user_id: String,
    /// 徽章 ID
    pub badge_id: i64,
    /// 变动类型
    pub change_type: ChangeType,
    /// 变动数量（始终为正数，符号由 change_type 决定）
    pub quantity: i32,
    /// 变动后的余额
    pub balance_after: i32,
    /// 关联的业务 ID
    #[sqlx(default)]
    pub ref_id: Option<String>,
    /// 关联类型
    pub ref_type: SourceType,
    /// 备注
    #[sqlx(default)]
    pub remark: Option<String>,
    pub created_at: DateTime<Utc>,
}

impl BadgeLedger {
    /// 计算实际变动值（带符号）
    pub fn signed_quantity(&self) -> i32 {
        self.quantity * self.change_type.sign()
    }

    /// 创建获取记录的构建器
    pub fn acquire(user_id: String, badge_id: i64, quantity: i32, balance_after: i32) -> Self {
        Self {
            id: 0,
            user_id,
            badge_id,
            change_type: ChangeType::Acquire,
            quantity,
            balance_after,
            ref_id: None,
            ref_type: SourceType::System,
            remark: None,
            created_at: Utc::now(),
        }
    }

    /// 创建兑换消耗记录
    pub fn redeem_out(
        user_id: String,
        badge_id: i64,
        quantity: i32,
        balance_after: i32,
        order_id: String,
    ) -> Self {
        Self {
            id: 0,
            user_id,
            badge_id,
            change_type: ChangeType::RedeemOut,
            quantity,
            balance_after,
            ref_id: Some(order_id),
            ref_type: SourceType::Redemption,
            remark: None,
            created_at: Utc::now(),
        }
    }
}

/// 用户徽章汇总视图
///
/// 用于展示用户的徽章统计信息，非数据库实体
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UserBadgeSummary {
    /// 用户 ID
    pub user_id: String,
    /// 总徽章数（去重后的徽章种类数）
    pub total_badge_types: i64,
    /// 总徽章数量
    pub total_quantity: i64,
    /// 有效徽章数量
    pub active_quantity: i64,
    /// 已过期数量
    pub expired_quantity: i64,
    /// 已兑换数量
    pub redeemed_quantity: i64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_user_badge_is_expired() {
        let now = Utc::now();
        let mut badge = create_test_user_badge();

        // 无过期时间
        badge.expires_at = None;
        assert!(!badge.is_expired(now));

        // 未过期
        badge.expires_at = Some(now + chrono::Duration::days(1));
        assert!(!badge.is_expired(now));

        // 已过期
        badge.expires_at = Some(now - chrono::Duration::days(1));
        assert!(badge.is_expired(now));
    }

    #[test]
    fn test_user_badge_is_valid() {
        let now = Utc::now();
        let mut badge = create_test_user_badge();

        // 有效状态且未过期
        badge.status = UserBadgeStatus::Active;
        badge.expires_at = None;
        assert!(badge.is_valid(now));

        // 有效状态但已过期
        badge.expires_at = Some(now - chrono::Duration::hours(1));
        assert!(!badge.is_valid(now));

        // 无效状态
        badge.status = UserBadgeStatus::Revoked;
        badge.expires_at = None;
        assert!(!badge.is_valid(now));
    }

    #[test]
    fn test_badge_ledger_signed_quantity() {
        let mut ledger = create_test_ledger();

        ledger.change_type = ChangeType::Acquire;
        ledger.quantity = 5;
        assert_eq!(ledger.signed_quantity(), 5);

        ledger.change_type = ChangeType::RedeemOut;
        assert_eq!(ledger.signed_quantity(), -5);

        ledger.change_type = ChangeType::RedeemFail;
        assert_eq!(ledger.signed_quantity(), 5);
    }

    #[test]
    fn test_badge_ledger_builders() {
        let ledger = BadgeLedger::acquire("user-1".to_string(), 1, 3, 10);
        assert_eq!(ledger.change_type, ChangeType::Acquire);
        assert_eq!(ledger.quantity, 3);
        assert_eq!(ledger.balance_after, 10);

        let ledger =
            BadgeLedger::redeem_out("user-1".to_string(), 1, 2, 8, "order-123".to_string());
        assert_eq!(ledger.change_type, ChangeType::RedeemOut);
        assert_eq!(ledger.ref_id, Some("order-123".to_string()));
        assert_eq!(ledger.ref_type, SourceType::Redemption);
    }

    fn create_test_user_badge() -> UserBadge {
        UserBadge {
            id: 1,
            user_id: "user-123".to_string(),
            badge_id: 1,
            status: UserBadgeStatus::Active,
            quantity: 1,
            acquired_at: Utc::now(),
            expires_at: None,
            source_type: SourceType::Manual,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        }
    }

    fn create_test_ledger() -> BadgeLedger {
        BadgeLedger {
            id: 1,
            user_id: "user-123".to_string(),
            badge_id: 1,
            change_type: ChangeType::Acquire,
            quantity: 1,
            balance_after: 1,
            ref_id: None,
            ref_type: SourceType::System,
            remark: None,
            created_at: Utc::now(),
        }
    }
}
