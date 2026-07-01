# 飞书连接器配置弹窗 UI 审计

日期：2026-07-01

## 覆盖状态

- 选表状态：mock 连接成功，表列表加载完成，未选择表，主按钮禁用。
- 宽度覆盖：410px、520px。
- 未覆盖：真实飞书宿主最终截图、下拉展开态、连接失败态。

## 对照规则

- Feishu Universe Design 色彩：主操作使用品牌色 `#1456F0`，文本使用 `#1F2329/#646A73/#8F959E` 灰阶。
- Feishu 圆角：按钮/输入/选择器使用 6px，轻量信息组使用 6px，去掉 16px 大圆角。
- Feishu 对话框：内容区不再使用有投影的大卡片，保留标题区、内容区、操作区分层。
- Feishu 选择器：触发器高度 32px，单行文本，右侧图标不裁切。
- 侧边栏约束：410px 和 520px 下无横向溢出，按钮完整可见。

## 修复内容

- 将黑白极简 token 改为飞书官方品牌色和灰阶。
- `.db-card` 改为平面内容容器，移除边框、阴影和大 padding。
- 连接信息改为两行布局，避免 PostgreSQL、数据库名、版本、表数和编辑按钮挤在一行。
- 控件高度压回 32px，按钮使用统一本地 Button 包装。
- 移除会造成 8px 横向溢出的负 margin。

## 验证证据

- `.context/ui-audits/feishu-config-table-410.png`
- `.context/ui-audits/feishu-config-table-520.png`

布局指标：

- 410px：`scrollWidth=410`，主按钮 `rgb(20, 86, 240)`，选择器 `32px`，配置容器无边框/无阴影。
- 520px：`scrollWidth=520`，主按钮 `rgb(20, 86, 240)`，选择器 `32px`，配置容器无边框/无阴影。

## 检查命令

- `npm test`
- `npm run build`
- `python3 ~/.claude/skills/feishu-ui-spec/scripts/check-against-spec.py frontend`
