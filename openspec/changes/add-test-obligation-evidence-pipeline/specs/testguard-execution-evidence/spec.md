## ADDED Requirements

### Requirement: Native adapter evidence fidelity

TestGuard SHALL 先实现真实 JUnit/Maven/Gradle 与固定版本 Cargo 适配，再对 Vitest/Jest、Playwright 和覆盖工具逐项验收。每个适配器 SHALL 声明支持版本、保存原始工件/native ID/参数/环境和发现/开始/结束状态，拒绝身份冲突、未知格式/版本、陈旧工件和退出/报告矛盾；exit=0 MUST NOT 代替测试执行证据。

#### Scenario: Zero tests with zero exit

- **WHEN** 原生命令返回零但没有必需用例执行
- **THEN** 不生成 ALLOW，保留零测试观察和必需缺口

#### Scenario: Adapter conformance corpus

- **WHEN** 某适配器申请加入受支持列表
- **THEN** 至少通过 pass、fail、skip/ignored、zero、partial、timeout、missing-report、malformed 八类真实运行对照，归一化计数与原生证据一致且无 false ALLOW

### Requirement: Isolated bounded execution

TestGuard SHALL 以固定 argv 在隔离的候选 worktree 执行，限制 CPU/内存/时间/进程/输出/工件，默认无网络、生产凭据、签名或合并凭据及宿主写入。XML 外部实体、路径/符号链接越界和超限工件 SHALL 拒绝；取消 SHALL 在配置终止预算内回收受管进程，未完成用例保留 unknown，回收失败不得报告成功。

#### Scenario: Parallel resources remain isolated

- **WHEN** 十个任务并行执行且包含两个不同 requirement scope
- **THEN** 每个任务的 worktree、工件、端口、数据库命名空间与批准引用不串用，结果只绑定原任务

#### Scenario: Cancellation or malicious artifact

- **WHEN** 测试取消或提交外部实体/越界路径/超限工件
- **THEN** 取消触发进程树回收，恶意工件拒绝；保留已有证据但不产生通过决定

### Requirement: Append-only retries and quarantine

TestGuard SHALL 为每轮尝试分配新身份并保留原始失败、命令、环境和工件；fail→pass SHALL 仍记录 flaky，默认诊断重试 MUST NOT 覆盖 mandatory failure。quarantine SHALL 有 owner、理由、期限、批准与替代保障，且 MUST NOT 自动删除必需分母。

#### Scenario: Successful retry retains failed attempt

- **WHEN** 同一必需测试先 fail 后 pass
- **THEN** 两轮证据都可查询，分类含 flaky，最终 pass 不自动使义务满足

#### Scenario: Quarantine expires

- **WHEN** quarantine 到期或其审批撤销
- **THEN** 例外不再可消费，必需义务仍存在，不通过减少分母获得 ALLOW

### Requirement: Strict engine projection and error transport

TestGuard SHALL 保持当前 `guard.partme.ai/v1alpha1` 严格 schema 与 exact forbid_relation，用明确版本映射将领域判断转成兼容事实。独立 `guard.integration/v1alpha1` 草案 SHALL 等 GE-CONTRACT/ADAPTER 冻结后实现，不假设 N/N-1。完整绑定、producer、必查 coverage 冻结之前失败 SHALL 只输出独立诊断、无信封；之后 error/cancelled SHALL decision=null；completed 引擎支持信封 SHALL 引用合同/事实/报告且 decision 等于报告。有效 partial facts SHALL BLOCK，不能伪装执行错误或审批通过。

#### Scenario: Unresolved candidate before execution

- **WHEN** candidate/base 或 required scope 无法解析冻结
- **THEN** 输出失败诊断及目标退出 4，无 GuardRunEnvelope，无伪造 OID 或空绑定字段

#### Scenario: Parser crash after binding freeze

- **WHEN** 完整绑定和覆盖已冻结但报告解析失败
- **THEN** error 信封 decision=null，保留已知失败工件，退出 4，不制造成功 GuardReport

#### Scenario: Completed partial projection

- **WHEN** 有效执行记录明确缺少必需测试范围
- **THEN** partial facts 求得 BLOCK/INDETERMINATE，completed 信封 decision=BLOCK 与引用 report 一致，退出 2

### Requirement: Authenticated evidence audit and freshness

TestGuard SHALL 保存计划、环境/工具、每轮观察、原始工件摘要、分析器范围和执行来源引用，按权限与保留政策访问并脱敏。可信消费 SHALL 等 GE-TRUST 且由外部控制器核验执行者/审批；unsigned 报告或重算 digest MUST NOT 证明身份。candidate/base/merge group、source/baseline/policy、analyzer/config/coverage 与批准到期/撤销变化 SHALL 使受影响资格失效。必要工件丢失 SHALL 拒绝消费；缓存默认不跨 run 复用执行结果。

#### Scenario: Digest matches but producer is unauthenticated

- **WHEN** 上传报告摘要正确但执行者身份无法由控制器认证
- **THEN** 可以记录一致性核验结果，但不能把它作为可信合并证据

#### Scenario: Stale or unavailable evidence

- **WHEN** 任一必需绑定变化或必要原始工件过期丢失
- **THEN** 历史证据保留为审计记录，当前资格失效，重新运行受影响义务

### Requirement: Candidate-bound concurrency and queue checks

TestGuard SHALL 用不可变输入绑定和计划摘要区分等价请求，以独立 attempt 追加重试；迟到结果 MUST NOT 覆盖新候选状态。可信 CI SHALL 检查精确 synthetic merge-group candidate 与 base，PR-head-only 证据 MUST NOT 满足 queue obligation。两个 requirement scope SHALL 保留独立义务、审批及执行历史；ALLOW 不授予合并/发布权。

#### Scenario: Late old candidate completes green

- **WHEN** 新队列候选已产生，而旧候选晚到 ALLOW
- **THEN** 旧结果只归档原绑定，不能更新新候选状态或免除其重检

#### Scenario: Duplicate execution request

- **WHEN** 相同不可变绑定与请求 ID 被重复提交
- **THEN** 不重复启动同一次尝试，显式新重试仍分配独立 attempt 并保留历史

### Requirement: Phased validation release and rollback

TestGuard SHALL 以 T0–T4 门逐步交付，记录真实正负例输入/工具版本/输出/退出/摘要；文档或 checkbox 不作为运行证据。生产独立发行 SHALL 等 GE-RELEASE 并发布显式兼容矩阵；集成 SHALL 从 advisory→shadow→opt-in protected checks，回滚 SHALL 保留工件及原生测试入口且不静默跳过已要求门禁。END-TO-END SHALL 验证准确队列候选、双需求、到期/撤销、漂移、迟到、native CodeGuard parity 与回滚。

#### Scenario: Unsupported integration after upgrade

- **WHEN** 升级适配器与固定引擎 profile 不兼容
- **THEN** 拒绝不支持的输入并回退已批准版本/配置，保留历史工件，不将未知证据标为 ALLOW

#### Scenario: Phase claims completion

- **WHEN** 某阶段请求验收
- **THEN** 提供该阶段量化夹具和实际执行证据；仅有设计文件或测试文件存在不能完成验收
