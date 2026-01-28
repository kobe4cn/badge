//! 通知模板管理
//!
//! 根据通知类型生成对应的标题和正文内容。
//! 当前使用硬编码模板以降低外部依赖，未来可扩展为
//! 从数据库或配置中心动态加载模板。

use badge_shared::events::NotificationType;

/// 通知模板引擎
///
/// 根据通知类型生成标题和内容。当前版本使用硬编码模板，
/// 未来可扩展为从数据库或配置中心加载。
pub struct NotificationTemplateEngine {
    // 预留扩展字段，用于后续添加模板缓存或外部配置源
}

impl NotificationTemplateEngine {
    pub fn new() -> Self {
        Self {}
    }

    /// 根据通知类型和上下文数据渲染标题
    ///
    /// 标题保持简洁固定，不做变量替换，便于客户端聚合展示同类通知
    pub fn render_title(notification_type: &NotificationType, _data: &serde_json::Value) -> String {
        match notification_type {
            NotificationType::BadgeGranted => "恭喜获得新徽章".to_string(),
            NotificationType::BadgeExpiring => "徽章即将过期".to_string(),
            NotificationType::BadgeRevoked => "徽章已被回收".to_string(),
            NotificationType::RedemptionSuccess => "兑换成功".to_string(),
            NotificationType::RedemptionFailed => "兑换失败".to_string(),
        }
    }

    /// 根据通知类型和上下文数据渲染正文
    ///
    /// 从 data 中提取业务字段填充模板。对于缺失字段使用默认占位符，
    /// 避免因上游数据不完整导致通知发送失败。
    pub fn render_body(notification_type: &NotificationType, data: &serde_json::Value) -> String {
        match notification_type {
            NotificationType::BadgeGranted => {
                let badge_name = extract_str(data, "badge_name", "未知徽章");
                format!("您已获得「{badge_name}」徽章！")
            }
            NotificationType::BadgeExpiring => {
                let badge_name = extract_str(data, "badge_name", "未知徽章");
                let days = extract_str(data, "days", "?");
                format!("您的「{badge_name}」徽章将在 {days} 天后过期")
            }
            NotificationType::BadgeRevoked => {
                let badge_name = extract_str(data, "badge_name", "未知徽章");
                let reason = extract_str(data, "reason", "未知原因");
                format!("您的「{badge_name}」徽章已被回收，原因：{reason}")
            }
            NotificationType::RedemptionSuccess => {
                let badge_name = extract_str(data, "badge_name", "未知徽章");
                let benefit_name = extract_str(data, "benefit_name", "未知权益");
                format!("您已成功使用「{badge_name}」徽章兑换「{benefit_name}」")
            }
            NotificationType::RedemptionFailed => {
                let reason = extract_str(data, "reason", "未知原因");
                format!("您的兑换请求未能成功，原因：{reason}")
            }
        }
    }
}

impl Default for NotificationTemplateEngine {
    fn default() -> Self {
        Self::new()
    }
}

/// 从 JSON 对象中安全提取字符串值
///
/// 优先取字符串类型的值，对数值类型自动转换为字符串表示，
/// 确保模板渲染不会因类型不匹配而 panic。
fn extract_str<'a>(data: &'a serde_json::Value, key: &str, default: &'a str) -> String {
    data.get(key)
        .map(|v| match v {
            serde_json::Value::String(s) => s.clone(),
            // 数值等非字符串类型也能安全渲染
            other => other.to_string(),
        })
        .unwrap_or_else(|| default.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_render_badge_granted() {
        let data = serde_json::json!({
            "badge_name": "首次购物"
        });

        let title =
            NotificationTemplateEngine::render_title(&NotificationType::BadgeGranted, &data);
        assert_eq!(title, "恭喜获得新徽章");

        let body = NotificationTemplateEngine::render_body(&NotificationType::BadgeGranted, &data);
        assert_eq!(body, "您已获得「首次购物」徽章！");
    }

    #[test]
    fn test_render_badge_expiring() {
        let data = serde_json::json!({
            "badge_name": "年度会员",
            "days": "7"
        });

        let title =
            NotificationTemplateEngine::render_title(&NotificationType::BadgeExpiring, &data);
        assert_eq!(title, "徽章即将过期");

        let body = NotificationTemplateEngine::render_body(&NotificationType::BadgeExpiring, &data);
        assert_eq!(body, "您的「年度会员」徽章将在 7 天后过期");
    }

    #[test]
    fn test_render_badge_revoked() {
        let data = serde_json::json!({
            "badge_name": "VIP 徽章",
            "reason": "会员已过期"
        });

        let title =
            NotificationTemplateEngine::render_title(&NotificationType::BadgeRevoked, &data);
        assert_eq!(title, "徽章已被回收");

        let body = NotificationTemplateEngine::render_body(&NotificationType::BadgeRevoked, &data);
        assert_eq!(body, "您的「VIP 徽章」徽章已被回收，原因：会员已过期");
    }

    #[test]
    fn test_render_redemption_success() {
        let data = serde_json::json!({
            "badge_name": "限量版徽章",
            "benefit_name": "免费咖啡券"
        });

        let title =
            NotificationTemplateEngine::render_title(&NotificationType::RedemptionSuccess, &data);
        assert_eq!(title, "兑换成功");

        let body =
            NotificationTemplateEngine::render_body(&NotificationType::RedemptionSuccess, &data);
        assert_eq!(body, "您已成功使用「限量版徽章」徽章兑换「免费咖啡券」");
    }

    #[test]
    fn test_render_redemption_failed() {
        let data = serde_json::json!({
            "reason": "徽章余额不足"
        });

        let title =
            NotificationTemplateEngine::render_title(&NotificationType::RedemptionFailed, &data);
        assert_eq!(title, "兑换失败");

        let body =
            NotificationTemplateEngine::render_body(&NotificationType::RedemptionFailed, &data);
        assert_eq!(body, "您的兑换请求未能成功，原因：徽章余额不足");
    }

    #[test]
    fn test_render_with_missing_data_uses_defaults() {
        // 缺少 badge_name 字段时应使用默认值
        let empty_data = serde_json::json!({});

        let body =
            NotificationTemplateEngine::render_body(&NotificationType::BadgeGranted, &empty_data);
        assert_eq!(body, "您已获得「未知徽章」徽章！");
    }

    #[test]
    fn test_render_with_numeric_days() {
        // days 为数值类型时也应正确渲染
        let data = serde_json::json!({
            "badge_name": "季度徽章",
            "days": 3
        });

        let body = NotificationTemplateEngine::render_body(&NotificationType::BadgeExpiring, &data);
        assert_eq!(body, "您的「季度徽章」徽章将在 3 天后过期");
    }
}
