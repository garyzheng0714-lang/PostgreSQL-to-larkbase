# FBIF DataBridge · 数据桥

![类型](https://img.shields.io/badge/%E7%B1%BB%E5%9E%8B-%E9%A3%9E%E4%B9%A6%E8%BF%9E%E6%8E%A5%E5%99%A8-2563eb)
![技术栈](https://img.shields.io/badge/%E6%8A%80%E6%9C%AF%E6%A0%88-FastAPI%20%7C%20React%20%7C%20PostgreSQL-0f766e)
![状态](https://img.shields.io/badge/%E7%8A%B6%E6%80%81-%E5%8F%AF%E8%87%AA%E9%83%A8%E7%BD%B2-16a34a)
![README](https://img.shields.io/badge/README-%E4%B8%AD%E6%96%87-111827)

把数据库同步到飞书多维表格的自定义连接器。当前首发适配 PostgreSQL（RDS），架构预留多数据源扩展。

飞书多维表格自定义连接器，将 PostgreSQL 表、视图或自定义 SQL 查询结果按连接器协议同步到多维表格。

## 仓库定位

| 项目 | 说明 |
| --- | --- |
| 分类 | 飞书插件 / 多维表格自定义连接器 / 数据源同步 |
| 服务对象 | 需要把 PostgreSQL 数据以飞书多维表格“自定义连接器 / 自助连接器”方式接入的团队 |
| 与其他表格仓库的区别 | 它不是广告平台报表写入服务，也不是一次性 CSV/Excel 转换器；它专门实现 PostgreSQL 到飞书多维表格的连接器协议 |
| 组成 | `backend/` 提供 FastAPI 连接器接口，`frontend/` 提供嵌入飞书多维表格的配置页 |

## 功能概览

- 支持 PostgreSQL 表、视图和自定义 SQL。
- 支持数据库/Schema/表选择、字段选择、字段重命名和数值精度配置。
- 将 PostgreSQL 类型映射到飞书多维表格常见字段类型。
- 通过 `/meta.json` 暴露连接器元数据，声明配置页、表结构接口和数据接口。
- 提供 `/api/table_meta` 表结构接口和 `/api/records` 分页数据接口。
- 支持飞书连接器请求签名校验、CORS 配置、连接池清理和查询超时。
- 提供 pytest、Ruff、mypy、Vite build 和 Docker Compose 配置。

## 工作方式

1. 飞书多维表格读取后端公网地址上的 `meta.json`。
2. `meta.json` 返回连接器类型、配置页地址 `dataSourceConfigUiUri`、表结构接口和数据接口。
3. 用户在前端配置页填写 PostgreSQL 连接信息，选择表/视图或输入自定义 SQL。
4. 多维表格保存 `datasourceConfig` 后调用 `/api/table_meta` 获取字段结构。
5. 多维表格调用 `/api/records` 分页拉取数据并写入同步表。

## 后端快速开始

```bash
cd backend
cp .env.example .env
uv sync --extra dev
uv run uvicorn src.main:app --reload --host 0.0.0.0 --port 8000
```

常用后端配置：

| 变量 | 用途 |
| --- | --- |
| `SECRET_KEY` | 飞书连接器请求签名密钥，生产环境必须改成强随机值 |
| `FRONTEND_URL` | 前端配置页公网地址，会写入 `meta.json` |
| `BACKEND_URL` | 后端公网地址 |
| `CORS_ORIGINS` | 允许访问后端的前端和飞书域名列表 |
| `PG_CONNECT_TIMEOUT` | PostgreSQL 连接超时时间 |
| `PG_QUERY_TIMEOUT` | PostgreSQL 查询超时时间 |
| `MAX_ROW_LIMIT` | 单次同步最大行数 |
| `HELPER_API_KEY` | 前端辅助接口 API key；Rust 生产后端会强制校验，前端构建需注入同值 `VITE_HELPER_API_KEY` |
| `POOL_MAX_SIZE` | 单个连接池最大连接数 |
| `POOL_IDLE_TIMEOUT` | 空闲连接池清理时间 |
| `POOL_MAX_POOLS` | 最大连接池数量 |

## 前端快速开始

```bash
cd frontend
npm ci
VITE_API_BASE_URL=http://localhost:8000 npm run dev
```

同域部署构建：

```bash
VITE_API_BASE_URL="" npm run build
```

前后端分域部署构建：

```bash
VITE_API_BASE_URL="https://pg2base-api.example.com" npm run build
```

## 常用命令

后端：

```bash
cd backend
uv run pytest
uv run ruff check src tests
uv run mypy src
```

前端：

```bash
cd frontend
npm run dev
npm run build
npm run preview
```

Docker 后端：

```bash
docker compose up -d --build
```

`docker-compose.yml` 当前依赖外部 Docker network `postgres_default`。如果 PostgreSQL 不在该网络中，请调整 Compose 网络配置或移除外部网络块。

## API 表面

| 端点 | 说明 |
| --- | --- |
| `/health` | 健康检查，返回 `{"status":"ok"}` |
| `/meta.json` | 飞书多维表格读取的连接器元数据 |
| `/api/table_meta` | 返回表/SQL 字段结构 |
| `/api/records` | 返回分页数据记录 |
| `/api/helper/*` | 配置页使用的辅助接口 |

## 项目结构

```text
.
├── backend/
│   ├── src/
│   │   ├── adapters/postgres/  # PostgreSQL formatter、pool、service、type mapping
│   │   ├── middleware/         # 错误处理和连接器签名校验
│   │   ├── models/             # 请求、响应、数据源和错误模型
│   │   ├── routers/            # meta.json、table_meta、records、helper endpoints
│   │   ├── services/           # SSL context helper
│   │   ├── utils/              # 分页、参数解析、ID 生成
│   │   ├── config.py           # 环境变量配置
│   │   └── main.py             # FastAPI application
│   ├── tests/
│   ├── pyproject.toml
│   └── uv.lock
├── frontend/
│   ├── src/
│   │   ├── api/                # 后端/helper API client
│   │   ├── components/         # 连接、表选择、字段和同步配置组件
│   │   ├── hooks/              # Lark Bitable/config hooks
│   │   └── App.tsx
│   ├── package.json
│   └── vite.config.ts
├── docs/
├── docker-compose.yml
└── deploy.sh
```

## 部署备注

推荐生产拓扑：

- 前端静态资源由 HTTPS 域名提供。
- `/meta.json` 和 `/api/*` 代理到 FastAPI 后端。
- `FRONTEND_URL`、`BACKEND_URL`、`CORS_ORIGINS` 和 `VITE_API_BASE_URL` 使用真实公网地址。

生产环境建议使用专门的只读 PostgreSQL 用户：

```sql
GRANT CONNECT ON DATABASE your_database TO sync_user;
GRANT USAGE ON SCHEMA public TO sync_user;
GRANT SELECT ON ALL TABLES IN SCHEMA public TO sync_user;
```

## 已知限制

- 单次同步最多返回 `50000` 行，可通过 `MAX_ROW_LIMIT` 调整。
- 字段数最多 `299`。
- 自动同步频率由飞书多维表格侧调度，后端只声明同步能力。
- 未识别的 PostgreSQL 类型会退化为文本。
- `time`、数组、`json/jsonb` 等类型会按文本或序列化字符串处理。

## 常见排障

- 配置页能打开但测试连接失败：检查 PostgreSQL 地址、端口、防火墙、数据库名、账号密码和 `SELECT` 权限。
- 配置页空白：检查 `FRONTEND_URL` 是否为公网 HTTPS、静态资源是否部署成功，以及 `CORS_ORIGINS` 是否包含前端域名。
- 多维表格提示签名失败：检查飞书侧签名密钥是否与 `SECRET_KEY` 一致，反向代理是否改写请求体。
- `docker compose up` 报网络不存在：调整或删除 `postgres_default` 外部网络配置。
