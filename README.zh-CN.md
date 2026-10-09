# TestGuard — 测试守卫

[English](README.md) · [简体中文](README.zh-CN.md)

TestGuard 是 Partme Guard 的**测试计划、行为验证、回归和证据守卫**。它要回答：**当前版本究竟执行了哪些被要求的测试？证据是否足以满足事先批准的验收义务？**

> **当前状态：详细架构和技术方案已建立，尚无可运行的 TestGuard CLI、MCP 或可信 CI 门禁。**

## 为什么独立设计 TestGuard

AI 自己编写几条容易通过的测试，不等于产品通过验收。仅看测试命令的退出码和覆盖率，也无法知道被删除的测试、被跳过的异常场景和未覆盖的需求。

~~~text
批准的验收/领域不变量 → 冻结测试计划 → 原生测试框架
                    → 运行结果/覆盖/环境/失败历史
                    → TestGuard 判断 → GuardEngine 证据
                    → FlowGuard / 独立 CI
~~~

## 核心责任

- 在执行测试**之前**固定必需用例、环境矩阵和覆盖分母。
- 接入 JUnit/Maven/Gradle、Cargo test、Vitest/Jest、Playwright 等原生工具。
- 区分通过、失败、跳过、0 用例、取消、超时、工具故障、报告缺失和部分覆盖。
- 检测候选删除必需测试、降低断言、只上传最后一次绿色报告等绕过方式。
- 关联 SpecGuard 的验收义务、ArchGuard 的领域不变量和 GitGuard 的最终合并候选。

**局部测试 PASS ≠ 代码完全正确，也不等于允许 Git 合并。** 通用规则/证据机制由 [GuardEngine](https://github.com/full-stack-plugins/guardengine) 提供，审批和阶段条件归 FlowGuard。

## 设计文档

[详细架构](docs/architecture.md) · [详细技术方案](docs/technical-design.md)

## 规划接口（尚不可执行）

~~~sh
testguard plan --contract approved.yaml --candidate HEAD
testguard run --plan test-plan.json --format json
testguard verify --plan test-plan.json --report evidence.json
~~~

首个实现先用真实 JUnit/Cargo 场景验证正例、错误断言、部分结果及零测试，随后完善覆盖、跨语言、可信 CI 与 MCP。全部功能以真实执行证据为完成标准，不将规划文档计为已交付代码。
