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
   * 登录并获取 token
   */
  async login(username: string, password: string): Promise<string> {
    const response = await this.request.post(`${this.baseUrl}/api/admin/auth/login`, {
      headers: this.getHeaders(),
      data: { username, password },
    });
    const data = await response.json();
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
    return response.json();
  }

  /**
   * 获取分类列表
   */
  async getCategories(params?: Record<string, any>): Promise<any> {
    const response = await this.request.get(`${this.baseUrl}/api/admin/categories`, {
      headers: this.getHeaders(),
      params,
    });
    return response.json();
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
    return response.json();
  }

  /**
   * 获取系列列表
   */
  async getSeries(params?: Record<string, any>): Promise<any> {
    const response = await this.request.get(`${this.baseUrl}/api/admin/series`, {
      headers: this.getHeaders(),
      params,
    });
    return response.json();
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
    return response.json();
  }

  /**
   * 获取徽章列表
   */
  async getBadges(params?: Record<string, any>): Promise<any> {
    const response = await this.request.get(`${this.baseUrl}/api/admin/badges`, {
      headers: this.getHeaders(),
      params,
    });
    return response.json();
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
    return response.json();
  }

  /**
   * 获取规则列表
   */
  async getRules(params?: Record<string, any>): Promise<any> {
    const response = await this.request.get(`${this.baseUrl}/api/admin/rules`, {
      headers: this.getHeaders(),
      params,
    });
    return response.json();
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
    return response.json();
  }

  // ===== 用户徽章 API =====

  /**
   * 为用户发放徽章
   */
  async grantBadge(userId: string, badgeId: number, source: string): Promise<any> {
    const response = await this.request.post(
      `${this.baseUrl}/api/admin/grants`,
      {
        headers: this.getHeaders(),
        data: { user_id: userId, badge_id: badgeId, source_type: source, source_id: 'e2e_test' },
      }
    );
    return response.json();
  }

  /**
   * 获取用户徽章
   */
  async getUserBadges(userId: string): Promise<any> {
    const response = await this.request.get(
      `${this.baseUrl}/api/admin/users/${userId}/badges`,
      {
        headers: this.getHeaders(),
      }
    );
    return response.json();
  }

  // ===== 清理 =====

  /**
   * 清理测试数据
   */
  async cleanup(prefix: string): Promise<void> {
    try {
      // 先删除徽章（依赖系列）
      const badgesResponse = await this.getBadges({ keyword: prefix });
      const badges = badgesResponse?.data?.items || [];
      for (const badge of badges) {
        await this.deleteBadge(badge.id);
      }

      // 删除规则
      const rulesResponse = await this.getRules({ keyword: prefix });
      const rules = rulesResponse?.data?.items || [];
      for (const rule of rules) {
        await this.deleteRule(rule.id);
      }

      // 删除系列（依赖分类）
      const seriesResponse = await this.getSeries({ keyword: prefix });
      const seriesList = seriesResponse?.data?.items || [];
      for (const series of seriesList) {
        await this.deleteSeries(series.id);
      }

      // 删除分类
      const categoriesResponse = await this.getCategories({ keyword: prefix });
      const categories = categoriesResponse?.data?.items || [];
      for (const category of categories) {
        await this.deleteCategory(category.id);
      }
    } catch (e) {
      // 清理失败不影响测试
      console.warn('Cleanup failed:', e);
    }
  }

  /**
   * 确保测试所需的基础数据存在
   */
  async ensureTestData(prefix: string): Promise<{ categoryId: number; seriesId: number }> {
    // 创建测试分类
    const categoryResult = await this.createCategory({
      name: `${prefix}默认分类`,
      sortOrder: 0,
    });
    const categoryId = categoryResult?.data?.id;

    // 创建测试系列
    const seriesResult = await this.createSeries({
      name: `${prefix}默认系列`,
      categoryId: categoryId,
      sortOrder: 0,
    });
    const seriesId = seriesResult?.data?.id;

    return { categoryId, seriesId };
  }
}
