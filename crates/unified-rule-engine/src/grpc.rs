//! gRPC 服务实现
//!
//! 实现 RuleEngineService gRPC 服务接口。

#![allow(clippy::result_large_err)]

use crate::executor::RuleExecutor;
use crate::models::{Condition, EvaluationContext, LogicalGroup, Rule, RuleNode};
use crate::operators::{LogicalOperator, Operator};
use crate::store::RuleStore;
use badge_proto::rule_engine::rule_engine_service_server::RuleEngineService;
use badge_proto::rule_engine::{
    BatchEvaluateRequest, BatchEvaluateResponse, ConditionNode, DeleteRuleRequest,
    DeleteRuleResponse, EvaluateRequest, EvaluateResponse, GroupNode, LoadRuleRequest,
    LoadRuleResponse, LogicalOperator as ProtoLogicalOperator, Operator as ProtoOperator,
    Rule as ProtoRule, RuleNode as ProtoRuleNode, TestRuleRequest, TestRuleResponse,
};
use std::sync::Arc;
use tonic::{Request, Response, Status};
use tracing::{info, instrument, warn};

/// gRPC 服务实现
pub struct RuleEngineServiceImpl {
    store: RuleStore,
    executor: Arc<RuleExecutor>,
}

impl RuleEngineServiceImpl {
    pub fn new(store: RuleStore) -> Self {
        Self {
            store,
            executor: Arc::new(RuleExecutor::new().with_trace()),
        }
    }

    /// Proto Rule 转换为内部 Rule
    fn convert_rule(proto: &ProtoRule) -> Result<Rule, Status> {
        let root = Self::convert_rule_node(
            proto
                .root
                .as_ref()
                .ok_or_else(|| Status::invalid_argument("规则根节点不能为空"))?,
        )?;

        Ok(Rule {
            id: proto.id.clone(),
            name: proto.name.clone(),
            version: proto.version.clone(),
            root,
            created_at: proto
                .created_at
                .as_ref()
                .map(|t| {
                    chrono::DateTime::from_timestamp(t.seconds, t.nanos as u32)
                        .unwrap_or_else(chrono::Utc::now)
                })
                .unwrap_or_else(chrono::Utc::now),
            updated_at: proto
                .updated_at
                .as_ref()
                .map(|t| {
                    chrono::DateTime::from_timestamp(t.seconds, t.nanos as u32)
                        .unwrap_or_else(chrono::Utc::now)
                })
                .unwrap_or_else(chrono::Utc::now),
        })
    }

    /// 转换规则节点
    fn convert_rule_node(proto: &ProtoRuleNode) -> Result<RuleNode, Status> {
        match &proto.node {
            Some(badge_proto::rule_engine::rule_node::Node::Condition(cond)) => {
                Ok(RuleNode::Condition(Self::convert_condition(cond)?))
            }
            Some(badge_proto::rule_engine::rule_node::Node::Group(group)) => {
                Ok(RuleNode::Group(Self::convert_group(group)?))
            }
            None => Err(Status::invalid_argument("规则节点不能为空")),
        }
    }

    /// 转换条件节点
    fn convert_condition(proto: &ConditionNode) -> Result<Condition, Status> {
        let operator = Self::convert_operator(proto.operator())?;
        let value = proto
            .value
            .as_ref()
            .map(Self::convert_value)
            .unwrap_or(serde_json::Value::Null);

        Ok(Condition {
            field: proto.field.clone(),
            operator,
            value,
        })
    }

    /// 转换逻辑组
    fn convert_group(proto: &GroupNode) -> Result<LogicalGroup, Status> {
        let operator = Self::convert_logical_operator(proto.operator())?;
        let children: Result<Vec<RuleNode>, Status> =
            proto.children.iter().map(Self::convert_rule_node).collect();

        Ok(LogicalGroup {
            operator,
            children: children?,
        })
    }

    /// 转换操作符
    fn convert_operator(proto: ProtoOperator) -> Result<Operator, Status> {
        match proto {
            ProtoOperator::Eq => Ok(Operator::Eq),
            ProtoOperator::Neq => Ok(Operator::Neq),
            ProtoOperator::Gt => Ok(Operator::Gt),
            ProtoOperator::Gte => Ok(Operator::Gte),
            ProtoOperator::Lt => Ok(Operator::Lt),
            ProtoOperator::Lte => Ok(Operator::Lte),
            ProtoOperator::Between => Ok(Operator::Between),
            ProtoOperator::In => Ok(Operator::In),
            ProtoOperator::NotIn => Ok(Operator::NotIn),
            ProtoOperator::Contains => Ok(Operator::Contains),
            ProtoOperator::StartsWith => Ok(Operator::StartsWith),
            ProtoOperator::EndsWith => Ok(Operator::EndsWith),
            ProtoOperator::Regex => Ok(Operator::Regex),
            ProtoOperator::IsEmpty => Ok(Operator::IsEmpty),
            ProtoOperator::IsNotEmpty => Ok(Operator::IsNotEmpty),
            ProtoOperator::ContainsAny => Ok(Operator::ContainsAny),
            ProtoOperator::ContainsAll => Ok(Operator::ContainsAll),
            ProtoOperator::Before => Ok(Operator::Before),
            ProtoOperator::After => Ok(Operator::After),
            ProtoOperator::Unspecified => Err(Status::invalid_argument("未指定操作符")),
        }
    }

    /// 转换逻辑操作符
    fn convert_logical_operator(proto: ProtoLogicalOperator) -> Result<LogicalOperator, Status> {
        match proto {
            ProtoLogicalOperator::And => Ok(LogicalOperator::And),
            ProtoLogicalOperator::Or => Ok(LogicalOperator::Or),
            ProtoLogicalOperator::Unspecified => Err(Status::invalid_argument("未指定逻辑操作符")),
        }
    }

    /// 转换 protobuf Value 到 serde_json Value
    fn convert_value(proto: &prost_types::Value) -> serde_json::Value {
        match &proto.kind {
            Some(prost_types::value::Kind::NullValue(_)) => serde_json::Value::Null,
            Some(prost_types::value::Kind::NumberValue(n)) => serde_json::json!(*n),
            Some(prost_types::value::Kind::StringValue(s)) => serde_json::json!(s),
            Some(prost_types::value::Kind::BoolValue(b)) => serde_json::json!(*b),
            Some(prost_types::value::Kind::ListValue(list)) => {
                let values: Vec<serde_json::Value> =
                    list.values.iter().map(Self::convert_value).collect();
                serde_json::Value::Array(values)
            }
            Some(prost_types::value::Kind::StructValue(s)) => {
                let map: serde_json::Map<String, serde_json::Value> = s
                    .fields
                    .iter()
                    .map(|(k, v)| (k.clone(), Self::convert_value(v)))
                    .collect();
                serde_json::Value::Object(map)
            }
            None => serde_json::Value::Null,
        }
    }

    /// 转换 protobuf Struct 到 EvaluationContext
    fn convert_context(proto: Option<&prost_types::Struct>) -> EvaluationContext {
        match proto {
            Some(s) => {
                let map: serde_json::Map<String, serde_json::Value> = s
                    .fields
                    .iter()
                    .map(|(k, v)| (k.clone(), Self::convert_value(v)))
                    .collect();
                EvaluationContext::new(serde_json::Value::Object(map))
            }
            None => EvaluationContext::default(),
        }
    }
}

#[tonic::async_trait]
impl RuleEngineService for RuleEngineServiceImpl {
    /// 评估单条规则
    #[instrument(skip(self, request))]
    async fn evaluate(
        &self,
        request: Request<EvaluateRequest>,
    ) -> Result<Response<EvaluateResponse>, Status> {
        let req = request.into_inner();

        let rule = self
            .store
            .get(&req.rule_id)
            .ok_or_else(|| Status::not_found(format!("规则不存在: {}", req.rule_id)))?;

        let context = Self::convert_context(req.context.as_ref());

        let result = self
            .executor
            .execute(&rule, &context)
            .map_err(|e| Status::internal(e.to_string()))?;

        Ok(Response::new(EvaluateResponse {
            matched: result.matched,
            rule_id: result.rule_id,
            rule_name: result.rule_name,
            matched_conditions: result.matched_conditions,
            evaluation_time_ms: result.evaluation_time_ms,
        }))
    }

    /// 批量评估规则
    #[instrument(skip(self, request))]
    async fn batch_evaluate(
        &self,
        request: Request<BatchEvaluateRequest>,
    ) -> Result<Response<BatchEvaluateResponse>, Status> {
        let req = request.into_inner();
        let start = std::time::Instant::now();

        let context = Self::convert_context(req.context.as_ref());
        let mut results = Vec::with_capacity(req.rule_ids.len());

        for rule_id in &req.rule_ids {
            let rule = match self.store.get(rule_id) {
                Some(r) => r,
                None => {
                    warn!("规则不存在: {}", rule_id);
                    continue;
                }
            };

            match self.executor.execute(&rule, &context) {
                Ok(result) => {
                    results.push(EvaluateResponse {
                        matched: result.matched,
                        rule_id: result.rule_id,
                        rule_name: result.rule_name,
                        matched_conditions: result.matched_conditions,
                        evaluation_time_ms: result.evaluation_time_ms,
                    });
                }
                Err(e) => {
                    warn!("规则执行失败: {} - {}", rule_id, e);
                }
            }
        }

        Ok(Response::new(BatchEvaluateResponse {
            results,
            total_evaluation_time_ms: start.elapsed().as_millis() as i64,
        }))
    }

    /// 加载规则
    #[instrument(skip(self, request))]
    async fn load_rule(
        &self,
        request: Request<LoadRuleRequest>,
    ) -> Result<Response<LoadRuleResponse>, Status> {
        let req = request.into_inner();

        let proto_rule = req
            .rule
            .ok_or_else(|| Status::invalid_argument("规则不能为空"))?;

        let rule = Self::convert_rule(&proto_rule)?;
        let rule_id = rule.id.clone();

        self.store
            .load(rule)
            .map_err(|e| Status::internal(e.to_string()))?;

        info!("规则已加载: {}", rule_id);

        Ok(Response::new(LoadRuleResponse {
            success: true,
            message: format!("规则 {} 加载成功", rule_id),
        }))
    }

    /// 删除规则
    #[instrument(skip(self, request))]
    async fn delete_rule(
        &self,
        request: Request<DeleteRuleRequest>,
    ) -> Result<Response<DeleteRuleResponse>, Status> {
        let req = request.into_inner();

        self.store
            .delete(&req.rule_id)
            .map_err(|e| Status::not_found(e.to_string()))?;

        info!("规则已删除: {}", req.rule_id);

        Ok(Response::new(DeleteRuleResponse {
            success: true,
            message: format!("规则 {} 删除成功", req.rule_id),
        }))
    }

    /// 测试规则（不需要预先加载）
    #[instrument(skip(self, request))]
    async fn test_rule(
        &self,
        request: Request<TestRuleRequest>,
    ) -> Result<Response<TestRuleResponse>, Status> {
        let req = request.into_inner();

        let proto_rule = req
            .rule
            .ok_or_else(|| Status::invalid_argument("规则不能为空"))?;

        let rule = Self::convert_rule(&proto_rule)?;

        // 临时编译规则
        let mut compiler = crate::compiler::RuleCompiler::new();
        let compiled = compiler
            .compile(rule)
            .map_err(|e| Status::invalid_argument(e.to_string()))?;

        let context = Self::convert_context(req.context.as_ref());

        // 使用带追踪的执行器
        let executor = RuleExecutor::new().with_trace();
        let result = executor
            .execute(&compiled, &context)
            .map_err(|e| Status::internal(e.to_string()))?;

        Ok(Response::new(TestRuleResponse {
            matched: result.matched,
            matched_conditions: result.matched_conditions,
            evaluation_trace: result.evaluation_trace,
            evaluation_time_ms: result.evaluation_time_ms,
        }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use prost_types::{Struct, Value as ProtoValue, value::Kind};
    use std::collections::BTreeMap;

    fn create_test_context() -> prost_types::Struct {
        let mut fields = BTreeMap::new();

        // event.type = "PURCHASE"
        let mut event_fields = BTreeMap::new();
        event_fields.insert(
            "type".to_string(),
            ProtoValue {
                kind: Some(Kind::StringValue("PURCHASE".to_string())),
            },
        );
        fields.insert(
            "event".to_string(),
            ProtoValue {
                kind: Some(Kind::StructValue(Struct {
                    fields: event_fields,
                })),
            },
        );

        // order.amount = 1000
        let mut order_fields = BTreeMap::new();
        order_fields.insert(
            "amount".to_string(),
            ProtoValue {
                kind: Some(Kind::NumberValue(1000.0)),
            },
        );
        fields.insert(
            "order".to_string(),
            ProtoValue {
                kind: Some(Kind::StructValue(Struct {
                    fields: order_fields,
                })),
            },
        );

        Struct { fields }
    }

    #[test]
    fn test_convert_value() {
        let proto = ProtoValue {
            kind: Some(Kind::StringValue("test".to_string())),
        };
        let value = RuleEngineServiceImpl::convert_value(&proto);
        assert_eq!(value, serde_json::json!("test"));
    }

    #[test]
    fn test_convert_context() {
        let context = create_test_context();
        let eval_context = RuleEngineServiceImpl::convert_context(Some(&context));

        assert_eq!(
            eval_context.get_field("event.type"),
            Some(&serde_json::json!("PURCHASE"))
        );
        assert_eq!(
            eval_context.get_field("order.amount"),
            Some(&serde_json::json!(1000.0))
        );
    }
}
