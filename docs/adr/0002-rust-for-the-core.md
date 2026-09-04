# ADR-0002: Rust for everything below the presentation layer

Date: 2026-09-03
Status: Accepted

## Context
Most of this project's code will be written by AI agents supervised by one
person. The core is a deterministic simulation: rules resolution, world state,
serialization, and networking. It must run headless on a server with no
graphics, be replay-deterministic, and be decomposable into subsystems that
agents can work on without breaking each other.

## Decision
Rust for the simulation core, server, CLI, editor document model, and the
GDExtension bridge. GDScript only for client and editor view code.

## Consequences
- The compiler rejects a whole class of agent mistakes before review.
- Cargo crates give compile-enforced subsystem boundaries and per-crate tests.
- serde and schemars generate the campaign format and its JSON Schema from
  one set of type definitions.
- No garbage collector, so the server tick has no pause risk.
- Cost: learning curve, and godot-rust is a third-party binding. Mitigated by
  keeping the FFI surface small.

## Alternatives rejected
- C++: matches Godot, but a poor substrate for agents (huge context, slow
  builds, silent memory bugs).
- C#: faster to write, but GC pauses on the server and a weaker determinism
  story. Acceptable fallback if Rust proves unworkable in practice.
- GDScript: structurally impossible. The server has no Godot, so a rule
  written in GDScript cannot run authoritatively.
