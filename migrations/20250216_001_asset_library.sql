-- 素材库表
-- 集中管理徽章系统使用的各类媒体资产

CREATE TABLE IF NOT EXISTS assets (
    id BIGSERIAL PRIMARY KEY,
    -- 素材名称
    name VARCHAR(100) NOT NULL,
    -- 素材类型：IMAGE（图片）, ANIMATION（动画）, VIDEO（视频）, MODEL_3D（3D模型）
    asset_type VARCHAR(20) NOT NULL,
    -- 文件 URL（OSS 或 CDN 地址）
    file_url TEXT NOT NULL,
    -- 缩略图 URL
    thumbnail_url TEXT,
    -- 文件大小（字节）
    file_size BIGINT DEFAULT 0,
    -- 文件格式（如 png, gif, glb, gltf）
    file_format VARCHAR(20),
    -- 图片/视频宽度
    width INT,
    -- 图片/视频高度
    height INT,
    -- 扩展元数据（JSON 格式，可存储 3D 模型信息等）
    metadata JSONB,
    -- 分类标签
    category VARCHAR(50),
    -- 搜索标签数组
    tags TEXT[],
    -- 状态：active（可用）, archived（已归档）
    status VARCHAR(20) DEFAULT 'active',
    -- 使用次数统计
    usage_count INT DEFAULT 0,
    -- 创建人
    created_by VARCHAR(100),
    created_at TIMESTAMPTZ DEFAULT NOW(),
    updated_at TIMESTAMPTZ DEFAULT NOW()
);

-- 素材类型索引
CREATE INDEX IF NOT EXISTS idx_assets_type ON assets(asset_type);

-- 分类索引
CREATE INDEX IF NOT EXISTS idx_assets_category ON assets(category);

-- 标签索引（GIN 索引支持数组查询）
CREATE INDEX IF NOT EXISTS idx_assets_tags ON assets USING GIN(tags);

-- 状态索引
CREATE INDEX IF NOT EXISTS idx_assets_status ON assets(status);

-- 创建时间索引（用于排序）
CREATE INDEX IF NOT EXISTS idx_assets_created ON assets(created_at DESC);

COMMENT ON TABLE assets IS '素材库：管理徽章系统的图片、动画、视频和 3D 模型资源';
COMMENT ON COLUMN assets.asset_type IS '素材类型：IMAGE-图片，ANIMATION-动画，VIDEO-视频，MODEL_3D-3D模型';
COMMENT ON COLUMN assets.metadata IS '扩展元数据，如 3D 模型的多边形数、材质信息等';
COMMENT ON COLUMN assets.tags IS '搜索标签数组，支持多标签筛选';
