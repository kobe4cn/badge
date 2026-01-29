/**
 * 画布键盘快捷键 Hook
 *
 * 处理画布中的键盘事件，支持删除、撤销、重做、保存等操作
 */

import { useEffect, useCallback } from 'react';

/**
 * 快捷键配置
 */
export interface HotkeyHandlers {
  /** 删除选中节点 */
  onDelete?: () => void;
  /** 撤销 */
  onUndo?: () => void;
  /** 重做 */
  onRedo?: () => void;
  /** 保存 */
  onSave?: () => void;
  /** 全选 */
  onSelectAll?: () => void;
  /** 复制 */
  onCopy?: () => void;
  /** 粘贴 */
  onPaste?: () => void;
}

/**
 * 画布快捷键 Hook
 *
 * 监听键盘事件并调用相应的处理函数
 */
export function useCanvasHotkeys(handlers: HotkeyHandlers): void {
  const handleKeyDown = useCallback(
    (event: KeyboardEvent) => {
      const target = event.target as HTMLElement;

      // 如果焦点在输入框内，不处理快捷键（除了 Escape）
      const isInputFocused =
        target.tagName === 'INPUT' ||
        target.tagName === 'TEXTAREA' ||
        target.isContentEditable;

      if (isInputFocused && event.key !== 'Escape') {
        return;
      }

      const isMac = navigator.platform.toUpperCase().indexOf('MAC') >= 0;
      const ctrlOrCmd = isMac ? event.metaKey : event.ctrlKey;

      // Delete / Backspace - 删除选中节点
      if ((event.key === 'Delete' || event.key === 'Backspace') && !ctrlOrCmd) {
        event.preventDefault();
        handlers.onDelete?.();
        return;
      }

      // Ctrl/Cmd + Z - 撤销
      if (ctrlOrCmd && event.key === 'z' && !event.shiftKey) {
        event.preventDefault();
        handlers.onUndo?.();
        return;
      }

      // Ctrl/Cmd + Y 或 Ctrl/Cmd + Shift + Z - 重做
      if (ctrlOrCmd && (event.key === 'y' || (event.key === 'z' && event.shiftKey))) {
        event.preventDefault();
        handlers.onRedo?.();
        return;
      }

      // Ctrl/Cmd + S - 保存
      if (ctrlOrCmd && event.key === 's') {
        event.preventDefault();
        handlers.onSave?.();
        return;
      }

      // Ctrl/Cmd + A - 全选
      if (ctrlOrCmd && event.key === 'a') {
        event.preventDefault();
        handlers.onSelectAll?.();
        return;
      }

      // Ctrl/Cmd + C - 复制
      if (ctrlOrCmd && event.key === 'c') {
        event.preventDefault();
        handlers.onCopy?.();
        return;
      }

      // Ctrl/Cmd + V - 粘贴
      if (ctrlOrCmd && event.key === 'v') {
        event.preventDefault();
        handlers.onPaste?.();
        return;
      }
    },
    [handlers]
  );

  useEffect(() => {
    document.addEventListener('keydown', handleKeyDown);
    return () => {
      document.removeEventListener('keydown', handleKeyDown);
    };
  }, [handleKeyDown]);
}

export default useCanvasHotkeys;
