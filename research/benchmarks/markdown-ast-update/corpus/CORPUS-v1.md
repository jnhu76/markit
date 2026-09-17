# CORPUS-v1 — frozen first-round synthetic corpus (R3)

Status: **R3 FREEZE CANDIDATE — READY_FOR_ADVERSARIAL_R3_REVIEW**
Authority: `protocol/R0-METHODOLOGY.md` §9 (shape set + sizes, FROZEN) +
this file (generation contract R3 freezes). Single owner of corpus shapes,
sizes, recipes, and determinism. `corpus/manifest.toml` is the
machine-readable projection of THIS file; where they disagree this file
wins and the manifest must be fixed.

R3 freezes the CONTRACT. No corpus bytes are generated in R3; the
reference generator is later infrastructure (first needed in R4) and must
satisfy this file byte-exactly.

---

## 1. What exists in the first round

Exactly 24 corpora: 8 shapes × 3 sizes. Nothing else is a CORPUS-v1
member. The R0 realism/sanity corpus (e.g. `CppCoreGuidelines.md`) is NOT
part of CORPUS-v1; its handling is a measurement-stage decision and, per
R0 §9, it can never enter strict horse comparison because its semantics
are not fully defined by BENCH-GRAMMAR-v1.

```text
corpus_id = "<shape>-<size>"     e.g. "mixed-1m"
size ∈ { 64k = 65536, 1m = 1048576, 16m = 16777216 }   (UTF-8 BYTES)
```

## 2. Size semantics (no tolerance)

```text
size == exact UTF-8 byte length of the generated file == payload_size_bytes
```

Generators are specified so the target is hit EXACTLY (every unit below
divides all three sizes; the padding rule never fires for CORPUS-v1 but
is frozen for safety). A `64 KiB` label backed by 49 KiB of source is a
defect, not an approximation. `actual_bytes == target_bytes` is asserted
at generation time and recorded in the generation receipt (§7).

## 3. Deterministic generation contract

```text
CORPUS_GENERATOR_VERSION = "corpus-gen-v1"
GLOBAL_SEED              = 0x4D41524B49542D33   (recorded; see below)
PRNG                     = splitmix64-v1 (RESERVED — corpus-gen-v1 does
                           NOT consume any random stream)
```

`corpus-gen-v1` is a pure tiling function of
`(shape, size, CORPUS_GENERATOR_VERSION)`:

```text
generate(shape, size) = header(shape) + unit(shape) × k + pad(remainder)
```

Byte-identical outputs are mandatory for the same inputs. Generators may
not depend on: hash-map iteration order, filesystem ordering, locale,
wall clock, `random_device`, thread scheduling, or environment. The seed
is recorded now so future generator versions that introduce variation
have a pinned, versioned derivation:
`seed(artifact) = splitmix64(GLOBAL_SEED + 0x9E3779B97F4A7C15 × (ordinal+1))`
— any use REQUIRES a `CORPUS_GENERATOR_VERSION` bump.

All corpora are generated ONCE per (shape, size), shared by every horse.
Corpora are never generated per-horse and never vary between runs.

### 3.1 Universal lexical conventions

```text
LF-terminated lines; files end with "\n"; no CR anywhere.
filler(i)  = ASCII letter 'a' + (i mod 26) for byte index i within the
             filler run (purely cosmetic variety; deterministic).
CJK        = the two characters 中 (E4 B8 AD) and 文 (E6 96 87), 3 bytes
             each. CJK appears only in the designated CJK variants below.
No tabs. No trailing spaces except where filler lands there.
```

### 3.2 Universal padding rule (reserved; never fires in CORPUS-v1)

If a remainder `r > 0` bytes is left after tiling units:

```text
r >= 2: emit "\n" + "x"×(r−2) + "\n"     (blank line closes ALL open
                                          containers; final x-line)
r == 1: emit "\n"                         (blank final line)
```

The emitted bytes are valid BENCH-GRAMMAR-v1 and cannot silently change a
shape's dominant property (blank lines terminate every container, so the
document is closed at depth 0).

---

## 4. The eight shapes (frozen recipes)

Each shape names: purpose, the exact unit, what is repeated, what is NOT
varied, the dominant structural property, and the mutation families it
validly carries.

### 4.1 PLAIN — low-structure baseline

```text
purpose      baseline: many small independent blocks, minimal nesting.
unit (64 B)  "\n" + line(62) + "\n"
             line(62)   = filler(62)
             CJK line   = "中文" + filler(56)          (every 16th unit)
repeated     one-line paragraphs separated by blank lines
NOT varied   block length, nesting depth (always 0), inline constructs
dominant     B (block count ≈ size/64); block-boundary density
families     LOCAL_TEXT, BLOCK_BOUNDARY, FORWARD_STATE(fence-open)
```

### 4.2 MANY_BLOCKS — boundary-est baseline

```text
purpose      maximal block-boundary count; stresses B and boundary edits.
unit (16 B)  line(14) + "\n" + "\n"
             line(14)   = filler(14)
             CJK line   = "中文" + filler(8)           (every 16th unit)
repeated     14-byte one-line paragraphs
NOT varied   line length, nesting, inlines
dominant     B (≈ size/16); the highest boundary density in the suite
families     LOCAL_TEXT, BLOCK_BOUNDARY, FORWARD_STATE(fence-open)
```

### 4.3 HUGE_BLOCK — L dominance

```text
purpose      one/few very large blocks; stresses L (affected block length).
unit (64 KiB) line(65534) + "\n" + "\n"
              line(65534) = "中文" + filler(65528)   (CJK head, every unit)
repeated      64 KiB single-line paragraphs; 64 KiB corpus = EXACTLY one
              block, 1 MiB = 16 blocks, 16 MiB = 256 blocks
NOT varied    block count per unit (1), nesting, inlines
dominant      L (affected block ≈ whole unit); tiny edit inside a huge
              semantic region
families      LOCAL_TEXT, BLOCK_BOUNDARY, FORWARD_STATE(fence-open)
```

### 4.4 DEEP_CONTAINER — hidden context propagation

```text
purpose      deep list/blockquote nesting; stresses D and container-state
             propagation (R2-H04, R2-H07).
mountain (2048 B)  32 lines × 64 B, one continuous container chain (NO
             blank lines). Mountain index m (0-based) alternates the
             container kind: m even -> list mountain, m odd ->
             blockquote mountain.
             lines d = 1..16 then 15..1 then 1 (32 lines):
               list line(d) = "  "×(d−1) + "- " + content(61−2(d−1)) + "\n"
               quote line(d) = "  "×(d−1) + "> " + content(61−2(d−1)) + "\n"
               content(k)   = filler(k)
               CJK variant  = "中文" + filler(k−6)     (every 8th line)
             line d and d+1 nest (indent step 2 satisfies the §7/§6
             nesting rules); after d=16 the ladder descends; the final
             d=1 line is a sibling. Containers stay open across the
             whole corpus (no blank line ever closes them).
repeated     alternating list/quote mountains tiled back to back
NOT varied   nesting step (2 spaces), line length (64 B), depth (16)
dominant     D (depth 16) + open-container state at every line start,
             for BOTH container kinds
families     CONTAINER_STATE (both kinds), LOCAL_TEXT,
             BLOCK_BOUNDARY(split only), FORWARD_STATE(fence-open
             inside containers)
```

### 4.5 INLINE_DENSE — inline delimiter state

```text
purpose      many emphasis/code-span/link delimiters; stresses inline
             scanning state (R2-H05, delimiter family).
unit (64 B)  "\n" + line(62) + "\n"
             line(62)   = "a *bb* `c` [x](/p) " ×3 + "a *b*"
             CJK line   = "中 *bb* `c` [x](/p) 中 *em* `cd` [y](/q) 文 *f* b [z](/r) c"
                                                            (every 16th unit)
repeated     delimiter-dense one-line paragraphs (all links INLINE — no
             definitions exist in this shape, by design)
NOT varied   delimiter density, paragraph length
dominant     K (inline construct count ≈ size/8 bytes of markup)
families     INLINE_DELIMITER_STATE, LOCAL_TEXT, BLOCK_BOUNDARY,
             FORWARD_STATE(fence-open)
```

### 4.6 FENCE_HEAVY — forward fence state

```text
purpose      many/large fenced regions; stresses forward fence state
             (R2-H03, R2-H05, R2-H10).
unit (512 B) "```x\n" + 2 × (body(250) + "\n") + "```\n" + "\n"
             body(250)  = filler(250)
             CJK body   = "中"×41 + filler(4)          (alternate units:
                           unit j uses CJK body iff j is odd)
repeated     closed 3-line fences with info string "x"
NOT varied   fence length, info string, body length
dominant     F (fence share of bytes ≈ 97.7%); long raw regions whose
             interpretation depends on a distant opener
families     FORWARD_STATE(fence-close), LOCAL_TEXT (inside bodies),
             BLOCK_BOUNDARY (at fence edges)
NOT_APPLICABLE  CONTAINER_STATE, INLINE_DELIMITER_STATE,
             SEMANTIC_DEPENDENCY, M-FS-FENCE-OPEN (no structure outside
             fences to swallow), M-LOC-UTF8-SWAP inside paragraph text
```

### 4.7 REFERENCE_FANOUT — semantic dependency

```text
purpose      many references depending on a small definition set;
             stresses semantic dependency / fallback (R2-H01).
header (512 B)  16 definition lines, one per label r00..r15:
             def(j) = "[r<j:02d>]: /" + filler(23, off=j) + "\n"   (32 B each)
unit (64 B)  "\n" + line(62) + "\n"
             line(62)   = "[t<n:03d>][r<i:02d>] " ×5 + "xx"
                           where n = global link ordinal (000..999 cycle)
                           and i = n mod 16        (uniform fanout)
             CJK line   = "中文" + link×4 + filler(8)  (every 16th unit)
repeated     reference links cycling over the SAME 16 definitions
NOT varied   definition count (16), destination length, fanout ratio
             (uses/def ≈ (5×links)/16, controlled by construction)
dominant     K_use/definitions ratio; every edit to a definition has a
             wide, precisely enumerable damage set
families     SEMANTIC_DEPENDENCY, INLINE_DELIMITER_STATE(link),
             LOCAL_TEXT, BLOCK_BOUNDARY
```

### 4.8 MIXED — controlled combination (not a random soup)

```text
purpose      all mechanisms in one deterministic document; the core
             measurement payload.
tile (1344 B) + 43 PLAIN units (64 B) = 4096 B per tile; every tile ends
             with a blank line, so every tile starts at depth 0:
  heading            "## section\n"                       (11 B)
  blank              "\n"                                 (1 B)
  definition table   "[rx{j}]: /" + filler(23, off=j) + "\n", j=0..3
                                                          (4 × 32 = 128 B)
  blank              "\n"                                 (1 B)
  blockquote         "> quoted aaaa\n> quoted bbbb\n"     (28 B)
  blank              "\n"                                 (1 B)
  list               "- item alpha\n- item beta\n  - nested gamma\n"
                                                          (42 B)
  blank              "\n"                                 (1 B)
  fence unit         512 B  (FENCE_HEAVY unit, ASCII bodies)
  inline line        "a *bb* `c` [x](/p) " ×3 + "a *b*\n"  (63 B)
  blank              "\n"                                 (1 B)
  link paragraph     8 lines, each "[t<n:03d>][rx<j>] " ×5 + "xx\n"
                       (63 B/line, labels cycling rx0..rx3, n continuing
                        across the corpus)                  (8 × 63 = 504 B)
  closer             "\n" + "x"×48 + "\n" + "\n"          (51 B)
repeated     the 4096 B tile (self-contained: all blocks opened in a
             tile are closed inside it)
NOT varied   per-tile composition; only the tile COUNT varies with size
dominant     a controlled mixture; attribution must use the other seven
             shapes, never MIXED alone
families     all six
```

## 5. UTF-8 / multibyte coverage

Every shape except FENCE_HEAVY contains CJK paragraphs/lines by recipe
(FENCE_HEAVY carries CJK fence bodies). Therefore every position class
(EARLY/MIDDLE/LATE anchors) can land near or inside multibyte runs, and
all edit ranges snap to UTF-8 char boundaries (down-snapping). Emoji are
covered by grammar fixtures (`utf8-emoji-heading`), not by corpora —
corpus multibyte stays 3-byte CJK to keep unit arithmetic exact.

## 6. Corpus validity invariants (checked at generation, recorded in receipts)

Every generated corpus must satisfy:

```text
valid UTF-8
valid BENCH-GRAMMAR-v1 input (parses with zero errors; total grammar)
byte-identical re-generation from (shape, size, generator version)
actual byte size == target size
SHA-256 recorded
no tabs, no CR, ends with exactly one "\n" at EOF position len−1
```

Every mutation applied to a corpus must satisfy (frozen in
`mutations/MUTATION-v1.md` §6, mirrored here as the corpus-side half):

```text
start <= end <= len(old_source)
start/end on UTF-8 char boundaries
apply(old_source, edit) == post_edit_source (host applies, byte-exact)
post-edit source is valid UTF-8 and valid BENCH-GRAMMAR-v1 input
```

## 7. Generation receipts (recorded later, schema frozen now)

R3 does not generate bytes, so `source_sha256` / `actual_bytes` cannot be
recorded yet. The receipt file `corpus/receipt-<corpus_id>.toml` is
produced by the FIRST generation (R4 or the measurement stage) and must
contain:

```toml
schema          = "corpus-receipt-v1"
corpus_id       = "mixed-1m"
generator_version = "corpus-gen-v1"
seed            = 0x4D41524B49542D33
target_bytes    = 1048576
actual_bytes    = 1048576          # asserted equal
source_sha256   = "<hex>"
shape_parameters = { }             # populated per shape from §4
```

`corpus/manifest.toml` carries `target_bytes` + generator parameters now;
`actual_bytes` / `source_sha256` live in receipts only (never
hand-written before generation exists).
