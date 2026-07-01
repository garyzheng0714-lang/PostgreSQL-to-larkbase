# 飞书连接器配置弹窗 UI 审计

日期：2026-07-01

## 覆盖状态

- 选表状态：mock 连接成功，表列表加载完成，未选择表，主按钮禁用。
- 宽度覆盖：410px、520px。
- 未覆盖：真实飞书宿主最终截图、下拉展开态、连接失败态。

## 对照规则

- Feishu Universe Design 布局：白底灰阶、轻描边、紧凑表单；主操作使用黑底白字，避免蓝/红/绿状态色干扰。
- Feishu 圆角：按钮/输入/选择器使用 6px，轻量信息组使用 6px，去掉 16px 大圆角。
- Feishu 对话框：内容区不再使用有投影的大卡片，保留标题区、内容区、操作区分层。
- Feishu 选择器：触发器高度 32px，单行文本，右侧图标不裁切。
- 侧边栏约束：410px 和 520px 下无横向溢出，按钮完整可见。

## 修复内容

- 将主题 token 收敛为白底灰阶；主按钮、连接状态和错误提示都不再使用蓝/红/绿功能色。
- `.db-card` 改为平面内容容器，移除边框、阴影和大 padding。
- 连接信息改为两行布局，避免 PostgreSQL、数据库名、版本、表数和编辑按钮挤在一行。
- 控件高度压回 32px，按钮使用统一本地 Button 包装。
- 移除会造成 8px 横向溢出的负 margin。

## 验证证据

当前提交不引用 `.context/` 下的旧截图；该目录被 git 忽略，截图仅作为本地验收证据，不作为仓库资产提交。

代码级验证：

- 410px：`MIN_FRAME_WIDTH=410`，`frameSize.test.ts` 覆盖 `300 -> 410` 与 `410 -> 410`。
- 520px：`INITIAL_FRAME_WIDTH=520`，`frameSize.test.ts` 覆盖初始宽度。
- 主题：`global.test.ts` 覆盖白底、黑灰主操作色，并防止旧蓝/红/绿 token 回流。
- 本地 Playwright smoke：`http://127.0.0.1:5174/?extension_market_spread_width=410&mock=1` 与 `520` 均无 console error、无 4xx 资源、`scrollWidth == viewport width`、无越界元素；页面背景为 `rgb(255, 255, 255)`，采样色板仅包含黑白灰。
- 待补：真实飞书宿主最终截图、下拉展开态、连接失败态。

## 检查命令

- `npm test`
- `npm run build`
- Playwright smoke：410px / 520px mock 配置页截图与 DOM overflow 检查
- `python3 ~/.claude/skills/feishu-ui-spec/scripts/check-against-spec.py frontend`
