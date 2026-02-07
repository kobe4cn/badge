-- 徽章系统初始化 schema
-- 包含核心表结构：徽章分类、系列、徽章、规则、用户徽章、账本等

-- 启用必要的扩展
CREATE EXTENSION IF NOT EXISTS "pg_trgm";

-- ==================== 徽章结构 ====================

-- 一级分类
CREATE TABLE IF NOT EXISTS badge_categories (
    id BIGSERIAL PRIMARY KEY,
    name VARCHAR(100) NOT NULL,
    icon_url TEXT,
    sort_order INT NOT NULL DEFAULT 0,
    status VARCHAR(20) NOT NULL DEFAULT 'active', -- active, inactive
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),

    CONSTRAINT badge_categories_name_unique UNIQUE (name)
);

COMMENT ON TABLE badge_categories IS '徽章一级分类，用于分类统计，如"交易徽章"、"互动徽章"';
COMMENT ON COLUMN badge_categories.status IS '状态：active-启用，inactive-停用';

-- 二级系列
CREATE TABLE IF NOT EXISTS badge_series (
    id BIGSERIAL PRIMARY KEY,
    category_id BIGINT NOT NULL REFERENCES badge_categories(id),
    name VARCHAR(100) NOT NULL,
    description TEXT,
    cover_url TEXT,
    sort_order INT NOT NULL DEFAULT 0,
    status VARCHAR(20) NOT NULL DEFAULT 'active',
    start_time TIMESTAMPTZ,
    end_time TIMESTAMPTZ,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),

    CONSTRAINT badge_series_name_unique UNIQUE (category_id, name)
);

COMMENT ON TABLE badge_series IS '徽章二级系列，用于分组展示，如"2024春节系列"';

CREATE INDEX IF NOT EXISTS idx_badge_series_category ON badge_series(category_id);

-- 徽章定义
CREATE TABLE IF NOT EXISTS badges (
    id BIGSERIAL PRIMARY KEY,
    series_id BIGINT NOT NULL REFERENCES badge_series(id),
    badge_type VARCHAR(50) NOT NULL, -- normal, limited, achievement, event
    name VARCHAR(100) NOT NULL,
    description TEXT,
    obtain_description TEXT, -- 获取条件描述，展示给用户

    -- 素材配置 (JSONB)
    assets JSONB NOT NULL DEFAULT '{"iconUrl": ""}', -- {"iconUrl", "imageUrl", "animationUrl", "disabledIconUrl"}

    -- 有效期配置 (JSONB)
    validity_config JSONB NOT NULL DEFAULT '{"validityType": "PERMANENT"}', -- {"validityType", "fixedDate", "relativeDays"}

    -- 库存控制
    max_supply BIGINT, -- NULL 表示不限量
    issued_count BIGINT NOT NULL DEFAULT 0, -- 已发放数量

    -- 排序
    sort_order INT NOT NULL DEFAULT 0,

    status VARCHAR(20) NOT NULL DEFAULT 'draft', -- draft, active, inactive, archived
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

COMMENT ON TABLE badges IS '徽章定义，实际发放给用户的徽章实体';
COMMENT ON COLUMN badges.badge_type IS '徽章类型：normal-普通，limited-限定，achievement-成就，event-活动';
COMMENT ON COLUMN badges.obtain_description IS '获取条件描述，向用户展示如何获得此徽章';
COMMENT ON COLUMN badges.assets IS '素材配置JSON：iconUrl-图标，imageUrl-大图，animationUrl-动效，disabledIconUrl-灰态图标';
COMMENT ON COLUMN badges.validity_config IS '有效期配置JSON：validityType-类型(PERMANENT/FIXED_DATE/RELATIVE_DAYS)，fixedDate-固定日期，relativeDays-相对天数';
COMMENT ON COLUMN badges.max_supply IS '最大发放总量，NULL表示不限量';
COMMENT ON COLUMN badges.issued_count IS '已发放数量，用于库存控制';
COMMENT ON COLUMN badges.status IS '状态：draft-草稿，active-已上线，inactive-已下线，archived-已归档';

CREATE INDEX IF NOT EXISTS idx_badges_series ON badges(series_id);
CREATE INDEX IF NOT EXISTS idx_badges_type ON badges(badge_type);
CREATE INDEX IF NOT EXISTS idx_badges_status ON badges(status);

-- ==================== 规则配置 ====================

-- 徽章获取规则
CREATE TABLE IF NOT EXISTS badge_rules (
    id BIGSERIAL PRIMARY KEY,
    badge_id BIGINT NOT NULL REFERENCES badges(id),
    rule_json JSONB NOT NULL, -- 规则 JSON，遵循统一规则引擎格式

    -- 时间限制
    start_time TIMESTAMPTZ, -- 规则生效开始时间
    end_time TIMESTAMPTZ, -- 规则生效结束时间

    -- 发放限制
    max_count_per_user INT, -- 每用户最大获取次数，NULL 表示不限制

    enabled BOOLEAN NOT NULL DEFAULT TRUE, -- 是否启用

    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

COMMENT ON TABLE badge_rules IS '徽章获取规则，定义触发徽章发放的条件';
COMMENT ON COLUMN badge_rules.rule_json IS '规则JSON，遵循统一规则引擎的JSON Schema格式';
COMMENT ON COLUMN badge_rules.start_time IS '规则生效开始时间，NULL表示立即生效';
COMMENT ON COLUMN badge_rules.end_time IS '规则生效结束时间，NULL表示永久有效';
COMMENT ON COLUMN badge_rules.max_count_per_user IS '每用户最大获取次数，NULL表示不限制';
COMMENT ON COLUMN badge_rules.enabled IS '是否启用此规则';

CREATE INDEX IF NOT EXISTS idx_badge_rules_badge ON badge_rules(badge_id);
CREATE INDEX IF NOT EXISTS idx_badge_rules_enabled ON badge_rules(enabled);
CREATE INDEX IF NOT EXISTS idx_badge_rules_json ON badge_rules USING GIN(rule_json);

-- ==================== 用户徽章 ====================

-- 用户徽章持有
CREATE TABLE IF NOT EXISTS user_badges (
    id BIGSERIAL PRIMARY KEY,
    user_id VARCHAR(100) NOT NULL, -- SWID
    badge_id BIGINT NOT NULL REFERENCES badges(id),
    quantity INT NOT NULL DEFAULT 1,
    status VARCHAR(20) NOT NULL DEFAULT 'active', -- active, expired, revoked, redeemed
    first_acquired_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    expires_at TIMESTAMPTZ,

    -- 发放来源
    source_type VARCHAR(20) NOT NULL, -- event, scheduled, manual
    source_ref VARCHAR(200), -- 来源引用（事件ID、任务ID等）

    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

COMMENT ON TABLE user_badges IS '用户徽章持有记录';
COMMENT ON COLUMN user_badges.user_id IS 'SWID，用户唯一标识';
COMMENT ON COLUMN user_badges.status IS '状态：active-有效，expired-已过期，revoked-已取消，redeemed-已兑换';
COMMENT ON COLUMN user_badges.source_type IS '发放来源：event-事件触发，scheduled-定时任务，manual-手动发放';

CREATE INDEX IF NOT EXISTS idx_user_badges_user ON user_badges(user_id);
CREATE INDEX IF NOT EXISTS idx_user_badges_badge ON user_badges(badge_id);
CREATE INDEX IF NOT EXISTS idx_user_badges_status ON user_badges(status);
CREATE INDEX IF NOT EXISTS idx_user_badges_user_status ON user_badges(user_id, status);
CREATE INDEX IF NOT EXISTS idx_user_badges_expires ON user_badges(expires_at) WHERE expires_at IS NOT NULL;
CREATE UNIQUE INDEX IF NOT EXISTS idx_user_badges_user_badge ON user_badges(user_id, badge_id);

-- 徽章账本（流水）- 复式记账设计
CREATE TABLE IF NOT EXISTS badge_ledger (
    id BIGSERIAL PRIMARY KEY,
    user_id VARCHAR(100) NOT NULL,
    badge_id BIGINT NOT NULL REFERENCES badges(id),
    user_badge_id BIGINT REFERENCES user_badges(id),

    change_type VARCHAR(20) NOT NULL, -- acquire, expire, cancel, redeem_out, redeem_fail
    source_type VARCHAR(20) NOT NULL, -- event, scheduled, manual, redemption, system
    quantity INT NOT NULL, -- 正数增加，负数减少
    balance_after INT NOT NULL, -- 变更后余额

    -- 关联来源
    ref_id VARCHAR(200),

    remark TEXT,
    operator VARCHAR(100), -- 操作人（手动操作时）

    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

COMMENT ON TABLE badge_ledger IS '徽章账本流水，采用复式记账思想记录每一笔徽章变动';
COMMENT ON COLUMN badge_ledger.change_type IS '变更类型：acquire-获取(+)，expire-过期(-)，cancel-取消(-)，redeem_out-兑换消耗(-)，redeem_fail-兑换失败回滚(+)';
COMMENT ON COLUMN badge_ledger.quantity IS '变更数量，正数表示增加，负数表示减少';
COMMENT ON COLUMN badge_ledger.balance_after IS '变更后的徽章余额，用于快速查询和对账';
COMMENT ON COLUMN badge_ledger.source_type IS '来源类型：event-事件触发，scheduled-定时任务，manual-手动发放，redemption-兑换订单，system-系统操作';

CREATE INDEX IF NOT EXISTS idx_badge_ledger_user ON badge_ledger(user_id);
CREATE INDEX IF NOT EXISTS idx_badge_ledger_badge ON badge_ledger(badge_id);
CREATE INDEX IF NOT EXISTS idx_badge_ledger_user_badge ON badge_ledger(user_badge_id);
CREATE INDEX IF NOT EXISTS idx_badge_ledger_ref ON badge_ledger(source_type, ref_id);
CREATE INDEX IF NOT EXISTS idx_badge_ledger_time ON badge_ledger(created_at);
CREATE INDEX IF NOT EXISTS idx_badge_ledger_change_type ON badge_ledger(change_type);

-- ==================== 兑换相关 ====================

-- 权益定义
CREATE TABLE IF NOT EXISTS benefits (
    id BIGSERIAL PRIMARY KEY,
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

COMMENT ON TABLE benefits IS '权益定义，徽章可兑换的权益';
COMMENT ON COLUMN benefits.benefit_type IS '权益类型：digital_asset-数字资产，coupon-优惠券，reservation-预约资格';
COMMENT ON COLUMN benefits.external_id IS '外部系统中的权益ID';
COMMENT ON COLUMN benefits.external_system IS '外部系统标识，如coupon_service、digital_asset_center等';

CREATE INDEX IF NOT EXISTS idx_benefits_type ON benefits(benefit_type);
CREATE INDEX IF NOT EXISTS idx_benefits_status ON benefits(status);

-- 兑换规则
CREATE TABLE IF NOT EXISTS badge_redemption_rules (
    id BIGSERIAL PRIMARY KEY,
    name VARCHAR(100) NOT NULL,
    description TEXT,
    benefit_id BIGINT NOT NULL REFERENCES benefits(id),

    -- 所需徽章配置
    required_badges JSONB NOT NULL, -- [{"badge_id": 1, "quantity": 1}]

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

COMMENT ON TABLE badge_redemption_rules IS '徽章兑换规则，定义徽章如何兑换为权益';
COMMENT ON COLUMN badge_redemption_rules.required_badges IS '所需徽章配置，JSON数组格式：[{"badge_id": 1, "quantity": 1}]';
COMMENT ON COLUMN badge_redemption_rules.frequency_type IS '频次限制类型：daily-每日，weekly-每周，monthly-每月，yearly-每年，account-账号维度';
COMMENT ON COLUMN badge_redemption_rules.auto_redeem IS '是否自动兑换，满足条件时自动触发兑换';

CREATE INDEX IF NOT EXISTS idx_redemption_rules_benefit ON badge_redemption_rules(benefit_id);
CREATE INDEX IF NOT EXISTS idx_redemption_rules_status ON badge_redemption_rules(status);
CREATE INDEX IF NOT EXISTS idx_redemption_rules_badges ON badge_redemption_rules USING GIN(required_badges);

-- 兑换订单
CREATE TABLE IF NOT EXISTS redemption_orders (
    id BIGSERIAL PRIMARY KEY,
    order_no VARCHAR(50) NOT NULL UNIQUE,
    user_id VARCHAR(100) NOT NULL,
    redemption_rule_id BIGINT NOT NULL REFERENCES badge_redemption_rules(id),
    benefit_id BIGINT NOT NULL REFERENCES benefits(id),

    status VARCHAR(20) NOT NULL DEFAULT 'pending', -- pending, completed, failed, cancelled

    -- 权益发放结果
    benefit_grant_ref VARCHAR(200), -- 外部系统权益发放ID
    benefit_grant_at TIMESTAMPTZ,

    failure_reason TEXT,

    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

COMMENT ON TABLE redemption_orders IS '兑换订单，记录用户的徽章兑换操作';
COMMENT ON COLUMN redemption_orders.status IS '订单状态：pending-处理中，completed-已完成，failed-失败，cancelled-已取消';
COMMENT ON COLUMN redemption_orders.benefit_grant_ref IS '外部系统权益发放的引用ID，用于追踪和对账';

CREATE INDEX IF NOT EXISTS idx_redemption_orders_user ON redemption_orders(user_id);
CREATE INDEX IF NOT EXISTS idx_redemption_orders_status ON redemption_orders(status);
CREATE INDEX IF NOT EXISTS idx_redemption_orders_rule ON redemption_orders(redemption_rule_id);
CREATE INDEX IF NOT EXISTS idx_redemption_orders_benefit ON redemption_orders(benefit_id);
CREATE INDEX IF NOT EXISTS idx_redemption_orders_time ON redemption_orders(created_at);

-- 兑换明细
CREATE TABLE IF NOT EXISTS redemption_details (
    id BIGSERIAL PRIMARY KEY,
    order_id BIGINT NOT NULL REFERENCES redemption_orders(id),
    user_badge_id BIGINT NOT NULL REFERENCES user_badges(id),
    badge_id BIGINT NOT NULL REFERENCES badges(id),
    quantity INT NOT NULL,

    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

COMMENT ON TABLE redemption_details IS '兑换明细，记录每次兑换消耗的具体徽章';
COMMENT ON COLUMN redemption_details.quantity IS '消耗的徽章数量';

CREATE INDEX IF NOT EXISTS idx_redemption_details_order ON redemption_details(order_id);
CREATE INDEX IF NOT EXISTS idx_redemption_details_user_badge ON redemption_details(user_badge_id);
CREATE INDEX IF NOT EXISTS idx_redemption_details_badge ON redemption_details(badge_id);

-- ==================== 通知相关 ====================

-- 通知配置
CREATE TABLE IF NOT EXISTS notification_configs (
    id BIGSERIAL PRIMARY KEY,
    badge_id BIGINT REFERENCES badges(id),
    benefit_id BIGINT REFERENCES benefits(id),

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

COMMENT ON TABLE notification_configs IS '通知配置，定义徽章/权益相关事件的通知规则';
COMMENT ON COLUMN notification_configs.trigger_type IS '触发类型：grant-发放，revoke-取消，expire-过期，expire_remind-过期提醒，redeem-兑换';
COMMENT ON COLUMN notification_configs.channels IS '通知渠道：["app_push", "sms", "wechat", "email", "in_app"]';
COMMENT ON COLUMN notification_configs.advance_days IS '提前通知天数，用于过期提醒场景';

CREATE INDEX IF NOT EXISTS idx_notification_configs_badge ON notification_configs(badge_id);
CREATE INDEX IF NOT EXISTS idx_notification_configs_benefit ON notification_configs(benefit_id);
CREATE INDEX IF NOT EXISTS idx_notification_configs_trigger ON notification_configs(trigger_type);

-- 通知任务
CREATE TABLE IF NOT EXISTS notification_tasks (
    id BIGSERIAL PRIMARY KEY,
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

COMMENT ON TABLE notification_tasks IS '通知任务，待发送的通知队列';
COMMENT ON COLUMN notification_tasks.status IS '任务状态：pending-待处理，processing-处理中，completed-已完成，failed-失败';
COMMENT ON COLUMN notification_tasks.template_params IS '模板参数，JSON格式，用于填充通知模板';

CREATE INDEX IF NOT EXISTS idx_notification_tasks_status ON notification_tasks(status);
CREATE INDEX IF NOT EXISTS idx_notification_tasks_user ON notification_tasks(user_id);
CREATE INDEX IF NOT EXISTS idx_notification_tasks_created ON notification_tasks(created_at);

-- ==================== 系统管理 ====================

-- 操作日志
CREATE TABLE IF NOT EXISTS operation_logs (
    id BIGSERIAL PRIMARY KEY,
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

COMMENT ON TABLE operation_logs IS '操作日志，记录管理后台的所有操作';
COMMENT ON COLUMN operation_logs.module IS '模块：badge, rule, grant, revoke, benefit, redemption等';
COMMENT ON COLUMN operation_logs.action IS '操作：create, update, delete, publish, activate, deactivate等';
COMMENT ON COLUMN operation_logs.before_data IS '操作前数据快照';
COMMENT ON COLUMN operation_logs.after_data IS '操作后数据快照';

CREATE INDEX IF NOT EXISTS idx_operation_logs_operator ON operation_logs(operator_id);
CREATE INDEX IF NOT EXISTS idx_operation_logs_module ON operation_logs(module);
CREATE INDEX IF NOT EXISTS idx_operation_logs_action ON operation_logs(action);
CREATE INDEX IF NOT EXISTS idx_operation_logs_target ON operation_logs(target_type, target_id);
CREATE INDEX IF NOT EXISTS idx_operation_logs_time ON operation_logs(created_at);

-- 批量任务
CREATE TABLE IF NOT EXISTS batch_tasks (
    id BIGSERIAL PRIMARY KEY,
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

COMMENT ON TABLE batch_tasks IS '批量任务，用于大批量发放、取消、导出等操作';
COMMENT ON COLUMN batch_tasks.task_type IS '任务类型：batch_grant-批量发放，batch_revoke-批量取消，data_export-数据导出';
COMMENT ON COLUMN batch_tasks.file_url IS '上传的源文件地址（OSS）';
COMMENT ON COLUMN batch_tasks.result_file_url IS '处理结果文件地址（OSS）';
COMMENT ON COLUMN batch_tasks.progress IS '处理进度，0-100';

CREATE INDEX IF NOT EXISTS idx_batch_tasks_status ON batch_tasks(status);
CREATE INDEX IF NOT EXISTS idx_batch_tasks_type ON batch_tasks(task_type);
CREATE INDEX IF NOT EXISTS idx_batch_tasks_creator ON batch_tasks(created_by);
CREATE INDEX IF NOT EXISTS idx_batch_tasks_time ON batch_tasks(created_at);

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
DROP TRIGGER IF EXISTS update_badge_categories_updated_at ON badge_categories;
CREATE TRIGGER update_badge_categories_updated_at
    BEFORE UPDATE ON badge_categories
    FOR EACH ROW EXECUTE FUNCTION update_updated_at_column();

DROP TRIGGER IF EXISTS update_badge_series_updated_at ON badge_series;
CREATE TRIGGER update_badge_series_updated_at
    BEFORE UPDATE ON badge_series
    FOR EACH ROW EXECUTE FUNCTION update_updated_at_column();

DROP TRIGGER IF EXISTS update_badges_updated_at ON badges;
CREATE TRIGGER update_badges_updated_at
    BEFORE UPDATE ON badges
    FOR EACH ROW EXECUTE FUNCTION update_updated_at_column();

DROP TRIGGER IF EXISTS update_badge_rules_updated_at ON badge_rules;
CREATE TRIGGER update_badge_rules_updated_at
    BEFORE UPDATE ON badge_rules
    FOR EACH ROW EXECUTE FUNCTION update_updated_at_column();

DROP TRIGGER IF EXISTS update_user_badges_updated_at ON user_badges;
CREATE TRIGGER update_user_badges_updated_at
    BEFORE UPDATE ON user_badges
    FOR EACH ROW EXECUTE FUNCTION update_updated_at_column();

DROP TRIGGER IF EXISTS update_benefits_updated_at ON benefits;
CREATE TRIGGER update_benefits_updated_at
    BEFORE UPDATE ON benefits
    FOR EACH ROW EXECUTE FUNCTION update_updated_at_column();

DROP TRIGGER IF EXISTS update_badge_redemption_rules_updated_at ON badge_redemption_rules;
CREATE TRIGGER update_badge_redemption_rules_updated_at
    BEFORE UPDATE ON badge_redemption_rules
    FOR EACH ROW EXECUTE FUNCTION update_updated_at_column();

DROP TRIGGER IF EXISTS update_redemption_orders_updated_at ON redemption_orders;
CREATE TRIGGER update_redemption_orders_updated_at
    BEFORE UPDATE ON redemption_orders
    FOR EACH ROW EXECUTE FUNCTION update_updated_at_column();

DROP TRIGGER IF EXISTS update_notification_configs_updated_at ON notification_configs;
CREATE TRIGGER update_notification_configs_updated_at
    BEFORE UPDATE ON notification_configs
    FOR EACH ROW EXECUTE FUNCTION update_updated_at_column();

DROP TRIGGER IF EXISTS update_notification_tasks_updated_at ON notification_tasks;
CREATE TRIGGER update_notification_tasks_updated_at
    BEFORE UPDATE ON notification_tasks
    FOR EACH ROW EXECUTE FUNCTION update_updated_at_column();

DROP TRIGGER IF EXISTS update_batch_tasks_updated_at ON batch_tasks;
CREATE TRIGGER update_batch_tasks_updated_at
    BEFORE UPDATE ON batch_tasks
    FOR EACH ROW EXECUTE FUNCTION update_updated_at_column();
