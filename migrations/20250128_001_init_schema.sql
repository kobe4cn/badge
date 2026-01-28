-- 徽章系统初始化 schema
-- 包含核心表结构：徽章分类、系列、徽章、规则、用户徽章、账本等

-- 启用必要的扩展
CREATE EXTENSION IF NOT EXISTS "uuid-ossp";
CREATE EXTENSION IF NOT EXISTS "pg_trgm";

-- ==================== 徽章结构 ====================

-- 一级分类
CREATE TABLE badge_category (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    name VARCHAR(100) NOT NULL,
    description TEXT,
    sort_order INT NOT NULL DEFAULT 0,
    status VARCHAR(20) NOT NULL DEFAULT 'active', -- active, inactive
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),

    CONSTRAINT badge_category_name_unique UNIQUE (name)
);

COMMENT ON TABLE badge_category IS '徽章一级分类，用于分类统计，如"交易徽章"、"互动徽章"';
COMMENT ON COLUMN badge_category.status IS '状态：active-启用，inactive-停用';

-- 二级系列
CREATE TABLE badge_series (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    category_id UUID NOT NULL REFERENCES badge_category(id),
    name VARCHAR(100) NOT NULL,
    description TEXT,
    sort_order INT NOT NULL DEFAULT 0,
    status VARCHAR(20) NOT NULL DEFAULT 'active',
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),

    CONSTRAINT badge_series_name_unique UNIQUE (category_id, name)
);

COMMENT ON TABLE badge_series IS '徽章二级系列，用于分组展示，如"2024春节系列"';

CREATE INDEX idx_badge_series_category ON badge_series(category_id);

-- 徽章定义
CREATE TABLE badge (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    series_id UUID NOT NULL REFERENCES badge_series(id),
    code VARCHAR(50) NOT NULL UNIQUE, -- 业务唯一标识
    name VARCHAR(100) NOT NULL,
    description TEXT,
    badge_type VARCHAR(50) NOT NULL, -- transaction, engagement, identity, seasonal

    -- 素材
    icon_url TEXT,
    icon_3d_url TEXT,

    -- 获取配置
    acquire_time_start TIMESTAMPTZ,
    acquire_time_end TIMESTAMPTZ,
    max_acquire_count INT, -- NULL 表示无限制

    -- 持有有效期配置
    validity_type VARCHAR(20) NOT NULL DEFAULT 'permanent', -- fixed, flexible, permanent
    validity_fixed_date TIMESTAMPTZ, -- validity_type = fixed 时使用，固定到期日
    validity_days INT, -- validity_type = flexible 时使用，获取后N天过期

    -- 发放对象
    grant_target VARCHAR(20) NOT NULL DEFAULT 'account', -- account, actual_user

    status VARCHAR(20) NOT NULL DEFAULT 'draft', -- draft, active, inactive
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

COMMENT ON TABLE badge IS '徽章定义，实际发放给用户的徽章实体';
COMMENT ON COLUMN badge.code IS '业务唯一标识，用于外部系统引用';
COMMENT ON COLUMN badge.badge_type IS '徽章类型：transaction-交易，engagement-互动，identity-身份，seasonal-季节性';
COMMENT ON COLUMN badge.validity_type IS '有效期类型：fixed-固定日期，flexible-相对天数，permanent-永久';
COMMENT ON COLUMN badge.status IS '状态：draft-草稿，active-已上线，inactive-已下线';

CREATE INDEX idx_badge_series ON badge(series_id);
CREATE INDEX idx_badge_type ON badge(badge_type);
CREATE INDEX idx_badge_status ON badge(status);
CREATE INDEX idx_badge_code ON badge(code);

-- ==================== 规则配置 ====================

-- 徽章获取规则
CREATE TABLE badge_rule (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    badge_id UUID NOT NULL REFERENCES badge(id),
    name VARCHAR(100) NOT NULL,
    description TEXT,
    rule_json JSONB NOT NULL, -- 规则 JSON，遵循统一规则引擎格式
    priority INT NOT NULL DEFAULT 0, -- 优先级，数值越大越优先
    status VARCHAR(20) NOT NULL DEFAULT 'active',
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

COMMENT ON TABLE badge_rule IS '徽章获取规则，定义触发徽章发放的条件';
COMMENT ON COLUMN badge_rule.rule_json IS '规则JSON，遵循统一规则引擎的JSON Schema格式';
COMMENT ON COLUMN badge_rule.priority IS '优先级，当多条规则匹配时，数值越大越优先';

CREATE INDEX idx_badge_rule_badge ON badge_rule(badge_id);
CREATE INDEX idx_badge_rule_status ON badge_rule(status);
CREATE INDEX idx_badge_rule_json ON badge_rule USING GIN(rule_json);

-- ==================== 用户徽章 ====================

-- 用户徽章持有
CREATE TABLE user_badge (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    user_id VARCHAR(100) NOT NULL, -- SWID
    badge_id UUID NOT NULL REFERENCES badge(id),
    quantity INT NOT NULL DEFAULT 1,
    status VARCHAR(20) NOT NULL DEFAULT 'active', -- active, expired, revoked, redeemed
    acquired_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    expires_at TIMESTAMPTZ,

    -- 发放来源
    source_type VARCHAR(20) NOT NULL, -- event, scheduled, manual
    source_ref VARCHAR(200), -- 来源引用（事件ID、任务ID等）

    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

COMMENT ON TABLE user_badge IS '用户徽章持有记录';
COMMENT ON COLUMN user_badge.user_id IS 'SWID，用户唯一标识';
COMMENT ON COLUMN user_badge.status IS '状态：active-有效，expired-已过期，revoked-已取消，redeemed-已兑换';
COMMENT ON COLUMN user_badge.source_type IS '发放来源：event-事件触发，scheduled-定时任务，manual-手动发放';

CREATE INDEX idx_user_badge_user ON user_badge(user_id);
CREATE INDEX idx_user_badge_badge ON user_badge(badge_id);
CREATE INDEX idx_user_badge_status ON user_badge(status);
CREATE INDEX idx_user_badge_user_status ON user_badge(user_id, status);
CREATE INDEX idx_user_badge_expires ON user_badge(expires_at) WHERE expires_at IS NOT NULL;

-- 徽章账本（流水）- 复式记账设计
CREATE TABLE badge_ledger (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    user_id VARCHAR(100) NOT NULL,
    badge_id UUID NOT NULL REFERENCES badge(id),
    user_badge_id UUID REFERENCES user_badge(id),

    change_type VARCHAR(20) NOT NULL, -- acquire, expire, cancel, redeem_out, redeem_fail
    quantity INT NOT NULL, -- 正数增加，负数减少
    balance_after INT NOT NULL, -- 变更后余额

    -- 关联来源
    ref_type VARCHAR(20) NOT NULL, -- event, scheduled, manual, redemption, system
    ref_id VARCHAR(200),

    reason TEXT,
    operator VARCHAR(100), -- 操作人（手动操作时）

    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

COMMENT ON TABLE badge_ledger IS '徽章账本流水，采用复式记账思想记录每一笔徽章变动';
COMMENT ON COLUMN badge_ledger.change_type IS '变更类型：acquire-获取(+)，expire-过期(-)，cancel-取消(-)，redeem_out-兑换消耗(-)，redeem_fail-兑换失败回滚(+)';
COMMENT ON COLUMN badge_ledger.quantity IS '变更数量，正数表示增加，负数表示减少';
COMMENT ON COLUMN badge_ledger.balance_after IS '变更后的徽章余额，用于快速查询和对账';
COMMENT ON COLUMN badge_ledger.ref_type IS '关联来源类型：event-事件触发，scheduled-定时任务，manual-手动发放，redemption-兑换订单，system-系统操作';

CREATE INDEX idx_badge_ledger_user ON badge_ledger(user_id);
CREATE INDEX idx_badge_ledger_badge ON badge_ledger(badge_id);
CREATE INDEX idx_badge_ledger_user_badge ON badge_ledger(user_badge_id);
CREATE INDEX idx_badge_ledger_ref ON badge_ledger(ref_type, ref_id);
CREATE INDEX idx_badge_ledger_time ON badge_ledger(created_at);
CREATE INDEX idx_badge_ledger_change_type ON badge_ledger(change_type);

-- ==================== 兑换相关 ====================

-- 权益定义
CREATE TABLE benefit (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    code VARCHAR(50) NOT NULL UNIQUE,
    name VARCHAR(100) NOT NULL,
    description TEXT,
    benefit_type VARCHAR(50) NOT NULL, -- digital_asset, coupon, reservation

    -- 外部系统关联
    external_id VARCHAR(200),
    external_system VARCHAR(50),

    -- 库存管理
    total_stock INT,
    remaining_stock INT,

    status VARCHAR(20) NOT NULL DEFAULT 'active',
    config JSONB, -- 权益特定配置

    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

COMMENT ON TABLE benefit IS '权益定义，徽章可兑换的权益';
COMMENT ON COLUMN benefit.benefit_type IS '权益类型：digital_asset-数字资产，coupon-优惠券，reservation-预约资格';
COMMENT ON COLUMN benefit.external_id IS '外部系统中的权益ID';
COMMENT ON COLUMN benefit.external_system IS '外部系统标识，如coupon_service、digital_asset_center等';

CREATE INDEX idx_benefit_type ON benefit(benefit_type);
CREATE INDEX idx_benefit_status ON benefit(status);

-- 兑换规则
CREATE TABLE badge_redemption_rule (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    name VARCHAR(100) NOT NULL,
    description TEXT,
    benefit_id UUID NOT NULL REFERENCES benefit(id),

    -- 所需徽章配置
    required_badges JSONB NOT NULL, -- [{"badge_id": "uuid", "quantity": 1}]

    -- 兑换时间限制
    redeem_time_start TIMESTAMPTZ,
    redeem_time_end TIMESTAMPTZ,
    redeem_after_acquire_days INT, -- 获取徽章后N天内可兑换

    -- 兑换频次限制
    frequency_type VARCHAR(20), -- daily, weekly, monthly, yearly, account
    frequency_limit INT,

    -- 自动兑换配置
    auto_redeem BOOLEAN NOT NULL DEFAULT FALSE,

    status VARCHAR(20) NOT NULL DEFAULT 'active',
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

COMMENT ON TABLE badge_redemption_rule IS '徽章兑换规则，定义徽章如何兑换为权益';
COMMENT ON COLUMN badge_redemption_rule.required_badges IS '所需徽章配置，JSON数组格式：[{"badge_id": "uuid", "quantity": 1}]';
COMMENT ON COLUMN badge_redemption_rule.frequency_type IS '频次限制类型：daily-每日，weekly-每周，monthly-每月，yearly-每年，account-账号维度';
COMMENT ON COLUMN badge_redemption_rule.auto_redeem IS '是否自动兑换，满足条件时自动触发兑换';

CREATE INDEX idx_redemption_rule_benefit ON badge_redemption_rule(benefit_id);
CREATE INDEX idx_redemption_rule_status ON badge_redemption_rule(status);
CREATE INDEX idx_redemption_rule_badges ON badge_redemption_rule USING GIN(required_badges);

-- 兑换订单
CREATE TABLE redemption_order (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    user_id VARCHAR(100) NOT NULL,
    redemption_rule_id UUID NOT NULL REFERENCES badge_redemption_rule(id),
    benefit_id UUID NOT NULL REFERENCES benefit(id),

    status VARCHAR(20) NOT NULL DEFAULT 'pending', -- pending, completed, failed, cancelled

    -- 权益发放结果
    benefit_grant_ref VARCHAR(200), -- 外部系统权益发放ID
    benefit_grant_at TIMESTAMPTZ,

    failure_reason TEXT,

    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

COMMENT ON TABLE redemption_order IS '兑换订单，记录用户的徽章兑换操作';
COMMENT ON COLUMN redemption_order.status IS '订单状态：pending-处理中，completed-已完成，failed-失败，cancelled-已取消';
COMMENT ON COLUMN redemption_order.benefit_grant_ref IS '外部系统权益发放的引用ID，用于追踪和对账';

CREATE INDEX idx_redemption_order_user ON redemption_order(user_id);
CREATE INDEX idx_redemption_order_status ON redemption_order(status);
CREATE INDEX idx_redemption_order_rule ON redemption_order(redemption_rule_id);
CREATE INDEX idx_redemption_order_benefit ON redemption_order(benefit_id);
CREATE INDEX idx_redemption_order_time ON redemption_order(created_at);

-- 兑换明细
CREATE TABLE redemption_detail (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    order_id UUID NOT NULL REFERENCES redemption_order(id),
    user_badge_id UUID NOT NULL REFERENCES user_badge(id),
    badge_id UUID NOT NULL REFERENCES badge(id),
    quantity INT NOT NULL,

    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

COMMENT ON TABLE redemption_detail IS '兑换明细，记录每次兑换消耗的具体徽章';
COMMENT ON COLUMN redemption_detail.quantity IS '消耗的徽章数量';

CREATE INDEX idx_redemption_detail_order ON redemption_detail(order_id);
CREATE INDEX idx_redemption_detail_user_badge ON redemption_detail(user_badge_id);
CREATE INDEX idx_redemption_detail_badge ON redemption_detail(badge_id);

-- ==================== 通知相关 ====================

-- 通知配置
CREATE TABLE notification_config (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    badge_id UUID REFERENCES badge(id),
    benefit_id UUID REFERENCES benefit(id),

    trigger_type VARCHAR(20) NOT NULL, -- grant, revoke, expire, expire_remind, redeem
    channels JSONB NOT NULL, -- ["app_push", "sms", "wechat", "email", "in_app"]
    template_id VARCHAR(100),
    advance_days INT, -- 提前通知天数（过期提醒场景）

    retry_count INT NOT NULL DEFAULT 3,
    retry_interval_seconds INT NOT NULL DEFAULT 60,

    status VARCHAR(20) NOT NULL DEFAULT 'active',
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

COMMENT ON TABLE notification_config IS '通知配置，定义徽章/权益相关事件的通知规则';
COMMENT ON COLUMN notification_config.trigger_type IS '触发类型：grant-发放，revoke-取消，expire-过期，expire_remind-过期提醒，redeem-兑换';
COMMENT ON COLUMN notification_config.channels IS '通知渠道：["app_push", "sms", "wechat", "email", "in_app"]';
COMMENT ON COLUMN notification_config.advance_days IS '提前通知天数，用于过期提醒场景';

CREATE INDEX idx_notification_config_badge ON notification_config(badge_id);
CREATE INDEX idx_notification_config_benefit ON notification_config(benefit_id);
CREATE INDEX idx_notification_config_trigger ON notification_config(trigger_type);

-- 通知任务
CREATE TABLE notification_task (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    user_id VARCHAR(100) NOT NULL,

    trigger_type VARCHAR(20) NOT NULL,
    channels JSONB NOT NULL,
    template_id VARCHAR(100),
    template_params JSONB,

    status VARCHAR(20) NOT NULL DEFAULT 'pending', -- pending, processing, completed, failed
    retry_count INT NOT NULL DEFAULT 0,
    max_retries INT NOT NULL DEFAULT 3,

    last_error TEXT,
    completed_at TIMESTAMPTZ,

    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

COMMENT ON TABLE notification_task IS '通知任务，待发送的通知队列';
COMMENT ON COLUMN notification_task.status IS '任务状态：pending-待处理，processing-处理中，completed-已完成，failed-失败';
COMMENT ON COLUMN notification_task.template_params IS '模板参数，JSON格式，用于填充通知模板';

CREATE INDEX idx_notification_task_status ON notification_task(status);
CREATE INDEX idx_notification_task_user ON notification_task(user_id);
CREATE INDEX idx_notification_task_created ON notification_task(created_at);

-- ==================== 系统管理 ====================

-- 操作日志
CREATE TABLE operation_log (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    operator_id VARCHAR(100) NOT NULL,
    operator_name VARCHAR(100),

    module VARCHAR(50) NOT NULL,
    action VARCHAR(50) NOT NULL,
    target_type VARCHAR(50),
    target_id VARCHAR(200),

    before_data JSONB,
    after_data JSONB,

    ip_address VARCHAR(50),
    user_agent TEXT,

    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

COMMENT ON TABLE operation_log IS '操作日志，记录管理后台的所有操作';
COMMENT ON COLUMN operation_log.module IS '模块：badge, rule, grant, revoke, benefit, redemption等';
COMMENT ON COLUMN operation_log.action IS '操作：create, update, delete, publish, activate, deactivate等';
COMMENT ON COLUMN operation_log.before_data IS '操作前数据快照';
COMMENT ON COLUMN operation_log.after_data IS '操作后数据快照';

CREATE INDEX idx_operation_log_operator ON operation_log(operator_id);
CREATE INDEX idx_operation_log_module ON operation_log(module);
CREATE INDEX idx_operation_log_action ON operation_log(action);
CREATE INDEX idx_operation_log_target ON operation_log(target_type, target_id);
CREATE INDEX idx_operation_log_time ON operation_log(created_at);

-- 批量任务
CREATE TABLE batch_task (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    task_type VARCHAR(50) NOT NULL, -- batch_grant, batch_revoke, data_export

    file_url TEXT, -- 上传的文件地址
    total_count INT NOT NULL DEFAULT 0,
    success_count INT NOT NULL DEFAULT 0,
    failure_count INT NOT NULL DEFAULT 0,

    status VARCHAR(20) NOT NULL DEFAULT 'pending', -- pending, processing, completed, failed
    progress INT NOT NULL DEFAULT 0, -- 0-100

    result_file_url TEXT, -- 结果文件地址
    error_message TEXT,

    created_by VARCHAR(100) NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

COMMENT ON TABLE batch_task IS '批量任务，用于大批量发放、取消、导出等操作';
COMMENT ON COLUMN batch_task.task_type IS '任务类型：batch_grant-批量发放，batch_revoke-批量取消，data_export-数据导出';
COMMENT ON COLUMN batch_task.file_url IS '上传的源文件地址（OSS）';
COMMENT ON COLUMN batch_task.result_file_url IS '处理结果文件地址（OSS）';
COMMENT ON COLUMN batch_task.progress IS '处理进度，0-100';

CREATE INDEX idx_batch_task_status ON batch_task(status);
CREATE INDEX idx_batch_task_type ON batch_task(task_type);
CREATE INDEX idx_batch_task_creator ON batch_task(created_by);
CREATE INDEX idx_batch_task_time ON batch_task(created_at);

-- ==================== 触发器 ====================

-- 更新 updated_at 触发器函数
CREATE OR REPLACE FUNCTION update_updated_at_column()
RETURNS TRIGGER AS $$
BEGIN
    NEW.updated_at = NOW();
    RETURN NEW;
END;
$$ language 'plpgsql';

-- 为所有需要自动更新时间戳的表添加触发器
CREATE TRIGGER update_badge_category_updated_at
    BEFORE UPDATE ON badge_category
    FOR EACH ROW EXECUTE FUNCTION update_updated_at_column();

CREATE TRIGGER update_badge_series_updated_at
    BEFORE UPDATE ON badge_series
    FOR EACH ROW EXECUTE FUNCTION update_updated_at_column();

CREATE TRIGGER update_badge_updated_at
    BEFORE UPDATE ON badge
    FOR EACH ROW EXECUTE FUNCTION update_updated_at_column();

CREATE TRIGGER update_badge_rule_updated_at
    BEFORE UPDATE ON badge_rule
    FOR EACH ROW EXECUTE FUNCTION update_updated_at_column();

CREATE TRIGGER update_user_badge_updated_at
    BEFORE UPDATE ON user_badge
    FOR EACH ROW EXECUTE FUNCTION update_updated_at_column();

CREATE TRIGGER update_benefit_updated_at
    BEFORE UPDATE ON benefit
    FOR EACH ROW EXECUTE FUNCTION update_updated_at_column();

CREATE TRIGGER update_badge_redemption_rule_updated_at
    BEFORE UPDATE ON badge_redemption_rule
    FOR EACH ROW EXECUTE FUNCTION update_updated_at_column();

CREATE TRIGGER update_redemption_order_updated_at
    BEFORE UPDATE ON redemption_order
    FOR EACH ROW EXECUTE FUNCTION update_updated_at_column();

CREATE TRIGGER update_notification_config_updated_at
    BEFORE UPDATE ON notification_config
    FOR EACH ROW EXECUTE FUNCTION update_updated_at_column();

CREATE TRIGGER update_notification_task_updated_at
    BEFORE UPDATE ON notification_task
    FOR EACH ROW EXECUTE FUNCTION update_updated_at_column();

CREATE TRIGGER update_batch_task_updated_at
    BEFORE UPDATE ON batch_task
    FOR EACH ROW EXECUTE FUNCTION update_updated_at_column();
