//! B端服务 DTO 模块
//!
//! 包含所有请求和响应的数据传输对象

pub mod request;
pub mod response;

// 重新导出常用类型
pub use request::{
    AutoRevokeRequest, AutoRevokeScenario, BadgeQueryFilter, BatchGrantRequest, BatchRevokeRequest,
    BatchTaskFilter, CreateBadgeRequest, CreateCategoryRequest, CreateRuleRequest,
    CreateSeriesRequest, GrantLogFilter, ManualGrantRequest, ManualRevokeRequest,
    OperationLogFilter, PaginationParams, RecipientType, TestRuleDefinitionRequest, TimeRangeParams,
    UpdateBadgeRequest, UpdateCategoryRequest, UpdateRuleRequest, UpdateSeriesRequest,
};

pub use response::{
    ApiResponse, BadgeAdminDto, BadgeListItemDto, BadgeRankingDto, BadgeStatsDto, BatchTaskDto,
    CategoryDto, CreatedResponse, DeletedResponse, GrantLogDto, OperationLogDto, PageResponse,
    RuleDto, SeriesDto, StatsOverview, TrendDataPoint, UserBadgeAdminDto, UserBadgeViewDto,
    UserLedgerDto, UserRedemptionDto, UserStatsDto,
};
