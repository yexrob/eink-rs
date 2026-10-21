# CLI 框架：路由 B

- 日期：2026-10-21
- 状态：已实施
- 提交：`fb0eb3f` feat(cli): generate commands from a directory
- Brief：[cli-route-b](briefs/cli-route-b.md)

## 背景

调研见 [005](005-cli-route-b-research.md)。Claude 据调研问了三个问题，用户都选了推荐项。

## 决策

用户定：

- **机制**：函数式宏 `eink_rs::cli::commands!("src/commands")` 写在内联的 `mod commands {}` 里，展开成 `pub mod` 和各层 enum；用户的包加一行 `cargo::rerun-if-changed` 的 build.rs。用户选「宏 + 一行 build.rs (Recommended)」。
- **根**：根 `Parser` 手写，宏只生成命令树；包裹和 default 写在根组件里，同路由 A。用户选「根手写，宏只生成命令树 (Recommended)」。
- **命令组**：子目录支持 `index.rs`：它的文档注释和别名成为组的描述和别名，不带子命令时渲染它；没有 `index.rs` 的组不带子命令时打印 help。用户选「支持 index.rs (Recommended)」。

以下由 Claude 按用户的授权（「搞」和以上选择）定，写进 brief：

- **类型名**取文件名的 PascalCase：`super_deploy.rs` 里是 `SuperDeploy`，clap 的命令名为 `super-deploy`。不固定叫 `Args`，避免和 `clap::Args` 的导入重名。
- **生成的类型**：每层一个 `pub enum Command`，derive `Subcommand`、`Clone`、`PartialEq` 和路由 A 的 `Commands`。带 `index.rs` 的组另生成 `pub struct Group`：flatten `index::Index`，加 `Option<Command>`，开 `args_conflicts_with_subcommands`；`From<Group> for Element` 在没有子命令时渲染 index。
- **文件名规则**：
  - `.rs` 文件名须是小写 snake_case 标识符；Rust 关键字用原始标识符（`type.rs` 是 `r#type`）。
  - 报错的情况：`self`、`super`、`crate`、`mod.rs`，以 `_` 或数字开头、含连字符或大写；`foo.rs` 与 `foo/` 并存；没有命令的目录；根目录的 `index.rs`（根的 index 写在根组件里）。
  - 非 `.rs` 文件和隐藏条目跳过，条目按名字排序。
  - 错误用 `compile_error!` 指向宏调用处。Pastel 在这些情况下静默覆盖或不检查。
- **build.rs 检查**：宏展开时检查 `OUT_DIR`；包没有 build script 时报错，提示加那一行。稳定版的宏察觉不到新增文件，漏写 build.rs 时新文件会被静默忽略。
- 不支持 nightly 的 `tracked::path`。
- **示例**做成 workspace 成员 `examples/cli-routes/`（`publish = false`）：路由 B 依赖包的目录布局和 build.rs，单文件示例放不下。路由 B 的集成测试放在这个包里，eink-rs 本身不加 build.rs。
- `Commands` derive 和 `cli::run` 不改。

## 备选与取舍

- **机制**：提交生成的 `mod.rs` 要多一个生成器，新增文件后要记得重跑；build.rs 全量生成加 `include!`，报错指向 `target/`，要绝对 `#[path]`。
- **根也从文件生成**：更接近 Pastel，但多 `_app.rs`、根 `index.rs`、default 标记三条约定，default 标记要宏解析文件内容。
- **组只是目录**：少一条约定，组没有描述和别名。
- **固定类型名 `Args`**：宏不用转换名字，但和 `use clap::Args` 冲突，用户要写 `#[derive(clap::Args)]`。
- **eink-rs 加 build.rs**，让自己的示例和测试通过 `OUT_DIR` 检查：每个下游构建都要编译、运行它，它只为仓库里的示例服务。

## 方案

- 派一个 Codex，独立 worktree，brief 见 [cli-route-b](briefs/cli-route-b.md)。宏、示例包和测试相互依赖，不拆。
- 测试：
  - 宏 crate 的单元测试覆盖名字转换和每条报错规则。读目录和规划分开，规划是纯函数。报错规则代替 rustc 难懂的报错，没有测试会回退。
  - 示例包的集成测试覆盖分派（叶、别名、带 index 的组带或不带子命令、嵌套组、不带 index 的组）和 index 给组的描述与别名。
  - `OUT_DIR` 检查由 worker 手动验证，不提交测试。
- 集成（主 agent）：review，跑全量，`docs/design.md` 的 `cli` 一节加路由 B，更新 README，在 tmux 里运行示例，补本记录的结果。

## 结果

Codex 写了 `macros/src/fs_commands.rs` 的 `commands!`、`src/cli/mod.rs` 的再导出和 rustdoc、示例包 `examples/cli-routes/` 和测试：宏 crate 2 个单元测试（名字转换、报错规则表），示例包 3 个集成测试。报告没有发现 ink-rs 的缺口。

主 agent 接受 worker 的以下偏离：

- `Commands` derive 的注册函数从 `commands` 改名 `derive_commands`，给函数式宏让出名字；公开的 derive 名不变。
- Rust 2024 的关键字 `gen` 用原始标识符。
- 生成项的文档写成 `#[doc = concat!(..)]`，clap 不把它当成命令描述。
- 元数据测试写 `<Cli as CommandFactory>::command()`：`Props` 生成的 `command` setter 和 trait 方法同名。

worker 报告的行为：

- 不带 index 的组不带子命令时，clap 报 `DisplayHelpOnMissingArgumentOrSubcommand`，打印 help，退出码 2。
- 删掉示例的 build.rs 后编译报 `src/commands: OUT_DIR is unset; add a build.rs containing ...`，proc macro 展开时读得到 `OUT_DIR`。这个检查确认包有 build script，确认不了它输出了 `rerun-if-changed`。

主 agent review 时把 `clap` 加进保留的文件名：生成的模块里有 `use ::eink_rs::cli::clap;`，`clap.rs` 会和它冲突。单元测试和 rustdoc 同步改。集成时 `cargo test --workspace --all-features` 122 个通过，clippy、fmt、无 feature 的 build 通过。`docs/design.md` 加「路由 B：目录即命令」一节，README 加示例的运行命令。

tmux 会话 `eink-rs` 里运行 `cli-routes`：

- `--help` 列出 deploy（别名 d）、domains（别名 dom）、team，退出码 0。
- 不带命令时显示 `Deployment console` 和 `Deploy website to us`；`d docs --region eu --preview` 显示 `Deploy docs to eu (preview)`。
- `domains --help` 列出 index 的 `--verbose` 和 add、list 两个子命令；`dom --verbose` 渲染 index；`domains list`、`domains add example.org` 渲染子命令。
- `team` 打印 help，退出码 2；`team list`、`team projects list` 正常。
- 新增 `src/commands/status.rs` 后 `cargo build`，`--help` 出现 status，`status` 正常渲染；删掉后重编，`status` 报未知子命令。

两个环境问题，与本仓库代码无关：

- 本机 fish 里的 `cargo` 经 mise 包装，rustc 外面套着编译缓存 `mbx-rustc`。新增命令文件后 cargo 判定要重编，缓存按上次的源文件命中，产物里没有新命令。rustup 的 cargo 正常。其他按源文件清单缓存 rustc 产物的工具可能有同样的问题，未验证。
- 一次经这个包装的 `cargo run -p cli-routes` 渲染后没有退出。直接运行二进制 34 次、经包装的 `cargo run` 6 次，进程正常退出，没有复现。
