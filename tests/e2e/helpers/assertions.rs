//! 自定义断言宏和辅助函数
//!
//! 提供针对 Badge 系统的专用断言功能。

/// 断言用户拥有指定徽章
#[macro_export]
macro_rules! assert_user_has_badge {
    ($db:expr, $user_id:expr, $badge_id:expr) => {
        let has_badge = $db.user_has_badge($user_id, $badge_id).await.unwrap();
        assert!(has_badge, "用户 {} 应该拥有徽章 {}", $user_id, $badge_id);
    };
}

/// 断言用户不拥有指定徽章
#[macro_export]
macro_rules! assert_user_not_has_badge {
    ($db:expr, $user_id:expr, $badge_id:expr) => {
        let has_badge = $db.user_has_badge($user_id, $badge_id).await.unwrap();
        assert!(!has_badge, "用户 {} 不应该拥有徽章 {}", $user_id, $badge_id);
    };
}

/// 断言用户徽章数量
#[macro_export]
macro_rules! assert_badge_quantity {
    ($db:expr, $user_id:expr, $badge_id:expr, $expected:expr) => {
        let quantity = $db.get_user_badge_count($user_id, $badge_id).await.unwrap();
        assert_eq!(
            quantity, $expected,
            "用户 {} 的徽章 {} 数量应为 {}，实际为 {}",
            $user_id, $badge_id, $expected, quantity
        );
    };
}

/// 断言权益已发放
#[macro_export]
macro_rules! assert_benefit_granted {
    ($db:expr, $user_id:expr, $benefit_id:expr) => {
        let granted = $db.benefit_granted($user_id, $benefit_id).await.unwrap();
        assert!(
            granted,
            "用户 {} 的权益 {} 应该已发放",
            $user_id, $benefit_id
        );
    };
}

/// 断言账本记录存在
#[macro_export]
macro_rules! assert_ledger_entry {
    ($db:expr, $user_id:expr, $badge_id:expr, $action:expr, $delta:expr) => {
        let ledger = $db.get_badge_ledger($badge_id, $user_id).await.unwrap();
        let entry = ledger
            .iter()
            .find(|e| e.action == $action && e.delta == $delta);
        assert!(
            entry.is_some(),
            "账本中应该有 action={}, delta={} 的记录",
            $action,
            $delta
        );
    };
}

/// 断言通知已发送
#[macro_export]
macro_rules! assert_notification_sent {
    ($notifications:expr, $user_id:expr, $notification_type:expr) => {
        let found = $notifications
            .iter()
            .any(|n| n.user_id == $user_id && n.notification_type == $notification_type);
        assert!(
            found,
            "应该发送 {} 类型的通知给用户 {}",
            $notification_type, $user_id
        );
    };
}

/// 断言 API 响应成功
#[macro_export]
macro_rules! assert_api_success {
    ($result:expr) => {
        assert!($result.is_ok(), "API 调用应该成功: {:?}", $result.err());
    };
}

/// 断言 API 响应失败
#[macro_export]
macro_rules! assert_api_error {
    ($result:expr) => {
        assert!($result.is_err(), "API 调用应该失败，但返回了成功");
    };
}

/// 等待条件满足（带超时）
pub async fn wait_until<F, Fut>(condition: F, timeout: std::time::Duration) -> bool
where
    F: Fn() -> Fut,
    Fut: std::future::Future<Output = bool>,
{
    let deadline = tokio::time::Instant::now() + timeout;

    while tokio::time::Instant::now() < deadline {
        if condition().await {
            return true;
        }
        tokio::time::sleep(std::time::Duration::from_millis(100)).await;
    }

    false
}

/// 重试执行（用于最终一致性场景）
pub async fn retry_until_ok<F, Fut, T, E>(
    operation: F,
    max_retries: usize,
    delay: std::time::Duration,
) -> Result<T, E>
where
    F: Fn() -> Fut,
    Fut: std::future::Future<Output = Result<T, E>>,
{
    let mut last_error = None;

    for _ in 0..max_retries {
        match operation().await {
            Ok(result) => return Ok(result),
            Err(e) => {
                last_error = Some(e);
                tokio::time::sleep(delay).await;
            }
        }
    }

    Err(last_error.unwrap())
}

/// 生成测试用户 ID
pub fn test_user_id(suffix: &str) -> String {
    format!("test_user_{}", suffix)
}

/// 生成测试订单 ID
pub fn test_order_id(suffix: &str) -> String {
    format!("test_order_{}", suffix)
}

/// 生成唯一测试 ID
pub fn unique_test_id() -> String {
    format!("test_{}", uuid::Uuid::now_v7())
}
