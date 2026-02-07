/**
 * 测试数据生成工具
 *
 * 所有测试数据都通过 uniqueId / testRunPrefix 保证跨测试/跨并行运行的隔离性。
 * 前缀格式: e2e_{timestamp}_{random} — 方便事后按前缀批量清理残留数据。
 */

/** 自增计数器，同一进程内保证即使同毫秒调用也不会碰撞 */
let _seq = 0;

/**
 * 生成唯一 ID，组合时间戳 + 自增序号 + 随机串，三重保障唯一性
 */
export function uniqueId(prefix = ''): string {
  _seq += 1;
  return `${prefix}${Date.now()}_${_seq}_${Math.random().toString(36).slice(2, 8)}`;
}

/**
 * 生成当前测试运行的隔离前缀，每个 spec 文件调用一次即可，
 * 后续所有测试数据都基于此前缀创建和清理
 */
export function testRunPrefix(): string {
  return `e2e_${Date.now()}_${Math.random().toString(36).slice(2, 6)}_`;
}

/**
 * 测试用户数据
 */
export const testUsers = {
  admin: {
    username: 'admin',
    password: 'admin123',
    role: 'admin',
  },
  operator: {
    username: 'operator',
    password: 'operator123',
    role: 'operator',
  },
  viewer: {
    username: 'viewer',
    password: 'viewer123',
    role: 'viewer',
  },
};

/**
 * 测试徽章数据
 * 格式匹配后端 API 要求
 */
export function createTestBadge(overrides: Partial<TestBadge> = {}): TestBadge {
  return {
    name: uniqueId('徽章_'),
    description: '这是一个测试徽章',
    seriesId: 0, // 需要通过 ensureTestData 获取实际 ID
    badgeType: 'NORMAL',
    assets: {
      iconUrl: 'https://example.com/badge.png',
    },
    validityConfig: {
      validityType: 'PERMANENT',
    },
    ...overrides,
  };
}

export interface TestBadge {
  name: string;
  description?: string;
  seriesId: number;
  badgeType: 'NORMAL' | 'LIMITED' | 'ACHIEVEMENT' | 'EVENT';
  assets: {
    iconUrl: string;
    imageUrl?: string;
    animationUrl?: string;
    disabledIconUrl?: string;
  };
  validityConfig: {
    validityType: 'PERMANENT' | 'RELATIVE_DAYS' | 'FIXED_DATE';
    relativeDays?: number;
    fixedDate?: string;
  };
  maxSupply?: number;
}

/**
 * 测试规则数据
 */
export function createTestRule(overrides: Partial<TestRule> = {}): TestRule {
  return {
    name: uniqueId('规则_'),
    description: '这是一个测试规则',
    priority: 100,
    status: 'draft',
    conditions: [],
    actions: [],
    ...overrides,
  };
}

export interface TestRule {
  name: string;
  description: string;
  priority: number;
  status: string;
  conditions: any[];
  actions: any[];
}

/**
 * 创建测试权益数据
 *
 * 支持多种权益类型：积分、优惠券、会员等。
 * benefitType 字段用于适配不同的 API 版本。
 */
export function createTestBenefit(overrides: Partial<TestBenefit> = {}): TestBenefit {
  return {
    name: uniqueId('权益_'),
    code: uniqueId('ben_'),
    type: 'COUPON',
    benefitType: 'COUPON',
    value: 100,
    externalId: uniqueId('ext_'),
    description: '测试权益',
    validityDays: 30,
    ...overrides,
  };
}

export interface TestBenefit {
  name: string;
  code: string;
  type: string;
  value: number;
  externalId: string;
  description?: string;
  validityDays?: number;
  benefitType?: string;
}

/**
 * 等待指定时间
 */
export function sleep(ms: number): Promise<void> {
  return new Promise((resolve) => setTimeout(resolve, ms));
}

/**
 * 资源收集器：在测试过程中记录创建的资源 ID，teardown 阶段按反向依赖顺序批量清理。
 * 避免测试残留数据影响后续运行或其他并行测试。
 */
export class TestResourceCollector {
  private resources: Array<{ type: string; id: number }> = [];

  track(type: 'badge' | 'rule' | 'series' | 'category' | 'benefit' | 'redemptionRule', id: number): void {
    this.resources.push({ type, id });
  }

  /**
   * 返回按安全删除顺序排列的资源列表（先删依赖方，再删被依赖方）
   */
  getOrderedForCleanup(): Array<{ type: string; id: number }> {
    const order: Record<string, number> = {
      badge: 0,
      rule: 1,
      redemptionRule: 2,
      benefit: 3,
      series: 4,
      category: 5,
    };
    return [...this.resources]
      .sort((a, b) => (order[a.type] ?? 99) - (order[b.type] ?? 99));
  }

  clear(): void {
    this.resources = [];
  }
}
