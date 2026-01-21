# 用户认证 API

用户账号系统相关接口，包括注册、登录和邮箱验证。

---

## POST /api/auth/register/request-code

请求注册验证码，系统会向指定邮箱发送 6 位数字验证码。

### 请求

```bash
curl -X POST http://localhost:8081/api/auth/register/request-code \
  -H "Content-Type: application/json" \
  -d '{
    "email": "user@example.com"
  }'
```

### 请求体

| 字段 | 类型 | 必填 | 描述 |
|------|------|------|------|
| `email` | string | 是 | 用户邮箱地址 |

### 响应

#### 成功 (200 OK)

```json
{
  "message": "Verification code sent",
  "expires_in_secs": 600
}
```

#### 邮箱已注册 (409 Conflict)

```json
{
  "error": "Email already registered"
}
```

#### 请求过于频繁 (429 Too Many Requests)

```json
{
  "error": "Please wait before requesting another code"
}
```

#### 邮箱格式无效 (400 Bad Request)

```json
{
  "error": "Invalid email format"
}
```

### 字段说明

| 字段 | 类型 | 描述 |
|------|------|------|
| `message` | string | 成功提示信息 |
| `expires_in_secs` | number | 验证码有效期（秒），默认 600 秒 |

### 注意事项

- 同一邮箱 60 秒内只能请求一次验证码
- 验证码有效期为 10 分钟
- 邮箱地址会自动转为小写存储

---

## POST /api/auth/register

使用邮箱、密码和验证码完成用户注册。

### 请求

```bash
curl -X POST http://localhost:8081/api/auth/register \
  -H "Content-Type: application/json" \
  -d '{
    "email": "user@example.com",
    "password": "SecurePass123!",
    "code": "123456"
  }'
```

### 请求体

| 字段 | 类型 | 必填 | 描述 |
|------|------|------|------|
| `email` | string | 是 | 用户邮箱地址 |
| `password` | string | 是 | 用户密码（至少 8 位） |
| `code` | string | 是 | 邮箱验证码（6 位数字） |

### 响应

#### 成功 (201 Created)

```json
{
  "user_id": "550e8400-e29b-41d4-a716-446655440000",
  "email": "user@example.com",
  "token": "eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9..."
}
```

#### 验证码错误 (400 Bad Request)

```json
{
  "error": "Invalid or expired verification code"
}
```

#### 密码太弱 (400 Bad Request)

```json
{
  "error": "Password must be at least 8 characters"
}
```

#### 邮箱已注册 (409 Conflict)

```json
{
  "error": "Email already registered"
}
```

### 字段说明

| 字段 | 类型 | 描述 |
|------|------|------|
| `user_id` | string | 用户唯一 ID (UUID) |
| `email` | string | 用户邮箱 |
| `token` | string | JWT 访问令牌，可直接用于后续请求 |

### 密码要求

- 最少 8 个字符
- 建议包含大小写字母、数字和特殊字符

---

## POST /api/auth/login

用户登录，验证邮箱和密码，返回 JWT Token。

### 请求

```bash
curl -X POST http://localhost:8081/api/auth/login \
  -H "Content-Type: application/json" \
  -d '{
    "email": "user@example.com",
    "password": "SecurePass123!"
  }'
```

### 请求体

| 字段 | 类型 | 必填 | 描述 |
|------|------|------|------|
| `email` | string | 是 | 用户邮箱地址 |
| `password` | string | 是 | 用户密码 |

### 响应

#### 成功 (200 OK)

```json
{
  "user_id": "550e8400-e29b-41d4-a716-446655440000",
  "email": "user@example.com",
  "token": "eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9..."
}
```

#### 认证失败 (401 Unauthorized)

```json
{
  "error": "Invalid credentials"
}
```

### 字段说明

| 字段 | 类型 | 描述 |
|------|------|------|
| `user_id` | string | 用户唯一 ID (UUID) |
| `email` | string | 用户邮箱 |
| `token` | string | JWT 访问令牌，有效期 24 小时 |

### 注意事项

- 邮箱验证不区分大小写
- 登录失败不会提示具体是邮箱还是密码错误（安全考虑）

---

## 完整注册流程示例

```bash
# 步骤 1: 请求验证码
curl -X POST http://localhost:8081/api/auth/register/request-code \
  -H "Content-Type: application/json" \
  -d '{"email": "newuser@example.com"}'

# 响应: {"message": "Verification code sent", "expires_in_secs": 600}
# 用户会收到一封包含 6 位验证码的邮件

# 步骤 2: 使用验证码完成注册
curl -X POST http://localhost:8081/api/auth/register \
  -H "Content-Type: application/json" \
  -d '{
    "email": "newuser@example.com",
    "password": "MySecurePassword123!",
    "code": "123456"
  }'

# 响应: 
# {
#   "user_id": "550e8400-e29b-41d4-a716-446655440000",
#   "email": "newuser@example.com",
#   "token": "eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9..."
# }

# 步骤 3: 使用 Token 访问受保护资源
curl -X GET http://localhost:8081/api/protected-resource \
  -H "Authorization: Bearer eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9..."
```

---

## JWT Token 说明

### Token 结构

```
Header.Payload.Signature
```

### Payload 内容

```json
{
  "sub": "550e8400-e29b-41d4-a716-446655440000",
  "email": "user@example.com",
  "iat": 1737452400,
  "exp": 1737538800
}
```

| 字段 | 描述 |
|------|------|
| `sub` | 用户 ID (Subject) |
| `email` | 用户邮箱 |
| `iat` | Token 签发时间 (Issued At) |
| `exp` | Token 过期时间 (Expiration) |

### Token 有效期

默认 24 小时，可通过 `JWT_EXPIRATION_SECS` 环境变量配置。
