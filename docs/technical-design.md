# TestGuard — 技术方案与实施设计

> 版本 V0.1（计划）。初始仓库只有文档，所有示例命令均为拟议，不可直接执行。

## 1. 工具与组件选择

Rust 2024/Serde/Clap/Tokio 负责计划编译、进程编排、限额、归一化与 GuardEngine SDK 交互；测试运行复用 Maven/Gradle/JUnit、Cargo、Vitest/Jest、Playwright 以及覆盖率与变异测试的官方输出。TestGuard 不取代框架本身，不从大模型自然语言声称中产生通过判定。

建议模块结构：

~~~text
src/{plan,obligation,runner,adapters,report,coverage,policy,cli}.rs
adapters/{junit,maven,gradle,cargo,vitest,playwright}/
schemas/{test-plan,test-execution,coverage}/
fixtures/{pass,fail,zero-tests,skipped,timeout,tamper,drift}/
docs/
openspec/changes/
~~~

首版可单 crate，进程外适配器使用有版本的 JSON envelope。

## 2. 数据模型

- TestObligation：id、owner、requirementIds、architectureInvariantIds、criticality、requiredEnvironments、approvedRevision。
- TestPlan：planId、candidateDigest、contractDigest、preRunRequiredTests、executorId/version、argv、timeout、expectedArtifacts、approvals。
- ExecutionRecord：runId、actualCommand、exitStatus、start/end、environmentDigest、toolVersion、rawArtifactDigests、runSequence、cancelReason。
- TestCaseResult：testId、discovered、started、finished、verdict(pass/fail/skip/unknown)、duration、sourceRef。
- CoverageEvidence：kind(statement/branch/requirement/contract/mutation)、numerator、denominator、unmappedObligations、limitations。

所有测试 ID 与实际执行依据必须保留，未发现必需测试时明确缺失，而不是自动删除 TestPlan 项目。

## 3. 验证状态机

~~~text
PLANNED → PREPARED → RUNNING → COLLECTING → ASSESSING
                                      │
                           ┌──────────┴──────────┐
                           ▼                     ▼
                         COMPLETE             INCOMPLETE
                           │                     │
                       PASS/FAIL             BLOCK/REVIEW
~~~

区分 runner 退出、断言失败、测试集合不足、原始报告格式故障和 coverage 未满足。禁止将测试“没有失败”解释成“覆盖全部批准义务”。所有输出附精确候选 Git commit/tree、环境及合同摘要；目标 ref 漂移需重新执行。

## 4. 集成协议及适配器

GuardEngine v1alpha1 只支持禁止关系匹配，不支持直接推断覆盖率或测试计划。“required test missing” 等领域结论由 TestGuard 独立计算后归一化为 GuardFacts 或具有严格 schema 的 TestGuard finding。升级统一协议时保留原始框架报告并做版本转换，不能只传 boolean passed。

具体策略示例：要求测试 TEST-42 属于批准计划但未出现在执行事实内 → finding TG-REQUIRED-TEST-MISSING；至少一个关键验收用例被跳过 → INDETERMINATE；报告无法解析 → ExecutionError 而非 ClaimPassed。

## 5. 命令/接口（规划）

~~~sh
testguard doctor --project .
testguard plan --contract approved.yaml --candidate HEAD
testguard run --plan test-plan.json --format json
testguard verify --plan test-plan.json --report evidence.json
testguard coverage --requirement REQ-017
~~~

MCP 初期仅提供发现、计划、运行状态、证据查询；启动真实测试有副作用，必须显示权限、超时和可能的网络/数据库影响。CI 使用不包含生产密钥的隔离执行环境，按固定计划/受保护合并候选重新验收。输出 SARIF 是可选诊断视图，不能替代原生测试证据。

## 6. 安全、可靠性、性能

- 对候选代码和测试脚本采用最小权限容器/VM 或 CI Runner；凭据与签名控制端隔离。
- 子进程限制 CPU/内存/时间/输出大小；取消具备进程树回收和 unknown 对账。
- Flaky 重试只能在策略授权范围内执行，保留每轮错误与原始报告；quarantine 不可默默缩小分母。
- 缓存以 candidate/plan/toolchain/env 摘要键控；测试结果对不同合入候选不可跨用。
- 大型测试项目可以增量调度，但所有必需义务仍要标明执行或缺口。

## 7. 交付 Waves

| Wave | 交付物 | 验收 |
|---|---|---|
| T0 | TestPlan/Execution 模型、CLI doctor、JSON Schema | 无运行也能准确列缺工具 |
| T1 | JUnit/Cargo 真实适配、用例 ID 映射 | pass/fail/skip/zero-tests 区分 |
| T2 | 冻结必需计划、覆盖差异、环境矩阵 | 缺必需测试即阻断 |
| T3 | 受保护测试基线、flaky/异常/取消恢复 | 删除断言和篡改不能放行 |
| T4 | Vitest/Playwright、CI、MCP、版本兼容 | 真实多语言与合并候选验证 |

每个 Wave 需要 RED→GREEN→受影响回归；先故意破坏再测试守卫是否拒绝。不能将完成文档计作已实现检查器。
