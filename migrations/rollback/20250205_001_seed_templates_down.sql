-- 回滚 20250205_001_seed_templates
DELETE FROM rule_templates WHERE code IN (
    'first_event', 'cumulative_amount', 'cumulative_count',
    'user_level_gte', 'tag_match', 'time_window_event',
    'streak_days', 'frequency_limit',
    'ecom_first_purchase', 'ecom_order_amount', 'ecom_repeat_purchase',
    'game_level_reached', 'game_achievement',
    'o2o_store_visit', 'o2o_review'
);
