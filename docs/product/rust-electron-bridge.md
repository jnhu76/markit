# Rust ↔ Electron Bridge Decision

Status: product decision note (foundation campaign Phase E, slice
`product/electron-ui-1`). Short by policy — architecture stays frozen;
this records one bounded implementation choice.

## The choice

Electron is the product shell (`apps/markit-desktop`). The Rust domain
core (markit-composition + document/filesystem/markdown crates) is
reached through **option B: a Rust sidecar process with a narrow
newline-delimited JSON transport over stdio**, typed by serde against the
frozen domain faces (document commands, pinned reads, semantic status).

## The bounded check (only the two realistic minimal options were compared)

Option A — N-API/native addon loaded by Electron main.
Option B — Rust sidecar, narrow stdio transport.

Not considered (per campaign): HTTP server, WebSocket, gRPC, generic
JSON-RPC framework, MCP.

| Criterion (frozen list) | A: N-API addon | B: sidecar + stdio |
|---|---|---|
| cross-platform packaging | addon must be built per Node ABI per platform; packaging couples the Rust workspace to Electron release cadence | one standalone binary per platform, copied into the app bundle |
| development friction | napi build rig in the product workspace; every domain change rebuilds the ABI layer | `cargo build -p <bin>`; transport is plain stdin/stdout |
| lifecycle/shutdown correctness | addon lives in the main process; teardown shares the Node event loop | explicit spawn/handshake/quit; orphan risk handled by parent-death close of the pipe |
| failure isolation | a Rust panic aborts the main process | sidecar crash is contained; the shell surfaces an explicit bridge failure and can respawn |
| typed surface | identical (serde-generated types over the same contracts) | identical |
| debugging | mixed into the Electron process | separate process, plain logs on stderr |
| native dependency burden | Node headers/ABI per version | none beyond the toolchain |
| incremental-build burden | ABI crate rebuilds couple into every domain edit | none: the sidecar binary is a leaf |
| security boundary | in-process: one memory-safety domain | process boundary: the renderer reaches the domain only through main + a narrow command set |

Nothing here is a performance claim; foundation measures no latency. A
rejected option may be revisited only through a concrete, documented
product blocker (same bar as the GPUI policy).

## Consequences

1. The transport carries the SAME typed commands the Rust domain defines;
   it is not a second authority and defines no semantics. Wire types live
   with the bridge and mirror the Rust contracts.
2. The renderer never speaks to the sidecar directly: main owns the child
   process; the preload exposes domain-shaped commands only.
3. The UI-track fake adapter (`src/renderer/adapter/fake.ts`) exists only
   until this bridge lands; the integration slice deletes it.
4. Shutdown: closing the window drops the stdio pipes; the sidecar exits
   on EOF. The sidecar performs no unsaved-file policy — save is always
   an explicit document command.
