/**
 * 饼图组件
 *
 * 基于 ECharts 封装的可复用饼图，支持环形展示
 * 适用于徽章类型分布、占比统计等场景
 */

import React, { useMemo } from 'react';
import ReactECharts from 'echarts-for-react';
import type { EChartsOption } from 'echarts';

export interface PieChartDataItem {
  /** 名称标签 */
  name: string;
  /** 数值 */
  value: number;
}

export interface PieChartProps {
  /** 饼图数据 */
  data: PieChartDataItem[];
  /** 图表标题 */
  title?: string;
  /** 图表高度，默认 300px */
  height?: number;
  /** 是否显示加载状态 */
  loading?: boolean;
  /** 是否显示为环形图 */
  donut?: boolean;
  /** 内圈半径百分比，仅环形图时生效 */
  innerRadius?: string;
  /** 外圈半径百分比 */
  outerRadius?: string;
  /** 颜色主题 */
  colors?: string[];
  /** 图例位置 */
  legendPosition?: 'left' | 'right' | 'top' | 'bottom';
}

const DEFAULT_COLORS = [
  '#1677ff',
  '#52c41a',
  '#faad14',
  '#eb2f96',
  '#722ed1',
  '#13c2c2',
  '#fa541c',
  '#2f54eb',
  '#a0d911',
  '#f5222d',
];

const PieChart: React.FC<PieChartProps> = ({
  data,
  title,
  height = 300,
  loading = false,
  donut = true,
  innerRadius = '45%',
  outerRadius = '70%',
  colors = DEFAULT_COLORS,
  legendPosition = 'right',
}) => {
  const option: EChartsOption = useMemo(() => {
    const isVertical = legendPosition === 'left' || legendPosition === 'right';
    const legendConfig = {
      left: legendPosition === 'left' ? 'left' : legendPosition === 'right' ? undefined : 'center',
      right: legendPosition === 'right' ? '5%' : undefined,
      top: legendPosition === 'top' ? 'top' : legendPosition === 'bottom' ? undefined : 'middle',
      bottom: legendPosition === 'bottom' ? 'bottom' : undefined,
      orient: (isVertical ? 'vertical' : 'horizontal') as 'vertical' | 'horizontal',
    };

    // 根据图例位置调整饼图中心位置
    const centerX = legendPosition === 'right' ? '40%' : legendPosition === 'left' ? '60%' : '50%';

    return {
      title: title
        ? {
            text: title,
            left: 'left',
            textStyle: {
              fontSize: 14,
              fontWeight: 500,
            },
          }
        : undefined,
      tooltip: {
        trigger: 'item',
        formatter: (params: unknown) => {
          const item = params as { name: string; value: number; percent: number; marker: string };
          return `${item.marker} ${item.name}<br/>数量: ${item.value.toLocaleString()}<br/>占比: ${item.percent.toFixed(1)}%`;
        },
      },
      legend: {
        ...legendConfig,
        type: 'scroll',
        pageButtonPosition: 'end',
        formatter: (name: string) => {
          // 名称过长时截断
          return name.length > 10 ? `${name.slice(0, 10)}...` : name;
        },
        textStyle: {
          color: '#595959',
          fontSize: 12,
        },
      },
      color: colors,
      series: [
        {
          type: 'pie',
          radius: donut ? [innerRadius, outerRadius] : outerRadius,
          center: [centerX, '55%'],
          avoidLabelOverlap: true,
          itemStyle: {
            borderRadius: donut ? 6 : 4,
            borderColor: '#fff',
            borderWidth: 2,
          },
          label: {
            show: false,
            position: 'center',
          },
          emphasis: {
            label: {
              show: true,
              fontSize: 16,
              fontWeight: 'bold',
              formatter: '{b}\n{d}%',
            },
            itemStyle: {
              shadowBlur: 10,
              shadowOffsetX: 0,
              shadowColor: 'rgba(0, 0, 0, 0.2)',
            },
          },
          labelLine: {
            show: false,
          },
          data: data.map((item) => ({
            name: item.name,
            value: item.value,
          })),
        },
      ],
    };
  }, [data, title, donut, innerRadius, outerRadius, colors, legendPosition]);

  return (
    <ReactECharts
      option={option}
      style={{ height }}
      showLoading={loading}
      opts={{ renderer: 'svg' }}
      notMerge
    />
  );
};

export default PieChart;
