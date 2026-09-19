# Deterministic project edit traces

Owns canonical real-project edit workloads for #31.

Traces are generated from the frozen eligible project files and are shared by
H0-H4. They must never contain horse-specific placement or preprocessing.

Expected shape:

```text
traces/
  README.md
  manifest/
    <project-id>.toml
  generated/
    <project-id>.jsonl
```

Minimum edit families:

```text
E1 LOCAL_TEXT
E2 PARAGRAPH_SPLIT_MERGE
E3 CONTAINER_DEPTH
E4 FENCE_OPEN_CLOSE
E5 INLINE_DELIMITER
E6 REFERENCE_DEFINITION
```

Every trace row must identify the pinned source/file, canonical UTF-8 byte edit,
family, deterministic selection seed/rule, and applicability status.

Timing, allocations, counters, and horse outputs do not belong here.
