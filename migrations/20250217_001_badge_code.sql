-- 为徽章添加业务唯一编码
-- 用于外部系统对接，提供稳定的业务标识

-- 添加 code 字段
ALTER TABLE badges
ADD COLUMN IF NOT EXISTS code VARCHAR(50);

-- 创建唯一索引（允许 NULL，只对非空值做唯一约束）
CREATE UNIQUE INDEX IF NOT EXISTS idx_badges_code
ON badges(code) WHERE code IS NOT NULL;

COMMENT ON COLUMN badges.code IS '业务唯一编码，用于外部系统对接';
