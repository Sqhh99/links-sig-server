# LiveKit Signaling Server API 文档

本文档描述了 LiveKit Signaling Server 提供的所有 HTTP API 接口。

---

## 目录

- [概述](#概述)
- [认证](#认证)
- [通用响应格式](#通用响应格式)
- [API 端点](#api-端点)
  - [健康检查](#健康检查)
  - [Token 管理](#token-管理)
  - [房间管理](#房间管理)
  - [参与者管理](#参与者管理)
- [错误码](#错误码)

---

## 概述

### 基础信息

| 项目 | 值 |
|------|-----|
| 基础 URL | `http://localhost:8081` |
| 协议 | HTTP/HTTPS |
| 数据格式 | JSON |
| 字符编码 | UTF-8 |

### 请求头

所有 POST/PUT/PATCH 请求需要设置：

```
Content-Type: application/json
```

---

## 认证

当前版本 API 不需要额外认证。生成的 LiveKit Token 包含访问 LiveKit 服务所需的所有权限。

---

## 通用响应格式

### 成功响应

```json
{
  "field1": "value1",
  "field2": "value2"
}
```

### 错误响应

```json
{
  "error": "错误描述信息"
}
```

---

## API 端点

### 健康检查

#### GET /health

检查服务器健康状态。

**请求**

```bash
curl -X GET http://localhost:8081/health
```

**响应**

| 状态码 | 描述 |
|--------|------|
| 200 | 服务正常 |

```json
{
  "status": "ok",
  "time": "2026-01-21T10:30:00.000Z"
}
```

**字段说明**

| 字段 | 类型 | 描述 |
|------|------|------|
| `status` | string | 服务状态，始终为 `"ok"` |
| `time` | string | 当前服务器时间（ISO 8601 格式） |

---

### Token 管理

#### POST /token

生成 LiveKit 访问令牌，用于客户端连接 LiveKit 服务。

**请求**

```bash
curl -X POST http://localhost:8081/token \
  -H "Content-Type: application/json" \
  -d '{
    "roomName": "meeting-room-1",
    "participantName": "张三",
    "isHost": false
  }'
```

**请求体**

| 字段 | 类型 | 必填 | 默认值 | 描述 |
|------|------|------|--------|------|
| `roomName` | string | 否 | `"default-room"` | 房间名称 |
| `participantName` | string | 否 | `"user-{timestamp}"` | 参与者名称 |
| `isHost` | boolean | 否 | `false` | 是否请求主持人权限 |

**响应**

| 状态码 | 描述 |
|--------|------|
| 200 | 成功生成 Token |
| 400 | 请求参数无效 |

```json
{
  "token": "eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9...",
  "url": "wss://livekit.example.com",
  "roomName": "meeting-room-1",
  "isHost": true
}
```

**字段说明**

| 字段 | 类型 | 描述 |
|------|------|------|
| `token` | string | LiveKit JWT 访问令牌 |
| `url` | string | LiveKit WebSocket 服务地址 |
| `roomName` | string | 实际使用的房间名称 |
| `isHost` | boolean | 是否为主持人（如果是首个加入者会自动成为主持人） |

**Token 权限说明**

生成的 Token 包含以下 VideoGrant 权限：
- `roomJoin`: true - 允许加入房间
- `room`: 指定房间名 - 限制只能加入该房间
- `canPublish`: true - 允许发布音视频
- `canSubscribe`: true - 允许订阅其他参与者

---

### 房间管理

#### GET /rooms

获取所有活跃房间列表。

**请求**

```bash
curl -X GET http://localhost:8081/rooms
```

**响应**

| 状态码 | 描述 |
|--------|------|
| 200 | 成功返回房间列表 |

```json
[
  {
    "name": "meeting-room-1",
    "displayName": "meeting-room-1",
    "participants": 3,
    "createdAt": "2026-01-21T09:00:00.000Z"
  },
  {
    "name": "meeting-room-2",
    "displayName": "meeting-room-2",
    "participants": 1,
    "createdAt": "2026-01-21T10:15:00.000Z"
  }
]
```

**字段说明**

| 字段 | 类型 | 描述 |
|------|------|------|
| `name` | string | 房间唯一标识名称 |
| `displayName` | string | 房间显示名称 |
| `participants` | number | 当前参与者数量 |
| `createdAt` | string | 房间创建时间（ISO 8601 格式） |

---

#### POST /rooms

创建新房间。

**请求**

```bash
curl -X POST http://localhost:8081/rooms \
  -H "Content-Type: application/json" \
  -d '{
    "name": "new-meeting-room"
  }'
```

**请求体**

| 字段 | 类型 | 必填 | 默认值 | 描述 |
|------|------|------|--------|------|
| `name` | string | 否 | `"room-{timestamp}"` | 房间名称 |

**响应**

| 状态码 | 描述 |
|--------|------|
| 201 | 房间创建成功 |
| 400 | 请求参数无效 |

```json
{
  "name": "new-meeting-room",
  "displayName": "new-meeting-room",
  "participants": 0,
  "createdAt": "2026-01-21T10:30:00.000Z"
}
```

**房间默认配置**

| 配置项 | 值 | 描述 |
|--------|-----|------|
| `emptyTimeout` | 300 秒 | 空房间自动删除时间 |
| `maxParticipants` | 50 | 最大参与者数量 |

---

#### DELETE /rooms/{room_name}

删除指定房间。

**请求**

```bash
curl -X DELETE http://localhost:8081/rooms/meeting-room-1
```

**路径参数**

| 参数 | 类型 | 描述 |
|------|------|------|
| `room_name` | string | 房间名称 |

**响应**

| 状态码 | 描述 |
|--------|------|
| 200 | 房间删除成功 |
| 500 | 服务器错误（房间不存在等） |

```json
{
  "message": "Room deleted"
}
```

---

#### POST /rooms/{room_name}/end

结束会议：移除所有参与者并删除房间。

**请求**

```bash
curl -X POST http://localhost:8081/rooms/meeting-room-1/end
```

**路径参数**

| 参数 | 类型 | 描述 |
|------|------|------|
| `room_name` | string | 房间名称 |

**响应**

| 状态码 | 描述 |
|--------|------|
| 200 | 会议结束成功 |
| 500 | 服务器错误 |

```json
{
  "message": "Meeting ended, 3 participants removed"
}
```

---

### 参与者管理

#### GET /rooms/{room_name}/participants

获取指定房间的参与者列表。

**请求**

```bash
curl -X GET http://localhost:8081/rooms/meeting-room-1/participants
```

**路径参数**

| 参数 | 类型 | 描述 |
|------|------|------|
| `room_name` | string | 房间名称 |

**响应**

| 状态码 | 描述 |
|--------|------|
| 200 | 成功返回参与者列表 |
| 500 | 服务器错误（房间不存在等） |

```json
{
  "participants": [
    {
      "sid": "PA_xxxxx",
      "identity": "user-001",
      "state": 1,
      "metadata": "{\"isHost\":true}",
      "joinedAt": 1737452400,
      "name": "张三",
      "isPublisher": true
    },
    {
      "sid": "PA_yyyyy",
      "identity": "user-002",
      "state": 1,
      "metadata": "{\"isHost\":false}",
      "joinedAt": 1737452500,
      "name": "李四",
      "isPublisher": false
    }
  ]
}
```

**字段说明**

| 字段 | 类型 | 描述 |
|------|------|------|
| `sid` | string | 参与者会话 ID |
| `identity` | string | 参与者唯一标识 |
| `state` | number | 连接状态（1=已连接） |
| `metadata` | string | 参与者元数据（JSON 字符串） |
| `joinedAt` | number | 加入时间（Unix 时间戳） |
| `name` | string | 参与者显示名称 |
| `isPublisher` | boolean | 是否正在发布音视频 |

---

#### DELETE /rooms/{room_name}/participants/{identity}

将指定参与者移出房间（踢人）。

**请求**

```bash
curl -X DELETE http://localhost:8081/rooms/meeting-room-1/participants/user-002
```

**路径参数**

| 参数 | 类型 | 描述 |
|------|------|------|
| `room_name` | string | 房间名称 |
| `identity` | string | 参与者标识 |

**响应**

| 状态码 | 描述 |
|--------|------|
| 200 | 参与者移除成功 |
| 500 | 服务器错误 |

```json
{
  "message": "Participant removed",
  "identity": "user-002"
}
```

---

## 错误码

### HTTP 状态码

| 状态码 | 描述 | 常见原因 |
|--------|------|---------|
| 200 | OK | 请求成功 |
| 201 | Created | 资源创建成功 |
| 400 | Bad Request | 请求参数无效、JSON 格式错误 |
| 401 | Unauthorized | 认证失败（保留） |
| 404 | Not Found | 资源不存在 |
| 500 | Internal Server Error | 服务器内部错误 |

### 错误响应示例

**400 Bad Request**

```json
{
  "error": "Failed to deserialize the JSON body into the target type: missing field `name`"
}
```

**500 Internal Server Error**

```json
{
  "error": "Failed to list rooms: connection refused"
}
```

---

## 使用示例

### 完整会议流程

```bash
# 1. 创建房间
curl -X POST http://localhost:8081/rooms \
  -H "Content-Type: application/json" \
  -d '{"name": "team-meeting"}'

# 2. 主持人获取 Token
curl -X POST http://localhost:8081/token \
  -H "Content-Type: application/json" \
  -d '{
    "roomName": "team-meeting",
    "participantName": "主持人",
    "isHost": true
  }'

# 3. 参与者获取 Token
curl -X POST http://localhost:8081/token \
  -H "Content-Type: application/json" \
  -d '{
    "roomName": "team-meeting",
    "participantName": "参与者A"
  }'

# 4. 查看房间参与者
curl -X GET http://localhost:8081/rooms/team-meeting/participants

# 5. 踢出某个参与者
curl -X DELETE http://localhost:8081/rooms/team-meeting/participants/参与者A

# 6. 结束会议
curl -X POST http://localhost:8081/rooms/team-meeting/end
```

---

## 环境配置

服务器通过环境变量配置：

| 变量 | 默认值 | 描述 |
|------|--------|------|
| `LIVEKIT_URL` | `http://localhost:7880` | LiveKit API 地址 |
| `LIVEKIT_WS_URL` | `ws://localhost:7880` | LiveKit WebSocket 地址 |
| `LIVEKIT_API_KEY` | `devkey` | LiveKit API Key |
| `LIVEKIT_API_SECRET` | `secret` | LiveKit API Secret |
| `SERVER_PORT` | `8081` | 服务器端口 |
| `SERVER_HOST` | `localhost` | 服务器主机 |
| `ENABLE_HTTPS` | `false` | 是否启用 HTTPS |
| `SSL_CERT_FILE` | `./certs/server.crt` | SSL 证书路径 |
| `SSL_KEY_FILE` | `./certs/server.key` | SSL 密钥路径 |
