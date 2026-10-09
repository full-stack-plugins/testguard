# TestGuard — 测试守卫

[English](README.md) · [简体中文](README.zh-CN.md)

**TestGuard 旨在验证被要求的行为是否实际接受了测试，而不只是测试命令返回零。**

> **状态：已有实验性本地 Rust 实现。** 当前树包含源码、Cargo 清单、测试、原生报告夹具及可执行 CLI；已独立验收 17/30 项任务。历史检查基线 `b4e8ed05b985f6233d671c76f1132aaa4d99a507` 当时仅含文档，不能代表当前树。下述端到端隔离执行与可信生产集成仍是目标，非已交付能力。详见[实施记录](docs/implementation-progress.md)与[实际 CLI 契约](docs/local-cli.md)。

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

当前原生 profile 覆盖固定 Cargo1.99.0 以及 Maven3.9.9/Surefire3.5.2、Gradle8.14.3/JUnit4.13.2 夹具（[原生矩阵](fixtures/junit/matrix/README.md)）；任务2.1待独立复核。Vitest/Jest、Playwright、覆盖/变异工具、MCP 与 CI 集成须随后逐项验收。仓库没有已验证的外部历史插件，其兼容性仅为待验证目标。

## 当前本地 CLI

```sh
cargo build --locked --bin testguard
testguard doctor
testguard plan OBLIGATIONS.json BINDING.json
testguard check PLAN.json ATTEMPT.json [CHANGES.json]
testguard coverage PLAN.json ATTEMPT.json
testguard verify PLAN.json ATTEMPT.json REPORT.json [CHANGES.json]
testguard check-engine PLAN.json INVOCATION.json ATTEMPT.json [CHANGES.json [ADVICE.json]]
testguard verify-engine PLAN.json BUNDLE.json
```

这些命令使用显式本地输入，不自动把 HEAD 解析为已认证候选。`check`/`check-engine` 的本轮决定对应 0/2/3，输入或核验错误为 4；`coverage` 输出分母和缺口，不代表门禁通过。`check-engine` 绑定后错误使用 null decision 信封，`verify-engine` 重算真实 GE 工件及领域投影；两者不认证执行者、审批或 Git 对象。精确参数和输出见[CLI 契约](docs/local-cli.md)。

主 CLI 的 `run` 始终拒绝并返回 4。独立的 [Linux 固定夹具生命周期 helper](docs/decisions/testguard-local-process-profile.md) 只验证受限固定进程树的超时/取消/输出上限与回收；它不是原生测试执行入口、OS 沙箱或生产授权。受限[工件采集器](docs/decisions/testguard-artifact-collection.md)与[本地证据存储](docs/decisions/testguard-evidence-store.md)为显式库边界，不会自动给所有 CLI 输入加上隔离能力。

当前可重跑 `cargo test --locked`。已验收生命周期检查为 85 项测试；对应源码证据见[进程测试](tests/process_lifecycle.rs)、[工件安全测试](tests/artifact_security.rs)及[引擎映射测试](tests/engine_mapping.rs)。最新计数与复核提交记录在[实施记录](docs/implementation-progress.md)，不宣称重新运行过所有历史外部工具。

## 信任与协议

计划和门槛必须来自受保护基线，并绑定不可变摘要及经认证的审批记录。目标生产执行器将运行不可信候选代码：隔离 worktree、进程、网络、资源与凭据，默认禁止外部副作用。保留全部重试、隔离决定与工件引用。candidate/base/merge group、规则、分析器/覆盖范围、基线或审批有效性变化会使相关证据失效；必须验收准确的合并队列候选。

现有 `guard.partme.ai/v1alpha1` 仅提供 GuardContract YAML、GuardFacts JSON、GuardReport JSON 及精确 `forbid_relation` 规则。领域 schema 与可选本地引擎 profile 使用的独立版本[集成信封](docs/integration-contract.md)独立，不是可以添加到当前严格协议的扩展字段。报告未签名，重算不等于认证。

## 设计与交付

- [总体架构、边界、信任及证据](docs/architecture.md)
- [技术设计、接口及可测量的阶段验收](docs/technical-design.md)
- [共享集成契约（草案）](docs/integration-contract.md)
- [外部 GuardEngine 协议参考](https://github.com/full-stack-plugins/guardengine/blob/main/docs/protocol.md)

T0–T4 从版本化模型和缺工具诊断，依次交付真实 JUnit/Cargo 夹具、冻结覆盖矩阵、隔离执行/flaky 恢复，最终扩展框架及可信集成。每期必须提供可复现正反例证据；文档不计实现里程碑。新增框架适配、生产沙箱隔离及部署保留策略仍需独立决定。


## OpenSpec 实施待办

新增增量 [proposal](openspec/changes/add-test-obligation-evidence-pipeline/proposal.md)、[design](openspec/changes/add-test-obligation-evidence-pipeline/design.md)、[规范](openspec/changes/add-test-obligation-evidence-pipeline/specs/) 与 [tasks](openspec/changes/add-test-obligation-evidence-pipeline/tasks.md)，将架构方案拆成待实施工作。参阅[跨仓依赖路线图](openspec/guard-roadmap.md)与[结构验证记录](openspec/validation-2026-10-09.md)。任务表目前记录 17/30 项独立接受；新实现只在独立复核通过后勾选。历史源码清单和结构校验记录只描述当时基线。当前可执行能力、测试证据与剩余限制以实施记录和各 profile 文档为准；结构校验不等于运行时验收。
