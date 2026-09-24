# Selection notes — H4-LARGE-N-CAUSE-1

Issue #50 §4 requires a short **selection note before each diagnostic added
after the first three lanes**: what was observed, which two explanations were
being separated, which variant/points were chosen, and what result would
support or refute each explanation.

These are `POST_HOC_EXPLANATORY` decision records (the issue's own result
class). They are not pre-registered hypotheses and they are not a substitute
for the measurement. Each note names the observation that triggered it.

**Provenance caveat.** The triggering observations were read from the
collection attempt archived under
`../attempt-3-mixed-provenance/`, which was superseded for provenance reasons
(its executable had been rebuilt). The decisions it triggered are recorded
here; every number that appears in the report and in the CSVs comes from the
final collection, produced by the frozen binaries in `../bin/`.

| Note | Decides |
|---|---|
| `01-ablation-adefs.md` | run `Adefs` |
| `02-ablation-adrop.md` | run `Adrop` |
| `03-ablation-acapacity.md` | run `Acapacity` |
| `04-pmu-points.md` | which N points the PMU lanes cover |
| `05-perf-record-pair.md` | the `perf record` pair and its scope |
| `06-ebpf-questions.md` | what the eBPF lane was asked to answer |
| `07-microkernels.md` | why no microkernel was built |
