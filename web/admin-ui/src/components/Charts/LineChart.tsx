/**
 * 折线图组件
 *
 * 基于 ECharts 封装的可复用折线图，支持趋势数据展示
 * 适用于发放趋势、活跃度趋势等时序数据可视化
 */

import React, { useMemo } from 'react';
import ReactECharts from 'echarts-for-react';
import type { EChartsOption } from 'echarts';
import type { TrendData } from '@/types/dashboard';

export interface LineChartProps {
  /** 趋势数据数组 */
  data: TrendData[];
  /** 图表标题 */
  title?: string;
  /** X 轴数据字段名，默认 'date' */
  xField?: string;
  /** Y 轴数据字段名，默认 'value' */
  yField?: string;
  /** 图表高度，默认 300px */
  height?: number;
  /** 是否显示加载状态 */
  loading?: boolean;
  /** 是否平滑曲线 */
  smooth?: boolean;
  /** 是否显示数据区域 */
  showArea?: boolean;
  /** 线条颜色 */
  color?: string;
}

const LineChart: React.FC<LineChartProps> = ({
  data,
  title,
  xField = 'date',
  yField = 'value',
  height = 300,
  loading = false,
  smooth = true,
  showArea = true,
  color = '#1677ff',
}) => {
  const option: EChartsOption = useMemo(() => {
    const xAxisData = data.map((item) => item[xField as keyof TrendData] as string);
    const seriesData = data.map((item) => item[yField as keyof TrendData] as number);

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
          type: 'cross',
          label: {
            backgroundColor: '#6a7985',
          },
        },
        formatter: (params: unknown) => {
          const arr = params as Array<{ axisValue: string; value: number; marker: string }>;
          if (arr && arr.length > 0) {
            const item = arr[0];
            return `${item.axisValue}<br/>${item.marker} ${item.value.toLocaleString()}`;
          }
          return '';
        },
      },
      grid: {
        left: '3%',
        right: '4%',
        bottom: '3%',
        top: title ? 60 : 30,
        containLabel: true,
      },
      xAxis: {
        type: 'category',
        boundaryGap: false,
        data: xAxisData,
        axisLine: {
          lineStyle: {
            color: '#d9d9d9',
          },
        },
        axisLabel: {
          color: '#8c8c8c',
          fontSize: 12,
        },
      },
      yAxis: {
        type: 'value',
        axisLine: {
          show: false,
        },
        axisTick: {
          show: false,
        },
        splitLine: {
          lineStyle: {
            color: '#f0f0f0',
            type: 'dashed',
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
      },
      // 支持缩放
      dataZoom: [
        {
          type: 'inside',
          start: 0,
          end: 100,
        },
      ],
      series: [
        {
          type: 'line',
          data: seriesData,
          smooth,
          symbol: 'circle',
          symbolSize: 6,
          showSymbol: false,
          emphasis: {
            focus: 'series',
            itemStyle: {
              borderWidth: 2,
            },
          },
          lineStyle: {
            width: 2,
            color,
          },
          itemStyle: {
            color,
          },
          areaStyle: showArea
            ? {
                color: {
                  type: 'linear',
                  x: 0,
                  y: 0,
                  x2: 0,
                  y2: 1,
                  colorStops: [
                    { offset: 0, color: `${color}40` },
                    { offset: 1, color: `${color}05` },
                  ],
                },
              }
            : undefined,
        },
      ],
    };
  }, [data, title, xField, yField, smooth, showArea, color]);

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

export default LineChart;
