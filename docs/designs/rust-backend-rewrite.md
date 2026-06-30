# FBIF DataBridge · 后端 Rust 重构设计

> 状态：**待审定**（语言已定 Rust；本稿已纳入 Codex 对抗性审查全部有效意见，并经逐条核对现有 Python 源码确认）
> 日期：2026-06-30（v2，对抗性审查后修订）
> 关联：`WORKLOG.md`、`docs/多维表格「数据同步插件」开发者指南/`、现有 Python 后端 `backend/src/`
> 评审基线：现有 Python 已真机上线，作为**正确性 oracle**；本文所有「现状」均标注源码行号，便于实现期逐条比对。

---

## 1. 定位与第一性原理

**本质**：无状态、拉取式 HTTP 服务。飞书 Base 主动来调三个端点，我们被动响应。无插件自有数据库、无后台任务、无长连接、每请求幂等。

**工作负载拆解**：收 HTTPS → 验签（CPU 极轻）→ 解析 JSON（轻）→ **连 PostgreSQL + 跑 SQL + 分页拉数据（I/O，占绝大部分时间）** → 类型映射（轻）→ 序列化返回（轻）。→ **I/O 密集**，瓶颈在网络与数据库。

**语言结论（已定 Rust）——表述为「本项目约束下的风险最小选择」，非「他语言第一性原理必败」**：
- 长期无人值守服务的高频故障源是**内存安全 bug**与**并发数据竞争**（业界统计 ~70% 严重漏洞属内存安全）。
- Rust 在**编译期**消灭这两类故障，无 GC、资源占用与 C/C++ 同级 → 对「一次性开发、永久运行」这一目标，是把最致命的整类故障前移到编译期的选择。
- C++ 并非不能做（libpq + Boost.Asio 有成熟生产案例），但其手动内存管理把风险留到运行期，与本项目「极致稳定」诉求方向相反；Go 亦可行（I/O 模型下 GC 影响有限），但有 GC 停顿与更高内存占用，非「极致」。**在本项目的明确约束（极致稳定+极致资源效率+工期无限）下，Rust 是风险最小解。**

**「永久免维护」的诚实边界**：Rust 消灭内存/并发故障，**不等于零维护**。仍需维护的有：依赖安全公告（`cargo audit`/`cargo deny`）、`Cargo.lock` 锁定、MSRV、TLS 根证书（CA bundle）轮换、Docker base 与 CI toolchain。本设计将这些纳入「长期维护契约」（见 §6.7），而非假装不存在。

**不可变约束（协议边界，长期稳定）**：

| 端点 | 方法 | 签名 | 超时 |
|---|---|---|---|
| `/meta.json` | GET | 无 | 10s |
| `/api/table_meta` | POST | SHA-1 | 10s |
| `/api/records` | POST | SHA-1 | 20s |

- 签名 = `hex(SHA1(timestamp + nonce + secretKey + rawBody))` —— **是 SHA-1 拼接，不是 HMAC**（`signature.py:55-57`）。
- records 每批 ≤ `maxPageSize`（≤1000）、字段 ≤299、tableName ≤100、fieldID ≤50、primaryID ≤100。
- 16 种字段类型枚举 + property 结构。
- **所有协议层错误一律 HTTP 200**，body 为 `{code, msg, data:null}`，`msg` 是中英双语 JSON 字符串（`error_handler.py:88`、`main.py:70`）。
- `dataSourceConfigUiUri` 必须带动态参数防飞书 CDN 缓存（`meta.py`、CLAUDE.md）。

**输出字段 casing（以已上线 Python 为准，`response.py`）**：`fieldID` / `fieldName` / `fieldType` / `isPrimary` / `primaryID` / `nextPageToken` / `hasMore`。
⚠️ 官方 Node demo 的 `fieldId` / `primaryId`（小写 d）**是错的**，群组 idType `ChatLarkID`（demo）vs `ChatOpenID`（指南）以指南 + Python 为准。三者全部纳入 golden test。

---

## 2. 技术选型（每项给「为什么 + 参考谁」，已补审查缺口）

> 原则：纯 Rust、零 C 依赖（不引 libpq/OpenSSL 的 C 风险面）、优先 tokio 官方生态系。

### 2.1 运行时与 HTTP

| 关注点 | 选型 | 论证 |
|---|---|---|
| 异步运行时 | **tokio** | 事实标准，Cloudflare/AWS/Discord 基建在用。 |
| HTTP 框架 | **axum** | 见下方 axum vs actix 决策表。 |

**axum vs actix-web 决策表**：

| 维度 | axum | actix-web | 取舍 |
|---|---|---|---|
| 维护方 | tokio-rs 官方 | 社区（曾有维护中断史） | axum 与运行时同源，长期风险低 |
| 抽象模型 | Tower Service/Layer，函数式 handler | Actor + 自有运行时层 | axum 心智更简单，无 actor 状态机 |
| 中间件生态 | tower-http（超时/限流/body-limit/trace/catch-panic 全有） | actix 自有中间件 | tower-http 覆盖本项目全部需求 |
| 性能 | 与 actix 同档（均 hyper 级） | 略高基准分 | 本服务 I/O 密集，差异可忽略 |
| 风险 | Tower trait 抽象学习成本 | 历史维护波动 + unsafe 较多 | **选 axum**：可接受 Tower 抽象成本，换长期稳定与生态一致性 |

### 2.2 PostgreSQL 驱动与连接池

| 关注点 | 选型 | 论证 |
|---|---|---|
| PG 驱动 | **tokio-postgres** | 纯 Rust 异步驱动（`sfackler/rust-postgres`），**不依赖 libpq**（零 C 风险面）。**刻意不选 sqlx**：其杀手锏是编译期 SQL 校验，但本服务 SQL 运行时动态（用户提供连接/表名/自定义 SQL），编译期无从校验，优势用不上、反增复杂度。 |
| 单配置连接池 | **deadpool-postgres** | 仅负责「单个连接配置下的池化」。 |
| 多配置池管理 | **自建 `PoolManager`** | ⚠️ 关键澄清：deadpool 本身**不提供**「多配置缓存 + TTL + LRU 淘汰」。现状是自写 `dict[key]→Entry` + 锁 + cleanup loop + max_pools 淘汰（`pool.py:46-172`）。Rust 必须**自建 `PoolManager { HashMap<PoolKey, PoolEntry> }`**，每个 entry 内嵌一个 deadpool Pool。 |

**池语义对比（为何 deadpool 而非 bb8/mobc/sqlx-pool）**：

| 库 | acquire 超时 | 连接校验 | 回收/recycle | 取消安全 | 结论 |
|---|---|---|---|---|---|
| deadpool | ✅ `timeouts` 可配 | ✅ recycle 钩子 | ✅ 显式 | ✅ | **选它**：failure semantics 显式可控，适合每请求短生命周期 |
| bb8 | ✅ | ✅ | ✅ | ✅ | 可用，API 偏回调式，较繁 |
| mobc | ✅ | ✅ | ✅ | ⚠️ 维护活跃度较低 | 否 |
| sqlx 内建池 | ✅ | ✅ | ✅ | ✅ | 编译期 SQL 校验用不上，整体重 |

> 实现期需为 deadpool 配置：`wait_timeout`（接入「连接预算」）、`recycle`（连接前 ping）、`max_size`（对应 `pool_max_size`，现 5）。

### 2.3 TLS（对 PG 的 SSL）

| 关注点 | 选型 | 论证 |
|---|---|---|
| PG 的 SSL | **tokio-postgres-rustls + rustls** | 纯 Rust，绕开 OpenSSL 历史漏洞。 |
| 对外 HTTPS | **不在 Rust 内做** | Caddy 反代终结 TLS（→127.0.0.1）。Rust 只监听本地 HTTP，职责单一。 |

**六种 ssl_mode 必须落到协议级（现状 `datasource.py:70` + `ssl_context.py`，Rust 须等价实现）**：

| ssl_mode | rustls 行为 |
|---|---|
| `disable` | 不启用 TLS（返回无 TLS connector） |
| `allow` / `prefer` | 现状映射为 require（尽力加密；rustls 侧建立 TLS，验证按 require 处理） |
| `require` | 建立 TLS，**不校验**主机名/证书链（仅加密） |
| `verify-ca` | 校验证书链至给定 root CA，**不校验**主机名 |
| `verify-full` | 校验证书链 + **主机名/SNI**（IP 连接需特殊处理） |

> 额外边界：自定义 root CA（`ssl_root_cert`）加载、客户端证书 + 私钥（`ssl_cert`/`ssl_key`，mTLS）、RDS CA 轮换（CA bundle 进镜像，见 §6.6）、IP-vs-hostname 的 SNI 处理。每个 mode 一条集成测试。

### 2.4 其余 crate（清单锁死，含 feature）

| 用途 | crate | 备注 |
|---|---|---|
| JSON | `serde` + `serde_json` | ⚠️ 验签需 raw body：用 `axum::body::Bytes` 取原始字节，**验签后**才反序列化。 |
| 原始 body | `bytes` | extractor 取 `Bytes`，禁止前置 `Json<T>` 消费 body。 |
| SHA-1 | `sha1`（RustCrypto） | 纯 Rust、经审计。**不是 HMAC**。 |
| 十六进制 | `hex` | 签名输出与比较。 |
| 常量时间比较 | `subtle`（`ConstantTimeEq`） | 防时序侧信道，对应 Python `hmac.compare_digest`。 |
| 中间件 | `tower-http`（features：`timeout`、`limit`、`trace`、`catch-panic`、`cors`） | 超时/body-limit/日志/panic 隔离/CORS。 |
| 精确小数 | `rust_decimal` | PG `numeric`/`money`：整数→整型、小数→浮点、NaN/Inf→null。 |
| 日期时间 | `chrono` | → Unix **毫秒**；date 用 UTC 午夜毫秒。 |
| 错误定义 | `thiserror` | 错误枚举 → 飞书错误码映射。 |
| 日志/追踪 | `tracing` + `tracing-subscriber`（json 格式） | 结构化字段见 §6.5。 |
| 指标 | `metrics` + `metrics-exporter-prometheus` | `/metrics` 见 §6.5。 |
| 配置 | `std::env` + `Config` struct | 极简，对应 `config.py`。 |

### 2.5 async trait 决策（闭合，不留「热点再优化」含糊）

- 注册表需要动态分发（`Arc<dyn DataSourceAdapter>`，对应 `registry.py`）。
- **决策：用 `async-trait` 宏 + 接受 boxing 开销**。理由：本服务每请求一次数据库 I/O（几十毫秒），一次 `Box::pin` 分配（纳秒级）完全淹没在 I/O 里，零开销在此无意义。
- 若未来确需零开销：改为 `enum Adapter { Postgres(..), .. }` 静态分发，**不**用 trait object。但当前不做——多数据源动态注册的灵活性 > 纳秒级分配。

---

## 3. 架构与模块映射（Python → Rust，标注关键差异）

保留「适配器抽象层 + 注册表」（多数据源可扩展），PostgreSQL 为首个、当前唯一精雕适配器。

```
backend-rs/
├── Cargo.toml                  # 依赖清单锁死（§2.4）；[profile.release] lto/codegen-units/panic
├── Cargo.lock                  # 提交，锁定可复现构建
├── src/
│   ├── main.rs                 ← main.py         入口、优雅关闭、信号
│   ├── config.rs               ← config.py       环境变量 + 超时预算常量
│   ├── server.rs               ← main.py         axum router、中间件顺序（§4.1）、超时层、panic→200 层
│   ├── protocol/               ← models/
│   │   ├── request.rs          ← request.rs      ConnectorRequest / params / context
│   │   ├── response.rs         ← response.rs     casing 锁死：fieldID/primaryID/...
│   │   └── error.rs            ← error.rs        8 error_key → 5 飞书码 + 双语 msg（§8）
│   ├── signature.rs            ← signature.py    SHA-1 验签 extractor（raw body + 防重放 + subtle）
│   ├── handlers/               ← routers/
│   │   ├── meta.rs             ← meta.rs         GET /meta.json（动态 ?v= 防 CDN）
│   │   ├── table_meta.rs       ← table_meta.rs   主键判定/299 上限/tableName 清洗（1:1）
│   │   ├── records.rs          ← records.rs      分页（offset token，§4.3）
│   │   └── helper.rs           ← helper.rs       ⚠️ 鉴权收紧（§6.4）
│   ├── adapter/                ← adapters/
│   │   ├── mod.rs              ← base.py         DataSourceAdapter trait（async-trait）
│   │   ├── registry.rs         ← registry.rs     Arc<dyn ..> 注册表
│   │   └── postgres/
│   │       ├── mod.rs          ← service.py      PostgresAdapter
│   │       ├── pool.rs         ← pool.py         自建 PoolManager（HashMap+TTL+LRU），含安全配置入 key
│   │       ├── type_map.rs     ← type_mapper.py  55 条映射 + 数组兜底 + OID/udt 别名统一（§4.5）
│   │       ├── format.rs       ← formatter.py    值格式化，绝不 panic
│   │       └── tls.rs          ← ssl_context.py  六种 ssl_mode（§2.3）
│   └── util/                   ← utils/
│       ├── id_gen.rs           ← id_generator.py fieldID/primaryID 生成（字符集+长度）
│       ├── pagination.rs       ← pagination.py   ⚠️ 改 hex/十进制 token（§4.3）
│       └── params.rs           ← params_parser.py 飞书 params 健壮解析 + 可观测字段提取
└── tests/                      # golden + 集成测试（§7 失败样本库）
```

**抽象层落地**：`trait DataSourceAdapter`（13 方法，对应 `base.py` Protocol），`registry: HashMap<SourceType, Arc<dyn DataSourceAdapter>>`。

---

## 4. 协议正确性细节与风险清单（已转为可测试约束）

### 4.1 验签 extractor 与中间件顺序（重要）
- **必须**封装为自定义 axum extractor：先取 `Bytes` raw body → 验签 → 通过后才 `serde_json` 反序列化。
- **禁止**任何前置消费 body 的层（`Json<T>`、重序列化中间件）跑在验签前。
- 中间件顺序（外→内）：`trace` → `cors` → `body-limit` → `timeout` → **验签 extractor** → handler。`catch-panic` 包在最外层但其响应映射成协议 200 错误体（§4.6）。
- **测试**：body 含多余空格 / 字段乱序 / 重复 key / 非 UTF-8 字节，签名仍能逐字节匹配（因用 raw bytes，不重序列化）。

### 4.2 防重放
- 时间戳与当前时间差 >300s 拒绝（`signature.py:105`）。空签名放行分支 1:1 复刻：①有 ts+nonce 但无 sig（未配 secretKey）放行；②dev mode 放行；③否则 `SIGNATURE_INVALID`。

### 4.3 分页（无状态，token 字符集合规）
- **维持 offset page token，绝不引入服务端游标状态**（保持幂等、无状态）。
- ⚠️ **修复现有潜在 bug**：现状用 `base64.urlsafe_b64encode`（`pagination.py:18`），其字母表含 `-`，而协议要求 nextPageToken 只允许 `[A-Za-z0-9_]` ≤100（指南 :358）。**Rust 改用十进制或 hex 编码 offset**（offset 本就是整数，无需 base64+JSON）。
- `transactionID`：只用于日志关联，**不**参与游标状态，不引入内存态；可选嵌入 token 做一致性校验。
- **测试**：遍历大量 offset，断言生成的 token 全部满足 `^[A-Za-z0-9_]{1,100}$`；round-trip 解码一致。

### 4.4 字段类型映射（55 条，非 73）
- ⚠️ 修正：`_PG_TYPE_MAP` 实为 **55 个 key**（`type_mapper.py:17-73`），文档与 WORKLOG 原写「73」为笔误。
- 规则：数组 `xxx[]` → TEXT（`endswith("[]")` 兜底）；带参 `varchar(255)` 取 `(` 前 base；大小写不敏感 + trim；未知类型 → TEXT（不报错）。
- **golden tests** 必覆盖：enum / user-defined / 数组 / timestamptz / numeric(精度) / time→TEXT / money→CURRENCY / bool→CHECKBOX / 各别名。

### 4.5 OID 与 information_schema 双路径类型名统一（重要）
- ⚠️ 现状两条路径产出**不同类型字符串**：table 模式读 `information_schema.columns.data_type`（给 `character varying`，`service.py:181`）；custom SQL 经 OID→`pg_type.typname`（给 `varchar`，`service.py:209/311`）。
- 现 type_map 两别名都收了所以能 work，但脆弱。**Rust 统一**：映射输入同时接受 `data_type` / `udt_name` / OID→typname 三来源，并保证别名对（`character varying`≡`varchar` 等）映射一致。测试对同一 PG 类型走两条路径，断言映射结果相同。

### 4.6 错误与 panic 的 HTTP 语义（致命，必须正确）
- **协议错误一律 HTTP 200** + `{code, msg(双语), data:null}`（现状 `error_handler.py:88`）。
- **handler panic 也兜底成 HTTP 200 + `1254500` 双语 msg**——`tower-http::catch-panic` 的 PanicHandler 自定义为返回协议 200 错误体，**不是裸 500**。
- 仅**进程级 / 框架级**故障（如 body-limit 超限、超时层触发）才允许非 200；其中超时也应尽量映射为 200+`QUERY_TIMEOUT`（飞书期望协议体）。
- **测试**：handler 内强制 panic / 抛各类错误，断言 HTTP 状态=200 且 body 含正确 code+双语 msg。

### 4.7 三层超时预算闭合
- 协议硬限：table_meta 10s、records 20s。预算分配（须显式、相加不超）：
  - records：connect ≤3s + query ≤15s + 序列化/HTTP 余量 ≤2s = 20s。
  - table_meta：connect ≤3s + query ≤5s + 余量 ≤2s = 10s。
- connect/command timeout 经 deadpool + tokio-postgres 配置；最外层 `tower-http::timeout` 兜整体；超时→`QUERY_TIMEOUT`(1254500) 200 响应。现状默认 connect=5s/command=15s（`pool.py:54-55`）需按上述预算收紧并可配。

### 4.8 字段与表名约束（1:1 复刻 `table_meta.py`）
- fieldID `[A-Za-z0-9_]` ≤50（`id_generator.py`）；fieldName ≤300；tableName ≤100 且去 `/\?*[]:`（`table_meta.py:119-122`）；字段数 >299 → `TOO_MANY_FIELDS`。
- 主键判定：有 PK 列且可作主键 → 选它；无 PK → 首个 `can_be_primary` 列；兜底仍无 → 遍历选首个可作主键（`table_meta.py:78-117`）。有且仅一个 isPrimary。

### 4.9 值格式化绝不 panic（`formatter.py`）
- 文本(1/3/4/6/9/10/13)：list/dict→JSON，否则字符串；数字：Decimal 整→整、小→浮、NaN/Inf→null；日期→Unix 毫秒（date 用 UTC 午夜）；checkbox→bool；货币：去 `$`/`,`→浮点，NaN/Inf→null；**任何转换失败 → 字符串兜底**（Rust 用 `Result` + fallback，禁止 `unwrap`/`expect` 在数据路径）。

---

## 5. SQL 与标识符安全（达到「无妥协」标准）

现状（须复刻并强化）：标识符白名单 `^[\w][\w ]*$`（`datasource.py:7`）+ 双引号包裹拼接（`service.py:231/235`）；custom_sql 黑名单关键字（`datasource.py:104`）+ `SELECT * FROM (sql) AS _sub` 子查询包裹 + **readonly 事务**（`service.py:239/284`）。

**Rust 强化为纵深防御（每层独立可测）**：
1. **标识符**：白名单正则校验（拒绝引号/分号/括号/反斜杠）**+** quoting 函数对 schema/table/field 做双引号包裹并**转义内部 `"`→`""`**（现状未转义，靠正则兜底；Rust 显式转义）。selected_fields 同样处理。
2. **custom_sql**：① 黑名单关键字（声明为**辅助层非唯一防线**）；② 子查询包裹；③ **readonly 事务**（真正护栏——任何写操作在 `BEGIN READ ONLY` 内被 PG 拒绝）；④ `EXPLAIN` 预检语法/只读性。四层叠加。
3. **连接层**：建议（运维项，写入部署文档）数据源使用**只读 PG 角色**，把安全下沉到数据库权限——最强护栏。
- **测试**：注入样本（`"; DROP`、`-- 注释绕过`、CTE 写入、`COPY`）逐条断言被拒或在 readonly 下失败。

---

## 6. 「极致稳定 / 极致效率」工程落地

### 6.1 panic 边界
- `catch-panic` 包最外，PanicHandler → 200 + `1254500` 双语（§4.6）。`panic=abort` **不用**（会让单请求 panic 杀进程，与 catch-panic 冲突）；保留默认 unwind 让 catch-panic 隔离单请求。

### 6.2 连接池（自建 PoolManager）
- `HashMap<PoolKey, PoolEntry { pool: deadpool::Pool, last_used }>` + 后台 cleanup（TTL 空闲回收）+ LRU 淘汰（max_pools）+ `close_all`（优雅关闭）。复刻 `pool.py:46-172`。
- ⚠️ **PoolKey 必须含安全配置**：现状 key 仅 `host:port:user:db:ssl_mode`（`pool.py:31`），**不含 password/证书** → 改密码会复用旧池（用旧凭证连接）。Rust key = `host:port:user:db:ssl_mode + hash(password, ssl_root_cert, ssl_cert, ssl_key)`。**日志只打脱敏 key（绝不含密码/证书 hash 原文）**。

### 6.3 连接池价值量化（补基准）
- 同步模型：每次全量拉取、每页 ≤1000、单表上限默认 50000 行 → 一次同步约 ≤50 页连续请求（同 transactionID）。池复用避免每页冷连接（TLS 握手 + auth，RDS 上常 50-150ms/次）。
- 默认值依据并发同步租户数设定：`pool_max_size`（单配置，现 5）、`max_pools`（现 20）、`idle_timeout`（现 300s）。实现期补一组「冷连接 vs 池复用」基准数据回填本节。

### 6.4 helper API 安全收紧（重要）
- ⚠️ 现状 `verify_helper_api_key`（`helper.py:27`）：**未配 helper_api_key 则放行** → helper 接口（连任意库、列库/表/列、跑预览 SQL）在未配置时**公开可探测**。
- **Rust 改为「默认拒绝」**：未配置 key → helper 路由**不挂载**或一律 401（fail-closed）。dev mode 仍可放行但需显式 `DEV_MODE=1`。补 CORS 收紧（仅飞书前端域 + 自有前端域）与基础限流（tower 限流层）。

### 6.5 可观测性
- `tracing` span 必带字段：`tenant_key` / `user_tenant_key` / `log_id` / `base_open_id` / `biz_instance_id`（从 context 解析，`params_parser.py:83-89`）/ `endpoint` / `transaction_id`。
- Prometheus `/metrics`：请求数与错误码计数（按 5 个飞书码）、签名失败率、PG 查询延迟直方图、池 gauge（活跃池数/活跃连接/等待数）、每页记录数。
- 健康检查分离：`/health`（存活，进程在）与 `/ready`（就绪，可选探测——但**不**对用户 RDS 探测，仅自身）。

### 6.6 部署：musl / scratch（补 CA 与可复现）
- 静态编译 `x86_64-unknown-linux-musl` → 单文件 → **distroless/scratch 镜像（几 MB）**（对比 Python 数百 MB）。
- ⚠️ TLS 需 **CA bundle**：scratch 无系统证书 → 镜像内显式注入 CA（`ca-certificates` 或 RDS CA 文件），rustls 加载之；记录 CA 来源与轮换流程。
- 可复现构建：提交 `Cargo.lock`、固定 toolchain（`rust-toolchain.toml`）、固定 base digest；非 root 运行用户；镜像内置 tzdata（如需）；时钟依赖宿主（验签防重放需准时钟，部署文档注明 NTP）。
- `[profile.release]`：`lto = "fat"`、`codegen-units = 1`、`opt-level = 3`、`strip = true`。

### 6.7 长期维护契约
- `Cargo.lock` 提交；声明 MSRV；CI 跑 `cargo audit` + `cargo deny`（依赖安全/许可）；季度依赖升级窗口；CA bundle 轮换检查项。这些是「永久免维护」的真实成本，写入 README 运维章节。

### 6.8 大响应内存预算
- 单页最大：1000 records × ≤299 字段。估算单页 JSON 峰值（实现期实测回填），设 `axum` body limit（请求侧）与 response 流式/分块策略上限。压测目标：单页 1000×299 在内存上限内、records 端点 P99 < 20s。

---

## 7. 实施计划（分阶段，每阶段真机/集成验证才算完成）

> 工期无限 → 不赶进度；「编译通过 / tsc 通过」**不算**完成，须真机或对 oracle 逐字节比对。

- **阶段 0** 脚手架：Cargo 工程、`rust-toolchain.toml`、CI 交叉编 musl、distroless 镜像（含 CA）、`/meta.json` + `/health`。验收：飞书填服务地址能读到 meta.json。
- **阶段 1** 协议类型 + 验签：`protocol/*` + `signature.rs`（raw body extractor）+ §4.6 错误 200 语义 + §8 错误码。验收：验签/防重放/错误体 golden 全绿。
- **阶段 2** PG 只读路径：PoolManager + tls（六 mode）+ type_map（55+OID 统一）+ format（不 panic）+ table_meta（主键/299/清洗）+ records（offset token 合规）。验收：对真实 RDS 与 Python **逐字节比对**。
- **阶段 3** helper（鉴权 fail-closed）+ 前端联调。验收：前端配置页全流程；未配 key 时 helper 401。
- **阶段 4** 韧性：三层超时预算、panic→200、池 TTL/LRU、SQL 安全四层、限流、优雅关闭、可观测性。验收：**故障注入**（断连/超时/脏数据/畸形请求/注入样本/panic）不崩、返回正确协议码。
- **阶段 5** 灰度切换（见 §7.1）。

### 7.1 灰度与回滚（带触发条件，非仅方向）
- 切换：Python 与 Rust 并行 → Caddy 按比例/按 header 分流 → 观察 → meta.json 指向新服务 → 全量 → 下线 Python。
- **回滚触发阈值（任一命中即回滚）**：签名校验失败率 > 基线 + 1%；table_meta 字段 diff（vs Python）非空；records 抽样 diff 非空；错误码 1254500 比例 > 0.5%；records P99 > 18s。
- **回滚命令**：Caddy 反代目标切回 Python 容器（一条配置 + reload），秒级生效。步骤写入 `WORKLOG` 与部署 runbook。

### 7.2 oracle 失败样本库
- 固化对比样本集：签名（空格/乱序/非 UTF-8）、分页（边界 offset/token 字符集）、字段名（casing）、类型映射（55 条 + OID 双路径）、错误码（8→5）、超时。每次 Rust 构建跑全集 vs Python 输出。

---

## 8. 协议错误码与 HTTP 语义（权威表，源 `error.py`）

所有响应 **HTTP 200**。`msg` 为 `{"zh":..,"en":..}` JSON 字符串。

| 内部 error_key | 飞书 code | 触发 |
|---|---|---|
| CONNECTION_FAILED | 1254400 | 连接失败 / 解析配置失败 |
| INVALID_SQL | 1254400 | SQL 语法错误 |
| TOO_MANY_FIELDS | 1254400 | 字段 >299 |
| TABLE_NOT_FOUND | 1254400 | 表不存在 / 无列 |
| SIGNATURE_INVALID | 1254403 | 验签失败 / 过期 |
| PERMISSION_DENIED | 1254403 | 数据库权限不足 |
| QUERY_TIMEOUT | 1254500 | 查询/整体超时 |
| UNKNOWN_ERROR | 1254500 | 兜底 / panic |

> 1254501（限流）、1254505（付费）现未使用；如启用限流层（§6.4）则 1254501 投入。

---

## 9. 待你确认的开放问题

1. **前端**：保持 React + TypeScript 不变（浏览器约束 + 飞书 SDK 强制 JS），本次只重构后端。确认？
2. **新旧目录**：Rust 放新目录 `backend-rs/`，Python `backend/` 保留为正确性 oracle 直到阶段 5 切换完成。确认？
3. **部署标识 `pg2bitable`**：本次一并迁 `databridge`，还是后端重构完单独迁？
4. **只读 PG 角色**（§5.3）：是否要求数据源连接使用只读角色作为最强护栏（运维约定）？

---

## 附：Codex 对抗性审查处置记录（透明）

全部有效意见已采纳并落为可测试约束，且经逐条核对源码确认属实：

| 审查项 | 核实结论 | 处置位置 |
|---|---|---|
| 错误→500 破坏协议（致命） | 属实（`error_handler.py:88`） | §1、§4.6、§6.1、§8 |
| SQL/identifier 安全（致命） | 属实（`service.py:231/235`、`datasource.py:104`） | §5 |
| 分页 token 含 `-` 违规 | 属实（`pagination.py:18`） | §4.3 |
| pool key 不含密码/证书 | 属实（`pool.py:31`） | §6.2 |
| deadpool ≠ 多配置 TTL，须自建 | 属实（`pool.py:46-172`） | §2.2、§6.2 |
| helper 未配 key 即公开 | 属实（`helper.py:27`） | §6.4 |
| OID vs information_schema 类型路径不一 | 属实（`service.py:181/209`） | §4.5 |
| 字段映射 55 非 73 | 属实（`type_mapper.py`） | §4.4（含修正 WORKLOG） |
| fieldID/primaryID casing（demo 错） | 属实（`response.py`、`table_meta.py:104`） | §1、§7.2 |
| axum/actix 论证不足 | 采纳 | §2.1 决策表 |
| async-trait 未闭合 | 采纳 | §2.5 |
| crate 清单不全 + 误用 HMAC 表述 | 采纳 | §2.4、§1 |
| TLS 六 mode 未落地 | 采纳 | §2.3 |
| 三层超时未闭合 | 采纳 | §4.7 |
| 一性原理过度绝对 / 永久免维护乐观 | 采纳软化 | §1、§6.7 |
| 灰度无回滚阈值 | 采纳 | §7.1 |
| 可观测性不具体 | 采纳 | §6.5 |
| musl/scratch CA 风险 | 采纳 | §6.6 |
| 大响应内存未量化 | 采纳 | §6.8 |
| oracle 失败样本库 | 采纳 | §7.2 |
