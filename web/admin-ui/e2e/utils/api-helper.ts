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
    const response = await this.request.post(`${this.baseUrl}/api/auth/login`, {
      headers: this.getHeaders(),
      data: { username, password },
    });
    const data = await response.json();
    this.token = data.token;
    return data.token;
  }

  // ===== 徽章 API =====

  /**
   * 创建徽章
   */
  async createBadge(badge: any): Promise<any> {
    const response = await this.request.post(`${this.baseUrl}/api/badges`, {
      headers: this.getHeaders(),
      data: badge,
    });
    return response.json();
  }

  /**
   * 获取徽章列表
   */
  async getBadges(params?: Record<string, any>): Promise<any> {
    const response = await this.request.get(`${this.baseUrl}/api/badges`, {
      headers: this.getHeaders(),
      params,
    });
    return response.json();
  }

  /**
   * 删除徽章
   */
  async deleteBadge(id: number): Promise<void> {
    await this.request.delete(`${this.baseUrl}/api/badges/${id}`, {
      headers: this.getHeaders(),
    });
  }

  // ===== 规则 API =====

  /**
   * 创建规则
   */
  async createRule(rule: any): Promise<any> {
    const response = await this.request.post(`${this.baseUrl}/api/rules`, {
      headers: this.getHeaders(),
      data: rule,
    });
    return response.json();
  }

  /**
   * 获取规则列表
   */
  async getRules(params?: Record<string, any>): Promise<any> {
    const response = await this.request.get(`${this.baseUrl}/api/rules`, {
      headers: this.getHeaders(),
      params,
    });
    return response.json();
  }

  /**
   * 删除规则
   */
  async deleteRule(id: number): Promise<void> {
    await this.request.delete(`${this.baseUrl}/api/rules/${id}`, {
      headers: this.getHeaders(),
    });
  }

  // ===== 权益 API =====

  /**
   * 创建权益
   */
  async createBenefit(benefit: any): Promise<any> {
    const response = await this.request.post(`${this.baseUrl}/api/benefits`, {
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
      `${this.baseUrl}/api/users/${userId}/badges/${badgeId}/grant`,
      {
        headers: this.getHeaders(),
        data: { source_type: source, source_id: 'e2e_test' },
      }
    );
    return response.json();
  }

  /**
   * 获取用户徽章
   */
  async getUserBadges(userId: string): Promise<any> {
    const response = await this.request.get(
      `${this.baseUrl}/api/users/${userId}/badges`,
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
    // 删除以 prefix 开头的测试数据
    const badges = await this.getBadges({ keyword: prefix });
    for (const badge of badges.data || []) {
      await this.deleteBadge(badge.id);
    }

    const rules = await this.getRules({ keyword: prefix });
    for (const rule of rules.data || []) {
      await this.deleteRule(rule.id);
    }
  }
}
