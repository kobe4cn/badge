-- 回滚脚本：撤销徽章系统初始化 schema
-- 对应 UP 迁移：20250128_001_init_schema.sql
-- 此脚本按依赖关系反序删除所有核心表、索引、触发器和函数
-- 注意：执行此回滚将丢失所有徽章系统核心数据

-- ============================================
-- 1. 删除触发器
-- ============================================
DROP TRIGGER IF EXISTS update_batch_tasks_updated_at ON batch_tasks;
DROP TRIGGER IF EXISTS update_notification_tasks_updated_at ON notification_tasks;
DROP TRIGGER IF EXISTS update_notification_configs_updated_at ON notification_configs;
DROP TRIGGER IF EXISTS update_redemption_orders_updated_at ON redemption_orders;
DROP TRIGGER IF EXISTS update_badge_redemption_rules_updated_at ON badge_redemption_rules;
DROP TRIGGER IF EXISTS update_benefits_updated_at ON benefits;
DROP TRIGGER IF EXISTS update_user_badges_updated_at ON user_badges;
DROP TRIGGER IF EXISTS update_badge_rules_updated_at ON badge_rules;
DROP TRIGGER IF EXISTS update_badges_updated_at ON badges;
DROP TRIGGER IF EXISTS update_badge_series_updated_at ON badge_series;
DROP TRIGGER IF EXISTS update_badge_categories_updated_at ON badge_categories;

-- ============================================
-- 2. 按依赖关系反序删除表（先删除有外键依赖的表）
-- ============================================

-- 兑换明细（依赖 redemption_orders, user_badges, badges）
DROP TABLE IF EXISTS redemption_details CASCADE;

-- 兑换订单（依赖 badge_redemption_rules, benefits）
DROP TABLE IF EXISTS redemption_orders CASCADE;

-- 兑换规则（依赖 benefits）
DROP TABLE IF EXISTS badge_redemption_rules CASCADE;

-- 通知任务（无外键依赖但属于通知模块）
DROP TABLE IF EXISTS notification_tasks CASCADE;

-- 通知配置（依赖 badges, benefits）
DROP TABLE IF EXISTS notification_configs CASCADE;

-- 徽章账本（依赖 badges, user_badges）
DROP TABLE IF EXISTS badge_ledger CASCADE;

-- 用户徽章（依赖 badges）
DROP TABLE IF EXISTS user_badges CASCADE;

-- 徽章规则（依赖 badges）
DROP TABLE IF EXISTS badge_rules CASCADE;

-- 徽章定义（依赖 badge_series）
DROP TABLE IF EXISTS badges CASCADE;

-- 徽章系列（依赖 badge_categories）
DROP TABLE IF EXISTS badge_series CASCADE;

-- 徽章分类（顶层表）
DROP TABLE IF EXISTS badge_categories CASCADE;

-- 权益定义
DROP TABLE IF EXISTS benefits CASCADE;

-- 批量任务
DROP TABLE IF EXISTS batch_tasks CASCADE;

-- 操作日志
DROP TABLE IF EXISTS operation_logs CASCADE;

-- ============================================
-- 3. 删除函数
-- ============================================
DROP FUNCTION IF EXISTS update_updated_at_column() CASCADE;

-- ============================================
-- 4. 删除扩展
-- ============================================
DROP EXTENSION IF EXISTS pg_trgm;
