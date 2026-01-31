/**
 * 规则模板 API 服务
 *
 * 封装模板列表、详情、预览和基于模板创建规则的接口
 */

import { get, post } from './api';

/**
 * 模板参数定义
 *
 * 描述模板中可配置参数的元信息，用于动态生成参数表单
 */
export interface ParameterDef {
  /** 参数名称（用于 JSON 中的键） */
  name: string;
  /** 参数类型 */
  type: 'string' | 'number' | 'boolean' | 'date' | 'array' | 'enum';
  /** 显示标签 */
  label: string;
  /** 参数说明 */
  description?: string;
  /** 默认值 */
  default?: unknown;
  /** 是否必填 */
  required: boolean;
  /** 数值类型最小值 */
  min?: number;
  /** 数值类型最大值 */
  max?: number;
  /** 枚举/选择类型的选项列表 */
  options?: { value: unknown; label: string }[];
}

/**
 * 规则模板实体
 *
 * 预定义的规则配置，支持参数化生成具体规则
 */
export interface RuleTemplate {
  id: number;
  /** 模板唯一编码 */
  code: string;
  /** 模板名称 */
  name: string;
  /** 模板描述 */
  description?: string;
  /** 模板分类 */
  category: 'basic' | 'advanced' | 'industry';
  /** 子分类（如电商、游戏等） */
  subcategory?: string;
  /** 模板 JSON 定义 */
  templateJson: Record<string, unknown>;
  /** 参数定义列表 */
  parameters: ParameterDef[];
  /** 版本号 */
  version: string;
  /** 是否系统内置模板 */
  isSystem: boolean;
  /** 是否启用 */
  enabled: boolean;
}

/**
 * 模板列表响应
 */
export interface TemplateListResponse {
  items: RuleTemplate[];
  total: number;
}

/**
 * 模板预览响应
 *
 * 返回参数替换后生成的规则 JSON
 */
export interface PreviewResponse {
  ruleJson: Record<string, unknown>;
}

/**
 * 基于模板创建规则请求
 */
export interface CreateRuleFromTemplateRequest {
  /** 模板编码 */
  templateCode: string;
  /** 关联的徽章 ID */
  badgeId: number;
  /** 模板参数值 */
  params: Record<string, unknown>;
  /** 是否启用规则 */
  enabled?: boolean;
}

/**
 * 基于模板创建规则响应
 */
export interface CreateRuleFromTemplateResponse {
  id: number;
  ruleJson: Record<string, unknown>;
}

/**
 * 获取模板列表
 *
 * @param category - 按分类筛选
 * @param enabledOnly - 是否只返回启用的模板
 */
export function listTemplates(
  category?: string,
  enabledOnly = true
): Promise<TemplateListResponse> {
  const params: Record<string, unknown> = { enabled_only: enabledOnly };
  if (category) {
    params.category = category;
  }
  return get<TemplateListResponse>('/admin/templates', params);
}

/**
 * 获取模板详情
 *
 * @param code - 模板编码
 */
export function getTemplate(code: string): Promise<RuleTemplate> {
  return get<RuleTemplate>(`/admin/templates/${code}`);
}

/**
 * 预览模板生成的规则
 *
 * 使用给定参数替换模板变量，返回最终的规则 JSON 配置
 *
 * @param code - 模板编码
 * @param params - 参数值
 */
export function previewTemplate(
  code: string,
  params: Record<string, unknown>
): Promise<PreviewResponse> {
  return post<PreviewResponse>(`/admin/templates/${code}/preview`, { params });
}

/**
 * 基于模板创建规则
 *
 * 使用模板和参数生成规则，并关联到指定徽章
 */
export function createRuleFromTemplate(
  request: CreateRuleFromTemplateRequest
): Promise<CreateRuleFromTemplateResponse> {
  return post<CreateRuleFromTemplateResponse>('/admin/rules/from-template', request);
}

/**
 * 模板服务对象
 */
export const templateService = {
  list: listTemplates,
  get: getTemplate,
  preview: previewTemplate,
  createRule: createRuleFromTemplate,
};
