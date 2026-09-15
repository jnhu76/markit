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
oracle_ok,note";

pub fn write_csv(rows: &[Row], path: &Path) -> std::io::Result<()> {
    let mut f = std::io::BufWriter::new(std::fs::File::create(path)?);
    writeln!(f, "{HEADER}")?;
    for r in rows {
        writeln!(
            f,
            "{},{},{},{},{},{},{},{},{},{},{},{:.1},{:.1},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{}",
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

pub fn summarize(rows: &[Row], skipped: usize) -> String {
    let mut s = String::new();
    s.push_str("# parser-survey run summary\n\n");
    s.push_str(&format!("scenarios measured: {} (skipped: {skipped})\n\n", rows.len()));

    let any_oracle_fail = rows.iter().any(|r| !r.oracle_ok);
    s.push_str(&format!(
        "oracle (incremental == clean rebuild): {}\n\n",
        if any_oracle_fail {
            "**FAILURE present — see rows below**"
        } else {
            "all scenarios pass"
        }
    ));

    // T1 — taxonomy validation at the largest plain corpus size. The
    // post-edit byte count differs per case, so pick the corpus whose
    // MINIMUM doc_bytes is the largest (pre-edit scale ordering).
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
    let mut t1: Vec<&Row> = rows
        .iter()
        .filter(|r| {
            r.corpus == best_plain
                && (r.position == "mid" || r.position == "target")
        })
        .collect();
    t1.sort_by(|a, b| a.case_id.cmp(&b.case_id));
    s.push_str(&format!(
        "## T1 — taxonomy at {} (mid position)\n\n\
        | case | pred | family | chg_B | inc_us | full_us | R | blocks_re | conv_Δ | surv_shift | rec_moved | proj_blk | D_content | oracle |\n\
        |---|---|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---|\n",
        best_plain
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
        | corpus | case | pred | chg_B | R | blocks_re | conv_Δ | surv_shift | rec_moved | inc_us | D_content | oracle |\n\
        |---|---|---|---:|---:|---:|---:|---:|---:|---:|---:|---|\n");
    let mut t3: Vec<&Row> = rows
        .iter()
        .filter(|r| r.corpus.starts_with("adv-") || r.corpus.contains("crlf") || r.corpus.contains("cjk"))
        .collect();
    t3.sort_by(|a, b| a.corpus.cmp(&b.corpus).then(a.case_id.cmp(&b.case_id)));
    for r in &t3 {
        s.push_str(&format!(
            "| {} | {} | {} | {} | {} | {} | {} | {} | {} | {} | {} | {} |\n",
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
        ));
    }

    s.push_str("\n## Column notes\n\n\
        - `R` = bytes_scanned / changed_bytes (reparse amplification).\n\
        - `D_content` = proj_bytes_invalidated / changed_bytes: how much source a\n\
          position-keyed downstream consumer must re-project, ignoring pure offset\n\
          shifts (those appear as `surv_shift` / `rec_moved`).\n\
        - `conv_Δ` = convergence_line − restart_line (propagation distance, lines).\n\
        - `surv_shift` = survivor_blocks_shifted, `rec_moved` = block_records_moved\n\
          (the hidden-O(N) gate: metadata work that parsing locality alone hides).\n");
    s
}
