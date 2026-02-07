# Token 管理 API

生成 LiveKit 访问令牌，用于客户端连接 LiveKit 实时音视频服务。

---

## POST /api/token

生成 LiveKit 访问令牌。

### 请求

```bash
curl -X POST http://localhost:8081/api/token \
  -H "Content-Type: application/json" \
  -d '{
    "roomName": "meeting-room-1",
    "participantName": "张三",
    "isHost": false
  }'
```

### 请求体

| 字段 | 类型 | 必填 | 默认值 | 描述 |
|------|------|------|--------|------|
| `roomName` | string | 否 | `"default-room"` | 房间名称 |
| `participantName` | string | 否 | `"user-{timestamp}"` | 参与者名称 |
| `isHost` | boolean | 否 | `false` | 是否请求主持人权限 |

> 字符串字段会先 `trim`；当字段缺失或为空字符串时，会使用默认值。

### 响应

#### 成功 (200 OK)

```json
{
  "token": "eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9...",
  "url": "wss://livekit.example.com",
  "roomName": "meeting-room-1",
  "isHost": true
}
```

#### 请求参数无效（4xx Client Error）

```json
{
  "error": "Failed to deserialize the JSON body into the target type"
}
```

> 说明：具体状态码由 Axum 的 JSON 解析失败类型决定，常见为 `400` 或 `422`。

#### 服务器错误 (500 Internal Server Error)

```json
{
  "error": "Failed to generate token: ..."
}
```

### 字段说明

| 字段 | 类型 | 描述 |
|------|------|------|
| `token` | string | LiveKit JWT 访问令牌 |
| `url` | string | LiveKit WebSocket 服务地址 |
| `roomName` | string | 实际使用的房间名称 |
| `isHost` | boolean | 是否为主持人（首个加入者会自动成为主持人） |

---

## Token 权限说明

生成的 Token 包含以下 VideoGrant 权限：

| 权限 | 值 | 描述 |
|------|-----|------|
| `roomJoin` | `true` | 允许加入房间 |
| `room` | 房间名 | 限制只能加入指定房间 |
| `canPublish` | `true` | 允许发布音视频 |
| `canSubscribe` | `true` | 允许订阅其他参与者 |

### `isHost` 字段说明

`isHost` 是服务端返回给客户端的业务标记，当前判定逻辑为：

1. 请求体中显式传入 `isHost: true`；或
2. 请求房间当前无参与者（或房间尚不存在）时自动设为 `true`。

该字段会被写入 token metadata（例如 `{"isHost":true}`），用于客户端业务判断。

---

## 使用示例

### 普通参与者加入房间

```bash
curl -X POST http://localhost:8081/api/token \
  -H "Content-Type: application/json" \
  -d '{
    "roomName": "team-meeting",
    "participantName": "参与者A"
  }'
```

### 主持人加入房间

```bash
curl -X POST http://localhost:8081/api/token \
  -H "Content-Type: application/json" \
  -d '{
    "roomName": "team-meeting",
    "participantName": "主持人",
    "isHost": true
  }'
```

### 使用默认值

```bash
# 不传任何参数，使用默认房间名和自动生成的用户名
curl -X POST http://localhost:8081/api/token \
  -H "Content-Type: application/json" \
  -d '{}'
```

返回示例：

```json
{
  "token": "eyJ...",
  "url": "ws://localhost:7880",
  "roomName": "default-room",
  "isHost": true
}
```

---

## 客户端使用 Token

### JavaScript/TypeScript

```typescript
import { Room } from 'livekit-client';

const response = await fetch('http://localhost:8081/api/token', {
  method: 'POST',
  headers: { 'Content-Type': 'application/json' },
  body: JSON.stringify({
    roomName: 'my-room',
    participantName: 'User'
  })
});

const { token, url } = await response.json();

const room = new Room();
await room.connect(url, token);
```

### Swift (iOS)

```swift
let room = Room()
try await room.connect(url: url, token: token)
```

### Kotlin (Android)

```kotlin
val room = LiveKit.connect(
    url = url,
    token = token
)
```
