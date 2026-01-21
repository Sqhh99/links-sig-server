# 代码编写与组织指南

本文档规范了 `links-sig-rust-server` 项目的代码风格和组织结构，确保团队成员编写一致、可维护的代码。

---

## 目录

- [项目结构](#项目结构)
- [模块组织规范](#模块组织规范)
- [代码风格](#代码风格)
- [命名规范](#命名规范)
- [错误处理](#错误处理)
- [测试规范](#测试规范)
- [文档注释](#文档注释)
- [Git 提交规范](#git-提交规范)

---

## 项目结构

```
src/
├── main.rs          # 程序入口，服务器启动
├── lib.rs           # 库入口，暴露公共模块供测试使用
├── config.rs        # 配置加载（环境变量）
├── state.rs         # 应用状态（AppState）
├── routes.rs        # 路由定义（集中管理）
├── auth/            # 认证模块
│   ├── mod.rs       # 模块导出
│   ├── jwt.rs       # JWT 编解码
│   └── extractor.rs # Axum 提取器
├── handlers/        # HTTP 处理器（薄层）
│   ├── mod.rs
│   ├── health.rs
│   ├── auth.rs
│   └── meeting.rs
├── services/        # 业务逻辑层
│   ├── mod.rs
│   ├── auth_service.rs
│   └── meeting_service.rs
├── integrations/    # 外部服务适配器
│   ├── mod.rs
│   └── livekit.rs
└── types/           # 类型定义
    ├── mod.rs
    ├── requests.rs  # 请求 DTO
    ├── responses.rs # 响应 DTO
    └── error.rs     # 统一错误类型

tests/               # 集成测试
├── support/         # 测试夹具
│   ├── mod.rs
│   ├── app.rs       # 测试 App 构建器
│   └── fake_livekit.rs
├── health.rs
├── rooms.rs
└── token.rs
```

---

## 模块组织规范

### 分层架构

```
HTTP Request
     │
     ▼
┌─────────────┐
│  Handlers   │  ← 薄层：解析请求，调用 Service，返回响应
└─────────────┘
     │
     ▼
┌─────────────┐
│  Services   │  ← 业务逻辑：验证、转换、编排
└─────────────┘
     │
     ▼
┌─────────────┐
│ Integrations│  ← 外部服务：LiveKit API、数据库等
└─────────────┘
```

### Handler 规范

Handler 应保持**薄层**，只做：
1. 提取请求参数（State、Path、Json）
2. 调用 Service 方法
3. 返回 HTTP 响应

```rust
// ✅ 正确：Handler 薄层
pub async fn handle_create_room(
    State(state): State<AppState>,
    Json(req): Json<CreateRoomRequest>,
) -> Result<impl IntoResponse, AppError> {
    let room = MeetingService::create_room(&*state.livekit, req).await?;
    Ok((StatusCode::CREATED, Json(room)))
}

// ❌ 错误：Handler 包含业务逻辑
pub async fn handle_create_room(
    State(state): State<AppState>,
    Json(req): Json<CreateRoomRequest>,
) -> Result<impl IntoResponse, AppError> {
    // 不要在 Handler 中写业务逻辑！
    if req.name.is_empty() {
        req.name = format!("room-{}", Utc::now().timestamp());
    }
    // ...
}
```

### Service 规范

Service 包含所有**业务逻辑**：
- 使用 `struct ServiceName;` 作为命名空间
- 方法接收 trait object 以支持测试 mock
- 返回领域类型而非 HTTP 类型

```rust
pub struct MeetingService;

impl MeetingService {
    pub async fn create_room<L: LiveKitService + ?Sized>(
        livekit: &L,
        req: CreateRoomRequest,
    ) -> Result<Room, AppError> {
        // 业务逻辑在这里
    }
}
```

### Integration 规范

外部服务适配器使用 **Trait + 实现** 模式：

```rust
// 1. 定义 Trait
#[async_trait]
pub trait LiveKitService: Send + Sync {
    async fn list_rooms(&self) -> Result<Vec<LiveKitRoom>, String>;
    // ...
}

// 2. 实现真实客户端
pub struct LiveKitClient { /* ... */ }

#[async_trait]
impl LiveKitService for LiveKitClient {
    // ...
}

// 3. 测试时提供 Fake 实现
pub struct FakeLiveKitService { /* ... */ }
```

---

## 代码风格

### 格式化

使用 `rustfmt` 默认配置：

```bash
cargo fmt
```

### Lint 检查

使用 `clippy` 并修复所有警告：

```bash
cargo clippy -- -D warnings
```

### 导入顺序

按以下顺序组织 `use` 语句，组之间空一行：

```rust
// 1. 标准库
use std::sync::Arc;

// 2. 第三方 crate
use axum::{extract::State, Json};
use serde::{Deserialize, Serialize};

// 3. 本 crate 模块
use crate::config::Config;
use crate::types::AppError;
```

### 行宽

保持每行不超过 **100 字符**。

---

## 命名规范

| 类型 | 规范 | 示例 |
|------|------|------|
| 模块/文件 | snake_case | `auth_service.rs` |
| 结构体/枚举 | PascalCase | `TokenRequest`, `AppError` |
| 函数/方法 | snake_case | `generate_token()` |
| 常量 | SCREAMING_SNAKE_CASE | `DEFAULT_TIMEOUT` |
| Handler 函数 | `handle_` 前缀 | `handle_get_token` |
| Service 结构体 | `Service` 后缀 | `AuthService` |
| Trait | 描述性名词 | `LiveKitService` |
| 请求类型 | `Request` 后缀 | `TokenRequest` |
| 响应类型 | `Response` 后缀 | `TokenResponse` |

### JSON 字段命名

使用 **camelCase**（通过 serde 属性）：

```rust
#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TokenRequest {
    pub room_name: String,      // JSON: "roomName"
    pub participant_name: String, // JSON: "participantName"
}
```

---

## 错误处理

### 统一错误类型

所有错误通过 `AppError` 枚举处理：

```rust
pub enum AppError {
    Internal(String),     // 500
    BadRequest(String),   // 400
    NotFound(String),     // 404
    Unauthorized(String), // 401
}
```

### 错误转换

使用 `?` 操作符配合 `map_err`：

```rust
// ✅ 正确
let rooms = livekit
    .list_rooms()
    .await
    .map_err(AppError::internal)?;

// ❌ 避免：手动 match
let rooms = match livekit.list_rooms().await {
    Ok(r) => r,
    Err(e) => return Err(AppError::internal(e)),
};
```

### 日志记录

- `error!` - 需要关注的错误
- `warn!` - 可恢复的异常
- `info!` - 重要业务事件
- `debug!` - 调试信息（生产环境不输出）

```rust
use tracing::{error, info, warn};

info!("Token generated for user '{}'", participant_name);
error!("Failed to create room: {}", e);
```

---

## 测试规范

### 测试分层

| 层级 | 位置 | 用途 |
|------|------|------|
| 单元测试 | `src/模块.rs` 内 `#[cfg(test)]` | 测试纯函数、业务规则 |
| 集成测试 | `tests/*.rs` | 测试 HTTP 路由行为 |

### 单元测试

```rust
// src/auth/jwt.rs
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_token_encode_decode() {
        // 测试纯函数
    }
}
```

### 集成测试

使用 `tower::ServiceExt::oneshot` 发送请求：

```rust
// tests/health.rs
#[tokio::test]
async fn test_health_check_returns_200() {
    let app = build_test_app();

    let response = app
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/health")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
}
```

### 测试命名

```rust
// 格式: test_<被测对象>_<场景>_<预期结果>
#[test]
fn test_token_with_invalid_secret_returns_error() { }

#[tokio::test]
async fn test_create_room_returns_201() { }
```

### Mock 服务

使用 `tests/support/fake_livekit.rs` 中的 `FakeLiveKitService`：

```rust
let fake_livekit = FakeLiveKitService::new()
    .with_room(LiveKitRoom { /* ... */ });

let state = build_test_state_with_livekit(fake_livekit);
let app = build_test_app_with_state(state);
```

---

## 文档注释

### 模块文档

每个模块文件顶部添加 `//!` 文档：

```rust
//! Meeting service - Room and participant management
//!
//! Handles all meeting-related business logic including room CRUD
//! and participant management.
```

### 函数文档

公共函数添加 `///` 文档：

```rust
/// Generate a LiveKit access token for a user
///
/// This method:
/// 1. Validates and normalizes the request
/// 2. Determines if user should be host
/// 3. Creates appropriate video grants
/// 4. Generates and returns the JWT token
pub async fn generate_token(...) -> Result<TokenResponse, AppError> {
```

### Handler 文档

包含路由信息和请求/响应示例：

```rust
/// Create a new room
///
/// POST /api/rooms
///
/// Request body:
/// ```json
/// {
///   "name": "my-room"
/// }
/// ```
///
/// Response: Room object (201 Created)
pub async fn handle_create_room(...) { }
```

---

## Git 提交规范

### 提交消息格式

```
<type>(<scope>): <subject>

<body>

<footer>
```

### Type 类型

| Type | 描述 |
|------|------|
| `feat` | 新功能 |
| `fix` | Bug 修复 |
| `docs` | 文档更新 |
| `style` | 代码格式（不影响逻辑） |
| `refactor` | 重构（无新功能或修复） |
| `test` | 测试相关 |
| `chore` | 构建/工具变更 |

### 示例

```
feat(auth): add JWT token refresh endpoint

- Add POST /api/token/refresh endpoint
- Implement token validation and renewal logic
- Add integration tests for token refresh

Closes #123
```

---

## 添加新 API 检查清单

添加新 API 时，确保完成以下步骤：

- [ ] 在 `types/requests.rs` 添加请求 DTO
- [ ] 在 `types/responses.rs` 添加响应 DTO
- [ ] 在 `services/` 添加业务逻辑
- [ ] 在 `handlers/` 添加薄层 Handler
- [ ] 在 `routes.rs` 注册路由
- [ ] 在 `handlers/mod.rs` 导出 Handler
- [ ] 在 `tests/` 添加集成测试
- [ ] 更新 `docs/API.md` 文档
- [ ] 运行 `cargo test` 确保通过
- [ ] 运行 `cargo clippy` 无警告
