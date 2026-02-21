# 会议号与预定会议 API

业务会议基于 `9` 位会议号（例如 `123456789`），支持预定开始时间、主持者开启、游客准入、会议密码、自动关闭。

## 关键时间与状态语义

- `scheduledStartAt`：预定开始时间。
- `openedAt`：主持者实际开启时间。
- `endedAt`：实际结束时间（手动结束或自动关闭）。
- `status`：`scheduled | open | ended | cancelled`。

> 重要：会议到达 `scheduledStartAt` 只表示进入“可开启窗口”，并不代表可加入。  
> 只有主持者先成功加入并触发开启（写入 `openedAt`）后，其他成员/游客才可加入。

---

## 认证要求

需要用户 JWT：

- `POST /api/meetings`
- `POST /api/meetings/{meeting_no}/join`
- `POST /api/meetings/{meeting_no}/leave`
- `POST /api/meetings/{meeting_no}/cancel`
- `GET /api/me/meeting-records`
- `GET /api/me/host-meetings`

无需用户 JWT：

- `POST /api/meetings/{meeting_no}/guest-join`

---

## POST /api/meetings

创建预定会议。

### 请求示例

```bash
curl -X POST http://localhost:8081/api/meetings \
  -H "Authorization: Bearer <user-jwt>" \
  -H "Content-Type: application/json" \
  -d '{
    "topic": "项目周会",
    "scheduledStartAt": "2026-02-20T14:30:00Z",
    "allowGuestJoin": true,
    "password": "secret12",
    "noJoinAutoEndMinutes": 15,
    "emptyAutoEndMinutes": 10
  }'
```

### 请求字段

| 字段 | 类型 | 必填 | 默认值 | 说明 |
|------|------|------|--------|------|
| `topic` | string | 否 | `""` | 会议主题（最长 200） |
| `scheduledStartAt` | string | 否 | `now` | UTC ISO-8601 |
| `allowGuestJoin` | boolean | 否 | `false` | 是否允许游客加入 |
| `password` | string | 否 | 无 | 会议密码，长度 `6~32` |
| `noJoinAutoEndMinutes` | number | 否 | `15` | 开始后无人入会自动结束阈值 |
| `emptyAutoEndMinutes` | number | 否 | `10` | 开启后空房自动结束阈值 |

### 成功响应（201）

```json
{
  "meetingNo": "123456789",
  "roomName": "m-123456789",
  "shareUrl": "http://localhost:3000/join?meetingNo=123456789",
  "status": "scheduled",
  "topic": "项目周会",
  "scheduledStartAt": "2026-02-20T14:30:00Z",
  "allowGuestJoin": true,
  "requiresPassword": true,
  "noJoinAutoEndMinutes": 15,
  "emptyAutoEndMinutes": 10,
  "createdAt": "2026-02-20T13:00:00Z"
}
```

---

## POST /api/meetings/{meeting_no}/join

登录用户按会议号加入会议（主持者可触发开启）。

### 请求示例

```bash
curl -X POST http://localhost:8081/api/meetings/123456789/join \
  -H "Authorization: Bearer <user-jwt>" \
  -H "Content-Type: application/json" \
  -d '{
    "participantName": "张三",
    "meetingPassword": "secret12"
  }'
```

### 规则

- `meeting_no` 必须是 `9` 位数字。
- 当前用户为主持者（`JWT user_id == creator_user_id`）时：
  - 到达预定时间后首次加入会触发 `scheduled -> open`。
  - 主持者可免输入会议密码。
- 非主持者：
  - 未到预定时间：`409` + `code=MEETING_NOT_STARTED`
  - 到点但主持者未开启：`409` + `code=HOST_NOT_JOINED`
  - 设密码且未提供：`403` + `code=PASSWORD_REQUIRED`
  - 密码错误：`403` + `code=PASSWORD_INVALID`

---

## POST /api/meetings/{meeting_no}/guest-join

游客按会议号加入会议（无需用户 JWT）。

### 请求示例

```bash
curl -X POST http://localhost:8081/api/meetings/123456789/guest-join \
  -H "Content-Type: application/json" \
  -d '{
    "participantName": "访客A",
    "meetingPassword": "secret12"
  }'
```

### 规则

- 仅当 `allowGuestJoin=true` 才允许。
- 未到预定时间：`409` + `MEETING_NOT_STARTED`
- 到点但主持者未开启：`409` + `HOST_NOT_JOINED`
- 设密码时游客必须提供正确密码。

### 游客权限边界

- `isHost=false`
- `canPublish=false`
- `canSubscribe=true`
- `canPublishData=false`

---

## POST /api/meetings/{meeting_no}/leave

当前登录用户离会（只允许自己离会，幂等）。

说明：

- 主持者离会：立即结束会议（会议状态改为 `ended`）；
- 若离会者是最后一名成员：立即结束会议（状态改为 `ended`）；
- 其他情况（仍有人在会中）：会议保持 `open`。

---

## POST /api/meetings/{meeting_no}/cancel

主持者取消会议（仅 `scheduled` 可取消）。

### 成功响应（200）

```json
{
  "message": "Meeting cancelled"
}
```

### 常见错误

- 非主持者：`403`
- 状态非 `scheduled`：`409` + `code=MEETING_NOT_CANCELLABLE`

---

## GET /api/me/meeting-records

查询当前用户的历史参会记录（参与维度）。

---

## GET /api/me/host-meetings

查询当前用户创建的会议（主持者视角）。

### 查询参数

| 参数 | 类型 | 默认值 | 说明 |
|------|------|--------|------|
| `page` | number | `1` | 页码 |
| `pageSize` | number | `20` | 每页 `1~100` |
| `status` | string | 无 | `scheduled/open/ended/cancelled` |
| `timeFrom` | string | 无 | 按 `scheduledStartAt` 下界筛选 |
| `timeTo` | string | 无 | 按 `scheduledStartAt` 上界筛选 |
| `includeEnded` | boolean | `false` | 不传时默认只返回 `scheduled/open` |

### 成功响应（200）

```json
{
  "meetings": [
    {
      "meetingNo": "123456789",
      "roomName": "m-123456789",
      "topic": "项目周会",
      "status": "scheduled",
      "scheduledStartAt": "2026-02-20T14:30:00Z",
      "openedAt": null,
      "endedAt": null,
      "cancelledAt": null,
      "allowGuestJoin": true,
      "requiresPassword": true,
      "noJoinAutoEndMinutes": 15,
      "emptyAutoEndMinutes": 10,
      "createdAt": "2026-02-20T13:00:00Z"
    }
  ],
  "page": 1,
  "pageSize": 20
}
```

---

## 错误响应格式（含稳定 code）

```json
{
  "error": "Meeting host has not opened the meeting yet",
  "code": "HOST_NOT_JOINED"
}
```
