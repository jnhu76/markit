// Deterministic extraction of plotting data from sealed derived authority
// artifacts. Fails closed: any input SHA-256 mismatch or any frozen-value
// guard violation aborts generation before a single PNG is rendered.

import { createHash } from 'node:crypto';
import { readFileSync } from 'node:fs';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';

const SCRIPT_DIR = dirname(fileURLToPath(import.meta.url));
export const INPUTS_DIR = join(SCRIPT_DIR, '..', 'inputs');

export const sha256 = (buf) => createHash('sha256').update(buf).digest('hex');

function readInput(name) {
  return readFileSync(join(INPUTS_DIR, name));
}

// ---- input integrity -------------------------------------------------------

function verifyInputSha256s() {
  const sumsText = readInput('SHA256SUMS').toString('utf8');
  const failures = [];
  const verified = {};
  for (const line of sumsText.split('\n').filter((l) => l.trim() !== '')) {
    const [expected, name] = line.split(/\s+/);
    const actual = sha256(readInput(name));
    if (actual !== expected) {
      failures.push(`${name}: expected ${expected}, got ${actual}`);
    } else {
      verified[name] = actual;
    }
  }
  if (failures.length > 0) {
    throw new Error(
      `Input integrity check FAILED (sealed artifact drift):\n  ${failures.join('\n  ')}` +
        `\nRefusing to generate figures. The committed inputs must match inputs/SHA256SUMS.`,
    );
  }
  return verified;
}

// ---- helpers ---------------------------------------------------------------

// Small CSV parser sufficient for the regime map: quoted fields with commas,
// no embedded newlines inside quoted fields.
function parseCsv(text) {
  const rows = [];
  let row = [];
  let field = '';
  let inQuotes = false;
  for (let i = 0; i < text.length; i++) {
    const c = text[i];
    if (inQuotes) {
      if (c === '"') {
        if (text[i + 1] === '"') {
          field += '"';
          i++;
        } else {
          inQuotes = false;
        }
      } else {
        field += c;
      }
    } else if (c === '"') {
      inQuotes = true;
    } else if (c === ',') {
      row.push(field);
      field = '';
    } else if (c === '\n') {
      row.push(field.replace(/\r$/, ''));
      field = '';
      if (row.some((f) => f !== '')) rows.push(row);
      row = [];
    } else {
      field += c;
    }
  }
  if (field !== '' || row.length > 0) {
    row.push(field.replace(/\r$/, ''));
    if (row.some((f) => f !== '')) rows.push(row);
  }
  return rows;
}

function loadRegimeMap() {
  const rows = parseCsv(readInput('regime-map.csv').toString('utf8'));
  const header = rows[0];
  const idIdx = header.indexOf('regime_id');
  const kmvIdx = header.indexOf('key_metric_values');
  const map = new Map();
  for (const r of rows.slice(1)) {
    map.set(r[idIdx], { keyMetricValues: r[kmvIdx] });
  }
  return map;
}

// Extracts "HorseA 2.811" / "H4 4.258" style readings from a regime-map
// key_metric_values field. Anchored to the exact expected prefix so that a
// re-sealed regime map with different wording fails loudly instead of
// silently plotting a wrong number.
function regimeValue(row, expectedPrefix, mech) {
  const s = row.keyMetricValues;
  if (!s.startsWith(expectedPrefix)) {
    throw new Error(
      `regime-map.csv field no longer has expected prefix "${expectedPrefix}": "${s}"`,
    );
  }
  const re = new RegExp(`${mech}\\s+([0-9.]+)`);
  const m = s.match(re);
  if (!m) {
    throw new Error(`regime-map.csv field has no ${mech} value: "${s}"`);
  }
  return Number(m[1]);
}

const round = (x, d) => Number(x.toFixed(d));

// Baseline record §4.1: latency reduction vs H0 = 1 − 1/speedup. The doc
// freezes these percentages as algebraic restatements of the frozen speedup
// values ("not new measurements or a new aggregation surface"), so figures
// display reduction computed from the same frozen display-form speedup that
// the figure shows in parentheses — never from a different rounding.
const reductionPct = (displaySpeedup) =>
  Number((((1 - 1 / displaySpeedup) * 100)).toFixed(1));

function median(xs) {
  const s = [...xs].sort((a, b) => a - b);
  const n = s.length;
  if (n === 0) throw new Error('median of empty list');
  return n % 2 === 1 ? s[(n - 1) / 2] : (s[n / 2 - 1] + s[n / 2]) / 2;
}

// ---- PMU derivation (must reproduce the published sealed summary) ----------

function pmuEditWriteMedians(pmuCells) {
  const horses = ['H0', 'H1', 'H2', 'H3', 'H4', 'HorseA'];
  const out = {};
  for (const h of horses) {
    const ew = pmuCells.filter(
      (c) =>
        c.horse === h &&
        c.selection_class === 'REPRESENTATIVE_PRE_OUTCOME' &&
        c.surface === 'edit_write',
    );
    if (ew.length === 0) throw new Error(`no PMU edit_write representative cells for ${h}`);
    out[h] = {
      cells: ew.length,
      instructionsMedian: median(ew.map((c) => c.A1_instructions_med).filter((v) => v != null)),
      cyclesMedian: median(ew.map((c) => c.A1_cycles_med).filter((v) => v != null)),
      l1dMpkiMedian: median(ew.map((c) => c.B2_mpki).filter((v) => v != null)),
      branchMpkiMedian: median(ew.map((c) => c.A2_mpki).filter((v) => v != null)),
    };
  }
  return out;
}

// ---- guard table -----------------------------------------------------------

// Frozen display values from the merged baseline record. Every plotted value
// must round to exactly these; any drift in re-sealed inputs stops the run.
const GUARDS = [
  // §4 primary EDIT_WRITE project-macro speedups (4 dp)
  ['fig1 H1 speedup', 'macro.H1', 1.0647],
  ['fig1 H2 speedup', 'macro.H2', 2.0699],
  ['fig1 H3 speedup', 'macro.H3', 1.9219],
  ['fig1 H4 speedup', 'macro.H4', 2.4535],
  ['fig1 HorseA speedup', 'macro.HorseA', 2.4214],
  // §6 size regime (3 dp)
  ['fig2 HorseA Q1', 'sizeRegime.HorseA.Q1', 1.414],
  ['fig2 HorseA Q2', 'sizeRegime.HorseA.Q2', 2.908],
  ['fig2 HorseA Q3', 'sizeRegime.HorseA.Q3', 5.276],
  ['fig2 HorseA Q4', 'sizeRegime.HorseA.Q4', 5.367],
  ['fig2 H4 Q1', 'sizeRegime.H4.Q1', 1.381],
  ['fig2 H4 Q2', 'sizeRegime.H4.Q2', 3.013],
  ['fig2 H4 Q3', 'sizeRegime.H4.Q3', 4.258],
  ['fig2 H4 Q4', 'sizeRegime.H4.Q4', 4.845],
  // §7 edit families (3 dp)
  ['fig3 E1', 'editFamily.E1', 3.314],
  ['fig3 E2', 'editFamily.E2', 3.451],
  ['fig3 E5', 'editFamily.E5', 3.92],
  ['fig3 ATX', 'editFamily.ATX', 5.234],
  ['fig3 list', 'editFamily.list', 2.811],
  ['fig3 blockquote', 'editFamily.blockquote', 4.531],
  ['fig3 E4', 'editFamily.E4', 2.391],
  ['fig3 E6', 'editFamily.E6', 0.583],
  // §11 allocation medians (exact sealed medians)
  ['fig4 H0 alloc count', 'allocation.H0.count', 215.5],
  ['fig4 H1 alloc count', 'allocation.H1.count', 290],
  ['fig4 H2 alloc count', 'allocation.H2.count', 72],
  ['fig4 H3 alloc count', 'allocation.H3.count', 71],
  ['fig4 H4 alloc count', 'allocation.H4.count', 51],
  ['fig4 HorseA alloc count', 'allocation.HorseA.count', 28],
  ['fig4 H0 alloc bytes', 'allocation.H0.bytes', 112978],
  ['fig4 H1 alloc bytes', 'allocation.H1.bytes', 133591],
  ['fig4 H2 alloc bytes', 'allocation.H2.bytes', 18456],
  ['fig4 H3 alloc bytes', 'allocation.H3.bytes', 16347],
  ['fig4 H4 alloc bytes', 'allocation.H4.bytes', 19524],
  ['fig4 HorseA alloc bytes', 'allocation.HorseA.bytes', 10631.5],
  // §12 PMU EDIT_WRITE representative medians (published 1 dp / exact)
  ['fig5 H4 instructions', 'pmu.H4.instructionsMedian', 23005],
  ['fig5 HorseA instructions', 'pmu.HorseA.instructionsMedian', 27998],
  ['fig5 H4 L1D MPKI', 'pmu.H4.l1dMpkiMedian', 21.689, 21.7],
  ['fig5 HorseA L1D MPKI', 'pmu.HorseA.l1dMpkiMedian', 11.522, 11.5],
  ['fig5 H4 branch MPKI', 'pmu.H4.branchMpkiMedian', 13.697, 13.7],
  ['fig5 HorseA branch MPKI', 'pmu.HorseA.branchMpkiMedian', 16.804, 16.8],
  // §5 RQ8 K1 (4 dp)
  ['fig6 H4 LTO-off', 'rq8K1.secondProfile.H4', 2.5982],
  ['fig6 HorseA LTO-off', 'rq8K1.secondProfile.HorseA', 2.4265],
  ['fig6 H4 primary', 'rq8K1.primary.H4', 2.4535],
  ['fig6 HorseA primary', 'rq8K1.primary.HorseA', 2.4214],
  // §9 bottom decile (3 dp, from regime map R1)
  ['fig8 bottom decile HorseA', 'bottomDecile.HorseA', 0.825],
  // §4.1 latency-reduction restatement of the frozen speedups (1 dp)
  ['fig1 H1 reduction', 'latencyReduction.macro.H1', 6.1],
  ['fig1 H2 reduction', 'latencyReduction.macro.H2', 51.7],
  ['fig1 H3 reduction', 'latencyReduction.macro.H3', 48.0],
  ['fig1 H4 reduction', 'latencyReduction.macro.H4', 59.2],
  ['fig1 HorseA reduction', 'latencyReduction.macro.HorseA', 58.7],
  ['fig2 HorseA Q1 reduction', 'latencyReduction.sizeRegime.HorseA.Q1', 29.3],
  ['fig2 HorseA Q2 reduction', 'latencyReduction.sizeRegime.HorseA.Q2', 65.6],
  ['fig2 HorseA Q3 reduction', 'latencyReduction.sizeRegime.HorseA.Q3', 81.0],
  ['fig2 HorseA Q4 reduction', 'latencyReduction.sizeRegime.HorseA.Q4', 81.4],
  ['fig3 E1 reduction', 'latencyReduction.editFamily.E1', 69.8],
  ['fig3 E2 reduction', 'latencyReduction.editFamily.E2', 71.0],
  ['fig3 E5 reduction', 'latencyReduction.editFamily.E5', 74.5],
  ['fig3 ATX reduction', 'latencyReduction.editFamily.ATX', 80.9],
  ['fig3 list reduction', 'latencyReduction.editFamily.list', 64.4],
  ['fig3 blockquote reduction', 'latencyReduction.editFamily.blockquote', 77.9],
  ['fig3 E4 reduction', 'latencyReduction.editFamily.E4', 58.2],
  ['fig3 E6 reduction (slower)', 'latencyReduction.editFamily.E6', -71.5],
  ['fig6 H4 LTO-off reduction', 'latencyReduction.rq8K1.secondProfile.H4', 61.5],
  ['fig6 HorseA LTO-off reduction', 'latencyReduction.rq8K1.secondProfile.HorseA', 58.8],
  ['fig6 H4 primary reduction', 'latencyReduction.rq8K1.primary.H4', 59.2],
  ['fig6 HorseA primary reduction', 'latencyReduction.rq8K1.primary.HorseA', 58.7],
];

function runGuards(data) {
  const failures = [];
  for (const g of GUARDS) {
    const [label, path, ...expectedList] = g;
    const actual = path.split('.').reduce((o, k) => (o == null ? undefined : o[k]), data);
    let ok = false;
    for (const expected of expectedList) {
      if (typeof expected === 'number') {
        // match at full precision or at the natural rounding of the display rule
        if (actual === expected || round(actual, String(expected).split('.')[1]?.length ?? 0) === expected) {
          ok = true;
          break;
        }
      }
    }
    if (!ok) failures.push(`${label}: expected one of [${expectedList.join(', ')}], got ${actual}`);
  }
  if (failures.length > 0) {
    throw new Error(
      `Frozen-value guards FAILED (${failures.length}):\n  ${failures.join('\n  ')}` +
        `\nRefusing to generate figures. Investigate input drift; do not hand-edit plotted values.`,
    );
  }
}

// ---- main extraction -------------------------------------------------------

export function extract() {
  const verified = verifyInputSha256s();

  const stageB = JSON.parse(readInput('stage-b-facts-v1.json').toString('utf8'));
  const r8Values = JSON.parse(readInput('r8-values.json').toString('utf8'));
  const rq8Facts = JSON.parse(readInput('rq8-sensitivity-facts-v1.json').toString('utf8'));
  const pmuCells = JSON.parse(readInput('pmu-cells-v1.json').toString('utf8'));
  const qualitative = JSON.parse(readInput('qualitative-verdicts.json').toString('utf8'));
  const regimeMap = loadRegimeMap();

  // Figure 1 — §4 primary EDIT_WRITE project-macro speedups.
  // H0 is the full-rebuild reference; its speedup is 1 by definition of the
  // ratio, and the sealed stage-b file stores only incremental mechanisms.
  const macroRaw = {};
  for (const mech of ['H1', 'H2', 'H3', 'H4', 'HorseA']) {
    macroRaw[mech] = stageB.aggregates.edit_write[mech].project_macro_speedup_vs_h0;
  }
  const macro = { H0: 1, ...macroRaw };

  // Figure 2 — §6 size quartiles from the sealed R8 regime map
  // (case-weighted geomean descriptive cuts), cross-checked against the RQ8
  // sealed K3 full-precision values for Q3/Q4.
  const sizeRegime = {
    HorseA: {
      Q1: regimeValue(regimeMap.get('R2'), 'Q1 geomeans:', 'HorseA'),
      Q2: regimeValue(regimeMap.get('R3'), 'Q2:', 'HorseA'),
      Q3: regimeValue(regimeMap.get('R4'), 'Q3:', 'HorseA'),
      Q4: regimeValue(regimeMap.get('R5'), 'Q4:', 'HorseA'),
    },
    H4: {
      Q1: regimeValue(regimeMap.get('R2'), 'Q1 geomeans:', 'H4'),
      Q2: regimeValue(regimeMap.get('R3'), 'Q2:', 'H4'),
      Q3: regimeValue(regimeMap.get('R4'), 'Q3:', 'H4'),
      Q4: regimeValue(regimeMap.get('R5'), 'Q4:', 'H4'),
    },
  };
  const k3 = rq8Facts.primary_sealed.K3;
  const crossCheck = {
    Q3_HorseA: { regimeMap: sizeRegime.HorseA.Q3, rq8FullPrecision: k3.Q3_HorseA },
    Q4_HorseA: { regimeMap: sizeRegime.HorseA.Q4, rq8FullPrecision: k3.Q4_HorseA },
    Q3_H4: { regimeMap: sizeRegime.H4.Q3, rq8FullPrecision: k3.Q3_H4 },
    Q4_H4: { regimeMap: sizeRegime.H4.Q4, rq8FullPrecision: k3.Q4_H4 },
  };
  for (const [k, v] of Object.entries(crossCheck)) {
    if (round(v.rq8FullPrecision, 3) !== v.regimeMap) {
      throw new Error(
        `Fig2 cross-check failed for ${k}: regime map ${v.regimeMap} vs RQ8 ${round(v.rq8FullPrecision, 3)}`,
      );
    }
  }

  // Figure 3 — §7 edit-family speedups: E1/E2/E5/E4/E6/ATX from the sealed
  // stage-b FAMILY_MACRO aggregates; list/blockquote from the sealed R8
  // regime-map E3 sub-stratum cuts (case-weighted), per the baseline note.
  const fam = stageB.family_macro.edit_write.HorseA;
  const editFamily = {
    E1: fam.E1_LOCAL_TEXT,
    E2: fam.E2_PARAGRAPH_SPLIT_MERGE,
    E5: fam.E5_INLINE_DELIMITER,
    ATX: fam.ATX_HEADING_TOGGLE,
    list: regimeValue(regimeMap.get('R8'), 'list_item:', 'HorseA'),
    blockquote: regimeValue(regimeMap.get('R9'), 'blockquote:', 'HorseA'),
    E4: fam.E4_FENCE_OPEN_CLOSE,
    E6: fam.E6_REFERENCE_DEFINITION,
  };
  const e6Context = {
    H1: stageB.family_macro.edit_write.H1.E6_REFERENCE_DEFINITION,
    H2: stageB.family_macro.edit_write.H2.E6_REFERENCE_DEFINITION,
    H3: stageB.family_macro.edit_write.H3.E6_REFERENCE_DEFINITION,
    H4: stageB.family_macro.edit_write.H4.E6_REFERENCE_DEFINITION,
    HorseA: fam.E6_REFERENCE_DEFINITION,
  };

  // Figure 4 — §11 EDIT_WRITE median allocation profile (R8 sealed values)
  // + §4 project-macro speedups. KB uses the 1000 B convention of the frozen
  // baseline table (10631.5 B -> 10.6 KB).
  const allocation = {};
  for (const mech of ['H0', 'H1', 'H2', 'H3', 'H4', 'HorseA']) {
    const v = r8Values[`edit_write/${mech}`];
    allocation[mech] = {
      count: v.allocation_count_median,
      bytes: v.allocated_bytes_median,
      kb: round(v.allocated_bytes_median / 1000, 1),
      speedup: macro[mech],
    };
  }

  // Figure 5 — §12 PMU medians recomputed from the sealed PMU analysis cells
  // under the frozen derivation rule (EDIT_WRITE representative
  // pre-outcome cells, median per horse); reproduces the published
  // PMU-EXPLANATION-v1 EDIT_WRITE table.
  const pmu = pmuEditWriteMedians(pmuCells);

  // Figure 6 — §5 RQ8 K1 macro under both compiler profiles.
  const rq8K1 = {
    primary: {
      H4: rq8Facts.primary_sealed.K1.macro_H4,
      HorseA: rq8Facts.primary_sealed.K1.macro_HorseA,
      gapPct: rq8Facts.primary_sealed.K1.gap_pct,
    },
    secondProfile: {
      H4: rq8Facts.second_profile.K1.macro_H4,
      HorseA: rq8Facts.second_profile.K1.macro_HorseA,
      gapPct: rq8Facts.second_profile.K1.gap_pct,
    },
  };

  // Figure 8 context — §9 bottom-decile cut from the sealed regime map R1.
  const bottomDecile = {
    HorseA: regimeValue(
      regimeMap.get('R1'),
      'bottom decile H1 0.847',
      'HorseA',
    ),
  };

  // §4.1 latency-reduction restatement (display metric for Figures 1–4, 6),
  // computed from the same frozen display-form speedups the figures plot.
  const latencyReduction = {
    rule: 'reduction = 1 − 1/speedup on the frozen display-form speedup (baseline record §4.1)',
    macro: Object.fromEntries(
      Object.entries(macro).map(([m, s]) => [m, m === 'H0' ? 0 : reductionPct(round(s, 4))]),
    ),
    sizeRegime: Object.fromEntries(
      Object.entries(sizeRegime).map(([mech, qs]) => [
        mech,
        Object.fromEntries(Object.entries(qs).map(([q, s]) => [q, reductionPct(round(s, 3))])),
      ]),
    ),
    editFamily: Object.fromEntries(
      Object.entries(editFamily).map(([f, s]) => [f, reductionPct(round(s, 3))]),
    ),
    rq8K1: {
      primary: Object.fromEntries(
        Object.entries(rq8K1.primary)
          .filter(([k]) => k !== 'gapPct')
          .map(([m, s]) => [m, reductionPct(round(s, 4))]),
      ),
      secondProfile: Object.fromEntries(
        Object.entries(rq8K1.secondProfile)
          .filter(([k]) => k !== 'gapPct')
          .map(([m, s]) => [m, reductionPct(round(s, 4))]),
      ),
    },
  };

  const data = {
    generatedBy: 'scripts/research/horse-a-v1-baseline-figures (deterministic extraction)',
    capsuleSha256: '156ec1f3fbdfe1759ce81d17940360c2cda3adb773e7be7634b1ccc1e893d849',
    inputSha256: verified,
    sourceNote:
      'All numeric values are extracted from sealed derived artifacts; see inputs/PROVENANCE.md. Qualitative verdict/priority text comes from inputs/qualitative-verdicts.json (transcription of baseline record §13/§16).',
    macro,
    latencyReduction,
    sizeRegime,
    sizeRegimeCrossCheck: crossCheck,
    editFamily,
    e6Context,
    allocation,
    pmu,
    rq8K1,
    bottomDecile,
    qualitative,
  };

  runGuards(data);
  return data;
}
