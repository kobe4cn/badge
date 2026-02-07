-- 级联评估日志和分布式锁表
-- 支持级联徽章评估的审计追踪和分布式锁机制

-- ==================== 级联评估日志 ====================

CREATE TABLE IF NOT EXISTS cascade_evaluation_logs (
    id BIGSERIAL PRIMARY KEY,
    user_id VARCHAR(64) NOT NULL,
    trigger_badge_id BIGINT NOT NULL REFERENCES badges(id) ON DELETE CASCADE,

    -- 评估上下文快照，记录评估开始时的完整状态
    evaluation_context JSONB NOT NULL, -- {depth, visited_badges, path, ...}

    -- 评估结果
    result_status VARCHAR(20) NOT NULL, -- success, cycle_detected, depth_exceeded, timeout
    granted_badges JSONB, -- [{badge_id, name}, ...]
    blocked_badges JSONB, -- [{badge_id, reason}, ...]
    error_message TEXT,

    -- 性能指标
    started_at TIMESTAMPTZ NOT NULL,
    completed_at TIMESTAMPTZ NOT NULL,
    duration_ms INT NOT NULL,

    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

COMMENT ON TABLE cascade_evaluation_logs IS '级联评估日志，记录每次徽章级联评估的完整过程，用于调试和审计';
COMMENT ON COLUMN cascade_evaluation_logs.user_id IS '被评估用户的 SWID';
COMMENT ON COLUMN cascade_evaluation_logs.trigger_badge_id IS '触发级联评估的起始徽章';
COMMENT ON COLUMN cascade_evaluation_logs.evaluation_context IS '评估上下文快照：depth-当前深度, visited_badges-已访问徽章集合, path-评估路径';
COMMENT ON COLUMN cascade_evaluation_logs.result_status IS '评估结果状态：success-成功, cycle_detected-检测到循环, depth_exceeded-超出深度限制, timeout-超时';
COMMENT ON COLUMN cascade_evaluation_logs.granted_badges IS '本次评估成功发放的徽章列表';
COMMENT ON COLUMN cascade_evaluation_logs.blocked_badges IS '本次评估被阻止的徽章及原因';
COMMENT ON COLUMN cascade_evaluation_logs.duration_ms IS '评估耗时（毫秒），用于性能监控和超时分析';

-- 按用户查询评估历史
CREATE INDEX IF NOT EXISTS idx_cascade_logs_user ON cascade_evaluation_logs(user_id);

-- 按触发徽章分析级联影响
CREATE INDEX IF NOT EXISTS idx_cascade_logs_trigger ON cascade_evaluation_logs(trigger_badge_id);

-- 按状态筛选异常记录（如循环、超时）
CREATE INDEX IF NOT EXISTS idx_cascade_logs_status ON cascade_evaluation_logs(result_status);

-- 按时间范围查询，支持审计和统计
CREATE INDEX IF NOT EXISTS idx_cascade_logs_created ON cascade_evaluation_logs(created_at);

-- ==================== 分布式锁 ====================

CREATE TABLE IF NOT EXISTS distributed_locks (
    lock_key VARCHAR(255) PRIMARY KEY,
    owner_id VARCHAR(100) NOT NULL, -- instance_id + thread_id 组合
    expires_at TIMESTAMPTZ NOT NULL,
    acquired_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    metadata JSONB -- 锁的额外上下文信息
);

COMMENT ON TABLE distributed_locks IS '分布式锁表，作为 Redis 锁的后备方案，当 Redis 不可用时提供基于数据库的锁机制';
COMMENT ON COLUMN distributed_locks.lock_key IS '锁的唯一标识，格式如 cascade:user:{user_id}';
COMMENT ON COLUMN distributed_locks.owner_id IS '锁持有者标识，通常是 instance_id:thread_id 的组合，用于锁续期和释放验证';
COMMENT ON COLUMN distributed_locks.expires_at IS '锁过期时间，过期后可被其他节点获取';
COMMENT ON COLUMN distributed_locks.metadata IS '可选的额外信息，如获取锁的原因、关联的业务ID等';

-- 定期清理过期锁时使用，支持批量删除过期记录
CREATE INDEX IF NOT EXISTS idx_distributed_locks_expires ON distributed_locks(expires_at);
