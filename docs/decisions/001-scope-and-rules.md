# 仓库范围与协作规则

- 日期：2026-10-21
- 状态：已实施
- 提交：本记录所在的提交，docs: add agent rules and decision records
- Brief：无，主 agent 自己做

## 背景

eink-rs 来自 ink-rs 的记录 [056](https://github.com/yexrob/ink-rs/blob/main/docs/decisions/056-upper-framework.md)。ink-rs 0.1.1 发布后，用户提出要一个类似 React 生态里 Next.js 的上层，让用户用起来方便。讨论的结论：

- 上层放新仓库，参照 Pastel（CLI 路由）和 Ink UI（组件库）。
- context、children、回调这些原语放进 ink-rs，见 ink-rs 的 [057](https://github.com/yexrob/ink-rs/blob/main/docs/decisions/057-runtime-primitives.md) 和 [059](https://github.com/yexrob/ink-rs/blob/main/docs/decisions/059-rsx-props-derive.md)，随 ink-rs 0.1.2 发布。
- 仓库名 eink-rs，GitHub 仓库在 ink-rs 的 [060](https://github.com/yexrob/ink-rs/blob/main/docs/decisions/060-release-0-1-2-and-eink-rs.md) 里建好。

Claude 给出 Ink UI 的移植方案后，用户说：「两件事儿 1. 新仓库的CLAUDE.md AGENTS.md 也初始化一下 和本仓库类似 2. 这个你仓库不只是ink ui, 还有后续的那个框架 注意哈」。

## 决策

- 仓库有两部分：Ink UI 组件的移植，参照 Pastel 的 CLI 框架。Ink UI 在前，框架在后；框架里 clap derive 的路由 A 和四项补充（退出码、异步命令、shell 补全、测试工具）在前，build.rs 的文件路由 B 在后。用户在 ink-rs 056 里定下，这次重申（「不只是ink ui, 还有后续的那个框架」）。
- `CLAUDE.md` 沿用 ink-rs 的四条规则，`AGENTS.md` 是指向它的符号链接，同 ink-rs。用户决定（「和本仓库类似」）。
- 以下由 Claude 按用户的指令定：
  - 开头加一句仓库范围，写明两部分。
  - 第 1 条去掉 Windows console FFI 的豁免，那些文件在 ink-rs 里。
  - 加第 5 条：功能缺 ink-rs 的能力时停下来问，通用原语放进 ink-rs，不在这里绕过。依据是 056 的分工：框架代码不进 ink-rs，通用原语进 ink-rs。
  - 决策记录的格式同 ink-rs，编号从 001 开始，ink-rs 里的相关记录用链接引用。

## 方案

主 agent 自己做：把空仓库克隆到本机，写 `CLAUDE.md`、`AGENTS.md`、`docs/decisions/README.md` 和本记录，一个提交。推送等用户同意。
