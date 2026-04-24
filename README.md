# PostgreSQL-to-larkbase

## Overview

把 PostgreSQL 表、视图或自定义 SQL 查询结果同步到飞书多维表格的自定义连接器。

项目由两部分组成：

- `backend/`：FastAPI 后端，实现连接器协议、签名校验、表结构读取和数据分页读取
- `frontend/`：React + Vite 配置页，在多维表格中收集数据库连接、表/SQL 和字段映射配置

典型使用方式是把前后端部署到自己的公网 HTTPS 域名，然后在飞书多维表格里以“自定义连接器 / 自助连接器”的形式接入。

## Features

- 支持 PostgreSQL 表、视图和自定义 SQL
- 支持 Schema / 表选择、字段选择、字段重命名和数值精度配置
- 将 PostgreSQL 类型映射到多维表格常见字段类型
- 通过 `meta.json` 暴露飞书连接器元数据
- 支持 `/api/table_meta` 表结构接口和 `/api/records` 数据接口
- 支持飞书连接器请求签名校验
- 支持前端辅助接口、CORS 配置、连接池清理和查询超时
- 提供后端测试、Ruff、mypy 和 Docker Compose 配置

## Current Limits

- 单次同步最多返回 `50000` 行，可通过 `MAX_ROW_LIMIT` 调整
- 字段数最多 `299`
- 自动同步频率由多维表格侧调度，后端只声明同步能力
- 未识别的 PostgreSQL 类型会退化为文本

## Tech Stack

| Area | Stack |
| --- | --- |
| Backend | Python 3.11, FastAPI, asyncpg, Pydantic, ORJSON |
| Frontend | React 18, TypeScript, Vite, Semi UI, Lark connector API |
| Tooling | uv, pytest, ruff, mypy, Docker Compose |

## Project Structure

```text
.
├── backend/
│   ├── src/
│   │   ├── adapters/postgres/  # PostgreSQL formatter, pool, service, type mapping
│   │   ├── middleware/         # Error handling and connector signature checks
│   │   ├── models/             # Request, response, datasource, and error models
│   │   ├── routers/            # meta.json, table_meta, records, helper endpoints
│   │   ├── services/           # SSL context helpers
│   │   ├── utils/              # Pagination, params parsing, ID generation
│   │   ├── config.py           # Environment-backed settings
│   │   └── main.py             # FastAPI application
│   ├── tests/
│   ├── pyproject.toml
│   └── uv.lock
├── frontend/
│   ├── src/
│   │   ├── api/                # Backend/helper API client code
│   │   ├── components/         # Connection, table, field, and sync UI
│   │   ├── hooks/              # Lark Bitable/config hooks
│   │   └── App.tsx
│   ├── package.json
│   └── vite.config.ts
├── docker-compose.yml
└── deploy.sh
```

## How It Works

1. 飞书多维表格读取后端的 `https://your-domain.example/meta.json`。
2. `meta.json` 返回连接器类型、配置页地址 `dataSourceConfigUiUri`、表结构接口和数据接口。
3. 用户在前端配置页填写 PostgreSQL 连接信息，选择表/视图或输入自定义 SQL。
4. 多维表格保存 `datasourceConfig` 后调用 `/api/table_meta` 获取字段结构。
5. 多维表格调用 `/api/records` 分页拉取数据并写入同步表。

## Backend Setup

```bash
cd backend
cp .env.example .env
uv sync --extra dev
uv run uvicorn src.main:app --reload --host 0.0.0.0 --port 8000
```

Backend environment variables:

| Variable | Purpose |
| --- | --- |
| `SECRET_KEY` | 飞书连接器请求签名密钥，生产环境必须改成强随机值 |
| `FRONTEND_URL` | 前端配置页公网地址，会写入 `meta.json` |
| `BACKEND_URL` | 后端公网地址 |
| `CORS_ORIGINS` | 允许访问后端的前端和飞书域名列表 |
| `PG_CONNECT_TIMEOUT` | PostgreSQL 连接超时时间 |
| `PG_QUERY_TIMEOUT` | PostgreSQL 查询超时时间 |
| `MAX_ROW_LIMIT` | 单次同步最大行数 |
| `HELPER_API_KEY` | 前端辅助接口可选 API key |
| `POOL_MAX_SIZE` | 单个连接池最大连接数 |
| `POOL_IDLE_TIMEOUT` | 空闲连接池清理时间 |
| `POOL_MAX_POOLS` | 最大连接池数量 |

## Frontend Setup

```bash
cd frontend
npm ci
VITE_API_BASE_URL=http://localhost:8000 npm run dev
```

Build for same-origin deployment:

```bash
VITE_API_BASE_URL="" npm run build
```

Build for split frontend/backend domains:

```bash
VITE_API_BASE_URL="https://pg2base-api.example.com" npm run build
```

## Commands

Backend:

```bash
cd backend
uv run pytest
uv run ruff check src tests
uv run mypy src
```

Frontend:

```bash
cd frontend
npm run dev
npm run build
npm run preview
```

Docker backend:

```bash
docker compose up -d --build
```

`docker-compose.yml` currently expects an external Docker network named `postgres_default`. If your PostgreSQL service is elsewhere, adjust the Compose network configuration or remove the external network block.

## API Surface

| Endpoint | Purpose |
| --- | --- |
| `/health` | Health check, returns `{"status":"ok"}` |
| `/meta.json` | Connector metadata consumed by Feishu Bitable |
| `/api/table_meta` | Return table/SQL field metadata |
| `/api/records` | Return paginated records |
| `/api/helper/*` | Helper endpoints used by the frontend configuration UI |

## Deployment Notes

Recommended production topology:

- `https://pg2base.example.com/` serves the built frontend
- `https://pg2base.example.com/meta.json` proxies to the backend
- `https://pg2base.example.com/api/*` proxies to the backend

For split-domain deployments, set `FRONTEND_URL`, `CORS_ORIGINS`, and `VITE_API_BASE_URL` to the real frontend/backend domains.

Existing project documentation referenced `https://pg2bitable.garyzheng.com/meta.json` as the author's deployed connector metadata endpoint. For your own deployment, register your own `meta.json` URL in Feishu Bitable.

## PostgreSQL Permission Suggestions

Use a dedicated read-only PostgreSQL user for synchronization:

```sql
GRANT CONNECT ON DATABASE your_database TO sync_user;
GRANT USAGE ON SCHEMA public TO sync_user;
GRANT SELECT ON ALL TABLES IN SCHEMA public TO sync_user;
```

Restrict PostgreSQL network access to the connector server when possible.

示例：

```sql
CREATE USER bitable_sync WITH PASSWORD 'replace_me';
GRANT CONNECT ON DATABASE your_db TO bitable_sync;
GRANT USAGE ON SCHEMA public TO bitable_sync;
GRANT SELECT ON ALL TABLES IN SCHEMA public TO bitable_sync;
ALTER DEFAULT PRIVILEGES IN SCHEMA public GRANT SELECT ON TABLES TO bitable_sync;
```

## 字段类型映射

当前内置映射大致如下：

- 文本：`text`、`varchar`、`uuid`、`json/jsonb`、数组和大部分未知类型
- 数字：`int`、`bigint`、`numeric`、`decimal`、`serial`
- 日期：`date`、`timestamp`、`timestamptz`
- 复选框：`bool`
- 货币：`money`

说明：

- `time` 和时区时间目前会按文本处理
- 数组会按文本处理
- `json/jsonb` 会序列化成字符串

## 常见排障

### 配置页能打开，但测试连接失败

优先检查：

- PostgreSQL 地址、端口、防火墙
- 数据库名是否存在
- 用户名密码是否正确
- 账号是否有 `SELECT` 权限

### 配置页打不开或是空白

优先检查：

- `FRONTEND_URL` 是否为公网 HTTPS 地址
- 前端静态文件是否真的部署到了 Nginx
- 如果前后端分域名，`CORS_ORIGINS` 是否包含前端域名

### 多维表格提示签名失败

优先检查：

- 多维表格里配置的签名密钥是否和 `SECRET_KEY` 一致
- 反向代理是否篡改了请求体
- 生产环境是否还在使用默认 `testBase`

### `docker compose up` 报网络不存在

这是因为仓库示例的 [docker-compose.yml](docker-compose.yml) 依赖外部网络 `postgres_default`。大多数自部署场景可以直接删掉这段网络配置。

### 自定义 SQL 预览失败

优先检查：

- SQL 语法是否正确
- SQL 是否引用了当前账号无权限访问的对象
- SQL 返回字段数是否过多
- SQL 执行是否超时

## 本地开发

### 前端

```bash
cd frontend
npm ci
npm run dev
```

### 后端

项目要求 Python `>= 3.11`。仓库中的 Docker 镜像使用的是 Python `3.12`。

```bash
cd backend
uv sync --dev
uv run uvicorn src.main:app --reload --host 0.0.0.0 --port 8000
```

本地开发时可以参考 [backend/.env.example](backend/.env.example)。

## 生产建议

- 使用 HTTPS，不要裸露 HTTP
- 使用只读数据库账号
- 把 `SECRET_KEY` 换成随机长字符串
- 用防火墙限制数据库来源 IP
- 为 Nginx 和容器日志做轮转
- 针对大表优先使用视图或自定义 SQL 做裁剪

## 当前仓库里几个需要注意的文件

- [deploy.sh](deploy.sh)：这是当前作者机器上的部署脚本示例，不是通用的一键安装脚本
- [docker-compose.yml](docker-compose.yml)：能直接起后端，但网络配置默认偏作者环境
- [backend/.env.example](backend/.env.example)：开发环境变量示例

## 下一步可做的增强

如果你准备把这个连接器长期给团队使用，建议继续补这些能力：

- 在后端直接托管前端静态文件，减少 Nginx 配置复杂度
- 增加更完整的生产版 `docker-compose.yml`
- 增加鉴权白名单和访问日志脱敏
- 增加 SQL 安全限制和只读校验
- 增加多租户隔离与监控告警
