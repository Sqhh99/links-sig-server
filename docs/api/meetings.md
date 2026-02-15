# 会议号与会议记录 API

业务会议接口：服务端自动生成 `9` 位数字会议号，支持分享链接加入/离开，以及用户维度的会议记录查询。

---

## 认证要求

以下接口都需要用户 JWT（登录或注册返回的 token）：

```http
Authorization: Bearer <user-jwt>
```

未携带或无效时返回：

```json
{
  "error": "Authorization header required"
}
```

---

## POST /api/meetings

创建会议。服务端会自动随机生成唯一 `9` 位会议号（例如 `012345678`）。

### 请求

```bash
curl -X POST http://localhost:8081/api/meetings \
  -H "Authorization: Bearer <user-jwt>"
```

### 成功响应（201 Created）

```json
{
  "meetingNo": "123456789",
  "roomName": "m-123456789",
  "shareUrl": "http://localhost:3000/join?meetingNo=123456789",
  "createdAt": "2026-02-07T12:34:56Z"
}
```

### 字段说明

| 字段 | 类型 | 描述 |
|------|------|------|
| `meetingNo` | string | 9 位数字会议号 |
| `roomName` | string | 对应 LiveKit 房间名（格式：`m-{meetingNo}`） |
| `shareUrl` | string | 客户端可直接分享的入会链接 |
| `createdAt` | string | 创建时间（ISO 8601） |

---

## POST /api/meetings/{meeting_no}/join

通过会议号加入会议，并返回 LiveKit 连接 token。

### 请求

```bash
curl -X POST http://localhost:8081/api/meetings/123456789/join \
  -H "Authorization: Bearer <user-jwt>" \
  -H "Content-Type: application/json" \
  -d '{
    "participantName": "张三"
  }'
```

### 路径参数

| 参数 | 类型 | 描述 |
|------|------|------|
| `meeting_no` | string | 9 位数字会议号 |

### 请求体

| 字段 | 类型 | 必填 | 默认值 | 描述 |
|------|------|------|--------|------|
| `participantName` | string | 否 | `displayName -> 用户邮箱` | 进入会议时的显示名（不作为 identity） |

> 可传 `{}`，服务端会优先回退为当前用户 `displayName`，未设置时再回退为邮箱。
>
> 服务端会用当前用户 JWT 的 `user_id` 作为 LiveKit identity，客户端不能自定义 identity。

### 成功响应（200 OK）

```json
{
  "meetingNo": "123456789",
  "token": "eyJ...",
  "url": "ws://localhost:7880",
  "roomName": "m-123456789",
  "isHost": false
}
```

### 可能错误

| 状态码 | 示例 |
|--------|------|
| 400 | `{"error":"meeting_no must be 9 digits"}` |
| 404 | `{"error":"Meeting not found"}` |
| 409 | `{"error":"Meeting has ended"}` |

---

## POST /api/meetings/{meeting_no}/leave

按会议号离开会议（只允许当前登录用户离开自己会话，幂等）。

当离会后该会议房间已无人时，服务端会把 `meetings.status` 从 `active` 更新为 `ended`（并写入 `ended_at`）。后续再调用 `/join` 会返回 `409`。

### 请求

```bash
curl -X POST http://localhost:8081/api/meetings/123456789/leave \
  -H "Authorization: Bearer <user-jwt>"
```

### 路径参数

| 参数 | 类型 | 描述 |
|------|------|------|
| `meeting_no` | string | 9 位数字会议号 |

### 成功响应（200 OK）

```json
{
  "message": "Left meeting",
  "meetingNo": "123456789",
  "roomName": "m-123456789",
  "identity": "550e8400-e29b-41d4-a716-446655440000",
  "left": true
}
```

重复调用（用户已不在房间）：

```json
{
  "message": "Already left meeting",
  "meetingNo": "123456789",
  "roomName": "m-123456789",
  "identity": "550e8400-e29b-41d4-a716-446655440000",
  "left": false
}
```

### 字段说明

| 字段 | 类型 | 描述 |
|------|------|------|
| `message` | string | 离会结果描述 |
| `meetingNo` | string | 9 位会议号 |
| `roomName` | string | 对应房间名 |
| `identity` | string | 当前用户 identity（固定为 JWT `user_id`） |
| `left` | boolean | 本次调用是否实际执行了移除 |

### 可能错误

| 状态码 | 示例 |
|--------|------|
| 400 | `{"error":"meeting_no must be 9 digits"}` |
| 401 | `{"error":"Authorization header required"}` |
| 404 | `{"error":"Meeting not found"}` |

---

## GET /api/me/meeting-records

查询当前用户的会议记录（按最近加入时间倒序）。

### 请求

```bash
curl -X GET "http://localhost:8081/api/me/meeting-records?page=1&pageSize=20" \
  -H "Authorization: Bearer <user-jwt>"
```

### 查询参数

| 参数 | 类型 | 必填 | 默认值 | 描述 |
|------|------|------|--------|------|
| `page` | number | 否 | `1` | 页码（最小 1） |
| `pageSize` | number | 否 | `20` | 每页条数（`1~100`） |

### 成功响应（200 OK）

```json
{
  "records": [
    {
      "meetingNo": "123456789",
      "roomName": "m-123456789",
      "meetingStatus": "active",
      "creatorUserId": "550e8400-e29b-41d4-a716-446655440000",
      "firstJoinedAt": "2026-02-07T12:35:10Z",
      "lastJoinedAt": "2026-02-07T13:01:42Z",
      "joinCount": 2
    }
  ],
  "page": 1,
  "pageSize": 20
}
```

### 记录去重规则

- 同一用户重复加入同一会议，不会新增多条记录；
- 会更新该条记录的 `lastJoinedAt`；
- `joinCount` 自增。
- `meetingStatus` 由服务端会议生命周期维护：
  - 默认创建后为 `active`；
  - 当会议房间无人（例如最后一人 `leave`）或主持人调用 `/rooms/{room_name}/end` 后会变为 `ended`。
