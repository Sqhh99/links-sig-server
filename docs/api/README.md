# Links Signaling Server API 文档

本目录包含 Links Signaling Server 的完整 API 文档。

## 文档索引

| 模块 | 描述 | 文件 |
|------|------|------|
| [概述](./overview.md) | 基础信息、认证、通用响应格式 | `overview.md` |
| [健康检查](./health.md) | 服务健康状态检查 | `health.md` |
| [用户认证](./auth.md) | 注册、登录、邮箱验证 | `auth.md` |
| [Token 管理](./token.md) | LiveKit 访问令牌生成 | `token.md` |
| [房间管理](./rooms.md) | 房间的创建、列表、删除 | `rooms.md` |
| [参与者管理](./participants.md) | 参与者列表、踢人 | `participants.md` |

## 快速开始

### 基础 URL

```
http://localhost:8081
```

### 完整会议流程示例

```bash
# 1. 用户注册
curl -X POST http://localhost:8081/api/auth/register/request-code \
  -H "Content-Type: application/json" \
  -d '{"email": "user@example.com"}'

# 2. 使用验证码完成注册
curl -X POST http://localhost:8081/api/auth/register \
  -H "Content-Type: application/json" \
  -d '{
    "email": "user@example.com",
    "password": "SecurePass123!",
    "code": "123456"
  }'

# 3. 登录获取用户 Token
curl -X POST http://localhost:8081/api/auth/login \
  -H "Content-Type: application/json" \
  -d '{
    "email": "user@example.com",
    "password": "SecurePass123!"
  }'

# 4. 创建房间
curl -X POST http://localhost:8081/api/rooms \
  -H "Content-Type: application/json" \
  -d '{"name": "team-meeting"}'

# 5. 获取 LiveKit Token 加入房间
curl -X POST http://localhost:8081/api/token \
  -H "Content-Type: application/json" \
  -d '{
    "roomName": "team-meeting",
    "participantName": "张三",
    "isHost": true
  }'

# 6. 查看房间参与者
curl -X GET http://localhost:8081/api/rooms/team-meeting/participants

# 7. 结束会议
curl -X POST http://localhost:8081/api/rooms/team-meeting/end
```

## API 端点一览

| 方法 | 路径 | 描述 |
|------|------|------|
| GET | `/api/health` | 健康检查 |
| POST | `/api/auth/register/request-code` | 请求注册验证码 |
| POST | `/api/auth/register` | 用户注册 |
| POST | `/api/auth/login` | 用户登录 |
| POST | `/api/token` | 生成 LiveKit Token |
| GET | `/api/rooms` | 获取房间列表 |
| POST | `/api/rooms` | 创建房间 |
| DELETE | `/api/rooms/{room_name}` | 删除房间 |
| POST | `/api/rooms/{room_name}/end` | 结束会议 |
| GET | `/api/rooms/{room_name}/participants` | 获取参与者列表 |
| DELETE | `/api/rooms/{room_name}/participants/{identity}` | 移除参与者 |
