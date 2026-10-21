This repository builds on ink-rs (https://github.com/yexrob/ink-rs) and holds two parts: a port of Ink UI's components, and a CLI framework modeled on Pastel.

1. Unsafe code is forbidden. If it's necessary, request permission to use it.
2. Don't write junk code to cater to the test, and do a good job of architecture design.
3. Agents may commit in small steps without asking. Each commit holds one logical change that builds and passes the tests covering it, with a Conventional Commits message. Stage your own files by path, since other agents share the working tree. Pushing needs the user's approval.
4. Record the basis of every piece of work before doing it. Before you write code or dispatch a worker, add a record to `docs/decisions/` with the decision, who made it, the options weighed and the plan (format in `docs/decisions/README.md`), and copy each worker brief verbatim into `docs/decisions/briefs/` with local paths replaced. Commit the record with the change or before it, and fill in its result when the work lands.
5. When a feature needs something ink-rs lacks, stop and ask. General primitives belong in ink-rs; don't work around the gap here.
