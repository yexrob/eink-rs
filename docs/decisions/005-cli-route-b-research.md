# CLI 框架：路由 B 调研

- 日期：2026-10-21
- 状态：已实施
- 提交：无
- Brief：[route-b-research](briefs/route-b-research.md)

## 背景

路由 A 落地（[004](004-cli-route-a.md)）。ink-rs 的 [056](https://github.com/yexrob/ink-rs/blob/main/docs/decisions/056-upper-framework.md) 定了路由 A 和 B 都做，先 A 后 B。Claude 问要不要开始调研路由 B，用户答「搞」。

路由 B 是 Pastel 的文件路由：`commands/` 下的文件即命令，目录即命令组。Rust 不能在运行时按文件加载模块，要在编译期从目录生成路由 A 的类型。

## 决策

- 开始路由 B：先调研，和用户对过方案再派 worker。开始是用户定的（「搞」）；先对方案是用户此前定下的做法。
- 以下由 Claude 按用户的授权（「搞」）定：
  - 调研派一个 Sonnet 子 agent。用户此前说「文档探索任务以后不要拍Opus， sonnet就够了」。
  - 调研回答五个问题：
    - Pastel 文件路由的全部规则；
    - Rust 里从目录生成代码的机制（build.rs、proc macro），比较重编译、rust-analyzer 和报错位置；
    - 已有 crate 的做法；
    - 命令文件怎么声明描述、别名和 default，clap 的属性写在 `Args` 结构体上是否生效；
    - 生成的代码怎么复用路由 A。
  - clap 的行为在 scratchpad 的临时 crate 里实测，不改仓库。

## 方案

- Sonnet 子 agent，brief 见 [route-b-research](briefs/route-b-research.md)。报告写到 scratchpad，结论补进本记录。
- 主 agent 读完报告，整理两三个方案给用户选。定下后另开实施记录。

## 结果

Sonnet 子 agent 的报告（scratchpad，未入库）结论：

- **Pastel**：每次启动扫描 `commands/` 并 import 所有文件。文件即命令，目录即组；根 `index` 是不带命令时的命令，子目录的 `index` 让组本身可运行，并提供组的描述；`_app` 包裹所有命令；`isDefault`、`alias`、`description` 是文件的导出。不检查冲突，同名时 `Map.set` 静默覆盖。
- **机制**（本机实测）：
  - build.rs 写 `OUT_DIR` 再 `include!`：被 include 的文件里，`mod` 相对 `OUT_DIR` 解析，要绝对 `#[path]`；rust-analyzer 不认内联模块里的绝对 `#[path]`；生成代码的报错指向 `target/`。
  - 函数式宏输出裸 `pub mod x;`：rustc 和 rust-analyzer 都相对宏调用处的模块解析，不用 `#[path]`。稳定版察觉不到新增文件，加一行 `cargo::rerun-if-changed` 的 build.rs 后增删改名都重编。nightly 的 `proc_macro::tracked::path`（#99515）未稳定。
  - 属性宏挂在外联的 `mod commands;` 上不可行（#54727）。
- **已有 crate**：`automod` 不递归，新增文件不重编；`axum-folder-router`、`yew-fs-router` 用「宏 + 一行 build.rs」。没有找到「目录即 CLI 命令」的 crate。
- **clap 元数据**：写在 `Args` 结构体上的文档注释、`about`、`long_about`、`alias`、`visible_alias`、`hide` 生效，`name` 不生效，变体上的设置优先；`#[command(..)]` 是 `Command` 的方法调用，不能借它写自定义标记。主 agent 补测：子目录的 index 结构体 flatten 进组后，它的文档注释和 `visible_alias` 成为组的描述和别名，不带子命令时解析成 index。
- **复用路由 A**：生成的 enum 满足 `Commands` derive 的限制，`Commands` 和 `cli::run` 不用改。

Claude 据此问了三个问题，用户的选择和实施方案见 [007](007-cli-route-b.md)。
