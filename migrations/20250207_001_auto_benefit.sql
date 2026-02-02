-- 自动权益发放相关表
-- 实现"获得徽章时自动发放权益"功能所需的数据结构

-- ============================================
-- 1. 自动权益发放记录表
-- ============================================

-- 核心作用：幂等控制
-- 当用户获得徽章时，系统自动评估是否满足兑换规则，若满足则自动发放权益
-- 通过 idempotency_key 确保同一触发条件不会重复发放
CREATE TABLE auto_benefit_grants (
    id BIGSERIAL PRIMARY KEY,
    user_id VARCHAR(100) NOT NULL,                                              -- SWID，用户唯一标识
    rule_id BIGINT NOT NULL REFERENCES badge_redemption_rules(id),              -- 触发的兑换规则
    trigger_badge_id BIGINT NOT NULL REFERENCES badges(id),                     -- 触发此次自动发放的徽章
    trigger_user_badge_id BIGINT NOT NULL REFERENCES user_badges(id),           -- 触发此次自动发放的用户徽章记录
    benefit_grant_id BIGINT REFERENCES benefit_grants(id),                      -- 关联的权益发放记录，成功发放后填充

    -- 幂等控制：相同用户+规则+触发徽章组合只能发放一次
    -- 格式：auto_benefit:{user_id}:{rule_id}:{trigger_user_badge_id}
    idempotency_key VARCHAR(200) NOT NULL,

    -- 状态管理
    -- PENDING: 待处理，已创建记录但尚未开始发放
    -- PROCESSING: 处理中，正在调用权益发放服务
    -- SUCCESS: 发放成功
    -- FAILED: 发放失败
    -- SKIPPED: 已跳过（如用户不满足前置条件）
    status VARCHAR(20) NOT NULL DEFAULT 'PENDING',
    error_message TEXT,                                                         -- 失败或跳过的原因

    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    completed_at TIMESTAMPTZ,                                                   -- 处理完成时间（成功或失败）
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),

    CONSTRAINT auto_benefit_grants_idempotency_unique UNIQUE (idempotency_key),
    CONSTRAINT auto_benefit_grants_valid_status CHECK (status IN ('PENDING', 'PROCESSING', 'SUCCESS', 'FAILED', 'SKIPPED'))
);

COMMENT ON TABLE auto_benefit_grants IS '自动权益发放记录，用于幂等控制和追踪自动发放流程';
COMMENT ON COLUMN auto_benefit_grants.user_id IS 'SWID，接收权益的用户标识';
COMMENT ON COLUMN auto_benefit_grants.rule_id IS '触发自动发放的兑换规则ID';
COMMENT ON COLUMN auto_benefit_grants.trigger_badge_id IS '触发此次自动发放的徽章ID';
COMMENT ON COLUMN auto_benefit_grants.trigger_user_badge_id IS '触发此次自动发放的用户徽章记录ID';
COMMENT ON COLUMN auto_benefit_grants.benefit_grant_id IS '关联的权益发放记录ID，发放成功后填充';
COMMENT ON COLUMN auto_benefit_grants.idempotency_key IS '幂等键，确保相同触发条件不会重复发放';
COMMENT ON COLUMN auto_benefit_grants.status IS '状态：PENDING-待处理，PROCESSING-处理中，SUCCESS-成功，FAILED-失败，SKIPPED-已跳过';
COMMENT ON COLUMN auto_benefit_grants.error_message IS '失败或跳过的原因说明';
COMMENT ON COLUMN auto_benefit_grants.completed_at IS '处理完成时间，无论成功或失败';

-- ============================================
-- 2. 自动权益评估日志表
-- ============================================

-- 核心作用：调试和审计
-- 记录每次徽章获得时的自动权益评估过程，便于问题排查和业务分析
CREATE TABLE auto_benefit_evaluation_logs (
    id BIGSERIAL PRIMARY KEY,
    user_id VARCHAR(100) NOT NULL,                                              -- SWID，被评估的用户
    trigger_badge_id BIGINT NOT NULL REFERENCES badges(id),                     -- 触发评估的徽章

    -- 评估上下文：记录评估时的用户状态快照
    -- 包含：用户持有的徽章列表、历史兑换记录等
    evaluation_context JSONB NOT NULL,

    -- 评估结果统计
    rules_evaluated INT NOT NULL DEFAULT 0,                                     -- 评估的规则总数
    rules_matched INT NOT NULL DEFAULT 0,                                       -- 匹配的规则数量
    grants_created INT NOT NULL DEFAULT 0,                                      -- 创建的发放记录数量

    -- 性能追踪
    duration_ms BIGINT NOT NULL,                                                -- 评估耗时（毫秒）

    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

COMMENT ON TABLE auto_benefit_evaluation_logs IS '自动权益评估日志，记录每次评估过程用于调试和审计';
COMMENT ON COLUMN auto_benefit_evaluation_logs.user_id IS 'SWID，被评估的用户标识';
COMMENT ON COLUMN auto_benefit_evaluation_logs.trigger_badge_id IS '触发此次评估的徽章ID';
COMMENT ON COLUMN auto_benefit_evaluation_logs.evaluation_context IS '评估上下文JSON，包含用户徽章状态、历史兑换等快照数据';
COMMENT ON COLUMN auto_benefit_evaluation_logs.rules_evaluated IS '本次评估检查的规则总数';
COMMENT ON COLUMN auto_benefit_evaluation_logs.rules_matched IS '满足条件的规则数量';
COMMENT ON COLUMN auto_benefit_evaluation_logs.grants_created IS '实际创建的自动发放记录数量';
COMMENT ON COLUMN auto_benefit_evaluation_logs.duration_ms IS '评估过程总耗时（毫秒），用于性能监控';

-- ============================================
-- 3. 创建索引
-- ============================================

-- auto_benefit_grants 索引

-- 用户+规则组合查询：检查用户是否已通过某规则获得过自动权益
CREATE INDEX idx_auto_benefit_grants_user_rule ON auto_benefit_grants(user_id, rule_id);

-- 触发徽章查询：通过用户徽章ID反查自动发放记录
CREATE INDEX idx_auto_benefit_grants_trigger ON auto_benefit_grants(trigger_user_badge_id);

-- 状态查询：查找待处理或失败的记录
CREATE INDEX idx_auto_benefit_grants_status ON auto_benefit_grants(status);

-- 时间范围查询：按创建时间统计和清理
CREATE INDEX idx_auto_benefit_grants_created ON auto_benefit_grants(created_at);

-- 权益发放关联查询：通过 benefit_grant_id 反查
CREATE INDEX idx_auto_benefit_grants_benefit ON auto_benefit_grants(benefit_grant_id)
    WHERE benefit_grant_id IS NOT NULL;

-- auto_benefit_evaluation_logs 索引

-- 用户+时间组合查询：查询用户的评估历史
CREATE INDEX idx_auto_benefit_eval_user ON auto_benefit_evaluation_logs(user_id, created_at);

-- 触发徽章查询：分析某徽章触发的评估情况
CREATE INDEX idx_auto_benefit_eval_badge ON auto_benefit_evaluation_logs(trigger_badge_id);

-- 时间范围查询：按时间统计和清理历史数据
CREATE INDEX idx_auto_benefit_eval_time ON auto_benefit_evaluation_logs(created_at);

-- 徽章+时间复合查询：分析某徽章在时间区间内的触发情况
CREATE INDEX idx_auto_benefit_eval_badge_time ON auto_benefit_evaluation_logs(trigger_badge_id, created_at);

-- ============================================
-- 4. 创建触发器
-- ============================================

CREATE TRIGGER update_auto_benefit_grants_updated_at
    BEFORE UPDATE ON auto_benefit_grants
    FOR EACH ROW EXECUTE FUNCTION update_updated_at_column();
