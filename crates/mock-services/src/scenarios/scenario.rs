//! 场景定义和执行器
//!
//! 提供场景（Scenario）的数据结构定义和执行能力。
//! 场景由多个步骤（Step）组成，支持嵌套循环和延时控制。

use std::time::Instant;

use serde::{Deserialize, Serialize};
use tokio::time::{Duration, sleep};
use tracing::{debug, error, info, instrument, warn};
use uuid::Uuid;

use crate::event_generator::{
    BatchEventSender, quick_checkin_event, quick_purchase_event, quick_refund_event,
};
use badge_shared::events::{EventPayload, EventType};

// ---------------------------------------------------------------------------
// 场景定义
// ---------------------------------------------------------------------------

/// 场景定义
///
/// 场景是一组按顺序执行的步骤，用于模拟真实用户行为流程。
/// 支持序列化为 JSON/YAML，便于从配置文件加载自定义场景。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Scenario {
    /// 场景名称，用于日志和结果报告中标识
    pub name: String,
    /// 场景描述，说明该场景模拟的业务流程
    pub description: String,
    /// 场景步骤列表，按顺序执行
    pub steps: Vec<ScenarioStep>,
}

impl Scenario {
    /// 从 JSON 字符串解析场景
    pub fn from_json(json: &str) -> Result<Self, serde_json::Error> {
        serde_json::from_str(json)
    }

    /// 从 YAML 字符串解析场景
    ///
    /// YAML 格式更适合人工编写场景配置，可读性更好。
    pub fn from_yaml(yaml: &str) -> Result<Self, serde_yaml::Error> {
        serde_yaml::from_str(yaml)
    }

    /// 将场景序列化为 JSON 字符串
    pub fn to_json(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string_pretty(self)
    }

    /// 将场景序列化为 YAML 字符串
    pub fn to_yaml(&self) -> Result<String, serde_yaml::Error> {
        serde_yaml::to_string(self)
    }

    /// 创建新场景的构建器
    pub fn builder(name: impl Into<String>) -> ScenarioBuilder {
        ScenarioBuilder::new(name)
    }
}

// ---------------------------------------------------------------------------
// 场景步骤
// ---------------------------------------------------------------------------

/// 场景步骤
///
/// 使用 serde 的 tagged enum 序列化，使 JSON/YAML 格式更直观。
/// `type` 字段指定步骤类型，其他字段为该类型的参数。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ScenarioStep {
    /// 生成购买事件
    Purchase {
        user_id: String,
        amount: f64,
        #[serde(skip_serializing_if = "Option::is_none")]
        delay_ms: Option<u64>,
    },
    /// 生成签到事件
    CheckIn {
        user_id: String,
        consecutive_days: i32,
        #[serde(skip_serializing_if = "Option::is_none")]
        delay_ms: Option<u64>,
    },
    /// 生成退款事件
    Refund {
        user_id: String,
        order_id: String,
        amount: f64,
        badge_ids: Vec<i64>,
        #[serde(skip_serializing_if = "Option::is_none")]
        delay_ms: Option<u64>,
    },
    /// 生成页面浏览事件
    PageView {
        user_id: String,
        page_url: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        delay_ms: Option<u64>,
    },
    /// 生成分享事件
    Share {
        user_id: String,
        platform: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        delay_ms: Option<u64>,
    },
    /// 等待一段时间
    Wait { duration_ms: u64 },
    /// 重复执行步骤
    ///
    /// 支持嵌套结构，可以用于模拟连续多天签到等场景。
    Repeat {
        count: usize,
        steps: Vec<ScenarioStep>,
    },
}

impl ScenarioStep {
    /// 获取步骤类型名称
    pub fn type_name(&self) -> &'static str {
        match self {
            Self::Purchase { .. } => "purchase",
            Self::CheckIn { .. } => "check_in",
            Self::Refund { .. } => "refund",
            Self::PageView { .. } => "page_view",
            Self::Share { .. } => "share",
            Self::Wait { .. } => "wait",
            Self::Repeat { .. } => "repeat",
        }
    }
}

// ---------------------------------------------------------------------------
// 场景构建器
// ---------------------------------------------------------------------------

/// 场景构建器
///
/// 提供流式 API 来构建场景，比直接构造 Scenario 更直观。
pub struct ScenarioBuilder {
    name: String,
    description: String,
    steps: Vec<ScenarioStep>,
}

impl ScenarioBuilder {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            description: String::new(),
            steps: Vec::new(),
        }
    }

    pub fn description(mut self, desc: impl Into<String>) -> Self {
        self.description = desc.into();
        self
    }

    pub fn step(mut self, step: ScenarioStep) -> Self {
        self.steps.push(step);
        self
    }

    pub fn purchase(self, user_id: impl Into<String>, amount: f64) -> Self {
        self.step(ScenarioStep::Purchase {
            user_id: user_id.into(),
            amount,
            delay_ms: None,
        })
    }

    pub fn checkin(self, user_id: impl Into<String>, consecutive_days: i32) -> Self {
        self.step(ScenarioStep::CheckIn {
            user_id: user_id.into(),
            consecutive_days,
            delay_ms: None,
        })
    }

    pub fn wait(self, duration_ms: u64) -> Self {
        self.step(ScenarioStep::Wait { duration_ms })
    }

    pub fn build(self) -> Scenario {
        Scenario {
            name: self.name,
            description: self.description,
            steps: self.steps,
        }
    }
}

// ---------------------------------------------------------------------------
// 场景执行结果
// ---------------------------------------------------------------------------

/// 场景执行结果
///
/// 完整记录场景执行的统计信息和各步骤的详细结果。
#[derive(Debug, Clone, Serialize)]
pub struct ScenarioResult {
    /// 场景名称
    pub scenario_name: String,
    /// 总步骤数（展开 Repeat 后）
    pub total_steps: usize,
    /// 成功步骤数
    pub success_steps: usize,
    /// 失败步骤数
    pub failed_steps: usize,
    /// 总发送事件数
    pub total_events_sent: usize,
    /// 执行总耗时（毫秒）
    pub duration_ms: u64,
    /// 各步骤执行结果
    pub step_results: Vec<StepResult>,
}

impl ScenarioResult {
    /// 是否全部成功
    pub fn is_all_success(&self) -> bool {
        self.failed_steps == 0
    }

    /// 成功率（百分比）
    pub fn success_rate(&self) -> f64 {
        if self.total_steps == 0 {
            100.0
        } else {
            (self.success_steps as f64 / self.total_steps as f64) * 100.0
        }
    }
}

/// 单个步骤执行结果
#[derive(Debug, Clone, Serialize)]
pub struct StepResult {
    /// 步骤索引（从 0 开始）
    pub step_index: usize,
    /// 步骤类型名称
    pub step_type: String,
    /// 是否执行成功
    pub success: bool,
    /// 该步骤发送的事件数
    pub events_sent: usize,
    /// 错误信息（如果失败）
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

// ---------------------------------------------------------------------------
// 场景执行器
// ---------------------------------------------------------------------------

/// 场景执行器
///
/// 负责解析和执行场景，将步骤转换为实际的事件并发送。
/// 通过 BatchEventSender 发送事件，保持与现有 Kafka 基础设施的集成。
pub struct ScenarioRunner {
    sender: BatchEventSender,
}

impl ScenarioRunner {
    /// 创建场景执行器
    pub fn new(sender: BatchEventSender) -> Self {
        Self { sender }
    }

    /// 执行场景
    ///
    /// 按顺序执行场景中的所有步骤，收集执行结果。
    /// 单个步骤失败不会中断整个场景的执行。
    #[instrument(skip(self), fields(scenario = %scenario.name))]
    pub async fn run(&self, scenario: &Scenario) -> ScenarioResult {
        let start = Instant::now();
        info!(scenario = %scenario.name, steps = scenario.steps.len(), "开始执行场景");

        let step_results = self.execute_all_steps(&scenario.steps).await;

        let duration_ms = start.elapsed().as_millis() as u64;

        let success_steps = step_results.iter().filter(|r| r.success).count();
        let failed_steps = step_results.iter().filter(|r| !r.success).count();
        let total_events_sent: usize = step_results.iter().map(|r| r.events_sent).sum();

        info!(
            scenario = %scenario.name,
            total_steps = step_results.len(),
            success_steps,
            failed_steps,
            total_events_sent,
            duration_ms,
            "场景执行完成"
        );

        ScenarioResult {
            scenario_name: scenario.name.clone(),
            total_steps: step_results.len(),
            success_steps,
            failed_steps,
            total_events_sent,
            duration_ms,
            step_results,
        }
    }

    /// 展开并执行所有步骤（将 Repeat 展开为扁平列表）
    ///
    /// 使用栈来迭代处理嵌套的 Repeat 步骤，避免异步递归导致的编译问题。
    async fn execute_all_steps(&self, steps: &[ScenarioStep]) -> Vec<StepResult> {
        let mut results = Vec::new();

        // 将步骤展开为扁平列表，避免异步递归
        let flattened = Self::flatten_steps(steps);

        for (step_index, step) in flattened.into_iter().enumerate() {
            let result = self.execute_single_step(&step, step_index).await;
            results.push(result);
        }

        results
    }

    /// 将嵌套的 Repeat 步骤展开为扁平列表
    ///
    /// 使用栈来处理嵌套结构，保持正确的执行顺序。
    fn flatten_steps(steps: &[ScenarioStep]) -> Vec<ScenarioStep> {
        let mut result = Vec::new();
        let mut stack: Vec<&ScenarioStep> = steps.iter().rev().collect();

        while let Some(step) = stack.pop() {
            match step {
                ScenarioStep::Repeat { count, steps } => {
                    debug!(count, nested_steps = steps.len(), "展开重复步骤");
                    // 将 Repeat 内的步骤展开 count 次，逆序压栈以保持正确顺序
                    for _ in 0..*count {
                        for s in steps.iter().rev() {
                            stack.push(s);
                        }
                    }
                }
                other => {
                    result.push(other.clone());
                }
            }
        }

        result
    }

    /// 执行单个步骤
    async fn execute_single_step(&self, step: &ScenarioStep, step_index: usize) -> StepResult {
        debug!(step_index, step_type = step.type_name(), "执行步骤");

        match step {
            ScenarioStep::Purchase {
                user_id,
                amount,
                delay_ms,
            } => {
                self.apply_delay(*delay_ms).await;
                let event = quick_purchase_event(user_id, *amount);
                self.send_event_and_result(step_index, "purchase", event)
                    .await
            }
            ScenarioStep::CheckIn {
                user_id,
                consecutive_days,
                delay_ms,
            } => {
                self.apply_delay(*delay_ms).await;
                let event = quick_checkin_event(user_id, *consecutive_days);
                self.send_event_and_result(step_index, "check_in", event)
                    .await
            }
            ScenarioStep::Refund {
                user_id,
                order_id,
                amount,
                badge_ids,
                delay_ms,
            } => {
                self.apply_delay(*delay_ms).await;
                let event = quick_refund_event(user_id, order_id, *amount, badge_ids);
                self.send_event_and_result(step_index, "refund", event)
                    .await
            }
            ScenarioStep::PageView {
                user_id,
                page_url,
                delay_ms,
            } => {
                self.apply_delay(*delay_ms).await;
                let event = create_page_view_event(user_id, page_url);
                self.send_event_and_result(step_index, "page_view", event)
                    .await
            }
            ScenarioStep::Share {
                user_id,
                platform,
                delay_ms,
            } => {
                self.apply_delay(*delay_ms).await;
                let event = create_share_event(user_id, platform);
                self.send_event_and_result(step_index, "share", event).await
            }
            ScenarioStep::Wait { duration_ms } => {
                debug!(duration_ms, "等待");
                sleep(Duration::from_millis(*duration_ms)).await;
                StepResult {
                    step_index,
                    step_type: "wait".to_string(),
                    success: true,
                    events_sent: 0,
                    error: None,
                }
            }
            ScenarioStep::Repeat { .. } => {
                // Repeat 在 execute_step_recursive 中处理，此处不会执行
                warn!("Repeat 步骤不应在此执行");
                StepResult {
                    step_index,
                    step_type: "repeat".to_string(),
                    success: false,
                    events_sent: 0,
                    error: Some("Repeat 步骤处理错误".to_string()),
                }
            }
        }
    }

    /// 应用延时
    async fn apply_delay(&self, delay_ms: Option<u64>) {
        if let Some(ms) = delay_ms
            && ms > 0
        {
            debug!(delay_ms = ms, "应用步骤延时");
            sleep(Duration::from_millis(ms)).await;
        }
    }

    /// 发送事件并生成结果
    async fn send_event_and_result(
        &self,
        step_index: usize,
        step_type: &str,
        event: EventPayload,
    ) -> StepResult {
        match self.sender.send(&event).await {
            Ok(()) => StepResult {
                step_index,
                step_type: step_type.to_string(),
                success: true,
                events_sent: 1,
                error: None,
            },
            Err(e) => {
                error!(step_index, step_type, error = %e, "事件发送失败");
                StepResult {
                    step_index,
                    step_type: step_type.to_string(),
                    success: false,
                    events_sent: 0,
                    error: Some(e.to_string()),
                }
            }
        }
    }
}

// ---------------------------------------------------------------------------
// 辅助函数：创建事件
// ---------------------------------------------------------------------------

/// 创建页面浏览事件
fn create_page_view_event(user_id: &str, page_url: &str) -> EventPayload {
    EventPayload::new(
        EventType::PageView,
        user_id,
        serde_json::json!({
            "page_url": page_url,
            "duration_seconds": 30,
            "referrer": "direct"
        }),
        "mock-services",
    )
}

/// 创建分享事件
fn create_share_event(user_id: &str, platform: &str) -> EventPayload {
    EventPayload::new(
        EventType::Share,
        user_id,
        serde_json::json!({
            "platform": platform,
            "content_type": "product",
            "content_id": format!("CNT-{}", Uuid::new_v4())
        }),
        "mock-services",
    )
}

// ---------------------------------------------------------------------------
// 单元测试
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_scenario_serialization() {
        let scenario = Scenario {
            name: "test-scenario".to_string(),
            description: "测试场景".to_string(),
            steps: vec![
                ScenarioStep::Purchase {
                    user_id: "user-001".to_string(),
                    amount: 99.99,
                    delay_ms: Some(100),
                },
                ScenarioStep::Wait { duration_ms: 500 },
                ScenarioStep::CheckIn {
                    user_id: "user-001".to_string(),
                    consecutive_days: 1,
                    delay_ms: None,
                },
            ],
        };

        let json = scenario.to_json().unwrap();
        assert!(json.contains("test-scenario"));
        assert!(json.contains("purchase"));
        assert!(json.contains("wait"));
        assert!(json.contains("check_in"));

        let deserialized = Scenario::from_json(&json).unwrap();
        assert_eq!(deserialized.name, "test-scenario");
        assert_eq!(deserialized.steps.len(), 3);
    }

    #[test]
    fn test_scenario_from_json() {
        let json = r#"{
            "name": "json-scenario",
            "description": "从 JSON 加载",
            "steps": [
                {"type": "purchase", "user_id": "u1", "amount": 100.0},
                {"type": "check_in", "user_id": "u1", "consecutive_days": 5}
            ]
        }"#;

        let scenario = Scenario::from_json(json).unwrap();
        assert_eq!(scenario.name, "json-scenario");
        assert_eq!(scenario.steps.len(), 2);

        match &scenario.steps[0] {
            ScenarioStep::Purchase {
                user_id, amount, ..
            } => {
                assert_eq!(user_id, "u1");
                assert!((amount - 100.0).abs() < f64::EPSILON);
            }
            _ => panic!("预期是 Purchase 步骤"),
        }
    }

    #[test]
    fn test_scenario_from_yaml() {
        let yaml = r#"
name: yaml-scenario
description: 从 YAML 加载
steps:
  - type: purchase
    user_id: u1
    amount: 200.0
  - type: wait
    duration_ms: 1000
  - type: share
    user_id: u1
    platform: wechat
"#;

        let scenario = Scenario::from_yaml(yaml).unwrap();
        assert_eq!(scenario.name, "yaml-scenario");
        assert_eq!(scenario.steps.len(), 3);
    }

    #[test]
    fn test_scenario_builder() {
        let scenario = Scenario::builder("builder-test")
            .description("使用构建器创建")
            .purchase("user-001", 50.0)
            .wait(100)
            .checkin("user-001", 3)
            .build();

        assert_eq!(scenario.name, "builder-test");
        assert_eq!(scenario.description, "使用构建器创建");
        assert_eq!(scenario.steps.len(), 3);
    }

    #[test]
    fn test_step_type_name() {
        assert_eq!(
            ScenarioStep::Purchase {
                user_id: "u".to_string(),
                amount: 1.0,
                delay_ms: None
            }
            .type_name(),
            "purchase"
        );
        assert_eq!(
            ScenarioStep::CheckIn {
                user_id: "u".to_string(),
                consecutive_days: 1,
                delay_ms: None
            }
            .type_name(),
            "check_in"
        );
        assert_eq!(ScenarioStep::Wait { duration_ms: 100 }.type_name(), "wait");
        assert_eq!(
            ScenarioStep::Repeat {
                count: 1,
                steps: vec![]
            }
            .type_name(),
            "repeat"
        );
    }

    #[test]
    fn test_scenario_result_success_rate() {
        let result = ScenarioResult {
            scenario_name: "test".to_string(),
            total_steps: 10,
            success_steps: 8,
            failed_steps: 2,
            total_events_sent: 8,
            duration_ms: 100,
            step_results: vec![],
        };

        assert!(!result.is_all_success());
        assert!((result.success_rate() - 80.0).abs() < f64::EPSILON);

        let all_success = ScenarioResult {
            scenario_name: "test".to_string(),
            total_steps: 5,
            success_steps: 5,
            failed_steps: 0,
            total_events_sent: 5,
            duration_ms: 50,
            step_results: vec![],
        };

        assert!(all_success.is_all_success());
        assert!((all_success.success_rate() - 100.0).abs() < f64::EPSILON);
    }

    #[test]
    fn test_repeat_step_serialization() {
        let step = ScenarioStep::Repeat {
            count: 3,
            steps: vec![
                ScenarioStep::CheckIn {
                    user_id: "u1".to_string(),
                    consecutive_days: 1,
                    delay_ms: None,
                },
                ScenarioStep::Wait { duration_ms: 100 },
            ],
        };

        let json = serde_json::to_string(&step).unwrap();
        assert!(json.contains("repeat"));
        assert!(json.contains("count"));

        let deserialized: ScenarioStep = serde_json::from_str(&json).unwrap();
        match deserialized {
            ScenarioStep::Repeat { count, steps } => {
                assert_eq!(count, 3);
                assert_eq!(steps.len(), 2);
            }
            _ => panic!("预期是 Repeat 步骤"),
        }
    }
}
