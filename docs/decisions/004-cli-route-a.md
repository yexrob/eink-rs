# CLI 框架：路由 A

- 日期：2026-10-21
- 状态：实施中
- 提交：无
- Brief：[cli-route-a](briefs/cli-route-a.md)

## 背景

Ink UI 移植完成（[002](002-inkui-port-plan.md)），按用户确认的顺序轮到 Pastel 的 Rust 版。ink-rs 的 [056](https://github.com/yexrob/ink-rs/blob/main/docs/decisions/056-upper-framework.md) 已定：Pastel 只借概念，不对齐 help 格式、`-v` 和它的测试；路由 A 和 B 都做，先 A；Pastel 之外补四项：退出码、异步命令、shell 补全、测试辅助。用户这次说「处理， 和做Pastel 并行」。

Pastel 4.0.1 在 commander 和 zod 上做了四件事：文件即命令、schema 生成参数、校验报错、`_app` 包裹。它还有 index 命令（不带命令名时运行）、default 命令（有名字，不带命令名时也运行）、命令别名，以及从 `package.json` 读名称、版本和描述。

## 决策

用户在 Claude 的三个问题里选了推荐项：

- **命令接线**：命令的 `Args` 结构体同时 derive `Props`，结构体就是命令的组件；子命令 enum 上 `#[derive(Commands)]` 生成 `From<Enum> for Element`。用户选「Args 兼当 Props + derive 分派 (Recommended)」。
- **顶层**：顶层 `Parser` 结构体也是组件，Pastel 的 `_app`、index 命令、default 命令都写成它渲染函数里的普通代码，不设专门的 API。用户选「顶层 Parser 也是组件 (Recommended)」。
- **补全**：开 `completion` feature 后，`run` 自动加 `completions <SHELL>` 子命令。用户选「run 自动加 completions 子命令 (Recommended)」。

以下由 Claude 按用户的授权（056 中的「3 那几项按照你的推荐来搞吧」）定，写进 brief：

- **结构**：仓库改成 workspace，加成员 `macros/`（`eink-rs-macros`）放 derive。feature：`cli`（clap、derive 宏）、`completion`（加 clap_complete）、`tokio`（异步入口，开 ink-rs 的 `tokio`），默认都不开，只用组件的项目不拉 clap。
- **clap**：`eink_rs::cli` 重新导出 `clap`，用户不必自己对齐版本。
- **入口**：`run`、`run_from`，`tokio` 下加 `run_async`、`run_async_from`，命名同 clap 的 `parse`、`parse_from`。
- **测试辅助**：`parse_from::<C>(argv) -> Result<Element, clap::Error>`，交给 ink-rs 的 `testing::render` 或 `testing::mount`。一个函数，不另包一层测试 API。
- **退出码**：clap 的错误用 clap 的码（help、version 为 0，用法错误为 2）；`exit_with_error` 打印到 stderr，返回 1；`exit_with` 的值是 `ExitCode` 时用它；组件 panic 继续 unwind，保留 101；其余为 0。
- **名称、版本、描述**：用 clap 的 `#[command(version, about)]` 从 `Cargo.toml` 读。

## 备选与取舍

- **手写 match，不加宏**：每个命令多一行，嵌套组各写一遍。Pastel 的卖点是少写样板，路由 B 也要生成同样的分派代码。
- **专用属性表达 index、default、`_app`**：和 Pastel 一一对应，但多三个专门写法，而顶层组件的 `match` 已经能表达。
- **补全只给生成函数**：用户要自己挂命令。自动加子命令不用写代码，代价是不能再有同名命令。
- **默认开 `cli`**：只用组件的项目会多编译 clap。

## 方案

- 派一个 Codex，独立 worktree，brief 见 [cli-route-a](briefs/cli-route-a.md)。derive、入口和测试相互依赖，不拆。clap 和 clap_complete 由主 agent 预先下载到本机缓存，worker 离线构建。
- 测试只覆盖：derive 分派（含嵌套组和别名）、顶层组件的 index/default、五种退出码、`completions` 子命令。
- 示例：`cli`（同步，演示命令、参数、嵌套组、别名、default、包裹和错误退出）和 `cli-async`（`use_future` 加 Spinner）。
- 集成（主 agent）：review，跑全量和各 feature 组合，CI 改成 `--all-features` 并加无 feature 构建，`docs/design.md` 加 `cli` 一节，更新 README，在 tmux 里运行示例，补本记录的结果。
- 路由 B 在 A 落地后另开记录。
