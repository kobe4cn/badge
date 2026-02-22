-- 回滚 20250220_001_expand_source_type
-- 注意：缩小 VARCHAR 长度时如果已有超长数据会失败，需先清理
ALTER TABLE user_badge_logs ALTER COLUMN source_type TYPE VARCHAR(20);
ALTER TABLE badge_ledger ALTER COLUMN source_type TYPE VARCHAR(20);
