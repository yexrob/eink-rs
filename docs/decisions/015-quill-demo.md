# 编码助手示例 `quill`

- 日期：2026-10-21
- 状态：实施中
- 提交：无
- Brief：[quill-demo](briefs/quill-demo.md)

## 背景

用户：「再写一个类似codex cli的 模拟tui 使用eink-rs」。

Claude 在 tmux 里打开本机的 Codex CLI 0.162.1，只截屏，不发消息：

- 它在普通屏里内联运行。开头一张信息卡，写着名字、版本、目录和权限；下面是输入框 `› Ask Codex to do anything`，再下面是一行模型、推理强度和上下文用量，最后一行按键提示。
- 输入 `/` 时，命令列表出现在输入框上方，每行是命令名和说明；输入框为空时按 `?`，显示三栏快捷键（Compose、Session、Transcript），`⌃j` 换行。
- 退出后，终端历史里留下 `codex resume <id>` 的提示。

Claude 按这些发了方案，用户答「可以」，又问用 eink 还是 ink。Claude 答：用 eink-rs 的组件、headless hook 和命令行框架；`Static`、`View`、`Text` 和输入、动画 hook 来自 ink-rs，eink-rs 构建在它上面，不重新导出它。

## 决策

用户定（「可以」，针对下面的方案）：

- 示例包 `examples/quill/`。`quill` 是一个虚构的编码助手，不用 Codex 和 OpenAI 的名字。
- 普通屏内联渲染：写完的对话单元经 `Static` 进终端历史，底部只留进行中的一轮、状态行、弹出列表、输入框和底栏。
- 对话单元：开头的信息卡、用户消息、助手回复（Markdown 子集）、思考摘要、读过的文件、命令和截断的输出、彩色 diff、计划清单。
- 进行中的一轮流式出现，状态行显示用时和「esc to interrupt」，Esc 中断。
- 审批用 Select：执行、本会话不再问、拒绝，y/a/n 直选。
- 弹出列表：`/` 命令（边输入边过滤，Tab 补全）、`@` 文件、`?` 快捷键。命令有 `/model`、`/approvals`、`/status`、`/diff`、`/new`、`/quit`。
- 多行输入框建在 `use_input_state` 和 `reduce_input` 上：Enter 发送，Ctrl-J 换行，上下键翻历史，忙时排队。
- 底栏：模型、推理强度、上下文用量；Ctrl-C 按两次退出，退出时打印 token 用量和 resume 提示。
- 命令行：`quill [PROMPT]`，`-m`，`-a untrusted|on-request|never`；`quill exec <PROMPT>`，失败时退出码 1；`quill completions`。
- `Agent` 接口是事件流，配一个按关键词选剧本、由时钟驱动的模拟实现；交互界面和 `exec` 消费同一个事件流。
- 测试两个：交互流程；`exec` 的退出码。
- 不做 Ctrl-T 的全屏历史（运行中切备用屏，ink-rs 不支持）、粘贴图片、vim 模式、外部编辑器、路由。
- 派一个 Codex 在独立 worktree 里写，主 agent review、合入、跑全量测试、在 tmux 里演示。

以下由 Claude 按用户的授权（同上）定：

- **依赖**：eink-rs 只开 `cli` 和 `completion`，加 ink-rs 和 std。
- **`-m`** 收字符串，由 `Agent` 列出的模型校验，不认识时 `exit_with_error`，退出码 1，同 `ship` 的服务名。模型清单属于 agent，不写进 clap。
- **Ctrl-C**：交互模式关掉 ink-rs 的 `exit_on_ctrl_c`，自己处理：忙时中断，输入框有字时清空，空闲时第一次提示、第二次退出。`exec` 保留默认。
- **退出时的摘要**画在最后一帧里，再请求退出：内联模式卸载后，最后一帧留在终端里。
- **剧本**：「test」「fix」走修测试（读文件、跑测试失败、改代码、再跑通过，`untrusted` 下要审批）；「explain」「what」「how」只读并回答；「refactor」「plan」以计划为主；含「fail」时流中断，这一轮出错；其余给简短回答。
- **审批模式**：`untrusted` 每条命令都问；`on-request` 只问剧本标了需要提权的命令；`never` 不问。`exec` 按 `never` 运行。
- **工作区**是模拟的文件清单，`@` 列表和 diff 都来自它；信息卡上的目录显示进程的真实当前目录，只读不写。
- Codex 只改 `examples/quill/` 和根 `Cargo.toml` 的 workspace 成员。README 和 `docs/design.md` 由主 agent 写。

## 备选与取舍

- **输入框用 eink-rs 的 TextInput**：单行，值由组件自己管，应用改不了，Tab 补全命令和插入文件路径做不到。headless 的 `use_input_state` 把值交给应用，正好用来演示 headless 那一层。
- **全屏**：Codex 和 Ink 写的工具都是内联的；全屏和路由由 `ship` 演示。
- **事件由线程推送**：要跨线程通道，测试也不确定。按时钟拉取，测试可以用 ink-rs 的假时钟逐步推进。

## 方案

1. 主 agent：本记录、brief 和 README 索引行先提交。
2. 派一个 Codex，worktree 基于 main，brief 见 [quill-demo](briefs/quill-demo.md)。它只跑 `quill` 包的测试和 brief 里列出的命令，不跑交互界面。
3. 主 agent 集成：review，合入，fmt、clippy、全量测试；README 和 `docs/design.md` 提到 `examples/quill/`；在 tmux 里走一遍全部剧本、弹出列表和命令给用户看；补本记录的结果；删 worktree。
