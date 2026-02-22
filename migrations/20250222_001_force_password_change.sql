-- 强制种子用户修改默认密码
-- 迁移中的硬编码密码（admin123 等）是公开的安全风险，
-- 通过标记 must_change_password 强制首次登录修改

ALTER TABLE admin_user ADD COLUMN IF NOT EXISTS must_change_password BOOLEAN NOT NULL DEFAULT FALSE;

-- 标记种子用户需要强制修改密码
UPDATE admin_user SET must_change_password = TRUE
WHERE username IN ('admin', 'operator', 'viewer');
