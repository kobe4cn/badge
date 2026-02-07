-- 用户徽章操作日志表
-- 记录徽章的发放、取消、兑换等操作，用于审计追踪

CREATE TABLE IF NOT EXISTS user_badge_logs (
    id BIGSERIAL PRIMARY KEY,
    user_badge_id BIGINT REFERENCES user_badges(id),
    user_id VARCHAR(100) NOT NULL,
    badge_id BIGINT NOT NULL REFERENCES badges(id),
    action VARCHAR(20) NOT NULL, -- grant, revoke, expire, redeem, refund
    reason TEXT,
    operator VARCHAR(100),
    quantity INT NOT NULL DEFAULT 1,
    source_type VARCHAR(20) NOT NULL, -- event, scheduled, manual, redemption, system, cascade
    source_ref_id VARCHAR(200),
    remark TEXT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

COMMENT ON TABLE user_badge_logs IS '用户徽章操作日志，用于审计追踪';
COMMENT ON COLUMN user_badge_logs.action IS '操作动作：grant-发放，revoke-取消，expire-过期，redeem-兑换，refund-退还';
COMMENT ON COLUMN user_badge_logs.source_type IS '来源类型：event, scheduled, manual, redemption, system, cascade';
COMMENT ON COLUMN user_badge_logs.source_ref_id IS '关联的业务ID（如事件ID、订单ID）';

CREATE INDEX IF NOT EXISTS idx_user_badge_logs_user ON user_badge_logs(user_id);
CREATE INDEX IF NOT EXISTS idx_user_badge_logs_badge ON user_badge_logs(badge_id);
CREATE INDEX IF NOT EXISTS idx_user_badge_logs_user_badge ON user_badge_logs(user_badge_id);
CREATE INDEX IF NOT EXISTS idx_user_badge_logs_action ON user_badge_logs(action);
CREATE INDEX IF NOT EXISTS idx_user_badge_logs_time ON user_badge_logs(created_at);
