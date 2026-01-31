//! 规则校验器

/// 规则校验器
///
/// 对规则进行时间窗口、用户限额、全局配额等校验。
pub struct RuleValidator;

impl RuleValidator {
    pub fn new() -> Self {
        Self
    }
}

impl Default for RuleValidator {
    fn default() -> Self {
        Self::new()
    }
}
