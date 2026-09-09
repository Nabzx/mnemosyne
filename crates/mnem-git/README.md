# mnem-git

The `mnem` command-line tool: version control for AI agent memory, part of
[Mnemosyne](https://github.com/Nabzx/mnemosyne).

An AI agent builds up memory as it works. `mnem` gives that memory what Git
gives code: an immutable, content-addressed history you can branch, merge,
`blame` and `bisect`. Local, deterministic, no network, no model calls.

```bash
cargo install mnem-git   # installs the `mnem` binary

mnem init ./agent-memory
mnem add customer-4821 "on the Enterprise plan" --source ticket-4821
mnem commit -m "open the case" --author agent
mnem blame customer-4821      # which commit set this, and the observation behind it
mnem bisect --node customer-4821 --equals '"downgraded to Pro"'
```

The crate is named `mnem-git`; the installed binary is `mnem`. The library
core is [`mnem-store`](https://crates.io/crates/mnem-store); the Python
SDK is `mnem-agents` on PyPI.

Licence: Apache-2.0.
