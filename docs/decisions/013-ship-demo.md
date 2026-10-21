# 部署控制台示例 `ship`；路由词不再被当成子命令

- 日期：2026-10-21
- 状态：已实施
- 提交：`d9b9210` fix(router): keep later route words from matching subcommands；`aaaa25f` feat(examples): add the ship deploy console
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

## 结果

- `RouteArgs` 的修正按方案落地（`d9b9210`）。去掉修正时，新加的断言失败；加上后 `--workspace --all-features` 的测试和 clippy 通过。
- Codex 交回 26 个文件，+1896 行，没有新增依赖，没有碰到缺的原语：
  - `backend.rs` 是接口和数据类型，加一个按指针比较的句柄；
  - `sim.rs` 是模拟实现；
  - `commands.rs` 是 `status` 和 `logs`；
  - `view.rs` 放共享的视图；
  - `src/routes/` 下 13 个页面、3 个布局；
  - `tests/ship.rs` 是两个测试。
- Codex 定的细节：
  - 根组件用 `use_animation` 推进 backend，再经 context 提供一个 `Revision`，读数据的组件靠它在每次推进后重渲染。
  - 向导的草稿放在布局里一个以服务名为 key 的子组件中，换服务时重挂载。
  - 服务页用 Tab/Shift-Tab 切标签，Ctrl-P 切换 pin，pin 是演示布局状态保留的那一项。
  - `worker v2.0.0` 在第 6 秒失败。
  - 导航：`push` 用在列表、向导和账号入口；`replace` 用在标签切换和确认后进入执行页；`back` 用在外壳的 Esc 和确认页的取消；`push_path` 用在总览菜单。
- review 时主 agent 改了这些：
  - `Backend::advance` 改名为 `refresh`：接口不该带模拟器的说法；
  - 在 scale 页输入副本数时，`d` 会跳到发布向导，现在这一页不响应 `d`；
  - 健康状态到颜色的映射原来写了两份，合成 `ConsoleTheme::health`；`view.rs` 补了空行和文档；
  - 草稿没选区域时，标题末尾多一个 `/`，现在显示 `choose regions`；
  - 账号页的邮箱框按 Enter 会接受补全并保存，而 token 还空，所以报错。现在 Enter 接受补全后把焦点移到 token 框，在 token 框按 Enter 才保存；
  - 模拟日志原来每秒同一句话，改成随机的请求和延迟。随机数原来是 xorshift，种子小，前几步高位几乎不变，延迟都是 112、113、114ms，改成 SplitMix64。
- 主 agent 在 tmux 窗口 `ink-rs:ship` 里跑过：
  - 总览：事故出现时，健康度降到 82%，Alert 也显示了；
  - 服务列表的过滤，详情页的标签切换（pin 保留）；
  - 日志的暂停；扩缩容的校验和确认；回滚和它的进度；
  - 向导选 `worker v2.0.0` 走到失败；从执行页返回时跳过确认页，草稿还在；
  - 切主题后，外壳和 Badge 的颜色一起变；账号页能补全邮箱、切焦点、保存；
  - `ship services api logs` 直接打开那一页；
  - `status` 退出码 0，`status nope` 退出码 1，`nope` 退出码 2；`logs api -n 3 -f` 的旧行留在终端历史里，按 q 退出。
- 全量：fmt（`routes!` 下的文件用 `rustfmt --edition 2024` 检查）、clippy 通过，`--workspace --all-features` 134 个测试通过。
- 修正没有发布，0.2.1 另问用户。
