//! 应用状态定义
//!
//! 包含 Axum 路由共享的应用状态

use badge_management::cascade::CascadeEvaluator;
use badge_management::repository::DependencyRepository;
use badge_shared::cache::Cache;
use sqlx::PgPool;
use std::sync::Arc;

use crate::error::AdminError;

/// Axum 应用共享状态
///
/// 包含数据库连接池、缓存客户端和可选的级联评估器，通过 Arc 在 handler 间共享
#[derive(Clone)]
pub struct AppState {
    /// PostgreSQL 连接池
    pub pool: PgPool,
    /// Redis 缓存客户端
    pub cache: Arc<Cache>,
    /// 依赖关系仓储（可选，用于依赖关系 CRUD）
    dependency_repo: Option<Arc<DependencyRepository>>,
    /// 级联评估器（可选，用于缓存刷新）
    pub cascade_evaluator: Option<Arc<CascadeEvaluator>>,
}

impl AppState {
    /// 创建新的应用状态（基础版本）
    pub fn new(pool: PgPool, cache: Arc<Cache>) -> Self {
        Self {
            pool,
            cache,
            dependency_repo: None,
            cascade_evaluator: None,
        }
    }

    /// 创建带有级联支持的应用状态
    ///
    /// 在需要依赖关系管理和级联触发功能时使用此构造函数
    pub fn with_cascade(
        pool: PgPool,
        cache: Arc<Cache>,
        dependency_repo: Arc<DependencyRepository>,
        cascade_evaluator: Arc<CascadeEvaluator>,
    ) -> Self {
        Self {
            pool,
            cache,
            dependency_repo: Some(dependency_repo),
            cascade_evaluator: Some(cascade_evaluator),
        }
    }

    /// 设置依赖关系仓储
    pub fn set_dependency_repo(&mut self, repo: Arc<DependencyRepository>) {
        self.dependency_repo = Some(repo);
    }

    /// 设置级联评估器
    pub fn set_cascade_evaluator(&mut self, evaluator: Arc<CascadeEvaluator>) {
        self.cascade_evaluator = Some(evaluator);
    }

    /// 获取依赖关系仓储的引用
    ///
    /// 如果未配置依赖关系仓储，则创建一个临时实例
    pub fn dependency_repo(&self) -> Result<Arc<DependencyRepository>, AdminError> {
        if let Some(ref repo) = self.dependency_repo {
            Ok(Arc::clone(repo))
        } else {
            // 如果未显式配置，使用连接池创建一个临时实例
            Ok(Arc::new(DependencyRepository::new(self.pool.clone())))
        }
    }
}
