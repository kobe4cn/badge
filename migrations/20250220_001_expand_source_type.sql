-- 扩展 source_type 字段长度以支持更长的自动取消场景标识
-- 如 AUTO_ACCOUNT_DELETION, AUTO_IDENTITY_CHANGE 等

-- badge_ledger 表
ALTER TABLE badge_ledger
ALTER COLUMN source_type TYPE VARCHAR(50);

-- user_badge_logs 表
ALTER TABLE user_badge_logs
ALTER COLUMN source_type TYPE VARCHAR(50);

COMMENT ON COLUMN badge_ledger.source_type IS '来源类型：EVENT, SCHEDULED, MANUAL, REDEMPTION, CASCADE, SYSTEM, AUTO_*';
COMMENT ON COLUMN user_badge_logs.source_type IS '来源类型：EVENT, SCHEDULED, MANUAL, REDEMPTION, CASCADE, SYSTEM, AUTO_*';
