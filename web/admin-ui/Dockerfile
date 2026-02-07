# 徽章管理后台前端
# 基于 Vite + React 构建，通过 nginx 提供静态资源并反向代理 API 请求

FROM nginx:1.27-alpine

COPY dist/ /usr/share/nginx/html/
COPY nginx.conf /etc/nginx/conf.d/default.conf

EXPOSE 80
