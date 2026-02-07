-- 回滚脚本：撤销密码哈希修复
-- 对应 UP 迁移：20250209_001_fix_password_hash.sql
-- 此迁移仅更新了已有用户的密码哈希值，无法精确恢复原始哈希
-- 回滚策略：将密码重置为一个已知的占位哈希，要求用户重新设置密码

-- 注意：原始密码哈希已不可恢复，以下操作将密码设置为无效值
-- 管理员需要在回滚后手动重置相关用户密码

UPDATE admin_user
SET password_hash = 'ROLLBACK_REQUIRED_RESET_PASSWORD'
WHERE username IN ('admin', 'operator', 'viewer');
