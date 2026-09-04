# ADR-0004: QUIC movement spike result — T2

Date: 2026-09-03 (NAT leg closed 2026-09-04)
Status: Accepted. Both of T2's test criteria have now been run (see
"NAT leg result" below); one flagged risk remains open (see Consequences).

## Context
`CRPG_ENGINE_SPEC.md` §7.2/§24 (task T2) picks `quinn` (QUIC) as the
transport for authoritative movement with client-side prediction of the
local player only (§7.5), and asks specifically for two things: (1) smooth
local movement at 150 ms RTT / 3% loss with non-visible reconciliation, and
(2) a connection succeeding over the public internet through a home
router's NAT with no manual port forwarding.

## What was built
A throwaway spike in `C:\CRPG\Dev\spike-quic` (separate repo, no dependency
on the `crpg` workspace, per spec "Affected: throwaway repo"):
- `server` (bin): 20 Hz authoritative loop (§2.5). Receives
  `MoveTo { seq, target }`-shaped client intents as QUIC datagrams, moves a
  single tracked position toward the latest (highest-`seq`) target at a
  fixed speed, sends back `{ last_processed_seq, server_tick, pos }` every
  tick, also as a datagram.
- `client` (bin): every 50 ms, predicts locally with the *same* deterministic
  step function the server uses, buffers `(seq, target, predicted_pos)`,
  sends the intent, and on each server ack snaps to the authoritative
  position for that `seq` and replays buffered inputs newer than it —
  standard client prediction/reconciliation (§7.5). A scripted waypoint tour
  stands in for real input so the run is reproducible headless.
- `netshim` (bin): a protocol-agnostic UDP relay (works below TLS/QUIC,
  since QUIC is just UDP payloads) that delays, jitters, and drops packets
  independently per direction — lets the 150 ms RTT / 3% loss condition be
  produced on one machine without a real bad network.
- Self-signed TLS via `rcgen`, trusted by the client via an exported cert
  DER file (dev-only; no CA, no revocation — fine for a spike, wrong for
  anything real).

## Measured (this machine, loopback, release build)

| Condition (one-way delay / jitter / loss) | ticks sent | acks received | mean divergence | max divergence | visible corrections (>0.5 units) |
|---|---|---|---|---|---|
| direct, no shim | 301 | 286 (95.0%) | 0.013 | 0.30 | 0 |
| shim 75ms / 0 / 0 | 301 | 291 (96.7%) | 0.003 | 0.30 | 0 |
| shim 75ms / ±5ms / 0 | 301 | 291 (96.7%) | 0.006 | 0.30 | 0 |
| shim 75ms / ±15ms / 0 | 301 | 222 (**73.8%**) | 0.081 | 0.30 | 0 |
| **shim 75ms / ±15ms / 3%** (≈150ms RTT, spec's condition) | 401 | 278 (69.3%) | 0.099 | 0.60 | 3 (1.1%) |
| control: raw UDP through the identical ±15ms-jitter shim, no QUIC | 300 sent | 300 (**100%**) | n/a | n/a | n/a |

Divergence is the distance between what the client had predicted for an
acked `seq` and what the server actually says, i.e. the size of the
correction reconciliation has to make. "Visible" (>0.5 world-units, versus
a max per-tick travel of 0.3 units at this speed/tick-rate) is a proxy for
a jump a player would notice through interpolation smoothing.

## The finding that matters more than the headline number
At ±5ms jitter, loss stays at the ~3% noise floor. At ±15ms jitter (still a
realistic WiFi/home-router figure, and well under the spec's own 150ms RTT
target), **loss jumps to ~26–30% with zero packets actually dropped** — the
control row proves this: an identical raw-UDP stream through the exact same
shim settings delivered 300/300. The shim is not losing packets; something
downstream of it is.

That something is `quinn-proto`'s packet dedup window: a 129-packet-wide
bitmask that rejects any packet arriving more than 129 packet-numbers
behind the highest one already authenticated, logging "discarding possible
duplicate packet" and dropping it — including any DATAGRAM frame inside it
— even when the packet is genuinely new, just reordered. This is a real,
open, currently-unfixed-upstream defect:
[quinn-rs/quinn#2710](https://github.com/quinn-rs/quinn/issues/2710),
filed and closed 2026-07-05 against **quinn 0.11.11 / quinn-proto
0.11.15 — the exact versions this spike is pinned to**. The reporter
measured the same signature (network-level proof of zero loss, `~56%`
"lost" per QUIC's own stats) on a WAN path and posted a tested fix (widen
the window to 2048 bits); the fix lives only in their private fork — closed
upstream as "not pursuing this upstream," not merged into quinn.

I have not proven the packet-number arithmetic lines up exactly at this
spike's low send rate (~20–40 pkt/s) the way it does at the WAN/Gbit rate in
that report — the window is sized in packet numbers, not milliseconds, and
129 packets at 30/s is nominally ~4s of headroom. Something is consuming
packet-number space faster than the app-visible datagram rate suggests
(ACK-only packets, retransmits from the very loss this causes feeding back
on itself, or similar) — worth understanding before this is load-bearing,
but the reproduction is solid enough (sharp, deterministic-feeling jitter
threshold; ruled out at the socket/OS layer by the control test) that a
known, matching, version-exact upstream defect is the more credible
explanation than an unrelated bug in ~250 lines of spike code.

## Decision
**Conditional go.** The reconciliation *algorithm* is validated: even at a
measured ~30% effective datagram loss, divergence stayed small (mean 0.1
units, 1.1% of acks producing a correction big enough to call "visible")
and did not compound or diverge — §7.5's design works. Proceed with QUIC /
`quinn` and the predict-and-reconcile design as specified.

But do not wire `crpg-net`'s snapshot channel straight to `quinn`'s
`send_datagram`/`read_datagram` on the current pinned version without
addressing #2710 first — real paths (WiFi, cellular, most home routers)
routinely reorder by more than 15ms, and this spike shows quinn silently
manufactures loss well beyond the path's actual loss rate under exactly
that condition. Before `crpg-net` depends on this: track upstream for a
fix, evaluate carrying a small patch (widening `Window` in
`quinn-proto`'s dedup code is a ~20-line, self-contained change per the
issue), or benchmark whether reliable-unordered delivery (a stream per
snapshot, or a lower send rate) sidesteps it. This is exactly the kind of
patch the spec's "pinned Godot + small patch queue" philosophy (§1) already
budgets for — it just turns out `quinn`, not Godot, is where the first one
is needed.

## NAT leg result (T002b, run 2026-09-04)

Run manually (human-executed, not agent-executable — see `tasks/T002b.md`)
across two real, differently-owned networks:

- **Setup A:** `server.exe` on a laptop tethered to a phone's mobile
  hotspot (cellular network, no admin access to the carrier's NAT). Client
  on a separate network. Result: client printed `[client] connecting to
  <addr>:5000 (SNI localhost)` then errored with a connect timeout; server
  logged no `[server] connection from ...` line at all — the handshake
  never reached it.
- **Setup B (the scenario §24/T002b actually specifies):** `server.exe` on
  a laptop connected to a normal home broadband router, **no manual port
  forward, no DMZ**. Client on a separate network (phone hotspot). Same
  result: client-side connect timeout, no inbound connection logged on the
  server.

Two local causes were checked and ruled out before attributing this to the
network:

- **Windows Firewall on the server host.** `Get-NetFirewallRule` showed an
  existing explicit inbound `Allow` rule scoped to exactly
  `...\spike-quic\target\release\server.exe`, UDP, port 5000.
  `Get-NetConnectionProfile` confirmed the active network's category
  (`Public`) matched the rule's profile in both setups. The firewall was
  not blocking this traffic.
- **UPnP.** The home router in Setup B had UPnP enabled (checked in its
  admin UI). The connection failed anyway. This is expected, not
  contradictory: UPnP only opens a mapping if an application explicitly
  requests one via the IGD protocol, and this spike's `server.rs` does no
  such thing — it only binds and listens (see `src/bin/server.rs`).
  A router capable of UPnP does nothing by itself.

**Conclusion:** the base case — an unmodified home router, zero manual
configuration — fails to accept an inbound QUIC connection, exactly as
spec §7.7 anticipated when it deferred "NAT punching" as future scope
rather than assumed-free behaviour. This is not a defect in the spike; it
is the NAT layer doing what NAT layers do absent a port mapping. Because
UPnP was confirmed present and unused, the fix is squarely *application*
work (have the real server request a UPnP/IGD mapping on startup, e.g. via
the `igd` crate) or *operational* (document that self-hosted servers need
a manual port forward) — not a router-configuration gap, and not the
harder "NAT punching/relay" case described in §7.7 for stricter NAT types
(e.g. CGNAT, which Setup A's cellular network likely also involves, though
that wasn't independently isolated from the general "no mapping exists"
result).

The NAT leg of T002 is now closed. T002b's outcome is: **fails without
manual configuration, cause fully attributed to the NAT layer (not local
firewall, not absence of router UPnP capability), with a known and scoped
future fix.**

## Consequences
- The prediction/reconciliation implementation in this spike
  (`src/bin/client.rs`) is a reasonable reference for the real
  `crpg-client-bridge` — ring buffer of buffered inputs, snap-and-replay on
  ack — and can be lifted forward.
- `crpg-net`'s eventual channel design (§7.2's `snapshot` = unreliable
  datagram) needs a decision on #2710 before it is trusted at anything but
  loopback-quality reordering. Track it as a named risk, not a footnote.
- The NAT half of T2 is closed (see "NAT leg result" above). `crpg-net`'s
  server needs either a UPnP/IGD client (e.g. the `igd` crate) to
  request a port mapping automatically on startup, or documented
  operator instructions for a manual port forward — plain "bind and
  listen" (this spike's approach) will not be reachable from outside the
  LAN on a default home router. True NAT punching/relay for stricter NAT
  types (symmetric NAT, CGNAT) remains deferred per §7.7 and was not
  proven necessary or sufficient by this test.

## Alternatives rejected
None — no need to escalate away from QUIC/`quinn`; the defect found is
real but has a known, scoped, previously-implemented fix, which is a much
smaller problem than switching transports.
