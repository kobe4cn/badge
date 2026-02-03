import { APIRequestContext } from '@playwright/test';

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
   * 安全地解析 JSON 响应，避免非 JSON 响应体（如 4xx/5xx 纯文本错误）导致 .json() 抛出异常
   */
  private async safeJson(response: any, context: string = ''): Promise<any> {
    if (!response.ok()) {
      const text = await response.text();
      console.error(`API Error [${response.status()}] ${context}: ${text.substring(0, 200)}`);
      return { success: false, error: text, status: response.status() };
    }
    try {
      return await response.json();
    } catch (e) {
      const text = await response.text();
      console.error(`JSON Parse Error ${context}: ${text.substring(0, 200)}`);
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
    return this.safeJson(response, 'getCategories');
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
    return this.safeJson(response, 'getSeries');
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
    return this.safeJson(response, 'getBadges');
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
    return this.safeJson(response, 'getRules');
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
    return this.safeJson(response, 'getGrantLogs');
  }

  // ===== 用户视图 =====

  async getUserBadges(userId: string): Promise<any> {
    const response = await this.request.get(`${this.baseUrl}/api/admin/users/${userId}/badges`, {
      headers: this.getHeaders(),
    });
    return this.safeJson(response, 'getUserBadges');
  }

  async searchUsers(keyword: string): Promise<any> {
    const response = await this.request.get(`${this.baseUrl}/api/admin/users/search`, {
      headers: this.getHeaders(),
      params: { keyword },
    });
    return this.safeJson(response, 'searchUsers');
  }

  async getUserStats(userId: string): Promise<any> {
    const response = await this.request.get(`${this.baseUrl}/api/admin/users/${userId}/stats`, {
      headers: this.getHeaders(),
    });
    return this.safeJson(response, 'getUserStats');
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
    return this.safeJson(response, 'getRedemptionRules');
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
    return this.safeJson(response, 'getRedemptionOrders');
  }

  // ===== 权益管理扩展 =====

  async getBenefits(params?: Record<string, any>): Promise<any> {
    const response = await this.request.get(`${this.baseUrl}/api/admin/benefits`, {
      headers: this.getHeaders(),
      params,
    });
    return this.safeJson(response, 'getBenefits');
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
    return this.safeJson(response, 'getBenefitGrants');
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
    return this.safeJson(response, 'getDependencies');
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
    return this.safeJson(response, 'getSystemUsers');
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
    return this.safeJson(response, 'getRoles');
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
    return this.safeJson(response, 'getPermissionTree');
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
    return this.safeJson(response, 'getApiKeys');
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
    return this.safeJson(response, 'getStatsOverview');
  }

  // ===== 模板 =====

  async getTemplates(): Promise<any> {
    const response = await this.request.get(`${this.baseUrl}/api/admin/templates`, {
      headers: this.getHeaders(),
    });
    return this.safeJson(response, 'getTemplates');
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
   * 确保测试所需的基础数据存在
   */
  async ensureTestData(prefix: string): Promise<{ categoryId: number; seriesId: number }> {
    // 创建测试分类（若重名冲突则查询已有的）
    let categoryId: number;
    const categoryResult = await this.createCategory({
      name: `${prefix}默认分类`,
      sortOrder: 0,
    });
    if (categoryResult?.data?.id) {
      categoryId = categoryResult.data.id;
    } else {
      const existing = await this.getCategories({ name: `${prefix}默认分类` });
      const found = (existing?.data?.items || []).find((c: any) => c.name === `${prefix}默认分类`);
      categoryId = found?.id || 0;
    }

    // 创建测试系列（若重名冲突则查询已有的）
    let seriesId: number;
    const seriesResult = await this.createSeries({
      name: `${prefix}默认系列`,
      categoryId,
      sortOrder: 0,
    });
    if (seriesResult?.data?.id) {
      seriesId = seriesResult.data.id;
    } else {
      const existing = await this.getSeries({ name: `${prefix}默认系列` });
      const found = (existing?.data?.items || []).find((s: any) => s.name === `${prefix}默认系列`);
      seriesId = found?.id || 0;
    }

    return { categoryId, seriesId };
  }
}
