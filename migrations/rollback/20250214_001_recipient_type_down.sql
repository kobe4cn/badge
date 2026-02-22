-- 回滚 20250214_001_recipient_type
ALTER TABLE badge_ledger DROP COLUMN IF EXISTS actual_user_id;
ALTER TABLE badge_ledger DROP COLUMN IF EXISTS recipient_type;
DROP INDEX IF EXISTS idx_user_badges_actual_user;
ALTER TABLE user_badges DROP COLUMN IF EXISTS actual_user_id;
ALTER TABLE user_badges DROP COLUMN IF EXISTS recipient_type;
