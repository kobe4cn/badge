-- 发放对象类型支持
-- 区分"账号注册人"和"实际使用人"两种发放对象

-- 添加发放对象类型列
-- OWNER: 账号注册人（默认，原有行为）
-- USER: 实际使用人（需要填写 actual_user_id）
ALTER TABLE user_badges
ADD COLUMN IF NOT EXISTS recipient_type VARCHAR(20) NOT NULL DEFAULT 'OWNER';

-- 添加实际使用人 ID
-- 当 recipient_type = 'USER' 时，记录实际使用人的标识
ALTER TABLE user_badges
ADD COLUMN IF NOT EXISTS actual_user_id VARCHAR(100);

COMMENT ON COLUMN user_badges.recipient_type IS '发放对象类型：OWNER-账号注册人，USER-实际使用人';
COMMENT ON COLUMN user_badges.actual_user_id IS '实际使用人 ID，当 recipient_type=USER 时必填';

-- 索引：用于按实际使用人查询
CREATE INDEX IF NOT EXISTS idx_user_badges_actual_user
ON user_badges(actual_user_id) WHERE actual_user_id IS NOT NULL;

-- 同步更新 badge_ledger 表
ALTER TABLE badge_ledger
ADD COLUMN IF NOT EXISTS recipient_type VARCHAR(20) NOT NULL DEFAULT 'OWNER';

ALTER TABLE badge_ledger
ADD COLUMN IF NOT EXISTS actual_user_id VARCHAR(100);

COMMENT ON COLUMN badge_ledger.recipient_type IS '发放对象类型：OWNER-账号注册人，USER-实际使用人';
COMMENT ON COLUMN badge_ledger.actual_user_id IS '实际使用人 ID';
