//! 规则与徽章的映射管理

use super::BadgeGrant;
use std::collections::HashMap;

/// 规则与徽章的映射表
///
/// 提供按事件类型快速查找匹配规则的能力。
#[derive(Debug, Default)]
pub struct RuleBadgeMapping {
    /// 按事件类型索引的规则列表
    by_event_type: HashMap<String, Vec<BadgeGrant>>,
}

impl RuleBadgeMapping {
    pub fn new() -> Self {
        Self::default()
    }
}
