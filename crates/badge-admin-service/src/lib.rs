//! 徽章管理后台服务（B端）
//!
//! 提供徽章配置、发放管理、统计报表等 REST API。
//!
//! ## 核心功能
//!
//! - **徽章管理**：徽章的 CRUD 操作，包括分类、系列、徽章定义
//! - **规则配置**：配置徽章的获取规则，与规则引擎集成
//! - **发放管理**：手动发放/取消徽章，支持批量操作
//! - **统计报表**：发放统计、趋势分析等数据看板
//! - **操作日志**：记录所有运营操作，支持审计追溯
//!
//! ## 模块结构
//!
//! - `dto`: 请求和响应的数据传输对象
//! - `models`: B端特有的实体模型
//! - `error`: 错误类型定义
//! - `handlers`: HTTP 请求处理器
//! - `routes`: 路由配置
//! - `state`: 应用状态
//!
//! ## 技术栈
//!
//! - Web 框架：Axum
//! - 数据验证：validator
//! - 序列化：serde (camelCase)

pub mod dto;
pub mod error;
pub mod handlers;
pub mod models;
pub mod routes;
pub mod state;

// 重新导出核心类型
pub use dto::{
    ApiResponse, BadgeAdminDto, BadgeQueryFilter, BatchGrantRequest, BatchTaskDto,
    CreateBadgeRequest, CreateCategoryRequest, CreateRuleRequest, CreateSeriesRequest,
    GrantLogFilter, ManualGrantRequest, ManualRevokeRequest, OperationLogDto, PageResponse,
    PaginationParams, StatsOverview, UpdateBadgeRequest,
};
pub use error::{AdminError, Result};
pub use models::{BatchTask, BatchTaskStatus, BatchTaskType, OperationLog};

// 从 badge-management-service 重新导出核心模型
// 便于 admin service 的其他模块直接使用
pub use badge_management::{
    Badge, BadgeAssets, BadgeCategory, BadgeRule, BadgeSeries, BadgeStatus, BadgeType,
    CategoryStatus, SourceType, ValidityConfig, ValidityType,
};
