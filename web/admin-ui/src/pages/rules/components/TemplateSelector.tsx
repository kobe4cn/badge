/**
 * 模板选择器组件
 *
 * 展示可用的规则模板列表，按分类组织，支持用户选择模板
 */

import React, { useState, useEffect } from 'react';
import { Card, Tabs, List, Tag, Button, Spin, Empty, message } from 'antd';
import { listTemplates, RuleTemplate } from '@/services/template';

interface TemplateSelectorProps {
  /** 模板选择回调 */
  onSelect: (template: RuleTemplate) => void;
}

/**
 * 模板分类标签映射
 */
const categoryLabels: Record<string, string> = {
  basic: '基础场景',
  advanced: '高级场景',
  industry: '行业模板',
};

/**
 * 子分类标签映射
 */
const subcategoryLabels: Record<string, string> = {
  'e-commerce': '电商',
  gaming: '游戏',
  o2o: 'O2O',
  education: '教育',
  finance: '金融',
};

export const TemplateSelector: React.FC<TemplateSelectorProps> = ({ onSelect }) => {
  const [templates, setTemplates] = useState<RuleTemplate[]>([]);
  const [loading, setLoading] = useState(false);
  const [category, setCategory] = useState<string>('basic');

  useEffect(() => {
    loadTemplates();
  }, [category]);

  /**
   * 加载模板列表
   *
   * 根据当前选中的分类获取对应模板
   */
  const loadTemplates = async () => {
    setLoading(true);
    try {
      const res = await listTemplates(category);
      setTemplates(res.items);
    } catch (error) {
      message.error('加载模板失败');
    } finally {
      setLoading(false);
    }
  };

  const tabItems = Object.entries(categoryLabels).map(([key, label]) => ({
    key,
    label,
  }));

  return (
    <Card title="选择规则模板" className="template-selector">
      <Tabs
        activeKey={category}
        onChange={setCategory}
        items={tabItems}
      />

      <Spin spinning={loading}>
        {templates.length === 0 ? (
          <Empty description="暂无模板" />
        ) : (
          <List
            dataSource={templates}
            renderItem={(template) => (
              <List.Item
                key={template.code}
                actions={[
                  <Button
                    type="primary"
                    size="small"
                    onClick={() => onSelect(template)}
                  >
                    使用此模板
                  </Button>,
                ]}
              >
                <List.Item.Meta
                  title={
                    <span>
                      {template.name}
                      {template.isSystem && (
                        <Tag color="blue" style={{ marginLeft: 8 }}>系统</Tag>
                      )}
                    </span>
                  }
                  description={template.description}
                />
                {template.subcategory && (
                  <Tag>{subcategoryLabels[template.subcategory] || template.subcategory}</Tag>
                )}
              </List.Item>
            )}
          />
        )}
      </Spin>
    </Card>
  );
};

export default TemplateSelector;
