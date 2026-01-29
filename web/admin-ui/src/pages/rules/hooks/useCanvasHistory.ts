/**
 * 画布历史记录管理 Hook
 *
 * 实现撤销/重做功能，记录画布状态变化历史
 */

import { useState, useCallback, useRef } from 'react';
import type { Node, Edge } from '@xyflow/react';
import { cloneDeep } from 'lodash-es';

/**
 * 历史记录项
 */
interface HistoryItem {
  nodes: Node[];
  edges: Edge[];
  timestamp: number;
}

/**
 * 历史记录配置
 */
interface UseCanvasHistoryOptions {
  /** 最大历史记录数量，超出后丢弃最早的记录 */
  maxHistory?: number;
}

/**
 * 历史记录管理返回值
 */
interface UseCanvasHistoryReturn {
  /** 是否可以撤销 */
  canUndo: boolean;
  /** 是否可以重做 */
  canRedo: boolean;
  /** 撤销操作 */
  undo: () => { nodes: Node[]; edges: Edge[] } | null;
  /** 重做操作 */
  redo: () => { nodes: Node[]; edges: Edge[] } | null;
  /** 记录当前状态到历史 */
  pushHistory: (nodes: Node[], edges: Edge[]) => void;
  /** 清空历史记录 */
  clearHistory: () => void;
  /** 历史记录长度 */
  historyLength: number;
  /** 当前位置 */
  currentIndex: number;
}

/**
 * 画布历史记录管理 Hook
 *
 * 使用栈结构维护历史状态，支持多步撤销和重做
 */
export function useCanvasHistory(
  options: UseCanvasHistoryOptions = {}
): UseCanvasHistoryReturn {
  const { maxHistory = 50 } = options;

  // 历史记录栈
  const [history, setHistory] = useState<HistoryItem[]>([]);
  // 当前位置索引（指向最后一次操作的位置）
  const [currentIndex, setCurrentIndex] = useState(-1);

  // 使用 ref 防止频繁触发导致的重复记录
  const lastPushTime = useRef(0);
  const DEBOUNCE_MS = 100;

  /**
   * 记录状态到历史
   *
   * 当用户执行操作时调用，保存当前画布状态
   */
  const pushHistory = useCallback(
    (nodes: Node[], edges: Edge[]) => {
      const now = Date.now();
      // 防抖：短时间内多次调用只记录最后一次
      if (now - lastPushTime.current < DEBOUNCE_MS) {
        return;
      }
      lastPushTime.current = now;

      const newItem: HistoryItem = {
        nodes: cloneDeep(nodes),
        edges: cloneDeep(edges),
        timestamp: now,
      };

      setHistory((prev) => {
        // 如果当前不在历史末尾，丢弃后面的记录（重做历史）
        const truncated = prev.slice(0, currentIndex + 1);
        // 添加新记录
        const newHistory = [...truncated, newItem];
        // 限制最大长度
        if (newHistory.length > maxHistory) {
          return newHistory.slice(-maxHistory);
        }
        return newHistory;
      });

      setCurrentIndex((prev) => {
        const newIndex = Math.min(prev + 1, maxHistory - 1);
        return newIndex;
      });
    },
    [currentIndex, maxHistory]
  );

  /**
   * 撤销操作
   *
   * 返回上一个历史状态
   */
  const undo = useCallback((): { nodes: Node[]; edges: Edge[] } | null => {
    if (currentIndex <= 0) return null;

    const prevIndex = currentIndex - 1;
    const prevState = history[prevIndex];

    if (prevState) {
      setCurrentIndex(prevIndex);
      return {
        nodes: cloneDeep(prevState.nodes),
        edges: cloneDeep(prevState.edges),
      };
    }

    return null;
  }, [currentIndex, history]);

  /**
   * 重做操作
   *
   * 恢复到下一个历史状态
   */
  const redo = useCallback((): { nodes: Node[]; edges: Edge[] } | null => {
    if (currentIndex >= history.length - 1) return null;

    const nextIndex = currentIndex + 1;
    const nextState = history[nextIndex];

    if (nextState) {
      setCurrentIndex(nextIndex);
      return {
        nodes: cloneDeep(nextState.nodes),
        edges: cloneDeep(nextState.edges),
      };
    }

    return null;
  }, [currentIndex, history]);

  /**
   * 清空历史记录
   */
  const clearHistory = useCallback(() => {
    setHistory([]);
    setCurrentIndex(-1);
  }, []);

  return {
    canUndo: currentIndex > 0,
    canRedo: currentIndex < history.length - 1,
    undo,
    redo,
    pushHistory,
    clearHistory,
    historyLength: history.length,
    currentIndex,
  };
}

export default useCanvasHistory;
