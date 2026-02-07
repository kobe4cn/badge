-- 添加兑换规则相对有效期支持
-- 支持"获取徽章后 N 天可兑换"的相对配置

-- 添加有效期类型字段
ALTER TABLE badge_redemption_rules
ADD COLUMN IF NOT EXISTS validity_type VARCHAR(20) DEFAULT 'FIXED';
-- FIXED: 固定时间段 (使用 start_time/end_time)
-- RELATIVE: 相对于徽章获取时间

-- 添加相对有效天数字段
ALTER TABLE badge_redemption_rules
ADD COLUMN IF NOT EXISTS relative_days INT;
-- 获取徽章后多少天内可兑换（validity_type=RELATIVE 时使用）

COMMENT ON COLUMN badge_redemption_rules.validity_type IS '有效期类型：FIXED-固定时间段，RELATIVE-相对徽章获取时间';
COMMENT ON COLUMN badge_redemption_rules.relative_days IS '相对有效天数（validity_type=RELATIVE 时使用）';
