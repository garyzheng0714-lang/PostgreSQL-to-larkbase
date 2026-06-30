# FBIF DataBridge · Rust 后端

飞书多维表格数据同步连接器后端的 Rust 实现。无状态、拉取式 HTTP 服务。
设计与选型见 `../docs/designs/rust-backend-rewrite.md`。

## 端点
- `GET /meta.json` —— 插件元信息（防 CDN 缓存动态 `?v=`）
- `GET /health` / `GET /ready` —— 存活/就绪
- `GET /metrics` —— Prometheus 指标
- `POST /api/table_meta` —— 表结构（SHA-1 验签）
- `POST /api/records` —— 分页记录（SHA-1 验签）
- `POST /api/helper/*` —— 前端辅助（fail-closed 鉴权）

## 构建 / 测试 / 运行
```bash
cargo test                 # 单元测试
cargo build --release      # 本地 release 构建
BIND_ADDR=127.0.0.1:18082 SECRET_KEY=<密钥> ./target/release/fbif-databridge
```

## 配置（环境变量）
| 变量 | 默认 | 说明 |
|---|---|---|
| `SECRET_KEY` | `testBase` | 验签密钥；保持默认即 dev 模式（放行验签/helper） |
| `FRONTEND_URL` | `http://localhost:5173` | 前端配置页 URL（写入 meta.json） |
| `BIND_ADDR` | `0.0.0.0:8000` | 监听地址（生产 18082，Caddy 反代目标） |
| `HELPER_API_KEY` | 空 | 前端辅助接口 key；**生产必须设置**，否则 helper fail-closed 拒绝 |
| `MAX_ROW_LIMIT` | `50000` | 单次同步最大行数 |
| `PG_CONNECT_TIMEOUT` / `PG_QUERY_TIMEOUT` | `5` / `15` | 连接/查询超时（秒） |
| `POOL_MAX_SIZE` / `POOL_IDLE_TIMEOUT` / `POOL_MAX_POOLS` | `5` / `300` / `20` | 连接池 |
| `RUST_LOG` | `info` | 日志级别 |

## Docker
```bash
docker build -t fbif-databridge .   # 多阶段 musl 静态，最终镜像 ~10MB
```

## 部署灰度切换 Runbook（阶段 5，需人工执行）
> Python 后端在 `../backend/`，作为正确性 oracle 与回滚目标，切换完成前不下线。

1. **构建**：CI 出 musl 二进制 + 镜像，部署到服务器新容器（端口与 Python 不同，如 18083）。
2. **比对**：对生产只读副本/同库，用同一批 table_meta/records 请求对比 Rust 与 Python 输出（JSON 语义等价，token 字面豁免，见设计 §7.2）。
3. **灰度**：Caddy 按比例/按 header 分流到 Rust。观察 §7.1 指标。
4. **回滚触发阈值（任一命中即回滚）**：签名失败率 > 基线+1%；字段/records diff 非空；1254500 占比 > 0.5%；records P99 > 18s。
5. **回滚命令**：Caddy 反代 `reverse_proxy 127.0.0.1:18082` 切回 Python 容器端口 + `caddy reload`，秒级生效。
6. **完成**：meta.json 指向新前端/服务，全量切换，观察 24h 后下线 Python。

## 安全运维建议（最强护栏）
数据源连接建议使用**只读 PG 角色**（设计 §5.3）。服务侧已强制
`default_transaction_read_only=on` + `statement_timeout` + 标识符白名单/quoting + 自定义 SQL 黑名单，构成纵深防御。

## 已知与 Python 的差异（均为有意取舍，非缺陷）
- 分页 token 改用十六进制（修复 Python urlsafe-base64 含 `-` 违反协议字符集；token 对飞书不透明，不影响功能）。
- PG 数组 `text[]` 输出为 PG 字面量 `{a,b}`（Python 为 JSON `["a","b"]`）；均为 text 字段，值信息性。
- `bytea` 输出为 PG 文本（`\x..` hex）；Python 为 `str(bytes)` 的 Python repr。两者都仅作文本展示，hex 更通用。
- 大整数 `numeric`（>i64≈9.2e18）经 f64 可能丢精度；Feishu number 字段本身按 f64 存储，实际无额外损失。
- `timestamp without time zone` 按 UTC 解释（Python 用服务器本地时区，环境相关）；本实现更确定。
- 签名校验更严：生产（配置了 SECRET_KEY）一律要求签名，无签名直接拒绝（Python 在「无 ts/nonce 才拒绝」，存在旁路）。
- `verify-ca` 当前同时校验主机名（行为同 verify-full，偏严、errs safe）；精确「跳过主机名」待补。
