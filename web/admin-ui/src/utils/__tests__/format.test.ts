import { describe, it, expect } from 'vitest';
import {
  formatDate,

  formatAmount,
  formatCount,
  formatPercent,
  getBadgeStatusText,
  getBadgeTypeText,
  getCategoryStatusText,
} from '../format';

describe('formatDate', () => {
  it('空值返回占位符', () => {
    expect(formatDate(null)).toBe('-');
    expect(formatDate(undefined)).toBe('-');
  });

  it('正确格式化 ISO 日期字符串', () => {
    const result = formatDate('2025-06-15T10:30:00Z', 'YYYY-MM-DD');
    expect(result).toBe('2025-06-15');
  });

  it('支持自定义格式', () => {
    const result = formatDate('2025-01-01T00:00:00Z', 'YYYY/MM/DD');
    expect(result).toBe('2025/01/01');
  });
});

describe('formatAmount', () => {
  it('空值返回占位符', () => {
    expect(formatAmount(null)).toBe('-');
    expect(formatAmount(undefined)).toBe('-');
  });

  it('格式化金额保留 2 位小数', () => {
    expect(formatAmount(1234.5)).toBe('1,234.50');
  });

  it('支持自定义小数位数', () => {
    expect(formatAmount(99.999, 1)).toBe('100.0');
  });
});

describe('formatCount', () => {
  it('空值返回占位符', () => {
    expect(formatCount(null)).toBe('-');
    expect(formatCount(undefined)).toBe('-');
  });

  it('小于 1000 直接显示', () => {
    expect(formatCount(999)).toBe('999');
  });

  it('千级使用 K 后缀', () => {
    expect(formatCount(1500)).toBe('1.5K');
  });

  it('百万级使用 M 后缀', () => {
    expect(formatCount(2_500_000)).toBe('2.5M');
  });

  it('十亿级使用 B 后缀', () => {
    expect(formatCount(3_000_000_000)).toBe('3.0B');
  });
});

describe('formatPercent', () => {
  it('空值返回占位符', () => {
    expect(formatPercent(null)).toBe('-');
  });

  it('将小数转为百分比', () => {
    expect(formatPercent(0.856)).toBe('85.6%');
  });

  it('支持自定义精度', () => {
    expect(formatPercent(0.3333, 2)).toBe('33.33%');
  });
});

describe('状态文本映射', () => {
  it('getBadgeStatusText 返回中文', () => {
    expect(getBadgeStatusText('ACTIVE')).toBe('已上线');
    expect(getBadgeStatusText('DRAFT')).toBe('草稿');
  });

  it('getBadgeTypeText 返回中文', () => {
    expect(getBadgeTypeText('NORMAL')).toBe('普通徽章');
    expect(getBadgeTypeText('LIMITED')).toBe('限定徽章');
  });

  it('getCategoryStatusText 返回中文', () => {
    expect(getCategoryStatusText('ACTIVE')).toBe('启用');
    expect(getCategoryStatusText('INACTIVE')).toBe('禁用');
  });

  it('未知状态回退为原始值', () => {
    expect(getBadgeStatusText('UNKNOWN' as any)).toBe('UNKNOWN');
  });
});
