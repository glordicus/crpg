# CRPG engine — agent rules

Rust workspace. Simulation core has NO game-engine dependency.

## Non-negotiable
- Only `crpg-godot` may depend on `godot`. Only `crpg-godot` may use `unsafe`;
  every other crate root carries `#![forbid(unsafe_code)]` and `deps.py` fails
  if one does not.
- Dependency direction: core <- data <- rules <- sim <- {net, ai, script} <- server.
  Never import upward. Never create a cycle. A dev-dependency counts as an
  import: `[dependencies]`, `[dev-dependencies]` and `[build-dependencies]` are
  all checked.
- No `HashMap`/`HashSet` in `crpg-core`, `crpg-rules` or `crpg-sim` — use
  `IndexMap`/`BTreeMap`.
- No `f32`/`f64` in `crpg-core` or `crpg-rules` — use integers or `Fx16_16`.
  `crpg-sim` may use `f32` for spatial positions only (spec §2.4), never in a
  rules path.
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
