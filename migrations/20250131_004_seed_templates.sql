-- 预置规则模板种子数据
-- 包含基础场景、高级场景和行业模板三大类共 15 个系统内置模板

-- ==================== 基础场景模板（5个）====================

-- 1. 首次事件触发
INSERT INTO rule_templates (code, name, description, category, subcategory, template_json, parameters, is_system) VALUES
('first_event', '首次事件触发', '用户首次完成指定类型的事件时触发，适用于新手引导、首次互动等场景', 'basic', NULL,
 '{
   "root": {
     "type": "condition",
     "field": "event.type",
     "operator": "eq",
     "value": "${event_type}"
   },
   "context": {
     "requireFirstTime": true
   }
 }',
 '[
   {
     "name": "event_type",
     "type": "string",
     "label": "事件类型",
     "required": true,
     "description": "触发徽章的事件类型",
     "options": [
       {"value": "checkin", "label": "签到"},
       {"value": "purchase", "label": "购买"},
       {"value": "share", "label": "分享"},
       {"value": "comment", "label": "评论"},
       {"value": "like", "label": "点赞"},
       {"value": "follow", "label": "关注"},
       {"value": "register", "label": "注册"}
     ]
   }
 ]',
 true)
ON CONFLICT (code) DO NOTHING;

-- 2. 累计金额达标
INSERT INTO rule_templates (code, name, description, category, subcategory, template_json, parameters, is_system) VALUES
('cumulative_amount', '累计金额达标', '用户累计金额达到指定阈值时触发，适用于消费激励、等级升级等场景', 'basic', NULL,
 '{
   "root": {
     "type": "condition",
     "field": "user.stats.total_amount",
     "operator": "gte",
     "value": "${amount}"
   }
 }',
 '[
   {
     "name": "amount",
     "type": "number",
     "label": "累计金额",
     "required": true,
     "description": "需要达到的累计金额阈值（单位：分）",
     "default": 10000,
     "min": 1,
     "validation": {
       "type": "number",
       "min": 1
     }
   }
 ]',
 true)
ON CONFLICT (code) DO NOTHING;

-- 3. 累计次数达标
INSERT INTO rule_templates (code, name, description, category, subcategory, template_json, parameters, is_system) VALUES
('cumulative_count', '累计次数达标', '用户完成指定事件累计次数达到阈值时触发，适用于活跃度激励、任务完成等场景', 'basic', NULL,
 '{
   "root": {
     "type": "group",
     "operator": "and",
     "children": [
       {
         "type": "condition",
         "field": "event.type",
         "operator": "eq",
         "value": "${event_type}"
       },
       {
         "type": "condition",
         "field": "user.stats.event_count.${event_type}",
         "operator": "gte",
         "value": "${count}"
       }
     ]
   }
 }',
 '[
   {
     "name": "event_type",
     "type": "string",
     "label": "事件类型",
     "required": true,
     "description": "需要统计的事件类型",
     "options": [
       {"value": "checkin", "label": "签到"},
       {"value": "purchase", "label": "购买"},
       {"value": "share", "label": "分享"},
       {"value": "comment", "label": "评论"},
       {"value": "like", "label": "点赞"}
     ]
   },
   {
     "name": "count",
     "type": "number",
     "label": "累计次数",
     "required": true,
     "description": "需要达到的累计次数",
     "default": 10,
     "min": 1
   }
 ]',
 true)
ON CONFLICT (code) DO NOTHING;

-- 4. 用户等级达标
INSERT INTO rule_templates (code, name, description, category, subcategory, template_json, parameters, is_system) VALUES
('user_level_gte', '用户等级达标', '用户等级达到指定级别时触发，适用于会员权益、等级奖励等场景', 'basic', NULL,
 '{
   "root": {
     "type": "condition",
     "field": "user.profile.level",
     "operator": "gte",
     "value": "${level}"
   }
 }',
 '[
   {
     "name": "level",
     "type": "number",
     "label": "等级要求",
     "required": true,
     "description": "用户需要达到的等级（大于等于）",
     "default": 1,
     "min": 1,
     "max": 100
   }
 ]',
 true)
ON CONFLICT (code) DO NOTHING;

-- 5. 用户标签匹配
INSERT INTO rule_templates (code, name, description, category, subcategory, template_json, parameters, is_system) VALUES
('tag_match', '用户标签匹配', '用户拥有指定标签时触发，适用于人群定向、特定用户群体奖励等场景', 'basic', NULL,
 '{
   "root": {
     "type": "condition",
     "field": "user.tags",
     "operator": "contains_any",
     "value": "${tags}"
   }
 }',
 '[
   {
     "name": "tags",
     "type": "array",
     "label": "用户标签",
     "required": true,
     "description": "需要匹配的用户标签列表（匹配任意一个即可）",
     "itemType": "string",
     "placeholder": "输入标签，如：vip, new_user"
   }
 ]',
 true)
ON CONFLICT (code) DO NOTHING;

-- ==================== 高级场景模板（3个）====================

-- 6. 时间窗口内事件
INSERT INTO rule_templates (code, name, description, category, subcategory, template_json, parameters, is_system) VALUES
('time_window_event', '时间窗口内事件', '在指定时间窗口内完成事件时触发，适用于限时活动、节日促销等场景', 'advanced', NULL,
 '{
   "root": {
     "type": "group",
     "operator": "and",
     "children": [
       {
         "type": "condition",
         "field": "event.type",
         "operator": "eq",
         "value": "${event_type}"
       },
       {
         "type": "condition",
         "field": "event.timestamp",
         "operator": "gte",
         "value": "${start_time}"
       },
       {
         "type": "condition",
         "field": "event.timestamp",
         "operator": "lte",
         "value": "${end_time}"
       }
     ]
   }
 }',
 '[
   {
     "name": "event_type",
     "type": "string",
     "label": "事件类型",
     "required": true,
     "description": "需要在时间窗口内触发的事件类型"
   },
   {
     "name": "start_time",
     "type": "datetime",
     "label": "开始时间",
     "required": true,
     "description": "时间窗口开始时间"
   },
   {
     "name": "end_time",
     "type": "datetime",
     "label": "结束时间",
     "required": true,
     "description": "时间窗口结束时间"
   }
 ]',
 true)
ON CONFLICT (code) DO NOTHING;

-- 7. 连续签到天数
INSERT INTO rule_templates (code, name, description, category, subcategory, template_json, parameters, is_system) VALUES
('streak_days', '连续签到天数', '用户连续签到达到指定天数时触发，适用于签到奖励、习惯养成等场景', 'advanced', NULL,
 '{
   "root": {
     "type": "group",
     "operator": "and",
     "children": [
       {
         "type": "condition",
         "field": "event.type",
         "operator": "eq",
         "value": "checkin"
       },
       {
         "type": "condition",
         "field": "user.stats.streak_days",
         "operator": "gte",
         "value": "${days}"
       }
     ]
   }
 }',
 '[
   {
     "name": "days",
     "type": "number",
     "label": "连续天数",
     "required": true,
     "description": "需要连续签到的天数",
     "default": 7,
     "min": 1,
     "max": 365
   }
 ]',
 true)
ON CONFLICT (code) DO NOTHING;

-- 8. 频次限制事件
INSERT INTO rule_templates (code, name, description, category, subcategory, template_json, parameters, is_system) VALUES
('frequency_limit', '频次限制事件', '在指定周期内完成指定次数事件时触发，适用于周期性任务、限频活动等场景', 'advanced', NULL,
 '{
   "root": {
     "type": "group",
     "operator": "and",
     "children": [
       {
         "type": "condition",
         "field": "event.type",
         "operator": "eq",
         "value": "${event_type}"
       },
       {
         "type": "condition",
         "field": "user.stats.period_count.${period}.${event_type}",
         "operator": "gte",
         "value": "${count}"
       }
     ]
   },
   "context": {
     "period": "${period}"
   }
 }',
 '[
   {
     "name": "event_type",
     "type": "string",
     "label": "事件类型",
     "required": true,
     "description": "需要统计的事件类型"
   },
   {
     "name": "count",
     "type": "number",
     "label": "触发次数",
     "required": true,
     "description": "周期内需要达到的事件次数",
     "default": 5,
     "min": 1
   },
   {
     "name": "period",
     "type": "string",
     "label": "统计周期",
     "required": true,
     "description": "事件统计的时间周期",
     "default": "daily",
     "options": [
       {"value": "daily", "label": "每日"},
       {"value": "weekly", "label": "每周"},
       {"value": "monthly", "label": "每月"}
     ]
   }
 ]',
 true)
ON CONFLICT (code) DO NOTHING;

-- ==================== 行业模板 - 电商（3个）====================

-- 9. 首次购买
INSERT INTO rule_templates (code, name, description, category, subcategory, template_json, parameters, is_system) VALUES
('ecom_first_purchase', '首次购买', '用户首次完成购买时触发，适用于新客激励、首单奖励等场景', 'industry', 'e-commerce',
 '{
   "root": {
     "type": "condition",
     "field": "event.type",
     "operator": "eq",
     "value": "purchase"
   },
   "context": {
     "requireFirstTime": true,
     "eventType": "purchase"
   }
 }',
 '[]',
 true)
ON CONFLICT (code) DO NOTHING;

-- 10. 订单满额
INSERT INTO rule_templates (code, name, description, category, subcategory, template_json, parameters, is_system) VALUES
('ecom_order_amount', '订单满额', '单笔订单金额达到指定阈值时触发，适用于满减促销、大单奖励等场景', 'industry', 'e-commerce',
 '{
   "root": {
     "type": "group",
     "operator": "and",
     "children": [
       {
         "type": "condition",
         "field": "event.type",
         "operator": "eq",
         "value": "purchase"
       },
       {
         "type": "condition",
         "field": "event.data.amount",
         "operator": "gte",
         "value": "${amount}"
       }
     ]
   }
 }',
 '[
   {
     "name": "amount",
     "type": "number",
     "label": "订单金额",
     "required": true,
     "description": "单笔订单金额阈值（单位：分）",
     "default": 10000,
     "min": 1
   }
 ]',
 true)
ON CONFLICT (code) DO NOTHING;

-- 11. 复购
INSERT INTO rule_templates (code, name, description, category, subcategory, template_json, parameters, is_system) VALUES
('ecom_repeat_purchase', '复购', '在指定天数内完成指定次数购买时触发，适用于复购激励、忠诚度奖励等场景', 'industry', 'e-commerce',
 '{
   "root": {
     "type": "group",
     "operator": "and",
     "children": [
       {
         "type": "condition",
         "field": "event.type",
         "operator": "eq",
         "value": "purchase"
       },
       {
         "type": "condition",
         "field": "user.stats.purchase_count_in_days.${days}",
         "operator": "gte",
         "value": "${count}"
       }
     ]
   },
   "context": {
     "lookbackDays": "${days}"
   }
 }',
 '[
   {
     "name": "count",
     "type": "number",
     "label": "购买次数",
     "required": true,
     "description": "需要完成的购买次数",
     "default": 3,
     "min": 2
   },
   {
     "name": "days",
     "type": "number",
     "label": "天数范围",
     "required": true,
     "description": "统计购买次数的天数范围",
     "default": 30,
     "min": 1,
     "max": 365
   }
 ]',
 true)
ON CONFLICT (code) DO NOTHING;

-- ==================== 行业模板 - 游戏（2个）====================

-- 12. 等级达成
INSERT INTO rule_templates (code, name, description, category, subcategory, template_json, parameters, is_system) VALUES
('game_level_reached', '等级达成', '游戏角色等级达到指定级别时触发，适用于成长激励、等级里程碑等场景', 'industry', 'gaming',
 '{
   "root": {
     "type": "group",
     "operator": "and",
     "children": [
       {
         "type": "condition",
         "field": "event.type",
         "operator": "eq",
         "value": "level_up"
       },
       {
         "type": "condition",
         "field": "event.data.level",
         "operator": "gte",
         "value": "${level}"
       }
     ]
   }
 }',
 '[
   {
     "name": "level",
     "type": "number",
     "label": "等级要求",
     "required": true,
     "description": "角色需要达到的等级",
     "default": 10,
     "min": 1,
     "max": 999
   }
 ]',
 true)
ON CONFLICT (code) DO NOTHING;

-- 13. 成就解锁
INSERT INTO rule_templates (code, name, description, category, subcategory, template_json, parameters, is_system) VALUES
('game_achievement', '成就解锁', '解锁指定成就时触发，适用于成就系统联动、特殊奖励等场景', 'industry', 'gaming',
 '{
   "root": {
     "type": "group",
     "operator": "and",
     "children": [
       {
         "type": "condition",
         "field": "event.type",
         "operator": "eq",
         "value": "achievement_unlock"
       },
       {
         "type": "condition",
         "field": "event.data.achievement_id",
         "operator": "eq",
         "value": "${achievement_id}"
       }
     ]
   }
 }',
 '[
   {
     "name": "achievement_id",
     "type": "string",
     "label": "成就ID",
     "required": true,
     "description": "需要解锁的成就标识",
     "placeholder": "输入成就ID"
   }
 ]',
 true)
ON CONFLICT (code) DO NOTHING;

-- ==================== 行业模板 - O2O（2个）====================

-- 14. 到店
INSERT INTO rule_templates (code, name, description, category, subcategory, template_json, parameters, is_system) VALUES
('o2o_store_visit', '到店', '用户到店签到时触发，可指定特定门店或任意门店，适用于到店奖励、线下引流等场景', 'industry', 'o2o',
 '{
   "root": {
     "type": "group",
     "operator": "and",
     "children": [
       {
         "type": "condition",
         "field": "event.type",
         "operator": "eq",
         "value": "store_visit"
       },
       {
         "type": "condition",
         "field": "event.data.store_id",
         "operator": "${store_id ? ''eq'' : ''exists''}",
         "value": "${store_id}"
       }
     ]
   }
 }',
 '[
   {
     "name": "store_id",
     "type": "string",
     "label": "门店ID",
     "required": false,
     "description": "指定门店ID，留空表示任意门店",
     "placeholder": "留空表示任意门店"
   }
 ]',
 true)
ON CONFLICT (code) DO NOTHING;

-- 15. 评价
INSERT INTO rule_templates (code, name, description, category, subcategory, template_json, parameters, is_system) VALUES
('o2o_review', '评价', '用户提交评价时触发，可设置最低评分要求，适用于好评激励、评价奖励等场景', 'industry', 'o2o',
 '{
   "root": {
     "type": "group",
     "operator": "and",
     "children": [
       {
         "type": "condition",
         "field": "event.type",
         "operator": "eq",
         "value": "review"
       },
       {
         "type": "condition",
         "field": "event.data.rating",
         "operator": "gte",
         "value": "${min_rating}"
       }
     ]
   }
 }',
 '[
   {
     "name": "min_rating",
     "type": "number",
     "label": "最低评分",
     "required": false,
     "description": "最低评分要求（1-5），留空表示无要求",
     "default": 1,
     "min": 1,
     "max": 5
   }
 ]',
 true)
ON CONFLICT (code) DO NOTHING;
