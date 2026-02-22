//! 命令执行器
//!
//! 负责执行各 CLI 子命令的具体逻辑。
//! 将命令行参数转化为实际的服务调用和数据操作。

use std::fs;
use std::io::Write as _;
use std::net::SocketAddr;
use std::sync::Arc;

use anyhow::{Context, Result, bail};
use axum::{Json, Router, routing::get};
use serde::Serialize;
use tokio::net::TcpListener;
use tracing::{error, info, warn};

use badge_shared::config::KafkaConfig;
use badge_shared::events::EventPayload;
use badge_shared::kafka::KafkaProducer;

use crate::event_generator::{
    BatchEventSender, quick_checkin_event, quick_purchase_event, quick_refund_event,
};
use crate::generators::{DataGenerator, GenerationStats, GeneratorConfig};
use crate::models::{MockCoupon, MockOrder, MockUser};
use crate::scenarios::{PredefinedScenarios, Scenario, ScenarioRunner};
use crate::services::{
    CouponServiceState, OrderServiceState, ProfileServiceState, coupon_routes, order_routes,
    profile_routes,
};
use crate::store::MemoryStore;

/// 命令执行器
///
/// 封装 Kafka 配置和各命令的执行逻辑。
/// 作为 CLI 与业务逻辑之间的桥梁，简化 main 函数的复杂度。
pub struct CommandRunner {
    kafka_brokers: String,
}

impl CommandRunner {
    /// 创建命令执行器
    pub fn new(kafka_brokers: String) -> Self {
        Self { kafka_brokers }
    }

    /// 执行 server 命令
    ///
    /// 启动 HTTP REST API 服务器，合并订单、用户、优惠券路由。
    /// 支持可选的数据预填充，便于快速开始测试。
    pub async fn run_server(&self, port: u16, populate: bool, user_count: usize) -> Result<()> {
        info!(port, populate, user_count, "启动 Mock 服务");

        // 创建各服务的共享状态
        let order_state = Arc::new(OrderServiceState::new());
        let profile_state = Arc::new(ProfileServiceState::new());
        let coupon_state = Arc::new(CouponServiceState::default());

        // 预填充测试数据
        if populate {
            info!(user_count, "预填充测试数据");
            let config = GeneratorConfig {
                user_count,
                orders_per_user: 1..10,
                coupons_per_user: 0..5,
            };
            let generator = DataGenerator::new(config);

            // 生成并填充数据
            generator.populate_stores(
                &profile_state.users,
                &order_state.orders,
                &coupon_state.coupons,
            );

            let stats = GenerationStats::from_stores(
                &profile_state.users,
                &order_state.orders,
                &coupon_state.coupons,
            );
            info!(
                users = stats.users_count,
                orders = stats.orders_count,
                coupons = stats.coupons_count,
                "数据预填充完成"
            );
        }

        // 合并所有服务路由到一个应用
        // 健康检查端点独立于业务服务，便于运维监控
        let app = Router::new()
            .route("/health", get(health_check))
            .route("/ready", get(readiness_check))
            .merge(order_routes().with_state(order_state))
            .merge(profile_routes().with_state(profile_state))
            .merge(coupon_routes().with_state(coupon_state));

        let addr = SocketAddr::from(([0, 0, 0, 0], port));
        let listener = TcpListener::bind(addr).await.context("绑定端口失败")?;

        info!("Mock 服务已启动: http://{}", addr);
        info!("可用端点:");
        info!("  GET /health - 健康检查");
        info!("  GET /ready - 就绪检查");
        info!("  GET/POST /orders - 订单管理");
        info!("  GET/POST /users - 用户管理");
        info!("  GET/POST /coupons - 优惠券管理");
        info!("按 Ctrl+C 停止服务");

        // 启动服务器并等待关闭信号
        axum::serve(listener, app)
            .with_graceful_shutdown(shutdown_signal())
            .await
            .context("服务器运行失败")?;

        info!("Mock 服务已停止");
        Ok(())
    }

    /// 执行 generate 命令
    ///
    /// 根据事件类型生成并发送事件到 Kafka。
    /// 支持批量生成同类型事件用于压力测试。
    pub async fn run_generate(
        &self,
        event_type: &str,
        user_id: &str,
        count: usize,
        amount: Option<f64>,
    ) -> Result<()> {
        info!(
            event_type,
            user_id,
            count,
            amount = ?amount,
            "生成事件"
        );

        // 创建 Kafka 生产者
        let kafka_config = KafkaConfig {
            brokers: self.kafka_brokers.clone(),
            consumer_group: "mock-services".to_string(),
            ..Default::default()
        };
        let producer = KafkaProducer::new(&kafka_config).context("创建 Kafka 生产者失败")?;
        let sender = BatchEventSender::new(producer);

        // 根据事件类型生成事件
        let events = self.generate_events(event_type, user_id, count, amount)?;

        // 发送事件
        let result = sender.send_batch(&events).await;

        info!(
            total = result.total,
            success = result.success,
            failed = result.failed,
            success_rate = format!("{:.1}%", result.success_rate()),
            "事件发送完成"
        );

        if !result.is_all_success() {
            for err in &result.errors {
                error!("{}", err);
            }
            bail!("部分事件发送失败: {}/{} 失败", result.failed, result.total);
        }

        Ok(())
    }

    /// 执行 scenario 命令
    ///
    /// 执行预定义或自定义场景，模拟用户行为流程。
    /// 支持从文件加载场景和覆盖用户 ID。
    pub async fn run_scenario(
        &self,
        name: &str,
        user_id: Option<String>,
        file: Option<String>,
    ) -> Result<()> {
        // 列出所有预定义场景
        if name == "list" {
            println!("\n可用的预定义场景:");
            println!("{}", "-".repeat(60));
            for scenario in PredefinedScenarios::all() {
                println!("  {} - {}", scenario.name, scenario.description);
            }
            println!("{}", "-".repeat(60));
            println!("\n使用示例: mock-server scenario -n first_purchase");
            return Ok(());
        }

        // 加载场景：优先从文件加载，否则使用预定义场景
        let mut scenario = if let Some(ref file_path) = file {
            self.load_scenario_from_file(file_path)?
        } else {
            PredefinedScenarios::get(name).ok_or_else(|| {
                anyhow::anyhow!(
                    "未找到场景 '{}'\n使用 'scenario -n list' 查看所有可用场景",
                    name
                )
            })?
        };

        // 覆盖用户 ID（如果提供）
        if let Some(ref uid) = user_id {
            info!(user_id = %uid, "覆盖场景中的用户 ID");
            scenario = self.override_scenario_user_id(scenario, uid);
        }

        info!(
            scenario = %scenario.name,
            steps = scenario.steps.len(),
            "执行场景"
        );

        // 创建 Kafka 发送器并执行场景
        let kafka_config = KafkaConfig {
            brokers: self.kafka_brokers.clone(),
            consumer_group: "mock-services".to_string(),
            ..Default::default()
        };
        let producer = KafkaProducer::new(&kafka_config).context("创建 Kafka 生产者失败")?;
        let sender = BatchEventSender::new(producer);
        let runner = ScenarioRunner::new(sender);

        let result = runner.run(&scenario).await;

        // 打印结果
        println!("\n场景执行结果:");
        println!("{}", "-".repeat(40));
        println!("场景名称: {}", result.scenario_name);
        println!("总步骤数: {}", result.total_steps);
        println!("成功步骤: {}", result.success_steps);
        println!("失败步骤: {}", result.failed_steps);
        println!("发送事件: {}", result.total_events_sent);
        println!("执行耗时: {} ms", result.duration_ms);
        println!("成功率: {:.1}%", result.success_rate());
        println!("{}", "-".repeat(40));

        if !result.is_all_success() {
            warn!("场景执行存在失败步骤");
            for step in result.step_results.iter().filter(|s| !s.success) {
                if let Some(ref err) = step.error {
                    error!(
                        "步骤 {} ({}) 失败: {}",
                        step.step_index, step.step_type, err
                    );
                }
            }
        }

        Ok(())
    }

    /// 执行 populate 命令
    ///
    /// 批量生成测试数据，可输出到文件供其他工具使用。
    pub async fn run_populate(
        &self,
        users: usize,
        orders: &str,
        output: Option<String>,
    ) -> Result<()> {
        // 解析订单数量范围
        let (min_orders, max_orders) = self.parse_range(orders)?;

        info!(users, min_orders, max_orders, "批量生成测试数据");

        let config = GeneratorConfig {
            user_count: users,
            orders_per_user: min_orders..max_orders,
            coupons_per_user: 0..5,
        };
        let generator = DataGenerator::new(config);

        // 创建存储并填充数据
        let user_store: MemoryStore<MockUser> = MemoryStore::new();
        let order_store: MemoryStore<MockOrder> = MemoryStore::new();
        let coupon_store: MemoryStore<MockCoupon> = MemoryStore::new();

        generator.populate_stores(&user_store, &order_store, &coupon_store);

        let stats = GenerationStats::from_stores(&user_store, &order_store, &coupon_store);

        // 输出到文件（如果指定）
        if let Some(ref path) = output {
            let data = PopulateOutput {
                users: user_store.list(),
                orders: order_store.list(),
                coupons: coupon_store.list(),
            };

            let json = serde_json::to_string_pretty(&data).context("序列化数据失败")?;

            let mut file = fs::File::create(path).context("创建输出文件失败")?;
            file.write_all(json.as_bytes()).context("写入文件失败")?;

            info!(path, "数据已输出到文件");
        }

        // 打印统计
        println!("\n数据生成完成:");
        println!("{}", "-".repeat(30));
        println!("用户数量: {}", stats.users_count);
        println!("订单数量: {}", stats.orders_count);
        println!("优惠券数量: {}", stats.coupons_count);
        println!("{}", "-".repeat(30));

        Ok(())
    }

    // ========================================================================
    // 辅助方法
    // ========================================================================

    /// 根据事件类型生成事件列表
    fn generate_events(
        &self,
        event_type: &str,
        user_id: &str,
        count: usize,
        amount: Option<f64>,
    ) -> Result<Vec<EventPayload>> {
        let mut events = Vec::with_capacity(count);

        for _ in 0..count {
            let event = match event_type.to_lowercase().as_str() {
                "purchase" => {
                    let amt = amount.unwrap_or(99.99);
                    quick_purchase_event(user_id, amt)
                }
                "checkin" | "check_in" => quick_checkin_event(user_id, 1),
                "refund" => {
                    let amt = amount.unwrap_or(50.0);
                    quick_refund_event(user_id, "ORD-MOCK-001", amt, &[])
                }
                "pageview" | "page_view" => create_pageview_event(user_id),
                "share" => create_share_event(user_id),
                _ => {
                    bail!(
                        "不支持的事件类型: {}\n支持的类型: purchase, checkin, refund, pageview, share",
                        event_type
                    );
                }
            };
            events.push(event);
        }

        Ok(events)
    }

    /// 从文件加载场景
    fn load_scenario_from_file(&self, path: &str) -> Result<Scenario> {
        let content =
            fs::read_to_string(path).with_context(|| format!("读取场景文件失败: {}", path))?;

        // 根据文件扩展名选择解析方式
        let scenario = if path.ends_with(".yaml") || path.ends_with(".yml") {
            Scenario::from_yaml(&content)
                .with_context(|| format!("解析 YAML 场景失败: {}", path))?
        } else {
            Scenario::from_json(&content)
                .with_context(|| format!("解析 JSON 场景失败: {}", path))?
        };

        info!(scenario = %scenario.name, path, "从文件加载场景");
        Ok(scenario)
    }

    /// 覆盖场景中所有步骤的用户 ID
    fn override_scenario_user_id(&self, mut scenario: Scenario, new_user_id: &str) -> Scenario {
        use crate::scenarios::ScenarioStep;

        for step in &mut scenario.steps {
            match step {
                ScenarioStep::Purchase { user_id, .. }
                | ScenarioStep::CheckIn { user_id, .. }
                | ScenarioStep::Refund { user_id, .. }
                | ScenarioStep::PageView { user_id, .. }
                | ScenarioStep::Share { user_id, .. } => {
                    *user_id = new_user_id.to_string();
                }
                ScenarioStep::Wait { .. } | ScenarioStep::Repeat { .. } => {}
            }
        }
        scenario
    }

    /// 解析范围字符串 (格式: "min-max")
    fn parse_range(&self, range_str: &str) -> Result<(usize, usize)> {
        let parts: Vec<&str> = range_str.split('-').collect();
        if parts.len() != 2 {
            bail!("无效的范围格式: {}，预期格式: min-max", range_str);
        }

        let min: usize = parts[0]
            .parse()
            .with_context(|| format!("无效的最小值: {}", parts[0]))?;
        let max: usize = parts[1]
            .parse()
            .with_context(|| format!("无效的最大值: {}", parts[1]))?;

        if min >= max {
            bail!("无效的范围: min ({}) 必须小于 max ({})", min, max);
        }

        Ok((min, max))
    }
}

// ============================================================================
// 辅助函数
// ============================================================================

/// 等待关闭信号
async fn shutdown_signal() {
    tokio::signal::ctrl_c()
        .await
        .expect("安装 CTRL+C 信号处理器失败");
    info!("收到关闭信号，正在停止服务...");
}

/// 创建页面浏览事件
fn create_pageview_event(user_id: &str) -> EventPayload {
    use badge_shared::events::EventType;
    use serde_json::json;

    EventPayload::new(
        EventType::PageView,
        user_id,
        json!({
            "page_url": "https://shop.example.com/product/123",
            "duration_seconds": 30,
            "referrer": "direct"
        }),
        "mock-services",
    )
}

/// 创建分享事件
fn create_share_event(user_id: &str) -> EventPayload {
    use badge_shared::events::EventType;
    use serde_json::json;
    use uuid::Uuid;

    EventPayload::new(
        EventType::Share,
        user_id,
        json!({
            "platform": "wechat",
            "content_type": "product",
            "content_id": format!("CNT-{}", Uuid::new_v4())
        }),
        "mock-services",
    )
}

// ============================================================================
// 健康检查端点
// ============================================================================

/// 健康检查响应
#[derive(Serialize)]
struct HealthResponse {
    status: &'static str,
}

/// 就绪检查响应
#[derive(Serialize)]
struct ReadinessResponse {
    status: &'static str,
    services: Vec<&'static str>,
}

/// 健康检查端点
///
/// 返回服务的基本存活状态，用于 K8s liveness probe
async fn health_check() -> Json<HealthResponse> {
    Json(HealthResponse { status: "healthy" })
}

/// 就绪检查端点
///
/// 返回服务的就绪状态及可用的模拟服务列表，用于 K8s readiness probe
async fn readiness_check() -> Json<ReadinessResponse> {
    Json(ReadinessResponse {
        status: "ready",
        services: vec!["order", "profile", "coupon"],
    })
}

/// 数据填充输出结构
#[derive(Serialize)]
struct PopulateOutput {
    users: Vec<MockUser>,
    orders: Vec<MockOrder>,
    coupons: Vec<MockCoupon>,
}

// ============================================================================
// 单元测试
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_range_valid() {
        let runner = CommandRunner::new("localhost:9092".to_string());

        let (min, max) = runner.parse_range("1-10").unwrap();
        assert_eq!(min, 1);
        assert_eq!(max, 10);

        let (min, max) = runner.parse_range("5-20").unwrap();
        assert_eq!(min, 5);
        assert_eq!(max, 20);
    }

    #[test]
    fn test_parse_range_invalid() {
        let runner = CommandRunner::new("localhost:9092".to_string());

        // 格式错误
        assert!(runner.parse_range("invalid").is_err());
        assert!(runner.parse_range("1-2-3").is_err());

        // 范围错误
        assert!(runner.parse_range("10-5").is_err());
        assert!(runner.parse_range("5-5").is_err());
    }

    #[test]
    fn test_generate_events_purchase() {
        let runner = CommandRunner::new("localhost:9092".to_string());

        let events = runner
            .generate_events("purchase", "user-001", 3, Some(199.99))
            .unwrap();

        assert_eq!(events.len(), 3);
        for event in &events {
            assert_eq!(event.user_id, "user-001");
            assert_eq!(event.data["amount"].as_f64().unwrap(), 199.99);
        }
    }

    #[test]
    fn test_generate_events_checkin() {
        let runner = CommandRunner::new("localhost:9092".to_string());

        let events = runner
            .generate_events("checkin", "user-002", 2, None)
            .unwrap();

        assert_eq!(events.len(), 2);
        for event in &events {
            assert_eq!(event.user_id, "user-002");
        }
    }

    #[test]
    fn test_generate_events_invalid_type() {
        let runner = CommandRunner::new("localhost:9092".to_string());

        let result = runner.generate_events("invalid_type", "user-001", 1, None);
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_health_check() {
        let response = health_check().await;
        assert_eq!(response.status, "healthy");
    }

    #[tokio::test]
    async fn test_readiness_check() {
        let response = readiness_check().await;
        assert_eq!(response.status, "ready");
        assert_eq!(response.services.len(), 3);
        assert!(response.services.contains(&"order"));
        assert!(response.services.contains(&"profile"));
        assert!(response.services.contains(&"coupon"));
    }
}
