/**
 * 规则相关类型定义
 *
 * 对应后端规则引擎的配置结构，
 * 用于可视化规则编辑器（基于 React Flow）
 */

/**
 * 规则定义
 *
 * 定义用户获取徽章的触发条件，与规则引擎配合使用
 */
export interface Rule {
  id: number;
  /** 关联的徽章 ID */
  badgeId: number;
  /** 关联的徽章名称（列表展示用） */
  badgeName: string;
  /** 规则定义（传给规则引擎的 JSON） */
  ruleJson: RuleDefinition;
  /** 规则生效开始时间 */
  startTime?: string;
  /** 规则生效结束时间 */
  endTime?: string;
  /** 单用户最大获取数量 */
  maxCountPerUser?: number;
  /** 规则是否启用 */
  enabled: boolean;
  createdAt: string;
  updatedAt: string;
}

/**
 * 规则定义（传给规则引擎）
 *
 * 使用 JSON 格式存储规则的条件和动作配置
 */
export interface RuleDefinition {
  /** 规则类型（event: 事件触发, scheduled: 定时任务） */
  type: 'event' | 'scheduled';
  /** 条件节点列表 */
  conditions: RuleCondition[];
  /** 条件组合方式 */
  operator: 'AND' | 'OR';
  /** 动作配置 */
  actions: RuleAction[];
}

/**
 * 规则条件
 *
 * 定义单个判断条件
 */
export interface RuleCondition {
  /** 条件 ID（画布中用于连接） */
  id: string;
  /** 条件类型 */
  type: ConditionType;
  /** 字段路径（如 event.type, user.level） */
  field: string;
  /** 比较操作符 */
  operator: ComparisonOperator;
  /** 比较值 */
  value: string | number | boolean | string[];
  /** 条件描述（用于展示） */
  description?: string;
}

/**
 * 条件类型
 */
export type ConditionType =
  | 'event'        // 事件属性
  | 'user'         // 用户属性
  | 'order'        // 订单属性
  | 'time'         // 时间条件
  | 'aggregate';   // 聚合统计

/**
 * 比较操作符
 */
export type ComparisonOperator =
  | 'eq'           // 等于
  | 'ne'           // 不等于
  | 'gt'           // 大于
  | 'gte'          // 大于等于
  | 'lt'           // 小于
  | 'lte'          // 小于等于
  | 'in'           // 包含于
  | 'notIn'        // 不包含于
  | 'contains'     // 字符串包含
  | 'startsWith'   // 以...开头
  | 'endsWith';    // 以...结尾

/**
 * 规则动作
 *
 * 定义条件满足后的执行操作
 */
export interface RuleAction {
  /** 动作 ID */
  id: string;
  /** 动作类型 */
  type: ActionType;
  /** 动作参数 */
  params: Record<string, unknown>;
}

/**
 * 动作类型
 */
export type ActionType =
  | 'grant_badge'     // 发放徽章
  | 'send_message'    // 发送通知
  | 'add_points';     // 增加积分

// ============ 规则画布节点类型 ============

/**
 * 画布节点类型
 *
 * 用于 React Flow 的节点分类
 */
export type RuleNodeType =
  | 'trigger'        // 触发器节点（入口）
  | 'condition'      // 条件节点
  | 'action'         // 动作节点
  | 'branch';        // 分支节点（AND/OR）

/**
 * 画布节点数据
 *
 * React Flow 节点的自定义数据结构
 */
export interface RuleNodeData {
  /** 节点标签 */
  label: string;
  /** 节点类型 */
  nodeType: RuleNodeType;
  /** 条件配置（条件节点专用） */
  condition?: RuleCondition;
  /** 动作配置（动作节点专用） */
  action?: RuleAction;
  /** 分支操作符（分支节点专用） */
  branchOperator?: 'AND' | 'OR';
  /** 触发事件类型（触发器节点专用） */
  triggerType?: string;
}

// ============ 表单数据类型 ============

/**
 * 创建规则请求
 */
export interface CreateRuleRequest {
  badgeId: number;
  ruleJson: RuleDefinition;
  startTime?: string;
  endTime?: string;
  maxCountPerUser?: number;
}

/**
 * 更新规则请求
 */
export interface UpdateRuleRequest {
  ruleJson?: RuleDefinition;
  startTime?: string;
  endTime?: string;
  maxCountPerUser?: number;
  enabled?: boolean;
}

/**
 * 规则测试请求
 */
export interface TestRuleRequest {
  /** 测试用的事件数据 */
  eventData: Record<string, unknown>;
  /** 测试用的用户数据 */
  userData?: Record<string, unknown>;
}

/**
 * 规则测试结果
 */
export interface TestRuleResult {
  /** 是否匹配 */
  matched: boolean;
  /** 匹配的条件描述 */
  matchedConditions: string[];
  /** 评估耗时（毫秒） */
  evaluationTimeMs: number;
}
