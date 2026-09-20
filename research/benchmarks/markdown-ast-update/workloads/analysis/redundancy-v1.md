# Redundancy-v1 (CORRECTIVE-B §21)

Method: `SHINGLE-W5-MINHASH64-JACCARD-v1` (normalized word-5-gram shingles + deterministic 64-permutation MinHash, threshold 0.75; no embeddings, no seed). Near-duplicate groups are diagnostic only — nothing is removed automatically.

Exact duplicate groups: **3** (byte-identical by source SHA-256; canonical member = lexically smallest; acquisition registry cross-checked).

- `065c3bd87ee2` (3 members: openapi/files/versions/3.0.2-editors.md, openapi/files/versions/3.0.3-editors.md, openapi/files/versions/3.1.0-editors.md)
- `73cf20f00bed` (2 members: openapi/files/versions/3.0.4-editors.md, openapi/files/versions/3.1.1-editors.md)
- `fc88736c3288` (3 members: openapi/files/versions/3.1.2-editors.md, openapi/files/versions/3.2.0-editors.md, openapi/files/versions/3.2.1-editors.md)

Near-duplicate candidate groups: **10**.

- estimated Jaccard ≥ 0.73: ethereum-eips/files/EIPS/eip-3041.md, ethereum-eips/files/EIPS/eip-3044.md, ethereum-eips/files/EIPS/eip-3045.md, ethereum-eips/files/EIPS/eip-3046.md
- estimated Jaccard ≥ 0.94: oci-image/files/GOVERNANCE.md, oci-runtime/files/GOVERNANCE.md
- estimated Jaccard ≥ 0.89: oci-image/files/RELEASES.md, oci-runtime/files/RELEASES.md
- estimated Jaccard ≥ 0.89: openapi/files/versions/3.0.0.md, openapi/files/versions/3.0.1.md, openapi/files/versions/3.0.2.md, openapi/files/versions/3.0.3.md
- estimated Jaccard ≥ 1.00: openapi/files/versions/3.0.2-editors.md, openapi/files/versions/3.0.3-editors.md, openapi/files/versions/3.1.0-editors.md
- estimated Jaccard ≥ 1.00: openapi/files/versions/3.0.4-editors.md, openapi/files/versions/3.1.1-editors.md
- estimated Jaccard ≥ 0.66: openapi/files/versions/3.0.4.md, openapi/files/versions/3.1.1.md, openapi/files/versions/3.1.2.md
- estimated Jaccard ≥ 1.00: openapi/files/versions/3.1.2-editors.md, openapi/files/versions/3.2.0-editors.md, openapi/files/versions/3.2.1-editors.md
- estimated Jaccard ≥ 0.97: openapi/files/versions/3.2.0.md, openapi/files/versions/3.2.1.md
- estimated Jaccard ≥ 0.89: opentelemetry-spec/files/specification/schemas/file_format_v1.0.0.md, opentelemetry-spec/files/specification/schemas/file_format_v1.1.0.md
