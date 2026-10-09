# TestGuard — 技术方案与实施设计

> 设计 V0.2；2026-10-09。检查基线 `b4e8ed05b985f6233d671c76f1132aaa4d99a507`，仅四份文档；源码、manifest、测试、CI、OpenSpec 和全部 CLI 均不存在。本文的模块、schema、命令、测试与阶段为拟议，不声称执行或交付。

## 1. 实施边界与组件

建议 Rust 2024 + Serde/Clap/Tokio 管理计划编译、CLI、限额、进程与归一化，具体依赖版本待首次 manifest 锁定和兼容验收。复用 Maven/Gradle/JUnit、Cargo、Vitest/Jest、Playwright 以及原生覆盖/变异报告；不凭大模型自然语言声明构造通过结论。可以从单 crate 开始，进程外适配器再采用版本化 JSON 接口。

拟议结构（均尚不存在）：

~~~text
src/{plan,obligation,runner,report,coverage,policy,cli}.rs
src/adapters/{junit,cargo,vitest,jest,playwright}.rs
schemas/{test-plan,test-execution,test-coverage}/
fixtures/{pass,fail,zero-tests,skipped,timeout,tamper,drift}/
openspec/changes/
~~~

`plan` 负责批准基线/引用校验与冻结；`runner` 管理隔离及进程树；`adapters` 严格解析原生报告；`coverage/policy` 计算领域结果；`report` 保存原始摘要、构建 schema 兼容事实及独立集成信封。隔离执行能力必须由选定 OS/容器平台证明，不能只依赖应用层工作目录限制。

## 2. 目标专业数据契约

以下为概念字段，不是现有 GuardEngine schema 或已发布 JSON Schema。

| 模型 | 必需概念 | 校验 |
|---|---|---|
| TestObligation | id、requirementIds、acceptanceIds、architectureInvariantIds、criticality、requiredEnvironments、approvedRevision | ID 唯一、引用存在、修订受保护；没有自动推断的需求映射 |
| TestPlan | schemaVersion、planId、candidate/base/merge-group、contract/baseline digest、requiredTests、executors、argv、limits、expectedArtifacts、approvalRefs | 固定不可变对象、禁止运行后改必需集合；环境完整；argv 不做 shell 拼接 |
| ExecutionRecord | runId、attemptId、planDigest、executor/toolchain/environment、start/end、exitStatus、cancelReason、artifactDigests、case observations | attempt 单调追加，工件范围合法；未开始或未完成不能 pass |
| TestCaseResult | stable/native test ID、suite、parameters、environment、discovered/started/finished、verdict、duration、sourceRef | 重复或碰撞 ID 拒绝；unknown 不隐式转换 skip/pass |
| CoverageEvidence | kind、numerator、denominator、scope、excluded、unmappedObligations、tool version、limitations | 分母稳定且可审计；0 分母 N/A；跨指标不汇总为一个百分比 |
| DomainFinding | code、义务/测试/环境 refs、artifact refs、解释、策略来源 | 代码稳定、可追踪；独立工件，不添加未知 GuardReport 字段 |

工件引用至少描述摘要算法/值、媒体类型、大小和受限存储定位；建议 SHA-256，具体 canonical JSON、路径排序与 schema 验证规则在 T0 固定。工件身份不得依赖可覆盖文件名。时间戳用于审计，判定绑定使用内容与不可变 Git 对象；墙钟差异不改变语义集合判断。仅归一化与引擎证据计算可追求确定性，测试执行本身可能受调度和环境影响。

### 需求追踪例（设计数据，不是实际测试）

| requirement/acceptance | obligation | 必需用例×环境 | 本轮观察 | 结果 |
|---|---|---|---|---|
| REQ-017/AC-1 | O-PAY-SUCCESS | pay_success×db-A | pass | 满足此项 |
| REQ-017/AC-2 | O-PAY-REJECT | pay_reject×db-A | skip | 未满足 |
| REQ-017/AC-3 | O-PAY-IDEMPOTENT | retry_once×db-A、retry_once×db-B | A pass、B missing | 未满足 |

执行覆盖为 2/4 完成必需实例（这里 skip 不算执行完成的有效验收实例），需求验收满足为 1/3；两者不能混称“覆盖率”。实现应额外展示 ended、skipped、passed 等原始计数，避免改变术语分母。无论行覆盖是多少，该例不允许 ALLOW。

## 3. 计划、运行及判定算法

1. 从受保护基线解析义务、门槛和授权记录；由可信控制器认证审批，不接受候选自批。
2. 将可变 ref 解析为 candidate/base/merge-group 不可变身份，绑定 repo/task/worktree/requirement 范围。
3. 冻结必需 `R={(obligation,test,environment)}` 和预期原始工件；发现集合 D 不可缩减 R。未知框架版本、悬空引用或缺环境使计划无效。
4. 在隔离 worktree 与每轮独立 artifact 目录运行固定 argv；记录发现/开始/结束和退出。报告文件必须属于本轮，防止上次绿色文件被重用。
5. 解析每次尝试，构建观察集合 E，计算 `missing=R−E`、skip/unknown/failure 和未映射报告项；额外测试不能抵销缺失必需测试。
6. 对每个义务的所有必需环境求满足性；按固定工具/过滤规则单独计算 statement、branch、contract、state、concurrency、mutation。
7. 应用 flaky/隔离/基线策略，生成领域 finding；用版本化映射生成现有 schema 能表示的事实，完整性如实声明。严禁把部分运行伪装 complete。
8. 校验实际序列化的合同/事实符合当前引擎 schema，再求值。固定映射版本与对照夹具，证明缺必需测试会触发预期强制关系规则；不得假设引擎原生理解百分比或任意 finding。
9. 封存原始工件、专业报告和集成信封。消费前重新核对候选、范围、基线和审批时效；变化则 stale 并重新安排检查。

完整性是分析器声明范围内的完成度，不能由“没有发现违规”推出。测试命令 exit=0 仅是输入之一；报告与退出矛盾应作为错误诊断，而非任选较乐观结果。

## 4. 状态机与错误合同

`DRAFT → VALIDATED → FROZEN → PREPARED → RUNNING → COLLECTING → ASSESSING → COMPLETED`；任何阶段可进入 ERROR/CANCELLED；失效是消费资格变化，不抹除历史终态。每次重试新建 attempt，恢复必须核对 immutable binding，不能继续写另一个候选的 run。

| 情况 | 领域/运行状态 | 目标 check 输出及 gate 行为 |
|---|---|---|
| 所有必需项通过且策略满足 | completed、充分 | ALLOW / 0，仅域内通过 |
| 必需断言失败、覆盖低于强制门槛 | completed、违反策略 | BLOCK / 2 |
| 有效报告但缺必需测试、全跳过、零测试 | completed、partial/INDETERMINATE | BLOCK / 2，不得批准覆盖 |
| 完整充分、明确 review 规则需审批 | completed、待审批 | REQUIRE_APPROVAL / 3 |
| 工具缺失、无报告、XML/JSON 截断、进程故障、非法输入 | error，保留已知观察 | 外层 decision=null / 4；门禁阻断 |
| 超时/资源限制 | error，保留部分工件 | decision=null / 4；门禁阻断 |
| 主动取消 | cancelled | decision=null；拟议 CLI 统一 4，OS 强制终止不保证落盘 |
| 摘要不符、来源不匹配、绑定过期 | verification error | decision=null / 4；拒绝消费 |

错误信封独立于当前 v1alpha1 GuardReport。已知失败不可被后续 parser 错误抹去，但工具错误也不能伪装有效引擎 BLOCK 报告。enforce/review/advise 的一般求值由 GuardEngine 负责，TestGuard 的必需证据充分性前置条件不能降成 advise 后放行。

## 5. CLI、MCP 与集成（全部规划，不可运行）

~~~sh
testguard doctor --project .
testguard plan --contract approved.yaml --candidate HEAD
testguard run --plan test-plan.json --format json
testguard check --plan test-plan.json --execution execution.json --format json
testguard verify --plan test-plan.json --report evidence.json
testguard coverage --requirement REQ-017
~~~

`doctor` 只检查本地依赖与权限，不自动安装或联网；`plan` 生成冻结计划；`run` 有执行候选代码的副作用；`check` 消费证据并给技术决定；`verify` 重算摘要与事实绑定；`coverage` 查询领域矩阵。读取命令默认只读。CLI 尚不存在，因此没有可验证的 `--report` 行为；拟议 `verify --report` 是输入文件，不能推断其为统一输出选项。未来输出文件应原子写入且不覆盖其他 run 工件。

`check` 对齐 0/2/3/4，JSON stdout 与 diagnostics stderr 分离；其他命令退出语义在 T0 固定，禁止把所有原生 runner 退出码直接透传为 guard 决策。MCP 首先提供发现、计划和证据/状态查询；运行接口须声明授权、资源预算及外部服务副作用。SARIF 可作诊断视图，不能替代原生报告或来源证明。

[共享集成信封](integration-contract.md)是独立版本 `guard.integration/v1alpha1` 草案，现有 engine 不解析；专业工件通过 digest 引用关联。调用绑定包含 repoId/taskId/worktreeId/requirementIds/candidateOid/baseOid/mergeGroupId；同时传递 runStatus、nullable decision、分析器/覆盖、审批引用和诊断。信封协议版本、软件 semver、测试策略版本分别升级。

CI 必须在受保护配置下重跑准确 merge-queue candidate；由可信控制器绑定 run 与执行身份、核验外部审批和门禁权限。不会由 TestGuard 发消息、推送、合并或发布。领域 parser/policy 留在本 guard；不存在已实现的外部旧插件适配器。

## 6. 隔离、重试、缓存与证据保留

- 工作目录绑定 repo/task/worktree/requirement/run/attempt；每轮独立临时目录、端口、数据库命名空间，固定 seed 并记录时间/locale 等相关环境。共享资源必须显式串行化。
- 最小权限容器/VM 或 CI runner；默认无网络、无生产凭据、无宿主写权限，依赖预置。不可信 YAML 不可改变沙箱权限。XML 禁外部实体；路径穿越/符号链接出界、压缩炸弹和超限工件拒绝。
- 限额覆盖 CPU/内存/子进程/超时/输出/工件；取消先受控终止后强制回收，未完成用例记 unknown。回收失败为运行错误，不能宣布清理完成。
- 默认重试仅作有界诊断；每轮都保留原始失败。批准策略可定义 flaky 是否需 review，但不能让最终 pass 自动覆盖 mandatory failure。quarantine 需 owner/原因/到期/批准/替代保障，不会偷偷缩小必需分母。
- 计划/工具链/环境/规则/分析器/范围/候选/base/merge-group/基线全部进入缓存绑定；审批每次消费重新检查。首版默认不跨 run 复用执行结果；后续缓存须证明所有依赖已覆盖，动态外部依赖无法固定则禁缓存。
- 幂等键为不可变绑定摘要加计划和执行请求 ID；重试另加 attempt ID，重复提交不重复启动同一尝试。当前候选指针以比较并交换更新，迟到结果仅封存自身记录。
- 工件写入后校验摘要；日志脱敏、访问隔离、可配置保留期。必要工件缺失或过期后重新运行，不能仅凭存留绿色摘要放行。报告 unsigned；摘要重算不认证来源。

## 7. 可测量交付阶段

以下均为**未来验收标准**，没有已运行结果。每期归档输入、固定工具版本、命令、期望/实际输出、原始工件摘要、负例与复现步骤；所有列出的验收用例必须通过，不能只给汇总绿色截图。

| 阶段 | 交付及依赖 | 可测量退出条件 |
|---|---|---|
| T0 | 专业 schema、CLI 骨架、doctor；无前置实现 | 至少 12 个 schema 正反例覆盖重复 ID、悬空引用、缺环境、非法版本、绑定缺失；每个受支持工具的已装/未装 fixture 各 1 个；重复归一化 10 次产生同一语义摘要；固定 CLI stdout/stderr/退出码合同 |
| T1 | JUnit 与固定版本 Cargo 实采；依赖 T0 | 每个适配器至少 8 个真实运行 fixture：pass、fail、skip/ignored、zero、partial、timeout、missing-report、malformed；每个都对照原生报告案例数与退出；0 false ALLOW；未支持的格式/版本拒绝 |
| T2 | 冻结计划、追踪与覆盖矩阵；依赖 T1 | 3 个义务×2 环境的矩阵，逐个删除必需观察均阻断；额外通过用例不抵销缺口；行/分支与需求覆盖分母分别验证；0 分母显示 N/A；候选修改基线/门槛不能自批 |
| T3 | 隔离、flaky、取消/恢复、可信基线；依赖 T2 | pass→fail、fail→pass、fail→fail 全历史保留；quarantine 到期/撤销均失效；10 个并行任务无工件/数据库交叉；取消在配置终止预算内回收所有受管进程；注入路径越界、实体解析、超限与报告篡改各 1 例均拒绝 |
| T4 | Vitest/Jest、Playwright、MCP、CI 与协议兼容；依赖 T3 | 每新增框架重复 T1 的 8 类矩阵；所有 0/2/3/4 输出契约通过；candidate/base/merge-group/rules/analyzer/coverage/baseline/approval 的逐项变化测试均失效；迟到旧 run 不覆盖新结果；完整 merge-queue 演练不得以旧分支报告放行 |

T3 的 10 并发是初始功能隔离验收规模，非性能容量承诺。性能先记录每 fixture 的时间/内存与 runner 基线，再基于真实数据制定预算，不编造吞吐量。每期都需要故意破坏再恢复的拒绝/通过对照和受影响回归。OpenSpec 目录若后续引入，再依实际 CLI 与 schema 做校验；目前不能声称 OpenSpec validate 通过。

## 8. 未决事项与可逆默认值

| 未决项 | 当前建议默认 | 解除条件 |
|---|---|---|
| Cargo 输出协议与工具版本 | 仅支持明确固定版本和验证过的报告适配；不猜测稳定 JSON 能力 | T1 真实 fixture 和版本文档 |
| JUnit 参数化/重复名称身份 | suite+参数+环境组合；碰撞拒绝，显式映射 | 真实框架 identity corpus |
| 沙箱平台和清理预算 | 首版单一受控 CI 平台，默认禁网络；限额配置必填 | T3 逃逸/回收与故障注入 |
| 断言弱化检测 | 文件/配置差异 + 已知变异负例 + 必要人工复核 | 不承诺任意语义弱化可自动识别 |
| 保留期与隐私 | 部署显式配置，至少覆盖证据有效期；缺失必要工件即失效 | 项目数据规则与容量评估 |
| 签名/远程执行来源 | unsigned 报告只做一致性核验；信任依赖外部受控执行者 | 独立版本化来源认证设计 |
| 跨运行缓存 | 默认关闭执行结果复用，允许按 digest 去重工件 | 完整依赖绑定和失效测试 |

以上选择与[总体架构](architecture.md)共同构成实现输入，不构成生产就绪声明。
