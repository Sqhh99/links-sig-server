## 目录结构

```
links-sig-server/
├── Cargo.toml                 # Rust 项目配置文件
├── Dockerfile                 # Docker 镜像构建配置
│
├── docker/                    # Docker 一键启动（PostgreSQL + LiveKit + 本服务）
│   ├── compose.yaml           # 编排文件
│   ├── .env.example           # 配置模板，复制为 .env 使用
│   ├── livekit.yaml           # LiveKit 配置
│   └── README.md              # 启动、日志、配置说明
│
├── docs/                      # 项目文档
│   ├── api/                   # API 接口文档
│   ├── CONTRIBUTING.md        # 贡献指南
│   └── TROUBLESHOOTING.md     # 故障排查
│
├── migrations/                # 数据库迁移脚本（SQL）
│
├── src/                       # 源代码主目录
│   ├── auth/                  # 认证相关（JWT、提取器）
│   ├── handlers/              # HTTP 请求处理器（认证、会议、健康检查）
│   ├── integrations/          # 外部服务集成（数据库、邮件、LiveKit）
│   ├── services/              # 业务逻辑服务层（认证、会议、生命周期）
│   ├── types/                 # 数据类型（错误、请求、响应）
│   ├── main.rs                # 应用程序入口
│   ├── lib.rs                 # 库定义
│   ├── config.rs              # 配置管理
│   ├── routes.rs              # 路由定义
│   └── state.rs               # 应用程序全局状态
│
├── static/                    # 前端静态资源
│   ├── css/                   # 样式表
│   ├── js/                    # JavaScript 脚本和库
│   └── join/                  # 加入会议页面
│
├── tests/                     # 集成测试
│   └── support/               # 测试支持工具（模拟数据库、LiveKit）
│
└── target/                    # 编译输出目录（Cargo 自动生成）
```

---

## 核心模块说明

### 认证模块 (`src/auth/`)
- **extractor.rs**: 自定义 Axum 提取器，用于 HTTP 请求中的身份验证
- **jwt.rs**: JWT 令牌的签发和验证逻辑
- **user_jwt.rs**: 用户特定的 JWT 处理

### 处理器模块 (`src/handlers/`)
- **auth.rs**: 用户登录（首次登录即创建账号）、令牌刷新等认证端点
- **health.rs**: 服务健康检查端点
- **meeting.rs**: 会议相关的 CRUD 操作端点

### 服务层 (`src/services/`)
实现业务逻辑和 LiveKit 集成：
- **auth_service.rs**: 认证业务逻辑
- **user_auth_service.rs**: 用户名密码登录、新账号的用户名与密码校验
- **meeting_service.rs**: 会议核心逻辑
- **meeting_registry_service.rs**: 会议注册和查询
- **meeting_lifecycle_service.rs**: 会议的启动、进行中、结束等状态管理

### 集成模块 (`src/integrations/`)
- **db.rs**: PostgreSQL 数据库操作（使用 sqlx）
- **livekit.rs**: LiveKit API 通信

### 类型模块 (`src/types/`)
- **error.rs**: 统一的错误处理和错误响应类型
- **requests.rs**: HTTP 请求体的数据结构
- **responses.rs**: HTTP 响应体的数据结构

---

## 数据库迁移

按顺序执行的 SQL 迁移文件：
1. **001_initial_schema.sql** - 基础表（用户、认证）
2. **002_meetings_and_records.sql** - 会议表和录制记录
3. **003_add_users_display_name.sql** - 用户显示名字段
4. **004_add_meetings_allow_guest_join.sql** - 访客加入权限
5. **005_add_scheduled_meeting_fields.sql** - 计划会议时间字段
6. **006_username_login.sql** - 以用户名替代邮箱作为登录标识，删除邮箱验证码表

---

## 依赖关键技术栈

| 技术 | 用途 |
|------|------|
| **Axum** | Web 框架和路由 |
| **Tokio** | 异步运行时 |
| **sqlx** | 数据库访问（PostgreSQL） |
| **jsonwebtoken** | JWT 令牌处理 |
| **livekit-api** | LiveKit SDK 集成 |
| **reqwest** | HTTP 客户端 |
| **serde** | 序列化/反序列化 |
| **chrono** | 时间处理 |
| **tracing** | 日志和追踪 |

---

## 部署相关

### Docker
- **Dockerfile** - 主应用镜像
- **docker/compose.yaml** - PostgreSQL、LiveKit 和本服务的一体化编排，另含测试库 `postgres-test`

### 文档
参考 `docker/README.md` 了解启动、日志、配置和部署。

---

## API 文档

详细的 API 文档在 `docs/api/` 目录：
- **health.md** - 健康检查接口
- **auth.md** - 用户认证接口（登录、刷新）
- **meetings.md** - 会议管理接口
- **rooms.md** - 房间管理接口
- **participants.md** - 参与者管理接口
- **token.md** - 令牌生成接口

---

## 测试

集成测试位于 `tests/` 目录，覆盖：
- 认证流程（登录、刷新）
- 健康检查
- 会议和房间操作
- 令牌生成

提供模拟 LiveKit 服务（`tests/support/fake_livekit.rs`）进行测试。

---

## 快速入门

```bash
# 构建项目
cargo build

# 运行测试
cargo test

# 启动开发服务器
cargo run

# 用 Docker 启动完整服务栈（先把 docker/.env.example 复制为 docker/.env 并填好）
cd docker && docker compose up -d --build
```

更多详情参考 `docs/CONTRIBUTING.md` 和 `docker/README.md`。
