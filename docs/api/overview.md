# API 概述

## 基础信息

| 项目 | 值 |
|------|-----|
| 基础 URL | `http://localhost:8081` |
| 协议 | HTTP/HTTPS |
| 数据格式 | JSON |
| 字符编码 | UTF-8 |

## 请求头

所有 POST/PUT/PATCH 请求需要设置：

```
Content-Type: application/json
```

需要用户认证的接口需要在请求头中携带 JWT Token：

```
Authorization: Bearer <token>
```

---

## 认证方式

### 1. 用户认证 (User Auth)

用于用户账号系统，通过登录获取 JWT Token。

**获取方式：** 调用 `POST /auth/login` 接口

**Token 格式：** JWT (JSON Web Token)

**有效期：** 24 小时（默认）

**使用方式：**
```bash
curl -X GET http://localhost:8081/protected-endpoint \
  -H "Authorization: Bearer eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9..."
```

### 2. LiveKit Token

用于客户端连接 LiveKit 实时音视频服务。

**获取方式：** 调用 `POST /token` 接口

**用途：** 连接 LiveKit WebSocket 服务，加入音视频房间

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
| 401 | Unauthorized | 认证失败、密码错误、Token 无效 |
| 404 | Not Found | 资源不存在 |
| 409 | Conflict | 资源冲突（邮箱已注册） |
| 429 | Too Many Requests | 请求过于频繁（验证码发送限制） |
| 500 | Internal Server Error | 服务器内部错误 |

---

## 错误响应示例

### 400 Bad Request

```json
{
  "error": "Invalid email format"
}
```

### 401 Unauthorized

```json
{
  "error": "Invalid credentials"
}
```

### 409 Conflict

```json
{
  "error": "Email already registered"
}
```

### 429 Too Many Requests

```json
{
  "error": "Please wait before requesting another code"
}
```

### 500 Internal Server Error

```json
{
  "error": "Failed to list rooms: connection refused"
}
```

---

## 环境配置

服务器通过环境变量配置：

### 服务器配置

| 变量 | 默认值 | 描述 |
|------|--------|------|
| `SERVER_HOST` | `localhost` | 服务器主机 |
| `SERVER_PORT` | `8081` | 服务器端口 |
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
| `DATABASE_URL` | - | PostgreSQL 连接字符串 |

### JWT 配置

| 变量 | 默认值 | 描述 |
|------|--------|------|
| `JWT_SECRET` | - | JWT 签名密钥 |
| `JWT_EXPIRATION_SECS` | `86400` | Token 有效期（秒） |

### 邮件配置

| 变量 | 默认值 | 描述 |
|------|--------|------|
| `SMTP_HOST` | - | SMTP 服务器地址 |
| `SMTP_PORT` | `465` | SMTP 端口 |
| `SMTP_SENDER` | - | 发件人邮箱 |
| `SMTP_PASSWORD` | - | SMTP 授权码 |
| `SMTP_USE_SSL` | `true` | 是否使用 SSL |

### 验证码配置

| 变量 | 默认值 | 描述 |
|------|--------|------|
| `CODE_HMAC_SECRET` | - | 验证码 HMAC 密钥 |
| `CODE_LENGTH` | `6` | 验证码长度 |
| `CODE_RATE_LIMIT_SECS` | `60` | 验证码发送间隔（秒） |
| `CODE_EXPIRATION_SECS` | `600` | 验证码有效期（秒） |
