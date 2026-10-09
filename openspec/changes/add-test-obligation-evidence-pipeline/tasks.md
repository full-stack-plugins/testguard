# TestGuard Evidence Pipeline Implementation Plan

> **For agentic workers:** 后续实施使用 superpowers:subagent-driven-development 或 superpowers:executing-plans 逐任务执行；功能实施已获授权；checkbox 仅按独立评审证据登记，未勾项仍待完成。

**Goal:** 建立批准义务、真实测试执行及精确候选证据的可追踪管线，防止部分绿色运行误报通过。

**Architecture:** 专业 schema 与 plan/runner/adapters/coverage/policy/report 分离。TestGuard 拥有领域充分性，GuardEngine 只提供通用 Contract/Rule/Evidence；可信控制器拥有身份、批准和消费授权。

**Tech Stack:** 建议 Rust 2024、Serde/Clap/Tokio、原生测试报告；具体版本/平台尚未选定，没有依赖已安装的声明。

**Spec:** [义务规划 delta](specs/testguard-obligation-planning/spec.md)、[执行证据 delta](specs/testguard-execution-evidence/spec.md)、[设计](design.md)、[架构](../../../docs/architecture.md)、[技术方案](../../../docs/technical-design.md)、[集成草案](../../../docs/integration-contract.md)、[跨仓库路线](../../guard-roadmap.md)。

## Global Constraints

- 历史规划基线没有运行时；当前实施已有源码、测试和原生报告。以实施进展记录和精确 commit 为准，不把历史规划描述当作当前状态。
- 每项实施以拒绝/接受 fixture 驱动，保存真实输入、工具版本、执行命令、输出/退出与 digest；不能凭文档检查宣称运行完成。
- 现有 `guard.partme.ai/v1alpha1` 严格 schema 不变；`guard.integration/v1alpha1` 为独立草案，GE-CONTRACT 冻结前不宣称互操作。无隐式 N/N-1。
- `check` 目标 0/2/3/4；绑定前失败无信封，绑定后 error/cancelled decision=null；completed 引擎 profile decision 等于 report，partial 为 BLOCK。
- 审批由外部控制器认证，不能改写专家 REQUIRE_APPROVAL 或绕过缺证据/运行失败。默认无外部副作用、无网络/生产密钥/合并权。
- 本地 fixture/schema 原型可并行 GE；互操作计划与事实等 SG-BASELINE + GE-CONTRACT/ADAPTER，可信消费等 GE-TRUST，独立生产发行等 GE-RELEASE；不等待其他仓库全完成。

## Review Focus

- 参数化用例同名/重复 XML 与旧报告冒充本轮：任务 2.1–2.3 必须拒绝冲突及旧工件。
- coverage 空分母/过滤改变或高行覆盖掩盖漏需求：任务 2.4–2.5 单独验分母与缺口。
- 配置沙箱看似成功但孙进程存活/共享数据库串扰：任务 3.1–3.2 实测回收预算与十并发隔离。
- 部分有效报告与 parser crash 混淆、无 OID 仍发信封：任务 4.2–4.3 验精确 transport 状态。
- 审批撤销、必要工件丢失和迟到旧 PASS 被缓存掩盖：任务 3.4、4.4–4.6 逐项拒绝。

## 1. T0 — 模型、接口与阶段决策

输入：设计和专业 fixture；输出：版本化模型、CLI 合同与本地验证入口。本组不依赖外部仓库整体完成；集成字段仍等待 GE-CONTRACT。

- [x] 1.1 在拟议 `docs/decisions/testguard-bootstrap.md` 与 `Cargo.toml` 固定首版 Rust/依赖、Cargo 报告协议支持版本、JUnit identity 规则及单 crate 边界；用 `tests/bootstrap_profiles.rs` 验明示支持/拒绝版本各至少一例，未选版本不得进入支持清单。追踪 “Versioned test domain contracts”“Native adapter evidence fidelity”。
- [x] 1.2 在 `schemas/test-obligation.json`、`schemas/test-plan.json` 和 `src/obligation.rs` 定义 `ObligationSet`/`FrozenPlan`，输入批准来源引用、输出唯一 ID/环境及 binding；`tests/domain_schema.rs` 建至少 12 个正反例，覆盖未知字段/版本、重复 ID、悬空引用、缺环境/绑定。追踪 “Versioned test domain contracts”。
- [x] 1.3 在 `schemas/{test-execution,test-coverage,domain-finding}.json` 和 `src/report.rs` 定义 `AttemptRecord`、`CoverageEvidence`、`DomainFinding`，保留 native 身份和原始工件引用；`tests/domain_records.rs` 验 unknown/skip/fail 不转 pass、无结束记录不能通过。追踪 “Versioned test domain contracts”“Requirement and environment trace coverage”。
- [x] 1.4 在 `src/report/normalize.rs` 定义受限工件引用及归一化规则，区分专业元数据与集成 ref；`tests/normalization.rs` 对每个有效输入重复十次摘要一致，未知字段和摘要冲突拒绝。追踪 “Versioned test domain contracts”“Authenticated evidence audit and freshness”。
- [ ] 1.5 在 `src/cli.rs` 固定 doctor/plan/run/check/verify/coverage 输入输出与退出合同，`tests/cli_contract.rs` 验 check 0/2/3/4、stdout/stderr 分离和其他命令已文档化语义；无 CLI 已存在假设。追踪 “Read-only and execution interface separation”。
- [x] 1.6 在 `src/doctor.rs` 实现只读工具/权限检测，`fixtures/doctor/` 为每个初始支持工具提供已装/未装两种环境；`tests/doctor.rs` 验无下载、无执行测试、无外部写入且诊断准确。追踪 “Read-only and execution interface separation”。

## 2. T1–T2 — 真实适配器与冻结追踪

输入：T0 schema、固定工具和原生工件；输出：观察集合及冻结义务覆盖。2.1–2.3 可独立于外部 gate；2.4–2.6 的真实生产导出依赖 SG-BASELINE 与 GE-CONTRACT/ADAPTER，之前只能运行明确标识的本地 fixture。

- [x] 2.1 在 `src/adapters/junit.rs` 与 `fixtures/junit/` 采集 Maven/Gradle 真实报告/退出，`tests/junit_adapter.rs` 验八类 pass/fail/skip/zero/partial/timeout/missing-report/malformed 和参数化碰撞、旧文件，计数对照原生来源，0 false ALLOW。追踪 “Native adapter evidence fidelity”。
- [x] 2.2 在 `src/adapters/cargo.rs` 与 `fixtures/cargo/` 依 1.1 固定协议采集真实 Cargo 运行，`tests/cargo_adapter.rs` 重复八类矩阵并验 ignored/参数/target/features 范围及未知版本拒绝，不假设未验证 JSON 能力。追踪 “Native adapter evidence fidelity”。
- [x] 2.3 在 `src/adapters/mod.rs` 统一 `RawArtifactSet + ExecutorProfile → CaseObservation[]`，`tests/adapter_identity.rs` 验 native/stable ID、参数/环境消歧、重复与退出矛盾拒绝、仅接受本 attempt 工件。追踪 “Native adapter evidence fidelity”。
- [x] 2.4 在 `src/plan.rs` 消费 SG-BASELINE 的 `ObligationSet` 和不可变 binding/policy 生成 `FrozenPlan`，`tests/plan_freeze.rs` 用三义务×两环境逐个删除实例均留缺口，候选自改名单不能缩集合，未知导出 capability 拒绝。追踪 “Frozen approved obligation plan”“Requirement and environment trace coverage”。
- [x] 2.5 在 `src/coverage.rs` 与 `src/policy.rs` 计算义务/测试/环境矩阵和分开的执行/statement/branch 指标，`tests/coverage_matrix.rs` 验空分母 N/A、额外通过不抵缺失、行覆盖不抵需求；阈值/过滤/已知断言删减保留原分母并提出复核。追踪 “Requirement and environment trace coverage”“Baseline authority and weakening review”。
- [x] 2.6 在 `src/obligation/trace.rs` 输出 requirement/acceptance/invariant 双向证据链与覆盖查询，`tests/requirement_trace.rs` 验每个失败/缺项能回到批准修订、悬空引用拒绝，分开两个 requirement scope。追踪 “Requirement and environment trace coverage”“Frozen approved obligation plan”。

## 3. T3 — 隔离运行与全部尝试证据

输入：FrozenPlan、受限 ExecutionPermission；输出：AttemptRecord、追加历史和工件。可信执行者身份接入仍等 GE-TRUST；本组本地沙箱实验不冒充可信生产来源。

- [ ] 3.1 在 `docs/decisions/testguard-sandbox.md` 与 `src/runner/sandbox.rs` 选定单平台、资源限额和进程终止预算，实现固定 argv/受控 worktree；`tests/sandbox.rs` 验默认禁网络/宿主写/生产凭据、路径/符号链接出界拒绝及十任务端口/数据库/工件不串扰。追踪 “Isolated bounded execution”。
- [x] 3.2 在 `src/runner/process.rs` 实现超时、取消与进程树回收，`tests/process_lifecycle.rs` 注入子孙进程、回收失败和超预算，验成功清理在配置预算内、未完成 unknown、失败不报告成功，记录 OS 强杀可能无输出。追踪 “Isolated bounded execution”“Strict engine projection and error transport”。
- [x] 3.3 在 `src/runner/collect.rs` 与原生 parser 实施工件大小/深度及 XML 无外部实体限制，`tests/artifact_security.rs` 注入实体、截断、超限、压缩/路径滥用和报告篡改各一例均拒绝且保留已有失败。追踪 “Isolated bounded execution”“Native adapter evidence fidelity”。
- [x] 3.4 在 `src/report/store.rs` 实现按 attempt 追加的受限存储、原始摘要/来源/脱敏与部署保留策略，`tests/evidence_store.rs` 验不可覆盖前轮、访问隔离、必要工件删除/到期拒绝消费，审计字段完整。追踪 “Authenticated evidence audit and freshness”。
- [x] 3.5 在 `src/policy/retry.rs` 实现有界诊断重试与 flaky 分类，`tests/flaky_history.rs` 覆盖 pass→fail、fail→pass、fail→fail，每轮都保留原始工件，fail→pass 不自动清除 mandatory failure。追踪 “Append-only retries and quarantine”。
- [x] 3.6 在 `src/policy/quarantine.rs` 校验 owner/原因/到期/批准/替代保障，`tests/quarantine.rs` 验字段缺失、到期/撤销、候选自批均不能缩分母，只有新的合法批准基线能改变义务。追踪 “Append-only retries and quarantine”“Baseline authority and weakening review”。

## 4. TG-EVIDENCE — 引擎投影、信任与并发

输入：领域 assessment、外部门冻结接口；输出：符合 profile 的集成证据。4.1–4.3 等 GE-CONTRACT/GE-ADAPTER；4.4–4.6 的可信消费额外等 GE-TRUST。`TG-EVIDENCE` 只有专业追踪、真实适配与本组互操作证据达到所需 profile 时成立，不能只凭 schema 文件打勾。

- [x] 4.1 在 `src/report/engine_adapter.rs` 固定映射版本与 engine profile，输入 DomainAssessment、输出当前严格合同/事实，`tests/engine_mapping.rs` 对必需缺失/失败/review/advise 正反 golden vectors 验 schema 和预期决定；未知 capability 拒绝，不扩展当前 wire。追踪 “Strict engine projection and error transport”。
- [x] 4.2 在 `src/report/transport.rs` 区分 `PreBindingDiagnostic` 与 `BoundAttemptEnvelope`，`tests/error_transport.rs` 验参数/仓库/候选/覆盖未冻结无信封、无空 OID；绑定后 parser/runtime/cancel error 为 null decision/4，已有失败保留。追踪 “Strict engine projection and error transport”。
- [x] 4.3 在 `src/report/envelope.rs` 使用 GE-CONTRACT 冻结的独立 schema，`tests/envelope_parity.rs` 验 completed 合同/事实/报告 refs 必需、decision 与 report 相等、partial 为 BLOCK/2、verify 重算不声称来源认证、未知版本/字段拒绝。追踪 “Strict engine projection and error transport”。
- [ ] 4.4 在 `src/integration/trust.rs` 接 GE-TRUST 验证端口，由外部控制器提供认证执行者/审批；`tests/trust_refs.rs` 验伪造/到期/撤销/不可访问/范围扩大均拒绝，unsigned hash 不能认证，专家 REQUIRE_APPROVAL 不被批准记录改写。追踪 “Authenticated evidence audit and freshness”“Baseline authority and weakening review”。
- [ ] 4.5 在 `src/report/freshness.rs` 定义不可变复用键且默认禁跨 run 结果缓存，`tests/invalidation.rs` 逐项改变 candidate/base/merge-group/source/baseline/policy/analyzer/config/coverage/approval 均失效，历史不变，缓存不掩盖撤销。追踪 “Authenticated evidence audit and freshness”。
- [ ] 4.6 在 `src/runner/scheduler.rs` 实现等价请求幂等、独立 attempt 和当前绑定 compare-and-set，`tests/concurrent_attempts.rs` 验重复请求不重复启动、两需求不串用审批/证据、晚到旧 ALLOW 不能覆盖新失败。追踪 “Candidate-bound concurrency and queue checks”。

## 5. T4 — 框架扩展、集成与发布

输入：通过 T0–T3 与相应 TG-EVIDENCE profile 的实现；输出：额外适配器、受控集成和可回滚交付。独立队列证据任务 5.4 仅等待 GG-CANDIDATE + GE-TRUST，不等待消费其结果的 FG-GATE；只有最终联合任务 5.6 等待相关 FG-GATE；独立生产发行等 GE-RELEASE，不阻塞早期 pinned-source 开发。

- [ ] 5.1 在 `src/adapters/{vitest,jest,playwright}.rs` 增加明示版本框架，`fixtures/{vitest,jest,playwright}/` 与 `tests/extended_adapters.rs` 每框架重跑 T1 八类真实矩阵并保留 native ID/环境，未验收版本仍 unsupported。追踪 “Native adapter evidence fidelity”。
- [ ] 5.2 在 `src/coverage/{contract,state,concurrency,mutation}.rs` 定义可支持的来源与过滤范围，`tests/advanced_coverage.rs` 对每指标验分母/排除/未映射与空分母，精选弱化 mutation 必须检出且明确不能证明任意语义完备。追踪 “Requirement and environment trace coverage”“Baseline authority and weakening review”。
- [ ] 5.3 在 `src/integration/mcp.rs` 提供发现/计划/状态/证据查询与显式受权 run，`tests/mcp_permissions.rs` 验只读 token 无法启动测试、执行需限额/权限、取消状态正确、输出 profile 与 CLI 一致。追踪 “Read-only and execution interface separation”“Isolated bounded execution”。
- [ ] 5.4 在 `src/integration/ci.rs` 与 `tests/merge_queue.rs` 接精确 synthetic candidate/base/merge-group，验 PR-head 证据拒绝、队列变更重检、外部审批不赋合并权；CI 配置受保护且控制端凭据不进入 runner。追踪 “Candidate-bound concurrency and queue checks”“Authenticated evidence audit and freshness”。
- [ ] 5.5 在 `docs/compatibility.md`、`docs/rollout.md` 与拟议发行配置固定独立分发及 GE-RELEASE 支持矩阵，`tests/rollout.rs` 验 advisory→shadow→opt-in、未知版本 fail closed、回退批准适配器/配置保留原生入口与历史且不跳过已要求检查。追踪 “Phased validation release and rollback”。
- [ ] 5.6 在 `fixtures/integration/` 与 `tests/end_to_end.rs` 接 END-TO-END 场景，记录准确队列候选、两需求、expiry/revocation、漂移、迟到、native CodeGuard parity 和回滚的真实证据；汇总 T0 至少 12 schema/T1 每适配器八类/T2 三×二/T3 十并行结果与实测资源成本，无运行记录不得声明完成。追踪 “Phased validation release and rollback”“Candidate-bound concurrency and queue checks”。
