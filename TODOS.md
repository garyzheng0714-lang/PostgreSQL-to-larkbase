# TODOS

## P2 — 前端组件测试
为前端 React 组件添加单元测试（ConnectionForm、BatchTableSelector、StepIndicator）。
多数据源后前端会变得更复杂（数据源选择器、不同 adapter 的配置表单），需要测试防止回归。
配置 Vitest + React Testing Library。
**Depends on:** 多数据源前端重构完成后再写测试更划算。
**Effort:** M → S (CC)

## P2 — 依赖安全扫描
在 CI 中添加依赖安全扫描（pip-audit + npm audit）。
多数据源会引入新依赖（pymysql、pymongo 等），安全漏洞风险增加。
**Depends on:** 无
**Effort:** S → S (CC)

## P3 — 多数据源架构文档
创建 ARCHITECTURE.md，文档化 adapter 抽象层、数据流、如何添加新数据源。
降低新 adapter 的开发门槛，有利于开源社区贡献。
**Depends on:** adapter 层设计完成
**Effort:** S → S (CC)
