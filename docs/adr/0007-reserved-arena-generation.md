# ADR-0007: reserve the arena's terminal generation

Date: 2026-09-05
Status: **Accepted**

## Context

ADR-0006 Decision 1 retired an arena slot only when incrementing its generation
would overflow `u32`. That permits issuing generation `u32::MAX`. The first
deserialization guard used that same value as the unambiguous marker for a
retired slot, so a live slot at `u32::MAX` could serialize to a shape its own
loader rejected. Making the loader accept both meanings would weaken the
invariant and make malformed free-list state harder to detect.

## Decision

`u32::MAX` is reserved as a tombstone and is never issued in an `EntityId`.
`u32::MAX - 1` is the last issuable generation. Removing an entry at that
generation retires its slot permanently by setting the slot generation to
`u32::MAX`, leaving it vacant, and not adding it to the free list.

This supersedes only ADR-0006 Decision 1's generation-exhaustion boundary. The
arena's ownership, ordering, reuse, serialization, and stale-id decisions are
unchanged.

## Consequences

- Live allocation and deserialization use one meaning for `u32::MAX`: retired.
- Every arena can serialize to a representation accepted by its own loader.
- One generation out of 2^32 is unavailable, after more than four billion
  allocations of the same slot.
- Deserializing an `EntityId` with generation 0 or `u32::MAX` fails.

## Rejected

Issuing `u32::MAX` and adding separate retirement state would preserve one more
generation but enlarge the serialized format and allow two representations of
the same terminal condition. Allowing a live ID and a tombstone to share the
same generation would make the loader's consistency checks ambiguous.
