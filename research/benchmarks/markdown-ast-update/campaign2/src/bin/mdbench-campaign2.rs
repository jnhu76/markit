//! `mdbench-campaign2` — MARKIT-31 FULL EVIDENCE CAMPAIGN-2 CLI.
//!
//! ```text
//! mdbench-campaign2 <subcommand> [benchmark_root] [flags...]
//!
//!   spec-bind <out>          freeze the Campaign-2 spec + all identities
//!   spec-verify              re-derive and verify every frozen identity
//!   machine-capture          capture the current host
//!   machine-verify           compare the current host to the frozen manifest
//!   workload-verify          materialization preflight (frozen source bytes)
//!   generators-verify        generate + verify every controlled cell
//!   lifecycle-freeze         freeze the lifecycle traces (BEFORE timing)
//!   schedule-generate        freeze the session schedules
//!   finalize --raw F         verify a raw file's correctness + identity
//!   run-construction    --session N [--out F] [--reps R]
//!   run-resident-update --session N [--out F]
//!   run-lifecycle       --session N [--out F]
//!   run-controlled      --axis A --session N [--cell L] [--out F]
//!   run-attribution     --surface S [--axis A] [--out F]
//!   run-memory          --mode construction|resident_update [--out F]
//!   pilot                    NON_PRIMARY pilot (never headline evidence)
//! ```

use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use markit_mdbench_campaign::workload::{load_campaign_workload, CampaignWorkload};
use markit_mdbench_campaign2::envelope::{
    Campaign2ObservationV1, CellIdentityV1, ExecutionIdentity2, LifecycleIdentityV1, SampleKind2,
    SCHEMA,
};
use markit_mdbench_campaign2::exec::{reference_for, CaseSpec};
use markit_mdbench_campaign2::generators::{self, Axis, ControlledCase};
use markit_mdbench_campaign2::lifecycle::{self, LifecycleTraceV1, RealPairChain, TracePlan, TraceStepV1};
use markit_mdbench_campaign2::spec::{self, SubCampaignBinding};
use markit_mdbench_campaign2::{
    current_executable_sha256, horse_mechanism_id, identity, sha256_hex, store, EvidenceClass,
    Surface2, HORSE_IDS,
};
use markit_mdbench_common::{CaseId, CaseKeyV1, Observed, OperationKind, PayloadShape, Source, SourceId};
use markit_mdbench_instrumentation::{InstantClock, LaneMeasurement};
use markit_mdbench_oracle::ReferenceOracle;
use markit_mdbench_runner::{current_build_identity, BuildIdentityV1, MeasurementV1};

/// The authority SHA of Campaign-2 (task §4).
pub const AUTHORITY_SHA: &str = "3762b7a42e1c284a4c2c2e0ebac8496e70c63431";
/// Frozen `perf stat` repetition count (task §35).
pub const PERF_STAT_REPETITIONS: u32 = 10;
/// Frozen session count (task §18).
pub const SESSION_COUNT: u32 = 3;
/// Frozen construction/resident-update sampling (task §18).
pub const WARMUP_ITERATIONS: u32 = 10;
pub const MEASURED_ITERATIONS: u32 = 30;

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().collect();
    let Some(subcommand) = args.get(1).cloned() else {
        eprintln!("usage: mdbench-campaign2 <subcommand> [benchmark_root] [flags]");
        return ExitCode::from(2);
    };
    let root = PathBuf::from(args.get(2).cloned().unwrap_or_else(|| ".".to_string()));
    let flags: Vec<String> = args[3.min(args.len())..].to_vec();
    let result: Result<(), String> = match subcommand.as_str() {
        "spec-bind" => cmd_spec_bind(&root, &flags),
        "spec-verify" => cmd_spec_verify(&root),
        "machine-capture" => cmd_machine_capture(&root, &flags),
        "machine-verify" => cmd_machine_verify(&root),
        "workload-verify" => cmd_workload_verify(&root),
        "generators-verify" => cmd_generators_verify(&root, &flags),
        "lifecycle-freeze" => cmd_lifecycle_freeze(&root, &flags),
        "schedule-generate" => cmd_schedule_generate(&root, &flags),
        "finalize" => cmd_finalize(&root, &flags),
        "run-construction" => cmd_run_construction(&root, &flags),
        "run-resident-update" => cmd_run_resident_update(&root, &flags),
        "run-lifecycle" => cmd_run_lifecycle(&root, &flags),
        "run-controlled" => cmd_run_controlled(&root, &flags),
        "run-attribution" => cmd_run_attribution(&root, &flags),
        "run-memory" => cmd_run_memory(&root, &flags),
        "pilot" => cmd_pilot(&root, &flags),
        "--help" | "help" => {
            print_help();
            Ok(())
        }
        other => Err(format!("unknown subcommand {other:?}")),
    };
    match result {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("mdbench-campaign2: {error}");
            ExitCode::from(1)
        }
    }
}

fn print_help() {
    println!(
        "mdbench-campaign2 — MARKIT-31 full evidence campaign 2\n\
         subcommands: spec-bind spec-verify machine-capture machine-verify\n\
         workload-verify generators-verify lifecycle-freeze schedule-generate\n\
         finalize run-construction run-resident-update run-lifecycle\n\
         run-controlled run-attribution run-memory pilot"
    );
}

fn flag_value(flags: &[String], name: &str) -> Option<String> {
    flags
        .iter()
        .position(|f| f == name)
        .and_then(|i| flags.get(i + 1))
        .cloned()
}

fn flag_u32(flags: &[String], name: &str, default: u32) -> u32 {
    flag_value(flags, name)
        .and_then(|v| v.parse().ok())
        .unwrap_or(default)
}

fn hex32(hex: &str) -> Result<[u8; 32], String> {
    if hex.len() != 64 {
        return Err(format!("digest {hex:?} is not 64 hex chars"));
    }
    (0..32)
        .map(|i| u8::from_str_radix(&hex[i * 2..i * 2 + 2], 16).map_err(|e| e.to_string()))
        .collect::<Result<Vec<_>, _>>()?
        .try_into()
        .map_err(|_| "digest length".to_string())
}

fn write_toml<T: serde::Serialize>(path: &Path, value: &T) -> Result<(), String> {
    let text =
        toml::to_string_pretty(value).map_err(|e| format!("serialize {}: {e}", path.display()))?;
    std::fs::write(path, text).map_err(|e| format!("write {}: {e}", path.display()))
}

fn count_lines(path: &Path) -> Result<u64, String> {
    let bytes = std::fs::read(path).map_err(|e| format!("read {}: {e}", path.display()))?;
    Ok(bytes.split(|b| *b == b'\n').filter(|l| !l.is_empty()).count() as u64)
}

// ---------------------------------------------------------------------------
// Frozen specification
// ---------------------------------------------------------------------------

/// Build the frozen campaign specification. Called with pilot sampling
/// values and then with the FINAL frozen values; the spec id moves when
/// they move, which is exactly the point (task §18, §26).
pub fn build_spec(
    benchmark_root: &Path,
    lifecycle_repetitions: u32,
    controlled_repetitions: u32,
) -> Result<spec::Campaign2Spec, String> {
    let workload = load_campaign_workload(benchmark_root)?;
    let paths = markit_mdbench_campaign2::manifest_paths(benchmark_root);
    let read_sha = |name: &str| -> Result<String, String> {
        let path = paths.payload(name);
        markit_mdbench_campaign2::sha256_file(&path)
            .map_err(|e| format!("frozen workload artifact {}: {e}", path.display()))
    };
    let (machine_cpu, machine_core, machine_siblings) =
        markit_mdbench_campaign::machine::select_primary_cpu()?;
    let families: std::collections::BTreeSet<&str> =
        workload.edit_write.iter().map(|c| c.edit_family.as_str()).collect();
    let trace_records = count_lines(&paths.payload("trace-manifest-v1.jsonl"))?;
    let envelope_sha = markit_mdbench_campaign2::sha256_file(
        &benchmark_root.join("protocol/result-schema-v2.json"),
    )
    .unwrap_or_else(|_| "UNAVAILABLE".to_string());
    let controlled_cells = generators::generate_all()?.len() as u64;

    Ok(spec::Campaign2Spec {
        study_id: identity::study_id(AUTHORITY_SHA),
        campaign_id: markit_mdbench_campaign2::CAMPAIGN2_ID.to_string(),
        schema: "campaign2-spec-v1".to_string(),
        authority_sha: AUTHORITY_SHA.to_string(),
        horse_set: HORSE_IDS.iter().map(|h| h.to_string()).collect(),
        horse_mechanism_ids: HORSE_IDS
            .iter()
            .map(|h| horse_mechanism_id(h).unwrap_or("UNKNOWN").to_string())
            .collect(),
        real_workload_identity: spec::RealWorkloadIdentity {
            source: "#35 CORRECTIVE-C frozen workload (workloads/); never re-selected".to_string(),
            full_read_manifest: "workloads/payloads/full-read-manifest-v1.jsonl".to_string(),
            full_read_manifest_sha256: read_sha("full-read-manifest-v1.jsonl")?,
            edit_write_manifest: "workloads/payloads/edit-write-manifest-v1.jsonl".to_string(),
            edit_write_manifest_sha256: read_sha("edit-write-manifest-v1.jsonl")?,
            trace_manifest: "workloads/payloads/trace-manifest-v1.jsonl".to_string(),
            trace_manifest_sha256: read_sha("trace-manifest-v1.jsonl")?,
            freeze_receipt: "workloads/payloads/freeze-receipt-v1.json".to_string(),
            freeze_receipt_sha256: read_sha("freeze-receipt-v1.json")?,
            full_read_qualified_cases: workload.clean_state.len() as u64,
            edit_write_qualified_cases: workload.edit_write.len() as u64,
            trace_records,
            break_restore_pairs: trace_records,
            projects: 0,
            edit_families: families.len() as u64,
            reselection_allowed: false,
        },
        controlled_generator_identity: spec::ControlledGeneratorIdentity {
            generator_id: generators::GENERATOR_ID.to_string(),
            generator_version: generators::GENERATOR_VERSION.to_string(),
            axes: ["N", "B", "D", "F", "K"].iter().map(|s| s.to_string()).collect(),
            n_scale_points: label_list(Axis::N),
            b_scale_points: label_list(Axis::B),
            d_scale_points: label_list(Axis::D),
            f_scale_points: label_list(Axis::F),
            k_scale_points: label_list(Axis::K),
            fixed_n_bytes: generators::FIXED_N_BYTES,
            note: format!(
                "{controlled_cells} controlled cells; documents are BENCH-GRAMMAR-v1 valid \
                 synthetic documents; edits are LOCAL_TEXT insertions or the axis's single \
                 designated structural edit; see campaign2/src/generators.rs for the frozen \
                 geometry and the recorded N-invariance entanglements"
            ),
        },
        timer_boundaries: spec::TimerBoundaries {
            construction: "source resident -> start -> clean parse + native state \
                           construction + seal -> usable -> stop; filesystem IO, oracle \
                           validation, normalize, checksum and report serialization excluded"
                .to_string(),
            resident_update: "fresh pre-edit state built OUTSIDE the timer; start -> \
                              prepare_update -> update -> seal -> stop; oracle outside; \
                              T_total = T_prepare + T_native"
                .to_string(),
            lifecycle: "state built once per (trace, horse, rep), untimed; then per step the \
                        resident_update boundaries with NO state reconstruction between edits"
                .to_string(),
            controlled: "identical to resident_update".to_string(),
        },
        sampling: spec::SamplingPolicy {
            session_count: SESSION_COUNT,
            warmup_iterations: WARMUP_ITERATIONS,
            measured_iterations: MEASURED_ITERATIONS,
            lifecycle_repetitions,
            controlled_repetitions,
            warmup_applies_to: vec!["construction".to_string(), "controlled".to_string()],
            note: "lifecycle records EVERY edit and additionally flags the K checkpoints; \
                   lifecycle/controlled repetition counts are frozen before the first formal \
                   row of their sub-campaign and never changed afterwards"
                .to_string(),
        },
        cpu_binding: spec::CpuBindingPolicy {
            selected_cpu: machine_cpu,
            selected_core_id: machine_core,
            selected_thread_siblings: machine_siblings,
            selected_numa_node: 0,
            policy: "one physical CPU for every session; affinity applied fail-closed; never \
                     changed between horses"
                .to_string(),
        },
        statistics: spec::StatisticsPolicy {
            quantile: "nearest-rank".to_string(),
            p50: "rank 15 of 30 measured".to_string(),
            p95: "rank 29 of 30 measured".to_string(),
            pooling: "sessions are NEVER pooled into n=90".to_string(),
            case_estimate: "median of the 3 session p50s".to_string(),
            h0_relative: "per-session p50(H0)/p50(Hx), then geometric mean across sessions"
                .to_string(),
            lifecycle: "per-step p50 plus a cumulative p50-derived trajectory, labelled \
                        derived and never presented as a directly measured wall-clock trace"
                .to_string(),
            instability_flag: "max(session p50)/min(session p50) > 1.5".to_string(),
            instability_action: "diagnostic flag only; never delete, downweight or rerun".to_string(),
        },
        correctness: spec::CorrectnessPolicy {
            oracle: "normalize(Hx result) == normalize(H0 clean full parse(post-edit source))"
                .to_string(),
            reference: "H0 clean full parse, NORMALIZED-RESULT-v1 validated".to_string(),
            outside_timer: true,
            lifecycle_verification: "after EVERY edit step, against the frozen trace checksum"
                .to_string(),
            failure_action: "stop the affected formal sub-campaign, preserve the failing \
                             evidence, do not retry until classified"
                .to_string(),
        },
        failure: spec::FailurePolicy {
            on_lane_failure: "preserve partial raw, mark the lane invalid, stop the affected \
                              sub-campaign"
                .to_string(),
            on_code_change: "new executable SHA, new RunId, restart the affected sub-campaign"
                .to_string(),
            forbidden: vec!["delete".to_string(), "impute".to_string(), "retry invisibly".to_string()],
            performance_driven_adaptation: "forbidden once formal rows start".to_string(),
        },
        profiling: spec::ProfilingPolicy {
            primary_timing_clean: true,
            forbidden_during_primary: [
                "perf record",
                "bpftrace",
                "strace",
                "heap profiler",
                "allocator interposer",
                "debug build",
            ]
            .iter()
            .map(|s| s.to_string())
            .collect(),
            matched_rule: "same exact case, source, edit, replay structure, profiling binary and \
                           CPU across H0-H4"
                .to_string(),
            perf_stat_repetitions: PERF_STAT_REPETITIONS,
            perf_stat_rotation: "horse order rotated per repetition".to_string(),
            perf_record_selection: "PROFILING-SELECTION-v1.md, POST_HOC / NON_PRIMARY".to_string(),
            ebpf_role: "capability-gated; supplemental only; never a substitute for PMU events"
                .to_string(),
            allocator_label: "PROFILE_ONLY_ALLOCATOR_EVIDENCE".to_string(),
            region_isolation: "dedicated replay command; a whole-process value is only reported \
                               together with its validating setup-only run"
                .to_string(),
        },
        interpretation_rules: markit_mdbench_campaign2::interpretation::rules()
            .iter()
            .map(|r| r.to_string())
            .collect(),
        envelope_schema_id: markit_mdbench_campaign2::envelope::SCHEMA.to_string(),
        envelope_schema_sha256: envelope_sha,
        metric_qualification: spec::metric_qualification(),
        order_algorithms: vec![
            markit_mdbench_runner::SHUFFLE_ALGORITHM_ID.to_string(),
            markit_mdbench_campaign::schedule::HORSE_ORDER_POLICY_ID.to_string(),
        ],
        base_authority_sha: AUTHORITY_SHA.to_string(),
    })
}

fn label_list(axis: Axis) -> Vec<String> {
    axis.point_labels().iter().map(|l| l.to_string()).collect()
}

fn axis_tag(axis: Axis) -> &'static str {
    match axis {
        Axis::N => "N",
        Axis::B => "B",
        Axis::D => "D-fence",
        Axis::F => "F-reference",
        Axis::K => "K-container",
    }
}

fn cmd_spec_bind(root: &Path, flags: &[String]) -> Result<(), String> {
    let lifecycle_reps = flag_u32(flags, "--lifecycle-reps", 30);
    let controlled_reps = flag_u32(flags, "--controlled-reps", 30);
    let campaign_spec = build_spec(root, lifecycle_reps, controlled_reps)?;
    let spec_id = identity::campaign_spec_id(&campaign_spec);
    let study = campaign_spec.study_id.clone();
    let manifests = store::ensure_layout(root)?.join("manifests");
    std::fs::create_dir_all(&manifests).map_err(|e| format!("mkdir manifests: {e}"))?;
    write_toml(&manifests.join("campaign-2-spec-v1.toml"), &campaign_spec)?;

    let workload = load_campaign_workload(root)?;
    let lifecycle_trace_count = load_frozen_traces(root).map(|t| t.len()).unwrap_or(0);
    let mut subs = Vec::new();
    for tag in identity::SUB_CAMPAIGNS {
        let binding = sub_campaign_binding(
            tag,
            &campaign_spec,
            &workload,
            lifecycle_trace_count,
        );
        let sub_id = identity::sub_campaign_spec_id(&spec_id, tag, &binding);
        write_toml(&manifests.join(format!("sub-campaign-{tag}-v1.toml")), &binding)?;
        subs.push(identity::SubCampaignIdentity {
            tag: tag.to_string(),
            sub_campaign_spec_id: sub_id,
        });
    }
    let block = identity::Campaign2Identity {
        study_id: study.clone(),
        campaign_spec_id: spec_id.clone(),
        campaign_id: campaign_spec.campaign_id.clone(),
        authority_sha: AUTHORITY_SHA.to_string(),
        envelope_schema_id: markit_mdbench_campaign2::envelope::SCHEMA.to_string(),
        seed_algorithm: identity::SEED_ALGORITHM_ID.to_string(),
        campaign_seed: identity::campaign_seed(AUTHORITY_SHA),
        sub_campaigns: subs,
    };
    let bytes = serde_json::to_vec_pretty(&block).map_err(|e| format!("serialize identity: {e}"))?;
    std::fs::write(manifests.join("campaign-2-identity-v1.json"), bytes)
        .map_err(|e| format!("write identity block: {e}"))?;
    println!("CAMPAIGN2_SPEC_OK study_id={study} campaign_spec_id={spec_id}");
    println!(
        "CAMPAIGN2_SPEC_SAMPLING lifecycle_repetitions={lifecycle_reps} controlled_repetitions={controlled_reps} sessions={SESSION_COUNT} warmup={WARMUP_ITERATIONS} measured={MEASURED_ITERATIONS}"
    );
    for sub in &block.sub_campaigns {
        println!(
            "CAMPAIGN2_SUB_SPEC tag={} sub_campaign_spec_id={}",
            sub.tag, sub.sub_campaign_spec_id
        );
    }
    Ok(())
}

fn sub_campaign_binding(
    tag: &str,
    campaign_spec: &spec::Campaign2Spec,
    workload: &CampaignWorkload,
    lifecycle_traces: usize,
) -> SubCampaignBinding {
    let (surface, cardinality, classes, lane, axis_values, raw_path): (String, u64, Vec<String>, &str, Vec<String>, String) = match tag {
        "construction" => (
            "construction".to_string(),
            workload.clean_state.len() as u64,
            vec![EvidenceClass::PrimaryTiming.as_str().to_string()],
            "timing",
            Vec::new(),
            "results/campaign-2/construction/".to_string(),
        ),
        "resident_update" => (
            "resident_update".to_string(),
            workload.edit_write.len() as u64,
            vec![EvidenceClass::PrimaryTiming.as_str().to_string()],
            "timing",
            Vec::new(),
            "results/campaign-2/resident-update/".to_string(),
        ),
        "lifecycle" => (
            "lifecycle".to_string(),
            lifecycle_traces as u64,
            vec![EvidenceClass::PrimaryLifecycle.as_str().to_string()],
            "lifecycle",
            Vec::new(),
            "results/campaign-2/lifecycle/".to_string(),
        ),
        "controlled_n" => controlled_binding(Axis::N),
        "controlled_b" => controlled_binding(Axis::B),
        "controlled_d_fence" => controlled_binding(Axis::D),
        "controlled_f_reference" => controlled_binding(Axis::F),
        "controlled_k_container" => controlled_binding(Axis::K),
        "profiling" => (
            "profiling".to_string(),
            0,
            vec![EvidenceClass::ProfilePerf.as_str().to_string()],
            "profile",
            Vec::new(),
            "results/campaign-2/profiling/".to_string(),
        ),
        _ => (
            format!("unknown:{tag}"),
            0,
            Vec::new(),
            "unknown",
            Vec::new(),
            "results/campaign-2/".to_string(),
        ),
    };
    SubCampaignBinding {
        tag: tag.to_string(),
        surface,
        evidence_classes: classes,
        lanes: vec![lane.to_string()],
        case_cardinality: cardinality,
        session_count: campaign_spec.sampling.session_count,
        warmup_iterations: campaign_spec.sampling.warmup_iterations,
        measured_iterations: campaign_spec.sampling.measured_iterations,
        axis_values,
        raw_path,
        note: String::new(),
    }
}

fn controlled_binding(axis: Axis) -> (String, u64, Vec<String>, &'static str, Vec<String>, String) {
    (
        "controlled".to_string(),
        axis.points().len() as u64,
        vec![
            EvidenceClass::ControlledTiming.as_str().to_string(),
            EvidenceClass::ControlledWork.as_str().to_string(),
        ],
        "timing",
        label_list(axis),
        format!("results/campaign-2/controlled/{}/", axis_tag(axis)),
    )
}

fn cmd_spec_verify(root: &Path) -> Result<(), String> {
    let frozen = markit_mdbench_campaign2::load_frozen_identity(root)?;
    let manifests = store::campaign_root(root).join("manifests");
    let spec_path = manifests.join("campaign-2-spec-v1.toml");
    let text = std::fs::read_to_string(&spec_path).map_err(|e| format!("read {}: {e}", spec_path.display()))?;
    let frozen_spec: spec::Campaign2Spec =
        toml::from_str(&text).map_err(|e| format!("parse {}: {e}", spec_path.display()))?;
    let spec_id = identity::campaign_spec_id(&frozen_spec);
    let mut blockers = Vec::new();
    if spec_id != frozen.campaign_spec_id {
        blockers.push(format!(
            "re-derived campaign spec id {spec_id} != frozen {}",
            frozen.campaign_spec_id
        ));
    }
    let expected_study = identity::study_id(AUTHORITY_SHA);
    if expected_study != frozen.study_id {
        blockers.push(format!("re-derived study id {expected_study} != frozen {}", frozen.study_id));
    }
    for sub in &frozen.sub_campaigns {
        let path = manifests.join(format!("sub-campaign-{}-v1.toml", sub.tag));
        let text = std::fs::read_to_string(&path).map_err(|e| format!("read {}: {e}", path.display()))?;
        let binding: SubCampaignBinding =
            toml::from_str(&text).map_err(|e| format!("parse {}: {e}", path.display()))?;
        let re_derived = identity::sub_campaign_spec_id(&spec_id, &sub.tag, &binding);
        if re_derived != sub.sub_campaign_spec_id {
            blockers.push(format!(
                "sub-campaign {}: re-derived {re_derived} != frozen {}",
                sub.tag, sub.sub_campaign_spec_id
            ));
        }
    }
    if blockers.is_empty() {
        println!(
            "CAMPAIGN2_SPEC_VERIFY_PASS study_id={} campaign_spec_id={} sub_campaigns={}",
            frozen.study_id,
            frozen.campaign_spec_id,
            frozen.sub_campaigns.len()
        );
        Ok(())
    } else {
        for blocker in &blockers {
            eprintln!("SPEC_BLOCKER {blocker}");
        }
        Err(format!("{} spec blockers", blockers.len()))
    }
}

// ---------------------------------------------------------------------------
// Machine + workload preflight
// ---------------------------------------------------------------------------

fn cmd_machine_capture(root: &Path, flags: &[String]) -> Result<(), String> {
    let machine_id = flag_value(flags, "--machine-id")
        .unwrap_or_else(|| "campaign2-primary".to_string());
    let notes = flag_value(flags, "--notes").unwrap_or_else(|| {
        "governor/turbo/SMT are inspected and recorded, never modified by tooling".to_string()
    });
    let machine = markit_mdbench_campaign::machine::capture_machine(&machine_id, root, &notes)?;
    let out = store::ensure_layout(root)?;
    let path = out.join("manifests/campaign-2-machine-v1.toml");
    write_toml(&path, &machine)?;
    let sha = markit_mdbench_campaign2::sha256_file(&path)?;
    println!(
        "CAMPAIGN2_MACHINE_CAPTURED path={} sha256={sha} cpu={} core={} siblings={}",
        path.display(),
        machine.selected_cpu,
        machine.selected_core_id,
        machine.selected_thread_siblings
    );
    Ok(())
}

fn cmd_machine_verify(root: &Path) -> Result<(), String> {
    let path = store::campaign_root(root).join("manifests/campaign-2-machine-v1.toml");
    let text = std::fs::read_to_string(&path).map_err(|e| format!("read {}: {e}", path.display()))?;
    let frozen: markit_mdbench_campaign2::machine::MachineManifest =
        toml::from_str(&text).map_err(|e| format!("parse {}: {e}", path.display()))?;
    let diagnostics = markit_mdbench_campaign2::machine::transient_diagnostics();
    println!(
        "CAMPAIGN2_MACHINE_DIAGNOSTICS {}",
        serde_json::to_string(&diagnostics).unwrap_or_default()
    );
    match markit_mdbench_campaign::machine::match_current_host(&frozen, root) {
        Ok(()) => {
            println!("CAMPAIGN2_MACHINE_VERIFY_PASS machine_id={}", frozen.machine_id);
            Ok(())
        }
        Err(blockers) => {
            for blocker in &blockers {
                eprintln!("MACHINE_BLOCKER {blocker}");
            }
            Err(format!("{} machine blockers", blockers.len()))
        }
    }
}

fn cmd_workload_verify(root: &Path) -> Result<(), String> {
    let workload = load_campaign_workload(root)?;
    let mut full_read_bytes = 0u64;
    for case in &workload.clean_state {
        if sha256_hex(case.source_text.as_bytes()) != case.source_sha256 {
            return Err(format!("FULL_READ source {} drifted from the frozen digest", case.source_key));
        }
        if case.source_text.len() as u64 != case.file_bytes {
            return Err(format!("FULL_READ source {} byte count drifted", case.source_key));
        }
        full_read_bytes += case.file_bytes;
    }
    let mut edit_bytes = 0u64;
    let mut missing_files = 0u64;
    for case in &workload.edit_write {
        if case.pre_source_text.is_empty() {
            missing_files += 1;
        }
        edit_bytes += case.pre_len_bytes;
    }
    if missing_files > 0 {
        return Err(format!("{missing_files} EDIT_WRITE pre-sources are empty"));
    }
    println!(
        "CAMPAIGN2_WORKLOAD_VERIFY_PASS full_read={} edit_write={} full_read_bytes={} edit_write_pre_bytes={} missing=0",
        workload.clean_state.len(),
        workload.edit_write.len(),
        full_read_bytes,
        edit_bytes
    );
    Ok(())
}

fn cmd_generators_verify(root: &Path, _flags: &[String]) -> Result<(), String> {
    let out_dir = store::ensure_layout(root)?;
    let mut notes = Vec::new();
    for axis in [Axis::N, Axis::B, Axis::D, Axis::F, Axis::K] {
        for case in generators::generate_axis(axis)? {
            generators::verify_case(&case)?;
            reference_for(&case.pre_source).map_err(|e| {
                format!("{} {}: pre source inadmissible: {e}", axis.as_str(), case.axis_label)
            })?;
            reference_for(&case.post_source).map_err(|e| {
                format!("{} {}: post source inadmissible: {e}", axis.as_str(), case.axis_label)
            })?;
            notes.push(format!(
                "CELL axis={} label={} value={} pre_bytes={} post_bytes={} edit_start={} edit_end={} target_relative={:.6} transition={} cell_id={} construction={}",
                axis.as_str(),
                case.axis_label,
                case.axis_value,
                case.pre_source.len(),
                case.post_source.len(),
                case.edit_start,
                case.edit_end,
                case.target_relative,
                case.transition_label,
                case.cell_id,
                case.construction
                    .iter()
                    .map(|(k, v)| format!("{k}={v}"))
                    .collect::<Vec<_>>()
                    .join(",")
            ));
        }
    }
    let path = out_dir.join("manifests/controlled-cells-v1.txt");
    std::fs::write(&path, notes.join("\n") + "\n")
        .map_err(|e| format!("write {}: {e}", path.display()))?;
    for note in &notes {
        println!("{note}");
    }
    println!("CONTROLLED_CELL_TABLE {}", path.display());
    println!("CAMPAIGN2_GENERATORS_VERIFY_PASS cells={}", notes.len());
    Ok(())
}

// ---------------------------------------------------------------------------
// Lifecycle freeze
// ---------------------------------------------------------------------------

/// Preregistered real-trace selection (task §16): at most
/// `REAL_TRACE_PER_FAMILY` traces per frozen edit family — the
/// lexicographically first `trace_id` of each family — restricted to
/// base sources of at most `MAX_REAL_TRACE_BYTES`.
pub const REAL_TRACE_PER_FAMILY: usize = 2;
pub const MAX_REAL_TRACE_BYTES: usize = 131_072;

fn real_pair_traces(workload: &CampaignWorkload) -> Vec<RealPairChain> {
    let mut chains = lifecycle::select_real_pair_traces(workload, REAL_TRACE_PER_FAMILY, lifecycle::TRACE_STEPS);
    chains.retain(|chain| chain.base_source.len() <= MAX_REAL_TRACE_BYTES);
    chains
}

fn materialize_real(chain: &RealPairChain) -> Result<LifecycleTraceV1, String> {
    let initial = chain.initial_source();
    let mut current = initial.clone();
    let mut steps: Vec<TraceStepV1> = Vec::with_capacity(chain.step_count() as usize);
    for step in 0..chain.step_count() {
        let (edit, label, transition) = chain.next_edit(step, &current)?;
        let current_source = Source::new(SourceId(0), current.clone());
        let post = edit
            .apply(&current_source, SourceId(1))
            .map_err(|e| format!("{} step {step}: {e:?}", chain.trace_id()))?
            .as_str()
            .to_string();
        let expected = lifecycle::expected_checksum(&post)?;
        steps.push(TraceStepV1 {
            step,
            edit_start: edit.start_byte(),
            edit_end: edit.end_byte(),
            inserted_text: edit.inserted_text().to_string(),
            post_source_sha256: sha256_hex(post.as_bytes()),
            expected_checksum: format!("{expected:016x}"),
            label,
            transition_label: transition,
        });
        current = post;
    }
    Ok(LifecycleTraceV1 {
        schema: lifecycle::LIFECYCLE_TRACE_SCHEMA.to_string(),
        trace_id: chain.trace_id(),
        family: chain.family().to_string(),
        chain_construction: chain.chain_construction(),
        initial_source_origin: lifecycle::TraceOriginV1::RealPayloadChain {
            workload_trace_id: chain.workload_trace_id.clone(),
            base_source_sha256: chain.base_sha256.clone(),
        },
        initial_source_sha256: sha256_hex(initial.as_bytes()),
        initial_source_bytes: initial.len() as u64,
        step_count: chain.step_count(),
        steps,
    })
}

fn cmd_lifecycle_freeze(root: &Path, _flags: &[String]) -> Result<(), String> {
    let workload = load_campaign_workload(root)?;
    let out_dir = store::ensure_layout(root)?;
    let mut records: Vec<String> = Vec::new();
    let mut summary = Vec::new();

    for chain in &real_pair_traces(&workload) {
        let trace = materialize_real(chain)?;
        summary.push(format!(
            "TRACE family=real_break_restore trace_id={} steps={} base_bytes={} base_sha256={} construction={}",
            trace.trace_id, trace.step_count, trace.initial_source_bytes, trace.initial_source_sha256,
            trace.chain_construction
        ));
        records.push(serde_json::to_string(&trace).map_err(|e| format!("serialize trace: {e}"))?);
    }

    for plan in lifecycle::controlled_plans() {
        let trace = lifecycle::materialize(plan.as_ref())?;
        summary.push(format!(
            "TRACE family={} trace_id={} steps={} base_bytes={} base_sha256={}",
            trace.family, trace.trace_id, trace.step_count, trace.initial_source_bytes,
            trace.initial_source_sha256
        ));
        records.push(serde_json::to_string(&trace).map_err(|e| format!("serialize trace: {e}"))?);
    }

    let path = out_dir.join("manifests/lifecycle-traces-v1.jsonl");
    std::fs::write(&path, records.join("\n") + "\n")
        .map_err(|e| format!("write {}: {e}", path.display()))?;
    let sha = markit_mdbench_campaign2::sha256_file(&path)?;
    for line in &summary {
        println!("{line}");
    }
    println!(
        "LIFECYCLE_FREEZE_PASS traces={} steps_total={} path={} sha256={sha}",
        summary.len(),
        summary.len() * lifecycle::TRACE_STEPS as usize,
        path.display()
    );
    Ok(())
}

fn load_frozen_traces(root: &Path) -> Result<Vec<LifecycleTraceV1>, String> {
    let path = store::campaign_root(root).join("manifests/lifecycle-traces-v1.jsonl");
    let bytes = std::fs::read(&path).map_err(|e| format!("read {}: {e}", path.display()))?;
    let mut out = Vec::new();
    for (index, line) in bytes.split(|b| *b == b'\n').enumerate() {
        if line.is_empty() {
            continue;
        }
        out.push(
            serde_json::from_slice(line)
                .map_err(|e| format!("{}: trace row {index}: {e}", path.display()))?,
        );
    }
    Ok(out)
}

fn trace_initial_source(trace: &LifecycleTraceV1, workload: &CampaignWorkload) -> Result<String, String> {
    match &trace.initial_source_origin {
        lifecycle::TraceOriginV1::RealPayloadChain { workload_trace_id, base_source_sha256 } => {
            for case in &workload.edit_write {
                if &case.trace_id == workload_trace_id {
                    if sha256_hex(case.pre_source_text.as_bytes()) != *base_source_sha256 {
                        return Err(format!("real trace {workload_trace_id}: workload base drifted"));
                    }
                    return Ok(case.pre_source_text.clone());
                }
            }
            Err(format!("real trace {workload_trace_id}: base not in the frozen workload"))
        }
        lifecycle::TraceOriginV1::Controlled { family, .. } => {
            for plan in lifecycle::controlled_plans() {
                if plan.family() == family {
                    return Ok(plan.initial_source());
                }
            }
            Err(format!("controlled trace family {family} has no plan"))
        }
    }
}

// ---------------------------------------------------------------------------
// Schedule freeze
// ---------------------------------------------------------------------------

/// Case identity of one frozen lifecycle trace: a `FullParse` case over
/// the trace's frozen initial source. The trace is the unit of scheduling,
/// so its identity must not depend on any step.
fn trace_case_id(trace: &LifecycleTraceV1) -> CaseId {
    let key = CaseKeyV1 {
        payload_id: format!("c2-lifecycle-trace:{}", trace.trace_id),
        payload_shape: PayloadShape::Mixed,
        payload_size_bytes: trace.initial_source_bytes,
        old_source_sha256: hex32(&trace.initial_source_sha256)
            .expect("frozen trace carries a 32-byte digest"),
        operation: OperationKind::FullParse,
        edit_start_byte: None,
        edit_end_byte: None,
        inserted_text_sha256: None,
        generator_id: Some("CAMPAIGN-2-LIFECYCLE".to_string()),
        generator_seed: None,
    }
    .validated()
    .expect("frozen trace case key is valid");
    CaseId::from_key(&key)
}

fn cmd_schedule_generate(root: &Path, flags: &[String]) -> Result<(), String> {
    let out_dir = store::ensure_layout(root)?;
    let root_seed = identity::campaign_seed(AUTHORITY_SHA);
    let horse_seed = identity::horse_base_seed(root_seed);
    let workload = load_campaign_workload(root)?;
    let traces = load_frozen_traces(root)?;
    let mut all_rows = Vec::new();

    for session in 0..SESSION_COUNT {
        let shim = |cases: Vec<(CaseId, String, Option<(String, String)>, Option<String>)>| {
            cases
                .into_iter()
                .map(|(case_id, payload_id, axis, trace_id)| {
                    markit_mdbench_campaign2::schedule::ScheduleCase {
                        case_id_hex: case_id.hex(),
                        case_id,
                        payload_id,
                        axis,
                        trace_id,
                    }
                })
                .collect::<Vec<_>>()
        };
        let construction_cases = shim(
            workload
                .clean_state
                .iter()
                .map(|c| (c.case_id, c.payload_id.clone(), None, None))
                .collect(),
        );
        all_rows.extend(markit_mdbench_campaign2::schedule::build_schedule(
            "construction",
            "construction",
            session,
            identity::session_seed(root_seed, "construction", session),
            horse_seed,
            &construction_cases,
        ));
        let resident_cases = shim(
            workload
                .edit_write
                .iter()
                .map(|c| (c.case_id, c.payload_id.clone(), None, Some(c.trace_id.clone())))
                .collect(),
        );
        all_rows.extend(markit_mdbench_campaign2::schedule::build_schedule(
            "resident_update",
            "resident_update",
            session,
            identity::session_seed(root_seed, "resident_update", session),
            horse_seed,
            &resident_cases,
        ));
        let lifecycle_cases = shim(
            traces
                .iter()
                .map(|t| {
                    (
                        trace_case_id(t),
                        t.trace_id.clone(),
                        None,
                        Some(t.trace_id.clone()),
                    )
                })
                .collect(),
        );
        all_rows.extend(markit_mdbench_campaign2::schedule::build_schedule(
            "lifecycle",
            "lifecycle",
            session,
            identity::session_seed(root_seed, "lifecycle", session),
            horse_seed,
            &lifecycle_cases,
        ));
        for axis in [Axis::N, Axis::B, Axis::D, Axis::F, Axis::K] {
            let cells = shim(
                generators::generate_axis(axis)?
                    .iter()
                    .map(|c| {
                        (
                            c.case_id,
                            c.cell_id.clone(),
                            Some((axis.as_str().to_string(), c.axis_label.clone())),
                            None,
                        )
                    })
                    .collect(),
            );
            all_rows.extend(markit_mdbench_campaign2::schedule::build_schedule(
                axis.sub_campaign_tag(),
                "controlled",
                session,
                identity::session_seed(root_seed, axis.sub_campaign_tag(), session),
                horse_seed,
                &cells,
            ));
        }
    }
    let bytes = markit_mdbench_campaign2::schedule::schedule_to_jsonl(&all_rows)?;
    let path = flag_value(flags, "--out")
        .map(PathBuf::from)
        .unwrap_or_else(|| out_dir.join("manifests/campaign-2-schedule-v1.jsonl"));
    std::fs::write(&path, &bytes).map_err(|e| format!("write {}: {e}", path.display()))?;
    let sha = markit_mdbench_campaign2::sha256_file(&path)?;
    println!(
        "CAMPAIGN2_SCHEDULE_PASS rows={} bytes={} sha256={sha} path={}",
        all_rows.len(),
        bytes.len(),
        path.display()
    );
    Ok(())
}

// ---------------------------------------------------------------------------
// Observation assembly
// ---------------------------------------------------------------------------

struct Emitter {
    exec_identity: ExecutionIdentity2,
    surface: Surface2,
    session_id: Option<String>,
    session_ordinal: Option<u32>,
    build: BuildIdentityV1,
}

impl Emitter {
    #[allow(clippy::too_many_arguments)]
    fn emit<W: std::io::Write>(
        &self,
        out: &mut W,
        case_order_ordinal: u32,
        horse_order_ordinal: u32,
        horse_id: &str,
        sample_kind: SampleKind2,
        iteration_ordinal: u32,
        case_id: &str,
        cell: Option<CellIdentityV1>,
        lifecycle: Option<LifecycleIdentityV1>,
        state_repr: Option<markit_mdbench_campaign2::envelope::StateReprV1>,
        row: markit_mdbench_runner::ResultRowV1,
    ) -> Result<(), String> {
        let session_id = self.session_id.clone().unwrap_or_default();
        let observation_id = identity::observation_id(
            &self.exec_identity.run_id,
            &session_id,
            self.surface.as_str(),
            case_id,
            horse_id,
            sample_kind.as_str(),
            iteration_ordinal,
        );
        let lane = match row.measurement {
            MeasurementV1::Timing(_) => "timing",
            MeasurementV1::Memory(_) => "memory",
            MeasurementV1::Attribution(_) => "attribution",
        };
        let observation = Campaign2ObservationV1 {
            schema: SCHEMA.to_string(),
            study_id: self.exec_identity.study_id.clone(),
            campaign_spec_id: self.exec_identity.campaign_spec_id.clone(),
            sub_campaign_spec_id: self.exec_identity.sub_campaign_spec_id.clone(),
            run_id: self.exec_identity.run_id.clone(),
            evidence_class: self.exec_identity.evidence_class.to_string(),
            surface: self.surface.as_str().to_string(),
            lane: lane.to_string(),
            session_id: self.session_id.clone(),
            session_ordinal: self.session_ordinal,
            case_order_ordinal,
            horse_order_ordinal,
            horse_id: horse_id.to_string(),
            sample_kind: sample_kind.as_str().to_string(),
            iteration_ordinal,
            observation_id,
            cell,
            lifecycle,
            state_repr,
            result_row_v2: row,
        };
        let line = serde_json::to_string(&observation).map_err(|e| format!("serialize observation: {e}"))?;
        out.write_all(line.as_bytes()).map_err(|e| format!("write: {e}"))?;
        out.write_all(b"\n").map_err(|e| format!("write: {e}"))
    }
}

fn provenance_for(evidence_class: &str) -> &'static str {
    match evidence_class {
        "PRIMARY_TIMING" => "CAMPAIGN-2/CONSTRUCTION|RESIDENT_UPDATE",
        "PRIMARY_LIFECYCLE" => "CAMPAIGN-2/LIFECYCLE",
        "CONTROLLED_TIMING" | "CONTROLLED_WORK" => "CAMPAIGN-2/CONTROLLED",
        "PRIMARY_WORK" => "CAMPAIGN-2/ATTRIBUTION",
        "DESCRIPTIVE_MEMORY" => "CAMPAIGN-2/MEMORY/DESCRIPTIVE_PROCESS_MEMORY",
        _ => "CAMPAIGN-2/OTHER",
    }
}

fn identity_for(
    root: &Path,
    tag: &str,
    evidence_class: &'static str,
    session_ordinal: Option<u32>,
    lane: &str,
) -> Result<(ExecutionIdentity2, Option<String>), String> {
    let frozen = markit_mdbench_campaign2::load_frozen_identity(root)?;
    let sub = frozen
        .sub_campaigns
        .iter()
        .find(|s| s.tag == tag)
        .ok_or_else(|| format!("no frozen sub-campaign {tag:?}"))?;
    let machine_path = store::campaign_root(root).join("manifests/campaign-2-machine-v1.toml");
    let machine_sha = markit_mdbench_campaign2::sha256_file(&machine_path)?;
    let build = current_build_identity();
    let executable = current_executable_sha256()?;
    let thread = std::thread::Builder::new()
        .name(format!("run-{tag}-{session_ordinal:?}"))
        .spawn({
            let machine_sha = machine_sha.clone();
            let build = build.clone();
            let executable = executable.clone();
            let frozen = frozen.clone();
            let sub_id = sub.sub_campaign_spec_id.clone();
            move || {
                identity::run_id(
                    &frozen.study_id,
                    &frozen.campaign_spec_id,
                    &sub_id,
                    &build.runner_git_commit,
                    &machine_sha,
                    &build,
                    &executable,
                )
            }
        })
        .map_err(|e| format!("spawn id thread: {e}"))?;
    let run_id = thread.join().map_err(|_| "run id thread panicked".to_string())?;
    let session_id = session_ordinal
        .map(|s| identity::session_id(&frozen.campaign_spec_id, tag, s, lane));
    Ok((
        ExecutionIdentity2 {
            study_id: frozen.study_id.clone(),
            campaign_spec_id: frozen.campaign_spec_id.clone(),
            sub_campaign_spec_id: sub.sub_campaign_spec_id.clone(),
            run_id,
            evidence_class,
            machine_environment_ref: format!(
                "results/campaign-2/manifests/campaign-2-machine-v1.toml#{machine_sha}"
            ),
            provenance: provenance_for(evidence_class),
            non_research: false,
        },
        session_id,
    ))
}

// ---------------------------------------------------------------------------
// Surface runners
// ---------------------------------------------------------------------------

fn cmd_run_construction(root: &Path, flags: &[String]) -> Result<(), String> {
    let session = flag_u32(flags, "--session", 0);
    let measured = flag_u32(flags, "--reps", MEASURED_ITERATIONS);
    let (exec_identity, session_id) = identity_for(
        root,
        "construction",
        EvidenceClass::PrimaryTiming.as_str(),
        Some(session),
        "timing",
    )?;
    let workload = load_campaign_workload(root)?;
    let out_path = out_path(root, flags, "construction", &format!("session-{session}-construction.jsonl"))?;
    let mut out = new_writer(&out_path)?;
    let emitter = Emitter {
        exec_identity,
        surface: Surface2::Construction,
        session_id,
        session_ordinal: Some(session),
        build: current_build_identity(),
    };
    let clock = InstantClock::new();
    let seed = identity::session_seed(identity::campaign_seed(AUTHORITY_SHA), "construction", session);
    let mut observations = 0u64;

    for (ordinal, case) in workload.clean_state.iter().enumerate() {
        let source = Source::new(SourceId(0), case.source_text.clone());
        let reference = reference_for(&case.source_text)?;
        for (horse_ordinal, horse_id) in HORSE_IDS.iter().copied().enumerate() {
            let mechanism_id = horse_mechanism_id(horse_id)?;
            let hook = ReferenceOracle::new(reference.clone());
            let facts = CaseSpec::full_parse(
                case.case_id,
                case.case_id_hex.clone(),
                case.payload_id.clone(),
                case.source_text.clone(),
            )
            .facts(mechanism_id, seed);
            markit_mdbench_campaign2::with_horse!(horse_id, |mech| {
                for iteration in 0..(WARMUP_ITERATIONS + measured) {
                    let (kind, ordinal_in_kind) = if iteration < WARMUP_ITERATIONS {
                        (SampleKind2::Warmup, iteration)
                    } else {
                        (SampleKind2::Measured, iteration - WARMUP_ITERATIONS)
                    };
                    let (row, repr) = markit_mdbench_campaign2::exec::construction_row(
                        &mech,
                        &source,
                        &clock,
                        &hook,
                        &facts,
                        &emitter.exec_identity,
                        &emitter.build,
                    );
                    emitter.emit(
                        &mut out,
                        ordinal as u32,
                        horse_ordinal as u32,
                        horse_id,
                        kind,
                        ordinal_in_kind,
                        &case.case_id_hex,
                        None,
                        None,
                        repr,
                        row,
                    )?;
                    observations += 1;
                }
                Ok::<(), String>(())
            })?;
        }
    }
    finish_lane(out, &out_path, observations)
}

fn cmd_run_resident_update(root: &Path, flags: &[String]) -> Result<(), String> {
    let session = flag_u32(flags, "--session", 0);
    let (exec_identity, session_id) = identity_for(
        root,
        "resident_update",
        EvidenceClass::PrimaryTiming.as_str(),
        Some(session),
        "timing",
    )?;
    let workload = load_campaign_workload(root)?;
    let out_path = out_path(root, flags, "resident-update", &format!("session-{session}-resident-update.jsonl"))?;
    let mut out = new_writer(&out_path)?;
    let emitter = Emitter {
        exec_identity,
        surface: Surface2::ResidentUpdate,
        session_id,
        session_ordinal: Some(session),
        build: current_build_identity(),
    };
    let clock = InstantClock::new();
    let seed = identity::session_seed(identity::campaign_seed(AUTHORITY_SHA), "resident_update", session);
    let mut observations = 0u64;
    for (ordinal, case) in workload.edit_write.iter().enumerate() {
        let spec_case = CaseSpec::update(
            case.case_id,
            case.case_id_hex.clone(),
            case.payload_id.clone(),
            case.pre_source_text.clone(),
            case.post_source_text.clone(),
            case.edit.clone(),
        );
        let reference = reference_for(&case.post_source_text)?;
        for (horse_ordinal, horse_id) in HORSE_IDS.iter().copied().enumerate() {
            let mechanism_id = horse_mechanism_id(horse_id)?;
            let hook = ReferenceOracle::new(reference.clone());
            let facts = spec_case.facts(mechanism_id, seed);
            markit_mdbench_campaign2::with_horse!(horse_id, |mech| {
                for iteration in 0..(WARMUP_ITERATIONS + MEASURED_ITERATIONS) {
                    let (kind, ordinal_in_kind) = if iteration < WARMUP_ITERATIONS {
                        (SampleKind2::Warmup, iteration)
                    } else {
                        (SampleKind2::Measured, iteration - WARMUP_ITERATIONS)
                    };
                    let row = markit_mdbench_campaign2::exec::resident_update_row(
                        &mech,
                        &spec_case,
                        &clock,
                        &hook,
                        &facts,
                        &emitter.exec_identity,
                        &emitter.build,
                    )?;
                    emitter.emit(
                        &mut out,
                        ordinal as u32,
                        horse_ordinal as u32,
                        horse_id,
                        kind,
                        ordinal_in_kind,
                        &case.case_id_hex,
                        None,
                        None,
                        None,
                        row,
                    )?;
                    observations += 1;
                }
                Ok::<(), String>(())
            })?;
        }
    }
    finish_lane(out, &out_path, observations)
}

fn cmd_run_lifecycle(root: &Path, flags: &[String]) -> Result<(), String> {
    let session = flag_u32(flags, "--session", 0);
    let (exec_identity, session_id) = identity_for(
        root,
        "lifecycle",
        EvidenceClass::PrimaryLifecycle.as_str(),
        Some(session),
        "lifecycle",
    )?;
    let spec_text = std::fs::read_to_string(store::campaign_root(root).join("manifests/campaign-2-spec-v1.toml"))
        .map_err(|e| format!("read spec: {e}"))?;
    let campaign_spec: spec::Campaign2Spec =
        toml::from_str(&spec_text).map_err(|e| format!("parse spec: {e}"))?;
    let reps = flag_u32(flags, "--reps", campaign_spec.sampling.lifecycle_repetitions);
    let only_trace = flag_value(flags, "--trace");
    let workload = load_campaign_workload(root)?;
    let mut traces = load_frozen_traces(root)?;
    if let Some(trace) = &only_trace {
        traces.retain(|t| &t.trace_id == trace);
    }
    let out_path = out_path(root, flags, "lifecycle", &format!("session-{session}-lifecycle.jsonl"))?;
    let mut out = new_writer(&out_path)?;
    let emitter = Emitter {
        exec_identity,
        surface: Surface2::Lifecycle,
        session_id,
        session_ordinal: Some(session),
        build: current_build_identity(),
    };
    let clock = InstantClock::new();
    let seed = identity::session_seed(identity::campaign_seed(AUTHORITY_SHA), "lifecycle", session);
    let mut observations = 0u64;
    let checkpoints: std::collections::BTreeSet<u32> = lifecycle::K_CHECKPOINTS.iter().copied().collect();

    for (ordinal, trace) in traces.iter().enumerate() {
        let trace_case_id = sha256_hex(trace.trace_id.as_bytes());
        let initial = trace_initial_source(trace, &workload)?;
        if sha256_hex(initial.as_bytes()) != trace.initial_source_sha256 {
            return Err(format!("trace {}: initial source drifted", trace.trace_id));
        }
        // Materialize the step sources and the (type-independent)
        // reference documents ONCE per trace, outside every timer.
        let mut step_sources = Vec::with_capacity(trace.steps.len());
        let mut references = Vec::with_capacity(trace.steps.len());
        let mut current = initial.clone();
        for step in &trace.steps {
            let edit = step.edit();
            let current_source = Source::new(SourceId(0), current.clone());
            let post = edit
                .apply(&current_source, SourceId(1))
                .map_err(|e| format!("trace {} step {}: {e:?}", trace.trace_id, step.step))?
                .as_str()
                .to_string();
            if sha256_hex(post.as_bytes()) != step.post_source_sha256 {
                return Err(format!(
                    "trace {} step {}: frozen post-source digest mismatch",
                    trace.trace_id, step.step
                ));
            }
            references.push(reference_for(&post)?);
            step_sources.push((
                Source::new(SourceId(0), current.clone()),
                Source::new(SourceId(1), post.clone()),
                edit,
            ));
            current = post;
        }
        let initial_source = Source::new(SourceId(9), initial.clone());

        for (horse_ordinal, horse_id) in HORSE_IDS.iter().copied().enumerate() {
            let mechanism_id = horse_mechanism_id(horse_id)?;
            let fact_list: Vec<markit_mdbench_runner::CaseFacts> = step_sources
                .iter()
                .enumerate()
                .map(|(index, (pre, post, edit))| {
                    Ok(lifecycle_case_spec(
                        &trace.trace_id,
                        step_index_payload(&trace.trace_id, index),
                        pre,
                        post,
                        edit,
                        &trace.steps[index],
                        seed,
                    )?
                    .facts(mechanism_id, seed))
                })
                .collect::<Result<Vec<_>, String>>()?;
            for rep in 0..reps {
                let outcome = markit_mdbench_campaign2::with_horse!(horse_id, |mech| {
                    let hooks: Vec<ReferenceOracle> =
                        references.iter().cloned().map(ReferenceOracle::new).collect();
                    markit_mdbench_campaign2::exec::lifecycle_run(
                        &mech,
                        &initial_source,
                        &step_sources,
                        &fact_list,
                        &clock,
                        &hooks,
                        &emitter.exec_identity,
                        &emitter.build,
                    )
                })?;
                let step_count = outcome.steps.len() as u32;
                for step_outcome in &outcome.steps {
                    let index = step_outcome.step as usize;
                    let step = &trace.steps[index];
                    let lifecycle_identity = LifecycleIdentityV1 {
                        trace_id: trace.trace_id.clone(),
                        family: trace.family.clone(),
                        chain_construction: trace.chain_construction.clone(),
                        step: step.step,
                        step_count: trace.step_count,
                        rep,
                        checkpoint: checkpoints.contains(&(step.step + 1)).then_some(step.step + 1),
                        cumulative_edits: step.step + 1,
                        step_label: step.label.clone(),
                        transition_label: step.transition_label.clone(),
                    };
                    // `iteration_ordinal` is unique across the whole run:
                    // rep * step_count + step.
                    let iteration_ordinal = rep * trace.step_count + step.step;
                    emitter.emit(
                        &mut out,
                        ordinal as u32,
                        horse_ordinal as u32,
                        horse_id,
                        if lifecycle_identity.checkpoint.is_some() {
                            SampleKind2::LifecycleCheckpoint
                        } else {
                            SampleKind2::LifecycleStep
                        },
                        iteration_ordinal,
                        &trace_case_id,
                        None,
                        Some(lifecycle_identity),
                        if step_outcome.step + 1 == step_count {
                            outcome.final_state_repr.clone()
                        } else {
                            None
                        },
                        step_outcome.row.clone(),
                    )?;
                    observations += 1;
                }
                if step_count as usize != step_sources.len() {
                    return Err(format!(
                        "trace {} horse {horse_id} rep {rep}: chain stopped after {step_count} of {} steps",
                        trace.trace_id,
                        step_sources.len()
                    ));
                }
            }
        }
    }
    finish_lane(out, &out_path, observations)
}

fn step_index_payload(trace_id: &str, index: usize) -> String {
    format!("c2-lifecycle:{trace_id}:{index}")
}

#[allow(clippy::too_many_arguments)]
fn lifecycle_case_spec(
    trace_id: &str,
    payload_id: String,
    pre: &Source,
    post: &Source,
    edit: &markit_mdbench_common::CanonicalEdit,
    step: &TraceStepV1,
    seed: u64,
) -> Result<CaseSpec, String> {
    let _ = trace_id;
    let operation = OperationKind::classify(edit);
    let key = CaseKeyV1 {
        payload_id,
        payload_shape: PayloadShape::Mixed,
        payload_size_bytes: pre.len_bytes() as u64,
        old_source_sha256: hex32(&sha256_hex(pre.as_bytes()))?,
        operation,
        edit_start_byte: Some(edit.start_byte()),
        edit_end_byte: Some(edit.end_byte()),
        inserted_text_sha256: operation
            .has_inserted_text()
            .then(|| hex32(&sha256_hex(edit.inserted_text().as_bytes())).ok())
            .flatten(),
        generator_id: Some("CAMPAIGN-2-LIFECYCLE".to_string()),
        generator_seed: Some(seed),
    }
    .validated()
    .map_err(|e| format!("lifecycle case key {trace_id} step {}: {e:?}", step.step))?;
    let _ = post;
    Ok(CaseSpec::update(
        CaseId::from_key(&key),
        key.payload_id.clone(),
        key.payload_id.clone(),
        pre.as_str().to_string(),
        post.as_str().to_string(),
        edit.clone(),
    ))
}

fn cmd_run_controlled(root: &Path, flags: &[String]) -> Result<(), String> {
    let axis = Axis::parse(&flag_value(flags, "--axis").ok_or_else(|| "--axis is required".to_string())?)?;
    let session = flag_u32(flags, "--session", 0);
    let (exec_identity, session_id) = identity_for(
        root,
        axis.sub_campaign_tag(),
        EvidenceClass::ControlledTiming.as_str(),
        Some(session),
        "timing",
    )?;
    let spec_text = std::fs::read_to_string(store::campaign_root(root).join("manifests/campaign-2-spec-v1.toml"))
        .map_err(|e| format!("read spec: {e}"))?;
    let campaign_spec: spec::Campaign2Spec =
        toml::from_str(&spec_text).map_err(|e| format!("parse spec: {e}"))?;
    let measured = flag_u32(flags, "--reps", campaign_spec.sampling.controlled_repetitions);
    let warmup = campaign_spec.sampling.warmup_iterations;
    let cells: Vec<ControlledCase> = match flag_value(flags, "--cell") {
        Some(label) => vec![generators::generate_cell(axis, &label)?],
        None => generators::generate_axis(axis)?,
    };
    let out_path = out_path(
        root,
        flags,
        &format!("controlled/{}", axis_tag(axis)),
        &format!("session-{session}-{}-controlled.jsonl", axis.as_str().to_lowercase()),
    )?;
    let mut out = new_writer(&out_path)?;
    let emitter = Emitter {
        exec_identity,
        surface: Surface2::Controlled,
        session_id,
        session_ordinal: Some(session),
        build: current_build_identity(),
    };
    let clock = InstantClock::new();
    let seed = identity::session_seed(
        identity::campaign_seed(AUTHORITY_SHA),
        axis.sub_campaign_tag(),
        session,
    );
    let mut observations = 0u64;
    for (ordinal, cell) in cells.iter().enumerate() {
        generators::verify_case(cell)?;
        let spec_case = markit_mdbench_campaign2::controlled::case_spec(cell);
        let reference = reference_for(&cell.post_source)?;
        for (horse_ordinal, horse_id) in HORSE_IDS.iter().copied().enumerate() {
            let mechanism_id = horse_mechanism_id(horse_id)?;
            let hook = ReferenceOracle::new(reference.clone());
            let facts = spec_case.facts(mechanism_id, seed);
            let cell_identity = CellIdentityV1 {
                axis: axis.as_str().to_string(),
                axis_point_index: cell.axis_point_index,
                axis_label: cell.axis_label.clone(),
                axis_value: cell.axis_value,
                cell_id: cell.cell_id.clone(),
                generator_id: generators::GENERATOR_ID.to_string(),
                generator_version: generators::GENERATOR_VERSION.to_string(),
            };
            markit_mdbench_campaign2::with_horse!(horse_id, |mech| {
                for iteration in 0..(warmup + measured) {
                    let (kind, ordinal_in_kind) = if iteration < warmup {
                        (SampleKind2::Warmup, iteration)
                    } else {
                        (SampleKind2::Measured, iteration - warmup)
                    };
                    let row = markit_mdbench_campaign2::exec::resident_update_row(
                        &mech,
                        &spec_case,
                        &clock,
                        &hook,
                        &facts,
                        &emitter.exec_identity,
                        &emitter.build,
                    )?;
                    emitter.emit(
                        &mut out,
                        ordinal as u32,
                        horse_ordinal as u32,
                        horse_id,
                        kind,
                        ordinal_in_kind,
                        &cell.case_id_hex,
                        Some(cell_identity.clone()),
                        None,
                        None,
                        row,
                    )?;
                    observations += 1;
                }
                Ok::<(), String>(())
            })?;
        }
    }
    finish_lane(out, &out_path, observations)
}

fn cmd_run_attribution(root: &Path, flags: &[String]) -> Result<(), String> {
    let surface = flag_value(flags, "--surface").unwrap_or_else(|| "construction".to_string());
    let workload = load_campaign_workload(root)?;
    let out_dir = store::ensure_layout(root)?;
    let (tag, evidence, axis) = match surface.as_str() {
        "construction" => ("construction", EvidenceClass::PrimaryWork.as_str(), None),
        "resident_update" => ("resident_update", EvidenceClass::PrimaryWork.as_str(), None),
        "controlled" => (
            "controlled",
            EvidenceClass::ControlledWork.as_str(),
            Some(Axis::parse(
                &flag_value(flags, "--axis").ok_or_else(|| "--axis is required".to_string())?,
            )?),
        ),
        other => return Err(format!("unknown attribution surface {other:?}")),
    };
    let tag = axis.map(|a| a.sub_campaign_tag()).unwrap_or(tag);
    let (exec_identity, _) = identity_for(root, tag, evidence, None, "attribution")?;
    let path = match flag_value(flags, "--out") {
        Some(path) => PathBuf::from(path),
        None => out_dir.join(format!("{surface}.attribution.jsonl")),
    };
    let mut out = new_writer(&path)?;
    let emitter = Emitter {
        exec_identity,
        surface: match surface.as_str() {
            "construction" => Surface2::Construction,
            "resident_update" => Surface2::ResidentUpdate,
            _ => Surface2::Controlled,
        },
        session_id: None,
        session_ordinal: None,
        build: current_build_identity(),
    };
    let mut observations = 0u64;
    match surface.as_str() {
        "construction" => {
            for (ordinal, case) in workload.clean_state.iter().enumerate() {
                let source = Source::new(SourceId(0), case.source_text.clone());
                let reference = reference_for(&case.source_text)?;
                for (horse_ordinal, horse_id) in HORSE_IDS.iter().copied().enumerate() {
                    let mechanism_id = horse_mechanism_id(horse_id)?;
                    let hook = ReferenceOracle::new(reference.clone());
                    let facts = CaseSpec::full_parse(
                        case.case_id,
                        case.case_id_hex.clone(),
                        case.payload_id.clone(),
                        case.source_text.clone(),
                    )
                    .facts(mechanism_id, 0);
                    markit_mdbench_campaign2::with_horse!(horse_id, |mech| {
                        let row = markit_mdbench_campaign2::exec::construction_attributed_row(
                            &mech,
                            &source,
                            &hook,
                            &facts,
                            &emitter.exec_identity,
                            &emitter.build,
                        );
                        emitter.emit(
                            &mut out,
                            ordinal as u32,
                            horse_ordinal as u32,
                            horse_id,
                            SampleKind2::Attribution,
                            0,
                            &case.case_id_hex,
                            None,
                            None,
                            None,
                            row,
                        )?;
                        observations += 1;
                        Ok::<(), String>(())
                    })?;
                }
            }
        }
        "resident_update" => {
            for (ordinal, case) in workload.edit_write.iter().enumerate() {
                let spec_case = CaseSpec::update(
                    case.case_id,
                    case.case_id_hex.clone(),
                    case.payload_id.clone(),
                    case.pre_source_text.clone(),
                    case.post_source_text.clone(),
                    case.edit.clone(),
                );
                let reference = reference_for(&case.post_source_text)?;
                for (horse_ordinal, horse_id) in HORSE_IDS.iter().copied().enumerate() {
                    let mechanism_id = horse_mechanism_id(horse_id)?;
                    let hook = ReferenceOracle::new(reference.clone());
                    let facts = spec_case.facts(mechanism_id, 0);
                    markit_mdbench_campaign2::with_horse!(horse_id, |mech| {
                        let row = markit_mdbench_campaign2::exec::update_attributed_row(
                            &mech,
                            &spec_case,
                            &hook,
                            &facts,
                            &emitter.exec_identity,
                            &emitter.build,
                        )?;
                        emitter.emit(
                            &mut out,
                            ordinal as u32,
                            horse_ordinal as u32,
                            horse_id,
                            SampleKind2::Attribution,
                            0,
                            &case.case_id_hex,
                            None,
                            None,
                            None,
                            row,
                        )?;
                        observations += 1;
                        Ok::<(), String>(())
                    })?;
                }
            }
        }
        _ => {
            let axis = axis.expect("controlled attribution carries an axis");
            for (ordinal, cell) in generators::generate_axis(axis)?.iter().enumerate() {
                let spec_case = markit_mdbench_campaign2::controlled::case_spec(cell);
                let reference = reference_for(&cell.post_source)?;
                for (horse_ordinal, horse_id) in HORSE_IDS.iter().copied().enumerate() {
                    let mechanism_id = horse_mechanism_id(horse_id)?;
                    let hook = ReferenceOracle::new(reference.clone());
                    let facts = spec_case.facts(mechanism_id, 0);
                    let cell_identity = CellIdentityV1 {
                        axis: axis.as_str().to_string(),
                        axis_point_index: cell.axis_point_index,
                        axis_label: cell.axis_label.clone(),
                        axis_value: cell.axis_value,
                        cell_id: cell.cell_id.clone(),
                        generator_id: generators::GENERATOR_ID.to_string(),
                        generator_version: generators::GENERATOR_VERSION.to_string(),
                    };
                    markit_mdbench_campaign2::with_horse!(horse_id, |mech| {
                        let row = markit_mdbench_campaign2::exec::update_attributed_row(
                            &mech,
                            &spec_case,
                            &hook,
                            &facts,
                            &emitter.exec_identity,
                            &emitter.build,
                        )?;
                        emitter.emit(
                            &mut out,
                            ordinal as u32,
                            horse_ordinal as u32,
                            horse_id,
                            SampleKind2::Attribution,
                            0,
                            &cell.case_id_hex,
                            Some(cell_identity.clone()),
                            None,
                            None,
                            row,
                        )?;
                        observations += 1;
                        Ok::<(), String>(())
                    })?;
                }
            }
        }
    }
    finish_lane(out, &path, observations)
}

fn cmd_run_memory(root: &Path, flags: &[String]) -> Result<(), String> {
    let mode = flag_value(flags, "--mode").unwrap_or_else(|| "construction".to_string());
    let out_dir = store::ensure_layout(root)?;
    let path = flag_value(flags, "--out")
        .map(PathBuf::from)
        .unwrap_or_else(|| out_dir.join(format!("memory-{mode}.jsonl")));
    let workload = load_campaign_workload(root)?;
    let mut probe = markit_mdbench_campaign2::memory::MemoryProbe::new();
    match mode.as_str() {
        "construction" => {
            for case in workload.clean_state.iter() {
                probe.probe_construction(&case.case_id_hex[..16], "ALL")?;
            }
        }
        "resident_update" => {
            for case in workload.edit_write.iter().take(64) {
                probe.probe_construction(&case.case_id_hex[..16], "ALL")?;
            }
        }
        other => return Err(format!("unknown memory mode {other:?}")),
    }
    let mut out = new_writer(&path)?;
    for sample in &probe.samples {
        let line = serde_json::to_string(sample).map_err(|e| format!("serialize: {e}"))?;
        out.write_all(line.as_bytes()).map_err(|e| format!("write: {e}"))?;
        out.write_all(b"\n").map_err(|e| format!("write: {e}"))?;
    }
    out.flush().map_err(|e| format!("flush: {e}"))?;
    drop(out);
    let sha = markit_mdbench_campaign2::sha256_file(&path)?;
    println!(
        "DESCRIPTIVE_MEMORY_LANE_COMPLETE rows={} path={} sha256={sha}",
        probe.samples.len(),
        path.display()
    );
    Ok(())
}

// ---------------------------------------------------------------------------
// Pilot
// ---------------------------------------------------------------------------

fn cmd_pilot(root: &Path, flags: &[String]) -> Result<(), String> {
    let out_dir = store::ensure_layout(root)?;
    let pilot_dir = out_dir.join("pilot");
    std::fs::create_dir_all(&pilot_dir).map_err(|e| format!("mkdir pilot: {e}"))?;
    let workload = load_campaign_workload(root)?;
    let frozen = markit_mdbench_campaign2::load_frozen_identity(root)?;
    let build = current_build_identity();
    let executable = current_executable_sha256()?;
    let machine_sha =
        markit_mdbench_campaign2::sha256_file(&out_dir.join("manifests/campaign-2-machine-v1.toml"))?;
    let pilot_spec_id = sha256_hex(format!("{}-PILOT", frozen.campaign_spec_id).as_bytes());
    let pilot_run_id = identity::run_id(
        &frozen.study_id,
        &pilot_spec_id,
        "pilot",
        &build.runner_git_commit,
        &machine_sha,
        &build,
        &executable,
    );
    let pilot_identity = ExecutionIdentity2 {
        study_id: frozen.study_id.clone(),
        campaign_spec_id: pilot_spec_id.clone(),
        sub_campaign_spec_id: "pilot".to_string(),
        run_id: pilot_run_id,
        evidence_class: "PILOT_NON_RESEARCH",
        machine_environment_ref: machine_sha,
        provenance: "CAMPAIGN2_PILOT/NON_RESEARCH_RESULT",
        non_research: true,
    };
    let clock = InstantClock::new();
    let lines = flag_u32(flags, "--lines", 3);
    let mut report: Vec<String> = Vec::new();

    let case = &workload.clean_state[0];
    let source = Source::new(SourceId(0), case.source_text.clone());
    let reference = reference_for(&case.source_text)?;
    for horse in HORSE_IDS {
        let mechanism_id = horse_mechanism_id(horse)?;
        let hook = ReferenceOracle::new(reference.clone());
        markit_mdbench_campaign2::with_horse!(horse, |mech| {
            let mut best = u64::MAX;
            for _ in 0..lines {
                let report_row =
                    markit_mdbench_runner::orchestrate::run_full_parse_timed(&mech, &source, &clock, &hook);
                if let LaneMeasurement::Timing(t) = &report_row.measurement {
                    if let Observed::Known(ns) = t.total_ns {
                        best = best.min(ns);
                    }
                }
            }
            report.push(format!(
                "PILOT_CONSTRUCTION horse={horse} mechanism={mechanism_id} source_bytes={} best_total_ns={best}",
                case.source_text.len()
            ));
            Ok::<(), String>(())
        })?;
    }

    let case = &workload.edit_write[0];
    let spec_case = CaseSpec::update(
        case.case_id,
        case.case_id_hex.clone(),
        case.payload_id.clone(),
        case.pre_source_text.clone(),
        case.post_source_text.clone(),
        case.edit.clone(),
    );
    let reference = reference_for(&case.post_source_text)?;
    for horse in HORSE_IDS {
        let mechanism_id = horse_mechanism_id(horse)?;
        let hook = ReferenceOracle::new(reference.clone());
        let facts = spec_case.facts(mechanism_id, 0);
        markit_mdbench_campaign2::with_horse!(horse, |mech| {
            let mut best = u64::MAX;
            for _ in 0..lines {
                let row = markit_mdbench_campaign2::exec::resident_update_row(
                    &mech,
                    &spec_case,
                    &clock,
                    &hook,
                    &facts,
                    &pilot_identity,
                    &build,
                )?;
                if let MeasurementV1::Timing(t) = &row.measurement {
                    if let Observed::Known(ns) = t.total_ns {
                        best = best.min(ns);
                    }
                }
            }
            report.push(format!(
                "PILOT_RESIDENT_UPDATE horse={horse} mechanism={mechanism_id} pre_bytes={} best_total_ns={best}",
                case.pre_source_text.len()
            ));
            Ok::<(), String>(())
        })?;
    }

    let traces = load_frozen_traces(root)?;
    if let Some(trace) = traces.iter().find(|t| t.family == "L1") {
        let initial = trace_initial_source(trace, &workload)?;
        let mut current = initial.clone();
        let mut step_sources = Vec::new();
        let mut references = Vec::new();
        for step in trace.steps.iter().take(8) {
            let edit = step.edit();
            let current_source = Source::new(SourceId(0), current.clone());
            let post = edit
                .apply(&current_source, SourceId(1))
                .map_err(|e| format!("pilot lifecycle: {e:?}"))?
                .as_str()
                .to_string();
            references.push(reference_for(&post)?);
            step_sources.push((Source::new(SourceId(0), current.clone()), Source::new(SourceId(1), post.clone()), edit));
            current = post;
        }
        let initial_source = Source::new(SourceId(9), initial.clone());
        for horse in HORSE_IDS.iter().copied() {
            let mechanism_id = horse_mechanism_id(horse)?;
            let fact_list: Vec<markit_mdbench_runner::CaseFacts> = step_sources
                .iter()
                .enumerate()
                .map(|(index, (pre, post, edit))| {
                    lifecycle_case_spec(
                        &trace.trace_id,
                        step_index_payload(&trace.trace_id, index),
                        pre,
                        post,
                        edit,
                        &trace.steps[index],
                        0,
                    )
                    .expect("pilot lifecycle case spec")
                    .facts(mechanism_id, 0)
                })
                .collect();
            markit_mdbench_campaign2::with_horse!(horse, |mech| {
                let hooks: Vec<ReferenceOracle> =
                    references.iter().cloned().map(ReferenceOracle::new).collect();
                let outcome = markit_mdbench_campaign2::exec::lifecycle_run(
                    &mech,
                    &initial_source,
                    &step_sources,
                    &fact_list,
                    &clock,
                    &hooks,
                    &pilot_identity,
                    &build,
                )?;
                let mut total = 0u64;
                for step in &outcome.steps {
                    if let MeasurementV1::Timing(t) = &step.row.measurement {
                        if let Observed::Known(ns) = t.total_ns {
                            total += ns;
                        }
                    }
                }
                report.push(format!(
                    "PILOT_LIFECYCLE_K8 horse={horse} mechanism={mechanism_id} steps={} cumulative_total_ns={total}",
                    outcome.steps.len()
                ));
                Ok::<(), String>(())
            })?;
        }
    }

    for axis in [Axis::N, Axis::B, Axis::D, Axis::F, Axis::K] {
        let label = axis.point_labels()[0];
        let cell = generators::generate_cell(axis, label)?;
        generators::verify_case(&cell)?;
        let spec_case = markit_mdbench_campaign2::controlled::case_spec(&cell);
        let reference = reference_for(&cell.post_source)?;
        for horse in HORSE_IDS.iter().copied() {
            let mechanism_id = horse_mechanism_id(horse)?;
            let hook = ReferenceOracle::new(reference.clone());
            let facts = spec_case.facts(mechanism_id, 0);
            markit_mdbench_campaign2::with_horse!(horse, |mech| {
                let mut best = u64::MAX;
                for _ in 0..lines {
                    let row = markit_mdbench_campaign2::exec::resident_update_row(
                        &mech,
                        &spec_case,
                        &clock,
                        &hook,
                        &facts,
                        &pilot_identity,
                        &build,
                    )?;
                    if let MeasurementV1::Timing(t) = &row.measurement {
                        if let Observed::Known(ns) = t.total_ns {
                            best = best.min(ns);
                        }
                    }
                }
                report.push(format!(
                    "PILOT_CONTROLLED axis={} cell={} pre_bytes={} horse={horse} best_total_ns={best}",
                    axis.as_str(),
                    cell.cell_id,
                    cell.pre_source.len()
                ));
                Ok::<(), String>(())
            })?;
        }
    }

    // Largest controlled cell (16 MiB) to bound the formal controlled cost.
    let big = generators::generate_cell(Axis::N, "16MiB")?;
    generators::verify_case(&big)?;
    let spec_case = markit_mdbench_campaign2::controlled::case_spec(&big);
    let reference = reference_for(&big.post_source)?;
    for horse in HORSE_IDS {
        let mechanism_id = horse_mechanism_id(horse)?;
        let hook = ReferenceOracle::new(reference.clone());
        let facts = spec_case.facts(mechanism_id, 0);
        markit_mdbench_campaign2::with_horse!(horse, |mech| {
            let start = std::time::Instant::now();
            let row = markit_mdbench_campaign2::exec::resident_update_row(
                &mech,
                &spec_case,
                &clock,
                &hook,
                &facts,
                &pilot_identity,
                &build,
            )?;
            let elapsed = start.elapsed().as_nanos() as u64;
            let total = match &row.measurement {
                MeasurementV1::Timing(t) => match t.total_ns {
                    Observed::Known(ns) => ns,
                    _ => 0,
                },
                _ => 0,
            };
            report.push(format!(
                "PILOT_CONTROLLED_16MIB horse={horse} mechanism={mechanism_id} pre_bytes={} total_ns={total} wall_ns={elapsed}",
                big.pre_source.len()
            ));
            Ok::<(), String>(())
        })?;
    }

    let path = pilot_dir.join("pilot-report-v1.txt");
    std::fs::write(&path, report.join("\n") + "\n").map_err(|e| format!("write {}: {e}", path.display()))?;
    for line in &report {
        println!("{line}");
    }
    println!("CAMPAIGN2_PILOT_COMPLETE rows={} path={} NON_RESEARCH=YES", report.len(), path.display());
    Ok(())
}

fn cmd_finalize(root: &Path, flags: &[String]) -> Result<(), String> {
    let _ = root;
    let raw = flag_value(flags, "--raw").ok_or_else(|| "--raw <path> is required".to_string())?;
    let raw_path = PathBuf::from(&raw);
    let rows = markit_mdbench_campaign2::receipt::read_observations(&raw_path)?;
    if let Err(blockers) = markit_mdbench_campaign2::finalize::verify_all_correct(&rows) {
        for blocker in &blockers {
            eprintln!("CORRECTNESS_BLOCKER {blocker}");
        }
        return Err("correctness blockers found; the lane is INVALID".to_string());
    }
    let distinct = markit_mdbench_campaign2::receipt::distinct_identity(&rows);
    println!(
        "RAW_IDENTITY rows={} study={} campaign={} sub={} run={} duplicates={}",
        rows.len(),
        distinct.study_ids,
        distinct.campaign_spec_ids,
        distinct.sub_campaign_spec_ids,
        distinct.run_ids,
        distinct.duplicate_observation_ids
    );
    Ok(())
}

fn new_writer(path: &Path) -> Result<std::io::BufWriter<std::fs::File>, String> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| format!("mkdir {}: {e}", parent.display()))?;
    }
    Ok(std::io::BufWriter::with_capacity(
        1 << 20,
        std::fs::File::create(path).map_err(|e| format!("create {}: {e}", path.display()))?,
    ))
}

fn out_path(root: &Path, flags: &[String], subdir: &str, name: &str) -> Result<PathBuf, String> {
    if let Some(path) = flag_value(flags, "--out") {
        return Ok(PathBuf::from(path));
    }
    let dir = store::campaign_root(root).join(subdir);
    std::fs::create_dir_all(&dir).map_err(|e| format!("mkdir {}: {e}", dir.display()))?;
    Ok(dir.join(name))
}

fn finish_lane<W: std::io::Write>(
    mut out: std::io::BufWriter<W>,
    path: &Path,
    observations: u64,
) -> Result<(), String> {
    out.flush().map_err(|e| format!("flush {}: {e}", path.display()))?;
    drop(out);
    let sha = markit_mdbench_campaign2::sha256_file(path)?;
    println!(
        "FINAL_RAW_FILE_PASS file={} rows={} sha256={sha}",
        path.display(),
        observations
    );
    println!("LANE_COMPLETE observations={observations} path={}", path.display());
    Ok(())
}
