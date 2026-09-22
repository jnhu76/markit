# result-contract.md — AUTHORITY POINTER (implementation deferred)

The normalized result contract (`normalize(update result) ==
normalize(H0 clean parse(post-edit source))`, comparison of
semantics/topology/spans, never pointer/NodeId/fragment/reuse identity) is
FROZEN in `R0-METHODOLOGY.md` §5 and is not restated or modified here.

R1 materializes only the correctness HOOK interface
(`oracle::CorrectnessHook`, checksum-based, executed outside `T_native`).
The real BENCH-GRAMMAR normalization oracle arrives with R4. The raw result
ROW schema is documented in `result-schema.md` / `result-schema-v2.json`
(schema v2, MEASUREMENT-CORRECTIVE-1).
