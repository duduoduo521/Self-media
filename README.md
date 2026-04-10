# Self-media - AI 多平台内容发布系统

> 一键生成 + 多平台自动发布自媒体解决方案

## 项目概述

Self-media 是一款基于 AI 的自媒体内容生成与多平台发布系统，支持：
- 🎯 **热点追踪**：实时聚合微博、B站、抖音、知乎、头条、小红书热点
- ✍️ **AI 生成**：使用 MiniMax API 自动生成文案、配图、视频
- 📤 **多平台发布**：一次生成，多平台同步发布
- 🔐 **本地加密**：用户数据本地加密存储，无需担心泄露

## 技术栈

- **后端**：Rust (Axum + SQLx + Tokio)
- **前端**：Vue 3 + TypeScript + Vite
- **数据库**：SQLite (开发) / MySQL (生产)
- **AI**：MiniMax API

## 项目结构

```
Self-media/
├── Cargo.toml           # Rust workspace 配置
├── crates/
│   ├── self-media-core/   # 核心业务逻辑
│   ├── self-media-crypto/ # 加密与密钥管理
│   ├── self-media-db/     # 数据库操作
│   ├── self-media-ai/     # AI 集成
│   ├── self-media-publish/# 平台发布适配器
│   └── self-media-web/    # Web 服务
├── frontend/            # Vue 3 前端
├── migrations/          # 数据库迁移
└── docs/               # 设计文档
```

## 快速开始

### 环境要求

- Rust 1.75+
- Node.js 18+
- SQLite3

### 安装

```bash
# 克隆项目
git clone https://github.com/duduoduo521/Self-media.git
cd Self-media

# 安装前端依赖
cd frontend && npm install

# 配置环境变量
cp .env.example .env
# 编辑 .env 填入必要配置

# 启动开发服务器
cargo run --bin self-media-web
```

### 环境变量配置

```env
# 数据库
SELF_MEDIA_DB_PATH=./data/self-media.db

# Web 服务
SELF_MEDIA_WEB_PORT=3000

# 系统密钥（用于加密用户数据）
SELF_MEDIA_MACHINE_KEY=your-machine-key

# MiniMax API
MINIMAX_API_KEY=your-api-key
MINIMAX_GROUP_ID=your-group-id

# 文件存储
STORAGE_TYPE=local
STORAGE_LOCAL_PATH=./data/uploads
```

## 功能特性

### 1. 用户管理
- 本地注册/登录
- JWT 会话认证
- 用户密钥加密存储

### 2. 热点追踪
| 平台 | 状态 |
|------|------|
| 微博 | ✅ |
| B站 | ✅ |
| 抖音 | ✅ |
| 知乎 | ✅ |
| 头条 | ✅ |
| 小红书 | ✅ |

### 3. AI 内容生成
- 文本生成（基于热点话题）
- 图片生成（配套配图）
- 视频生成（带语音解说）
- 语音合成

### 4. 多平台发布

#### 图文发布
| 平台 | 状态 | 备注 |
|------|------|------|
| 微博 | ✅ | |
| B站 | ✅ | |
| 头条 | ✅ | |
| 小红书 | ✅ | |
| 抖音 | ⚠️ | 图文需验证 |
| 公众号 | ✅ | |

#### 视频发布
| 平台 | 状态 | 备注 |
|------|------|------|
| 微博 | ⚠️ | 需完善 |
| B站 | ⚠️ | 分片上传待实现 |
| 头条 | ⚠️ | 需完善 |
| 小红书 | ⚠️ | 需完善 |
| 抖音 | ⚠️ | X-Bogus 签名缺失 |

### 5. 扫码登录
支持以下平台扫码登录：
- ✅ 微博
- ✅ B站
- ✅ 头条
- ✅ 抖音
- ✅ 小红书
- ✅ 微信公众号

## API 文档

### 认证接口
- `POST /api/auth/register` - 用户注册
- `POST /api/auth/login` - 用户登录
- `POST /api/auth/logout` - 用户登出

### 热点接口
- `GET /api/hotspot` - 获取所有热点
- `GET /api/hotspot?source=weibo` - 获取指定平台热点

### 任务接口
- `GET /api/tasks` - 获取任务列表
- `POST /api/tasks` - 创建任务
- `POST /api/tasks/:id/execute` - 执行任务
- `DELETE /api/tasks/:id` - 取消任务

### 扫码登录
- `POST /api/qr/generate` - 生成二维码
- `GET /api/qr/status/:platform/:qr_id` - 查询扫码状态
- `GET /api/qr/wait/:platform/:qr_id` - 等待扫码完成（SSE）

### 平台配置
- `GET /api/config/platforms` - 获取平台列表
- `PUT /api/config/platforms/:platform` - 更新平台配置
- `GET /api/config/keys` - 获取 API Key 列表
- `PUT /api/config/keys` - 设置 API Key

## 安全特性

- ✅ HttpOnly Cookie 存储 Token
- ✅ CSRF 防护（Double Submit Cookie）
- ✅ 用户数据 AES-256-GCM 加密
- ✅ JWT 会话认证
- ✅ 任务权限校验

## 部署

### 生产构建

```bash
# 构建后端
cargo build --release

# 构建前端
cd frontend && npm run build
```

### systemd 服务

```ini
[Unit]
Description=Self-media Service
After=network.target

[Service]
Type=simple
User=your-user
WorkingDirectory=/path/to/self-media
ExecStart=/path/to/self-media/target/release/self-media-web
Environment=...
Restart=on-failure

[Install]
WantedBy=multi-user.target
```

## 开发指南

### 添加新平台适配器

1. 在 `crates/self-media-publish/src/adapters/` 创建新文件
2. 实现 `PlatformPublisher` trait
3. 在 `mod.rs` 注册适配器
4. 在扫码登录模块添加对应处理器

### 添加新热点源

在 `crates/self-media-core/src/hotspot/service.rs` 添加 `fetch_{source}()` 方法

## 贡献

欢迎提交 Issue 和 Pull Request！

## 许可证

MIT License
