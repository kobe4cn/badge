-- 过期处理 Worker 支持
-- 添加过期提醒标记和优化查询索引

-- 添加过期提醒已发送标记（避免重复发送）
DO $$ BEGIN
    ALTER TABLE user_badges ADD COLUMN expire_reminded BOOLEAN NOT NULL DEFAULT FALSE;
EXCEPTION WHEN duplicate_column THEN NULL;
END $$;

-- 添加过期处理时间戳（记录何时被系统标记为过期）
DO $$ BEGIN
    ALTER TABLE user_badges ADD COLUMN expired_at TIMESTAMPTZ;
EXCEPTION WHEN duplicate_column THEN NULL;
END $$;

COMMENT ON COLUMN user_badges.expire_reminded IS '是否已发送过期提醒通知，避免重复发送';
COMMENT ON COLUMN user_badges.expired_at IS '实际过期处理时间，由 ExpireWorker 写入';

-- 优化过期查询索引：查找即将过期的活跃徽章
CREATE INDEX IF NOT EXISTS idx_user_badges_expires_active
    ON user_badges(expires_at)
    WHERE status = 'active' AND expires_at IS NOT NULL;

-- 优化过期提醒查询索引：查找需要发送提醒的徽章
CREATE INDEX IF NOT EXISTS idx_user_badges_expire_remind
    ON user_badges(expires_at, expire_reminded)
    WHERE status = 'active' AND expires_at IS NOT NULL AND expire_reminded = FALSE;

-- 添加复合索引：用户 + 过期状态查询
CREATE INDEX IF NOT EXISTS idx_user_badges_user_expired
    ON user_badges(user_id)
    WHERE status = 'expired';
