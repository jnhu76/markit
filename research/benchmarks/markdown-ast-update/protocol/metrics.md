# metrics.md — AUTHORITY POINTER (implementation deferred)

Headline metrics, work counters, Parse Amplification definition, and the
attribution ladder are FROZEN in `R0-METHODOLOGY.md` §10-§11 and are not
restated or modified here.

R1 materializes only their schema/interface: the `WorkCounters` slots with
three-valued `Observed<u64>` (`Known`/`Unknown`/`NotApplicable`), the
`TimingRecord` arithmetic (`T_total = T_prepare + T_native`), and the
T/M/A lane separation. No metric is measured for research in R1; no PA is
inferred from anything in R1.

Metric QUALIFICATION — which metric families the substrate can honestly
support (LATENCY, WORK_COUNTERS, PA = QUALIFIED; CPU_TIME, ALLOC_*,
PEAK, RETAINED = UNAVAILABLE) — is frozen in
`MEASUREMENT-CORRECTIVE-1.md` §9 and applies to every promoted
conclusion.
