# CRPG engine — agent rules

Rust workspace. Simulation core has NO game-engine dependency.

## Non-negotiable
- Only `crpg-godot` may depend on `godot`. Only `crpg-godot` may use `unsafe`.
- Dependency direction: core <- data <- rules <- sim <- {net, ai, script} <- server.
  Never import upward. Never create a cycle.
- No `HashMap` iteration and no `f32`/`f64` in `crpg-rules` or `crpg-sim` rules paths.
  Use `IndexMap`/`BTreeMap` and integers or fixed-point.
- Do not add dependencies without being asked.
- Do not modify `crpg-contracts` or `rust-toolchain.toml`.
- Do not weaken or delete an existing test to make a build pass. Stop and say so.

## Working rules
- One task = one crate. If a task needs two crates, stop and say so.
- Finish only when the task file's stated command passes.
- Before finishing: `cargo fmt --all`, `cargo clippy -p <crate> -- -D warnings`,
  `cargo test -p <crate>`.

## Note
- "Godot4" is available in PATH CLI
