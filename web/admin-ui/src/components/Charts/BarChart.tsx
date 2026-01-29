/**
 * 柱状图组件
 *
 * 基于 ECharts 封装的可复用柱状图，支持水平/垂直方向
 * 适用于排行榜、对比分析等场景
 */

import React, { useMemo } from 'react';
import ReactECharts from 'echarts-for-react';
import type { EChartsOption } from 'echarts';

export interface BarChartDataItem {
  /** 名称标签 */
  name: string;
  /** 数值 */
  value: number;
}

export interface BarChartProps {
  /** 柱状图数据 */
  data: BarChartDataItem[];
  /** 图表标题 */
  title?: string;
  /** 图表高度，默认 300px */
  height?: number;
  /** 是否显示加载状态 */
  loading?: boolean;
  /** 是否水平显示（条形图） */
  horizontal?: boolean;
  /** 柱子颜色 */
  color?: string;
  /** 是否显示渐变色 */
  gradient?: boolean;
  /** 是否显示数据标签 */
  showLabel?: boolean;
  /** 柱子圆角 */
  barRadius?: number;
}

const BarChart: React.FC<BarChartProps> = ({
  data,
  title,
  height = 300,
  loading = false,
  horizontal = false,
  color = '#1677ff',
  gradient = true,
  showLabel = true,
  barRadius = 4,
}) => {
  const option: EChartsOption = useMemo(() => {
    const names = data.map((item) => item.name);
    const values = data.map((item) => item.value);

    // 水平柱状图需要反转数据以使排名第一的在最上面
    const displayNames = horizontal ? [...names].reverse() : names;
    const displayValues = horizontal ? [...values].reverse() : values;

    const barColor = gradient
      ? {
          type: 'linear' as const,
          x: horizontal ? 0 : 0,
          y: horizontal ? 0 : 1,
          x2: horizontal ? 1 : 0,
          y2: horizontal ? 0 : 0,
          colorStops: [
            { offset: 0, color: `${color}` },
            { offset: 1, color: `${color}80` },
          ],
        }
      : color;

    const categoryAxis = {
      type: 'category' as const,
      data: displayNames,
      axisLine: {
        show: false,
      },
      axisTick: {
        show: false,
      },
      axisLabel: {
        color: '#595959',
        fontSize: 12,
        formatter: (value: string) => {
          // 名称过长时截断
          return value.length > 8 ? `${value.slice(0, 8)}...` : value;
        },
      },
    };

    const valueAxis = {
      type: 'value' as const,
      axisLine: {
        show: false,
      },
      axisTick: {
        show: false,
      },
      splitLine: {
        lineStyle: {
          color: '#f0f0f0',
          type: 'dashed' as const,
        },
      },
      axisLabel: {
        color: '#8c8c8c',
        fontSize: 12,
        formatter: (value: number) => {
          if (value >= 10000) {
            return `${(value / 10000).toFixed(1)}w`;
          }
          if (value >= 1000) {
            return `${(value / 1000).toFixed(1)}k`;
          }
          return value.toString();
        },
      },
    };

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
        trigger: 'axis',
        axisPointer: {
          type: 'shadow',
        },
        formatter: (params: unknown) => {
          const arr = params as Array<{ name: string; value: number; marker: string }>;
          if (arr && arr.length > 0) {
            const item = arr[0];
            return `${item.marker} ${item.name}<br/>数量: ${item.value.toLocaleString()}`;
          }
          return '';
        },
      },
      grid: {
        left: horizontal ? '3%' : '3%',
        right: showLabel ? '15%' : '4%',
        bottom: '3%',
        top: title ? 50 : 20,
        containLabel: true,
      },
      xAxis: horizontal ? valueAxis : categoryAxis,
      yAxis: horizontal ? categoryAxis : valueAxis,
      series: [
        {
          type: 'bar',
          data: displayValues,
          barWidth: horizontal ? 16 : '40%',
          itemStyle: {
            color: barColor,
            borderRadius: horizontal ? [0, barRadius, barRadius, 0] : [barRadius, barRadius, 0, 0],
          },
          emphasis: {
            itemStyle: {
              color: color,
              shadowBlur: 10,
              shadowColor: 'rgba(0, 0, 0, 0.1)',
            },
          },
          label: showLabel
            ? {
                show: true,
                position: horizontal ? 'right' : 'top',
                color: '#595959',
                fontSize: 12,
                formatter: (params: unknown) => {
                  const p = params as { value: number | number[] };
                  const val = Array.isArray(p.value) ? p.value[0] : p.value;
                  if (val >= 10000) {
                    return `${(val / 10000).toFixed(1)}w`;
                  }
                  if (val >= 1000) {
                    return `${(val / 1000).toFixed(1)}k`;
                  }
                  return val.toString();
                },
              }
            : undefined,
        },
      ],
    };
  }, [data, title, horizontal, color, gradient, showLabel, barRadius]);

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

export default BarChart;
