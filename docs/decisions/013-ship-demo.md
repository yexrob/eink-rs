# 部署控制台示例 `ship`；路由词不再被当成子命令

- 日期：2026-10-21
- 状态：实施中
- 提交：无
- Brief：[ship-demo](briefs/ship-demo.md)

## 背景

用户：「你给我制作一个多页面的demo 不是web3场景，能充分说明eink-rs的能力的」。

Claude 问场景和位置，用户选「部署控制台」和「eink-rs 仓库示例包 (Recommended)」。Claude 按这两个选择发了方案，用户答「可以」。

写 brief 前，Claude 核对了方案里的 `ship services api logs`。clap 把第三个词 `logs` 当成了 `logs` 子命令，报缺参数。原因是 `RouteArgs` 给根命令设了 `subcommand_precedence_over_arg(true)`：clap 在填变长位置参数的每个词时，都先查它是不是子命令名。不设这一项时，clap 只在第一个词和紧跟 flag 的词上查子命令。用 clap 4.6.7 写小程序对比过两种设置：

| argv | 设了 | 不设 |
|---|---|---|
| `services api logs` | 报错 | 路由 `/services/api/logs` |
| `logs api` | 子命令 | 子命令 |
| `-v wallet` | 子命令 | 子命令 |
| `market -v logs` | 报错 | 报错 |

## 决策

用户定（「可以」，针对下面的方案）：

- 示例包放在 `examples/ship/`，`ship` 是一个虚构的部署工具，数据是模拟的。
- 全屏页面用 `routes!` 从 `src/routes/` 生成：
  - `/` 总览：区域健康度、同步状态、当前事故、最近部署、菜单；
  - `/services` 服务列表，用 `use_select_state` 自己画行，带状态 Badge 和过滤输入；
  - `/services/:service` 带参数的布局，下面是概览、`logs`、`scale`、`rollback` 四页；
  - `/deploy/…` 发布向导，草稿存在布局里，换步骤不丢；
  - `/settings` 切主题（根布局换 Palette，全局生效），`/settings/account` 填邮箱和 token，Tab 切焦点。
- 共用外壳：面包屑、按键提示、Esc 返回，第一页按 Esc 退出；push、replace、back 都用到。
- 命令行：`ship` 进备用屏，`ship services api logs` 直接打开那一页；`ship status [SERVICE]` 打印快照，服务不存在时退出码 1；`ship logs api -f` 在普通屏持续输出；`ship completions zsh`。
- 数据经 `Backend` 接口读写，配一个时钟驱动的模拟实现。
- 测试只加两个：每条路由能渲染；`status` 的退出码。
- 不覆盖 tokio 入口，`cli-async` 示例已有。
- 派一个 Codex 在独立 worktree 里写，主 agent review、合入、跑全量测试、在 tmux 里演示。

以下由 Claude 按用户的授权（同上）定：

- **修 `RouteArgs`**：去掉 `subcommand_precedence_over_arg(true)`。第一个词和紧跟 flag 的词仍先匹配子命令，其余的词都是路径段。`tests/router.rs` 的 `subcommand_precedence` 加一条断言：子命令名出现在第二个词时是路径段。文档写明哪些位置的词会匹配子命令。这是 0.2.0 的 bug，修正后是否发 0.2.1 另问用户。
- **路由表**：
  - `deploy/index.rs` 选服务；
  - 选完进 `deploy/[service]/` 下的版本、区域、确认、执行四步，草稿放在 `deploy/[service]/_layout.rs`；
  - 服务页按键直接进 `/deploy/<服务名>`，argv 也能 `ship deploy api`。
  - 两处 `[service]/` 各生成自己的 `Service` newtype。
- **按键提示**由外壳对当前 `Route` 做 match 得出，页面不用往上传。外壳的颜色是自定义组件主题：`From<&Palette>`，用 `use_theme` 读，换 Palette 时跟着变。
- **全局按键**只有 Esc（和 ink-rs 默认的 Ctrl-C）：多个页面有文本输入，字母键留给页面。
- **部署进度**存在 backend 里，离开执行页不中断，总览的最近部署能看到它。
- **备用屏**：没有子命令时开，`status`、`logs` 不开，输出留在终端历史里。
- **依赖**：只用 eink-rs（`cli`、`router`、`completion`）、ink-rs 和 std。随机数自己写一个带种子的生成器。
- **Codex 只改 `examples/ship/` 和根 `Cargo.toml` 的 workspace 成员。** README 和 `docs/design.md` 由主 agent 写。

## 备选与取舍

- **向导用静态路径**（`/deploy/target`、`/deploy/review`）：服务页没法带着服务名跳进向导，argv 也打不开某个服务的向导。
- **页面经 context 把按键提示传给外壳**：要一个反向通道（外壳提供回调，页面在 effect 里写），多一层状态。外壳按路由 match 就够了，还能展示类型化的路由。
- **`RouteArgs` 改用 `args_conflicts_with_subcommands`**：`market -v logs` 也能当路径，但它让根上的 flag 和子命令互斥，`-v wallet` 会报错，影响用户自己的根命令。

## 方案

1. 主 agent：本记录、brief 和 README 索引行先提交；再改 `src/router/args.rs`、`src/router/mod.rs` 的模块文档、`docs/design.md` 的 argv 一节和 `tests/router.rs`，跑 `--workspace --all-features` 的测试和 clippy，提交修正。
2. 派一个 Codex，worktree 基于修正后的 main，brief 见 [ship-demo](briefs/ship-demo.md)。它只跑 `ship` 包的测试和 brief 里列出的命令，不跑需要终端的全屏和 `-f`。
3. 主 agent 集成：review，合入，fmt（`routes!` 下的文件 `cargo fmt` 进不去，用 `rustfmt --edition 2024`）、clippy、全量测试；README 和 `docs/design.md` 提到 `examples/ship/`；在 tmux 里跑全部页面和命令给用户看；补本记录的结果；删 worktree。
