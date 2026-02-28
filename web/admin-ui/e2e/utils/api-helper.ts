import { APIRequestContext } from '@playwright/test';
import { TestResourceCollector } from './test-data';

/**
 * API 辅助工具
 *
 * 用于在测试中直接调用后端 API。
 */
export class ApiHelper {
  private request: APIRequestContext;
  private baseUrl: string;
  private token?: string;

  constructor(request: APIRequestContext, baseUrl: string) {
    this.request = request;
    this.baseUrl = baseUrl;
  }

  /**
   * 设置认证 token
   */
  setToken(token: string): void {
    this.token = token;
  }

  /**
   * 获取请求头
   */
  private getHeaders(): Record<string, string> {
    const headers: Record<string, string> = {
      'Content-Type': 'application/json',
    };
    if (this.token) {
      headers['Authorization'] = `Bearer ${this.token}`;
    }
    return headers;
  }

  /**
   * 解析 JSON 响应并在失败时抛出异常，确保测试数据准备阶段的错误不会被静默吞掉
   */
  private async safeJson(response: any, context: string = ''): Promise<any> {
    if (!response.ok()) {
      const text = await response.text();
      throw new Error(`API Error [${response.status()}] ${context}: ${text.substring(0, 200)}`);
    }
    try {
      return await response.json();
    } catch (e) {
      const text = await response.text();
      throw new Error(`JSON Parse Error ${context}: ${text.substring(0, 200)}`);
    }
  }

  /**
   * 宽松版本的 JSON 解析，用于清理、查询等允许失败的场景
   */
  private async safeJsonSoft(response: any, context: string = ''): Promise<any> {
    if (!response.ok()) {
      const text = await response.text();
      console.warn(`API Warning [${response.status()}] ${context}: ${text.substring(0, 200)}`);
      return { success: false, error: text, status: response.status() };
    }
    try {
      return await response.json();
    } catch (e) {
      const text = await response.text();
      console.warn(`JSON Parse Warning ${context}: ${text.substring(0, 200)}`);
      return { success: false, error: text, status: response.status() };
    }
  }

  /**
   * 登录并获取 token
   *
   * 兼容两种后端响应格式：
   * - 嵌套格式: { success: true, data: { token: "jwt...", user: {...}, permissions: [...] } }
   * - 扁平格式: { token: "jwt..." }
   */
  async login(username: string, password: string): Promise<string> {
    const response = await this.request.post(`${this.baseUrl}/api/admin/auth/login`, {
      headers: this.getHeaders(),
      data: { username, password },
    });
    const data = await this.safeJson(response, 'login');
    // 优先从标准嵌套格式提取，回退到扁平格式
    this.token = data.data?.token || data.token;

    // 种子用户首次登录需要修改默认密码，自动完成密码变更以便后续测试流程继续
    const mustChange = data.data?.mustChangePassword || data.mustChangePassword;
    if (mustChange && this.token) {
      const newPassword = `${password}_changed`;
      await this.request.post(`${this.baseUrl}/api/admin/auth/change-password`, {
        headers: { ...this.getHeaders(), 'Authorization': `Bearer ${this.token}` },
        data: { oldPassword: password, newPassword },
      });
      // 用新密码重新登录以获取不含 must_change_password 标记的 token
      return this.login(username, newPassword);
    }

    return this.token || '';
  }

  // ===== 分类 API =====

  /**
   * 创建分类
   */
  async createCategory(category: any): Promise<any> {
    const response = await this.request.post(`${this.baseUrl}/api/admin/categories`, {
      headers: this.getHeaders(),
      data: category,
    });
    return this.safeJson(response, 'createCategory');
  }

  /**
   * 获取分类列表
   */
  async getCategories(params?: Record<string, any>): Promise<any> {
    const response = await this.request.get(`${this.baseUrl}/api/admin/categories`, {
      headers: this.getHeaders(),
      params,
    });
    return this.safeJsonSoft(response, 'getCategories');
  }

  /**
   * 删除分类
   */
  async deleteCategory(id: number): Promise<void> {
    await this.request.delete(`${this.baseUrl}/api/admin/categories/${id}`, {
      headers: this.getHeaders(),
    });
  }

  // ===== 系列 API =====

  /**
   * 创建系列
   */
  async createSeries(series: any): Promise<any> {
    const response = await this.request.post(`${this.baseUrl}/api/admin/series`, {
      headers: this.getHeaders(),
      data: series,
    });
    return this.safeJson(response, 'createSeries');
  }

  /**
   * 获取系列列表
   */
  async getSeries(params?: Record<string, any>): Promise<any> {
    const response = await this.request.get(`${this.baseUrl}/api/admin/series`, {
      headers: this.getHeaders(),
      params,
    });
    return this.safeJsonSoft(response, 'getSeries');
  }

  /**
   * 删除系列
   */
  async deleteSeries(id: number): Promise<void> {
    await this.request.delete(`${this.baseUrl}/api/admin/series/${id}`, {
      headers: this.getHeaders(),
    });
  }

  // ===== 徽章 API =====

  /**
   * 创建徽章
   */
  async createBadge(badge: any): Promise<any> {
    const response = await this.request.post(`${this.baseUrl}/api/admin/badges`, {
      headers: this.getHeaders(),
      data: badge,
    });
    return this.safeJson(response, 'createBadge');
  }

  /**
   * 获取徽章列表
   */
  async getBadges(params?: Record<string, any>): Promise<any> {
    const response = await this.request.get(`${this.baseUrl}/api/admin/badges`, {
      headers: this.getHeaders(),
      params,
    });
    return this.safeJsonSoft(response, 'getBadges');
  }

  /**
   * 删除徽章
   */
  async deleteBadge(id: number): Promise<void> {
    await this.request.delete(`${this.baseUrl}/api/admin/badges/${id}`, {
      headers: this.getHeaders(),
    });
  }

  // ===== 规则 API =====

  /**
   * 创建规则
   */
  async createRule(rule: any): Promise<any> {
    const response = await this.request.post(`${this.baseUrl}/api/admin/rules`, {
      headers: this.getHeaders(),
      data: rule,
    });
    return this.safeJson(response, 'createRule');
  }

  /**
   * 获取规则列表
   */
  async getRules(params?: Record<string, any>): Promise<any> {
    const response = await this.request.get(`${this.baseUrl}/api/admin/rules`, {
      headers: this.getHeaders(),
      params,
    });
    return this.safeJsonSoft(response, 'getRules');
  }

  /**
   * 删除规则
   */
  async deleteRule(id: number): Promise<void> {
    await this.request.delete(`${this.baseUrl}/api/admin/rules/${id}`, {
      headers: this.getHeaders(),
    });
  }

  // ===== 权益 API =====

  /**
   * 创建权益
   */
  async createBenefit(benefit: any): Promise<any> {
    const response = await this.request.post(`${this.baseUrl}/api/admin/benefits`, {
      headers: this.getHeaders(),
      data: benefit,
    });
    return this.safeJson(response, 'createBenefit');
  }

  // ===== 徽章状态管理 =====

  async publishBadge(id: number): Promise<any> {
    const response = await this.request.post(`${this.baseUrl}/api/admin/badges/${id}/publish`, {
      headers: this.getHeaders(),
    });
    return this.safeJson(response, 'publishBadge');
  }

  async offlineBadge(id: number): Promise<any> {
    const response = await this.request.post(`${this.baseUrl}/api/admin/badges/${id}/offline`, {
      headers: this.getHeaders(),
    });
    return this.safeJson(response, 'offlineBadge');
  }

  async archiveBadge(id: number): Promise<any> {
    const response = await this.request.post(`${this.baseUrl}/api/admin/badges/${id}/archive`, {
      headers: this.getHeaders(),
    });
    return this.safeJson(response, 'archiveBadge');
  }

  async updateBadge(id: number, data: any): Promise<any> {
    const response = await this.request.put(`${this.baseUrl}/api/admin/badges/${id}`, {
      headers: this.getHeaders(),
      data,
    });
    return this.safeJson(response, 'updateBadge');
  }

  // ===== 规则状态管理 =====

  async publishRule(id: number): Promise<any> {
    const response = await this.request.post(`${this.baseUrl}/api/admin/rules/${id}/publish`, {
      headers: this.getHeaders(),
    });
    return this.safeJson(response, 'publishRule');
  }

  async disableRule(id: number): Promise<any> {
    const response = await this.request.post(`${this.baseUrl}/api/admin/rules/${id}/disable`, {
      headers: this.getHeaders(),
    });
    return this.safeJson(response, 'disableRule');
  }

  async testRule(id: number, context: any): Promise<any> {
    const response = await this.request.post(`${this.baseUrl}/api/admin/rules/${id}/test`, {
      headers: this.getHeaders(),
      data: context,
    });
    return this.safeJson(response, 'testRule');
  }

  // ===== 发放管理 =====

  async grantBadge(userId: string, badgeId: number, source: string): Promise<any> {
    const response = await this.request.post(`${this.baseUrl}/api/admin/grants`, {
      headers: this.getHeaders(),
      data: { userId, badgeId, sourceType: source, sourceId: 'e2e_test' },
    });
    return this.safeJson(response, 'grantBadge');
  }

  async grantBadgeManual(userId: string, badgeId: number, reason: string): Promise<any> {
    const response = await this.request.post(`${this.baseUrl}/api/admin/grants/manual`, {
      headers: this.getHeaders(),
      data: { userId, badgeId, quantity: 1, reason },
    });
    return this.safeJson(response, 'grantBadgeManual');
  }

  async getGrantLogs(params?: Record<string, any>): Promise<any> {
    const response = await this.request.get(`${this.baseUrl}/api/admin/grants/logs`, {
      headers: this.getHeaders(),
      params,
    });
    return this.safeJsonSoft(response, 'getGrantLogs');
  }

  // ===== 用户视图 =====

  async getUserBadges(userId: string): Promise<any> {
    const response = await this.request.get(`${this.baseUrl}/api/admin/users/${userId}/badges`, {
      headers: this.getHeaders(),
    });
    return this.safeJsonSoft(response, 'getUserBadges');
  }

  async searchUsers(keyword: string): Promise<any> {
    const response = await this.request.get(`${this.baseUrl}/api/admin/users/search`, {
      headers: this.getHeaders(),
      params: { keyword },
    });
    return this.safeJsonSoft(response, 'searchUsers');
  }

  async getUserStats(userId: string): Promise<any> {
    const response = await this.request.get(`${this.baseUrl}/api/admin/users/${userId}/stats`, {
      headers: this.getHeaders(),
    });
    return this.safeJsonSoft(response, 'getUserStats');
  }

  // ===== 兑换管理 =====

  async createRedemptionRule(rule: any): Promise<any> {
    const response = await this.request.post(`${this.baseUrl}/api/admin/redemption/rules`, {
      headers: this.getHeaders(),
      data: rule,
    });
    return this.safeJson(response, 'createRedemptionRule');
  }

  async getRedemptionRules(params?: Record<string, any>): Promise<any> {
    const response = await this.request.get(`${this.baseUrl}/api/admin/redemption/rules`, {
      headers: this.getHeaders(),
      params,
    });
    return this.safeJsonSoft(response, 'getRedemptionRules');
  }

  async deleteRedemptionRule(id: number): Promise<void> {
    await this.request.delete(`${this.baseUrl}/api/admin/redemption/rules/${id}`, {
      headers: this.getHeaders(),
    });
  }

  async redeemBadge(userId: string, ruleId: number): Promise<any> {
    const response = await this.request.post(`${this.baseUrl}/api/admin/redemption/redeem`, {
      headers: this.getHeaders(),
      data: { userId, ruleId },
    });
    return this.safeJson(response, 'redeemBadge');
  }

  async getRedemptionOrders(params?: Record<string, any>): Promise<any> {
    const response = await this.request.get(`${this.baseUrl}/api/admin/redemption/orders`, {
      headers: this.getHeaders(),
      params,
    });
    return this.safeJsonSoft(response, 'getRedemptionOrders');
  }

  // ===== 权益管理扩展 =====

  async getBenefits(params?: Record<string, any>): Promise<any> {
    const response = await this.request.get(`${this.baseUrl}/api/admin/benefits`, {
      headers: this.getHeaders(),
      params,
    });
    return this.safeJsonSoft(response, 'getBenefits');
  }

  async deleteBenefit(id: number): Promise<void> {
    await this.request.delete(`${this.baseUrl}/api/admin/benefits/${id}`, {
      headers: this.getHeaders(),
    });
  }

  async linkBadgeToBenefit(benefitId: number, badgeId: number): Promise<any> {
    const response = await this.request.post(`${this.baseUrl}/api/admin/benefits/${benefitId}/link-badge`, {
      headers: this.getHeaders(),
      data: { badgeId, quantity: 1 },
    });
    return this.safeJson(response, 'linkBadgeToBenefit');
  }

  async getBenefitGrants(params?: Record<string, any>): Promise<any> {
    const response = await this.request.get(`${this.baseUrl}/api/admin/benefit-grants`, {
      headers: this.getHeaders(),
      params,
    });
    return this.safeJsonSoft(response, 'getBenefitGrants');
  }

  // ===== 依赖管理 =====

  async createDependency(badgeId: number, dependency: any): Promise<any> {
    const response = await this.request.post(`${this.baseUrl}/api/admin/badges/${badgeId}/dependencies`, {
      headers: this.getHeaders(),
      data: dependency,
    });
    return this.safeJson(response, 'createDependency');
  }

  async getDependencies(badgeId: number): Promise<any> {
    const response = await this.request.get(`${this.baseUrl}/api/admin/badges/${badgeId}/dependencies`, {
      headers: this.getHeaders(),
    });
    return this.safeJsonSoft(response, 'getDependencies');
  }

  async deleteDependency(id: number): Promise<void> {
    await this.request.delete(`${this.baseUrl}/api/admin/dependencies/${id}`, {
      headers: this.getHeaders(),
    });
  }

  // ===== 系统管理 =====

  async createSystemUser(user: any): Promise<any> {
    const response = await this.request.post(`${this.baseUrl}/api/admin/system/users`, {
      headers: this.getHeaders(),
      data: user,
    });
    return this.safeJson(response, 'createSystemUser');
  }

  async getSystemUsers(params?: Record<string, any>): Promise<any> {
    const response = await this.request.get(`${this.baseUrl}/api/admin/system/users`, {
      headers: this.getHeaders(),
      params,
    });
    return this.safeJsonSoft(response, 'getSystemUsers');
  }

  async updateSystemUser(id: number, data: any): Promise<any> {
    const response = await this.request.put(`${this.baseUrl}/api/admin/system/users/${id}`, {
      headers: this.getHeaders(),
      data,
    });
    return this.safeJson(response, 'updateSystemUser');
  }

  /** 确保用户存在且拥有指定角色，若不存在则创建并分配角色 */
  async ensureUser(username: string, password: string, roleId: number): Promise<void> {
    // 尝试创建用户，如果已存在则忽略错误
    let userId: number | null = null;
    try {
      const created = await this.createSystemUser({
        username, password, nickname: username, email: `${username}@test.com`,
      });
      userId = created?.data?.id;
    } catch (e: any) {
      // 用户已存在（中文消息或数据库唯一约束冲突），通过登录获取用户信息确定 ID
      if (e.message?.includes('已存在') || e.message?.includes('duplicate key') || e.message?.includes('409')) {
        const tempContext = this.request;
        const meResp = await tempContext.post(`${this.baseUrl}/api/admin/auth/login`, {
          data: { username, password },
        });
        const meData = await meResp.json();
        userId = meData?.data?.user?.id;
      } else {
        throw e;
      }
    }
    // 分配角色
    if (userId) {
      try {
        await this.updateSystemUser(userId, { roleIds: [roleId] });
      } catch {
        // 角色分配失败不阻塞（可能已分配）
      }
    }
  }

  async deleteSystemUser(id: number): Promise<void> {
    await this.request.delete(`${this.baseUrl}/api/admin/system/users/${id}`, {
      headers: this.getHeaders(),
    });
  }

  async createRole(role: any): Promise<any> {
    const response = await this.request.post(`${this.baseUrl}/api/admin/system/roles`, {
      headers: this.getHeaders(),
      data: role,
    });
    return this.safeJson(response, 'createRole');
  }

  async getRoles(params?: Record<string, any>): Promise<any> {
    const response = await this.request.get(`${this.baseUrl}/api/admin/system/roles`, {
      headers: this.getHeaders(),
      params,
    });
    return this.safeJsonSoft(response, 'getRoles');
  }

  async deleteRole(id: number): Promise<void> {
    await this.request.delete(`${this.baseUrl}/api/admin/system/roles/${id}`, {
      headers: this.getHeaders(),
    });
  }

  async getPermissionTree(): Promise<any> {
    const response = await this.request.get(`${this.baseUrl}/api/admin/system/permissions/tree`, {
      headers: this.getHeaders(),
    });
    return this.safeJsonSoft(response, 'getPermissionTree');
  }

  async createApiKey(name: string, permissions: string[]): Promise<any> {
    const response = await this.request.post(`${this.baseUrl}/api/admin/system/api-keys`, {
      headers: this.getHeaders(),
      data: { name, permissions },
    });
    return this.safeJson(response, 'createApiKey');
  }

  async getApiKeys(): Promise<any> {
    const response = await this.request.get(`${this.baseUrl}/api/admin/system/api-keys`, {
      headers: this.getHeaders(),
    });
    return this.safeJsonSoft(response, 'getApiKeys');
  }

  async deleteApiKey(id: number): Promise<void> {
    await this.request.delete(`${this.baseUrl}/api/admin/system/api-keys/${id}`, {
      headers: this.getHeaders(),
    });
  }

  // ===== 统计 =====

  async getStatsOverview(): Promise<any> {
    const response = await this.request.get(`${this.baseUrl}/api/admin/stats/overview`, {
      headers: this.getHeaders(),
    });
    return this.safeJsonSoft(response, 'getStatsOverview');
  }

  // ===== 模板 =====

  async getTemplates(): Promise<any> {
    const response = await this.request.get(`${this.baseUrl}/api/admin/templates`, {
      headers: this.getHeaders(),
    });
    return this.safeJsonSoft(response, 'getTemplates');
  }

  // ===== 清理 =====

  /**
   * 清理测试数据
   */
  async cleanup(prefix: string): Promise<void> {
    try {
      // 删除顺序：徽章 → 规则 → 兑换规则 → 权益 → 系列 → 分类
      const badgesResponse = await this.getBadges({ keyword: prefix });
      const badges = badgesResponse?.data?.items || [];
      for (const badge of badges) {
        await this.deleteBadge(badge.id).catch(() => {});
      }

      const rulesResponse = await this.getRules({ keyword: prefix });
      const rules = rulesResponse?.data?.items || [];
      for (const rule of rules) {
        await this.deleteRule(rule.id).catch(() => {});
      }

      const redemptionRulesResponse = await this.getRedemptionRules({ keyword: prefix });
      const redemptionRules = redemptionRulesResponse?.data?.items || [];
      for (const rule of redemptionRules) {
        await this.deleteRedemptionRule(rule.id).catch(() => {});
      }

      const benefitsResponse = await this.getBenefits({ keyword: prefix });
      const benefits = benefitsResponse?.data?.items || [];
      for (const benefit of benefits) {
        await this.deleteBenefit(benefit.id).catch(() => {});
      }

      const seriesResponse = await this.getSeries({ name: prefix });
      const seriesList = seriesResponse?.data?.items || [];
      for (const series of seriesList) {
        await this.deleteSeries(series.id).catch(() => {});
      }

      const categoriesResponse = await this.getCategories({ name: prefix });
      const categories = categoriesResponse?.data?.items || [];
      for (const category of categories) {
        await this.deleteCategory(category.id).catch(() => {});
      }
    } catch (e) {
      console.warn('Cleanup failed:', e);
    }
  }

  /**
   * 根据 TestResourceCollector 中记录的资源 ID 进行精确清理，
   * 比基于关键字搜索的 cleanup() 更可靠，不会误删其他并行测试的数据
   */
  async cleanupCollected(collector: TestResourceCollector): Promise<void> {
    const deleters: Record<string, (id: number) => Promise<void>> = {
      badge: (id) => this.deleteBadge(id),
      rule: (id) => this.deleteRule(id),
      redemptionRule: (id) => this.deleteRedemptionRule(id),
      benefit: (id) => this.deleteBenefit(id),
      series: (id) => this.deleteSeries(id),
      category: (id) => this.deleteCategory(id),
    };

    for (const { type, id } of collector.getOrderedForCleanup()) {
      try {
        await deleters[type]?.(id);
      } catch {
        // 清理阶段的错误不应阻塞其余资源的删除
      }
    }
    collector.clear();
  }

  /**
   * 确保测试所需的基础数据存在
   */
  async ensureTestData(prefix: string): Promise<{ categoryId: number; seriesId: number }> {
    // 创建测试分类（若重名冲突则查询已有的）
    let categoryId: number;
    try {
      const categoryResult = await this.createCategory({
        name: `${prefix}默认分类`,
        sortOrder: 0,
      });
      categoryId = categoryResult?.data?.id || 0;
    } catch {
      const existing = await this.getCategories({ name: `${prefix}默认分类` });
      const found = (existing?.data?.items || []).find((c: any) => c.name === `${prefix}默认分类`);
      categoryId = found?.id || 0;
    }

    // 创建测试系列（若重名冲突则查询已有的）
    let seriesId: number;
    try {
      const seriesResult = await this.createSeries({
        name: `${prefix}默认系列`,
        categoryId,
        sortOrder: 0,
      });
      seriesId = seriesResult?.data?.id || 0;
    } catch {
      const existing = await this.getSeries({ name: `${prefix}默认系列` });
      const found = (existing?.data?.items || []).find((s: any) => s.name === `${prefix}默认系列`);
      seriesId = found?.id || 0;
    }

    return { categoryId, seriesId };
  }
}
