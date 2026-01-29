/**
 * 页面切换动画组件
 *
 * 使用 CSS transition 实现淡入效果，提升页面切换的视觉体验
 * 相比 framer-motion 更轻量，满足基本的过渡需求
 */

import React, { useEffect, useState, type ReactNode } from 'react';
import { useLocation } from 'react-router-dom';

interface PageTransitionProps {
  children: ReactNode;
}

/**
 * 动画样式配置
 *
 * 使用 opacity 和 transform 实现平滑的淡入上移效果
 */
const transitionStyles: Record<string, React.CSSProperties> = {
  entering: {
    opacity: 0,
    transform: 'translateY(8px)',
  },
  entered: {
    opacity: 1,
    transform: 'translateY(0)',
  },
};

const PageTransition: React.FC<PageTransitionProps> = ({ children }) => {
  const location = useLocation();
  const [transitionState, setTransitionState] = useState<'entering' | 'entered'>('entered');

  // 路由变化时触发进入动画
  useEffect(() => {
    setTransitionState('entering');

    // 使用 requestAnimationFrame 确保初始状态被渲染后再切换到最终状态
    const frameId = requestAnimationFrame(() => {
      requestAnimationFrame(() => {
        setTransitionState('entered');
      });
    });

    return () => cancelAnimationFrame(frameId);
  }, [location.pathname]);

  return (
    <div
      style={{
        ...transitionStyles[transitionState],
        transition: 'opacity 0.25s ease-out, transform 0.25s ease-out',
        willChange: 'opacity, transform',
      }}
    >
      {children}
    </div>
  );
};

export default PageTransition;
