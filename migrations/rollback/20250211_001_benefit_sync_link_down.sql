-- 回滚 20250211_001_benefit_sync_link
DROP INDEX IF EXISTS idx_badge_benefit_links_benefit;
DROP INDEX IF EXISTS idx_badge_benefit_links_badge;
DROP TABLE IF EXISTS badge_benefit_links;
DROP INDEX IF EXISTS idx_benefit_sync_logs_status;
DROP TABLE IF EXISTS benefit_sync_logs;
