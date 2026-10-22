# 发布 0.3.0

- 日期：2026-10-22
- 状态：进行中
- 提交：
- Brief：无，主 agent 自己做

## 背景

[024](024-bingo-research.md) 把下一版定为 0.2.2（「不拆 tokio，发 0.2.2 (Recommended)」），依据是 v0.2.1 之后 `src` 的改动全是新增。之后 [025](025-list-fence.md)、[029](029-stream-held-gap.md) 和 [026](026-controls.md) 合入 main，[027](027-composer.md) 和 [028](028-code.md) 在分支上做完。`Composer` 用 ink-rs 的 `use_node_cursor`，`Code` 用 `use_color_level`，两者等 ink-rs 0.1.7（ink-rs 的记录 076）。

主 agent 问这一版带哪些内容，用户选「带上控件、Composer、Code (Recommended)」。核对公开 API 时主 agent 发现 026 给字段全公开的结构体加了字段：`SelectOption` 的 `description`，`Select` 的 `id`、`auto_focus`、`value`、`number_hotkeys`、`on_select`，`TextInput` 的 `id`、`auto_focus`、`value`。按 Cargo 的 semver 规则这是破坏性改动：写全字段的结构体字面量编译失败。主 agent 问版本号，用户选「发 0.3.0 (Recommended)」。

## 决策

- 用户定：
  - 「带上控件、Composer、Code (Recommended)」：这一版带 Markdown（016、018、021、025、029）、控件（026）、`Composer`（027）、`Code`（028）。
  - 「发 0.3.0 (Recommended)」：版本号 0.3.0。
  - 发 ink-rs 0.1.7 时用户选「发布并推送 (Recommended)」：推送 ink-rs 和 eink-rs 的 main。
- 以下由 Claude 按用户的授权定：
  - 只发 eink-rs 0.3.0。v0.2.1 之后 `macros/` 没有改动，eink-rs-macros 留在 0.2.0。
  - 依赖 ink-rs 0.1.7。依赖升级单独一个提交，排在 `Composer` 和 `Code` 之前，两个分支 rebase 到它之上再合入。
  - [023](023-image.md) 的图片不进这一版：Markdown 接入在做，终端里的核对没做。
  - 顺序同 [014](014-release-0-2-1.md)：本地全量、推送 main、等 CI 通过、打 `v0.3.0` 标签、`cargo publish -p eink-rs`。不建 GitHub Release。

## 备选与取舍

- **照发 0.2.2**：依赖 `"0.2"` 的下游 `cargo update` 后，写全字段的字面量编译失败。
- **0.2.2 不带控件**：bingo 要的控件晚一版；破坏性改动留到下一版，问题不变。
- **给这些结构体加 `#[non_exhaustive]`**：它本身是破坏性改动，并且禁止下游用字面量构造组件的属性。

## 方案

主 agent 自己执行：

1. 本记录先提交。
2. ink-rs 0.1.7 发布后，依赖改为 `0.1.7`，提交。
3. `wt/composer` 和 `wt/code` rebase 到它之上，合入 main；补 027、028 的结果。
4. 本地全量：fmt；clippy（`--all-features`、无 feature、`markdown`）；`--workspace --all-features` 和无 feature 的测试；`cargo test --features markdown --test markdown`；`cargo doc`；`cargo package -p eink-rs` 检查打包的文件和编译。
5. 升版本号到 0.3.0，提交 `chore: release 0.3.0`。
6. 推送 main，等 CI，打 `v0.3.0` 标签并推送，`cargo publish -p eink-rs`。
7. 告诉 bingo 会话把依赖改为 `"0.3"`。

## 结果
