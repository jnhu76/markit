# Markit — Plugin Compatibility Contract

Status: **product design contract / transport and runtime remain evidence-driven**.

This document defines the compatibility boundary between Markit and future
plugins/extensions. It exists before the plugin runtime itself so that the
editor core, Markdown IR, rendering model, and scheduler do not accidentally
become the public plugin API.

The governing rule is:

> **A Markit update must not require every plugin to be rebuilt merely because
> Markit's internal implementation changed.**

The companion rule is:

> **Plugins depend on a versioned semantic contract, never on Markit internals.**

This is specifically intended to avoid the failure mode where a host update
changes internal types, paths, runtime assumptions, or bundled dependencies and
silently breaks otherwise unrelated plugins.

## 1. What is stable, and what is not

The following are **not** plugin API unless a future compatibility document
explicitly promotes them:

```text
markit-core internal Rust structs
Document storage representation (String / Rope / Piece Table / tree)
Markdown parser implementation details
Markdown IR concrete Rust layout
GPUI Entity / Element types
scheduler queues / task types
cache entries / cache keys
layout/shaping internals
private filesystem paths
private database/storage schemas
```

Plugins must not rely on those details.

The stable surface is a small semantic API built from concepts such as:

```text
DocumentSnapshot
DocumentId
DocumentRevision
StableBlockId
SourceRange
public Markdown semantic nodes / views
commands
workspace/document events
export/print requests
read-only queries
explicit capabilities
versioned settings/schema
```

The exact Rust, IPC, Wasm, or other transport representation is deliberately
**not frozen yet**. The semantic contract must survive a transport change.

## 2. Compatibility architecture

```text
                    Markit internals

 Document / Markdown IR / Scheduler / GPUI / Caches
                         │
                         │ private adapters
                         ▼
              ┌──────────────────────┐
              │ Plugin API Boundary  │
              │                      │
              │ versioned contract   │
              │ capability checks    │
              │ stable identifiers   │
              │ snapshots / commands │
              │ events               │
              └──────────┬───────────┘
                         │
                 compatibility layer
                         │
         ┌───────────────┼────────────────┐
         ▼               ▼                ▼
      Print/PDF       Exporter          Lint
      plugin          plugin            plugin
```

The adapter between internals and the Plugin API Boundary is Markit's
responsibility. Internal refactors may require adapter changes, but should not
force compatible plugins to change.

## 3. Version negotiation is mandatory

A plugin is never loaded by assuming that host and plugin versions match.
Loading begins with compatibility negotiation.

Conceptually:

```text
HostHello
  host_version
  plugin_api_major
  plugin_api_minor
  capabilities[]

PluginManifest
  plugin_version
  supported_api_major
  minimum_api_minor
  required_capabilities[]
  optional_capabilities[]
```

The concrete encoding is open. The semantics are not.

Rules:

1. **Major mismatch is explicit incompatibility.** The host refuses to load the
   plugin with a human-readable reason instead of crashing or partially loading.
2. **Minor evolution is additive by default.** New optional fields,
   capabilities, commands, events, or methods must not invalidate older clients.
3. **Capability negotiation beats version guessing.** A plugin asks whether a
   feature exists rather than inferring behavior from `host_version >= X` where
   practical.
4. Unknown optional fields/capabilities are ignored safely.
5. Required capabilities must fail closed and explain what is missing.

## 4. Compatibility promise

Once the public plugin API reaches its first stable release, Markit follows
these rules within one API major version:

- existing public operations keep their documented semantics;
- additive optional functionality does not require recompilation/rewrite of
  unrelated plugins at the semantic protocol level;
- deprecated operations remain available for a documented migration window;
- removal or incompatible semantic change requires a plugin API major version
  change;
- the host may ship compatibility adapters for older API majors when the cost is
  reasonable and measured;
- host startup must identify incompatible plugins before they can mutate editor
  state.

Before the first stable plugin API release, experimental APIs must be labeled
`experimental` and must not be presented as compatibility-stable.

## 5. Deprecation and migration

A public capability is not removed in one step.

```text
supported
   ↓
deprecated + replacement documented
   ↓
compatibility window
   ↓
telemetry/tests show migration path works
   ↓
removal only at an allowed compatibility boundary
```

Every deprecation must record:

- what is deprecated;
- replacement API/capability;
- first deprecated host/API version;
- earliest allowed removal boundary;
- migration example;
- compatibility test covering the old behavior during the support window.

Do not silently reinterpret an old operation to mean something materially
new.

## 6. Stable identity, not internal pointers

Plugins must never retain references to mutable internal Markit objects.
Cross-boundary identity uses stable opaque identifiers and revisions.

Prefer:

```text
DocumentId
StableBlockId
DocumentRevision
SourceRange
SnapshotId
```

Avoid exposing:

```text
raw pointers
GPUI Entity ids as document identity
Vec indexes as durable block identity
absolute line numbers as durable content identity
Rust enum discriminants / memory layout
```

A plugin result that targets document state must carry enough identity/revision
information for Markit to reject stale results.

## 7. Snapshot-first data access

Plugins read coherent snapshots or explicit query results rather than borrowing
the live mutable editor model.

For example:

```text
plugin request
    ↓
DocumentSnapshot(revision = 412)
    ↓
plugin computes result
    ↓
result(document = D, base_revision = 412)
    ↓
Markit validates compatibility / staleness
    ↓
commit command OR reject/rebase
```

This aligns the plugin model with Markit's realtime execution contract: stale
work does not overwrite newer editor state.

## 8. Commands, not arbitrary mutation

Plugins do not directly mutate `Document`, `Markdown IR`, render state, or
GPUI entities.

Mutation crosses the boundary through documented commands/transactions such as:

```text
ApplyTextEdits
ReplaceSelection
SetDocumentMetadata
RegisterCommand
RequestExport
RequestDecoration
```

Names above are examples, not frozen APIs.

Markit validates commands, permissions, revision assumptions, and undo/redo
semantics before applying them.

This provides one place to preserve:

- undo correctness;
- dirty/change propagation;
- revision identity;
- IME/input invariants;
- scheduling priority;
- security/capability policy.

## 9. Capability-based extension model

Plugins declare the minimum capabilities they require. Example capability
families may include:

```text
document.read
 document.edit
 markdown.semantic.read
 commands.register
 decorations.publish
 export.provide
 print.provide
 workspace.read
 settings.read
 filesystem.scoped
 network.outbound
```

Capability names and permission granularity will be designed with the actual
plugin runtime. The important rule now is that **authority is explicit and
feature-detectable**.

Print/PDF is the model example: a print plugin should consume a documented
snapshot/semantic view and provide an export/print result. It must not need
access to GPUI internals or the editor's live render tree.

## 10. Plugin work must not contaminate the input hot path

The plugin boundary also inherits Markit's realtime rules.

A plugin may not make ordinary typing synchronously wait on unbounded plugin
work.

Conceptually:

```text
input / IME / visible edit      critical
visible plugin decoration       bounded / deadline-aware
near-viewport plugin work       deferrable
index/export/print/background   background
```

Slow, hung, or crashed plugin work must be isolatable from the editor's critical
interaction path by the chosen runtime architecture.

This requirement will influence the eventual choice between in-process,
Wasm, subprocess/IPC, or hybrid plugin execution; that choice is not made by
this document.

## 11. Contract tests are the real compatibility guarantee

Version numbers alone do not prove compatibility. Markit must maintain a
fixture suite of representative old plugins/manifests against new hosts.

When the plugin API becomes implementable, CI must include at least:

```text
host N loads plugin built for supported API N-k
old plugin can perform its documented operations
unknown optional capability is tolerated
missing required capability fails cleanly
stale plugin result cannot overwrite a newer document revision
deprecated operation remains functional during support window
malformed/incompatible plugin fails without damaging the workspace
plugin failure does not block ordinary editor input
```

For serialized protocols, retain golden messages/schemas and decode tests.
For a compiled ABI, add ABI-specific compatibility tests; do not assume the
compiler/runtime preserves layout.

## 12. Packaging and dependency isolation

A plugin should declare its own identity, version, API requirement,
capabilities, entry point, and dependencies in a manifest.

The host must not rely on accidental shared dependency resolution between the
host and plugins. Upgrading Markit's private dependencies must not silently
change the dependency graph visible to an installed plugin.

This is one reason not to expose Markit's Rust dependency graph as the plugin
ABI.

## 13. Failure and rollback behavior

Plugin compatibility failures are product states, not crashes.

The host should eventually support:

```text
compatible        → load
compatible+old    → load through supported contract/adapter
missing optional  → load with reduced capability
incompatible      → disable + explain
crash/hang        → isolate/disable + preserve document
bad update        → allow plugin rollback where packaging supports it
```

The editor must remain usable when a non-essential plugin is disabled.

## 14. MVP boundary

The Windows editor MVP does **not** require a general plugin runtime or
marketplace.

MVP/P0-P1 must only preserve the future boundary:

- do not expose `markit-core` internals as a public extension API;
- keep commands/transactions explicit;
- keep stable document/block identity possible;
- keep coherent snapshot/revision semantics;
- implement built-in features such as export/print through boundaries that can
  later become plugin capabilities where practical.

This prevents premature plugin-framework work while avoiding an architecture
that makes a stable plugin system impossible later.

## 15. Change-control rule

Any future change to the public plugin contract must answer:

```text
Is this additive or breaking?
Which API major/minor owns it?
Can an old plugin safely ignore it?
Is capability negotiation sufficient?
What is the migration/deprecation path?
Which old-plugin contract test proves compatibility?
Does the change expose a new Markit internal detail?
Can plugin failure affect the input hot path?
```

A PR that changes public plugin semantics without updating this contract,
compatibility fixtures, and the relevant roadmap/agent rules is incomplete.

## 16. Non-decisions

This contract deliberately does **not** yet choose:

```text
Rust dynamic library ABI
C ABI
Wasm component model
subprocess + IPC
JSON / MessagePack / protobuf / other wire encoding
plugin marketplace
signing model
network permission UX
exact compatibility-window duration
```

Those decisions require real plugin workloads and security/performance evidence.

What is fixed now is the boundary discipline: **versioned semantic protocol,
capability negotiation, stable identity, snapshot/command access, explicit
compatibility testing, and no dependency on Markit internals.**
