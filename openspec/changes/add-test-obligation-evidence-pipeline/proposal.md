# TestGuard 测试义务与证据管线增量提案

## Why

检查源基线 `b4e8ed05b985f6233d671c76f1132aaa4d99a507` 仅有两份 README 和两份设计文档；当前增加的共享集成文档仍是草案。仓库没有运行时、manifest、测试夹具、CLI、CI 或历史 OpenSpec change 可复用。本变更把设计转换成未来实施要求，不把新增 OpenSpec 文档计为能力完成。测试命令退出零无法证明必需义务、环境或负例已执行；需要冻结计划、实采报告和候选绑定的完整证据。

## What Changes

- 增加领域专用 TestObligation/TestPlan/Execution/Coverage 的版本化模型、CLI 合同和受保护基线输入。
- 分阶段实现 JUnit/Maven/Gradle 与固定版本 Cargo 的独立报告适配，然后增加 Vitest/Jest、Playwright 和覆盖/变异来源；每种适配器先通过真实负例矩阵。
- 增加冻结的测试×环境矩阵、需求/验收/不变量追踪、分开的覆盖分母与测试弱化复核入口。
- 增加隔离 Runner、限额/取消、flaky 全历史、quarantine 时效、证据保留和并发隔离。
- 在外部依赖门通过后增加兼容现有严格引擎 schema 的领域事实映射及独立草案信封，支持可信控制器验真、过期/撤销和精确合并队列候选。
- 最后增量接入只读查询优先的 MCP、CI shadow/opt-in enforcement、独立发行与可回滚适配器。

## Capabilities

### New Capabilities

- `testguard-obligation-planning`: 专业 schema、冻结义务矩阵、追踪覆盖、受保护基线及权限边界。
- `testguard-execution-evidence`: 真实框架适配、隔离/重试/证据、严格引擎投影、错误传输、失效及交付验证。

### Modified Capabilities

无。没有已有 OpenSpec requirement 被替换，也没有本地历史实现任务被宣布完成。

## Impact

拟议新增 `Cargo.toml`、`src/{plan,obligation,runner,report,coverage,policy,cli}.rs`、`src/adapters/`、`schemas/`、`tests/`、`fixtures/`；均尚不存在。本次只新增此 change 文档，不写运行时、不安装依赖、不启用门禁。GuardEngine 继续只拥有通用 Contract/Rule/Evidence；领域解析、测试充分性、flaky 与断言变化政策留在 TestGuard。TestGuard 不签发身份/审批，不持有合并或签名凭据，不发起 Git 合并。

## Dependency Gates

- 原生发现、schema 原型及 JUnit/Cargo fixtures 可与引擎并行开始。
- `TG-EVIDENCE` 的需求计划生产者依赖 `SG-BASELINE`（SpecGuard groups 1–2）及 `GE-CONTRACT`/`GE-ADAPTER`；只有来源明确的试验夹具可在此前使用，不伪装真实互操作。
- 真实可信 CI/审批/失效消费额外依赖 `GE-TRUST`；生产独立发行等 `GE-RELEASE`。不等待整个 SpecGuard/GuardEngine 仓库全部完成。
- `END-TO-END` 等待相关 `GG-CANDIDATE` 与 `FG-GATE` 等集成门，包含准确 synthetic queue candidate、两个并行需求、审批到期/撤销、漂移、迟到和回滚。只读 Git 绑定先于组合门，不引入循环依赖。

依赖源码计划：[GuardEngine](https://github.com/full-stack-plugins/guardengine/tree/docs/guard-design-20261009/openspec/changes/add-versioned-guard-integration-contracts)、[SpecGuard](https://github.com/full-stack-plugins/specguard/tree/docs/guard-design-20261009/openspec/changes/add-specification-baseline-analysis)、[GitGuard](https://github.com/full-stack-plugins/gitguard/tree/docs/guard-design-20261009/openspec/changes/add-candidate-bound-git-governance)、[FlowGuard](https://github.com/full-stack-plugins/flowguard/tree/docs/guard-design-20261009/openspec/changes/add-evidence-bound-workflow-gates)。

## References and Acceptance

以[架构](../../../docs/architecture.md)、[技术方案](../../../docs/technical-design.md)、[共享集成契约草案](../../../docs/integration-contract.md)及[跨仓库阶段路线](../../guard-roadmap.md)为约束；[设计](design.md)、[任务](tasks.md)和两个 delta spec 给出验收。T0–T4 全部是未完成未来任务；schema 验收至少 12 例、每适配器至少 8 类真实运行、3 义务×2 环境、10 并行隔离以及逐项失效验证。精确 Cargo 协议、平台、保留期与来源认证提供方尚未选定，必须在相应阶段先记录决策。审批不能改写专家 REQUIRE_APPROVAL 或绕过缺失证据。
