# WORKLOG

## 2026-06-30 项目改名：pg2bitable → DataBridge（数据桥）

### 目标
项目定位为「数据库 → 飞书多维表格」的同步连接器。数据源用 RDS PostgreSQL，
架构上预留多数据源扩展（已有 `backend/src/adapters/` 抽象层 + registry）。
新名字让 PostgreSQL 成为「首发适配数据源」，而非品牌的一部分。

最终名：**FBIF DataBridge / 数据桥**（用户在候选 BaseSync / DataBridge / Bitable Connector / 保留 pg2bitable 中选定 DataBridge，后加 FBIF 品牌前缀）。
package name: `fbif-databridge-frontend`。

### 改名分层（按风险）
- **A 层 — 文档展示层（零风险，本次执行）**
  - `README.md` 标题与正文表述
  - `CLAUDE.md` 项目标题 + Project Overview
  - `frontend/package.json` 的 `name` 字段
- **B 层 — 部署基础设施层（牵动线上，本次不动）**
  - 域名 `pg2bitable.garyzheng.com`（DNS + 服务器 Caddy）
  - docker 容器名 `pg2bitable-backend`（docker-compose.yml）
  - 服务器路径 `/opt/pg2bitable`（deploy.sh、CI deploy.yml）
  - 理由：改这些需同步迁移服务器域名/Caddy/目录，否则线上服务断连。
    属于单独的「部署迁移」任务，待用户决定后单独做（先应用新规则再撤旧规则）。
- **目录名** `多维表格连接器插件-PostgreSQL同步到多维表格`：涉及本地路径 + CodeGraph/graphify 索引重建，待用户确认是否改。

---

## 2026-06-30 后端重构：Python → Rust（地基级决策）

### 项目重新定位（用户给定，亘古不变）
1. 一切改动须在「多维表格数据同步插件开发者指南」框架内（`docs/多维表格「数据同步插件」开发者指南/`）。
2. 这是**底层基础能力**：一次性开发、永久免维护、追求**极致稳定 + 极致资源效率**，无妥协，工期无限。
3. 多参考成熟开源库，从第一性原理出发。

### 语言决策：Rust（已与用户确认）
- 第一性原理：服务是**纯 I/O 密集**（瓶颈在网络+PG，非 CPU）；长期运行头号杀手是**内存安全+数据竞争**（~70% 严重漏洞源于内存安全）。
- 结论：C++ 的手动内存管理恰是长期稳定最大风险源、计算优势在 I/O 场景发挥不出；**Rust 编译期消灭内存安全/数据竞争 + 无 GC + 资源与 C++ 同级**，是「永久免维护」的正确答案。否决 C++、Go。

### 调研产出（已完成）
- 读透开发者指南：协议边界（3 端点 / SHA-1 验签 / 超时 / 字段类型 / 错误码）。
- 解压并读官方 Node demo（`data-sync-be-demo.zip`）：发现 demo 用 `JSON.stringify` 验签是**坑**；demo 与指南有 `primaryId/primaryID`、`ChatLarkID/ChatOpenID` 矛盾，待以已上线 Python 为准核实。
- 摸清现有 Python 后端（2290 行 / 28 模块）作为**正确性 oracle**：验签（raw body 正确）、类型映射表（**55 条**，原误写 73）、值格式化规则已掌握。
- 选定技术栈：tokio + axum + tokio-postgres + deadpool + rustls + serde + rust_decimal + chrono + tracing（逐项 why 见设计文档）。

### 交付物
- **设计文档 v2**：`docs/designs/rust-backend-rewrite.md` —— 已纳入 Codex 对抗性审查全部有效意见，并逐条核对源码确认。

### Codex 对抗性审查（已完成，全部意见采纳）
评分 5/10 → 经修订。两个**致命缺陷**已核实属实并修复设计：
1. 错误→HTTP 500 破坏协议（现有 Python 是 HTTP 200 + {code,msg,data}）→ §4.6/§6.1：协议错误与 panic 均兜底 200。
2. SQL/identifier 安全不足 → §5：标识符白名单+quoting转义、custom_sql 四层纵深防御、建议只读 PG 角色。
其余核实属实的现状缺陷（写入设计）：分页 token 含 `-` 违规、pool key 不含密码/证书、helper 未配 key 即公开、OID/information_schema 类型路径不一、类型映射 55 非 73、casing（demo 的 fieldId/primaryId 是错的）。
软化项：第一性原理表述改「本项目约束下风险最小」；「永久免维护」补长期维护契约（cargo audit / CA 轮换 / MSRV）。
完整处置记录见设计文档「附：Codex 对抗性审查处置记录」。

### 状态
- [x] 项目重新定位（3 条原则）
- [x] 语言决策 Rust（用户确认）
- [x] 调研：指南 + 官方 demo + 现有 Python 正确性基线 + Rust 选型
- [x] 设计文档 v1 落盘
- [x] Codex 对抗性审查 + 逐条核实源码 + 设计文档 v2 修订
- [x] 4 个开放问题取默认：前端不动 / 新目录 backend-rs / pg2bitable 后迁 / 只读角色作建议
- [x] 阶段 0：Cargo 脚手架 + meta.json + health（真机验证 meta.json 动态 ?v=）
- [x] 阶段 1：协议类型 + SHA-1 验签 extractor + 错误 200 语义（12 单测绿）
- [x] 阶段 2：PG 适配器（pool/tls/type_map/format/mod）+ table_meta + records
      → 真机 PG 17.9 端到端验证：类型映射/PK 识别/日期时区折算/NULL/整数化全部正确
      → 值解码用 simple_query 文本协议（天然覆盖所有 PG 类型）
- [x] 阶段 3：helper 接口（鉴权 fail-closed，收紧 Python）
- [x] 阶段 4：韧性中间件（catch-panic→200 / body-limit / cors / 兜底超时）+ Prometheus /metrics + /ready
      → 真机验证：签名强制(1254403)、helper fail-closed(401)、签名经 shasum 独立交叉验证、指标
- [x] 阶段 5：Dockerfile（musl 多阶段 ~10MB）+ .dockerignore + README runbook（灰度/回滚阈值）
      注：实际生产切换涉及线上 DNS/Caddy，留人工执行，不自动切。
- [x] 全量 48 单测绿 + 真机端到端
- [x] **codex 全量审查（24 文件逐行）+ 据结果修改 + 真机复验**

### Codex 审查修复（全部 MUST-FIX 已修，真机验证）
必修（致命/正确性/安全）——已修复并验证：
1. 多语句 SQL 注入：custom_sql 用 simple_query 允许 `;` 多语句逃逸（`...) AS _sub; SET ...`）→ 拒绝内部分号（仅允许尾部）。✅真机：注入→1254400
2. 签名旁路：生产模式「有 ts+nonce 无 sig 即放行」→ 改为仅 dev 放行。✅真机：→1254403
3. 分页死循环/越界：maxPageSize=0 死循环 + 可能多返回一行 → clamp 1..=1000、effective=min(remaining)、checked_add 防溢出。✅真机
4. primaryID 重复：selected_fields 排除主键时全行 primaryID 相同 → 主键列不全在结果中则回退唯一行号。
5. selected_fields=[] 不一致：空字段 table_meta 返回空而 records 当 `*` → 归一空为 None + 筛选后空报配置错误。
6. regclass 主键检测：未引用标识符遇大写/特殊名失败 → quote_ident 引用。
7. TLS：allow/prefer 可能回落明文 → 一律强制 require；客户端证书(mTLS)此前未加载 → 加载 ssl_cert/ssl_key。
8. 超时违反协议：504 → 每端点 tokio timeout（table_meta 10s/records 20s）映射 200+1254500；连接池 acquire 加等待超时。
9. fieldName 未截断/去[] → clean_field_name(去[]+截断300)。
10. CA 解析错误静默吞 → 解析失败一律报 ConnectionFailed。
11. 日志泄露密码 hash → 淘汰日志不打印含 hash 的 key。
12. test_connection 掩盖失败 → version() 失败如实报 success=false。

经判断保留为「有意差异」（非缺陷，见 backend-rs/README）：
- 数组/bytea 文本表示与 Python repr 不同（text 字段、信息性，PG 文本更通用）。
- 大整数 numeric>i64 经 f64（Feishu number 本就 f64 存储，无额外损失）。
- verify-ca 暂同 verify-full（偏严 errs safe）。

### Rust 实现关键决策（备忘）
- 值解码：simple_query 文本协议（非逐类型 FromSql），列类型从元数据查询取，handler 按字段类型格式化文本。
- SQL 安全：连接级 `default_transaction_read_only=on` + `statement_timeout` + 标识符白名单/quoting(转义") + 自定义 SQL 黑名单，四层纵深。
- pool key 含密码/证书 hash，日志脱敏。
- 已知差异见 backend-rs/README「已知与 Python 的差异」。

---

## 2026-06-30 项目改名：pg2bitable → FBIF DataBridge（数据桥）

### 状态
- [x] 候选命名 + 用户选定 DataBridge
- [x] 扫描 pg2bitable / 项目名引用范围
- [x] A 层展示层改名（README / CLAUDE.md / frontend package.json 已改）
- [ ] B 层部署标识迁移（待决策，暂不动）
- [ ] 目录名是否改（待决策）

---

## 2026-06-30 前端重构：落地 claude.ai「1B」设计稿到真实 React 前端

### 来源与决策
- 用户给定 claude.ai design 稿 `FBIF DataBridge - 1B.dc.html`（单色 / Geist / 发丝边框 / 大留白，Codex/Notion 风）。
- **用户明确选择产出落点 = 实现进真实 React 前端 `frontend/`**（而非打磨独立 HTML）。
- 设计稿源码经 RPC 流式下发、跨域沙箱渲染，浏览器无法稳定抠源码 + 原型交互在远程操控下不稳；
  已通过 Present 模式看清 idle 态全貌 + 设计自述流程，未截到的状态（连接态/表搜索/同步）按既定视觉语言 + 自身判断设计（用户授权「结合你的判断」）。

### 设计稿要点（目标）
- 顶部极简头：`数据同步`（左）· `重置`（右）。
- 单卡片「连接数据源 / 把数据库同步到多维表格」：
  1. **数据源类型**选择器：`Pg PostgreSQL`（激活）；MySQL/MongoDB/SQL Server/SQLite/ClickHouse 标「即将支持」——预留多数据源，加库=列表加一行，无需重设计。
  2. **连接串**（URI）= 主输入：`postgres://用户:密码@主机:5432/库名`。
  3. **「不知道连接信息？让 AI 帮你填」** 安静入口行（带灰色 ✨ 字形）——本地 agent 集成思路落地。
  4. 黑色 **连接** 按钮。
- 连接成功 → 连接态（如 `PostgreSQL · analytics`）→ 可搜索**单表**下拉（自述 112 张表规模）→ **同步**。

### 关键架构判断
1. **多表批量 → 单表选择**（重大且正确的方向）：核实过的协议硬事实「一配置=一表」+ brief §7 推荐 (a)。
   废弃 `BatchTableSelector` 的多选误导 UI，改为单表可搜索下拉。
2. **保留不动（数据契约/集成层）**：`types/index.ts` 的 `DatasourceConfig` 字段名、`hooks/useBitable.ts`（SDK）、`api/helper.ts`、`api/client.ts`。
3. **重写**：`App.tsx` 流程、连接表单、表选择、新增数据源类型选择器 + AI 帮填、`global.css`。`useConfig` 适配单表。
4. **组件库取舍**：保留 Semi UI 承载行为（Select 搜索 / Input / Button / **暗色主题现成**），叠加单色设计层（CSS）还原 Geist/单色观感；gsap 做动效。
   理由：飞书同源、暗色现成、不重造可访问下拉/搜索（规则十一 + 规则二）。
5. **AI 帮填的优雅落地（DESIGN.md §3 归一）**：点开 → 复制一段固定提示词（命令 agent 产出**只读角色**的 `postgresql://` URI）→ 粘回**同一个连接串框**。
   不新增独立粘贴框——URI 框本身就是粘贴目标，0 新概念。密码走只读角色，低风险（与后端 §5.3 只读角色建议咬合）。
6. **不弃用无 URI 用户**（Rams #2 有用 + 不删既有能力）：默认 URI-first，保留一个安静的「手动填写」渐进展开（host/port/user/pwd/SSL），折叠默认。
   注：项目历史在「URI-first + 手动兜底」与「直接铺开手动字段」间反复过（commit d7b6ac9 / 7083bdb），本轮按 1B 设计取 URI-first + 渐进展开。

### 飞书 iframe 约束（硬）
- 宽 420–840、高 226–606。卡片 = 全宽（max ~620），高度按内容动态 `setHostContainerDetailSize`。
- 暗色 + 亮色双主题必须过（Semi `theme-mode` body 属性，useBitable 已接 getTheme/onThemeChange）。

### 验证策略（CLAUDE.md UI 验证协议，强制）
- 跑 vite dev，用 claude-in-chrome 开 localhost 真机看。
- 后端 helper 依赖的状态（连接/列表）：加 DEV-only mock（env 守卫，不污染生产）驱动全流程。
- 全状态逐个扫：idle / 连接中 / 连接失败 / 空表 / 暗色 / 最小高度不溢出 / AI 帮填展开 / 各分辨率。
- 「build / tsc 通过」≠ 完成；眼睛在浏览器看过才算。

### 验收原则
- Dieter Rams 十条逐条自检（自解释 #4 / less but better #10 / 诚实 #6）+ DESIGN.md。
- 图标一律开源库（Lucide），不手搓 SVG（DESIGN.md §5）。
- 对抗性审查（codex）后据结果 + 自身判断修，真机复验。

### 状态
- [x] 看清设计稿 + 摸清现有前端代码（活跃 9 文件 + 死代码 4 文件）
- [x] 方案落 WORKLOG（本节）
- [x] 依赖：lucide-react + gsap + @fontsource/geist-sans + geist-mono
- [x] 设计层 CSS（单色 token / Geist / 发丝边框 / 留白 / 亮暗双主题）
- [x] 数据源类型选择器组件（自建小下拉，PostgreSQL 激活 + 5 个即将支持）
- [x] 连接表单 ConnectStep（URI-first + AI 帮填 + 手动渐进展开，URI↔字段双向同步）
- [x] 单表可搜索选择 TableStep（Semi Select filter，112 表规模 + 自定义选项行）
- [x] App 流程（连接↔选表双相）+ 动态高度（ResizeObserver→飞书，夹 226–606）+ gsap 动效
- [x] DEV mock + vite dev 真机全状态验证（详见下）
- [x] 清理死代码（删 ConnectionForm/BatchTableSelector/StepIndicator/FieldConfig/SyncSettings/CustomSQL/TableSelector）
- [x] codex 对抗性审查（13 项）+ 全部采纳修复 + 真机复验

### Codex 对抗性审查处置（13 项全修，真机复验）
**CRITICAL**
1. buildConfig 可空字段 undefined 被 JSON.stringify 丢弃 → 全部 `?? null`。
2. saveConfig 的 savedRef 在 await 前置真，失败后重试静默失效 → 改 savingRef，仅成功保持锁、失败释放。
**HIGH**
3. 换连接后旧 selectedTable 可被保存 → handleConnect 清 selectedTable + 同步按钮 loadingTables 期间禁用。
4. 回填后 URI 框空连不上 → ConnectStep 加 uriDirtyRef，未手动编辑时 connection 变化镜像进 URI 框。
5. URI 改成无效串仍可用旧 connection 连 → canConnect 改由「当前 URI 能否解析」决定，连接时传解析后的 connection 作权威。
6. URI 解析不支持 IPv6/特殊字符 → 重写 connectionUri，借 `new URL()`（postgres→http scheme）正确处理编码/IPv6/query；buildUri 正确编码 + IPv6 加方括号。
7. AI 提示词把密码交第三方 → 提示词强化只读角色 + 加 `<PASSWORD>` 占位符选项；UI 加隐私提示行。
**MEDIUM**
8. mock 可进生产 → `MOCK_ENABLED = import.meta.env.DEV && ...`（生产摇树移除）。
9. 连接中输入可改致失配 → 连接中禁用 URI 输入/手动展开。
10. gsap 中途隐藏冻结 → 加 visibilitychange，隐藏时 killTweensOf + clearProps 快照到终态。
11. 异步回填覆盖用户输入 → hydratedRef 只回填一次 + cancelled 守卫。
**LOW**
12. 自建下拉缺键盘 → SourceTypePicker 加 ArrowUp/Down/Enter/Esc + roving 高亮 + aria。
13. 死导出 → 删 listDatabases/listSchemas/listColumns/previewSQL + 死类型（BatchSyncItem/SQLPreviewResult/ColumnInfo/ConnectionDiagnostics/BITABLE_TYPE_LABELS/StepKey）。

复验：`tsc -b && vite build` 绿；真机连接流程（含 IPv6 风格编码密码 URI `p%40ss`）→ 选表步「PostgreSQL · analytics · 112 张表」正确；selectedTable 随新连接清空。

### 结论
设计稿「1B」已落地为真实 React 前端（飞书 iframe 约束内），单色/Geist/Lucide/gsap，全状态真机验证 + 对抗审查闭环。
保留待用户定的取舍：URI 框明文密码显示（DB 连接串本质含密码，与旧版一致；可选连接后掩码）。
DEV-only：VITE_MOCK + `?mock=fail|empty` 状态测试开关（生产摇树移除）。

### 真机验证（claude-in-chrome，全部眼睛看过）
逐状态验证通过：idle（亮/暗）/ 数据源下拉（6 项，5 个「即将支持」）/ AI 帮填展开（3 步 + 复制提示词）/
URI 连接→选表 / 连接态横幅（PostgreSQL · analytics + 版本/表数，0 表时省略计数）/ 表搜索（112 张，过滤正确，自定义行：图标+名+行数+视图标）/
选中→同步按钮激活（近黑）/ 暗色（按钮反相浅色）/ 手动字段展开 + URI↔字段双向同步预填 / 错误态（陶土色横幅）/ 空表态（Database 图标）/
响应式 420×606 + 620×606（iframe harness，等同飞书渲染，无溢出、媒体查询生效）。

### 验证中发现并修复
1. 数据源「PostgreSQL」文字居中（button 默认 text-align:center 被 flex 子项继承）→ `.db-src__name { text-align:left }`。
2. Semi 下拉内搜索框蓝色聚焦环破坏单色 → `.semi-select-popover .semi-input-wrapper` 单色覆盖。
3. **gsap 健壮性（重要）**：标签页隐藏时 gsap rAF 冻结，挂载即隐藏会把卡片卡在低透明度不自愈 →
   入场/相位动画加 `canAnimate()` 守卫（document.hidden / prefers-reduced-motion 时跳过，留 CSS 默认 opacity:1）+ `clearProps` + kill 清理。已复验：hidden=true 时 cardOpacity=1。
4. ResizeObserver 去抖：高度未变不调用 SDK，避免抖动/循环。

### 自审（Rams 十条 + DESIGN.md）结论
- 减法：单卡片替代 2 步多表批量（去掉误导多选）；加的 AI 帮填（本地 agent 集成、惊喜感）+ 手动渐进展开（不弃用无 URI 用户）均折叠默认、可辩护。
- 自解释 #4：「不知道连接信息？让 AI 帮你填」直接化解非技术用户的「懵」。
- 已知取舍（待用户定）：URI 框显示明文密码（与旧版一致，DB 连接串本质含密码；可改为连接后掩码，但破坏可编辑性）。
