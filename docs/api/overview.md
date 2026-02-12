# API 概述

## 基础信息

| 项目 | 值 |
|------|-----|
| 基础 URL | `http://localhost:8081` |
| API 前缀 | `/api` |
| 协议 | HTTP/HTTPS |
| 数据格式 | JSON |
| 字符编码 | UTF-8 |

## 请求头

所有 `POST` 请求建议设置：

```
Content-Type: application/json
```

用户登录/注册接口会返回用户 JWT，可放入请求头：

```
Authorization: Bearer <token>
```

其中会议业务接口（`/api/meetings*`、`/api/me/meeting-records`）会强制校验用户 JWT。
此外，`/api/rooms/{room_name}/participants/{identity}` 与 `/api/rooms/{room_name}/end` 在命中业务会议房间（`m-#########` 且存在于 `meetings`）时，也会强制校验主持人权限。

---

## 认证方式

### 1. 用户 JWT（账号体系）

- 获取方式：`POST /api/auth/login` 或 `POST /api/auth/register`
- 刷新方式：`POST /api/auth/refresh`（需携带当前用户 JWT）
- 默认有效期：`604800` 秒（7 天，可通过 `JWT_EXPIRATION_SECS` 配置）
- 用途：账号体系身份凭证（后续可用于受保护接口）

### 2. LiveKit Token（音视频接入）

- 获取方式：`POST /api/token`
- 默认有效期：24 小时（服务端固定）
- 用途：客户端连接 LiveKit WebSocket，加入房间

---

## 通用响应格式

### 成功响应

HTTP 状态码：`2xx`

```json
{
  "field1": "value1",
  "field2": "value2"
}
```

### 错误响应

HTTP 状态码：`4xx` 或 `5xx`

```json
{
  "error": "错误描述信息"
}
```

---

## HTTP 状态码

| 状态码 | 描述 | 常见场景 |
|--------|------|---------|
| 200 | OK | 请求成功 |
| 201 | Created | 资源创建成功（注册、创建房间） |
| 400 | Bad Request | 请求参数无效、JSON 格式错误、验证码错误 |
| 401 | Unauthorized | 登录认证失败 |
| 403 | Forbidden | 已认证但无权限（如非主持人踢人/结束业务会议） |
| 404 | Not Found | 资源不存在 |
| 409 | Conflict | 资源冲突（邮箱已注册） |
| 429 | Too Many Requests | 请求过于频繁（验证码发送限制） |
| 500 | Internal Server Error | 服务器内部错误 |

---

## 错误响应示例

```json
{
  "error": "Invalid email format"
}
```

```json
{
  "error": "Invalid email or password"
}
```

```json
{
  "error": "Email is already registered"
}
```

```json
{
  "error": "Please wait 42 seconds before requesting another code"
}
```

---

## 环境变量

服务器通过环境变量配置：

### 服务器配置

| 变量 | 默认值 | 描述 |
|------|--------|------|
| `SERVER_HOST` | `localhost` | 服务器主机 |
| `SERVER_PORT` | `8081` | 服务器端口 |
| `APP_BASE_URL` | `http://localhost:3000` | 会议分享链接前端地址 |
| `ENABLE_HTTPS` | `false` | 是否启用 HTTPS |
| `SSL_CERT_FILE` | `./certs/server.crt` | SSL 证书路径 |
| `SSL_KEY_FILE` | `./certs/server.key` | SSL 密钥路径 |

### LiveKit 配置

| 变量 | 默认值 | 描述 |
|------|--------|------|
| `LIVEKIT_URL` | `http://localhost:7880` | LiveKit API 地址 |
| `LIVEKIT_WS_URL` | `ws://localhost:7880` | LiveKit WebSocket 地址 |
| `LIVEKIT_API_KEY` | `devkey` | LiveKit API Key |
| `LIVEKIT_API_SECRET` | `secret` | LiveKit API Secret |

### 数据库配置

| 变量 | 默认值 | 描述 |
|------|--------|------|
| `DATABASE_URL` | `postgres://links_sig:links_sig_password@localhost:5432/links_sig` | PostgreSQL 连接字符串 |

### 用户 JWT 配置

| 变量 | 默认值 | 描述 |
|------|--------|------|
| `JWT_SECRET` | `your-super-secret-jwt-key-change-in-production` | JWT 签名密钥 |
| `JWT_EXPIRATION_SECS` | `604800` | 用户 Token 有效期（秒） |

### 验证码配置

| 变量 | 默认值 | 描述 |
|------|--------|------|
| `CODE_HMAC_SECRET` | `your-super-secret-hmac-key-change-in-production` | 验证码 HMAC 密钥 |
| `CODE_LENGTH` | `6` | 验证码长度 |
| `CODE_RATE_LIMIT_SECS` | `60` | 验证码发送间隔（秒） |
| `CODE_EXPIRATION_SECS` | `600` | 验证码有效期（秒） |

### SMTP 配置

| 变量 | 默认值 | 描述 |
|------|--------|------|
| `SMTP_HOST` | `smtp.example.com` | SMTP 服务器地址 |
| `SMTP_PORT` | `587` | SMTP 端口 |
| `SMTP_SENDER` | `noreply@example.com` | 发件人邮箱 |
| `SMTP_PASSWORD` | 空字符串 | SMTP 密码或授权码 |
| `SMTP_USE_SSL` | `false` | 是否使用 SSL/TLS |
