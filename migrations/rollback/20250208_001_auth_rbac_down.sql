-- 回滚脚本：撤销认证与 RBAC 权限管理模块
-- 对应 UP 迁移：20250208_001_auth_rbac.sql
-- 此脚本按照创建的反序删除所有 RBAC 相关表及其索引和触发器
-- 注意：执行此回滚将丢失所有管理员用户、角色、权限和 API Key 数据

-- ============================================
-- 1. 删除触发器
-- ============================================
DROP TRIGGER IF EXISTS update_role_updated_at ON role;
DROP TRIGGER IF EXISTS update_admin_user_updated_at ON admin_user;

-- ============================================
-- 2. 按创建反序删除表（先删除有外键依赖的表）
-- ============================================

-- API Key 表（依赖 admin_user）
DROP TABLE IF EXISTS api_key CASCADE;

-- 角色-权限关联表（依赖 role 和 permission）
DROP TABLE IF EXISTS role_permission CASCADE;

-- 用户-角色关联表（依赖 admin_user 和 role）
DROP TABLE IF EXISTS user_role CASCADE;

-- 权限表
DROP TABLE IF EXISTS permission CASCADE;

-- 角色表
DROP TABLE IF EXISTS role CASCADE;

-- 系统用户表
DROP TABLE IF EXISTS admin_user CASCADE;
