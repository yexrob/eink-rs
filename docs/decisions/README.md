# 决策记录

每次写代码或派 worker 之前，先在这里记下依据：做了什么决定、谁定的、比较过哪些方案、工作怎么拆。`docs/design.md` 写设计现在的样子，这里写它怎么来的。

本仓库之前的讨论记在 ink-rs 的[决策记录](https://github.com/yexrob/ink-rs/tree/main/docs/decisions)里，这里用链接引用，不复制。

## 格式

文件名 `NNN-slug.md`，编号按时间顺序。

```markdown
# 标题

- 日期：2026-10-21
- 状态：实施中 | 已实施 | 部分实施 | 已延后 | 被 NNN 取代
- 提交：`abc1234` feat: …（没有写「无」）
- Brief：[name](briefs/name.md)（没有写「无」）

## 背景
为什么要做决定。用户的原话用「」引。

## 决策
逐条写，每条注明谁定的：用户（引原话），或 Claude 按用户的授权（引授权的话）。

## 备选与取舍
比较过的方案和放弃的原因。没有就省略。

## 方案
工作怎么拆：派给谁（Codex、Claude 子 agent、主 agent 自己），brief 的要点，验收命令和测试范围，调研问了什么、结论是什么。

## 结果
提交、review 发现的问题和处理、偏离方案的地方、后续由哪条记录接手。
```

背景和决策必填，其他小节空着就省略。写之前开一条记录，状态填「实施中」，和改动一起提交或先于改动提交；工作落地后补结果和提交。

## Brief

`briefs/` 存派给 worker 的原文。本机路径换成 `<repo>`（仓库根）、`<scratchpad>`（会话的临时目录）和 `~`。brief 引用的 `<scratchpad>` 下的文件（Ink UI、Pastel 的源码克隆，调研笔记，worker 报告）没有入库，记录里写了它们的结论。

## 索引

| 编号 | 日期 | 标题 | 状态 |
|---|---|---|---|
| [001](001-scope-and-rules.md) | 2026-10-21 | 仓库范围与协作规则 | 已实施 |
| [002](002-inkui-port-plan.md) | 2026-10-21 | Ink UI 移植：结构、主题与分工 | 已实施 |
| [003](003-inkui-examples.md) | 2026-10-21 | Ink UI 示例 | 已实施 |
| [004](004-cli-route-a.md) | 2026-10-21 | CLI 框架：路由 A | 已实施 |
| [005](005-cli-route-b-research.md) | 2026-10-21 | CLI 框架：路由 B 调研 | 已实施 |
| [006](006-ink-rs-0-1-3.md) | 2026-10-21 | 升级 ink-rs 0.1.3 | 已实施 |
| [007](007-cli-route-b.md) | 2026-10-21 | CLI 框架：路由 B | 已实施 |
| [008](008-release-0-1-0.md) | 2026-10-21 | 发布 0.1.0 | 已实施 |
| [009](009-web3-demo.md) | 2026-10-21 | 用 eink-rs 改写 web3 demo，加 `cli::run_with` | 已实施 |
| [010](010-router-research.md) | 2026-10-21 | 应用内路由调研 | 已实施 |
| [011](011-router.md) | 2026-10-21 | 应用内路由：Router、`#[derive(Routes)]`、`routes!` | 已实施 |
| [012](012-release-0-2-0.md) | 2026-10-21 | 发布 0.2.0 | 已实施 |
| [013](013-ship-demo.md) | 2026-10-21 | 部署控制台示例 `ship`；路由词不再被当成子命令 | 已实施 |
| [014](014-release-0-2-1.md) | 2026-10-21 | 发布 0.2.1 | 已实施 |
| [015](015-quill-demo.md) | 2026-10-21 | 编码助手示例 `quill` | 已实施 |
| [016](016-markdown-stream.md) | 2026-10-22 | Markdown 流式渲染：先做 demo | 已实施 |
| [017](017-ink-rs-0-1-4.md) | 2026-10-22 | 升级 ink-rs 到 0.1.4 | 已实施 |
| [018](018-markdown-stream-formal.md) | 2026-10-22 | Markdown 流式渲染：正式实施 | 已实施 |
| [019](019-pierre-highlight-research.md) | 2026-10-22 | 调研 pierre 的高亮方案 | 已实施 |
| [020](020-markdown-image-research.md) | 2026-10-22 | 调研 Markdown 图片显示 | 已实施 |
| [021](021-highlight-long-lines-and-reset.md) | 2026-10-22 | 高亮的长行上限和重置后的排队工作 | 已实施 |
| [022](022-gpuix-herdr-research.md) | 2026-10-22 | 调研 GPUIX 的终端渲染和 herdr 的图形处理 | 已实施 |
| [024](024-bingo-research.md) | 2026-10-22 | 调研 bingo 需要的 eink-rs 改动：0.3.0、Composer、控件、Code | 实施中 |
