# 健康检查 API

## GET /api/health

检查服务器健康状态。

### 请求

```bash
curl -X GET http://localhost:8081/api/health
```

### 响应

#### 成功 (200 OK)

```json
{
  "status": "ok",
  "time": "2026-01-21T10:30:00.000Z"
}
```

### 字段说明

| 字段 | 类型 | 描述 |
|------|------|------|
| `status` | string | 服务状态，始终为 `"ok"` |
| `time` | string | 当前服务器时间（ISO 8601 格式） |

### 使用场景

- 负载均衡器健康检查
- 监控系统探活
- 服务启动就绪检测

### 示例

```bash
# 检查服务是否正常
curl -s http://localhost:8081/api/health | jq .

# 输出
{
  "status": "ok",
  "time": "2026-01-21T10:30:00.000Z"
}
```
