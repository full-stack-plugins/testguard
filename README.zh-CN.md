# TestGuard — 测试守卫

[English](README.md) · [简体中文](README.zh-CN.md)

**TestGuard 旨在验证被要求的行为是否实际接受了测试，而不只是测试命令返回零。**

> **状态：仅有设计文档。** 2026-10-09 检查基线 `b4e8ed05b985f6233d671c76f1132aaa4d99a507`，跟踪树只有下列两份 README 和两份设计文档，没有应用源码、包清单、测试、夹具、CI、OpenSpec、CLI、MCP 服务或发布。下文能力与命令全部为规划，并非已验证的可执行功能；不宣称运行过实现测试或 OpenSpec 校验。

## 目标流程

执行之前，受保护且已批准的测试计划将 SpecGuard 验收义务与 ArchGuard 领域不变量映射到必需测试身份及环境。原生框架适配器采集结果、覆盖与执行来源，TestGuard 判断充分性；GuardEngine 提供通用合同校验、中性规则求值和确定性证据计算。

~~~text
批准的需求 + 不变量 + 不可变 candidate/base
                ↓
冻结计划 + 必需测试 × 环境 + 覆盖分母
                ↓
隔离的原生 Runner → 原始报告 + 全部尝试 + 来源
                ↓
TestGuard 充分性 finding → 符合现有 schema 的 GuardEngine facts
                ↓
限定范围的技术决定 → 可信 CI / FlowGuard / GitGuard
~~~

典型场景包括验证需求的负例确实执行、跨环境回归、识别全绿但不完整的测试集合，以及保留 flaky 失败历史。零用例、必需用例跳过、报告缺失、候选过期或未经批准的测试削减均不能通过。行覆盖不证明需求满足；重试成功不抹掉先前失败。任意断言语义弱化的自动检测仍有研究与人工复核限制。

## 输入、输出与边界

- **输入：** 不可变的已批准义务/计划引用、受保护规则合同、candidate/base/merge-group 身份、执行器/工具链/环境清单、发现的测试身份及原生报告。
- **规划输出：** 冻结 `TestPlan`、每轮执行记录、原始工件摘要引用、需求/测试/环境追踪矩阵、分开的覆盖指标、诊断 finding 与限定范围决定；明确列出缺失证据。
- **本域责任：** 测试计划充分性、Runner 适配器、实际执行、覆盖分母、回归/flaky 证据与来源检查。
- **其他产品：** SpecGuard 定义需求含义；ArchGuard 定义架构不变量；CodeGuard 负责静态质量；GitGuard 负责候选/分支安全；FlowGuard 负责生命周期条件。六个守卫独立构建于 GuardEngine 之上。TestGuard 不授予合并/发布权，也不认证自己的审批。

初期目标为带 Maven/Gradle 来源的 JUnit 报告及固定版本 Cargo 适配器；Vitest/Jest、Playwright、覆盖/变异工具、MCP 与 CI 集成须随后逐项验收。仓库没有已验证的外部历史插件，其兼容性仅为待验证目标。

## 规划 CLI — 不可执行

~~~sh
testguard doctor --project .
testguard plan --contract approved.yaml --candidate HEAD
testguard run --plan test-plan.json --format json
testguard check --plan test-plan.json --execution execution.json --format json
testguard verify --plan test-plan.json --report evidence.json
testguard coverage --requirement REQ-017
~~~

`plan` 将在冻结前把 `HEAD` 解析为不可变对象身份。`check` 目标退出码为 **0 ALLOW、2 BLOCK、3 REQUIRE_APPROVAL、4 输入/运行/核验错误**，stdout 为 JSON，stderr 为诊断。这是设计约定而非现有行为；其他子命令及 `--report` 写文件语义尚未实现。不完整分析必须阻断，审批不能覆盖。`verify` 只重算证据，不认证执行者可信性或授权合并。

## 信任与协议

计划和门槛必须来自受保护基线，并绑定不可变摘要及经认证的审批记录。测试会运行不可信候选代码：隔离 worktree、进程、网络、资源与凭据，默认禁止外部副作用。保留全部重试、隔离决定与工件引用。candidate/base/merge group、规则、分析器/覆盖范围、基线或审批有效性变化会使相关证据失效；必须验收准确的合并队列候选。

现有 `guard.partme.ai/v1alpha1` 仅提供 GuardContract YAML、GuardFacts JSON、GuardReport JSON 及精确 `forbid_relation` 规则。领域 schema 与拟议[集成信封](docs/integration-contract.md)独立，不是可以添加到当前严格协议的扩展字段。报告未签名，重算不等于认证。

## 设计与交付

- [总体架构、边界、信任及证据](docs/architecture.md)
- [技术设计、接口及可测量的阶段验收](docs/technical-design.md)
- [共享集成契约（草案）](docs/integration-contract.md)
- [外部 GuardEngine 协议参考](https://github.com/full-stack-plugins/guardengine/blob/main/docs/protocol.md)

T0–T4 从版本化模型和缺工具诊断，依次交付真实 JUnit/Cargo 夹具、冻结覆盖矩阵、隔离执行/flaky 恢复，最终扩展框架及可信集成。每期必须提供可复现正反例证据；文档不计实现里程碑。适配器版本、测试 ID 稳定性、沙箱平台及保留限额仍待确定。


## OpenSpec 实施待办

新增增量 [proposal](openspec/changes/add-test-obligation-evidence-pipeline/proposal.md)、[design](openspec/changes/add-test-obligation-evidence-pipeline/design.md)、[规范](openspec/changes/add-test-obligation-evidence-pipeline/specs/) 与 [tasks](openspec/changes/add-test-obligation-evidence-pipeline/tasks.md)，将架构方案拆成待实施工作。参阅[跨仓依赖路线图](openspec/guard-roadmap.md)与[结构验证记录](openspec/validation-2026-10-09.md)。所有新增实施任务保持未勾选；本分支新增规划，不新增产品功能。前文源码树清单和验证限制对应检查基线或较早的架构审阅阶段；本次另行新增 OpenSpec 文档并记录实际 CLI 校验。既有 change 的任务归属和历史完成证据继续保留。
