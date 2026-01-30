use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use uuid::Uuid;

/// 依赖类型
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum DependencyType {
    /// 前置条件 - 需要持有此徽章才能获得目标徽章
    Prerequisite,
    /// 消耗 - 需要消耗此徽章才能获得目标徽章
    Consume,
    /// 互斥 - 持有此徽章则不能获得目标徽章
    Exclusive,
}

impl std::str::FromStr for DependencyType {
    type Err = String;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "prerequisite" => Ok(Self::Prerequisite),
            "consume" => Ok(Self::Consume),
            "exclusive" => Ok(Self::Exclusive),
            _ => Err(format!("unknown dependency type: {}", s)),
        }
    }
}

impl DependencyType {
    /// 从字符串解析依赖类型（可选）
    pub fn parse(s: &str) -> Option<Self> {
        s.parse().ok()
    }
}

/// 徽章依赖关系
#[derive(Debug, Clone)]
pub struct BadgeDependency {
    pub id: Uuid,
    /// 目标徽章（获得此徽章需要满足依赖条件）
    pub badge_id: Uuid,
    /// 依赖的徽章
    pub depends_on_badge_id: Uuid,
    /// 依赖类型
    pub dependency_type: DependencyType,
    /// 需要的数量
    pub required_quantity: i32,
    /// 互斥组ID
    pub exclusive_group_id: Option<String>,
    /// 是否自动触发
    pub auto_trigger: bool,
    /// 评估优先级
    pub priority: i32,
    /// 依赖组ID（同组 AND，不同组 OR）
    pub dependency_group_id: String,
}

/// 级联配置
#[derive(Debug, Clone)]
pub struct CascadeConfig {
    /// 最大递归深度
    pub max_depth: u32,
    /// 超时时间（毫秒）
    pub timeout_ms: u64,
    /// 依赖图缓存时间（秒）
    pub graph_cache_seconds: u64,
}

impl Default for CascadeConfig {
    fn default() -> Self {
        Self {
            max_depth: 10,
            timeout_ms: 5000,
            graph_cache_seconds: 300, // 5分钟
        }
    }
}

/// 级联评估上下文
#[derive(Debug, Clone)]
pub struct CascadeContext {
    /// 当前递归深度
    pub depth: u32,
    /// 已访问的徽章（用于循环检测）
    pub visited: HashSet<Uuid>,
    /// 访问路径（用于错误报告）
    pub path: Vec<Uuid>,
    /// 开始时间
    pub started_at: std::time::Instant,
}

impl CascadeContext {
    pub fn new() -> Self {
        Self {
            depth: 0,
            visited: HashSet::new(),
            path: Vec::new(),
            started_at: std::time::Instant::now(),
        }
    }

    /// 进入下一层
    pub fn enter(&mut self, badge_id: Uuid) {
        self.depth += 1;
        self.visited.insert(badge_id);
        self.path.push(badge_id);
    }

    /// 离开当前层
    pub fn leave(&mut self) {
        self.depth = self.depth.saturating_sub(1);
        self.path.pop();
    }

    /// 检查是否存在循环
    pub fn has_cycle(&self, badge_id: Uuid) -> bool {
        self.visited.contains(&badge_id)
    }

    /// 获取已用时间（毫秒）
    pub fn elapsed_ms(&self) -> u64 {
        self.started_at.elapsed().as_millis() as u64
    }
}

impl Default for CascadeContext {
    fn default() -> Self {
        Self::new()
    }
}

/// 级联评估结果
#[derive(Debug, Clone, Default)]
pub struct CascadeResult {
    /// 成功发放的徽章
    pub granted_badges: Vec<GrantedBadge>,
    /// 被阻止的徽章
    pub blocked_badges: Vec<BlockedBadge>,
}

#[derive(Debug, Clone, Serialize)]
pub struct GrantedBadge {
    pub badge_id: Uuid,
    pub badge_name: String,
    pub triggered_by: Uuid,
}

#[derive(Debug, Clone, Serialize)]
pub struct BlockedBadge {
    pub badge_id: Uuid,
    pub badge_name: Option<String>,
    pub reason: BlockReason,
}

#[derive(Debug, Clone, Serialize)]
pub enum BlockReason {
    PrerequisiteNotMet { missing: Vec<Uuid> },
    ExclusiveConflict { conflicting: Uuid },
    CycleDetected,
    DepthExceeded,
    Timeout,
}
