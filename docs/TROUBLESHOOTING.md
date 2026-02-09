# 故障排查指南

## 1. 测试数据库连接超时（pool timed out while waiting for an open connection）

### 症状

运行 `cargo test` 时，集成测试报错：

```
Failed to connect to test database after retries.
TEST_DATABASE_URL=postgres://links_sig_test:links_sig_test_password@localhost:5433/links_sig_test.
Hint: start test DB with `docker compose up -d postgres-test`.
Last error: pool timed out while waiting for an open connection
```

即使 `docker compose up -d postgres-test` 已执行、容器状态为 healthy，测试仍然失败。

### 根本原因

**Windows 上 `localhost` 解析为 IPv6 地址 `::1`，而 Docker 端口映射默认监听在 IPv4 `0.0.0.0` 上，不包含 `::1`，导致 TCP 连接超时。**

验证方式：

```powershell
# IPv6 — 连接失败
Test-NetConnection -ComputerName localhost -Port 5433

# IPv4 — 连接成功
Test-NetConnection -ComputerName 127.0.0.1 -Port 5433
```

### 解决方案

在数据库连接地址中使用 `127.0.0.1` 代替 `localhost`。

涉及的文件：

- `tests/support/db.rs` — 测试数据库默认连接地址
- `.env` / `.env.example` — `DATABASE_URL` 配置项

```diff
- postgres://links_sig_test:links_sig_test_password@localhost:5433/links_sig_test
+ postgres://links_sig_test:links_sig_test_password@127.0.0.1:5433/links_sig_test
```

### 预防措施

1. **所有数据库连接地址统一使用 `127.0.0.1`**，不要使用 `localhost`。
2. 如果必须使用 `localhost`，可在 Docker Compose 中为 IPv6 显式绑定端口：
   ```yaml
   ports:
     - "127.0.0.1:5433:5432"
     - "[::1]:5433:5432"
   ```

---

## 2. 测试数据库缺少新迁移的表

### 症状

测试报出表不存在的错误（如 `relation "meetings" does not exist`），或者连接成功后测试仍然失败。

### 根本原因

新增了迁移文件（如 `002_meetings_and_records.sql`），但测试数据库没有运行过该迁移。

### 解决方案

测试代码中已调用 `run_test_migrations()`，正常情况下连接成功后会自动执行迁移。

如果需要手动补跑：

```powershell
$env:DATABASE_URL="postgres://links_sig_test:links_sig_test_password@127.0.0.1:5433/links_sig_test"
cargo sqlx migrate run
```

验证迁移状态：

```powershell
docker exec links-sig-postgres-test psql -U links_sig_test -d links_sig_test -c "SELECT version, description FROM _sqlx_migrations ORDER BY version;"
```

### 预防措施

1. 添加新迁移文件后，确认 CI 和本地测试数据库都能正确执行。
2. 如果测试数据库状态异常，可以重建：
   ```powershell
   docker compose down -v postgres-test
   docker compose up -d postgres-test
   ```

---

## 常用命令速查

| 操作 | 命令 |
|------|------|
| 启动测试数据库 | `docker compose up -d postgres-test` |
| 停止并清除测试数据 | `docker compose down -v postgres-test` |
| 检查容器状态 | `docker ps -a --filter "name=links-sig-postgres-test"` |
| 检查端口连通性 | `Test-NetConnection -ComputerName 127.0.0.1 -Port 5433` |
| 手动运行迁移 | `$env:DATABASE_URL="postgres://...@127.0.0.1:5433/..."; cargo sqlx migrate run` |
| 运行全部测试 | `cargo test` |
| 运行指定测试文件 | `cargo test --test meetings` |
