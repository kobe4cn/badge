-- 回滚 20250217_001_badge_code
DROP INDEX IF EXISTS idx_badges_code;
ALTER TABLE badges DROP COLUMN IF EXISTS code;
