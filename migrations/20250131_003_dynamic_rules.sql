-- 动态规则加载支持
-- 包含事件类型配置表和 badge_rules 表扩展

-- ==================== 事件类型配置 ====================

-- 事件类型表：定义系统支持的所有事件类型及其所属服务组
CREATE TABLE event_types (
    id BIGSERIAL PRIMARY KEY,
    code VARCHAR(50) NOT NULL UNIQUE,
    name VARCHAR(100) NOT NULL,
    service_group VARCHAR(50) NOT NULL,  -- 'transaction' 或 'engagement'
    description TEXT,
    enabled BOOLEAN NOT NULL DEFAULT TRUE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

COMMENT ON TABLE event_types IS '事件类型配置表，用于动态加载规则时确定事件归属的服务组';
COMMENT ON COLUMN event_types.code IS '事件类型唯一编码，如 purchase、checkin 等';
COMMENT ON COLUMN event_types.name IS '事件类型显示名称';
COMMENT ON COLUMN event_types.service_group IS '服务分组：transaction-交易类事件由 event-transaction-service 处理，engagement-互动类事件由 event-engagement-service 处理';
COMMENT ON COLUMN event_types.enabled IS '是否启用此事件类型';

CREATE INDEX idx_event_types_service_group ON event_types(service_group);
CREATE INDEX idx_event_types_enabled ON event_types(enabled);

-- 预置事件类型数据
INSERT INTO event_types (code, name, service_group, description) VALUES
    -- 交易类事件
    ('purchase', '购买', 'transaction', '用户完成购买行为'),
    ('refund', '退款', 'transaction', '用户申请退款'),
    ('order_cancel', '订单取消', 'transaction', '用户取消订单'),
    -- 互动类事件
    ('checkin', '签到', 'engagement', '用户每日签到'),
    ('page_view', '页面浏览', 'engagement', '用户浏览指定页面'),
    ('share', '分享', 'engagement', '用户分享内容'),
    ('profile_update', '资料更新', 'engagement', '用户更新个人资料'),
    ('review', '评价', 'engagement', '用户提交评价');

-- 更新 updated_at 触发器
CREATE TRIGGER update_event_types_updated_at
    BEFORE UPDATE ON event_types
    FOR EACH ROW EXECUTE FUNCTION update_updated_at_column();

-- ==================== 扩展 badge_rules 表 ====================

-- 关联事件类型，用于规则路由
ALTER TABLE badge_rules ADD COLUMN event_type VARCHAR(50);

-- 规则唯一编码，便于管理和日志追踪
ALTER TABLE badge_rules ADD COLUMN rule_code VARCHAR(100);

-- 全局配额控制：限制该规则可发放的徽章总数
ALTER TABLE badge_rules ADD COLUMN global_quota INT;

-- 全局已发放计数：与 global_quota 配合实现总量控制
ALTER TABLE badge_rules ADD COLUMN global_granted INT NOT NULL DEFAULT 0;

COMMENT ON COLUMN badge_rules.event_type IS '关联的事件类型编码，决定该规则由哪个事件服务处理';
COMMENT ON COLUMN badge_rules.rule_code IS '规则唯一编码，用于日志追踪和管理后台展示';
COMMENT ON COLUMN badge_rules.global_quota IS '全局配额，限制该规则可发放的徽章总数，NULL 表示不限制';
COMMENT ON COLUMN badge_rules.global_granted IS '已发放数量，用于配额校验';

-- 外键约束：确保 event_type 引用有效的事件类型
ALTER TABLE badge_rules
    ADD CONSTRAINT fk_badge_rules_event_type
    FOREIGN KEY (event_type) REFERENCES event_types(code);

-- 唯一约束：rule_code 必须唯一
ALTER TABLE badge_rules
    ADD CONSTRAINT uq_badge_rules_rule_code
    UNIQUE (rule_code);

-- 复合索引：按事件类型和启用状态查询，用于规则加载
CREATE INDEX idx_badge_rules_event_type_enabled
    ON badge_rules(event_type, enabled);
