# TestGuard 实施设计

## Context

源基线 `b4e8ed05b985f6233d671c76f1132aaa4d99a507` 只有四份文档；本 change 是第一个本地 OpenSpec 增量，没有旧功能代码、测试或待完成 native change 可以宣称复用。共享信封已增加为设计文档，尚无解析实现。本文引用[架构](../../../docs/architecture.md)、[技术方案](../../../docs/technical-design.md)、[共享集成契约](../../../docs/integration-contract.md)及[路线](../../guard-roadmap.md)；以下路径/类型/接口全部拟议。

## Goals / Non-Goals

目标：让必需义务→测试×环境→每轮观察→原始证据→域内判定可追踪、可复核，无法通过零测试、缩分母、丢弃失败或替换候选报告放行。先交付独立适配器，再组合已稳定的外部契约。

非目标：证明程序完全正确、检测任意语义断言弱化、在引擎内实现领域 parser、颁发身份或合并授权、替代原生框架、自动执行不可信政策脚本。没有外部旧插件已验证兼容；本地无旧 CLI 需迁移，但不得修改其他 Guard 的 native 命令或退出码。

## Decisions

### 1. 分离模型与版本

专业 `TestObligation`/`TestPlan`/`ExecutionRecord`/`CoverageEvidence`/`DomainFinding` 的 schema 版本独立于 crate semver、策略 revision、引擎版本及集成信封版本。现有 `guard.partme.ai/v1alpha1` 严格字段与 exact `forbid_relation` 不变；`guard.integration/v1alpha1` 是独立草案，GE-CONTRACT 冻结前只提供明确标为实验的 fixture，不宣传已支持。拒绝未知字段、未知版本和未知 capability；无隐式 N/N-1 兼容。

专业原始工件记录可以含大小、摘要算法等字段；信封内引用只使用冻结 schema 定义的字段，不能把专业字段塞进引擎/信封。采用确定性归一化，时间戳不代替不可变绑定；测试运行本身不保证确定。

### 2. 模块与接口

| 拟议文件 | 输入 → 输出 | 边界 |
|---|---|---|
| `src/obligation.rs` | 已批准 SG TestObligation export + invariant refs → `ObligationSet` | 校验 ID/修订/引用，不自行批准需求 |
| `src/plan.rs` | `ObligationSet` + `InvocationBinding` + `ProtectedPolicy` → `FrozenPlan` | 执行前冻结集合、环境、限额、argv、工件 |
| `src/adapters/*.rs` | `RawArtifactSet` + `ExecutorProfile` → `Vec<CaseObservation>` 或解析错误 | strict 格式/版本，保留 native ID、参数和环境 |
| `src/runner.rs` | `FrozenPlan` + `ExecutionPermission` → `AttemptRecord` | 固定 argv，隔离、限额、取消，不签发凭据 |
| `src/coverage.rs` | `FrozenPlan` + observations + coverage reports → `CoverageEvidence` | 按指标单独分母、缺口与过滤范围 |
| `src/policy.rs` | 上述领域数据 + protected policy → `DomainAssessment` | flaky、缺失、弱化 findings 属于本域 |
| `src/report.rs` | `DomainAssessment` + immutable artifacts → engine inputs + `AttemptOutput` | schema-safe 投影；decision 与引用 report 相等 |
| `src/cli.rs` | 参数/文件 → transport output | check 0/2/3/4；stdout JSON/stderr 诊断；绑定前错误无信封 |
| `src/integration/{trust,ci,mcp}.rs` | 外部认证引用/调用 → scoped evidence 查询或受权测试 | GE-TRUST 之后才可信消费；不执行合并 |

函数名和序列化细节由 T0 schema/接口测试冻结，上表的边界类型不暗示已存在 Rust struct。`AttemptOutput` 区分 `PreBindingDiagnostic` 与 `BoundAttemptEnvelope`，不得用空 OID 填补绑定错误。`run` 执行 untrusted code，`doctor/coverage` 只读。

### 3. 计划与覆盖

冻结 `R={(obligationId,testId,environmentId)}`，发现集合不能缩减 R。稳定 test ID 关联 native suite/case/parameters/environment，碰撞拒绝。`missing=R−observed`；skip、unknown、failure 另列，额外通过测试不能填补缺项。只有全部必需环境满足批准策略的义务可计满足分子。statement/branch/requirement/contract/state/concurrency/mutation 分开，空分母 N/A；代码行覆盖不能填需求覆盖。

SG-BASELINE 提供稳定 ID、源图、批准基线和 TestObligation 导出。ArchGuard 不变量可通过明确引用补充测试义务，基本 runner 不等待其全部实现。弱化检测以删除/配置/阈值差异和已知变异测试为基础，任意断言语义变化需要人工复核，不保证自动完备。

### 4. 生命周期、协议与错误

计划从 draft/validated/frozen 到 prepared/running/collecting/assessing；尝试终态为 completed/error/cancelled，资格 stale 是外部派生状态，不改历史信封。有效但 partial 的领域事实→引擎 BLOCK/INDETERMINATE；工具/输入/解析/核验错误不是伪造的引擎报告。

解析参数、仓库或候选失败，或 producer/必查覆盖未冻结：独立诊断、退出 4、无信封。绑定与 producer/coverage 冻结后 error/cancelled 信封 decision=null，保留已知观察，目标退出 4；信号强制杀死可能无落盘输出。completed 引擎支持 profile 必须提供 contract/facts/report refs，decision 等于 report，0/2/3 分别为 ALLOW/BLOCK/REQUIRE_APPROVAL。`verify` 重算，不证明来源可信。

enforce/review/advise 保持引擎语义；必要覆盖缺口不能靠审批消除。外部审批记录不能改写 TestGuard REQUIRE_APPROVAL；FlowGuard 可在自己的作用域求新 gate report，历史专家报告不变。

### 5. 隔离与证据

测试默认无网络、生产凭据或宿主写权限，依赖预置；控制器认证与签名密钥远离候选。固定 argv，不 shell 拼接；限制 CPU/内存/进程/时间/输出/工件；XML 禁外部实体、拒绝路径/符号链接出界。隔离 worktree/端口/数据库/每轮目录；可信 runner 在候选进程外采集摘要，候选写出的 XML 本身不可信。

每轮追加保存，fail→pass 仍 flaky，默认诊断重试不覆盖 mandatory failure。quarantine 必须 owner/理由/期限/审批/替代保障，不能自动删分母。证据以受限 URI 与 digest 引用，审计记录含执行身份、候选/基线/规则、因果尝试、工具、审批与结果；日志脱敏，保留期至少覆盖消费期，必要工件缺失须重跑。

缓存默认不跨 run 复用执行结果；未来必须绑定 repo/task/requirement、candidate/base/merge group、source digest、baseline、policy、analyzer/toolchain/env/config/coverage。审批到期/撤销及全部绑定变化使资格失效。幂等键区分等价请求与 attempt；迟到旧结果只附原始绑定，不能覆盖当前候选。两并行需求不得串用测试/审批/工件。

### 6. 依赖、发布与回滚

T0–T1 本地 schema 原型与 fixtures 可先行；T2 requirement-based producer 等 SG-BASELINE + GE-CONTRACT/ADAPTER，T3 可信消费等 GE-TRUST，T4 独立生产发行等 GE-RELEASE。详见[引擎 change](https://github.com/full-stack-plugins/guardengine/tree/docs/guard-design-20261009/openspec/changes/add-versioned-guard-integration-contracts)和[SpecGuard change](https://github.com/full-stack-plugins/specguard/tree/docs/guard-design-20261009/openspec/changes/add-specification-baseline-analysis)。可用 pinned source 开发，不把外部门依赖变为全仓库阻塞。

先 advisory capture，再 shadow 比较，最后显式 opt-in protected checks。不兼容时禁用新适配器/消费者并恢复已批准配置，保留不可变历史证据和原生测试入口；未知证据仍 fail closed，回滚不能让已要求的门禁静默通过。END-TO-END 使用准确 synthetic candidate、两个需求、审批过期/撤销、漂移、迟到、原生 CodeGuard parity 与回滚，由跨仓库集成夹具证明而非 TestGuard 重实现 CodeGuard。

## Risks / Open Questions

| 决策 | 可逆默认 | 必须解决的验收任务 |
|---|---|---|
| Cargo 报告协议/支持版本、JUnit 参数化身份 | 固定经过真实 fixture 验证版本，未知拒绝 | 1.1、2.1–2.3 |
| Rust/dependency 版本与 SDK profile | 首版单 crate、pinned 源，未选择版本不宣称安装 | 1.1、4.1 |
| 沙箱平台/限额/终止预算 | 单一受控 CI 平台、默认禁网络、预算显式配置 | 3.1–3.2 |
| 签名/批准 provider、时钟/存储信任 | 可信外部控制器，报告本身 unsigned | 4.4 |
| retention/隐私和跨 run 缓存 | 显式部署配置、缺工件重跑、禁结果复用 | 3.4、4.5 |
| 任意断言语义弱化 | 差异与已知 mutation + 必要复核，不做完备声明 | 2.5、5.2 |

## Validation Strategy

每个任务先准备可复现拒绝/接受夹具再实现，并记录实际输入、版本、命令、输出/退出与 artifact digest。T0 至少 12 schema 正反例及 10 次同输入归一化；T1 每适配器 8 类真实运行；T2 3 义务×2 环境逐项缺失；T3 10 并行隔离与取消预算；T4 逐类绑定失效及合并队列演练。文档语法校验不是这些运行验收。详细 requirement→task 对应见 [tasks](tasks.md)，全部未勾选。
