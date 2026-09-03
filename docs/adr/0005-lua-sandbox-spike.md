# ADR-0005: Lua sandbox spike result — T3

Date: 2026-09-03
Status: Accepted — go

## Context
`CRPG_ENGINE_SPEC.md` §5.4/§24 (task T3) picks Lua 5.4 via `mlua` for
campaign event handlers and states plainly that "the sandbox is not
optional." The spec's required properties: no `io`, `os` (except a
whitelisted `os.time` returning sim time), `require`, `dofile`, `load`
(5.1-era `loadstring`), `debug`, `package`, or raw FFI; `math.random`
replaced by the deterministic sim RNG; `pairs` replaced with a
deterministic ordered iterator; an instruction-count hook with a memory
ceiling; no coroutine yields across tick boundaries. Nothing in
`crpg-script` should be built until this is proven to actually hold.

## What was built
A throwaway spike in `C:\CRPG\Dev\spike-lua-sandbox` (separate repo, no
dependency on the `crpg` workspace, per spec "Affected: throwaway repo"):

- `src/sandbox.rs`: the sandbox construction under test, written to be
  small enough to copy into `crpg-script` largely as-is.
- `src/main.rs`: a manual runner that prints the base-library surface and
  every fixture's pass/fail, for eyeballing.
- `fixtures/*.lua`: ten escape-attempt scripts, one infinite-loop script,
  one determinism script.
- `tests/sandbox.rs`: seven `cargo test` cases, one per Definition-of-done
  criterion — nothing here is eyeballed.

`mlua` version: `0.12.1`, features `lua54, vendored`.

## Sandbox construction
- `Lua::new_with(StdLib::TABLE | StdLib::STRING | StdLib::MATH, ..)` — `io`,
  `os`, `package`, `debug`, and `coroutine` are never compiled into the
  state at all. `new_with`'s own safe mode actively *refuses* to load
  `debug` even if asked (`Error::SafetyError`), which is a stronger
  guarantee than post-hoc stripping.
- An instruction-count hook (`every_nth_instruction`, granularity 200)
  aborts a script once a running total exceeds a configured budget.
- `Lua::set_memory_limit` caps per-sandbox allocation.
- `math.random`/`math.randomseed` are replaced with a splitmix64 stream
  seeded from `(tick, entity, call_index)`. `randomseed` is a deliberate
  no-op — the stream is host-controlled per invocation, not
  script-controlled, which is what determinism/replay actually requires.
- `pairs` is replaced with a version that sorts keys into a total order
  (by type, then value) before returning an iterator, so iteration order
  can't depend on Lua's internal hash-table layout.
- A synthetic `os` table is installed with exactly one member, `time`,
  returning the host-supplied sim tick — spec's one whitelisted exception.

## The finding that matters more than the checklist
**`Lua::new_with`'s `StdLib` bitset does not gate the base library, and
the base library is always loaded.** `mlua` unconditionally calls
`luaopen_base` for `_G` regardless of what's passed in — there is no flag
to suppress it. Lua 5.4's base library includes `load`, `loadfile`, and
`dofile`, so requesting `StdLib::TABLE | StdLib::STRING | StdLib::MATH`
alone leaves all three globally callable — confirmed empirically (see
`src/main.rs`'s "base-library surface" printout, reproduced below):

```
present without stripping: load       -> true
present without stripping: loadfile   -> true
present without stripping: dofile     -> true
present without stripping: require    -> false   (package lib never loaded)
present without stripping: os/io/debug/package -> false
```

`require` is absent by construction (it's defined by the `package`
library, which genuinely is gated by the bitset), but `load`/`loadfile`/
`dofile` are not — they must be nilled out of `globals()` by hand after
construction. This is exactly what spec §5.4 lists as required removals,
but it's easy to assume `new_with`'s library selection handles it and skip
the explicit strip. `sandbox.rs`'s `BANNED_BASE_GLOBALS` list is that
strip; skipping it would have been a real, silent hole.

## Test results
All ten escape attempts and both budget tests pass; `cargo test` is green
(7/7):

| Fixture | Attempt | Result |
|---|---|---|
| `escape_01_io_open` | `io.open(...)` | blocked — `io` doesn't exist |
| `escape_02_os_execute` | `os.execute(...)` | blocked — not on the synthetic `os` table |
| `escape_03_require` | `require("os")` | blocked — `package` never loaded |
| `escape_04_load_string` | `load("return 1+1")()` | blocked — stripped from base lib |
| `escape_05_debug_library` | `debug.getinfo(1)` | blocked — `debug` never loaded |
| `escape_06_metatable_pivot` | `getmetatable("").__index`, then `string.dump` on a host-provided Rust closure | pivot succeeds (expected — string methods are legitimate), but `string.dump` on the host closure fails: it isn't a Lua closure, so there's no bytecode to extract |
| `escape_07_global_audit` | walk `_G` and the `os` table looking for anything not on the allowlist | nothing found |
| `escape_08_coroutine` | `coroutine.create(...)` | blocked — coroutine library never loaded, so there is nothing to suspend a `Wait` with in the first place |
| `escape_09_memory_bomb` | unbounded table of 1 KiB strings against a 2 MiB ceiling | `MemoryError("not enough memory")` |
| `escape_10_tamper_no_persist` | script overwrites its own `math.random`/`os.time` | runs clean (harmless — scoped to the one Lua state); a freshly constructed sandbox in the same process is unaffected, confirmed from the Rust side |
| `infinite_loop` | `while true do end` against a 50,000-instruction budget | aborts in <2s wall-clock, does not hang |
| `determinism` | `pairs` over a mixed string/int-keyed table + 5×`math.random` + `os.time`, same seed twice | byte-identical trace both times; a different seed changes the trace |

## Decision
**Go.** The sandbox construction holds against all ten scripted escape
attempts, the instruction budget bounds an infinite loop, the memory
ceiling bounds unbounded allocation, and the determinism substitutions
(`pairs`, `math.random`) are reproducible and seed-sensitive. `sandbox.rs`
is small (~180 lines) and framework-free enough to move into `crpg-script`
with minimal changes.

## Consequences
- When `crpg-script` is built, carry forward `sandbox.rs`'s structure
  directly: `Lua::new_with` restricted to `TABLE | STRING | MATH`
  (add others only as specific abilities need them, e.g. `UTF8`), explicit
  post-construction stripping of `load`/`loadfile`/`dofile`/`require`
  (**do not assume the `StdLib` bitset covers this** — it doesn't), the
  instruction hook, the memory limit, and the deterministic `pairs`/
  `math.random` substitutions.
- The instruction-budget granularity (200) and default budget
  (100,000/invocation in this spike) are placeholders; real values need
  tuning against actual event-handler script sizes once `crpg-script`
  exists, per §5.2's "budgeted... exceeding it aborts the graph."
- `math.randomseed` is a no-op in this design — if a future ruleset genuinely
  needs script-visible reseeding, that has to be a host-mediated API, not a
  restored passthrough, or replay determinism breaks.
- Not exercised here: the async/`Wait`-node interaction (§5.2's requirement
  that long waits use the serializable IR node, never a suspended
  coroutine) — this spike's answer is structural (coroutine library isn't
  loaded, so nothing to suspend with), not a runtime policy tested against
  a real event-graph interpreter, because that interpreter doesn't exist
  yet. Re-verify the interaction once `crpg-script`/the event IR (§5.2) are
  built.

## Alternatives rejected
None — no need to escalate away from `mlua`/Lua 5.4; the one gap found
(base-library `load`/`loadfile`/`dofile`) has a one-line-per-global fix,
not a reason to reconsider the language choice.
