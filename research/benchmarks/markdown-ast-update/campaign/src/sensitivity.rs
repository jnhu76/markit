//! #33 RQ8 optimization-sensitivity replication (#33; R0 §4.6).
//!
//! The second frozen profile `release-sensitivity-lto-off-v1` (single
//! intentional difference vs `release-primary-v1`: `lto = false`)
//! replays ONLY the frozen K1–K6 shortlist
//! (`results/synthesis/R8-FINAL-SYNTHESIS-v1/OPTIMIZATION-SENSITIVITY-SHORTLIST-v1.md`,
//! host-sealed R8 authority). This module owns:
//!
//! ```text
//! sensitivity manifest   results/manifests/sensitivity/rq8-sensitivity-manifest-v1.toml
//!                       (frozen case/horse matrix + pinned primary artifacts)
//! schedule projection   the frozen PRIMARY schedule restricted to the
//!                       K1–K6 matrix: same sessions, same case order
//!                       ordinals, same per-case horse order with the
//!                       non-required horses removed (NO new randomness;
//!                       policy "primary-schedule-projection-v1")
//! spec identity         CampaignSpecId of the sensitivity campaign,
//!                       derived from the primary spec binding with the
//!                       campaign id + build profile overridden
//! execution path        run-sensitivity-session in mdbench-campaign
//!                       (same SessionExecutor, same timer boundaries,
//!                       same finalization contract — only the identity,
//!                       the schedule source, and the raw root differ)
//! ```
//!
//! The primary campaign paths (`run-session`, `run-attribution`) are
//! untouched and still refuse every profile except `release-primary-v1`;
//! the sensitivity path refuses every profile except
//! `release-sensitivity-lto-off-v1` (fail-closed in both directions).

use std::collections::BTreeSet;
use std::path::Path;

use serde::{Deserialize, Serialize};

use crate::identity::CampaignSpecBinding;
use crate::manifest::{
    parse_seed_value, CampaignManifest, CAMPAIGN_MANIFEST_PATH, MACHINE_MANIFEST_PATH,
    SCHEDULE_MANIFEST_PATH,
};
use crate::schedule::{schedule_from_jsonl, ScheduleRow};
use crate::{sha256_file, Surface, HORSE_IDS};

/// Campaign id of the sensitivity replication.
pub const SENSITIVITY_CAMPAIGN_ID: &str = "MARKIT-33-RQ8-OPTIMIZATION-SENSITIVITY-v1";
/// Provenance tag of sensitivity timing rows.
pub const PROVENANCE_SENSITIVITY_TIMING: &str = "MARKIT-33-RQ8-OPTIMIZATION-SENSITIVITY-v1/TIMING";
/// Schedule policy: the frozen primary schedule restricted to the K1–K6
/// matrix — no regeneration, no new seeds, order preserved verbatim.
pub const SENSITIVITY_SCHEDULE_POLICY: &str = "primary-schedule-projection-v1";
/// Path of the sensitivity manifest below the benchmark root.
pub const SENSITIVITY_MANIFEST_PATH: &str =
    "results/manifests/sensitivity/rq8-sensitivity-manifest-v1.toml";
/// Path of the frozen projected schedule below the benchmark root.
pub const SENSITIVITY_SCHEDULE_PATH: &str =
    "results/manifests/sensitivity/rq8-sensitivity-schedule-v1.jsonl";
/// Raw evidence root of the sensitivity campaign (isolated from the
/// primary `results/raw/` tree; same append/create-only discipline).
pub const SENSITIVITY_RAW_ROOT: &str = "results/sensitivity/raw";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SensitivityAuthority {
    pub primary_campaign_id: String,
    pub primary_campaign_spec_id: String,
    pub primary_schedule_sha256: String,
    pub primary_campaign_manifest_sha256: String,
    pub primary_machine_manifest_sha256: String,
    pub primary_workload_receipt_sha256: String,
    pub shortlist_sha256: String,
    pub shortlist_note: String,
    /// Exact `rustc -V` string the sensitivity binary must be built with
    /// (identical toolchain to the primary campaign).
    pub pinned_rustc: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SensitivitySessions {
    pub count: u32,
    pub warmup_iterations: u32,
    pub measured_iterations: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SensitivitySeed {
    /// `inherit-primary`: same root seed + same session seed derivation
    /// as the primary campaign — identical per-case seeds.
    pub policy: String,
    pub value: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SensitivityHorses {
    /// EDIT_WRITE horses measured on EVERY case of the population
    /// (K1 H4/HorseA macro + K3 + K6 H1 macro + H0 normalization).
    pub population: Vec<String>,
    /// Horses added ONLY on the K2/K4 subset cases (where all six run).
    pub subset_extra: Vec<String>,
    /// CLEAN_STATE horses (K5): the full roster.
    pub clean_state: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SensitivityCaseSet {
    pub count: usize,
    pub derivation: String,
    pub case_ids: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SensitivityPhCase {
    pub case_id: String,
    pub derivation: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SensitivityCardinality {
    pub edit_cases_per_session: usize,
    pub edit_horse_cells_per_session: usize,
    pub clean_cases_per_session: usize,
    pub clean_horse_cells_per_session: usize,
    pub schedule_rows: usize,
    pub timing_rows_per_session: usize,
    pub total_timing_rows: usize,
}

/// The frozen sensitivity execution manifest.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SensitivityManifest {
    pub schema: String,
    pub campaign_id: String,
    pub frozen_date: String,
    pub profile_id: String,
    pub sensitivity_of_profile: String,
    pub schedule_policy: String,
    pub authority: SensitivityAuthority,
    pub sessions: SensitivitySessions,
    pub seed: SensitivitySeed,
    pub horses: SensitivityHorses,
    pub bottom_decile: SensitivityCaseSet,
    pub e6_reference_definition: SensitivityCaseSet,
    pub ph_a_loss: SensitivityPhCase,
    pub cardinality: SensitivityCardinality,
}

impl SensitivityManifest {
    pub fn load(benchmark_root: &Path) -> Result<Self, String> {
        let path = benchmark_root.join(SENSITIVITY_MANIFEST_PATH);
        let text =
            std::fs::read_to_string(&path).map_err(|e| format!("read {}: {e}", path.display()))?;
        let manifest: SensitivityManifest =
            toml::from_str(&text).map_err(|e| format!("parse {}: {e}", path.display()))?;
        Ok(manifest)
    }

    /// Self-consistency of the frozen matrix (no filesystem access).
    pub fn verify(&self) -> Result<(), Vec<String>> {
        let mut blockers = Vec::new();
        let frozen_roster: BTreeSet<&str> = HORSE_IDS.iter().copied().collect();

        if self.schema != "rq8-sensitivity-manifest-v1" {
            blockers.push(format!(
                "schema {:?} != rq8-sensitivity-manifest-v1",
                self.schema
            ));
        }
        if self.campaign_id != SENSITIVITY_CAMPAIGN_ID {
            blockers.push(format!(
                "campaign_id {:?} != {SENSITIVITY_CAMPAIGN_ID}",
                self.campaign_id
            ));
        }
        if self.profile_id != markit_mdbench_runner::build_identity::SENSITIVITY_LTO_OFF_PROFILE_ID
        {
            blockers.push(format!(
                "profile_id {:?} != the frozen second profile {}",
                self.profile_id,
                markit_mdbench_runner::build_identity::SENSITIVITY_LTO_OFF_PROFILE_ID
            ));
        }
        if self.sensitivity_of_profile
            != markit_mdbench_runner::build_identity::RELEASE_PRIMARY_PROFILE_ID
        {
            blockers.push(format!(
                "sensitivity_of_profile {:?} != release-primary-v1",
                self.sensitivity_of_profile
            ));
        }
        if self.schedule_policy != SENSITIVITY_SCHEDULE_POLICY {
            blockers.push(format!(
                "schedule_policy {:?} != {SENSITIVITY_SCHEDULE_POLICY}",
                self.schedule_policy
            ));
        }
        if self.sessions.count != 3
            || self.sessions.warmup_iterations != 10
            || self.sessions.measured_iterations != 30
        {
            blockers.push(format!(
                "sessions {}/{}/{} != the frozen 3/10/30 primary policy",
                self.sessions.count,
                self.sessions.warmup_iterations,
                self.sessions.measured_iterations
            ));
        }
        if self.seed.policy != "inherit-primary" {
            blockers.push(format!(
                "seed policy {:?} != inherit-primary",
                self.seed.policy
            ));
        }
        if parse_seed_value(&self.seed.value).is_err() {
            blockers.push(format!("seed value {:?} is not 0x-hex", self.seed.value));
        }

        // Horse matrix: subsets of the frozen roster, no duplicates, and
        // population + subset_extra must cover the full roster (subset
        // cases run all six horses).
        let check_set = |name: &str, set: &[String], blockers: &mut Vec<String>| {
            if set.is_empty() || set.len() != BTreeSet::<&String>::from_iter(set.iter()).len() {
                blockers.push(format!("horses.{name} is empty or repeats an id"));
            }
            for horse in set {
                if !frozen_roster.contains(horse.as_str()) {
                    blockers.push(format!(
                        "horses.{name} has {horse:?} outside the frozen roster"
                    ));
                }
            }
        };
        check_set("population", &self.horses.population, &mut blockers);
        check_set("subset_extra", &self.horses.subset_extra, &mut blockers);
        check_set("clean_state", &self.horses.clean_state, &mut blockers);
        let mut covered: BTreeSet<&str> =
            BTreeSet::from_iter(self.horses.population.iter().map(|h| h.as_str()));
        covered.extend(self.horses.subset_extra.iter().map(|h| h.as_str()));
        if covered != frozen_roster {
            blockers
                .push("population + subset_extra must cover the full six-horse roster".to_string());
        }
        let overlap: Vec<&String> = self
            .horses
            .population
            .iter()
            .filter(|horse| self.horses.subset_extra.contains(horse))
            .collect();
        if !overlap.is_empty() {
            blockers.push(format!(
                "horses.population and horses.subset_extra overlap ({overlap:?}); the matrix must partition the roster"
            ));
        }
        let clean_sorted = {
            let mut c = self.horses.clean_state.clone();
            c.sort();
            c
        };
        if clean_sorted != HORSE_IDS.iter().map(|h| h.to_string()).collect::<Vec<_>>() {
            blockers.push("horses.clean_state != the full frozen roster".to_string());
        }
        if !self.horses.population.iter().any(|h| h == "H0") {
            blockers.push("horses.population must include H0 (speedup normalization)".to_string());
        }

        // Case sets: unique, valid hex, counts match.
        let check_cases = |name: &str, set: &SensitivityCaseSet, blockers: &mut Vec<String>| {
            if set.count != set.case_ids.len() {
                blockers.push(format!(
                    "{name}: count {} != {} listed ids",
                    set.count,
                    set.case_ids.len()
                ));
            }
            let unique: BTreeSet<&String> = BTreeSet::from_iter(set.case_ids.iter());
            if unique.len() != set.case_ids.len() {
                blockers.push(format!("{name}: duplicate case ids"));
            }
            for id in &set.case_ids {
                if let Err(e) = crate::schedule::validate_case_id_hex(id) {
                    blockers.push(format!("{name}: {e}"));
                }
            }
        };
        check_cases("bottom_decile", &self.bottom_decile, &mut blockers);
        check_cases(
            "e6_reference_definition",
            &self.e6_reference_definition,
            &mut blockers,
        );
        if let Err(e) = crate::schedule::validate_case_id_hex(&self.ph_a_loss.case_id) {
            blockers.push(format!("ph_a_loss: {e}"));
        }

        // Cardinality arithmetic (the union of the subset case sets is
        // what carries the full roster on EDIT_WRITE).
        let mut union: BTreeSet<&String> = BTreeSet::from_iter(self.bottom_decile.case_ids.iter());
        union.extend(self.e6_reference_definition.case_ids.iter());
        union.insert(&self.ph_a_loss.case_id);
        let c = &self.cardinality;
        if c.edit_cases_per_session != 362 || c.clean_cases_per_session != 22 {
            blockers.push(format!(
                "cardinality cases {}/{} != the frozen 362/22 population",
                c.edit_cases_per_session, c.clean_cases_per_session
            ));
        }
        let expect_edit_cells = self.horses.population.len() * c.edit_cases_per_session
            + self.horses.subset_extra.len() * union.len();
        let expect_clean_cells = self.horses.clean_state.len() * c.clean_cases_per_session;
        if c.edit_horse_cells_per_session != expect_edit_cells
            || c.clean_horse_cells_per_session != expect_clean_cells
        {
            blockers.push(format!(
                "cardinality horse cells {}/{} != derived {expect_edit_cells}/{expect_clean_cells}",
                c.edit_horse_cells_per_session, c.clean_horse_cells_per_session
            ));
        }
        let expect_rows = (expect_edit_cells + expect_clean_cells)
            * (self.sessions.warmup_iterations + self.sessions.measured_iterations) as usize;
        if c.timing_rows_per_session != expect_rows || c.total_timing_rows != expect_rows * 3 {
            blockers.push(format!(
                "cardinality rows {}/{} != derived {expect_rows}/{}",
                c.timing_rows_per_session,
                c.total_timing_rows,
                expect_rows * 3
            ));
        }
        let expect_schedule_rows =
            (c.edit_cases_per_session + c.clean_cases_per_session) * self.sessions.count as usize;
        if c.schedule_rows != expect_schedule_rows {
            blockers.push(format!(
                "cardinality schedule rows {} != derived {expect_schedule_rows}",
                c.schedule_rows
            ));
        }

        if blockers.is_empty() {
            Ok(())
        } else {
            Err(blockers)
        }
    }

    /// EDIT_WRITE case ids that carry the FULL roster (K2 bottom decile
    /// + PH-A-LOSS + K4 E6 family).
    pub fn full_roster_edit_cases(&self) -> BTreeSet<&str> {
        let mut union: BTreeSet<&str> = BTreeSet::from_iter(
            self.bottom_decile
                .case_ids
                .iter()
                .map(|id| id.as_str())
                .chain(
                    self.e6_reference_definition
                        .case_ids
                        .iter()
                        .map(|id| id.as_str()),
                ),
        );
        union.insert(self.ph_a_loss.case_id.as_str());
        union
    }
}

/// `CampaignSpecId` of the sensitivity campaign: the PRIMARY spec
/// binding (all frozen inputs identical — workload, machine, envelope
/// schema, policies) with the campaign id + build profile overridden and
/// the sensitivity lineage recorded. Deterministic canonical-JSON SHA256
/// (serde_json object keys in sorted order).
pub fn sensitivity_spec_id(primary: &CampaignSpecBinding) -> String {
    let primary_id = crate::identity::campaign_spec_id(primary);
    let mut value = serde_json::to_value(primary).expect("spec binding serializes");
    let object = value
        .as_object_mut()
        .expect("spec binding serializes to an object");
    object.insert(
        "campaign_id".to_string(),
        serde_json::json!(SENSITIVITY_CAMPAIGN_ID),
    );
    object.insert(
        "build_profile_id".to_string(),
        serde_json::json!(markit_mdbench_runner::build_identity::SENSITIVITY_LTO_OFF_PROFILE_ID),
    );
    object.insert(
        "sensitivity_of_primary_spec_id".to_string(),
        serde_json::json!(primary_id),
    );
    object.insert(
        "sensitivity_schedule_policy".to_string(),
        serde_json::json!(SENSITIVITY_SCHEDULE_POLICY),
    );
    let bytes = serde_json::to_vec(&value).expect("value serializes");
    crate::sha256_hex(&bytes)
}

/// Project the frozen PRIMARY schedule onto the K1–K6 matrix.
///
/// For every primary row (same session ordinals, same order ordinals,
/// same case/payload/source/trace identity):
///
/// ```text
/// clean_state      -> horse order = the primary row's order verbatim
/// edit_write       -> in the full-roster subset (K2/K4 cases):
///                     the primary row's order verbatim;
///                     otherwise: population horses only, in the
///                     primary row's relative order
/// ```
///
/// No new randomness: the projection is a pure restriction of the
/// frozen primary schedule (policy `primary-schedule-projection-v1`).
pub fn project_schedule(
    primary_rows: &[ScheduleRow],
    manifest: &SensitivityManifest,
    sensitivity_campaign_spec_id: &str,
) -> Result<Vec<ScheduleRow>, String> {
    let full_roster = manifest.full_roster_edit_cases();
    let population: BTreeSet<&str> =
        BTreeSet::from_iter(manifest.horses.population.iter().map(|h| h.as_str()));
    let mut projected = Vec::with_capacity(primary_rows.len());
    for row in primary_rows {
        let horses: Vec<String> = match row.surface.as_str() {
            "clean_state" => row.horse_order.clone(),
            "edit_write" => {
                if full_roster.contains(row.case_id.as_str()) {
                    row.horse_order.clone()
                } else {
                    row.horse_order
                        .iter()
                        .filter(|horse| population.contains(horse.as_str()))
                        .cloned()
                        .collect()
                }
            }
            other => return Err(format!("primary schedule row surface {other:?}")),
        };
        if horses.is_empty() {
            return Err(format!(
                "projection emptied the horse order of case {} — the manifest matrix is inconsistent with the schedule",
                row.case_id
            ));
        }
        if row.surface == "edit_write" && !full_roster.contains(row.case_id.as_str()) {
            let projected_set: BTreeSet<&str> =
                BTreeSet::from_iter(horses.iter().map(|h| h.as_str()));
            if projected_set != population {
                return Err(format!(
                    "case {}: projected horses {:?} != manifest population — the primary schedule row did not carry the full roster",
                    row.case_id, horses
                ));
            }
        }
        projected.push(ScheduleRow {
            schema: row.schema.clone(),
            campaign_spec_id: sensitivity_campaign_spec_id.to_string(),
            surface: row.surface.clone(),
            session_ordinal: row.session_ordinal,
            order_ordinal: row.order_ordinal,
            case_id: row.case_id.clone(),
            payload_id: row.payload_id.clone(),
            source_key: row.source_key.clone(),
            trace_id: row.trace_id.clone(),
            horse_order: horses,
        });
    }
    Ok(projected)
}

/// Load + hash-verify the pinned primary artifacts and return the
/// primary schedule rows. Fails closed on any drift.
pub fn load_verified_primary_schedule(
    benchmark_root: &Path,
    manifest: &SensitivityManifest,
) -> Result<Vec<ScheduleRow>, String> {
    let pin = |path: &str, expected: &str| -> Result<(), String> {
        let live = sha256_file(&benchmark_root.join(path))?;
        if live != expected {
            return Err(format!(
                "{path} sha256 {live} != pinned {expected}: the sensitivity replication must consume the frozen primary artifacts"
            ));
        }
        Ok(())
    };
    pin(
        SCHEDULE_MANIFEST_PATH,
        &manifest.authority.primary_schedule_sha256,
    )?;
    pin(
        CAMPAIGN_MANIFEST_PATH,
        &manifest.authority.primary_campaign_manifest_sha256,
    )?;
    pin(
        MACHINE_MANIFEST_PATH,
        &manifest.authority.primary_machine_manifest_sha256,
    )?;
    pin(
        "workloads/payloads/freeze-receipt-v1.json",
        &manifest.authority.primary_workload_receipt_sha256,
    )?;
    let bytes = std::fs::read(benchmark_root.join(SCHEDULE_MANIFEST_PATH))
        .map_err(|e| format!("read primary schedule: {e}"))?;
    schedule_from_jsonl(&bytes)
}

/// Verify the on-disk sensitivity schedule: byte-identical to a fresh
/// projection of the verified primary schedule, carrying the
/// recomputed sensitivity spec id, and matching the frozen cardinality.
pub fn verify_sensitivity_schedule(
    benchmark_root: &Path,
    manifest: &SensitivityManifest,
    sensitivity_campaign_spec_id: &str,
) -> Result<usize, String> {
    let primary_rows = load_verified_primary_schedule(benchmark_root, manifest)?;
    let projected = project_schedule(&primary_rows, manifest, sensitivity_campaign_spec_id)?;
    let expected_bytes = crate::schedule::schedule_to_jsonl(&projected)?;
    let on_disk = std::fs::read(benchmark_root.join(SENSITIVITY_SCHEDULE_PATH))
        .map_err(|e| format!("read {}: {e}", SENSITIVITY_SCHEDULE_PATH))?;
    if on_disk != expected_bytes {
        return Err(format!(
            "{SENSITIVITY_SCHEDULE_PATH} is not byte-identical to the deterministic projection of the pinned primary schedule"
        ));
    }
    let rows = schedule_from_jsonl(&on_disk)?;
    if rows.len() != manifest.cardinality.schedule_rows {
        return Err(format!(
            "sensitivity schedule has {} rows != frozen {}",
            rows.len(),
            manifest.cardinality.schedule_rows
        ));
    }
    Ok(rows.len())
}

/// Derive the sensitivity spec id from the live (hash-verified) primary
/// artifacts — the single derivation path shared by generation,
/// verification, and execution.
pub fn sensitivity_spec_id_from_root(
    benchmark_root: &Path,
) -> Result<(String, CampaignManifest), String> {
    let primary_manifest = CampaignManifest::load(benchmark_root)?;
    let binding = crate::receipt::build_spec_binding(benchmark_root, &primary_manifest)?;
    let primary_id = crate::identity::campaign_spec_id(&binding);
    let manifest = SensitivityManifest::load(benchmark_root)?;
    if primary_id != manifest.authority.primary_campaign_spec_id {
        return Err(format!(
            "recomputed primary spec id {primary_id} != pinned {} — the primary freeze inputs drifted",
            manifest.authority.primary_campaign_spec_id
        ));
    }
    Ok((sensitivity_spec_id(&binding), primary_manifest))
}

/// Result of a sensitivity preflight run (same fail-closed shape as
/// [`crate::preflight::PreflightReport`]).
pub struct SensitivityPreflightReport {
    pub pass: bool,
    pub blockers: Vec<String>,
    pub diagnostics: serde_json::Value,
}

/// Fail-closed preflight for one sensitivity timing session. Checks
/// (mirroring the primary preflight, restricted to what the sensitivity
/// replication consumes):
///
/// ```text
/// sensitivity manifest self-consistency
/// pinned primary artifacts (schedule / campaign manifest / machine
///   manifest / workload receipt) hash-verified against live state
/// recomputed primary spec id == pinned
/// primary session/seed policy == sensitivity policy
/// on-disk sensitivity schedule == deterministic projection + cardinality
/// workload freeze receipt valid + sources materialized (hash-verified)
/// build identity: second frozen profile, pinned rustc, Cargo.lock
///   digest matches the live lockfile, commit never "unknown"
/// result + envelope schemas have not drifted from the Rust models
/// host binding: machine match (exact HARD fields) + affinity probe
/// observation-tuple uniqueness for the requested (surface, session)
/// ```
pub fn sensitivity_preflight(
    benchmark_root: &Path,
    surface: Surface,
    session: u32,
) -> SensitivityPreflightReport {
    let mut blockers: Vec<String> = Vec::new();
    let finish =
        |blockers: Vec<String>, diagnostics: serde_json::Value| SensitivityPreflightReport {
            pass: blockers.is_empty(),
            blockers,
            diagnostics,
        };

    // 1. Manifest self-consistency.
    let manifest = match SensitivityManifest::load(benchmark_root) {
        Ok(manifest) => manifest,
        Err(error) => {
            return finish(
                vec![format!("sensitivity manifest: {error}")],
                serde_json::Value::Null,
            )
        }
    };
    if let Err(manifest_blockers) = manifest.verify() {
        blockers.extend(manifest_blockers);
    }

    // 2. Pinned primary artifacts + recomputed spec identity.
    let (spec_id, primary_manifest) = match sensitivity_spec_id_from_root(benchmark_root) {
        Ok((spec_id, primary)) => (spec_id, primary),
        Err(error) => {
            blockers.push(format!("primary authority: {error}"));
            return finish(blockers, serde_json::Value::Null);
        }
    };
    if (
        primary_manifest.sessions.count,
        primary_manifest.sessions.warmup_iterations,
        primary_manifest.sessions.measured_iterations,
    ) != (
        manifest.sessions.count,
        manifest.sessions.warmup_iterations,
        manifest.sessions.measured_iterations,
    ) {
        blockers.push("primary session policy (3/10/30) != sensitivity session policy".to_string());
    }
    if parse_seed_value(&primary_manifest.seed.value) != parse_seed_value(&manifest.seed.value) {
        blockers.push("sensitivity seed != inherited primary seed".to_string());
    }

    // 3. Workload freeze receipt + materialized sources.
    if let Err(workload_blockers) = crate::receipt::verify_workload_freeze_receipt(benchmark_root) {
        blockers.extend(workload_blockers);
    }
    if let Err(error) = crate::workload::load_campaign_workload(benchmark_root) {
        blockers.push(format!("source materialization: {error}"));
    }

    // 4. Build identity: the second frozen profile, pinned toolchain.
    let build = markit_mdbench_runner::current_build_identity();
    if build.build_profile_id
        != markit_mdbench_runner::build_identity::SENSITIVITY_LTO_OFF_PROFILE_ID
    {
        blockers.push(format!(
            "build profile {:?} != {}; sensitivity sessions run ONLY under the second frozen profile",
            build.build_profile_id,
            markit_mdbench_runner::build_identity::SENSITIVITY_LTO_OFF_PROFILE_ID
        ));
    }
    if build.rustc != manifest.authority.pinned_rustc {
        blockers.push(format!(
            "rustc {:?} != pinned {:?} (the sensitivity replication must use the identical primary toolchain)",
            build.rustc, manifest.authority.pinned_rustc
        ));
    }
    if build.runner_git_commit == "unknown" {
        blockers.push("runner git commit unavailable (built outside a git workspace?)".to_string());
    }
    match sha256_file(&benchmark_root.join("Cargo.lock")) {
        Ok(live) if live == build.cargo_lock_sha256 => {}
        Ok(live) => blockers.push(format!(
            "Cargo.lock sha256 {live} != the digest baked into the binary {} — rebuild required",
            build.cargo_lock_sha256
        )),
        Err(error) => blockers.push(format!("Cargo.lock: {error}")),
    }

    // 5. Schema drift (same check as the primary preflight).
    let result_schema_generated =
        serde_json::to_value(schemars::schema_for!(markit_mdbench_runner::ResultRowV1))
            .expect("result schema is serializable");
    let result_schema_path = benchmark_root.join("protocol/result-schema-v2.json");
    match std::fs::read_to_string(&result_schema_path)
        .map_err(|e| format!("read {}: {e}", result_schema_path.display()))
        .and_then(|text| {
            serde_json::from_str::<serde_json::Value>(&text)
                .map_err(|e| format!("parse {}: {e}", result_schema_path.display()))
        }) {
        Ok(checked) if checked == result_schema_generated => {}
        Ok(_) => blockers.push(
            "protocol/result-schema-v2.json drifted from the Rust ResultRowV1 model".to_string(),
        ),
        Err(error) => blockers.push(error),
    }
    let envelope_generated =
        serde_json::to_value(schemars::schema_for!(crate::execute::CampaignObservationV1))
            .expect("envelope schema is serializable");
    let envelope_path = benchmark_root.join(crate::manifest::ENVELOPE_SCHEMA_PATH);
    match std::fs::read_to_string(&envelope_path)
        .map_err(|e| format!("read {}: {e}", envelope_path.display()))
        .and_then(|text| {
            serde_json::from_str::<serde_json::Value>(&text)
                .map_err(|e| format!("parse {}: {e}", envelope_path.display()))
        }) {
        Ok(checked) if checked == envelope_generated => {}
        Ok(_) => blockers.push(format!(
            "{} drifted from the Rust CampaignObservationV1 model",
            crate::manifest::ENVELOPE_SCHEMA_PATH
        )),
        Err(error) => blockers.push(error),
    }

    // 6. On-disk sensitivity schedule == deterministic projection.
    let schedule_rows = match verify_sensitivity_schedule(benchmark_root, &manifest, &spec_id) {
        Ok(count) => count,
        Err(error) => {
            blockers.push(format!("sensitivity schedule: {error}"));
            0
        }
    };

    // 7. Observation-tuple uniqueness for this (surface, session).
    let bytes = std::fs::read(benchmark_root.join(SENSITIVITY_SCHEDULE_PATH)).unwrap_or_default();
    let rows = schedule_from_jsonl(&bytes).unwrap_or_default();
    let mut tuples: BTreeSet<(String, String, String, u32)> = BTreeSet::new();
    let mut tuple_rows = 0usize;
    for row in rows
        .iter()
        .filter(|row| row.surface == surface.as_str() && row.session_ordinal == session)
    {
        for horse in &row.horse_order {
            for kind in ["warmup", "measured"] {
                let iterations = if kind == "warmup" {
                    manifest.sessions.warmup_iterations
                } else {
                    manifest.sessions.measured_iterations
                };
                for iteration in 0..iterations {
                    tuple_rows += 1;
                    tuples.insert((
                        row.case_id.clone(),
                        horse.clone(),
                        kind.to_string(),
                        iteration,
                    ));
                }
            }
        }
    }
    let expected_scope_rows = match surface {
        Surface::CleanState => manifest.cardinality.clean_horse_cells_per_session,
        Surface::EditWrite => manifest.cardinality.edit_horse_cells_per_session,
    } * (manifest.sessions.warmup_iterations
        + manifest.sessions.measured_iterations) as usize;
    let duplicate_tuples = tuple_rows - tuples.len();
    if duplicate_tuples != 0 || tuple_rows != expected_scope_rows {
        blockers.push(format!(
            "observation tuples for {} session {session}: {tuple_rows} rows, {} duplicates, expected {expected_scope_rows}",
            surface.as_str(),
            duplicate_tuples
        ));
    }

    // 8. Host binding: machine match + affinity probe.
    let mut machine_memory = serde_json::Value::Null;
    match crate::manifest::MachineManifest::load(benchmark_root) {
        Ok(machine) => {
            let (observed, mut capture_blockers) =
                crate::machine::observe_host_for_binding(&machine, benchmark_root);
            blockers.append(&mut capture_blockers);
            blockers.extend(crate::machine::compare_host_binding(&machine, &observed));
            machine_memory = crate::machine::memory_binding_diagnostic(
                machine.total_ram_bytes,
                observed.mem_total_bytes,
            );
            if let Err(error) = crate::machine::probe_apply_affinity(machine.selected_cpu) {
                blockers.push(format!("affinity probe: {error}"));
            }
        }
        Err(error) => blockers.push(format!("machine manifest: {error}")),
    }

    let diagnostics = serde_json::json!({
        "transient": crate::machine::transient_diagnostics(),
        "machine_memory": machine_memory,
        "sensitivity_spec_id": spec_id,
        "schedule_rows": schedule_rows,
        "scope": format!("sensitivity-timing:{}:{session}", surface.as_str()),
        "observation_tuples": {"rows": tuple_rows, "unique": tuples.len()},
    });
    finish(blockers, diagnostics)
}
