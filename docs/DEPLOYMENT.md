# Docker 部署说明

本项目现在包含两套 Docker 编排文件：

- `docker-compose.yml`：本地开发 / 测试数据库
- `docker-compose.deploy.yml`：部署 Rust 服务本身

## 1. 准备部署环境变量

复制部署环境变量模板：

```bash
cp .env.deploy.example .env.deploy
```

然后按你的环境修改以下关键项：

- `DATABASE_URL`
- `LIVEKIT_URL`
- `LIVEKIT_WS_URL`
- `LIVEKIT_API_KEY`
- `LIVEKIT_API_SECRET`
- `JWT_SECRET`
- `CODE_HMAC_SECRET`
- `SMTP_*`
- `APP_BASE_URL`

注意：

- 容器内访问外部 PostgreSQL / LiveKit 时，不要把地址写成 `localhost`
- 如果依赖服务就在宿主机上，可以使用 `host.docker.internal`
- 生产环境下保持 `ENABLE_HTTPS=false`，HTTPS 由反向代理或负载均衡处理

## 2. 构建镜像

```bash
docker build -t links-sig-rust-server:latest .
```

如果你想使用其他 tag，可在 `.env.deploy` 中修改 `APP_IMAGE`。

## 3. 启动服务

```bash
docker compose --env-file .env.deploy -f docker-compose.deploy.yml up -d
```

这条命令会同时完成两件事：

- 用 `.env.deploy` 替换 compose 文件中的 `${APP_IMAGE}` 和 `${HOST_PORT}`
- 把 `.env.deploy` 注入容器运行环境

查看日志：

```bash
docker compose --env-file .env.deploy -f docker-compose.deploy.yml logs -f app
```

## 4. 更新部署

代码更新后执行：

```bash
docker build -t links-sig-server:latest .
docker compose --env-file .env.deploy -f docker-compose.deploy.yml up -d
```

## 5. 验证

服务启动后检查：

```bash
curl http://127.0.0.1:8081/health
```

如果你修改了 `HOST_PORT`，把命令中的端口改成对应值。

如果启动失败，先检查：

- 数据库地址是否可从容器内访问
- LiveKit 地址是否可从容器内访问
- 密钥和 SMTP 配置是否完整
- 反向代理是否把外部 HTTPS 正确转发到容器 HTTP 端口
