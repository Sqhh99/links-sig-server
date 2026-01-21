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

#### 请求参数无效 (400 Bad Request)

```json
{
  "error": "Failed to deserialize the JSON body into the target type"
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

### 主持人额外权限

当 `isHost: true` 时，Token 还包含：

| 权限 | 值 | 描述 |
|------|-----|------|
| `roomAdmin` | `true` | 房间管理权限 |
| `roomRecord` | `true` | 录制权限 |

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
