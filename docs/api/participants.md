# 参与者管理 API

管理房间内的参与者，包括列表查看和移除操作。

---

## GET /api/rooms/{room_name}/participants

获取指定房间的参与者列表。

### 请求

```bash
curl -X GET http://localhost:8081/api/rooms/meeting-room-1/participants
```

### 路径参数

| 参数 | 类型 | 描述 |
|------|------|------|
| `room_name` | string | 房间名称 |

### 响应

#### 成功 (200 OK)

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

#### 服务器错误 (500 Internal Server Error)

```json
{
  "error": "Room not found"
}
```

### 字段说明

| 字段 | 类型 | 描述 |
|------|------|------|
| `sid` | string | 参与者会话 ID |
| `identity` | string | 参与者唯一标识 |
| `state` | number | 连接状态（1=已连接） |
| `metadata` | string | 参与者元数据（JSON 字符串） |
| `joinedAt` | number | 加入时间（Unix 时间戳，秒） |
| `name` | string | 参与者显示名称 |
| `isPublisher` | boolean | 是否正在发布音视频 |

### 连接状态值

| 值 | 状态 | 描述 |
|-----|------|------|
| 0 | JOINING | 正在加入 |
| 1 | JOINED | 已加入 |
| 2 | ACTIVE | 活跃中 |
| 3 | DISCONNECTED | 已断开 |

---

## DELETE /api/rooms/{room_name}/participants/{identity}

将指定参与者移出房间（踢人）。

### 请求

```bash
curl -X DELETE http://localhost:8081/api/rooms/meeting-room-1/participants/user-002
```

### 路径参数

| 参数 | 类型 | 描述 |
|------|------|------|
| `room_name` | string | 房间名称 |
| `identity` | string | 参与者标识 |

### 响应

#### 成功 (200 OK)

```json
{
  "message": "Participant removed",
  "identity": "user-002"
}
```

#### 服务器错误 (500 Internal Server Error)

```json
{
  "error": "Participant not found"
}
```

### 字段说明

| 字段 | 类型 | 描述 |
|------|------|------|
| `message` | string | 操作结果描述 |
| `identity` | string | 被移除的参与者标识 |

---

## 使用示例

### 查看房间参与者

```bash
# 获取参与者列表
curl -s http://localhost:8081/api/rooms/team-meeting/participants | jq .

# 只显示参与者名称和状态
curl -s http://localhost:8081/api/rooms/team-meeting/participants | \
  jq '.participants[] | {name, isPublisher}'
```

### 踢出指定参与者

```bash
# 移除 identity 为 "user-002" 的参与者
curl -X DELETE http://localhost:8081/api/rooms/team-meeting/participants/user-002
```

### 踢出所有非主持人

```bash
# 获取非主持人列表并逐个踢出
curl -s http://localhost:8081/api/rooms/team-meeting/participants | \
  jq -r '.participants[] | select(.metadata | fromjson | .isHost != true) | .identity' | \
  while read identity; do
    curl -X DELETE "http://localhost:8081/api/rooms/team-meeting/participants/$identity"
  done
```

### 监控房间参与者数量

```bash
# 每 5 秒检查一次参与者数量
while true; do
  count=$(curl -s http://localhost:8081/api/rooms/team-meeting/participants | jq '.participants | length')
  echo "$(date): $count participants"
  sleep 5
done
```
