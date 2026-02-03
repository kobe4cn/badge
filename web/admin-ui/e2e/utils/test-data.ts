/**
 * 测试数据生成工具
 */

/**
 * 生成唯一 ID
 */
export function uniqueId(prefix = ''): string {
  return `${prefix}${Date.now()}_${Math.random().toString(36).slice(2, 8)}`;
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
