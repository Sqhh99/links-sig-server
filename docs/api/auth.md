# 用户认证 API

用户账号系统相关接口，包括注册验证码、注册与登录。

---

## POST /api/auth/register/request-code

请求注册验证码，服务端会发送数字验证码邮件。

### 请求

```bash
curl -X POST http://localhost:8081/api/auth/register/request-code \
  -H "Content-Type: application/json" \
  -d '{"email": "user@example.com"}'
```

### 请求体

| 字段 | 类型 | 必填 | 描述 |
|------|------|------|------|
| `email` | string | 是 | 用户邮箱地址 |

### 成功响应（200 OK）

```json
{
  "message": "Verification code sent to your email",
  "retryAfterSecs": 60
}
```

### 可能错误

| 状态码 | 示例 |
|--------|------|
| 400 | `{"error":"Invalid email format"}` |
| 409 | `{"error":"Email is already registered"}` |
| 429 | `{"error":"Please wait 42 seconds before requesting another code"}` |
| 500 | `{"error":"Failed to send verification email"}` |

### 说明

- 邮箱会先做 `trim + lowercase`。
- `retryAfterSecs` 是再次请求的固定间隔（来自 `CODE_RATE_LIMIT_SECS`），不是剩余秒数。

---

## POST /api/auth/register

使用邮箱、验证码和密码完成注册。

### 请求

```bash
curl -X POST http://localhost:8081/api/auth/register \
  -H "Content-Type: application/json" \
  -d '{
    "email": "user@example.com",
    "code": "123456",
    "password": "SecurePass123!"
  }'
```

### 请求体

| 字段 | 类型 | 必填 | 描述 |
|------|------|------|------|
| `email` | string | 是 | 用户邮箱 |
| `code` | string | 是 | 邮箱验证码 |
| `password` | string | 是 | 密码（最少 8 位） |

### 成功响应（201 Created）

```json
{
  "userId": "550e8400-e29b-41d4-a716-446655440000",
  "email": "user@example.com",
  "token": "eyJ..."
}
```

### 可能错误

| 状态码 | 示例 |
|--------|------|
| 400 | `{"error":"Invalid email format"}` |
| 400 | `{"error":"Password must be at least 8 characters"}` |
| 400 | `{"error":"Invalid or expired verification code"}` |
| 409 | `{"error":"Email is already registered"}` |

---

## POST /api/auth/login

邮箱+密码登录，返回用户 JWT。

### 请求

```bash
curl -X POST http://localhost:8081/api/auth/login \
  -H "Content-Type: application/json" \
  -d '{
    "email": "user@example.com",
    "password": "SecurePass123!"
  }'
```

### 成功响应（200 OK）

```json
{
  "userId": "550e8400-e29b-41d4-a716-446655440000",
  "email": "user@example.com",
  "token": "eyJ..."
}
```

### 可能错误

| 状态码 | 示例 |
|--------|------|
| 401 | `{"error":"Invalid email or password"}` |

### 说明

- 登录邮箱同样会 `trim + lowercase`，因此大小写不敏感。
- 登录失败不会区分“邮箱不存在”还是“密码错误”。

---

## POST /api/auth/refresh

使用当前用户 JWT 刷新并获取一个新的用户 JWT。

### 请求

```bash
curl -X POST http://localhost:8081/api/auth/refresh \
  -H "Authorization: Bearer <user-jwt>"
```

> 无需请求体。

### 成功响应（200 OK）

```json
{
  "userId": "550e8400-e29b-41d4-a716-446655440000",
  "email": "user@example.com",
  "token": "eyJ...",
  "expiresInSecs": 604800
}
```

### 字段说明

| 字段 | 类型 | 描述 |
|------|------|------|
| `userId` | string | 用户 ID |
| `email` | string | 用户邮箱 |
| `token` | string | 新签发的 JWT |
| `expiresInSecs` | number | 新 token 有效期（秒） |

### 可能错误

| 状态码 | 示例 |
|--------|------|
| 401 | `{"error":"Authorization header required"}` |
| 401 | `{"error":"Token has expired"}` |
| 401 | `{"error":"Invalid token format"}` |
| 401 | `{"error":"User not found"}` |

---

## JWT 说明

用户 JWT（来自注册/登录）为 HS256 签名，Payload 主要字段：

```json
{
  "sub": "550e8400-e29b-41d4-a716-446655440000",
  "email": "user@example.com",
  "iat": 1737452400,
  "nbf": 1737452400,
  "exp": 1738057200
}
```

默认有效期为 `604800` 秒（7 天），可通过 `JWT_EXPIRATION_SECS` 调整。
