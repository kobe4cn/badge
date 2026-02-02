//! 通知模板引擎
//!
//! 提供模板变量替换功能，支持 `{{variable}}` 语法。
//!
//! ## 使用示例
//!
//! ```ignore
//! let engine = TemplateEngine::with_defaults();
//!
//! let mut context = NotificationContext::new();
//! context.set("badge_name", "首次购物");
//! context.set("user_name", "张三");
//!
//! let rendered = engine.render("恭喜 {{user_name}}，您获得了「{{badge_name}}」徽章！", &context);
//! // 输出: "恭喜 张三，您获得了「首次购物」徽章！"
//! ```

use std::collections::HashMap;

use regex::Regex;
use tracing::warn;

use super::types::NotificationContext;
use badge_shared::events::NotificationType;

/// 模板引擎
///
/// 管理通知模板并提供变量替换功能
pub struct TemplateEngine {
    /// 标题模板（按通知类型）
    title_templates: HashMap<NotificationType, String>,
    /// 正文模板（按通知类型）
    body_templates: HashMap<NotificationType, String>,
    /// 变量匹配正则
    variable_regex: Regex,
}

impl Default for TemplateEngine {
    fn default() -> Self {
        Self::new()
    }
}

impl TemplateEngine {
    /// 创建空的模板引擎
    pub fn new() -> Self {
        Self {
            title_templates: HashMap::new(),
            body_templates: HashMap::new(),
            // 匹配 {{variable_name}} 格式，变量名支持字母、数字、下划线
            variable_regex: Regex::new(r"\{\{(\w+)\}\}").unwrap(),
        }
    }

    /// 创建带有默认模板的引擎
    pub fn with_defaults() -> Self {
        let mut engine = Self::new();
        engine.register_default_templates();
        engine
    }

    /// 注册默认模板
    fn register_default_templates(&mut self) {
        // 徽章获取通知
        self.register_template(
            NotificationType::BadgeGranted,
            "恭喜获得新徽章！",
            "您已获得「{{badge_name}}」徽章，快去看看吧！",
        );

        // 徽章即将过期通知
        self.register_template(
            NotificationType::BadgeExpiring,
            "徽章即将过期提醒",
            "您的「{{badge_name}}」徽章将在 {{days_left}} 天后过期，请及时使用！",
        );

        // 徽章撤销通知
        self.register_template(
            NotificationType::BadgeRevoked,
            "徽章已被撤销",
            "您的「{{badge_name}}」徽章已被撤销，原因：{{reason}}",
        );

        // 兑换成功通知
        self.register_template(
            NotificationType::RedemptionSuccess,
            "兑换成功！",
            "您已成功兑换「{{benefit_name}}」，请查收！",
        );

        // 兑换失败通知
        self.register_template(
            NotificationType::RedemptionFailed,
            "兑换失败",
            "兑换失败，原因：{{reason}}。如有疑问请联系客服。",
        );
    }

    /// 注册模板
    pub fn register_template(
        &mut self,
        notification_type: NotificationType,
        title_template: impl Into<String>,
        body_template: impl Into<String>,
    ) {
        self.title_templates
            .insert(notification_type.clone(), title_template.into());
        self.body_templates
            .insert(notification_type, body_template.into());
    }

    /// 获取模板
    pub fn get_template(&self, notification_type: &NotificationType) -> Option<(&str, &str)> {
        let title = self.title_templates.get(notification_type)?;
        let body = self.body_templates.get(notification_type)?;
        Some((title, body))
    }

    /// 渲染模板
    ///
    /// 将模板中的 `{{variable}}` 替换为上下文中的对应值。
    /// 未找到的变量会保留原样并记录警告日志。
    pub fn render(&self, template: &str, context: &NotificationContext) -> String {
        let result = self
            .variable_regex
            .replace_all(template, |caps: &regex::Captures| {
                let var_name = &caps[1];
                match context.get(var_name) {
                    Some(value) => value.to_string(),
                    None => {
                        warn!(variable = var_name, "模板变量未找到，保留原样");
                        caps[0].to_string()
                    }
                }
            });

        result.into_owned()
    }

    /// 使用变量 HashMap 渲染模板
    pub fn render_with_map(&self, template: &str, variables: &HashMap<String, String>) -> String {
        let context = NotificationContext::from(variables.clone());
        self.render(template, &context)
    }

    /// 渲染标题和正文
    ///
    /// 如果通知类型有对应模板，使用模板渲染；否则使用传入的默认值。
    pub fn render_notification(
        &self,
        notification_type: &NotificationType,
        default_title: &str,
        default_body: &str,
        context: &NotificationContext,
    ) -> (String, String) {
        let (title_template, body_template) = self
            .get_template(notification_type)
            .unwrap_or((default_title, default_body));

        let rendered_title = self.render(title_template, context);
        let rendered_body = self.render(body_template, context);

        (rendered_title, rendered_body)
    }

    /// 验证模板语法
    ///
    /// 检查模板中的变量是否都能在给定的上下文中找到
    pub fn validate_template(&self, template: &str, context: &NotificationContext) -> Vec<String> {
        let mut missing_vars = Vec::new();

        for caps in self.variable_regex.captures_iter(template) {
            let var_name = &caps[1];
            if context.get(var_name).is_none() {
                missing_vars.push(var_name.to_string());
            }
        }

        missing_vars
    }

    /// 提取模板中的所有变量名
    pub fn extract_variables(&self, template: &str) -> Vec<String> {
        self.variable_regex
            .captures_iter(template)
            .map(|caps| caps[1].to_string())
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_template_engine_creation() {
        let engine = TemplateEngine::new();
        assert!(engine.title_templates.is_empty());
        assert!(engine.body_templates.is_empty());
    }

    #[test]
    fn test_template_engine_with_defaults() {
        let engine = TemplateEngine::with_defaults();

        // 验证默认模板已注册
        assert!(engine.get_template(&NotificationType::BadgeGranted).is_some());
        assert!(engine.get_template(&NotificationType::BadgeExpiring).is_some());
        assert!(engine.get_template(&NotificationType::RedemptionSuccess).is_some());
    }

    #[test]
    fn test_register_template() {
        let mut engine = TemplateEngine::new();
        engine.register_template(
            NotificationType::BadgeGranted,
            "自定义标题",
            "自定义正文 {{badge_name}}",
        );

        let (title, body) = engine.get_template(&NotificationType::BadgeGranted).unwrap();
        assert_eq!(title, "自定义标题");
        assert_eq!(body, "自定义正文 {{badge_name}}");
    }

    #[test]
    fn test_render_simple() {
        let engine = TemplateEngine::new();
        let mut context = NotificationContext::new();
        context.set("name", "张三");

        let result = engine.render("你好，{{name}}！", &context);
        assert_eq!(result, "你好，张三！");
    }

    #[test]
    fn test_render_multiple_variables() {
        let engine = TemplateEngine::new();
        let mut context = NotificationContext::new();
        context.set("badge_name", "首次购物");
        context.set("user_name", "李四");

        let result = engine.render(
            "恭喜 {{user_name}}，您获得了「{{badge_name}}」徽章！",
            &context,
        );
        assert_eq!(result, "恭喜 李四，您获得了「首次购物」徽章！");
    }

    #[test]
    fn test_render_missing_variable() {
        let engine = TemplateEngine::new();
        let context = NotificationContext::new();

        let result = engine.render("你好，{{name}}！", &context);
        // 未找到的变量保留原样
        assert_eq!(result, "你好，{{name}}！");
    }

    #[test]
    fn test_render_with_map() {
        let engine = TemplateEngine::new();
        let mut variables = HashMap::new();
        variables.insert("benefit_name".to_string(), "VIP 优惠券".to_string());

        let result = engine.render_with_map("您已成功兑换「{{benefit_name}}」！", &variables);
        assert_eq!(result, "您已成功兑换「VIP 优惠券」！");
    }

    #[test]
    fn test_render_notification() {
        let engine = TemplateEngine::with_defaults();
        let mut context = NotificationContext::new();
        context.set("badge_name", "消费达人");

        let (title, body) = engine.render_notification(
            &NotificationType::BadgeGranted,
            "默认标题",
            "默认正文",
            &context,
        );

        assert_eq!(title, "恭喜获得新徽章！");
        assert!(body.contains("消费达人"));
    }

    #[test]
    fn test_validate_template() {
        let engine = TemplateEngine::new();
        let mut context = NotificationContext::new();
        context.set("badge_name", "测试");

        let missing = engine.validate_template(
            "{{badge_name}} - {{user_name}} - {{unknown}}",
            &context,
        );

        assert_eq!(missing.len(), 2);
        assert!(missing.contains(&"user_name".to_string()));
        assert!(missing.contains(&"unknown".to_string()));
    }

    #[test]
    fn test_extract_variables() {
        let engine = TemplateEngine::new();
        let template = "您好 {{user_name}}，您的「{{badge_name}}」徽章将在 {{days_left}} 天后过期。";

        let variables = engine.extract_variables(template);

        assert_eq!(variables.len(), 3);
        assert!(variables.contains(&"user_name".to_string()));
        assert!(variables.contains(&"badge_name".to_string()));
        assert!(variables.contains(&"days_left".to_string()));
    }

    #[test]
    fn test_render_no_variables() {
        let engine = TemplateEngine::new();
        let context = NotificationContext::new();

        let result = engine.render("这是一条没有变量的消息", &context);
        assert_eq!(result, "这是一条没有变量的消息");
    }

    #[test]
    fn test_render_repeated_variable() {
        let engine = TemplateEngine::new();
        let mut context = NotificationContext::new();
        context.set("name", "测试");

        let result = engine.render("{{name}} loves {{name}}", &context);
        assert_eq!(result, "测试 loves 测试");
    }
}
