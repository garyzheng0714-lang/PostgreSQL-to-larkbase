# FBIF DataBridge · 数据桥

## Project Overview
飞书多维表格自定义数据连接器，把数据库同步到飞书 Bitable。当前首发适配 PostgreSQL（RDS），架构预留多数据源扩展（见 `backend/src/adapters/`）。

## Architecture
- **Frontend**: React + Vite + Semi UI (`frontend/`)，作为飞书 iframe 内的配置页面
- **Backend**: 生产跑 Rust 重写版 `backend-rs/`（容器 `pg2bitable-backend-rs`）；`backend/` 是旧 Python 版，已不部署
- **Adapter Layer**: `backend/src/adapters/` — 数据源适配器抽象层，支持多数据源扩展
  - `base.py` — DataSourceAdapter Protocol + 共享数据类型
  - `registry.py` — 全局适配器注册表
  - `postgres/` — PostgreSQL 适配器实现（含连接池、类型映射、值格式化）
- **Error Handling**: `backend/src/middleware/error_handler.py` — 统一 ConnectorError + 全局异常处理
- **Reverse Proxy**: Caddy，域名 `pg2bitable.garyzheng.com`
- **Server**: 阿里云 ECS `121.40.214.5`（2026-09-13 从已下线的 112.124.103.65 迁入）

## Deployment
- **Auto Deploy**: push 到 `main`（或手动 workflow_dispatch）触发 `.github/workflows/deploy.yml`
- **CI**: 前端 `npm test` + build；Rust 镜像在 CI 构建，tag 为提交 SHA，scp 到服务器 `docker load`
- **Server**: 改 `/opt/fbif-one-api/services/postgres-to-feishu/compose.yaml` 的镜像 tag，跑同目录 `deploy.sh apply`；前端换到同目录 `frontend/dist`（上一版留在 `dist.prev`）；健康/公网校验失败自动回到上一个 tag 和上一版前端
- **Caddy**（`/etc/caddy/sites/pg2bitable.caddy`）serve 前端静态文件，反代后端 `127.0.0.1:18083`
- 仓库根目录的 `deploy.sh`、`docker-compose.yml` 是旧 Python 部署，勿用

## Critical: Feishu CDN Cache (MUST FOLLOW)

飞书服务端会缓存 `dataSourceConfigUiUri` 指向的前端页面。一旦缓存，即使你更新了前端代码，飞书仍然加载旧版本。

### 症状
- 修改了前端 UI 并成功部署
- 直接浏览器访问能看到新页面
- 但飞书多维表格里的连接器配置页面还是旧的
- 服务器日志里看不到前端页面请求（只有 meta.json 和 API 请求）

### 解决方案
`dataSourceConfigUiUri` **MUST** 包含动态参数（时间戳），防止飞书缓存：

```python
# backend/src/routers/meta.py
"dataSourceConfigUiUri": f"{settings.frontend_url}?v={int(time.time())}"
```

### NEVER
- **NEVER** 使用固定 URL 作为 `dataSourceConfigUiUri`（如 `https://pg2bitable.garyzheng.com`）
- **NEVER** 假设"版本号变了飞书就会重新加载前端"——飞书按 URL 缓存，不看 version 字段

## Feishu Connector Protocol

### meta.json
- `GET /meta.json` — 无需认证，返回插件元数据
- `dataSourceConfigUiUri` — 飞书在 iframe 中加载此 URL 展示配置页面
- `version` — 插件版本号，更新后需在飞书连接器中心"更新版本"

### API Endpoints
- `POST /api/table_meta` — 签名认证，返回表结构
- `POST /api/records` — 签名认证，返回数据记录

### Signature Verification
飞书请求使用 SHA-1 签名：`SHA1(timestamp + nonce + secretKey + body).hex()`
Headers: `X-Base-Request-Timestamp`, `X-Base-Request-Nonce`, `X-Base-Signature`

## Database
- 本机 PostgreSQL 端口 `5433`，用户 `postgres`
- Docker PostgreSQL（shared-postgres）端口 `5432`，用户 `admin`

## Key Files
- `backend/src/adapters/base.py` — DataSourceAdapter Protocol 定义
- `backend/src/adapters/registry.py` — 适配器注册表（注册/查找/关闭）
- `backend/src/adapters/postgres/service.py` — PostgreSQL 适配器实现
- `backend/src/adapters/postgres/pool.py` — 连接池管理器（多配置缓存 + TTL 清理）
- `backend/src/middleware/error_handler.py` — ConnectorError + 全局异常处理器
- `backend/src/routers/meta.py` — meta.json 端点（含缓存破坏逻辑）
- `backend/src/middleware/signature.py` — 飞书签名验证
- `frontend/src/hooks/useBitable.ts` — 飞书 Connector SDK 集成
- `frontend/src/hooks/useConfig.ts` — 默认连接配置（host/port/username）
- `.github/workflows/deploy.yml` — 自动部署工作流
