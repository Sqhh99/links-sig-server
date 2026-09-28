# 用户认证 API

用户账号相关接口：登录（首次登录即创建账号）与刷新 JWT。没有单独的注册接口，也不做邮箱验证。

---

## POST /api/auth/login

用户名 + 密码登录，返回用户 JWT。用户名不存在时，服务端会校验用户名和密码规则，通过后直接创建账号并登录。

### 请求

```bash
curl -X POST http://localhost:8081/api/auth/login \
  -H "Content-Type: application/json" \
  -d '{
    "username": "alice",
    "password": "SecurePass123"
  }'
```

### 请求体

| 字段 | 类型 | 必填 | 描述 |
|------|------|------|------|
| `username` | string | 是 | 用户名（首尾空白会被去掉） |
| `password` | string | 是 | 密码 |

### 成功响应

已有账号返回 `200 OK`，本次登录新建了账号则返回 `201 Created`，响应体相同：

```json
{
  "userId": "550e8400-e29b-41d4-a716-446655440000",
  "username": "alice",
  "token": "eyJ...",
  "displayName": "张三",
  "accountCreated": false
}
```

| 字段 | 类型 | 描述 |
|------|------|------|
| `userId` | string | 用户 ID |
| `username` | string | 用户名，保持创建时的大小写 |
| `token` | string | 用户 JWT |
| `displayName` | string | 用户展示名（未设置时省略） |
| `accountCreated` | boolean | 本次登录是否新建了账号 |

### 可能错误

| 状态码 | `code` | 示例 |
|--------|--------|------|
| 400 | — | `{"error":"Username and password are required"}` |
| 400 | `INVALID_USERNAME` | `{"error":"Username must be 2-32 characters","code":"INVALID_USERNAME"}` |
| 400 | `WEAK_PASSWORD` | `{"error":"Password must contain both letters and digits","code":"WEAK_PASSWORD"}` |
| 401 | `INVALID_CREDENTIALS` | `{"error":"Username is taken or password is incorrect","code":"INVALID_CREDENTIALS"}` |
| 422 | — | 请求体缺少 `username` 或 `password` 字段 |

### 规则

- 用户名匹配不区分大小写，`Alice` 与 `alice` 是同一个账号。
- 以下规则只在创建账号时检查：
  - **用户名**：2–32 个字符，只能包含字母（任意文字）、数字、`_`、`-`、`.`。
  - **密码**：8–128 个字符，同时包含英文字母和数字，且不能与用户名相同（不区分大小写）。
- 用户名已存在而密码不匹配时返回 401。服务端无法区分“输错密码”和“想用的用户名已被占用”，因此错误信息同时提示这两种情况。
- 旧版本以邮箱注册的账号，迁移后把邮箱作为用户名，仍可用“邮箱 + 原密码”登录。

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
  "username": "alice",
  "token": "eyJ...",
  "expiresInSecs": 604800,
  "displayName": "张三"
}
```

### 字段说明

| 字段 | 类型 | 描述 |
|------|------|------|
| `userId` | string | 用户 ID |
| `username` | string | 用户名 |
| `token` | string | 新签发的 JWT |
| `expiresInSecs` | number | 新 token 有效期（秒） |
| `displayName` | string | 用户展示名（未设置时省略） |

### 可能错误

| 状态码 | 示例 |
|--------|------|
| 401 | `{"error":"Authorization header required"}` |
| 401 | `{"error":"Token has expired"}` |
| 401 | `{"error":"Invalid token format"}` |
| 401 | `{"error":"User not found"}` |

---

## JWT 说明

用户 JWT（来自登录）为 HS256 签名，Payload 主要字段：

```json
{
  "sub": "550e8400-e29b-41d4-a716-446655440000",
  "username": "alice",
  "iat": 1737452400,
  "nbf": 1737452400,
  "exp": 1738057200
}
```

默认有效期为 `604800` 秒（7 天），可通过 `JWT_EXPIRATION_SECS` 调整。

升级前签发的 JWT 用 `email` 字段携带同一个值，服务端仍然接受，直到它们过期。
