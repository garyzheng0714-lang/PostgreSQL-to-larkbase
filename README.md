# FBIF DataBridge · 数据桥

![可见性](https://img.shields.io/badge/%E5%8F%AF%E8%A7%81%E6%80%A7-%E5%85%AC%E5%BC%80%E4%BB%93%E5%BA%93-0A66C2?style=flat-square)
![数据源](https://img.shields.io/badge/%E6%95%B0%E6%8D%AE%E6%BA%90-PostgreSQL-4169E1?style=flat-square&logo=postgresql&logoColor=white)
![生产后端](https://img.shields.io/badge/%E7%94%9F%E4%BA%A7%E8%B7%AF%E5%BE%84-Python_FastAPI-009688?style=flat-square&logo=fastapi&logoColor=white)
![灰度后端](https://img.shields.io/badge/%E7%81%B0%E5%BA%A6%E8%B7%AF%E5%BE%84-Rust-DEA584?style=flat-square&logo=rust&logoColor=111827)
![前端](https://img.shields.io/badge/React-18-61DAFB?style=flat-square&logo=react&logoColor=111827)

把 PostgreSQL 表、视图或只读 SQL 查询接入飞书多维表格的自定义数据连接器。后端实现飞书 Connector HTTP 协议，前端提供嵌入式连接配置页面。

> 当前 `main` 自动部署工作流仍发布 Python/FastAPI 后端。`backend-rs/` 是已实现的 Rust 替代后端，但只通过手动工作流部署到独立灰度端口；未经对比验收和流量切换，不能描述为当前生产实现。

## 导航

- [产品边界](#产品边界)
- [能力与限制](#能力与限制)
- [架构与协议](#架构与协议)
- [实现状态](#实现状态)
- [快速开始](#快速开始)
- [配置](#配置)
- [测试与部署](#测试与部署)
- [安全边界](#安全边界)
- [文档索引](#文档索引)

## 产品边界

| 项目 | 说明 |
| --- | --- |
| 使用者 | 需要把 PostgreSQL 数据同步到飞书多维表格的团队 |
| 数据方向 | PostgreSQL → 飞书多维表格，拉取式同步 |
| 首发数据源 | PostgreSQL / RDS PostgreSQL |
| 配置方式 | 飞书 iframe 内选择数据库、Schema、表/视图/字段或填写自定义 SQL |
| 调度归属 | 同步频率和任务调度由飞书侧负责；本服务只响应协议请求 |
| 非目标 | 双向同步、CDC、数据库管理工具、通用 ETL 编排平台 |

## 能力与限制

- 列出数据库、Schema、表、视图和字段，并测试连接。
- 支持整表/视图同步与自定义 SQL 结果同步。
- 支持字段筛选、重命名、主键识别、类型映射和分页 token。
- `/meta.json` 动态追加时间戳，避免飞书缓存旧配置页 URL。
- `/api/table_meta` 返回字段结构，`/api/records` 分页返回记录。
- Python 后端提供连接池缓存、超时、统一协议错误与 pytest 覆盖。
- Rust 后端额外提供 `/ready`、`/metrics`、严格验签路径、只读事务和更细的资源边界。

已实现限制：

| 限制 | 当前值/行为 |
| --- | --- |
| 单次同步总行数 | 默认最多 `50000`，由 `MAX_ROW_LIMIT` 调整 |
| 单页记录数 | 由飞书请求约束，后端限制在安全范围内 |
| 字段数 | 最多 `299` |
| 未识别类型 | 退化为文本 |
| 数组、JSON、time 等 | 文本或序列化字符串 |
| 写入数据库 | 不需要；应使用数据库只读账号做最终权限护栏 |

## 架构与协议

```text
飞书多维表格
   ├─ GET  /meta.json
   ├─ POST /api/table_meta  ─┐
   └─ POST /api/records     ─┼─ SHA-1 协议签名
                              ▼
                      Connector backend
                       ├─ Python/FastAPI（main 自动部署）
                       └─ Rust/Axum（手动灰度）
                              │
                              ▼
                         PostgreSQL

配置 iframe ─► React/Vite ─► /api/helper/* ─► 数据库元数据与预览
```

飞书协议签名算法由平台规定：

```text
SHA1(timestamp + nonce + secretKey + rawBody).hex()
```

请求头：`X-Base-Request-Timestamp`、`X-Base-Request-Nonce`、`X-Base-Signature`。验签必须使用原始 body，不能先反序列化再重新 `JSON.stringify`。

### 核心端点

| 方法 | 路径 | 用途 |
| --- | --- | --- |
| `GET` | `/health` | 存活检查 |
| `GET` | `/meta.json` | Connector 元数据与动态配置页 URL |
| `POST` | `/api/table_meta` | 返回表结构与唯一主字段 |
| `POST` | `/api/records` | 分页返回记录 |
| `POST` | `/api/helper/test_connection` | 配置页测试 PostgreSQL 连接 |
| `POST` | `/api/helper/databases`、`schemas`、`tables`、`columns` | 配置页元数据发现 |
| `POST` | `/api/helper/preview_sql` | 预览自定义 SQL |
| `GET` | `/ready`、`/metrics` | 仅 Rust 后端提供 |

## 实现状态

| 维度 | Python `backend/` | Rust `backend-rs/` |
| --- | --- | --- |
| 框架 | FastAPI + asyncpg | Axum + tokio-postgres + deadpool |
| 自动部署 | `main` push，经 `deploy.yml` | 否；`deploy-rust.yml` 仅手动灰度 |
| Docker Compose 默认服务 | 是 | 否，单独构建/运行 |
| Connector 端点 | 是 | 是 |
| helper 鉴权 | 生产 `HELPER_API_KEY` 为空时仍会放行 | 非开发模式下 fail-closed |
| 无签名兼容 | 带 timestamp + nonce、缺 signature 时会放行 | 默认严格；`ALLOW_UNSIGNED=true` 才兼容 |
| 指标/就绪检查 | 无独立端点 | `/metrics`、`/ready` |
| 回滚角色 | 当前基线 | 灰度候选 |

这张表描述代码和工作流现状，不代表任一环境已完成灰度切换。

## 快速开始

### Python 后端

要求：Python 3.11+、[`uv`](https://docs.astral.sh/uv/)、PostgreSQL 可达。

```bash
cd backend
cp .env.example .env
uv sync --extra dev
uv run uvicorn src.main:app --reload --host 0.0.0.0 --port 8000
```

### 前端配置页

```bash
cd frontend
npm ci
VITE_API_BASE_URL=http://localhost:8000 npm run dev
```

同域部署构建：

```bash
VITE_API_BASE_URL='' npm run build
```

### Docker Compose

```bash
cp backend/.env.example backend/.env.production
docker compose up -d --build
```

当前 Compose 只构建 Python 后端，并依赖已经存在的外部网络 `postgres_default`。如果目标 PostgreSQL 不在该网络，先调整网络配置，不能直接把生产数据库暴露到公网。

### Rust 后端

```bash
cd backend-rs
cargo test
cargo build --release
BIND_ADDR=127.0.0.1:18082 \
SECRET_KEY='replace-with-a-strong-secret' \
HELPER_API_KEY='replace-with-an-independent-key' \
./target/release/fbif-databridge
```

Rust 灰度、对比与回滚步骤见 [`backend-rs/README.md`](backend-rs/README.md)。

## 配置

### 后端通用

| 变量 | 说明 |
| --- | --- |
| `SECRET_KEY` | 飞书 Connector 请求签名密钥；默认占位值只用于本地开发 |
| `FRONTEND_URL` | 写入 `meta.json` 的配置页地址 |
| `MAX_ROW_LIMIT` | 单次同步总行数上限 |
| `PG_CONNECT_TIMEOUT` / `PG_QUERY_TIMEOUT` | PostgreSQL 连接与查询超时 |
| `POOL_MAX_SIZE` / `POOL_IDLE_TIMEOUT` / `POOL_MAX_POOLS` | 连接池上限与回收策略 |
| `HELPER_API_KEY` | 配置页 helper API key |

Python 还使用 `BACKEND_URL`、`CORS_ORIGINS`；Rust 使用 `BIND_ADDR`、`RUST_LOG` 和兼容开关 `ALLOW_UNSIGNED`。

### 前端构建

| 变量 | 说明 |
| --- | --- |
| `VITE_API_BASE_URL` | 后端基地址；同域部署使用空字符串 |
| `VITE_HELPER_API_KEY` | 发送给 helper API 的 key，必须与后端一致 |
| `VITE_DEFAULT_PORT` / `VITE_DEFAULT_SCHEMA` | 配置页默认 PostgreSQL 端口与 Schema |
| `VITE_MOCK` | 仅开发模式的 UI mock；生产构建不会启用 |

不要把真实数据库密码或 helper key 固化进公开构建。`VITE_*` 会进入浏览器 bundle，因此 helper key 只能被视为低强度调用门槛，真正安全边界必须是网络策略、限流和数据库只读账号。

## 测试与部署

### 本地验证

```bash
cd backend
uv run pytest
uv run ruff check src tests
uv run mypy src

cd ../frontend
npm ci
npm test
npm run build

cd ../backend-rs
cargo test
```

### 工作流事实

- `.github/workflows/deploy.yml`：`main` push 自动构建前端、rsync 前后端并重建 Python 容器；当前不执行 Python pytest/Ruff/mypy。
- `.github/workflows/deploy-rust.yml`：仅 `workflow_dispatch`，构建 Rust 镜像并部署到独立灰度端口，不修改反向代理。
- Rust 灰度工作流当前显式设置 `ALLOW_UNSIGNED=true` 以兼容旧环境。若要晋升为正式路径，必须先配置飞书签名并关闭该开关。

发布前应完成：Python/前端/Rust 测试、`meta.json` 动态 URL 检查、签名正反例、只读数据库权限、table_meta/records 跨实现语义对比，以及真实飞书连接器同步。

## 安全边界

- 数据源必须使用专用只读 PostgreSQL 角色，只授予目标库/Schema 的 `CONNECT`、`USAGE`、`SELECT`。
- 生产 `SECRET_KEY` 必须为强随机值，并与飞书连接器配置一致。
- Python helper 在未配置 `HELPER_API_KEY` 时会放行；部署 Python 后端必须显式配置该值并限制 helper 路由网络来源。
- Python 的“带 timestamp + nonce、缺 signature”兼容路径不是强鉴权；在未迁移前应由反向代理和网络 allowlist 补强。
- Rust 生产路径必须保持 `ALLOW_UNSIGNED=false`，并验证过期时间、nonce 与签名失败均被拒绝。
- 自定义 SQL 必须限制为只读查询；数据库只读角色是防止绕过应用层检查的最终护栏。
- TLS 证书、客户端证书和数据库密码只通过部署 secret 注入；不要写入 datasource 日志或文档示例。
- `/metrics` 可能暴露运行特征，只应在受控监控网络中开放。

## 项目结构

```text
.
├── frontend/              # React 配置 iframe
├── backend/               # 当前自动部署的 Python/FastAPI 实现
├── backend-rs/            # Rust/Axum 灰度实现
├── docs/
│   ├── 多维表格「数据同步插件」开发者指南/
│   └── designs/           # 平台、前端与 Rust 重写设计
├── .github/workflows/     # Python 自动部署与 Rust 手动灰度
├── docker-compose.yml     # Python 后端 Compose
└── deploy.sh              # 现有部署脚本
```

## 文档索引

- [`docs/多维表格「数据同步插件」开发者指南/多维表格「数据同步插件」开发者指南.md`](docs/%E5%A4%9A%E7%BB%B4%E8%A1%A8%E6%A0%BC%E3%80%8C%E6%95%B0%E6%8D%AE%E5%90%8C%E6%AD%A5%E6%8F%92%E4%BB%B6%E3%80%8D%E5%BC%80%E5%8F%91%E8%80%85%E6%8C%87%E5%8D%97/%E5%A4%9A%E7%BB%B4%E8%A1%A8%E6%A0%BC%E3%80%8C%E6%95%B0%E6%8D%AE%E5%90%8C%E6%AD%A5%E6%8F%92%E4%BB%B6%E3%80%8D%E5%BC%80%E5%8F%91%E8%80%85%E6%8C%87%E5%8D%97.md)：飞书 Connector 协议基线。
- [`docs/designs/data-connector-platform.md`](docs/designs/data-connector-platform.md)：多数据源平台方向；未落地部分不代表当前能力。
- [`docs/designs/rust-backend-rewrite.md`](docs/designs/rust-backend-rewrite.md)：Rust 设计、兼容性与灰度验收标准。
- [`docs/designs/frontend-refactor-brief.md`](docs/designs/frontend-refactor-brief.md)：前端重构语境。
- [`backend-rs/README.md`](backend-rs/README.md)：Rust 构建、配置和切换 runbook。
- [`CHANGELOG.md`](CHANGELOG.md)：已记录的版本变化。
- [`WORKLOG.md`](WORKLOG.md)：阶段性决策与验证记录；其中未来计划不应作为线上状态依据。
