# 房间管理 API

管理 LiveKit 房间的创建、列表、删除和结束会议。

---

## GET /api/rooms

获取所有活跃房间列表。

### 请求

```bash
curl -X GET http://localhost:8081/api/rooms
```

### 响应

#### 成功 (200 OK)

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

### 字段说明

| 字段 | 类型 | 描述 |
|------|------|------|
| `name` | string | 房间唯一标识名称 |
| `displayName` | string | 房间显示名称 |
| `participants` | number | 当前参与者数量 |
| `createdAt` | string | 房间创建时间（ISO 8601 格式） |

---

## POST /api/rooms

创建新房间。

### 请求

```bash
curl -X POST http://localhost:8081/api/rooms \
  -H "Content-Type: application/json" \
  -d '{
    "name": "new-meeting-room"
  }'
```

### 请求体

| 字段 | 类型 | 必填 | 默认值 | 描述 |
|------|------|------|--------|------|
| `name` | string | 否 | `"room-{timestamp}"` | 房间名称 |

### 响应

#### 成功 (201 Created)

```json
{
  "name": "new-meeting-room",
  "displayName": "new-meeting-room",
  "participants": 0,
  "createdAt": "2026-01-21T10:30:00.000Z"
}
```

#### 请求参数无效 (400 Bad Request)

```json
{
  "error": "Failed to deserialize the JSON body into the target type"
}
```

### 房间默认配置

| 配置项 | 值 | 描述 |
|--------|-----|------|
| `emptyTimeout` | 300 秒 | 空房间自动删除时间 |
| `maxParticipants` | 50 | 最大参与者数量 |

---

## DELETE /api/rooms/{room_name}

删除指定房间。

### 请求

```bash
curl -X DELETE http://localhost:8081/api/rooms/meeting-room-1
```

### 路径参数

| 参数 | 类型 | 描述 |
|------|------|------|
| `room_name` | string | 房间名称 |

### 响应

#### 成功 (200 OK)

```json
{
  "message": "Room deleted"
}
```

#### 服务器错误 (500 Internal Server Error)

```json
{
  "error": "Room not found"
}
```

---

## POST /api/rooms/{room_name}/end

结束会议：移除所有参与者并删除房间。

### 请求

```bash
curl -X POST http://localhost:8081/api/rooms/meeting-room-1/end
```

### 路径参数

| 参数 | 类型 | 描述 |
|------|------|------|
| `room_name` | string | 房间名称 |

### 响应

#### 成功 (200 OK)

```json
{
  "message": "Meeting ended, 3 participants removed"
}
```

#### 服务器错误 (500 Internal Server Error)

```json
{
  "error": "Failed to end meeting"
}
```

---

## 使用示例

### 完整房间生命周期

```bash
# 1. 创建房间
curl -X POST http://localhost:8081/api/rooms \
  -H "Content-Type: application/json" \
  -d '{"name": "team-standup"}'

# 2. 查看所有房间
curl -X GET http://localhost:8081/api/rooms

# 3. 结束会议（踢出所有人并删除房间）
curl -X POST http://localhost:8081/api/rooms/team-standup/end

# 或者直接删除房间
curl -X DELETE http://localhost:8081/api/rooms/team-standup
```

### 列出活跃房间

```bash
curl -s http://localhost:8081/api/rooms | jq '.[] | {name, participants}'
```

输出：
```json
{"name": "meeting-1", "participants": 5}
{"name": "meeting-2", "participants": 2}
```
