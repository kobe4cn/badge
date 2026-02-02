import { Page, expect } from '@playwright/test';
import { BasePage } from './BasePage';

/**
 * 规则编辑器（画布）页面对象
 *
 * 基于 React Flow 的规则可视化编辑器。
 */
export class RuleEditorPage extends BasePage {
  // 画布区域
  readonly canvas = this.page.locator('.react-flow');
  readonly canvasViewport = this.page.locator('.react-flow__viewport');

  // 节点面板 - 节点类型区域
  readonly nodePanel = this.page.locator('.node-panel, [class*="node-panel"], [class*="节点类型"]').first();
  readonly conditionNode = this.page.locator('text=条件节点').first();
  readonly actionNode = this.page.locator('text=动作节点').first();
  readonly combinerNode = this.page.locator('text=逻辑组节点').first();

  // 工具栏 - 使用更具体的选择器
  readonly addNodeButton = this.page.locator('button:has-text("添加节点")');
  readonly saveButton = this.page.getByRole('button', { name: /保存/, exact: false }).first();
  readonly testButton = this.page.locator('button:has-text("测试")');
  readonly publishButton = this.page.locator('button:has-text("发布")');
  readonly undoButton = this.page.locator('button[title="撤销"], button:has([class*="undo"])').first();
  readonly redoButton = this.page.locator('button[title="重做"], button:has([class*="redo"])').first();

  // 节点配置面板
  readonly configPanel = this.page.locator('.config-panel, [class*="config-panel"]');

  constructor(page: Page) {
    super(page);
  }

  /**
   * 导航到规则编辑器
   */
  async goto(ruleId?: number): Promise<void> {
    if (ruleId) {
      await this.page.goto(`/rules/${ruleId}/edit`);
    } else {
      await this.page.goto('/rules/create');
    }
    await this.waitForPageLoad();
    await this.waitForCanvasReady();
  }

  /**
   * 等待画布加载完成
   */
  async waitForCanvasReady(): Promise<void> {
    await this.canvas.waitFor({ state: 'visible', timeout: 15000 });
    // 等待 React Flow 初始化
    await this.page.waitForTimeout(500);
  }

  /**
   * 从节点面板拖拽节点到画布
   */
  async dragNodeToCanvas(
    nodeType: 'condition' | 'action' | 'combiner',
    targetX: number,
    targetY: number
  ): Promise<void> {
    const nodeSelector = {
      condition: this.conditionNode,
      action: this.actionNode,
      combiner: this.combinerNode,
    };

    const node = nodeSelector[nodeType];
    const canvasBox = await this.canvas.boundingBox();

    if (!canvasBox) {
      throw new Error('Canvas not found');
    }

    // 确保节点可见
    await node.waitFor({ state: 'visible', timeout: 5000 });

    await node.dragTo(this.canvas, {
      targetPosition: {
        x: targetX,
        y: targetY,
      },
    });
  }

  /**
   * 通过点击"添加节点"按钮添加节点
   */
  async addNodeViaButton(nodeType: 'condition' | 'action' | 'combiner'): Promise<void> {
    await this.addNodeButton.click();
    // 等待节点类型选择弹窗
    await this.page.waitForTimeout(300);

    const nodeLabels = {
      condition: '条件节点',
      action: '动作节点',
      combiner: '逻辑组节点',
    };

    await this.page.locator(`text=${nodeLabels[nodeType]}`).click();
  }

  /**
   * 选择画布上的节点
   */
  async selectNode(nodeId: string): Promise<void> {
    await this.page.locator(`[data-id="${nodeId}"]`).click();
  }

  /**
   * 连接两个节点
   */
  async connectNodes(sourceId: string, targetId: string): Promise<void> {
    const sourceHandle = this.page.locator(
      `[data-id="${sourceId}"] .react-flow__handle-bottom`
    );
    const targetHandle = this.page.locator(
      `[data-id="${targetId}"] .react-flow__handle-top`
    );

    await sourceHandle.dragTo(targetHandle);
  }

  /**
   * 配置条件节点
   */
  async configureCondition(options: {
    field: string;
    operator: string;
    value: string;
  }): Promise<void> {
    await this.selectOption('字段', options.field);
    await this.selectOption('操作符', options.operator);
    await this.fillFormItem('值', options.value);
  }

  /**
   * 配置动作节点
   */
  async configureAction(options: {
    actionType: string;
    badgeId?: string;
    benefitId?: string;
  }): Promise<void> {
    await this.selectOption('动作类型', options.actionType);
    if (options.badgeId) {
      await this.selectOption('徽章', options.badgeId);
    }
    if (options.benefitId) {
      await this.selectOption('权益', options.benefitId);
    }
  }

  /**
   * 配置组合节点
   */
  async configureCombiner(logicType: 'AND' | 'OR'): Promise<void> {
    await this.selectOption('逻辑类型', logicType);
  }

  /**
   * 保存规则
   */
  async save(): Promise<void> {
    await this.saveButton.click();
    await this.waitForMessage('success');
  }

  /**
   * 预览规则
   */
  async preview(): Promise<void> {
    await this.testButton.click();
    await this.page.locator('.ant-modal, .preview-modal').waitFor({ state: 'visible' });
  }

  /**
   * 发布规则
   */
  async publish(): Promise<void> {
    await this.publishButton.click();
    await this.confirmModal();
    await this.waitForMessage('success');
  }

  /**
   * 获取画布上的节点数量
   */
  async getNodeCount(): Promise<number> {
    return await this.page.locator('.react-flow__node').count();
  }

  /**
   * 获取画布上的连线数量
   */
  async getEdgeCount(): Promise<number> {
    return await this.page.locator('.react-flow__edge').count();
  }

  /**
   * 验证节点存在
   */
  async expectNodeExists(nodeId: string): Promise<void> {
    await expect(this.page.locator(`[data-id="${nodeId}"]`)).toBeVisible();
  }

  /**
   * 验证连线存在
   */
  async expectEdgeExists(sourceId: string, targetId: string): Promise<void> {
    const edgeId = `reactflow__edge-${sourceId}-${targetId}`;
    await expect(this.page.locator(`[data-testid="${edgeId}"]`)).toBeVisible();
  }

  /**
   * 缩放画布
   */
  async zoom(level: number): Promise<void> {
    await this.page.keyboard.down('Control');
    if (level > 1) {
      await this.page.mouse.wheel(0, -100 * (level - 1));
    } else {
      await this.page.mouse.wheel(0, 100 * (1 - level));
    }
    await this.page.keyboard.up('Control');
  }

  /**
   * 平移画布
   */
  async pan(deltaX: number, deltaY: number): Promise<void> {
    const canvasBox = await this.canvas.boundingBox();
    if (!canvasBox) return;

    const centerX = canvasBox.x + canvasBox.width / 2;
    const centerY = canvasBox.y + canvasBox.height / 2;

    await this.page.mouse.move(centerX, centerY);
    await this.page.mouse.down({ button: 'middle' });
    await this.page.mouse.move(centerX + deltaX, centerY + deltaY);
    await this.page.mouse.up({ button: 'middle' });
  }
}
