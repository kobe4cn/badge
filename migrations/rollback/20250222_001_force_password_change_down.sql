-- 回滚 20250222_001_force_password_change
ALTER TABLE admin_user DROP COLUMN IF EXISTS must_change_password;
