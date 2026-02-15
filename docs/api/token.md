# Token 管理 API

生成 LiveKit 访问令牌，用于客户端连接 LiveKit 实时音视频服务。

---

## POST /api/token

生成 LiveKit 访问令牌（仅用于加入已存在的普通房间）。

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
| `roomName` | string | 是 | 无 | 已存在的普通房间名称 |
| `participantName` | string | 否 | `"user-{timestamp}"` | 参与者名称 |
| `isHost` | boolean | 否 | `false` | 客户端可传，但服务端游客模式下固定返回 `false` |

> 字符串字段会先 `trim`。
>
> 限制：
> - `roomName` 不能为空，否则返回 `400`；
> - 只允许加入已存在房间，不会隐式创建房间；不存在返回 `404`；
> - `roomName` 若命中业务会议命名（`m-#########`）会返回 `403`，需改用会议体系接口（登录用户用 `/api/meetings/{meeting_no}/join`，游客用 `/api/meetings/{meeting_no}/guest-join`）。

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
| `isHost` | boolean | 是否为主持人（`/api/token` 游客模式下固定为 `false`） |

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

`/api/token` 面向游客加入场景，服务端不会授予主持人标记，`isHost` 固定为 `false`（并写入 token metadata）。

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

### 游客加入已存在房间

```bash
curl -X POST http://localhost:8081/api/token \
  -H "Content-Type: application/json" \
  -d '{
    "roomName": "team-meeting",
    "participantName": "访客"
  }'
```

### 房间不存在（返回 404）

```bash
curl -X POST http://localhost:8081/api/token \
  -H "Content-Type: application/json" \
  -d '{"roomName":"nonexistent-room","participantName":"访客"}'
```

```json
{
  "error": "Room not found"
}
```

### 业务会议房间名（返回 403）

```bash
curl -X POST http://localhost:8081/api/token \
  -H "Content-Type: application/json" \
  -d '{"roomName":"m-123456789","participantName":"访客"}'
```

```json
{
  "error": "Business meetings must be joined via /api/meetings/{meeting_no}/join"
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
