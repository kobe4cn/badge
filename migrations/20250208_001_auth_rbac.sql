-- 认证与权限管理模块
-- 包含系统用户、角色、权限和 API Key 管理

-- ============================================
-- 系统用户表
-- ============================================
CREATE TABLE IF NOT EXISTS admin_user (
    id BIGSERIAL PRIMARY KEY,
    username VARCHAR(50) NOT NULL UNIQUE,
    password_hash VARCHAR(200) NOT NULL,
    email VARCHAR(100),
    display_name VARCHAR(100),
    avatar_url VARCHAR(500),
    status VARCHAR(20) NOT NULL DEFAULT 'ACTIVE', -- ACTIVE, DISABLED, LOCKED
    failed_login_attempts INT NOT NULL DEFAULT 0,
    locked_until TIMESTAMPTZ,
    last_login_at TIMESTAMPTZ,
    password_changed_at TIMESTAMPTZ,
    created_by BIGINT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- 用户索引
CREATE INDEX IF NOT EXISTS idx_admin_user_username ON admin_user(username);
CREATE INDEX IF NOT EXISTS idx_admin_user_status ON admin_user(status);
CREATE INDEX IF NOT EXISTS idx_admin_user_email ON admin_user(email) WHERE email IS NOT NULL;

-- ============================================
-- 角色表
-- ============================================
CREATE TABLE IF NOT EXISTS role (
    id BIGSERIAL PRIMARY KEY,
    code VARCHAR(50) NOT NULL UNIQUE,
    name VARCHAR(100) NOT NULL,
    description TEXT,
    is_system BOOLEAN NOT NULL DEFAULT FALSE, -- 系统内置角色不可删除
    enabled BOOLEAN NOT NULL DEFAULT TRUE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- 角色索引
CREATE INDEX IF NOT EXISTS idx_role_code ON role(code);
CREATE INDEX IF NOT EXISTS idx_role_enabled ON role(enabled);

-- ============================================
-- 权限表
-- ============================================
CREATE TABLE IF NOT EXISTS permission (
    id BIGSERIAL PRIMARY KEY,
    code VARCHAR(100) NOT NULL UNIQUE,
    name VARCHAR(100) NOT NULL,
    module VARCHAR(50) NOT NULL, -- system, badge, rule, grant, benefit, stats, log
    action VARCHAR(50) NOT NULL, -- read, write, publish, delete
    resource_pattern VARCHAR(200), -- 资源匹配模式，如 /badges/*
    description TEXT,
    sort_order INT NOT NULL DEFAULT 0,
    enabled BOOLEAN NOT NULL DEFAULT TRUE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- 权限索引
CREATE INDEX IF NOT EXISTS idx_permission_module ON permission(module);
CREATE INDEX IF NOT EXISTS idx_permission_code ON permission(code);

-- ============================================
-- 用户-角色关联表
-- ============================================
CREATE TABLE IF NOT EXISTS user_role (
    user_id BIGINT NOT NULL REFERENCES admin_user(id) ON DELETE CASCADE,
    role_id BIGINT NOT NULL REFERENCES role(id) ON DELETE CASCADE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    PRIMARY KEY (user_id, role_id)
);

-- 用户角色索引
CREATE INDEX IF NOT EXISTS idx_user_role_user ON user_role(user_id);
CREATE INDEX IF NOT EXISTS idx_user_role_role ON user_role(role_id);

-- ============================================
-- 角色-权限关联表
-- ============================================
CREATE TABLE IF NOT EXISTS role_permission (
    role_id BIGINT NOT NULL REFERENCES role(id) ON DELETE CASCADE,
    permission_id BIGINT NOT NULL REFERENCES permission(id) ON DELETE CASCADE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    PRIMARY KEY (role_id, permission_id)
);

-- 角色权限索引
CREATE INDEX IF NOT EXISTS idx_role_permission_role ON role_permission(role_id);
CREATE INDEX IF NOT EXISTS idx_role_permission_perm ON role_permission(permission_id);

-- ============================================
-- 外部 API Key 表
-- ============================================
CREATE TABLE IF NOT EXISTS api_key (
    id BIGSERIAL PRIMARY KEY,
    name VARCHAR(100) NOT NULL,
    key_prefix VARCHAR(10) NOT NULL, -- 用于标识 key 的前缀，如 "bk_"
    key_hash VARCHAR(200) NOT NULL,  -- SHA256 哈希存储
    permissions JSONB NOT NULL DEFAULT '[]'::jsonb, -- 允许的权限码列表
    rate_limit INT DEFAULT 1000, -- 每分钟请求限制
    expires_at TIMESTAMPTZ,
    enabled BOOLEAN NOT NULL DEFAULT TRUE,
    last_used_at TIMESTAMPTZ,
    created_by BIGINT REFERENCES admin_user(id),
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- API Key 索引
CREATE INDEX IF NOT EXISTS idx_api_key_prefix ON api_key(key_prefix);
CREATE INDEX IF NOT EXISTS idx_api_key_enabled ON api_key(enabled);

-- ============================================
-- 初始化默认角色
-- ============================================
INSERT INTO role (code, name, description, is_system) VALUES
('admin', '超级管理员', '拥有系统所有权限，可管理用户、角色和系统配置', TRUE),
('operator', '运营人员', '负责徽章、规则、发放等日常运营管理', TRUE),
('viewer', '只读用户', '仅可查看数据，无法进行任何修改操作', TRUE)
ON CONFLICT (code) DO NOTHING;

-- ============================================
-- 初始化权限定义
-- ============================================
INSERT INTO permission (code, name, module, action, resource_pattern, description, sort_order) VALUES
-- 系统管理模块 (module: system)
('system:user:read', '查看用户', 'system', 'read', '/system/users/*', '查看系统用户列表和详情', 100),
('system:user:write', '管理用户', 'system', 'write', '/system/users/*', '创建、编辑、删除系统用户', 101),
('system:role:read', '查看角色', 'system', 'read', '/system/roles/*', '查看角色列表和详情', 110),
('system:role:write', '管理角色', 'system', 'write', '/system/roles/*', '创建、编辑、删除角色及分配权限', 111),
('system:apikey:read', '查看 API Key', 'system', 'read', '/system/api-keys/*', '查看 API Key 列表', 120),
('system:apikey:write', '管理 API Key', 'system', 'write', '/system/api-keys/*', '创建、删除、重新生成 API Key', 121),

-- 徽章管理模块 (module: badge)
('badge:category:read', '查看分类', 'badge', 'read', '/categories/*', '查看徽章分类列表和详情', 200),
('badge:category:write', '管理分类', 'badge', 'write', '/categories/*', '创建、编辑、删除徽章分类', 201),
('badge:series:read', '查看系列', 'badge', 'read', '/series/*', '查看徽章系列列表和详情', 210),
('badge:series:write', '管理系列', 'badge', 'write', '/series/*', '创建、编辑、删除徽章系列', 211),
('badge:badge:read', '查看徽章', 'badge', 'read', '/badges/*', '查看徽章列表和详情', 220),
('badge:badge:write', '管理徽章', 'badge', 'write', '/badges/*', '创建、编辑、删除徽章', 221),
('badge:badge:publish', '发布徽章', 'badge', 'publish', '/badges/*/publish', '发布和下线徽章', 222),
('badge:dependency:read', '查看依赖', 'badge', 'read', '/dependencies/*', '查看徽章依赖关系', 230),
('badge:dependency:write', '管理依赖', 'badge', 'write', '/dependencies/*', '配置徽章依赖关系', 231),

-- 规则管理模块 (module: rule)
('rule:rule:read', '查看规则', 'rule', 'read', '/rules/*', '查看规则列表和详情', 300),
('rule:rule:write', '管理规则', 'rule', 'write', '/rules/*', '创建、编辑、删除规则', 301),
('rule:rule:publish', '发布规则', 'rule', 'publish', '/rules/*/publish', '发布和禁用规则', 302),
('rule:rule:test', '测试规则', 'rule', 'write', '/rules/*/test', '测试规则执行', 303),
('rule:template:read', '查看模板', 'rule', 'read', '/templates/*', '查看规则模板列表和详情', 310),

-- 发放管理模块 (module: grant)
('grant:grant:read', '查看发放', 'grant', 'read', '/grants/*', '查看发放记录和日志', 400),
('grant:grant:write', '发放徽章', 'grant', 'write', '/grants/*', '手动发放和批量发放徽章', 401),
('grant:revoke:read', '查看撤销', 'grant', 'read', '/revokes/*', '查看撤销记录', 410),
('grant:revoke:write', '撤销徽章', 'grant', 'write', '/revokes/*', '撤销用户徽章', 411),
('grant:task:read', '查看任务', 'grant', 'read', '/tasks/*', '查看批量任务', 420),
('grant:task:write', '管理任务', 'grant', 'write', '/tasks/*', '创建和取消批量任务', 421),

-- 权益管理模块 (module: benefit)
('benefit:benefit:read', '查看权益', 'benefit', 'read', '/benefits/*', '查看权益列表和详情', 500),
('benefit:benefit:write', '管理权益', 'benefit', 'write', '/benefits/*', '创建、编辑、删除权益', 501),
('benefit:grant:read', '查看权益发放', 'benefit', 'read', '/benefit-grants/*', '查看权益发放记录', 510),
('benefit:redemption:read', '查看兑换', 'benefit', 'read', '/redemption/*', '查看兑换规则和记录', 520),
('benefit:redemption:write', '管理兑换', 'benefit', 'write', '/redemption/*', '创建、编辑兑换规则', 521),

-- 用户视图模块 (module: user)
('user:view:read', '查看用户', 'user', 'read', '/users/*', '搜索和查看用户信息', 600),
('user:badge:read', '查看用户徽章', 'user', 'read', '/users/*/badges', '查看用户持有的徽章', 610),

-- 统计模块 (module: stats)
('stats:overview:read', '查看统计', 'stats', 'read', '/stats/*', '查看数据统计和看板', 700),

-- 日志模块 (module: log)
('log:operation:read', '查看日志', 'log', 'read', '/logs/*', '查看操作日志', 800)

ON CONFLICT (code) DO UPDATE SET
    name = EXCLUDED.name,
    module = EXCLUDED.module,
    action = EXCLUDED.action,
    resource_pattern = EXCLUDED.resource_pattern,
    description = EXCLUDED.description,
    sort_order = EXCLUDED.sort_order;

-- ============================================
-- 角色权限分配
-- ============================================

-- admin 角色：拥有所有权限
INSERT INTO role_permission (role_id, permission_id)
SELECT r.id, p.id
FROM role r, permission p
WHERE r.code = 'admin' AND p.enabled = TRUE
ON CONFLICT DO NOTHING;

-- operator 角色：拥有除系统管理外的所有权限
INSERT INTO role_permission (role_id, permission_id)
SELECT r.id, p.id
FROM role r, permission p
WHERE r.code = 'operator' AND p.module != 'system' AND p.enabled = TRUE
ON CONFLICT DO NOTHING;

-- viewer 角色：仅拥有 read 权限
INSERT INTO role_permission (role_id, permission_id)
SELECT r.id, p.id
FROM role r, permission p
WHERE r.code = 'viewer' AND p.action = 'read' AND p.enabled = TRUE
ON CONFLICT DO NOTHING;

-- ============================================
-- 创建默认管理员用户
-- 密码: admin123 (bcrypt 加密，cost=12)
-- 使用 cargo run -p badge-admin-service --example gen_password_hash 生成
-- ============================================
INSERT INTO admin_user (username, password_hash, display_name, status)
VALUES (
    'admin',
    '$2b$12$16C5oGtzpTqIacMpazO4Ee0RY1ynSbjWzAsYppO8H1UCrrSR1SSju', -- admin123
    '系统管理员',
    'ACTIVE'
)
ON CONFLICT (username) DO NOTHING;

-- 为默认管理员分配 admin 角色
INSERT INTO user_role (user_id, role_id)
SELECT u.id, r.id
FROM admin_user u, role r
WHERE u.username = 'admin' AND r.code = 'admin'
ON CONFLICT DO NOTHING;

-- ============================================
-- 创建测试用户（仅开发环境）
-- ============================================

-- operator 用户 (密码: operator123)
INSERT INTO admin_user (username, password_hash, display_name, status)
VALUES (
    'operator',
    '$2b$12$nKZgCHbvHx84mQF/BUOQSuuQZ5EA9hY/QKy9tMSlTQcboHI8WUr9a', -- operator123
    '运营管理员',
    'ACTIVE'
)
ON CONFLICT (username) DO NOTHING;

INSERT INTO user_role (user_id, role_id)
SELECT u.id, r.id
FROM admin_user u, role r
WHERE u.username = 'operator' AND r.code = 'operator'
ON CONFLICT DO NOTHING;

-- viewer 用户 (密码: viewer123)
INSERT INTO admin_user (username, password_hash, display_name, status)
VALUES (
    'viewer',
    '$2b$12$x97BU2ItIckH7iARLWQ8IuBENVKA70jzgK97fspbx.V4Xa/g.hXdO', -- viewer123
    '只读用户',
    'ACTIVE'
)
ON CONFLICT (username) DO NOTHING;

INSERT INTO user_role (user_id, role_id)
SELECT u.id, r.id
FROM admin_user u, role r
WHERE u.username = 'viewer' AND r.code = 'viewer'
ON CONFLICT DO NOTHING;

-- ============================================
-- 更新时间触发器
-- ============================================
CREATE OR REPLACE FUNCTION update_updated_at_column()
RETURNS TRIGGER AS $$
BEGIN
    NEW.updated_at = NOW();
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

-- admin_user 更新触发器
DROP TRIGGER IF EXISTS update_admin_user_updated_at ON admin_user;
CREATE TRIGGER update_admin_user_updated_at
    BEFORE UPDATE ON admin_user
    FOR EACH ROW
    EXECUTE FUNCTION update_updated_at_column();

-- role 更新触发器
DROP TRIGGER IF EXISTS update_role_updated_at ON role;
CREATE TRIGGER update_role_updated_at
    BEFORE UPDATE ON role
    FOR EACH ROW
    EXECUTE FUNCTION update_updated_at_column();
