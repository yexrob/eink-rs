# CLI 框架：路由 B 调研

- 日期：2026-10-21
- 状态：实施中
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
