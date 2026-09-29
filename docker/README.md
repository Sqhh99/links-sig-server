# 用 Docker 启动 Links 后端

`compose.yaml` 用一条命令启动后端需要的全部服务：

| 服务 | 作用 | 宿主机端口 |
| --- | --- | --- |
| `postgres` | 主数据库，数据保存在 Docker 数据卷里 | `127.0.0.1:5432`（只对本机开放） |
| `livekit` | LiveKit 媒体服务器 | `7880/tcp` 信令，`7881/tcp` 和 `7882/udp` 媒体 |
| `server` | links-sig-server，从上一级目录的 `Dockerfile` 构建 | `8081/tcp` |
| `postgres-test` | 给 `cargo test` 用的测试库，默认不启动 | `127.0.0.1:5433` |

下面的命令都要在 **`docker/` 目录** 里执行。

需要 Docker Desktop，或者装了 Compose v2 插件的 Docker Engine（命令是 `docker compose`，不是 `docker-compose`）。

## 第一次启动

```bash
cd docker
cp .env.example .env
```

打开 `.env`，填上两个密钥：

- `JWT_SECRET`
- `LIVEKIT_API_SECRET`，至少 32 个字符

生成随机值：

```bash
openssl rand -hex 32
```

PowerShell 7 里可以用：

```powershell
[Convert]::ToHexString([Security.Cryptography.RandomNumberGenerator]::GetBytes(32)).ToLower()
```

然后构建并启动：

```bash
docker compose up -d --build
```

第一次构建要下载 Rust 工具链并编译全部依赖，需要几分钟。之后只改代码的话，依赖不会重新编译。

检查是否起来了：

```bash
docker compose ps                       # 三个服务都是 Up，server 和 postgres 显示 (healthy)
curl http://127.0.0.1:8081/api/health   # {"status":"ok",...}
```

最后在客户端里把服务器地址设成 `http://127.0.0.1:8081`。

## 常用命令

| 要做的事 | 命令 |
| --- | --- |
| 启动（或按 `.env` 的改动重建容器） | `docker compose up -d` |
| 代码更新后重新构建并启动 | `docker compose up -d --build` |
| 查看状态 | `docker compose ps` |
| 实时看全部日志 | `docker compose logs -f` |
| 只看某个服务 | `docker compose logs -f server`（或 `livekit`、`postgres`） |
| 最近 200 行 / 最近 10 分钟 | `docker compose logs --tail 200 server` / `docker compose logs --since 10m server` |
| 重启某个服务 | `docker compose restart server` |
| 停止，保留容器 | `docker compose stop` |
| 停止并删除容器，数据保留 | `docker compose down`（不包括测试库；连测试库一起停用 `docker compose --profile test down`） |
| **连同数据库一起删除** | `docker compose down -v`（账号、会议记录全部丢失） |
| 进入数据库 | `docker compose exec postgres psql -U links_sig -d links_sig` |
| 备份数据库 | `docker compose exec -T postgres pg_dump -U links_sig links_sig > backup.sql` |

每个容器的日志最多保留 3 个 10 MB 的文件，不会无限增长。

所有服务都设了 `restart: unless-stopped`：Docker 启动时会自动拉起上次没被手动停止的服务。

## 配置

`.env.example` 里每一项都有注释，这里只说最容易出错的几项。改完 `.env` 后执行 `docker compose up -d`，Compose 只会重建受影响的容器。

### 客户端从哪里访问：`LIVEKIT_WS_URL` 和 `LIVEKIT_NODE_IP`

入会时，服务端会把 `LIVEKIT_WS_URL` 发给客户端，客户端用它连接 LiveKit。

连上以后，LiveKit 会告诉客户端把音视频发到 `LIVEKIT_NODE_IP`。容器自己的 IP 客户端访问不到，所以这一项必须明确填写。

这两项必须指向同一台机器：

| 场景 | `LIVEKIT_WS_URL` | `LIVEKIT_NODE_IP` |
| --- | --- | --- |
| 客户端和 Docker 在同一台电脑 | `ws://127.0.0.1:7880` | `127.0.0.1` |
| 局域网里的其他电脑也要用 | `ws://192.168.1.10:7880` | `192.168.1.10`（本机局域网 IP） |
| 公网服务器 | `ws://<公网 IP 或域名>:7880` | 服务器公网 IP |

局域网和公网场景下，还要：

- 在防火墙放行 `8081/tcp`、`7880/tcp`、`7881/tcp`、`7882/udp`；
- 把 `APP_BASE_URL` 改成对应地址，会议分享链接用的就是它。

UDP 被挡住时，LiveKit 会退回到 `7881/tcp` 传媒体，但延迟会高一些。

### 公网部署时的 HTTPS

server 和 LiveKit 本身只提供明文的 HTTP / WS。需要 HTTPS 时，在前面加一层反向代理（Nginx、Caddy 等）：

- `https://<域名>` 转发到 `8081`；
- `wss://<LiveKit 域名>` 转发到 `7880`，要支持 WebSocket；
- `.env` 里对应改成 `LIVEKIT_WS_URL=wss://...`、`APP_BASE_URL=https://...`。

媒体端口 `7881/tcp` 和 `7882/udp` 不经过代理，仍然直接对外开放。

### 密钥

- `LIVEKIT_API_KEY` / `LIVEKIT_API_SECRET` 同时给 LiveKit 和 server 使用，不用改两处。
- 改了 `JWT_SECRET` 以后，已登录的用户需要重新登录。
- 旧的 `docker/livekit-docker/livekit.yaml` 里的 LiveKit secret 已经在 Git 历史里公开，不要再用。

### 数据库

- `POSTGRES_USER` / `POSTGRES_PASSWORD` / `POSTGRES_DB` 只在第一次创建数据卷时生效，之后改 `.env` 不会改变已有数据库。
  - 如果改完后 server 连不上数据库，把值改回去，或者进 `psql` 执行 `ALTER USER ... PASSWORD ...`。
- 密码只用字母和数字，因为它会被直接拼进连接串。
- 服务端每次启动都会自动执行 `migrations/` 里的迁移，不需要手动操作。

### LiveKit 的高级配置

LiveKit 的其他配置写在 `livekit.yaml` 里，可选项参考 [LiveKit 的配置示例](https://github.com/livekit/livekit/blob/master/config-sample.yaml)。

- 端口要和 `compose.yaml` 里 `livekit.ports` 一起改：LiveKit 把端口号直接发给客户端，所以宿主机和容器的端口必须一致。
- LiveKit 版本固定在 `compose.yaml` 的 `image:` 一行。升级时改这一行，再执行 `docker compose up -d`。

## 只用 Docker 跑依赖，本机 `cargo run`

开发服务端时，可以只在 Docker 里跑数据库和 LiveKit：

```bash
docker compose up -d postgres livekit
# 如果 server 容器也在跑，先停掉它，免得占用 8081：docker compose stop server
```

然后在仓库根目录执行 `cargo run`。根目录的 `.env` 要满足下面三条：

- `DATABASE_URL=postgres://links_sig:<POSTGRES_PASSWORD>@127.0.0.1:5432/links_sig`
- `LIVEKIT_URL=http://127.0.0.1:7880`
- `LIVEKIT_API_KEY`、`LIVEKIT_API_SECRET` 和 `docker/.env` 里的**完全一致**

## 测试数据库

```bash
docker compose up -d postgres-test   # 只在点名时启动
cd .. && cargo test
```

- 测试库的数据放在内存里（tmpfs），容器重启后是一个空库；测试会自己执行迁移。
- 测试库状态乱了的话，`docker compose restart postgres-test` 就能得到一个干净的库。

## 从旧的 Docker 配置迁移

原来的 `docker/postgres-docker/`、`docker/livekit-docker/` 和 `docker/links-sig-server/` 已经合并到这里，并删除。

- **还在运行的旧容器**：`links-sig-postgres`、`links-sig-postgres-test`、`livekit-server` 会占用同样的端口，先删除它们（数据卷不受影响）：

  ```bash
  docker rm -f links-sig-postgres links-sig-postgres-test livekit-server
  ```

- **沿用旧数据库**：旧的主库在数据卷 `postgres-docker_postgres_data` 里，用户名、密码、库名和这里的默认值一样。在 `.env` 里设置下面这一行，就会直接使用它：

  ```
  POSTGRES_VOLUME=postgres-docker_postgres_data
  ```

  - 服务端启动时会对旧库执行尚未应用的迁移。比如 006 会把 `email` 列改名为 `username`。
- **旧的部署配置**：`.env.deploy` 里的值对应到 `.env` 的同名项。
  - `DATABASE_URL` 和 `LIVEKIT_URL` 不需要再填，Compose 会自动指向容器内的服务。
  - `HOST_PORT` 改名为 `SERVER_PORT`。

## 常见问题

### `docker compose up` 报 `set JWT_SECRET in docker/.env`

`.env` 不存在，或者必填项是空的。按「第一次启动」一节填好。

### 报端口被占用（`port is already allocated`）

通常是旧容器，或者本机装的 PostgreSQL 占着 5432。

- 用 `docker ps` 找到占用端口的容器并停掉；
- 或者在 `.env` 里改 `SERVER_PORT`、`POSTGRES_PORT`。
- LiveKit 的端口要和 `livekit.yaml` 一起改，见上文「LiveKit 的高级配置」。

### 客户端能登录，但进会议后没有画面，或者一直在连接

几乎都是 `LIVEKIT_NODE_IP` 或 `LIVEKIT_WS_URL` 填错了，或者 `7882/udp`、`7881/tcp` 被防火墙挡住了。

- 查看 LiveKit 实际通告的地址：`docker compose logs livekit | grep -i "node ip"`

### server 反复重启

先看日志：`docker compose logs --tail 100 server`。

- 最常见的原因是数据库密码和数据卷里的不一致，见上文「数据库」。

### 本机 `curl` 访问 127.0.0.1 没有返回

系统开了 HTTP 代理时，curl 会把本机请求也转给代理。加上 `--noproxy '*'` 再试。
