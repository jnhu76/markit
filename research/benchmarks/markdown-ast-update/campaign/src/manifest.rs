//! The frozen campaign manifest
//! (`results/manifests/primary-performance-campaign-v1.toml`) and its
//! verification (task §2, §9-§19, §40).
//!
//! The manifest freezes WHAT runs and HOW it is scheduled, aggregated,
//! and qualified — it binds no measured value. Verification recomputes
//! every binding against the live repository state (workload manifests,
//! environment authority, mechanism identities) and fails closed on
//! drift.

use serde::{Deserialize, Serialize};

use crate::{
    CAMPAIGN_ID, CAMPAIGN_MANIFEST_SCHEMA, HORSE_ROSTER, MACHINE_MANIFEST_SCHEMA,
    METRIC_QUALIFICATION,
};

/// Path of the campaign manifest below the benchmark root.
pub const CAMPAIGN_MANIFEST_PATH: &str = "results/manifests/primary-performance-campaign-v1.toml";

/// Path of the machine manifest below the benchmark root.
pub const MACHINE_MANIFEST_PATH: &str = "results/manifests/primary-machine-v1.toml";

/// Path of the schedule manifest below the benchmark root.
pub const SCHEDULE_MANIFEST_PATH: &str = "results/manifests/primary-schedule-v1.jsonl";

/// Path of the campaign receipt below the benchmark root.
pub const CAMPAIGN_RECEIPT_PATH: &str = "results/manifests/primary-campaign-receipt-v1.json";

/// Path of the raw observation envelope schema below the benchmark root.
pub const ENVELOPE_SCHEMA_PATH: &str = "protocol/campaign-observation-schema-v1.json";

/// Frozen quantile ranks with exactly 30 measured observations
/// (1-indexed nearest-rank; task §29): p50 rank 15, p95 rank 29.
pub const P50_RANK: usize = 15;
pub const P95_RANK: usize = 29;
pub const QUANTILE_SAMPLE_SIZE: usize = 30;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SurfaceSpec {
    pub kind: String,
    pub case_count: usize,
    pub horse_count: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SessionsSpec {
    pub count: u32,
    pub warmup_iterations: u32,
    pub measured_iterations: u32,
    /// Sessions are fresh worker processes (task §11-§12); CLEAN_STATE
    /// and EDIT_WRITE never interleave inside one process.
    pub separate_processes_per_surface: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct HorseSpec {
    pub id: String,
    pub mechanism_id: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SeedSpec {
    pub derivation: String,
    pub algorithm: String,
    /// First 64 bits of the SHA256 digest, big-endian.
    pub byte_order: String,
    pub base_authority_sha: String,
    pub value: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct OrderingSpec {
    pub case_algorithm: String,
    pub horse_policy: String,
    pub session_seed_domain: String,
    pub horse_base_seed_domain: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct QuantileSpec {
    pub method: String,
    pub sample_size: usize,
    pub p50_rank: usize,
    pub p95_rank: usize,
    pub indexing: String,
    pub interpolation: bool,
    pub note: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CardinalitySpec {
    pub logical_cells_per_session: usize,
    pub timing_rows_per_session: usize,
    pub total_timing_rows: usize,
    pub measured_rows_total: usize,
    pub warmup_rows_total: usize,
    pub attribution_rows: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RefsSpec {
    pub machine_manifest: String,
    pub schedule_manifest: String,
    pub campaign_receipt: String,
    pub envelope_schema: String,
    pub result_schema_json: String,
    pub result_schema_doc: String,
    pub environment_record: String,
    pub workload_freeze_receipt: String,
    pub protocol_doc: String,
}

/// The full campaign manifest (serde-toml model).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CampaignManifest {
    pub schema: String,
    pub campaign_id: String,
    /// PR #41 MEASUREMENT-SUBSTRATE-CORRECTIVE merge SHA the freeze is
    /// rooted at (or a recorded descendant).
    pub base_authority_sha: String,
    /// Primary trace semantics (task §4).
    pub trace_semantics: String,
    pub surfaces: SurfacesMap,
    pub sessions: SessionsSpec,
    pub horses: Vec<HorseSpec>,
    pub seed: SeedSpec,
    pub ordering: OrderingSpec,
    pub quantiles: QuantileSpec,
    #[serde(rename = "metric")]
    pub metrics: Vec<MetricSpec>,
    pub aggregation: Vec<String>,
    pub failure_policy: Vec<String>,
    pub cardinality: CardinalitySpec,
    pub refs: RefsSpec,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SurfacesMap {
    pub clean_state: SurfaceSpec,
    pub edit_write: SurfaceSpec,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MetricSpec {
    pub name: String,
    pub status: String,
}

impl CampaignManifest {
    /// Load the campaign manifest from the benchmark root.
    pub fn load(benchmark_root: &std::path::Path) -> Result<Self, String> {
        let path = benchmark_root.join(CAMPAIGN_MANIFEST_PATH);
        let text =
            std::fs::read_to_string(&path).map_err(|e| format!("read {}: {e}", path.display()))?;
        toml::from_str(&text).map_err(|e| format!("parse {}: {e}", path.display()))
    }

    /// Total logical case × horse cells per session (task §10).
    pub fn cells_per_session(&self) -> usize {
        (self.surfaces.clean_state.case_count + self.surfaces.edit_write.case_count)
            * self.surfaces.clean_state.horse_count
    }

    /// Timing observations per session including warmups.
    pub fn timing_rows_per_session(&self) -> usize {
        self.cells_per_session()
            * (self.sessions.warmup_iterations + self.sessions.measured_iterations) as usize
    }

    /// Verify every frozen binding against live repository state.
    /// Returns a blocker list (empty = OK).
    pub fn verify(&self, benchmark_root: &std::path::Path) -> Result<(), Vec<String>> {
        let mut blockers = Vec::new();

        if self.schema != CAMPAIGN_MANIFEST_SCHEMA {
            blockers.push(format!(
                "schema {:?} != {CAMPAIGN_MANIFEST_SCHEMA}",
                self.schema
            ));
        }
        if self.campaign_id != CAMPAIGN_ID {
            blockers.push(format!(
                "campaign_id {:?} != {CAMPAIGN_ID}",
                self.campaign_id
            ));
        }
        if self.trace_semantics != "SINGLE_RESET" {
            blockers.push(format!(
                "trace_semantics {:?} != SINGLE_RESET (chained editing is not this campaign)",
                self.trace_semantics
            ));
        }
        if self.base_authority_sha.is_empty()
            || self.base_authority_sha.len() != 40
            || !self
                .base_authority_sha
                .bytes()
                .all(|b| b.is_ascii_hexdigit())
        {
            blockers.push("base_authority_sha must be a 40-hex git SHA".to_string());
        }

        // Surfaces: exactly the frozen primary workload cardinalities.
        if self.surfaces.clean_state.kind != "clean_state"
            || self.surfaces.clean_state.case_count
                != crate::Surface::CleanState.frozen_case_count()
        {
            blockers.push(format!(
                "clean_state surface {} x {} != 22 x 5",
                self.surfaces.clean_state.kind, self.surfaces.clean_state.case_count
            ));
        }
        if self.surfaces.edit_write.kind != "edit_write"
            || self.surfaces.edit_write.case_count != crate::Surface::EditWrite.frozen_case_count()
        {
            blockers.push(format!(
                "edit_write surface {} x {} != 362 x 5",
                self.surfaces.edit_write.kind, self.surfaces.edit_write.case_count
            ));
        }
        if self.surfaces.clean_state.horse_count != 5 || self.surfaces.edit_write.horse_count != 5 {
            blockers.push("both surfaces must dispatch all five horses H0-H4".to_string());
        }

        // Sessions: the frozen R0 sampling policy (task §9).
        if self.sessions.count != 3 {
            blockers.push(format!("session count {} != 3", self.sessions.count));
        }
        if self.sessions.warmup_iterations != 10 || self.sessions.measured_iterations != 30 {
            blockers.push(format!(
                "iterations warmup={} measured={} != 10/30",
                self.sessions.warmup_iterations, self.sessions.measured_iterations
            ));
        }
        if !self.sessions.separate_processes_per_surface {
            blockers
                .push("CLEAN_STATE and EDIT_WRITE must not interleave in one worker".to_string());
        }

        // Horses: exactly the frozen roster with the crates' own ids.
        if self.horses.len() != HORSE_ROSTER.len() {
            blockers.push(format!(
                "horse roster has {} entries != 5",
                self.horses.len()
            ));
        } else {
            for (spec, entry) in self.horses.iter().zip(HORSE_ROSTER.iter()) {
                if spec.id != entry.id || spec.mechanism_id != entry.mechanism_id {
                    blockers.push(format!(
                        "horse {} -> {:?} does not match the frozen mechanism roster {} -> {}",
                        spec.id, spec.mechanism_id, entry.id, entry.mechanism_id
                    ));
                }
            }
        }

        // Seed: recomputation must match the recorded value.
        let recomputed = crate::identity::campaign_seed(&self.base_authority_sha);
        if self.seed.value != recomputed {
            blockers.push(format!(
                "campaign seed {} != recomputed {recomputed} for base authority {}",
                self.seed.value, self.base_authority_sha
            ));
        }
        if self.seed.base_authority_sha != self.base_authority_sha {
            blockers.push(format!(
                "seed.base_authority_sha {:?} != top-level base_authority_sha {:?}",
                self.seed.base_authority_sha, self.base_authority_sha
            ));
        }
        if self.seed.algorithm != crate::identity::CAMPAIGN_SEED_ALGORITHM_ID
            || self.seed.byte_order != "big-endian"
        {
            blockers.push(format!(
                "seed algorithm {}/{} drifted from the frozen derivation",
                self.seed.algorithm, self.seed.byte_order
            ));
        }

        // Ordering algorithms: exactly the runner-frozen ones.
        if self.ordering.case_algorithm != markit_mdbench_runner::SHUFFLE_ALGORITHM_ID {
            blockers.push(format!(
                "case_algorithm {:?} != runner-frozen {:?}",
                self.ordering.case_algorithm,
                markit_mdbench_runner::SHUFFLE_ALGORITHM_ID
            ));
        }
        if self.ordering.horse_policy != crate::schedule::HORSE_ORDER_POLICY_ID {
            blockers.push(format!(
                "horse_policy {:?} != {:?}",
                self.ordering.horse_policy,
                crate::schedule::HORSE_ORDER_POLICY_ID
            ));
        }
        if self.ordering.session_seed_domain != crate::identity::SESSION_SEED_DOMAIN
            || self.ordering.horse_base_seed_domain != crate::identity::HORSE_BASE_SEED_DOMAIN
        {
            blockers.push("ordering seed domains drifted from the frozen identities".to_string());
        }

        // Quantiles: nearest-rank, frozen ranks, no interpolation.
        let q = &self.quantiles;
        if q.method != "nearest-rank"
            || q.sample_size != QUANTILE_SAMPLE_SIZE
            || q.p50_rank != P50_RANK
            || q.p95_rank != P95_RANK
            || q.indexing != "1-indexed"
            || q.interpolation
        {
            blockers.push(format!(
                "quantile policy drifted: {q:?} (nearest-rank, n=30, ranks 15/29, 1-indexed, no interpolation)"
            ));
        }

        // Metrics: exactly the frozen qualification table.
        let frozen: Vec<(String, String)> = METRIC_QUALIFICATION
            .iter()
            .map(|(m, s)| (m.to_string(), s.to_string()))
            .collect();
        let declared: Vec<(String, String)> = self
            .metrics
            .iter()
            .map(|m| (m.name.clone(), m.status.clone()))
            .collect();
        if declared != frozen {
            blockers.push(format!(
                "metric qualification table drifted from the frozen #41 table: {declared:?}"
            ));
        }

        // Cardinality: exact arithmetic (task §10).
        let cells = self.cells_per_session();
        let per_session = self.timing_rows_per_session();
        let total = per_session * self.sessions.count as usize;
        let measured =
            cells * self.sessions.measured_iterations as usize * self.sessions.count as usize;
        let warmup =
            cells * self.sessions.warmup_iterations as usize * self.sessions.count as usize;
        let attribution =
            (self.surfaces.clean_state.case_count + self.surfaces.edit_write.case_count) * 5;
        let c = &self.cardinality;
        if (
            c.logical_cells_per_session,
            c.timing_rows_per_session,
            c.total_timing_rows,
            c.measured_rows_total,
            c.warmup_rows_total,
            c.attribution_rows,
        ) != (cells, per_session, total, measured, warmup, attribution)
        {
            blockers.push(format!(
                "cardinality drift: declared {c:?} != computed cells={cells} per_session={per_session} total={total} measured={measured} warmup={warmup} attribution={attribution}"
            ));
        }

        // Aggregation + failure policy keywords.
        let aggregation = self.aggregation.join(";");
        if !aggregation.contains("CASE_WEIGHTED")
            || !aggregation.contains("PROJECT_MACRO")
            || !aggregation.contains("FAMILY_MACRO")
        {
            blockers.push(
                "aggregation policy must freeze CASE_WEIGHTED / PROJECT_MACRO / FAMILY_MACRO"
                    .to_string(),
            );
        }
        let failure = self.failure_policy.join(";");
        if !failure.contains("fail-closed")
            || !failure.contains("no-outlier-deletion")
            || !failure.contains("PRIMARY_CAMPAIGN_INVALID")
        {
            blockers.push("failure policy must freeze fail-closed / no-outlier-deletion / PRIMARY_CAMPAIGN_INVALID".to_string());
        }

        // Cross-artifact authorities: result schema v2 + environment echo.
        if self.refs.result_schema_json != "protocol/result-schema-v2.json" {
            blockers
                .push("refs.result_schema_json must be protocol/result-schema-v2.json".to_string());
        }
        let env_path = benchmark_root.join("manifest/environment.toml");
        let env_text = std::fs::read_to_string(&env_path)
            .map_err(|e| format!("read {}: {e}", env_path.display()))
            .unwrap_or_default();
        if !env_text
            .contains("shuffle_algorithm = \"splitmix64-v1+fisher-yates-lemire-rejection-v2\"")
            || !env_text.contains("schema_version = 2")
        {
            blockers.push(
                "manifest/environment.toml no longer echoes the frozen schema/shuffle authorities"
                    .to_string(),
            );
        }

        // Workload cardinality against the frozen manifests themselves.
        match crate::workload::load_campaign_workload(benchmark_root) {
            Ok(workload) => {
                if workload.clean_state.len() != self.surfaces.clean_state.case_count {
                    blockers.push(format!(
                        "materialized G0-strict FULL_READ {} != manifest {}",
                        workload.clean_state.len(),
                        self.surfaces.clean_state.case_count
                    ));
                }
                if workload.edit_write.len() != self.surfaces.edit_write.case_count {
                    blockers.push(format!(
                        "materialized G0_PRIMARY EDIT_WRITE {} != manifest {}",
                        workload.edit_write.len(),
                        self.surfaces.edit_write.case_count
                    ));
                }
            }
            Err(error) => blockers.push(format!("frozen workload consumption failed: {error}")),
        }

        if blockers.is_empty() {
            Ok(())
        } else {
            Err(blockers)
        }
    }
}

/// The machine manifest model (task §24). Hard host-binding fields plus
/// recorded host observations — transient values (current frequency,
/// load average, temperature, uptime) belong to per-session preflight
/// diagnostics, never here. The tier of each field is defined in
/// `machine.rs` (MARKIT-31-MACHINE-BINDING-CORRECTIVE-1): only hard
/// binding fields are matched byte-exact at preflight.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MachineManifest {
    pub schema: String,
    pub machine_id: String,
    pub architecture: String,
    pub os_distribution: String,
    pub kernel: String,
    pub cpu_vendor: String,
    pub cpu_model: String,
    pub microcode: String,
    pub physical_cores: u32,
    pub logical_cpus: u32,
    pub smt_enabled: bool,
    pub numa_nodes: u32,
    pub numa_cpu_map: String,
    /// The one frozen logical CPU primary horse execution is pinned to.
    pub selected_cpu: u32,
    /// Physical core id of `selected_cpu` (`/sys .../topology/core_id`).
    pub selected_core_id: u32,
    /// SMT sibling list of `selected_cpu` (includes itself).
    pub selected_thread_siblings: String,
    /// NUMA node containing `selected_cpu`.
    pub selected_numa_node: u32,
    pub frequency_governor: String,
    /// Honest record of turbo/boost controllability (task §26: what is
    /// controlled vs merely recorded).
    pub turbo_boost_policy: String,
    /// `/proc/meminfo:MemTotal` at capture time — a RECORDED host
    /// observation, not a hard binding field: MemTotal is usable RAM
    /// (it moves with firmware/kernel reservations between boots), so
    /// preflight reports a frozen-vs-current delta as a diagnostic and
    /// never blocks on it (MARKIT-31-MACHINE-BINDING-CORRECTIVE-1).
    pub total_ram_bytes: u64,
    pub rustc: String,
    pub cargo: String,
    pub llvm: String,
    pub target_triple: String,
    pub allocator_policy: String,
    pub release_profile_id: String,
    pub rustflags: String,
    pub cargo_lock_sha256: String,
    /// Human notes on what is controlled vs merely recorded.
    pub control_notes: String,
}

impl MachineManifest {
    pub fn load(benchmark_root: &std::path::Path) -> Result<Self, String> {
        let path = benchmark_root.join(MACHINE_MANIFEST_PATH);
        let text =
            std::fs::read_to_string(&path).map_err(|e| format!("read {}: {e}", path.display()))?;
        let manifest: MachineManifest =
            toml::from_str(&text).map_err(|e| format!("parse {}: {e}", path.display()))?;
        if manifest.schema != MACHINE_MANIFEST_SCHEMA {
            return Err(format!(
                "machine manifest schema {:?} != {MACHINE_MANIFEST_SCHEMA}",
                manifest.schema
            ));
        }
        Ok(manifest)
    }
}
