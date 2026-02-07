-- 权益同步日志：记录每次同步操作的状态和结果
CREATE TABLE IF NOT EXISTS benefit_sync_logs (
    id BIGSERIAL PRIMARY KEY,
    sync_type VARCHAR(50) NOT NULL DEFAULT 'full',
    status VARCHAR(20) NOT NULL DEFAULT 'PENDING',
    total_count INT NOT NULL DEFAULT 0,
    success_count INT NOT NULL DEFAULT 0,
    failed_count INT NOT NULL DEFAULT 0,
    error_message TEXT,
    started_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    completed_at TIMESTAMPTZ,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_benefit_sync_logs_status ON benefit_sync_logs(status);

COMMENT ON TABLE benefit_sync_logs IS '权益同步日志，追踪每次同步任务的进度和结果';

-- 徽章-权益关联：定义哪些徽章自动关联哪些权益
CREATE TABLE IF NOT EXISTS badge_benefit_links (
    id BIGSERIAL PRIMARY KEY,
    badge_id BIGINT NOT NULL REFERENCES badges(id),
    benefit_id BIGINT NOT NULL REFERENCES benefits(id),
    quantity INT NOT NULL DEFAULT 1,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE(badge_id, benefit_id)
);

CREATE INDEX IF NOT EXISTS idx_badge_benefit_links_badge ON badge_benefit_links(badge_id);
CREATE INDEX IF NOT EXISTS idx_badge_benefit_links_benefit ON badge_benefit_links(benefit_id);

COMMENT ON TABLE badge_benefit_links IS '徽章与权益的关联关系，用于自动权益发放';
