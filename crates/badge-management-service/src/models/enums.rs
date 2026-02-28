//! 徽章服务枚举类型定义
//!
//! 所有枚举都支持数据库（sqlx）和 JSON（serde）序列化

use serde::{Deserialize, Serialize};

/// 徽章类型
///
/// 区分不同性质的徽章，影响徽章的获取方式和展示逻辑
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize, sqlx::Type)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
#[sqlx(type_name = "varchar", rename_all = "SCREAMING_SNAKE_CASE")]
pub enum BadgeType {
    /// 普通徽章 - 常规活动可获取
    #[default]
    Normal,
    /// 限定徽章 - 限时/限量，强调稀缺性
    Limited,
    /// 成就徽章 - 达成特定条件自动授予
    Achievement,
    /// 活动徽章 - 特定活动期间发放
    Event,
}

/// 徽章状态（运营侧）
///
/// 控制徽章是否对用户可见和可获取
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize, sqlx::Type)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
#[sqlx(type_name = "varchar", rename_all = "lowercase")]
pub enum BadgeStatus {
    /// 草稿 - 配置中，不对用户展示
    #[default]
    Draft,
    /// 已上线 - 正常展示和发放
    Active,
    /// 已下线 - 停止发放，已获取的仍可展示
    Inactive,
    /// 已归档 - 历史数据，不展示
    Archived,
}

/// 用户徽章状态
///
/// 追踪用户持有徽章的生命周期
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize, sqlx::Type)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
#[sqlx(type_name = "varchar", rename_all = "lowercase")]
pub enum UserBadgeStatus {
    /// 有效 - 正常持有中
    #[default]
    Active,
    /// 已过期 - 超过有效期
    Expired,
    /// 已取消 - 被系统或运营撤回
    Revoked,
    /// 已兑换 - 用于兑换权益（可部分兑换）
    Redeemed,
}

/// 有效期类型
///
/// 决定徽章过期时间的计算方式
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize, sqlx::Type)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
#[sqlx(type_name = "varchar", rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ValidityType {
    /// 永久有效 - 无过期时间
    #[default]
    Permanent,
    /// 固定日期 - 所有用户在同一时间过期
    FixedDate,
    /// 相对天数 - 从获取时起算的相对有效期
    RelativeDays,
}

/// 账本变动类型
///
/// 采用复式记账思想，记录徽章数量的每一次变动
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, sqlx::Type)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
#[sqlx(type_name = "varchar", rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ChangeType {
    /// 获取（+）- 通过任何方式获得徽章
    Acquire,
    /// 过期（-）- 徽章有效期结束
    Expire,
    /// 取消（-）- 运营或系统撤回徽章
    Cancel,
    /// 兑换消耗（-）- 用于兑换权益
    RedeemOut,
    /// 兑换回滚（+）- 兑换失败退回
    RedeemFail,
}

impl ChangeType {
    /// 返回该变动类型的数量符号
    /// 正数表示增加，负数表示减少
    pub fn sign(&self) -> i32 {
        match self {
            Self::Acquire | Self::RedeemFail => 1,
            Self::Expire | Self::Cancel | Self::RedeemOut => -1,
        }
    }
}

/// 来源/关联类型
///
/// 标识徽章变动的触发来源，用于追溯和审计
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize, sqlx::Type)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
#[sqlx(type_name = "varchar", rename_all = "SCREAMING_SNAKE_CASE")]
pub enum SourceType {
    /// 事件触发 - 用户行为触发规则引擎
    Event,
    /// 定时任务 - 批量发放或过期处理
    Scheduled,
    /// 手动发放 - 运营后台操作
    Manual,
    /// 兑换 - 兑换流程产生
    Redemption,
    /// 级联触发 - 依赖关系自动触发的徽章授予
    Cascade,
    /// 系统操作 - 系统自动处理
    #[default]
    System,
}

/// 权益类型
///
/// 定义徽章可兑换的权益种类，不同类型有不同的发放和撤销特性
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, sqlx::Type)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
#[sqlx(type_name = "varchar", rename_all = "SCREAMING_SNAKE_CASE")]
pub enum BenefitType {
    /// 数字资产 - NFT、虚拟物品等
    DigitalAsset,
    /// 优惠券 - 折扣券、满减券等，支持同步发放
    Coupon,
    /// 预约资格 - VIP 通道、优先预约等
    Reservation,
    /// 积分 - 可累积使用，支持同步发放
    Points,
    /// 实物奖品 - 需要物流配送，发放后不可撤销
    Physical,
    /// 会员权益 - 会员等级、VIP 资格等
    Membership,
    /// 外部回调 - 通用接口，由外部系统处理具体逻辑
    ExternalCallback,
}

impl BenefitType {
    /// 判断该权益类型是否支持同步发放
    ///
    /// 同步类型可以在请求中立即完成发放，适合响应时间敏感的场景
    pub fn is_sync(&self) -> bool {
        matches!(self, Self::Coupon | Self::Points)
    }

    /// 判断该权益类型是否支持撤销
    ///
    /// 实物奖品一旦发出无法收回，其他虚拟权益均可撤销
    pub fn is_revocable(&self) -> bool {
        !matches!(self, Self::Physical)
    }
}

/// 权益发放状态
///
/// 追踪单次权益发放的处理进度
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize, sqlx::Type)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
#[sqlx(type_name = "varchar", rename_all = "SCREAMING_SNAKE_CASE")]
pub enum GrantStatus {
    /// 待处理 - 发放请求已接收，等待执行
    #[default]
    Pending,
    /// 处理中 - 正在执行发放操作（异步场景）
    Processing,
    /// 成功 - 发放完成
    Success,
    /// 失败 - 发放失败（权益系统错误、用户状态异常等）
    Failed,
    /// 已撤销 - 发放后被撤回
    Revoked,
}

/// 撤销原因
///
/// 记录权益被撤销的具体原因，用于审计和统计分析
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, sqlx::Type)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
#[sqlx(type_name = "varchar", rename_all = "SCREAMING_SNAKE_CASE")]
pub enum RevokeReason {
    /// 用户主动退回 - 用户发起的退还请求
    UserRequest,
    /// 订单退款 - 关联订单退款触发的权益收回
    OrderRefund,
    /// 过期清理 - 权益有效期结束
    Expiration,
    /// 违规收回 - 用户违反使用规则
    Violation,
    /// 系统错误修正 - 发放错误后的人工修正
    SystemError,
}

/// 兑换订单状态
///
/// 追踪兑换订单的处理进度
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize, sqlx::Type)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
#[sqlx(type_name = "varchar", rename_all = "SCREAMING_SNAKE_CASE")]
pub enum OrderStatus {
    /// 待处理 - 订单已创建，等待执行
    #[default]
    Pending,
    /// 成功 - 兑换完成
    Success,
    /// 失败 - 兑换失败（库存不足、权益发放失败等）
    Failed,
    /// 已取消 - 用户或系统取消
    Cancelled,
}

/// 日志动作类型
///
/// 用于用户徽章操作日志的分类
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, sqlx::Type)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
#[sqlx(type_name = "varchar", rename_all = "SCREAMING_SNAKE_CASE")]
pub enum LogAction {
    /// 发放
    Grant,
    /// 取消/撤回
    Revoke,
    /// 兑换
    Redeem,
    /// 过期
    Expire,
}

/// 发放对象类型
///
/// 区分"账号注册人"和"实际使用人"，支持代领场景
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize, sqlx::Type)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
#[sqlx(type_name = "varchar", rename_all = "SCREAMING_SNAKE_CASE")]
pub enum RecipientType {
    /// 账号注册人（默认，原有行为）
    #[default]
    Owner,
    /// 实际使用人（需要填写 actual_user_id）
    User,
}

/// 兑换规则有效期类型
///
/// 控制兑换规则的时间窗口计算方式（区别于徽章自身的 ValidityType）
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize, sqlx::Type)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
#[sqlx(type_name = "varchar", rename_all = "SCREAMING_SNAKE_CASE")]
pub enum RedemptionValidityType {
    /// 固定时间段（使用 start_time/end_time）
    #[default]
    Fixed,
    /// 相对于徽章获取时间
    Relative,
}

/// 分类/系列状态
///
/// 控制徽章分类和系列的可见性
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize, sqlx::Type)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
#[sqlx(type_name = "varchar", rename_all = "lowercase")]
pub enum CategoryStatus {
    /// 启用 - 正常展示
    #[default]
    Active,
    /// 禁用 - 不展示
    Inactive,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_badge_type_serialization() {
        assert_eq!(
            serde_json::to_string(&BadgeType::Limited).unwrap(),
            "\"LIMITED\""
        );
        assert_eq!(
            serde_json::from_str::<BadgeType>("\"ACHIEVEMENT\"").unwrap(),
            BadgeType::Achievement
        );
    }

    #[test]
    fn test_change_type_sign() {
        assert_eq!(ChangeType::Acquire.sign(), 1);
        assert_eq!(ChangeType::RedeemOut.sign(), -1);
        assert_eq!(ChangeType::RedeemFail.sign(), 1);
    }

    #[test]
    fn test_user_badge_status_default() {
        assert_eq!(UserBadgeStatus::default(), UserBadgeStatus::Active);
    }

    #[test]
    fn test_benefit_type_is_sync() {
        // 同步发放类型
        assert!(BenefitType::Coupon.is_sync());
        assert!(BenefitType::Points.is_sync());

        // 异步发放类型
        assert!(!BenefitType::DigitalAsset.is_sync());
        assert!(!BenefitType::Reservation.is_sync());
        assert!(!BenefitType::Physical.is_sync());
        assert!(!BenefitType::Membership.is_sync());
        assert!(!BenefitType::ExternalCallback.is_sync());
    }

    #[test]
    fn test_benefit_type_is_revocable() {
        // 可撤销类型
        assert!(BenefitType::Coupon.is_revocable());
        assert!(BenefitType::Points.is_revocable());
        assert!(BenefitType::DigitalAsset.is_revocable());
        assert!(BenefitType::Reservation.is_revocable());
        assert!(BenefitType::Membership.is_revocable());
        assert!(BenefitType::ExternalCallback.is_revocable());

        // 不可撤销类型
        assert!(!BenefitType::Physical.is_revocable());
    }

    #[test]
    fn test_benefit_type_serialization() {
        assert_eq!(
            serde_json::to_string(&BenefitType::ExternalCallback).unwrap(),
            "\"EXTERNAL_CALLBACK\""
        );
        assert_eq!(
            serde_json::from_str::<BenefitType>("\"POINTS\"").unwrap(),
            BenefitType::Points
        );
    }

    #[test]
    fn test_grant_status_default() {
        assert_eq!(GrantStatus::default(), GrantStatus::Pending);
    }

    #[test]
    fn test_grant_status_serialization() {
        assert_eq!(
            serde_json::to_string(&GrantStatus::Processing).unwrap(),
            "\"PROCESSING\""
        );
        assert_eq!(
            serde_json::from_str::<GrantStatus>("\"REVOKED\"").unwrap(),
            GrantStatus::Revoked
        );
    }

    #[test]
    fn test_revoke_reason_serialization() {
        assert_eq!(
            serde_json::to_string(&RevokeReason::OrderRefund).unwrap(),
            "\"ORDER_REFUND\""
        );
        assert_eq!(
            serde_json::from_str::<RevokeReason>("\"SYSTEM_ERROR\"").unwrap(),
            RevokeReason::SystemError
        );
    }
}
