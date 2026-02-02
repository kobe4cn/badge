/**
 * 规则模板页面
 *
 * 展示预定义的规则模板，支持预览和使用模板创建规则
 */

import React, { useState, useMemo } from 'react';
import {
  Card,
  Row,
  Col,
  Input,
  Tag,
  Button,
  Modal,
  Empty,
  Spin,
  Typography,
  Space,
  Segmented,
} from 'antd';
import { PageContainer } from '@ant-design/pro-components';
import {
  SearchOutlined,
  EyeOutlined,
  PlayCircleOutlined,
  FileTextOutlined,
  PlusOutlined,
} from '@ant-design/icons';
import { useQuery } from '@tanstack/react-query';
import { useNavigate } from 'react-router-dom';
import {
  ReactFlow,
  Background,
  ReactFlowProvider,
} from '@xyflow/react';
import type { Node, Edge } from '@xyflow/react';
import '@xyflow/react/dist/style.css';
import { listTemplates, type RuleTemplate } from '@/services/template';

const { Text, Paragraph } = Typography;

/**
 * 模板分类显示配置
 */
const CATEGORY_MAP: Record<string, { text: string; color: string }> = {
  basic: { text: '基础模板', color: 'blue' },
  advanced: { text: '进阶模板', color: 'purple' },
  industry: { text: '行业模板', color: 'green' },
};

/**
 * 模板卡片组件
 */
interface TemplateCardProps {
  template: RuleTemplate;
  onPreview: (template: RuleTemplate) => void;
  onUse: (template: RuleTemplate) => void;
}

const TemplateCard: React.FC<TemplateCardProps> = ({ template, onPreview, onUse }) => {
  const categoryConfig = CATEGORY_MAP[template.category];

  return (
    <Card
      hoverable
      className={`template-card ${template.isSystem ? 'system-template' : ''}`}
      actions={[
        <Button
          key="preview"
          type="link"
          icon={<EyeOutlined />}
          onClick={() => onPreview(template)}
        >
          预览
        </Button>,
        <Button
          key="use"
          type="link"
          icon={<PlayCircleOutlined />}
          onClick={() => onUse(template)}
        >
          使用
        </Button>,
      ]}
    >
      <Card.Meta
        avatar={
          <div
            style={{
              width: 48,
              height: 48,
              borderRadius: 8,
              backgroundColor: '#f0f5ff',
              display: 'flex',
              alignItems: 'center',
              justifyContent: 'center',
            }}
          >
            <FileTextOutlined style={{ fontSize: 24, color: '#1677ff' }} />
          </div>
        }
        title={
          <Space>
            <span className="template-name">{template.name}</span>
            {template.isSystem && <Tag color="gold">内置</Tag>}
          </Space>
        }
        description={
          <>
            <Paragraph
              ellipsis={{ rows: 2 }}
              style={{ marginBottom: 8, color: '#8c8c8c' }}
            >
              {template.description || '暂无描述'}
            </Paragraph>
            <Space size={4}>
              <Tag color={categoryConfig?.color}>{categoryConfig?.text}</Tag>
              {template.subcategory && <Tag>{template.subcategory}</Tag>}
              <Text type="secondary" style={{ fontSize: 12 }}>
                v{template.version}
              </Text>
            </Space>
          </>
        }
      />
    </Card>
  );
};

/**
 * 将模板 JSON 转换为 ReactFlow 节点和边
 */
function templateToFlowElements(templateJson: Record<string, unknown>): { nodes: Node[]; edges: Edge[] } {
  const nodes: Node[] = [];
  const edges: Edge[] = [];
  let nodeId = 0;

  // 解析规则根节点
  const root = templateJson.root as Record<string, unknown> | undefined;
  if (!root) {
    // 简单条件模板
    nodes.push({
      id: 'condition-1',
      type: 'default',
      position: { x: 100, y: 50 },
      data: { label: '条件节点' },
      style: { background: '#e6f7ff', border: '1px solid #1890ff', borderRadius: 8 },
    });
    nodes.push({
      id: 'badge-1',
      type: 'default',
      position: { x: 300, y: 50 },
      data: { label: '徽章节点' },
      style: { background: '#fffbe6', border: '1px solid #faad14', borderRadius: 8 },
    });
    edges.push({ id: 'e1', source: 'condition-1', target: 'badge-1' });
    return { nodes, edges };
  }

  // 递归解析节点
  function parseNode(node: Record<string, unknown>, x: number, y: number, _parentId?: string): string {
    const currentId = `node-${++nodeId}`;
    const type = node.type as string;

    if (type === 'group') {
      const operator = node.operator as string || 'and';
      nodes.push({
        id: currentId,
        type: 'default',
        position: { x, y },
        data: { label: operator.toUpperCase() },
        style: { background: '#f9f0ff', border: '1px solid #722ed1', borderRadius: 8, padding: '8px 16px' },
      });

      const children = node.children as Record<string, unknown>[] | undefined;
      if (children && Array.isArray(children)) {
        children.forEach((child, index) => {
          const childId = parseNode(child, x - 100 + index * 80, y + 80, currentId);
          edges.push({ id: `e-${currentId}-${childId}`, source: childId, target: currentId });
        });
      }
    } else if (type === 'condition') {
      const field = node.field as string || '字段';
      const operator = node.operator as string || '=';
      const value = node.value as string || '值';
      nodes.push({
        id: currentId,
        type: 'default',
        position: { x, y },
        data: { label: `${field} ${operator} ${value}` },
        style: { background: '#e6f7ff', border: '1px solid #1890ff', borderRadius: 8, padding: '8px 12px', fontSize: 12 },
      });
    }

    return currentId;
  }

  const rootId = parseNode(root, 150, 30);

  // 添加徽章节点
  const badgeId = `badge-${++nodeId}`;
  nodes.push({
    id: badgeId,
    type: 'default',
    position: { x: 150, y: nodes.length * 60 + 50 },
    data: { label: '发放徽章' },
    style: { background: '#fffbe6', border: '1px solid #faad14', borderRadius: 8 },
  });
  edges.push({ id: `e-${rootId}-${badgeId}`, source: rootId, target: badgeId });

  return { nodes, edges };
}

/**
 * 模板预览弹窗
 */
interface PreviewModalProps {
  template: RuleTemplate | null;
  open: boolean;
  onClose: () => void;
  onUse: (template: RuleTemplate) => void;
}

const PreviewModal: React.FC<PreviewModalProps> = ({
  template,
  open,
  onClose,
  onUse,
}) => {
  // 将模板转换为流程图节点
  const { nodes, edges } = useMemo(() => {
    if (!template?.templateJson) {
      return { nodes: [], edges: [] };
    }
    return templateToFlowElements(template.templateJson);
  }, [template?.templateJson]);

  if (!template) return null;

  return (
    <Modal
      title={`预览模板: ${template.name}`}
      open={open}
      onCancel={onClose}
      width={800}
      className="template-preview-modal"
      footer={[
        <Button key="close" onClick={onClose}>
          关闭
        </Button>,
        <Button key="use" type="primary" onClick={() => onUse(template)}>
          使用此模板
        </Button>,
      ]}
    >
      <div style={{ marginBottom: 16 }}>
        <Text strong>描述：</Text>
        <Paragraph style={{ marginTop: 8 }}>
          {template.description || '暂无描述'}
        </Paragraph>
      </div>

      {template.parameters && template.parameters.length > 0 && (
        <div style={{ marginBottom: 16 }}>
          <Text strong>可配置参数：</Text>
          <ul style={{ marginTop: 8 }}>
            {template.parameters.map((param) => (
              <li key={param.name}>
                <Text code>{param.name}</Text>
                <Text type="secondary"> - {param.label}</Text>
                {param.required && <Tag color="red" style={{ marginLeft: 8 }}>必填</Tag>}
                {param.description && (
                  <Text type="secondary" style={{ display: 'block', marginLeft: 16 }}>
                    {param.description}
                  </Text>
                )}
              </li>
            ))}
          </ul>
        </div>
      )}

      <div>
        <Text strong>规则预览：</Text>
        <div
          style={{
            marginTop: 8,
            borderRadius: 8,
            height: 250,
            border: '1px solid #f0f0f0',
          }}
        >
          <ReactFlowProvider>
            <ReactFlow
              nodes={nodes}
              edges={edges}
              fitView
              nodesDraggable={false}
              nodesConnectable={false}
              elementsSelectable={false}
              panOnDrag={false}
              zoomOnScroll={false}
              zoomOnPinch={false}
              zoomOnDoubleClick={false}
              preventScrolling={false}
            >
              <Background color="#f5f5f5" gap={16} />
            </ReactFlow>
          </ReactFlowProvider>
        </div>
      </div>
    </Modal>
  );
};

const TemplatesPage: React.FC = () => {
  const navigate = useNavigate();

  // 搜索和筛选状态
  const [searchKeyword, setSearchKeyword] = useState('');
  const [selectedCategory, setSelectedCategory] = useState<string>('all');

  // 预览弹窗状态
  const [previewTemplate, setPreviewTemplate] = useState<RuleTemplate | null>(null);
  const [previewOpen, setPreviewOpen] = useState(false);

  // 获取模板列表
  const { data, isLoading } = useQuery({
    queryKey: ['templates', selectedCategory],
    queryFn: () =>
      listTemplates(selectedCategory === 'all' ? undefined : selectedCategory),
  });

  // 过滤模板
  const filteredTemplates = React.useMemo(() => {
    if (!data?.items) return [];
    if (!searchKeyword) return data.items;

    const keyword = searchKeyword.toLowerCase();
    return data.items.filter(
      (t) =>
        t.name.toLowerCase().includes(keyword) ||
        t.description?.toLowerCase().includes(keyword) ||
        t.code.toLowerCase().includes(keyword)
    );
  }, [data?.items, searchKeyword]);

  /**
   * 预览模板
   */
  const handlePreview = (template: RuleTemplate) => {
    setPreviewTemplate(template);
    setPreviewOpen(true);
  };

  /**
   * 使用模板
   */
  const handleUse = (template: RuleTemplate) => {
    setPreviewOpen(false);
    navigate(`/rules/create?template=${template.code}`);
  };

  return (
    <PageContainer
      title="规则模板"
      extra={
        <Space>
          <Input
            placeholder="搜索模板"
            prefix={<SearchOutlined />}
            value={searchKeyword}
            onChange={(e) => setSearchKeyword(e.target.value)}
            style={{ width: 300 }}
            allowClear
          />
          <Button
            type="primary"
            icon={<PlusOutlined />}
            onClick={() => navigate('/rules/create')}
          >
            创建模板
          </Button>
        </Space>
      }
    >
      {/* 分类筛选 */}
      <div style={{ marginBottom: 24 }}>
        <Segmented
          value={selectedCategory}
          onChange={(value) => setSelectedCategory(value as string)}
          options={[
            { label: '全部', value: 'all' },
            { label: '基础模板', value: 'basic' },
            { label: '进阶模板', value: 'advanced' },
            { label: '行业模板', value: 'industry' },
          ]}
        />
      </div>

      {/* 模板列表 */}
      <Spin spinning={isLoading}>
        <div className="template-list">
          {filteredTemplates.length === 0 ? (
            <Empty description="暂无模板" />
          ) : (
            <Row gutter={[16, 16]}>
              {filteredTemplates.map((template) => (
                <Col key={template.id} xs={24} sm={12} md={8} lg={6}>
                  <TemplateCard
                    template={template}
                    onPreview={handlePreview}
                    onUse={handleUse}
                  />
                </Col>
              ))}
            </Row>
          )}
        </div>
      </Spin>

      {/* 预览弹窗 */}
      <PreviewModal
        template={previewTemplate}
        open={previewOpen}
        onClose={() => setPreviewOpen(false)}
        onUse={handleUse}
      />
    </PageContainer>
  );
};

export default TemplatesPage;
