# TestGuard — 总体架构

> 设计基线 V0.2，2026-10-09。所有组件、数据模型、流程及验收均为目标，不是实现声明。

## 1. 事实基线与阅读范围

检查 main 提交 `b4e8ed05b985f6233d671c76f1132aaa4d99a507` 的完整跟踪树并阅读全部内容：[`README.md`](../README.md)、[`README.zh-CN.md`](../README.zh-CN.md)、本文件和 [`technical-design.md`](technical-design.md)。没有 `src/`、manifest、测试、fixture、CI、OpenSpec 或可执行 CLI；没有仓库内 AGENTS.md/skills。现有文档描述的是愿景，不能作为功能已实现的证据。下列模块路径都是建议创建的路径。外部框架、历史插件及 SDK 的实际兼容性尚未检查；共享协议约束来自本次跨仓库设计基线，不声称本仓库实现了它。

## 2. 定位与责任

TestGuard 要回答：针对已批准需求与行为契约，哪些测试在什么环境下必须执行，实际上执行了什么，证据是否足以支持**当前精确候选**？它覆盖计划充分性、执行、行为/契约/回归观察、覆盖和来源；不能通过少量容易通过的测试、自行缩小分母或上传最后一轮绿色报告宣布验收。

| 产品 | 拥有的语义 | TestGuard 的协作输入/输出（目标） |
|---|---|---|
| SpecGuard | 需求与验收义务 | 引用 requirement/acceptance ID、批准修订与负例要求 |
| ArchGuard | 领域不变量、状态/边界约束 | 引用 invariant ID、转换及并发测试义务 |
| CodeGuard | 静态质量和构建检查 | 引用工具链/构建前置证据，不把静态通过当执行通过 |
| TestGuard | 测试计划充分性及实际执行证据 | 输出追踪矩阵、缺口、执行历史、域内结论 |
| GitGuard | 候选与分支安全 | 使用准确 candidate/base/merge-group 身份；不执行合并 |
| FlowGuard | 生命周期条件与审批流程 | 消费技术决定；可信控制器核验审批身份/范围/有效期 |
| GuardEngine | 通用 Contract 校验、中性 Rule 求值、确定性 Evidence | 接收已归一化的兼容事实，不解析 JUnit、不计算测试充分性、不签发审批 |

六个守卫是独立产品，没有 GuardCore。ALLOW 是分析范围内的技术判断，不是代码完全正确的证明，也不是合并或发布权。

### 典型场景

- REQ-017 要求支付拒绝后不扣款：关联成功、拒绝、超时及重试幂等测试，而不仅检查支付函数行覆盖。
- 领域状态机允许 A→B、禁止 A→C：分别追踪正常转换、非法转换与并发竞争的测试及环境。
- PR 的 12 个必需测试只执行 8 个，全部绿色：仍有 4 个缺口，必须 BLOCK。
- 主分支变化产生新合并队列候选：即使分支头未变也重新执行受影响检查。

## 3. 目标组件和数据流

~~~text
Protected requirement/invariant/plan baselines + immutable Git bindings
                         ↓
Plan compiler → Frozen required test × environment matrix
                         ↓
Discovery/selection → Isolated bounded native runner
                         ↓
Artifact collector → Strict framework adapters → Attempt records
                         ↓
Adequacy/coverage/flaky policies → TestGuard domain findings
                         ↓
Schema-safe facts adapter → GuardEngine → Technical decision
                         ↓
Separate integration envelope → trusted CI / FlowGuard / GitGuard
~~~

**Plan compiler** 冻结 obligation ID、test ID、环境矩阵、执行器版本、固定 argv、超时、预期工件、阈值和批准基线摘要。输入缺失、循环/悬空追踪引用或缺环境映射时不能冻结有效计划。动态发现补充执行清单，不能删掉预先要求的义务。候选可提出新增测试，不能自批删必需测试或降门槛。

**Discovery/selection** 比较必需与发现集合，使用 suite、参数化案例、环境和适配器身份消歧。增量选择必须说明未选择项由何种仍有效证据覆盖；没有证明则运行全部必需项。发现信息本身不是执行证据。

**Runner** 在受控 worktree 内以 argv 执行原生测试，不在 Rust 重写框架。首批针对 JUnit/Maven/Gradle 和固定版本 Cargo，随后 Vitest/Jest、Playwright。按 repo/task/worktree/requirement 与不可变候选隔离目录、端口、数据库命名空间、进程组和缓存；共享可变 fixture 必须锁定或复制。禁止把不可信策略脚本作为受信引擎扩展。

**Collector/adapters** 保存退出状态、测试发现/开始/结束、每次尝试、原生报告摘要和来源。JUnit XML 只是一种报告格式，不自动证明 runner 可信；候选可能伪造文件。可信执行服务应在进程外采集、封存工件并记录执行身份。截断、重复 ID、未知报告格式与退出状态矛盾都必须显式报错或标为不完整。

**Adequacy evaluator** 独立计算集合差、阈值、行为结果和 flaky 状态。领域结果转换为当前引擎可表达的关系事实；详细 TestPlan/Execution/Coverage 保留在独立专业工件内。不得声称引擎能直接理解覆盖百分比。

## 4. 领域模型与追踪

| 目标实体 | 关键含义与约束 |
|---|---|
| TestObligation | 稳定 ID、requirement/acceptance/invariant 引用、criticality、批准修订、必需环境 |
| TestPlan | 不可变 plan digest、受保护 contract/baseline、candidate/base/merge group、必需矩阵、argv、限额、工件和审批引用 |
| TestIdentity | suite + case + 参数身份 + 环境；框架原生 ID 与稳定映射并存；碰撞拒绝 |
| ExecutionRecord | run/attempt ID、执行身份、命令、时间、环境/工具版本、退出/取消原因、原始工件摘要 |
| CaseObservation | discovered/started/finished 独立标记；pass/fail/skip/unknown；来源定位和失败信息 |
| CoverageEvidence | metric kind、分子/分母、适用范围、工具/过滤条件、未映射项、限制 |
| DomainFinding | 稳定 code、义务/测试/环境定位、证据引用、修复建议；不扩展当前 GuardReport |

追踪链为 `requirement@revision → acceptance/invariant → obligation → required test×environment → attempt → raw artifact → finding/decision`。一个测试可覆盖多个义务，但每条映射要由受保护计划明确声明，不能凭命名相似推断语义通过。义务未映射、必需环境未运行、报告 ID 无法对应均保留缺口。输出应同时支持从需求找证据、从失败找受影响需求。

## 5. 覆盖、flaky 与弱化检测

| 指标 | 分母冻结方式 | 不可推出的结论 |
|---|---|---|
| 必需执行覆盖 | 计划中的 test×environment 对 | 执行结束不等于断言成功 |
| 需求/验收覆盖 | 批准范围内必须满足的义务及环境 | 行覆盖不能替代此指标 |
| statement/branch | 固定候选、instrumentation、过滤规则的工具报告 | 高比例不证明语义正确 |
| contract/state/concurrency | 计划枚举的契约、状态转换及交错场景 | 有限场景不证明所有交错 |
| mutation | 固定工具配置下有效 mutant；排除项单列 | 分数不等同需求满足 |

空分母为 N/A 且说明原因，不得表示 100%。一个义务的所有必需案例和环境都成功，且符合批准重试策略，才可计入其满足分子。缺报告或未结束项不能进入通过分子。

Flaky 的默认拟议策略为首次失败后即保留不通过结论，只有经批准、有限次的诊断重试；最终通过记录为 flaky，而非覆盖首次失败。quarantine 有 owner、原因、截止期、批准引用和替代保障；它不会自动删除必需分母。移除义务必须产生新的批准基线。通过测试文件/配置差异检测删除、阈值降低与可疑断言变化；任意语义弱化无法静态保证识别，需基线回归、精选变异负例和独立复核，不能保证“所有弱化均可检出”。

## 6. 生命周期、错误与决策

~~~text
DRAFT → VALIDATED → FROZEN → PREPARED → RUNNING → COLLECTING → ASSESSING
                                                                          ↓
                                                                       COMPLETED
任何阶段 → ERROR 或 CANCELLED；部分执行工件仍保留
任意终态证据在绑定变化后 → STALE（禁止门禁消费）
~~~

生命周期与技术决定分离：completed 不意味着 ALLOW。有效但部分事实为 BLOCK/INDETERMINATE；工具崩溃、不可解析输入/报告和核验失败属于 error，外层 decision 为 null，CI 阻断并使用退出码 4。完整事实中强制规则违规为 BLOCK；只有证据充分但存在明确 review 规则义务时才可 REQUIRE_APPROVAL。审批不能绕过不完整扫描、工具故障或实际强制失败。取消为 cancelled，保留中间结果但不产生通过结论。

## 7. 信任、安全、审计与失效

受保护 baseline 以不可变 digest/ref 引用，审批来源必须由可信控制器核验身份、范围、候选绑定和有效期；`approved: true` 或 Markdown “accepted” 不构成授权。控制端与候选测试进程分离，测试不给生产密钥、合并令牌或签名凭据。默认禁网络和外部写入，依赖预置；确需数据库/网络时显式授予受限临时测试资源。CPU、内存、进程数、时间、输出/工件大小均有限额；取消回收进程树并对未结束用例记 unknown。

保留计划、工具链/环境摘要、每轮尝试、已知失败、原始工件摘要、采集身份、策略版本和审批引用。日志对凭据脱敏，访问按项目隔离；保留期可配置且须覆盖审批/门禁有效期，过期或无法读取的必要工件不能当作仍可核验。重算 hash 只验证一致性；现有报告未签名，不能由此建立来源信任。

candidate/base/merge group、合同规则、分析器版本/覆盖范围、基线修订及审批过期/撤销都会使相关结果失效。可信控制器验证准确 merge-queue candidate。按不可变绑定与 plan/attempt 标识做幂等及并发隔离；旧任务迟到结果只能写自身记录，不能覆盖当前候选状态。

## 8. 协议与扩展边界

现有 `guard.partme.ai/v1alpha1` 只有 GuardContract YAML、GuardFacts JSON、GuardReport JSON，严格拒绝未知字段，规则为精确 `forbid_relation`，enforcement 为 enforce/review/advise，decision 为 ALLOW/BLOCK/REQUIRE_APPROVAL；partial facts 导致 BLOCK/INDETERMINATE。complete 仅指分析器声明范围，并非整个仓库无遗漏。历史命名空间为兼容保留。

共享 [GuardRunEnvelope](integration-contract.md) 使用独立 `guard.integration/v1alpha1` **草案**，现有引擎不解析。它承载 runStatus、失败时 nullable decision、调用绑定、工件摘要、分析器/覆盖、审批引用和诊断，不向现有 GuardReport 塞入错误信封或领域扩展。专业 schema 版本、crate semver 和策略修订各自独立。新适配器先验证版本和 fixture 矩阵，再注册；未知版本拒绝解析，不能猜测为通过。

## 9. ADR 与验收入口

- TG-ADR-001：执行前冻结计划，执行结果不得反推必需义务。
- TG-ADR-002：生命周期、行为结果、覆盖和审批分开记录。
- TG-ADR-003：复用原生框架，优先验证真实报告及负例。
- TG-ADR-004：零用例、全跳过、报告丢失和超时均不能通过。
- TG-ADR-005：测试弱化需要受保护基线变更及独立审批。
- TG-ADR-006：重试追加证据，quarantine 不静默缩分母。
- TG-ADR-007：门禁绑定准确合并候选，审批不覆盖未知证据。

可量化阶段、错误矩阵与未决选择见[技术设计](technical-design.md)。这些 ADR 是可修订目标决策，不是已有运行时保证。
