-- 徽章依赖关系表
-- 支持徽章之间的前置条件、消耗关系和互斥关系

-- ==================== 徽章依赖 ====================

CREATE TABLE IF NOT EXISTS badge_dependencies (
    id BIGSERIAL PRIMARY KEY,
    badge_id BIGINT NOT NULL REFERENCES badges(id) ON DELETE CASCADE,
    depends_on_badge_id BIGINT NOT NULL REFERENCES badges(id) ON DELETE CASCADE,

    -- 依赖配置
    dependency_type VARCHAR(20) NOT NULL, -- prerequisite, consume, exclusive
    required_quantity INT NOT NULL DEFAULT 1,
    exclusive_group_id VARCHAR(100), -- 互斥组ID，同组徽章只能持有一个

    -- 自动触发配置
    auto_trigger BOOLEAN NOT NULL DEFAULT FALSE,
    priority INT NOT NULL DEFAULT 0, -- 评估优先级，数值越小越先评估

    -- 依赖分组（同组是 AND 关系，不同组是 OR 关系）
    dependency_group_id VARCHAR(100) NOT NULL,

    enabled BOOLEAN NOT NULL DEFAULT TRUE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),

    -- 防止自引用
    CONSTRAINT badge_dependencies_no_self_ref CHECK (badge_id != depends_on_badge_id),
    -- 防止重复依赖（同一组内的相同依赖关系）
    CONSTRAINT badge_dependencies_unique UNIQUE (badge_id, depends_on_badge_id, dependency_group_id),
    -- exclusive 类型必须指定互斥组，非 exclusive 类型不应有互斥组
    CONSTRAINT badge_dependencies_exclusive_group_check CHECK (
        (dependency_type = 'exclusive' AND exclusive_group_id IS NOT NULL)
        OR (dependency_type != 'exclusive' AND exclusive_group_id IS NULL)
    )
);

COMMENT ON TABLE badge_dependencies IS '徽章依赖关系，定义徽章之间的前置条件、消耗和互斥关系';
COMMENT ON COLUMN badge_dependencies.badge_id IS '目标徽章ID，获得此徽章需要满足依赖条件';
COMMENT ON COLUMN badge_dependencies.depends_on_badge_id IS '依赖的徽章ID';
COMMENT ON COLUMN badge_dependencies.dependency_type IS '依赖类型：prerequisite-前置条件（需持有），consume-消耗（发放时扣减），exclusive-互斥（不能同时持有）';
COMMENT ON COLUMN badge_dependencies.required_quantity IS '需要的数量，默认为1';
COMMENT ON COLUMN badge_dependencies.exclusive_group_id IS '互斥组ID，同组内的徽章互斥，用户只能持有其中一个';
COMMENT ON COLUMN badge_dependencies.auto_trigger IS '是否自动触发，当依赖徽章发放时自动检查是否满足条件';
COMMENT ON COLUMN badge_dependencies.priority IS '评估优先级，数值越小越先评估，用于确定多个依赖的检查顺序';
COMMENT ON COLUMN badge_dependencies.dependency_group_id IS '依赖组ID，同组条件是 AND 关系，不同组是 OR 关系';
COMMENT ON COLUMN badge_dependencies.enabled IS '是否启用此依赖规则';

-- 查询某徽章的所有依赖条件
CREATE INDEX IF NOT EXISTS idx_badge_deps_badge ON badge_dependencies(badge_id);

-- 查询依赖某徽章的所有徽章（反向查询）
CREATE INDEX IF NOT EXISTS idx_badge_deps_depends ON badge_dependencies(depends_on_badge_id);

-- 自动触发场景：当某徽章发放时，快速找到需要检查的目标徽章
CREATE INDEX IF NOT EXISTS idx_badge_deps_auto ON badge_dependencies(depends_on_badge_id, auto_trigger)
    WHERE auto_trigger = TRUE AND enabled = TRUE;

-- 互斥组查询
CREATE INDEX IF NOT EXISTS idx_badge_deps_exclusive_group ON badge_dependencies(exclusive_group_id)
    WHERE exclusive_group_id IS NOT NULL;

-- 按依赖组查询
CREATE INDEX IF NOT EXISTS idx_badge_deps_group ON badge_dependencies(dependency_group_id);

-- 按依赖类型查询
CREATE INDEX IF NOT EXISTS idx_badge_deps_type ON badge_dependencies(dependency_type);

-- ==================== 触发器 ====================

DROP TRIGGER IF EXISTS update_badge_dependencies_updated_at ON badge_dependencies;
CREATE TRIGGER update_badge_dependencies_updated_at
    BEFORE UPDATE ON badge_dependencies
    FOR EACH ROW EXECUTE FUNCTION update_updated_at_column();
