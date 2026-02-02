//! 测试环境管理
//!
//! 统一管理测试所需的所有外部依赖和服务连接。

use anyhow::Result;
use sqlx::PgPool;
use std::time::Duration;

use super::super::helpers::{ApiClient, DbVerifier, KafkaHelper};
use super::{ServiceManager, TestCleanup};

/// 测试环境配置
#[derive(Debug, Clone)]
pub struct TestEnvConfig {
    /// 数据库连接 URL
    pub database_url: String,
    /// Redis 连接 URL
    pub redis_url: String,
    /// Kafka Broker 地址
    pub kafka_brokers: String,
    /// Admin Service 地址
    pub admin_service_url: String,
    /// Rule Engine gRPC 地址
    pub rule_engine_addr: String,
    /// Badge Management gRPC 地址
    pub badge_management_addr: String,
    /// Event Engagement Service 地址
    pub event_engagement_url: String,
    /// Event Transaction Service 地址
    pub event_transaction_url: String,
    /// Notification Worker 地址
    pub notification_worker_url: String,
    /// Mock Services 地址
    pub mock_services_url: String,
    /// 等待服务就绪的超时时间
    pub service_ready_timeout: Duration,
    /// 是否跳过服务健康检查
    pub skip_health_check: bool,
    /// 是否只检查核心服务
    pub core_services_only: bool,
}

impl Default for TestEnvConfig {
    fn default() -> Self {
        Self {
            database_url: std::env::var("DATABASE_URL")
                .unwrap_or_else(|_| "postgres://badge:badge_secret@localhost:5432/badge_db".into()),
            redis_url: std::env::var("REDIS_URL")
                .unwrap_or_else(|_| "redis://localhost:6379".into()),
            kafka_brokers: std::env::var("KAFKA_BROKERS")
                .unwrap_or_else(|_| "localhost:9092".into()),
            // 使用 127.0.0.1 而非 localhost，避免 IPv6 连接问题
            admin_service_url: std::env::var("ADMIN_SERVICE_URL")
                .unwrap_or_else(|_| "http://127.0.0.1:8080".into()),
            rule_engine_addr: std::env::var("RULE_ENGINE_ADDR")
                .unwrap_or_else(|_| "http://127.0.0.1:50051".into()),
            badge_management_addr: std::env::var("BADGE_MANAGEMENT_ADDR")
                .unwrap_or_else(|_| "http://127.0.0.1:50052".into()),
            event_engagement_url: std::env::var("EVENT_ENGAGEMENT_URL")
                .unwrap_or_else(|_| "http://127.0.0.1:8081".into()),
            event_transaction_url: std::env::var("EVENT_TRANSACTION_URL")
                .unwrap_or_else(|_| "http://127.0.0.1:8082".into()),
            notification_worker_url: std::env::var("NOTIFICATION_WORKER_URL")
                .unwrap_or_else(|_| "http://127.0.0.1:8083".into()),
            mock_services_url: std::env::var("MOCK_SERVICES_URL")
                .unwrap_or_else(|_| "http://127.0.0.1:8090".into()),
            service_ready_timeout: Duration::from_secs(30),
            skip_health_check: std::env::var("SKIP_HEALTH_CHECK")
                .map(|v| v == "true" || v == "1")
                .unwrap_or(false),
            core_services_only: std::env::var("CORE_SERVICES_ONLY")
                .map(|v| v == "true" || v == "1")
                .unwrap_or(false),
        }
    }
}

impl TestEnvConfig {
    /// 创建仅用于核心服务测试的配置
    pub fn core_only() -> Self {
        Self {
            core_services_only: true,
            ..Default::default()
        }
    }

    /// 创建跳过健康检查的配置（用于调试）
    pub fn skip_checks() -> Self {
        Self {
            skip_health_check: true,
            ..Default::default()
        }
    }
}

/// 测试环境
///
/// 封装测试所需的所有客户端和工具，提供统一的接口。
pub struct TestEnvironment {
    /// 配置
    pub config: TestEnvConfig,
    /// 数据库连接池
    pub db_pool: PgPool,
    /// REST API 客户端
    pub api: ApiClient,
    /// Kafka 辅助工具
    pub kafka: KafkaHelper,
    /// 数据库验证工具
    pub db: DbVerifier,
    /// 服务管理器
    pub services: ServiceManager,
    /// 清理器
    cleanup: TestCleanup,
}

impl TestEnvironment {
    /// 创建并初始化测试环境
    pub async fn setup() -> Result<Self> {
        Self::setup_with_config(TestEnvConfig::default()).await
    }

    /// 使用自定义配置创建测试环境
    pub async fn setup_with_config(config: TestEnvConfig) -> Result<Self> {
        tracing::info!("初始化测试环境...");

        // 1. 连接数据库
        tracing::debug!("连接数据库: {}", config.database_url);
        let db_pool = PgPool::connect(&config.database_url).await?;

        // 2. 创建服务管理器并检查健康状态
        let services = ServiceManager::new(&config);

        if !config.skip_health_check {
            tracing::debug!("检查服务健康状态...");
            if config.core_services_only {
                // 仅检查核心服务
                let results = services.check_core_services().await;
                let unhealthy: Vec<_> = results
                    .iter()
                    .filter(|(_, h)| *h != super::ServiceHealth::Healthy)
                    .collect();
                if !unhealthy.is_empty() {
                    return Err(anyhow::anyhow!("核心服务不健康: {:?}", unhealthy));
                }
            } else {
                services
                    .wait_all_ready(config.service_ready_timeout)
                    .await?;
            }
        } else {
            tracing::warn!("跳过服务健康检查");
        }

        // 3. 创建辅助工具
        tracing::debug!("创建辅助工具...");
        let api = ApiClient::new(&config.admin_service_url);
        let kafka = KafkaHelper::new(&config.kafka_brokers).await?;
        let db = DbVerifier::new(db_pool.clone());
        let cleanup = TestCleanup::new(db_pool.clone());

        tracing::info!("测试环境初始化完成");

        Ok(Self {
            config,
            db_pool,
            api,
            kafka,
            db,
            services,
            cleanup,
        })
    }

    /// 仅初始化核心服务（不启动事件服务）
    pub async fn setup_core_only() -> Result<Self> {
        Self::setup_with_config(TestEnvConfig::core_only()).await
    }

    /// 为性能测试创建环境（跳过某些初始化）
    pub async fn setup_for_perf() -> Result<Self> {
        let mut config = TestEnvConfig::default();
        config.service_ready_timeout = Duration::from_secs(60);
        Self::setup_with_config(config).await
    }

    /// 等待规则热加载完成
    pub async fn wait_for_rule_reload(&self) -> Result<()> {
        // 发送规则刷新消息后等待一段时间
        // 增加到 2 秒以确保规则在 Kafka 消费者和 gRPC 服务之间完成同步
        tokio::time::sleep(Duration::from_secs(2)).await;
        Ok(())
    }

    /// 等待事件处理完成
    pub async fn wait_for_processing(&self, timeout: Duration) -> Result<()> {
        tokio::time::sleep(timeout).await;
        Ok(())
    }

    /// 等待用户获得指定徽章
    ///
    /// 轮询检查用户是否获得徽章，超时返回错误。
    pub async fn wait_for_badge(
        &self,
        user_id: &str,
        badge_id: i64,
        timeout: Duration,
    ) -> Result<()> {
        let start = tokio::time::Instant::now();
        let poll_interval = Duration::from_millis(200);

        while start.elapsed() < timeout {
            if self.db.user_has_badge(user_id, badge_id).await? {
                return Ok(());
            }
            tokio::time::sleep(poll_interval).await;
        }

        Err(anyhow::anyhow!(
            "等待用户 {} 获得徽章 {} 超时",
            user_id,
            badge_id
        ))
    }

    /// 等待权益发放完成
    pub async fn wait_for_benefit(
        &self,
        user_id: &str,
        benefit_id: i64,
        timeout: Duration,
    ) -> Result<()> {
        let start = tokio::time::Instant::now();
        let poll_interval = Duration::from_millis(200);

        while start.elapsed() < timeout {
            if self.db.benefit_granted(user_id, benefit_id).await? {
                return Ok(());
            }
            tokio::time::sleep(poll_interval).await;
        }

        Err(anyhow::anyhow!(
            "等待用户 {} 获得权益 {} 超时",
            user_id,
            benefit_id
        ))
    }

    /// 等待条件满足
    ///
    /// 通用的条件等待方法，适用于各种异步验证场景。
    pub async fn wait_until<F, Fut>(
        &self,
        condition: F,
        timeout: Duration,
        error_msg: &str,
    ) -> Result<()>
    where
        F: Fn() -> Fut,
        Fut: std::future::Future<Output = Result<bool>>,
    {
        let start = tokio::time::Instant::now();
        let poll_interval = Duration::from_millis(200);

        while start.elapsed() < timeout {
            if condition().await? {
                return Ok(());
            }
            tokio::time::sleep(poll_interval).await;
        }

        Err(anyhow::anyhow!("{}", error_msg))
    }

    /// 执行测试前的数据准备
    ///
    /// 1. 清理测试数据
    /// 2. 刷新 badge-management-service 的依赖图缓存（清除旧依赖数据的缓存）
    pub async fn prepare_test_data(&self) -> Result<()> {
        self.cleanup.clean_all().await?;
        // 刷新依赖图缓存，确保 badge-management-service 不会使用旧的依赖数据
        if let Err(e) = self.api.refresh_dependency_cache().await {
            tracing::warn!("刷新依赖缓存失败（可能不影响测试）: {}", e);
        }
        Ok(())
    }

    /// 清理测试数据
    pub async fn cleanup(&self) -> Result<()> {
        self.cleanup.clean_all().await
    }
}

impl Drop for TestEnvironment {
    fn drop(&mut self) {
        // 异步清理在这里无法执行，依赖显式调用 cleanup()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    #[ignore = "需要运行的服务"]
    async fn test_environment_setup() {
        let env = TestEnvironment::setup().await;
        assert!(env.is_ok(), "环境初始化应该成功");
    }
}
