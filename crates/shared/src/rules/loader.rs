//! 规则加载器

/// 规则加载器
///
/// 从数据库加载规则配置并维护内存缓存。
pub struct RuleLoader;

impl RuleLoader {
    pub fn new() -> Self {
        Self
    }
}

impl Default for RuleLoader {
    fn default() -> Self {
        Self::new()
    }
}
