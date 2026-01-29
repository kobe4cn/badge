//! 场景模拟器模块
//!
//! 提供场景定义、执行和预定义场景集合，用于模拟真实用户行为流程。
//!
//! # 模块结构
//!
//! - `scenario` - 场景定义（Scenario）、步骤（ScenarioStep）和执行器（ScenarioRunner）
//! - `predefined` - 预定义的标准测试场景
//!
//! # 使用示例
//!
//! ```rust,ignore
//! use mock_services::scenarios::{PredefinedScenarios, Scenario, ScenarioRunner};
//! use mock_services::event_generator::BatchEventSender;
//!
//! // 方式 1：使用预定义场景
//! let scenario = PredefinedScenarios::first_purchase();
//!
//! // 方式 2：从 JSON 加载自定义场景
//! let json = r#"{
//!     "name": "custom",
//!     "description": "自定义场景",
//!     "steps": [
//!         {"type": "purchase", "user_id": "u1", "amount": 100.0}
//!     ]
//! }"#;
//! let custom_scenario = Scenario::from_json(json).unwrap();
//!
//! // 方式 3：使用构建器
//! let builder_scenario = Scenario::builder("my-scenario")
//!     .description("构建器创建的场景")
//!     .purchase("user-001", 50.0)
//!     .wait(100)
//!     .checkin("user-001", 1)
//!     .build();
//!
//! // 执行场景
//! // let runner = ScenarioRunner::new(sender);
//! // let result = runner.run(&scenario).await;
//! ```

mod predefined;
mod scenario;

pub use predefined::PredefinedScenarios;
pub use scenario::{
    Scenario, ScenarioBuilder, ScenarioResult, ScenarioRunner, ScenarioStep, StepResult,
};
