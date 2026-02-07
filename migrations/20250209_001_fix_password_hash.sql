-- 修复密码哈希
-- 之前的密码哈希格式不正确，这里更新为正确的 bcrypt 哈希

-- admin 用户密码: admin123
UPDATE admin_user
SET password_hash = '$2b$12$16C5oGtzpTqIacMpazO4Ee0RY1ynSbjWzAsYppO8H1UCrrSR1SSju'
WHERE username = 'admin';

-- operator 用户密码: operator123
UPDATE admin_user
SET password_hash = '$2b$12$nKZgCHbvHx84mQF/BUOQSuuQZ5EA9hY/QKy9tMSlTQcboHI8WUr9a'
WHERE username = 'operator';

-- viewer 用户密码: viewer123
UPDATE admin_user
SET password_hash = '$2b$12$x97BU2ItIckH7iARLWQ8IuBENVKA70jzgK97fspbx.V4Xa/g.hXdO'
WHERE username = 'viewer';
