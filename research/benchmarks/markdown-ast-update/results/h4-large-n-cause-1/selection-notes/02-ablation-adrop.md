# Selection note — `Adrop`

**Trigger observation (from the superseded attempt-3 run).** `P7`
(sealing + the explicit retirement of the consumed old state) is 19.5 % of
U_PHASE at 128 KiB and 12.3 % at 16 MiB — a share that *shrinks* with N while
the absolute time grows from 17.5 µs to 3.21 ms. The allocator lane shows the
same region frees the old state's two vectors and releases M `Arc` handles,
of which only one is the last reference.

**Two explanations to separate.**

1. Retirement is *intrinsic* to the resident-update contract: the old state
   must stop being live before the update is usable, so its destruction is
   part of U by definition.
2. Retirement is *separable*: the destroyed objects are the consumed old
   state, whose destruction could happen at any later point without changing
   the produced state or any reference relation.

**Variant chosen.** `Adrop` (Issue #50 §10): move the complete consumed old
`H4DiagState` into an explicit post-timer owner. Nothing is cloned; the
deferred set is exactly that one value, with a recorded retirement inventory
(old slots, old checkpoint records, handles kept shared, handles that are the
last reference). `T_drain` is measured after the outer timer stops, with an
empty-drain control on every other variant.

**What would support each.** A near-zero U difference supports explanation 1
(or leaves it unresolved if the resolution is too coarse). A consistent
negative U difference that disappears when `T_drain` is added back supports
explanation 2: the cost is *moved*, not removed, and the design question is
which side of the timer it belongs on.
