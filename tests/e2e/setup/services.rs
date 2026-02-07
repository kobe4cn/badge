//! 服务健康检查和管理
//!
//! 确保所有依赖服务在测试开始前已就绪。

use anyhow::{Result, anyhow};
use std::time::Duration;
use tokio::time::{Instant, sleep};

use super::environment::TestEnvConfig;

/// 服务端口配置（与实际服务保持一致）
pub mod ports {
    #![allow(dead_code)] // 端口配置用于文档参考和将来扩展
    // HTTP/gRPC 服务端口
    pub const ADMIN_SERVICE: u16 = 8080;
    pub const RULE_ENGINE_GRPC: u16 = 50051;
    pub const BADGE_MANAGEMENT_GRPC: u16 = 50052;
    pub const MOCK_SERVICES: u16 = 8090;

    // Observability metrics_port（提供 /health 端点）
    pub const RULE_ENGINE_METRICS: u16 = 9990;
    pub const ADMIN_SERVICE_METRICS: u16 = 9991;
    pub const BADGE_MANAGEMENT_METRICS: u16 = 9992;
    pub const EVENT_ENGAGEMENT_METRICS: u16 = 9993;
    pub const EVENT_TRANSACTION_METRICS: u16 = 9994;
    pub const NOTIFICATION_WORKER_METRICS: u16 = 9995;
}

/// 服务健康状态
#[derive(Debug, Clone, PartialEq)]
pub enum ServiceHealth {
    /// 服务健康
    Healthy,
    /// 服务不健康
    Unhealthy(String),
    /// 服务不可达
    Unreachable,
}

/// 服务管理器
pub struct ServiceManager {
    config: TestEnvConfig,
    client: reqwest::Client,
}

impl ServiceManager {
    pub fn new(config: &TestEnvConfig) -> Self {
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(10))
            .connect_timeout(Duration::from_secs(5))
            .build()
            .expect("创建 HTTP 客户端失败");

        Self {
            config: config.clone(),
            client,
        }
    }

    /// 等待所有服务就绪
    pub async fn wait_all_ready(&self, timeout: Duration) -> Result<()> {
        let start = Instant::now();

        loop {
            let results = self.check_all_services().await;
            let all_healthy = results
                .iter()
                .all(|(_, health)| *health == ServiceHealth::Healthy);

            if all_healthy {
                tracing::info!("所有服务已就绪");
                return Ok(());
            }

            if start.elapsed() > timeout {
                let unhealthy: Vec<_> = results
                    .iter()
                    .filter(|(_, h)| *h != ServiceHealth::Healthy)
                    .map(|(name, health)| format!("{}: {:?}", name, health))
                    .collect();

                return Err(anyhow!(
                    "等待服务就绪超时，以下服务不健康: {}",
                    unhealthy.join(", ")
                ));
            }

            tracing::debug!("等待服务就绪...");
            sleep(Duration::from_secs(1)).await;
        }
    }

    /// 检查所有服务健康状态
    pub async fn check_all_services(&self) -> Vec<(&'static str, ServiceHealth)> {
        // 并行检查所有服务
        let (admin, rule_engine, badge_mgmt, event_engage, event_trans, notify, db, redis, kafka) = tokio::join!(
            self.check_admin_service(),
            self.check_rule_engine(),
            self.check_badge_management(),
            self.check_event_engagement(),
            self.check_event_transaction(),
            self.check_notification_worker(),
            self.check_database(),
            self.check_redis(),
            self.check_kafka(),
        );

        vec![
            ("admin-service", admin),
            ("rule-engine", rule_engine),
            ("badge-management", badge_mgmt),
            ("event-engagement", event_engage),
            ("event-transaction", event_trans),
            ("notification-worker", notify),
            ("database", db),
            ("redis", redis),
            ("kafka", kafka),
        ]
    }

    /// 仅检查核心服务（用于快速测试）
    pub async fn check_core_services(&self) -> Vec<(&'static str, ServiceHealth)> {
        let (admin, db, redis) = tokio::join!(
            self.check_admin_service(),
            self.check_database(),
            self.check_redis(),
        );

        vec![("admin-service", admin), ("database", db), ("redis", redis)]
    }

    /// 检查 Admin Service
    async fn check_admin_service(&self) -> ServiceHealth {
        let url = format!("{}/health", self.config.admin_service_url);
        self.check_http_health(&url).await
    }

    /// 检查规则引擎
    async fn check_rule_engine(&self) -> ServiceHealth {
        // gRPC 健康检查：通过 TCP 连接验证端口可达
        let addr = format!("127.0.0.1:{}", ports::RULE_ENGINE_GRPC);
        match tokio::net::TcpStream::connect(&addr).await {
            Ok(_) => ServiceHealth::Healthy,
            Err(_) => ServiceHealth::Unreachable,
        }
    }

    /// 检查徽章管理服务
    async fn check_badge_management(&self) -> ServiceHealth {
        match tokio::net::TcpStream::connect(format!("127.0.0.1:{}", ports::BADGE_MANAGEMENT_GRPC))
            .await
        {
            Ok(_) => ServiceHealth::Healthy,
            Err(_) => ServiceHealth::Unreachable,
        }
    }

    /// 检查行为事件服务（通过 observability metrics_port 的 /health 端点）
    async fn check_event_engagement(&self) -> ServiceHealth {
        let url = format!("http://127.0.0.1:{}/health", ports::EVENT_ENGAGEMENT_METRICS);
        self.check_http_health(&url).await
    }

    /// 检查交易事件服务（通过 observability metrics_port 的 /health 端点）
    async fn check_event_transaction(&self) -> ServiceHealth {
        let url = format!("http://127.0.0.1:{}/health", ports::EVENT_TRANSACTION_METRICS);
        self.check_http_health(&url).await
    }

    /// 检查通知服务（通过 observability metrics_port 的 /health 端点）
    async fn check_notification_worker(&self) -> ServiceHealth {
        let url = format!("http://127.0.0.1:{}/health", ports::NOTIFICATION_WORKER_METRICS);
        self.check_http_health(&url).await
    }

    /// 检查数据库
    async fn check_database(&self) -> ServiceHealth {
        match sqlx::PgPool::connect(&self.config.database_url).await {
            Ok(pool) => match sqlx::query("SELECT 1").execute(&pool).await {
                Ok(_) => ServiceHealth::Healthy,
                Err(e) => ServiceHealth::Unhealthy(e.to_string()),
            },
            Err(_) => ServiceHealth::Unreachable,
        }
    }

    /// 检查 Redis
    async fn check_redis(&self) -> ServiceHealth {
        match redis::Client::open(self.config.redis_url.as_str()) {
            Ok(client) => match client.get_multiplexed_async_connection().await {
                Ok(_) => ServiceHealth::Healthy,
                Err(e) => ServiceHealth::Unhealthy(e.to_string()),
            },
            Err(_) => ServiceHealth::Unreachable,
        }
    }

    /// 检查 Kafka
    async fn check_kafka(&self) -> ServiceHealth {
        // 简化实现：尝试 TCP 连接
        let broker = self
            .config
            .kafka_brokers
            .split(',')
            .next()
            .unwrap_or("localhost:9092");
        match tokio::net::TcpStream::connect(broker).await {
            Ok(_) => ServiceHealth::Healthy,
            Err(_) => ServiceHealth::Unreachable,
        }
    }

    /// HTTP 健康检查
    async fn check_http_health(&self, url: &str) -> ServiceHealth {
        match self.client.get(url).send().await {
            Ok(resp) if resp.status().is_success() => ServiceHealth::Healthy,
            Ok(resp) => ServiceHealth::Unhealthy(format!("状态码: {}", resp.status())),
            Err(e) => {
                tracing::debug!("健康检查失败 {}: {}", url, e);
                ServiceHealth::Unreachable
            }
        }
    }
}
