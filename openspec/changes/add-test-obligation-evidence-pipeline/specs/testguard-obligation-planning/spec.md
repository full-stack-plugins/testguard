## ADDED Requirements

### Requirement: Versioned test domain contracts

TestGuard SHALL 定义独立版本化的 TestObligation、TestPlan、ExecutionRecord、CoverageEvidence 与 DomainFinding schema，拒绝未知字段、版本、重复身份与悬空引用；专业模型 MUST NOT 作为未知扩展字段写入现有 GuardContract/GuardFacts/GuardReport。软件版本、policy revision、专业 schema 与集成版本 SHALL 分开。

#### Scenario: Unknown domain schema or duplicate identity

- **WHEN** 输入含未知 schema 版本、额外字段或相同 test×environment 身份冲突
- **THEN** 验证拒绝输入并给定位诊断，不生成可消费的冻结计划

#### Scenario: Deterministic normalization

- **WHEN** 对同一有效领域输入进行十次归一化
- **THEN** 十次得到同一语义摘要，且无需改写原始报告或依赖墙钟时间

### Requirement: Frozen approved obligation plan

TestGuard SHALL 在执行前从受保护的已批准基线冻结义务、测试×环境集合、policy/baseline digest、executor/argv/限额、工件要求与不可变调用绑定。实际 requirement-based 生产者 SHALL 消费 SG-BASELINE 稳定 ID 与 TestObligation export 并等待 GE-CONTRACT/GE-ADAPTER；发现结果、候选编辑或运行结果 MUST NOT 缩减必需集合。

#### Scenario: Candidate removes a mandatory test

- **WHEN** 候选删除已批准计划中的必需测试或降低阈值
- **THEN** 冻结义务不变，缺失/弱化显式列出，候选的自批标记不产生批准

#### Scenario: Obligation export not yet supported

- **WHEN** SG TestObligation 导出版本或所需 capability 未实现
- **THEN** 互操作计划生成失败并说明不支持；本地实验 fixture 不冒充已批准生产输入

### Requirement: Requirement and environment trace coverage

TestGuard SHALL 保留 requirement/acceptance/invariant→obligation→test×environment→attempt→artifact→finding 的双向追踪，分别报告执行、需求、statement、branch、contract、state、concurrency、mutation 的分母、范围、过滤和缺口。空分母 SHALL 为 N/A；额外通过用例和行覆盖 MUST NOT 抵销缺失必需义务。

#### Scenario: Missing one required environment

- **WHEN** 三个义务各需两个环境，任意一个必需实例未运行，即使额外用例全部通过
- **THEN** 对应义务不计满足分子，缺失实例可回溯到需求，事实为 partial 且不能 ALLOW

#### Scenario: Empty coverage denominator

- **WHEN** 某覆盖指标适用分母为零
- **THEN** 输出 N/A 及原因，保留其他指标，不声称 100% 或需求已满足

### Requirement: Baseline authority and weakening review

TestGuard SHALL 以不可变 revision/ref 和 digest 引用基线，要求可信控制器验证审批身份、作用域、有效期及撤销状态；布尔值或 Markdown accepted MUST NOT 作为授权。删除测试/改变过滤/降低阈值/可疑断言差异 SHALL 提示受控复核，系统 MUST NOT 声称能自动识别任意语义弱化。审批 MUST NOT 修复不完整事实、工具失败或改写专家技术决定。

#### Scenario: Approval expires after plan creation

- **WHEN** 已冻结计划的批准记录在消费前过期、撤销或不能核验
- **THEN** 计划证据的消费资格失效，历史记录不变，不能沿用缓存批准

#### Scenario: Assertion changes without new approved baseline

- **WHEN** 候选减少已知必需断言或修改覆盖过滤但只有自行填写的 accepted 标记
- **THEN** 输出弱化诊断并保留原分母，要求独立基线审查，不把该标记当审批

### Requirement: Read-only and execution interface separation

TestGuard SHALL 分离 doctor/plan/coverage/证据查询与执行候选代码的 run 权限，明确输入输出/版本/能力；doctor MUST NOT 自动联网安装工具，MCP run SHALL 显式要求执行权限及预算。目标 check SHALL 使用 0 ALLOW、2 BLOCK、3 REQUIRE_APPROVAL、4 输入/执行/核验错误，stdout JSON 与 stderr 诊断分离；其他子命令语义 SHALL 在发布前固定。

#### Scenario: Missing tool during discovery

- **WHEN** doctor 检测到某受支持工具未安装
- **THEN** 返回明确缺失诊断且无下载安装或测试执行副作用

#### Scenario: Query caller attempts execution

- **WHEN** 仅有查询权限的 MCP 调用者请求 run
- **THEN** 请求在启动进程前被拒绝，不使用查询权限隐式授权测试代码执行
