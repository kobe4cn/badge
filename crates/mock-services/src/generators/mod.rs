//! 生成器模块
//!
//! 提供测试数据的批量生成功能。

pub mod data_generator;

pub use data_generator::{DataGenerator, GenerationStats, GeneratorConfig};
