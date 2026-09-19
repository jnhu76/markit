# Real-project corpus

Owns the #31 real-project input surface.

This directory stores **metadata and derived profiles**, not benchmark results.
Third-party repository checkouts should normally live outside the Markit tree;
commit immutable source pins and hashes here instead.

Expected shape:

```text
projects/
  README.md
  manifest/
    <project-id>.toml
  profiles/
    <project-id>.json
```

Each project manifest must record at least:

```text
project_id
repository URL
immutable commit SHA
retrieval date
license / redistribution note
all Markdown file/byte counts
eligible file/byte counts
eligibility policy version
exclusion reasons
```

Each profile describes explanatory workload structure such as file size, block
count, largest block, container depth, fence density, inline density,
reference-definition density, and CJK share where relevant.

Performance numbers do not belong here.
