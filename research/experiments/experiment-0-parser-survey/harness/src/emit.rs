//! CSV emission and markdown summary tables.

use std::io::Write;
use std::path::Path;

use crate::measure::Row;

const HEADER: &str = "\
case_id,family,construct,predicted,corpus,position,doc_bytes,doc_lines,doc_blocks,\
changed_bytes,changed_lines,full_us,inc_us,bytes_scanned,lines_scanned,blocks_reparsed,\
blocks_reused,blocks_created,blocks_removed,blocks_examined,dirty_regions,restart_line,\
convergence_line,inline_blocks_reparsed,inline_bytes_scanned,survivor_blocks_shifted,\
survivor_inline_nodes_shifted,block_records_moved,proj_blocks_invalidated,\
proj_bytes_invalidated,alloc_count_inc,alloc_bytes_inc,alloc_count_full,alloc_bytes_full,\
oracle_ok,edit_start,edit_end,cover_old_kind,cover_old_lines,cover_new_kind,\
prefix_aligned,suffix_aligned,gap_bytes_unclaimed,lossless_ok,syntax_class,\
semantic_class,representation_class,provider,note";

pub fn write_csv(rows: &[Row], path: &Path) -> std::io::Result<()> {
    let mut f = std::io::BufWriter::new(std::fs::File::create(path)?);
    writeln!(f, "{HEADER}")?;
    for r in rows {
        writeln!(
            f,
            "{},{},{},{},{},{},{},{},{},{},{},{:.1},{:.1},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{}",
            r.case_id, r.family, r.construct, r.predicted, r.corpus, r.position,
            r.doc_bytes, r.doc_lines, r.doc_blocks,
            r.changed_bytes, r.changed_lines,
            r.full_us, r.inc_us,
            r.bytes_scanned, r.lines_scanned, r.blocks_reparsed,
            r.blocks_reused, r.blocks_created, r.blocks_removed, r.blocks_examined,
            r.dirty_regions, r.restart_line, r.convergence_line,
            r.inline_blocks_reparsed, r.inline_bytes_scanned,
            r.survivor_blocks_shifted, r.survivor_inline_nodes_shifted, r.block_records_moved,
            r.proj_blocks_invalidated, r.proj_bytes_invalidated,
            r.alloc_count_inc, r.alloc_bytes_inc, r.alloc_count_full, r.alloc_bytes_full,
            r.oracle_ok,
            r.edit_start, r.edit_end,
            r.cover_old_kind, r.cover_old_lines, r.cover_new_kind,
            r.prefix_aligned, r.suffix_aligned,
            r.gap_bytes_unclaimed, r.lossless_ok,
            r.syntax_class, r.semantic_class, r.representation_class, r.provider,
            r.note.replace(',', ";"),
        )?;
    }
    f.flush()?;
    Ok(())
}

fn is_plain(corpus: &str) -> bool {
    corpus.starts_with("synth-") && !corpus.contains("crlf") && !corpus.contains("cjk")
}

fn ratio(num: u64, den: u64) -> String {
    let d = den.max(1) as f64;
    let v = num as f64 / d;
    if v >= 100.0 {
        format!("{v:.0}")
    } else {
        format!("{v:.1}")
    }
}

fn us(v: f64) -> String {
    if v >= 10_000.0 {
        format!("{:.1}ms", v / 1000.0)
    } else {
        format!("{v:.1}")
    }
}

fn oracle(ok: bool) -> &'static str {
    if ok {
        "ok"
    } else {
        "**FAIL**"
    }
}

/// Least squares `y = a + b·x` with R², used for the attribution slope
/// (inc_us vs survivor records shifted). Support evidence only — the
/// structural counters stay the primary evidence.
fn linfit(pts: &[(f64, f64)]) -> (f64, f64, f64) {
    let n = pts.len() as f64;
    if n < 2.0 {
        return (0.0, 0.0, 0.0);
    }
    let (sx, sy): (f64, f64) = pts.iter().fold((0.0, 0.0), |(a, b), (x, y)| (a + x, b + y));
    let (sxx, sxy): (f64, f64) = pts
        .iter()
        .fold((0.0, 0.0), |(a, b), (x, y)| (a + x * x, b + x * y));
    let denom = n * sxx - sx * sx;
    if denom.abs() < f64::EPSILON {
        return (0.0, 0.0, 0.0);
    }
    let b = (n * sxy - sx * sy) / denom;
    let a = (sy - b * sx) / n;
    let mean = sy / n;
    let ss_tot: f64 = pts.iter().map(|(_, y)| (y - mean).powi(2)).sum();
    let ss_res: f64 = pts.iter().map(|(x, y)| (y - (a + b * x)).powi(2)).sum();
    let r2 = if ss_tot > 0.0 {
        1.0 - ss_res / ss_tot
    } else {
        0.0
    };
    (a, b, r2)
}

pub fn summarize(rows: &[Row], skipped: usize) -> String {
    let mut s = String::new();
    s.push_str("# parser-survey run summary\n\n");
    s.push_str(&format!("scenarios measured: {} (skipped: {skipped})\n\n", rows.len()));

    let pass_a = rows.iter().filter(|r| r.oracle_ok).count();
    s.push_str(&format!(
        "ORACLE-A (SELF_EQUIVALENCE) incremental == same-parser clean rebuild: {pass_a}/{} pass{}\n\n",
        rows.len(),
        if pass_a == rows.len() { "" } else { " — **FAILURE present, see rows below**" }
    ));
    let pass_c = rows.iter().filter(|r| r.lossless_ok).count();
    s.push_str(&format!(
        "ORACLE-C (LOSSLESSNESS) full byte coverage, whitespace-only gaps: {pass_c}/{} pass{}\n\n",
        rows.len(),
        if pass_c == rows.len() { "" } else { " — **FAILURE present, see rows below**" }
    ));

    // T0 — influence vectors at the largest plain corpus (observed
    // classes, run-1.1 schema). Semantic is D_UNKNOWN everywhere: no
    // semantic dependency layer exists yet.
    use std::collections::HashMap;
    let mut mins: HashMap<&str, u64> = HashMap::new();
    for r in rows.iter().filter(|r| is_plain(&r.corpus)) {
        let e = mins.entry(r.corpus.as_str()).or_insert(u64::MAX);
        *e = (*e).min(r.doc_bytes);
    }
    let best_plain = mins
        .iter()
        .max_by_key(|(_, &m)| m)
        .map(|(c, _)| *c)
        .unwrap_or("");
    let mut t0: Vec<&Row> = rows
        .iter()
        .filter(|r| {
            r.corpus == best_plain
                && (r.position == "mid" || r.position == "target")
        })
        .collect();
    t0.sort_by(|a, b| a.case_id.cmp(&b.case_id));
    s.push_str(&format!(
        "## T0 — influence vectors at {best_plain} (mid position; observed classes)\n\n\
        | case | pred(I) | syntax | semantic | representation | Psem_blk | Pcoord_rec+inline | provider |\n\
        |---|---|---|---|---|---:|---:|---|\n"
    ));
    for r in &t0 {
        s.push_str(&format!(
            "| {} | {} | {} | {} | {} | {} | {}+{} | {} |\n",
            r.case_id,
            r.predicted,
            r.syntax_class,
            r.semantic_class,
            r.representation_class,
            r.proj_blocks_invalidated,
            r.survivor_blocks_shifted,
            r.survivor_inline_nodes_shifted,
            r.provider,
        ));
    }

    // Attribution fit — length-changing edits across all plain sizes and
    // positions: does wall-clock track survivor movement?
    let fit_pts: Vec<(f64, f64)> = rows
        .iter()
        .filter(|r| {
            is_plain(&r.corpus)
                && matches!(
                    r.case_id.as_str(),
                    "para_insert_char" | "para_delete_char" | "bof_insert_char" | "eof_append_char"
                )
        })
        .map(|r| (r.survivor_blocks_shifted as f64, r.inc_us))
        .collect();
    let (a, b, r2) = linfit(&fit_pts);
    s.push_str(&format!(
        "\nAttribution support (NOT primary evidence): inc_us ≈ {a:.2}µs + {:.1}ns × survivor_blocks_shifted over the {} length-changing-edit rows below (R² = {r2:.3}).\n",
        b * 1000.0,
        fit_pts.len()
    ));

    // T1 — taxonomy validation at the largest plain corpus size. The
    // post-edit byte count differs per case, so pick the corpus whose
    // MINIMUM doc_bytes is the largest (pre-edit scale ordering).
    let t1: Vec<&Row> = t0.clone();
    s.push_str(&format!(
        "\n## T1 — taxonomy at {best_plain} (mid position)\n\n\
        | case | pred | family | chg_B | inc_us | full_us | R | blocks_re | conv_Δ | surv_shift | rec_moved | proj_blk | D_content | oracleA |\n\
        |---|---|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---|\n"
    ));
    for r in &t1 {
        s.push_str(&format!(
            "| {} | {} | {} | {} | {} | {} | {} | {} | {} | {} | {} | {} | {} | {} |\n",
            r.case_id,
            r.predicted,
            r.family,
            r.changed_bytes,
            us(r.inc_us),
            us(r.full_us),
            ratio(r.bytes_scanned, r.changed_bytes),
            r.blocks_reparsed,
            r.convergence_line.saturating_sub(r.restart_line),
            r.survivor_blocks_shifted,
            r.block_records_moved,
            r.proj_blocks_invalidated,
            ratio(r.proj_bytes_invalidated, r.changed_bytes),
            oracle(r.oracle_ok),
        ));
    }

    // T2 — hidden-O(N) gate: offset-shifting edits across sizes/positions.
    s.push_str("\n## T2 — hidden-O(N) gate (offset rewrites vs parse radius)\n\n\
        | case | corpus | pos | R | survivor_shifted | records_moved | proj_bytes | inc_us |\n\
        |---|---|---|---:|---:|---:|---:|---:|\n");
    let mut t2: Vec<&Row> = rows
        .iter()
        .filter(|r| {
            is_plain(&r.corpus)
                && matches!(
                    r.case_id.as_str(),
                    "para_insert_char" | "para_delete_char" | "bof_insert_char" | "eof_append_char"
                )
        })
        .collect();
    t2.sort_by(|a, b| {
        a.corpus
            .cmp(&b.corpus)
            .then(a.case_id.cmp(&b.case_id))
            .then(a.position.cmp(&b.position))
    });
    for r in &t2 {
        s.push_str(&format!(
            "| {} | {} | {} | {} | {} | {} | {} | {} |\n",
            r.case_id,
            r.corpus,
            r.position,
            ratio(r.bytes_scanned, r.changed_bytes),
            r.survivor_blocks_shifted,
            r.block_records_moved,
            r.proj_bytes_invalidated,
            us(r.inc_us),
        ));
    }

    // T3 — adversarial + encoding variants.
    s.push_str("\n## T3 — adversarial & encoding-variant corpora\n\n\
        | corpus | case | pred | chg_B | R | blocks_re | conv_Δ | surv_shift | rec_moved | inc_us | D_content | oracleA | losslessC |\n\
        |---|---|---|---:|---:|---:|---:|---:|---:|---:|---:|---|---|\n");
    let mut t3: Vec<&Row> = rows
        .iter()
        .filter(|r| r.corpus.starts_with("adv-") || r.corpus.contains("crlf") || r.corpus.contains("cjk"))
        .collect();
    t3.sort_by(|a, b| a.corpus.cmp(&b.corpus).then(a.case_id.cmp(&b.case_id)));
    for r in &t3 {
        s.push_str(&format!(
            "| {} | {} | {} | {} | {} | {} | {} | {} | {} | {} | {} | {} | {} |\n",
            r.corpus,
            r.case_id,
            r.predicted,
            r.changed_bytes,
            ratio(r.bytes_scanned, r.changed_bytes),
            r.blocks_reparsed,
            r.convergence_line.saturating_sub(r.restart_line),
            r.survivor_blocks_shifted,
            r.block_records_moved,
            us(r.inc_us),
            ratio(r.proj_bytes_invalidated, r.changed_bytes),
            oracle(r.oracle_ok),
            oracle(r.lossless_ok),
        ));
    }

    s.push_str("\n## Column notes\n\n\
        - `R` = bytes_scanned / changed_bytes (reparse amplification).\n\
        - `D_content` = proj_bytes_invalidated / changed_bytes: how much source a\n\
          position-keyed downstream consumer must re-project, ignoring pure offset\n\
          shifts (those appear as `surv_shift` / `rec_moved`).\n\
        - `conv_Δ` = convergence_line − restart_line (propagation distance, lines).\n\
        - `surv_shift` = survivor_blocks_shifted, `rec_moved` = block_records_moved\n\
          (the hidden-O(N) gate: metadata work that parsing locality alone hides).\n\
        - Influence classes (T0) are derived ONLY from observed counters of the same\n\
          row: S0 needs blocks_reparsed=0 (a ZERO_REPARSE_CONVERGENCE — zero parse\n\
          work, NOT zero metadata work); S5/S6 split by restart position on ≥half-doc\n\
          scans; M4 needs survivor shifts ≥ half the suffix-proportional expectation.\n\
          Semantic is D_UNKNOWN for every row: no semantic dependency layer exists in\n\
          this run. See harness src/influence.rs (research/experiments/\n\
          experiment-0-parser-survey) for the full rules.\n");
    s
}
