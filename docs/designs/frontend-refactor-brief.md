# 前端重构 Brief — FBIF DataBridge 配置页

> 状态：**草稿（第 4 节待用户确认动机/痛点后定稿）**
> 日期：2026-06-30
> 用途：交给设计协作方（Claude design）执行前端重构。发送时请连同第 9 节列出的上下文一起给。
> 关联：`docs/多维表格「数据同步插件」开发者指南/`、`backend-rs/`（后端契约）、`~/.claude/DESIGN.md`

---

## 1. 背景与定位
飞书多维表格「数据同步插件」的配置前端，跑在**飞书 Bitable 的 iframe 弹窗**里。
用户用它配置「PostgreSQL → 多维表格」的同步：填连接 → 选库/表 → 保存并同步。
技术栈：React + TypeScript + Vite，当前用 Semi UI（`@douyinfe/semi-ui`）。

## 2. 不可变硬约束（改坏 = 不能上线）
- **技术栈**：必须 React + TypeScript（浏览器环境 + 飞书 SDK 强制）。
- **飞书 SDK**：必须用 `@lark-base-open/connector-api` 的：
  - `bitable.getConfig()` —— 编辑已有配置时回填；
  - `bitable.saveConfigAndGoNext({ datasourceConfig })` —— 保存并进入字段配置；
  - `bitable.ui.setHostContainerDetailSize({ width, height })` —— 每步调整 iframe 尺寸；
  - `bitable.bridge.getTheme()` + `onThemeChange()` —— **暗色主题必须支持**（当前用 `theme-mode` body 属性切换）。
- **iframe 尺寸**：宽 420–840、高 226–606（飞书 meta.json 限制）。每步用 `setHostContainerDetailSize` 调整。
  当前：连接步 620×340、选表步 620×520。**所有布局必须在该小窗口内不溢出、不出横向滚动**。
- **保存的数据结构字段名不可改**：`saveConfigAndGoNext` 存的是 `JSON.stringify(DatasourceConfig)`，
  字段 **后端逐字段依赖**（见 `frontend/src/types/index.ts`）：
  `host / port / username / password / database / mode / schema_name / table_name /
  selected_fields / custom_sql / field_renames / number_formats / auto_sync /
  ssl_mode / ssl_root_cert / ssl_cert / ssl_key / connect_timeout / query_timeout`。
  **只能动 UI/交互，绝不能改这些字段名或结构。**
- **后端辅助接口契约固定**（只调，不改）：`POST /api/helper/{test_connection, databases, schemas, tables, columns, preview_sql}`，
  请求体 = 连接字段（+ `schema_name` / `table_name` / `sql`）。鉴权：生产需 `X-Helper-Api-Key` 头（fail-closed）。
- **飞书 CDN 缓存**：页面 URL 由后端 meta.json 带时间戳 `?v=` 防缓存；前端构建产物路径不要写死强依赖。

## 3. 现状（重构起点，已核对代码）
- 主流程 **2 步**：
  1. `ConnectionForm`（连接，325 行，当前最大最复杂的组件）→
  2. `BatchTableSelector`（批量选库表，270 行）→ 保存并同步。
- 步骤切换时手动 `resizeContainer(620, 高度)`；步骤指示器 `StepIndicator`。
- **协议硬事实：一次配置 = 一张同步表**（已核对指南：第 32 行「保存并同步后自动创建一张表」、
  第 269/285 行 `table_meta` 返回单个 `tableName` + 单个 `fields` 数组、第 769 行「创建同步表」单数）。
  `saveConfigAndGoNext` 只存一份 `datasourceConfig` → 飞书只建一张表。**无法用一次配置同步多张表**；
  要同步 N 张需把连接器配置 N 次。
- ⚠️ **当前 `BatchTableSelector` 的多选 UI 与协议能力不符**：可勾多张，但只同步第一张，其余靠 `ErrorBanner` 事后提示。
  这是重构要正面解决的体验问题（见第 7 节）。
- **存在死代码（可清理）**：`FieldConfig(253) / SyncSettings(144) / CustomSQL(82) / TableSelector(166)` 已不在主流程引用。
- 目录：`src/components/* + hooks/{useBitable, useConfig} + api/{client, helper} + types/index.ts + styles/global.css`。

## 4. 重构动机与目标 【★ 待用户确认 —— 这是 Brief 的灵魂】
> 以下是从代码里**观察到的候选痛点（假设，非确认目标）**，请用户逐条确认 / 否决 / 补充。
> 设计方：在用户确认前，不要把这些当作既定需求。

候选痛点（待确认）：
- [ ] `ConnectionForm` 325 行、字段多（host/port/库/SSL/超时…），在 340 高的小窗口里可能偏长偏挤。
- [ ] 选表步在 520 高窗口内承载「多库 × 多表」的批量选择，密度/可扫性是否够。
- [ ] 「只能同步 1 张表」的限制目前靠事后文案提示，交互上不够前置/顺滑。
- [ ] 整体视觉是否够「飞书原生连接器」的克制感（vs 通用表单感）。
- [ ] 死代码残留，组件边界需要重新梳理。

用户填写区：
- 我为什么要重构（具体到「我在 ___ 这一步卡了一下」）：______
- 我希望达到的效果：______
- 必须改 / 不动的范围边界：______

## 5. 设计准则（请设计方严格遵循）
- 全局 `~/.claude/DESIGN.md`：用户体验第一；**优雅 = 用最少的新概念达成目的**（能升级已有入口就别新增按钮/模块）；
  Dieter Rams 好设计十条（自解释 #4、less but better #10、诚实 #6）逐条验收。源头是「把自己当用户、撞到卡顿处、再顺平」。
- 飞书风格：走 `feishu-ui-spec`（飞书 Universe Design），克制、统一，避免「AI 生成感」。
- 组件库取舍见第 7 节，请给方案而非默默选。

## 6. 必须覆盖的状态（别只做 happy path）
连接失败 / 连接中（loading）/ 库表为空 / 字段数超 299 / 暗色 + 亮色双主题 / 最小 iframe 高度下不溢出 / SSL 各模式（disable/require/verify-full）的表单呈现。

## 7. 待决策（请设计方给方案 + 取舍理由，别默默拍板）
- **组件库**：保留 Semi UI（与飞书同源、暗色现成）vs 桥接 shadcn（`feishu-ui-spec`）？
- **流程**：维持 2 步 vs 合并为 1 屏（在 iframe 尺寸约束下是否可行）？
- **多表同步（重要，协议锁死「一配置一表」）**：飞书无法一次同步多张表（见第 3 节证据）。三个诚实方向，请设计方权衡给方案：
  - (a) **改成单表选择**：UI 与协议对齐，最简最诚实，去掉误导的多选；
  - (b) **保留多选但显式引导**：勾多张后，明确告知"将逐张创建连接器/需重复 N 次"，把"再来一次"的流程做顺；
  - (c) 维持现状（多选 + 事后提示）——不推荐，体验与能力不符。

## 8. 交付物与验收（硬性）
- 交付：可运行的 React 组件/页面 + `tsc`/build 通过 + **在飞书 iframe 内真机验证**。
- 验收（强制，UI 验证协议）：**在真实飞书多维表格里打开配置页**，亮/暗双主题、最小窗口、第 6 节全状态各扫一遍，
  整页 + 截图自查（对齐/呼吸感/溢出/断行/hover/空态/错误态）。
  **「build 通过」「tsc 通过」不算完成**——眼睛在飞书里看过没问题才算完成。

## 9. 附：必带上下文（发送时一起给）
- `docs/多维表格「数据同步插件」开发者指南/`（协议 + 飞书侧用户表现 + 截图）
- `~/.claude/DESIGN.md`（设计信条）
- `frontend/src/types/index.ts`（数据契约，字段名不可改）
- `backend-rs/README.md`（后端接口契约 + helper 鉴权）
- 现状截图：连接步 + 选表步，亮/暗各一张（**务必附图，别只文字描述**）
```
