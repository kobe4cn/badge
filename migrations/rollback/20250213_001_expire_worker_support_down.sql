-- 回滚 20250213_001_expire_worker_support
DROP INDEX IF EXISTS idx_user_badges_user_expired;
DROP INDEX IF EXISTS idx_user_badges_expire_remind;
DROP INDEX IF EXISTS idx_user_badges_expires_active;
ALTER TABLE user_badges DROP COLUMN IF EXISTS expired_at;
ALTER TABLE user_badges DROP COLUMN IF EXISTS expire_reminded;
