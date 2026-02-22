-- 回滚 20250216_001_asset_library
DROP INDEX IF EXISTS idx_assets_created;
DROP INDEX IF EXISTS idx_assets_status;
DROP INDEX IF EXISTS idx_assets_tags;
DROP INDEX IF EXISTS idx_assets_category;
DROP INDEX IF EXISTS idx_assets_type;
DROP TABLE IF EXISTS assets;
