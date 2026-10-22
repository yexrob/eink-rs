# Follow-up: the two gaps are decided

The user decided the two gaps in your report. Everything else in the brief stands. Continue with the whole task, and end with the report the brief asks for.

## 1. `Static::from_fn`

The user chose to add it to ink-rs («ink-rs 加 Static::from_fn»). It is in the local ink-rs at `~/Episodes/Projects/bingo-inc/ink-rs`, `src/components.rs`. It is not released yet and will ship as 0.1.5:

```rust
/// `len` items rendered by index, for items the app keeps in storage of
/// its own, such as an `Rc<RefCell<Vec<T>>>`: a growing history needs no
/// copy per render. Only items past those already written are rendered.
pub fn from_fn(len: usize, render: impl Fn(usize) -> Element + 'static) -> Static
```

- `Static::new` builds through it.
- The worktree's `Cargo.toml` has a `[patch.crates-io]` entry pointing `ink-rs` at that path.
  - The main agent added it, and will replace it with the 0.1.5 requirement at integration.
  - Do not change it, and do not change anything in ink-rs.
- The demo and the hook test keep their history in storage of their own, and render it through `Static::from_fn` with no per-frame copy.

## 2. The sender takes the caller's `AppHandle`

The user chose «sender 接收调用方的 AppHandle»:

- **The signature** is `stream.sender(app) -> MarkdownSender`. Pick whether `app` is `AppHandle` or `&AppHandle`, and report the choice.
- **The handle the hook returns holds no `AppHandle`.**
- **Waking the UI for highlight results.** The stream borrows the handle of the work in flight, and drops it once the result is delivered:
  - lines completed by a push through a sender carry the sender's handle;
  - lines revealed by a paced tick carry a handle the hook takes with `cx.use_app()` during render. The running clock keeps the app alive meanwhile.
- **No idle handle.** Nothing holds an `AppHandle` while no work is pending, as the brief requires.
