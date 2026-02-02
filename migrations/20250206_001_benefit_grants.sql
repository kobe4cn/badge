-- 权益发放记录表
-- 记录每次权益发放的详细信息，支持异步发放、外部系统回调、失败重试等场景

-- ============================================
-- 1. 权益类型说明（仅文档记录，实际类型由 Rust 枚举控制）
-- ============================================

-- 支持的权益类型（不修改数据库约束，由应用层控制）：
-- - digital_asset: 数字资产（NFT、虚拟商品等）
-- - coupon: 优惠券（折扣券、满减券等）
-- - reservation: 预约资格（活动名额、服务预约等）
-- - points: 积分（奖励积分、消费积分等）
-- - physical: 实物奖品（需邮寄配送）
-- - membership: 会员权益（等级升级、会员期限等）
-- - external_callback: 外部回调（通用外部系统对接）

-- ============================================
-- 2. 创建 benefit_grants 表
-- ============================================

CREATE TABLE benefit_grants (
    id BIGSERIAL PRIMARY KEY,
    grant_no VARCHAR(200) NOT NULL,  -- 加长以支持自动权益的幂等键格式

    -- 关联信息
    user_id VARCHAR(100) NOT NULL,                                              -- SWID，用户唯一标识
    benefit_id BIGINT NOT NULL REFERENCES benefits(id) ON DELETE RESTRICT,      -- 权益不可删除（有发放记录时）
    redemption_order_id BIGINT REFERENCES redemption_orders(id) ON DELETE SET NULL, -- 订单删除时置空

    -- 来源追踪
    source_type VARCHAR(50) NOT NULL DEFAULT 'MANUAL',  -- 发放来源：MANUAL(手动), AUTO(自动), REDEMPTION(兑换), CASCADE(级联)
    source_id VARCHAR(200),                              -- 来源关联ID（如规则ID、兑换订单号等）
    quantity INT NOT NULL DEFAULT 1,                     -- 发放数量

    -- 状态管理
    -- pending: 待处理，等待外部系统确认
    -- processing: 处理中，已发送到外部系统
    -- success: 发放成功
    -- failed: 发放失败，可重试
    -- revoked: 已撤销
    status VARCHAR(20) NOT NULL DEFAULT 'pending',
    status_message TEXT,                                         -- 状态变更原因或错误信息

    -- 外部系统交互
    external_ref VARCHAR(200),                                   -- 外部系统的发放单号/引用ID
    external_response JSONB,                                     -- 外部系统返回的完整响应

    -- 权益数据（根据权益类型存储不同内容）
    -- coupon: {"coupon_code": "ABC123", "coupon_url": "..."}
    -- points: {"amount": 100, "point_type": "bonus"}
    -- physical: {"sku": "...", "address": {...}, "tracking_no": "..."}
    -- membership: {"level": "gold", "duration_days": 30}
    -- external_callback: {"callback_url": "...", "callback_status": "..."}
    payload JSONB,

    -- 时间追踪
    granted_at TIMESTAMPTZ,                                      -- 实际发放成功的时间
    expires_at TIMESTAMPTZ,                                      -- 权益过期时间
    revoked_at TIMESTAMPTZ,                                      -- 撤销时间

    -- 重试机制
    retry_count INT NOT NULL DEFAULT 0,                          -- 已重试次数
    next_retry_at TIMESTAMPTZ,                                   -- 下次重试时间

    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),

    CONSTRAINT benefit_grants_grant_no_unique UNIQUE (grant_no)
);

COMMENT ON TABLE benefit_grants IS '权益发放记录，追踪每次权益发放的完整生命周期';
COMMENT ON COLUMN benefit_grants.grant_no IS '发放单号，全局唯一，格式如 BG20250131xxxx';
COMMENT ON COLUMN benefit_grants.user_id IS 'SWID，接收权益的用户标识';
COMMENT ON COLUMN benefit_grants.benefit_id IS '关联的权益定义ID';
COMMENT ON COLUMN benefit_grants.redemption_order_id IS '关联的兑换订单ID，手动发放或活动发放时可为空';
COMMENT ON COLUMN benefit_grants.source_type IS '发放来源类型：MANUAL-手动发放，AUTO-自动发放，REDEMPTION-兑换发放，CASCADE-级联发放';
COMMENT ON COLUMN benefit_grants.source_id IS '来源关联ID，如规则ID、兑换订单号等';
COMMENT ON COLUMN benefit_grants.quantity IS '发放数量，默认为1';
COMMENT ON COLUMN benefit_grants.status IS '发放状态：pending-待处理，processing-处理中，success-成功，failed-失败，revoked-已撤销';
COMMENT ON COLUMN benefit_grants.status_message IS '状态说明，如失败原因、撤销原因等';
COMMENT ON COLUMN benefit_grants.external_ref IS '外部系统返回的发放单号或引用ID，用于对账和查询';
COMMENT ON COLUMN benefit_grants.external_response IS '外部系统的完整响应数据，用于问题排查';
COMMENT ON COLUMN benefit_grants.payload IS '权益发放数据，如优惠券码、积分数量、实物信息等';
COMMENT ON COLUMN benefit_grants.granted_at IS '权益实际发放成功的时间';
COMMENT ON COLUMN benefit_grants.expires_at IS '权益过期时间，过期后用户无法使用';
COMMENT ON COLUMN benefit_grants.revoked_at IS '权益撤销时间';
COMMENT ON COLUMN benefit_grants.retry_count IS '发放失败后的重试次数';
COMMENT ON COLUMN benefit_grants.next_retry_at IS '下次重试时间，用于定时任务扫描';

-- ============================================
-- 3. 创建索引
-- ============================================

-- 用户查询：查询用户的所有权益发放记录
CREATE INDEX idx_benefit_grants_user ON benefit_grants(user_id);

-- 状态查询：查询待处理、失败需重试的记录
CREATE INDEX idx_benefit_grants_status ON benefit_grants(status);

-- 权益查询：按权益类型统计发放情况
CREATE INDEX idx_benefit_grants_benefit ON benefit_grants(benefit_id);

-- 订单关联：通过兑换订单查询发放记录
CREATE INDEX idx_benefit_grants_order ON benefit_grants(redemption_order_id)
    WHERE redemption_order_id IS NOT NULL;

-- 用户+状态组合查询：查询用户的有效权益
CREATE INDEX idx_benefit_grants_user_status ON benefit_grants(user_id, status);

-- 重试任务扫描：定时任务查找需要重试的记录
CREATE INDEX idx_benefit_grants_retry ON benefit_grants(next_retry_at)
    WHERE status = 'failed' AND next_retry_at IS NOT NULL;

-- 过期扫描：查找即将过期或已过期的权益
CREATE INDEX idx_benefit_grants_expires ON benefit_grants(expires_at)
    WHERE expires_at IS NOT NULL AND status = 'success';

-- 外部引用查询：通过外部系统单号反查
CREATE INDEX idx_benefit_grants_external_ref ON benefit_grants(external_ref)
    WHERE external_ref IS NOT NULL;

-- 时间范围查询：按创建时间统计
CREATE INDEX idx_benefit_grants_created ON benefit_grants(created_at);

-- 来源类型查询：按来源统计
CREATE INDEX idx_benefit_grants_source ON benefit_grants(source_type);

-- ============================================
-- 4. 创建触发器
-- ============================================

CREATE TRIGGER update_benefit_grants_updated_at
    BEFORE UPDATE ON benefit_grants
    FOR EACH ROW EXECUTE FUNCTION update_updated_at_column();
