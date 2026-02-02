import { defineConfig, Plugin } from 'vite';
import react from '@vitejs/plugin-react';
import { resolve } from 'path';

/**
 * Mock API 插件
 *
 * 在开发环境中拦截认证 API 请求，返回 mock 数据
 */
function mockApiPlugin(): Plugin {
  return {
    name: 'mock-api',
    configureServer(server) {
      // 认证 Mock 数据
      const mockUsers: Record<string, { password: string; user: any }> = {
        admin: {
          password: 'admin123',
          user: {
            id: '1',
            username: 'admin',
            displayName: '系统管理员',
            role: 'admin',
          },
        },
        operator: {
          password: 'operator123',
          user: {
            id: '2',
            username: 'operator',
            displayName: '运营人员',
            role: 'operator',
          },
        },
        viewer: {
          password: 'viewer123',
          user: {
            id: '3',
            username: 'viewer',
            displayName: '访客',
            role: 'viewer',
          },
        },
      };

      // 生成 mock token
      const generateToken = (user: any): string => {
        const payload = {
          sub: user.id,
          username: user.username,
          role: user.role,
          exp: Date.now() + 24 * 60 * 60 * 1000,
        };
        return `mock.${Buffer.from(JSON.stringify(payload)).toString('base64')}.signature`;
      };

      // Token 存储（模拟服务端 session）
      const validTokens = new Map<string, any>();

      server.middlewares.use('/api/admin/auth', (req, res, next) => {
        // 设置响应头
        res.setHeader('Content-Type', 'application/json');

        // 解析请求体
        let body = '';
        req.on('data', (chunk) => {
          body += chunk;
        });

        req.on('end', () => {
          try {
            // 登录接口
            if (req.url === '/login' && req.method === 'POST') {
              const { username, password } = JSON.parse(body);
              const userData = mockUsers[username];

              if (!userData || userData.password !== password) {
                res.statusCode = 401;
                res.end(
                  JSON.stringify({
                    success: false,
                    error: {
                      code: 'INVALID_CREDENTIALS',
                      message: '用户名或密码错误',
                    },
                  })
                );
                return;
              }

              const token = generateToken(userData.user);
              validTokens.set(token, userData.user);

              res.statusCode = 200;
              res.end(
                JSON.stringify({
                  success: true,
                  data: {
                    token,
                    user: userData.user,
                  },
                })
              );
              return;
            }

            // 登出接口
            if (req.url === '/logout' && req.method === 'POST') {
              const authHeader = req.headers.authorization;
              if (authHeader) {
                const token = authHeader.replace('Bearer ', '');
                validTokens.delete(token);
              }

              res.statusCode = 200;
              res.end(JSON.stringify({ success: true, data: null }));
              return;
            }

            // 获取当前用户信息
            if (req.url === '/me' && req.method === 'GET') {
              const authHeader = req.headers.authorization;
              if (!authHeader) {
                res.statusCode = 401;
                res.end(
                  JSON.stringify({
                    success: false,
                    error: {
                      code: 'UNAUTHORIZED',
                      message: '请先登录',
                    },
                  })
                );
                return;
              }

              const token = authHeader.replace('Bearer ', '');
              const user = validTokens.get(token);

              if (!user) {
                res.statusCode = 401;
                res.end(
                  JSON.stringify({
                    success: false,
                    error: {
                      code: 'INVALID_TOKEN',
                      message: '登录已过期，请重新登录',
                    },
                  })
                );
                return;
              }

              res.statusCode = 200;
              res.end(JSON.stringify({ success: true, data: user }));
              return;
            }

            // 刷新 token
            if (req.url === '/refresh' && req.method === 'POST') {
              const authHeader = req.headers.authorization;
              if (!authHeader) {
                res.statusCode = 401;
                res.end(
                  JSON.stringify({
                    success: false,
                    error: {
                      code: 'UNAUTHORIZED',
                      message: '请先登录',
                    },
                  })
                );
                return;
              }

              const oldToken = authHeader.replace('Bearer ', '');
              const user = validTokens.get(oldToken);

              if (!user) {
                res.statusCode = 401;
                res.end(
                  JSON.stringify({
                    success: false,
                    error: {
                      code: 'INVALID_TOKEN',
                      message: '登录已过期，请重新登录',
                    },
                  })
                );
                return;
              }

              // 生成新 token
              const newToken = generateToken(user);
              validTokens.delete(oldToken);
              validTokens.set(newToken, user);

              res.statusCode = 200;
              res.end(JSON.stringify({ success: true, data: { token: newToken } }));
              return;
            }

            // 未匹配的路由
            next();
          } catch (e) {
            res.statusCode = 400;
            res.end(
              JSON.stringify({
                success: false,
                error: {
                  code: 'BAD_REQUEST',
                  message: '请求格式错误',
                },
              })
            );
          }
        });
      });
    },
  };
}

export default defineConfig({
  plugins: [react(), mockApiPlugin()],
  resolve: {
    alias: {
      '@': resolve(__dirname, 'src'),
    },
  },
  server: {
    port: 3001,
    proxy: {
      // 非认证 API 代理到后端服务
      '/api': {
        target: 'http://localhost:8080',
        changeOrigin: true,
        // 排除已被 mock 处理的认证 API
        bypass: (req) => {
          if (req.url?.startsWith('/api/admin/auth')) {
            return req.url;
          }
        },
      },
    },
  },
});
