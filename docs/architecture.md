# TestGuard — 测试守卫总体架构

> 版本 V0.1 | 状态：设计基线（尚无 TestGuard CLI 实现） | 2026-10-09

## 1. 架构目标

TestGuard 的核心问题不是“有没有跑过测试”，而是**针对已批准的需求和行为契约，哪些测试必须执行、实际上执行了什么、证据是否足以支持当前候选**。防止 AI 自己生成少量容易通过的测试、删除失败用例、降低断言或者仅上传最后一次绿色报告，就宣布交付完成。

本守卫拥有冻结测试计划、测试执行与覆盖事实、行为/契约/回归验证、测试证据可信性和负向测试检查；不拥有需求含义（SpecGuard）、领域不变量的定义（ArchGuard）、一般静态代码质量（CodeGuard）、审批与阶段转换（FlowGuard）、Git 合并（GitGuard）。

### 设计原则

- 应执行的测试集合必须在运行前由已批准的验收义务和变更范围确定，不能根据已通过的测试集合反推“全部覆盖”。
- 执行状态、行为结论、覆盖状态彼此独立；测试命令退出码 0 但 0 用例执行，**不代表行为通过**。
- 编码 Agent 允许新增测试，但删除必需测试、降低断言、降覆盖门槛均属于受控变更，不能自我批准。
- 覆盖率和变异分数只能说明被测范围中的部分质量属性，绝不能证明程序完全正确。

## 2. 逻辑分层

~~~text
Approved requirements / Architecture invariants / Change scope
                        │
                        ▼
                 Frozen Test Plan
              Test obligation / environments
                        │
                        ▼
             Test Discovery & Selection
                        │
                        ▼
              Bounded Test Executor
      JUnit / Maven / Gradle / Cargo / Vitest / Playwright
                        │
                        ▼
               Evidence Collectors
       Original tool reports + exit + tests actually run
                        │
                        ▼
             Adequacy & Coverage Rules
       Required set vs executed set, assertion drift
                        │
                        ▼
                  GuardEngine
              Contract / Rule / Evidence
                        │
                        ▼
                 TestGuard Result
               CLI / CI / FlowGuard
~~~

### 2.1 Test Plan Compiler

接收来源：SpecGuard 的 Requirement/Acceptance ID、ArchGuard 的状态机和领域不变量测试义务、CodeGuard 的构建前置条件、GitGuard 的文件和符号变更范围。编译为 TestPlan（testId、suite、executor、args、environment、required、reason、origin、expectedArtifacts、timeout）。计划必须标明哪种语言/框架、target/features、数据库/外部服务条件已覆盖，避免用一次本地 smoke test 冒充所有配置。

### 2.2 Runner 与 Adapter

首批推荐 Maven Surefire/Failsafe、JUnit XML、Cargo test、Vitest/Jest 和 Playwright 报告。测试命令以固定 argv 和受控工作目录执行；标准输出和错误输出受界限控制；取消须终止受管子进程并保留部分证据；不能偷偷联网下载测试工具。生产密钥不可暴露给不可信候选代码。

### 2.3 Coverage & Evidence

证据包含 RequiredTestSet、DiscoveredTestSet、ActuallyExecutedSet、Passed/Failed/Skipped/TimedOut、environmentDigest、sourceDigest、contractDigest、verifierVersion、inputCommit、原生报告摘要与可核验执行者。单测覆盖（行/分支）、需求覆盖、契约覆盖、并发/状态机覆盖单独给出分母和目标，不能用单一“80% 覆盖率”代替需求满足。

## 3. 安全与信任

可信 CI 的必需测试计划和门槛来自受保护基线，不能读候选 PR 自己放宽的 YAML/CI 配置做放行。流水线区分“检查器失败”“测试断言失败”“环境故障”“不完整扫描”；失败/重试/隔离历史全部保留，禁止只呈现最后一次绿色状态。Evidence 本地重算不等于可信签名，真正合并由独立 Git 平台执行。

## 4. Guard Protocol 协作

GuardEngine v1alpha1 支持 GuardContract/GuardFacts/GuardReport 和精确关系禁止规则；TestGuard 本域必须先完成集合与覆盖计算，再输出带来源事实、finding、coverage 信息。完整 TestPlan/ExecutionRecord/TestCoverage 在后续版本化专业协议中补充，不能将未定义字段塞给现有 schema 后假装获得引擎支持。

SpecGuard 定义验收义务，ArchGuard 定义领域测试不变量，TestGuard 自己决定测试计划及实测是否充分；FlowGuard 仅决定阶段推进资格；GitGuard 校验最终候选与授权合并条件。

## 5. ADR

- TG-ADR-001：计划冻结在运行前；实际测试结果不能改变必需义务。
- TG-ADR-002：分开记录执行状态、行为 verdict 和覆盖情况。
- TG-ADR-003：底层优先调用原生测试框架，不在 Rust 重写 JUnit/Cargo/Vitest。
- TG-ADR-004：零用例、全部跳过、超时和报告丢失不能 PASS。
- TG-ADR-005：测试配置、阈值和必需用例的弱化必须经独立批准。

## 6. 反例与验收

| 注入场景 | 预期 |
|---|---|
| 测试计划要求 12 项，实际只执行 8 项且全部绿 | INCOMPLETE / BLOCK |
| 0 tests executed, exit=0 | 不能宣布行为通过 |
| 候选删除失败断言、修改必需测试名单 | 检测计划/测试基线弱化 |
| Flaky 连续重试最终成功 | 保留中间失败，依批准策略处理 |
| Parser 崩溃/报告截断 | 保留已知失败 + 未覆盖范围 |
| 源码或最终合并基线发生变化 | 旧报告失效 |
| 真实合法功能及全部必需测试完成 | 本域可通过，但不生成 Git 合入权 |

实现方案及阶段依赖见 [技术设计](technical-design.md)。
