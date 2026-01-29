/**
 * Ant Design 主题配置
 *
 * 定义徽章管理系统的品牌色、字体、间距等设计规范
 */

import type { ThemeConfig } from 'antd';

/**
 * 品牌色定义
 *
 * 主色采用蓝色系，符合企业管理后台的专业感
 */
export const brandColors = {
  /** 主色 - 品牌蓝 */
  primary: '#1677ff',
  /** 成功色 - 绿色 */
  success: '#52c41a',
  /** 警告色 - 橙色 */
  warning: '#faad14',
  /** 错误色 - 红色 */
  error: '#ff4d4f',
  /** 信息色 - 蓝色 */
  info: '#1677ff',
};

/**
 * 徽章状态色
 *
 * 与后端 BadgeStatus 枚举对应，用于状态标签展示
 */
export const badgeStatusColors: Record<string, string> = {
  DRAFT: '#d9d9d9',     // 草稿 - 灰色
  ACTIVE: '#52c41a',    // 已上线 - 绿色
  INACTIVE: '#faad14',  // 已下线 - 橙色
  ARCHIVED: '#8c8c8c',  // 已归档 - 深灰
};

/**
 * 会员等级色
 *
 * 与 MembershipLevel 对应的品牌色
 */
export const membershipColors: Record<string, string> = {
  Bronze: '#CD7F32',
  Silver: '#C0C0C0',
  Gold: '#FFD700',
  Platinum: '#E5E4E2',
  Diamond: '#B9F2FF',
};

/**
 * Ant Design 主题配置
 */
export const themeConfig: ThemeConfig = {
  token: {
    // 品牌色
    colorPrimary: brandColors.primary,
    colorSuccess: brandColors.success,
    colorWarning: brandColors.warning,
    colorError: brandColors.error,
    colorInfo: brandColors.info,

    // 字体
    fontFamily:
      "-apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, 'Helvetica Neue', Arial, 'Noto Sans', sans-serif, 'Apple Color Emoji', 'Segoe UI Emoji', 'Segoe UI Symbol', 'Noto Color Emoji'",
    fontSize: 14,
    fontSizeSM: 12,
    fontSizeLG: 16,
    fontSizeXL: 20,

    // 圆角
    borderRadius: 6,
    borderRadiusSM: 4,
    borderRadiusLG: 8,

    // 间距
    padding: 16,
    paddingSM: 12,
    paddingLG: 24,
    paddingXL: 32,

    // 线条
    lineWidth: 1,
    lineType: 'solid',

    // 动画
    motionDurationFast: '0.1s',
    motionDurationMid: '0.2s',
    motionDurationSlow: '0.3s',
  },
  components: {
    // 布局组件配置
    Layout: {
      headerBg: '#001529',
      siderBg: '#001529',
      bodyBg: '#f0f2f5',
    },
    // 菜单组件配置
    Menu: {
      darkItemBg: '#001529',
      darkSubMenuItemBg: '#000c17',
      darkItemSelectedBg: brandColors.primary,
    },
    // 表格组件配置
    Table: {
      headerBg: '#fafafa',
      headerColor: 'rgba(0, 0, 0, 0.85)',
      rowHoverBg: '#f5f5f5',
    },
    // 卡片组件配置
    Card: {
      headerBg: '#fafafa',
    },
    // 按钮组件配置
    Button: {
      primaryShadow: '0 2px 0 rgba(5, 145, 255, 0.1)',
    },
  },
};

export default themeConfig;
