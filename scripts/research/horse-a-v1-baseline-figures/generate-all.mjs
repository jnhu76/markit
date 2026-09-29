// One-command figure generation for the Horse-A v1 baseline record (#91).
//
//   npm install        (first time only)
//   npm run generate
//
// Pipeline: verify sealed inputs (SHA-256) -> extract plotting data (frozen
// value guards) -> ECharts SSR render -> PNG. Writes:
//   derived/figures-data.json              (deterministic projection)
//   ../../../docs/research/figures/horse-a-v1-baseline/*.png
//   ../../../docs/research/figures/horse-a-v1-baseline/manifest.json
//
// Fails closed on input drift or any frozen-value guard violation.

import { mkdirSync, writeFileSync } from 'node:fs';
import { dirname, join, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';
import { execSync } from 'node:child_process';

import { extract, INPUTS_DIR, sha256 } from './lib/extract.mjs';
import { renderChartToPng, FIG_WIDTH, FIG_HEIGHT } from './lib/render.mjs';
import {
  fig1SixMechanismSpeedup,
  fig2SizeRegime,
  fig3EditFamilies,
  fig4LatencyAllocationPareto,
  fig5PmuProfile,
  fig6Rq8Sensitivity,
  fig7WeaknessMatrix,
  fig8NextMechanismPriority,
} from './lib/charts.mjs';

const SCRIPT_DIR = dirname(fileURLToPath(import.meta.url));
const REPO_ROOT = resolve(SCRIPT_DIR, '..', '..', '..');
const FIGURES_DIR = join(REPO_ROOT, 'docs', 'research', 'figures', 'horse-a-v1-baseline');
const DERIVED_DIR = join(SCRIPT_DIR, 'derived');

const FIGURES = [
  {
    file: '01-six-mechanism-edit-write-speedup.png',
    title: 'Six-mechanism EDIT_WRITE speedup vs H0',
    build: fig1SixMechanismSpeedup,
    sourceFiles: ['stage-b-facts-v1.json'],
    extraction:
      'aggregates.edit_write.<mech>.project_macro_speedup_vs_h0 for H1/H2/H3/H4/HorseA; H0 = 1 by definition (full-rebuild reference). Display rounding: 4 dp (frozen baseline §4 form).',
  },
  {
    file: '02-horse-a-size-regime-q1-q4.png',
    title: 'Horse-A Q1→Q4 size scaling',
    build: fig2SizeRegime,
    sourceFiles: ['regime-map.csv', 'rq8-sensitivity-facts-v1.json'],
    extraction:
      'regime-map.csv rows R2–R5 key_metric_values (case-weighted geomean cuts; anchored-prefix parse) for Horse-A and H4; Q3/Q4 cross-checked against rq8-sensitivity-facts-v1.json primary_sealed.K3 at 3 dp. Display rounding: 3 dp (frozen baseline §6 form).',
  },
  {
    file: '03-horse-a-edit-family-regimes.png',
    title: 'Horse-A selected edit-family / regime speedups',
    build: fig3EditFamilies,
    sourceFiles: ['stage-b-facts-v1.json', 'regime-map.csv'],
    extraction:
      'E1/E2/E5/E4/E6/ATX from stage-b-facts-v1.json family_macro.edit_write.HorseA (equal-weight FAMILY_MACRO aggregates); list/blockquote from regime-map.csv R8/R9 key_metric_values (case-weighted E3 sub-stratum cuts). Display rounding: 3 dp (frozen baseline §7 form).',
  },
  {
    file: '04-edit-latency-allocation-pareto.png',
    title: 'Edit latency × allocation Pareto view',
    build: fig4LatencyAllocationPareto,
    sourceFiles: ['r8-values.json', 'stage-b-facts-v1.json'],
    extraction:
      'r8-values.json edit_write/<mech>.allocation_count_median and .allocated_bytes_median; speedup from stage-b-facts-v1.json project macro (same authority as Figure 1). Latency representation: speedup vs H0 (relative latency = 1/speedup); KB = bytes/1000, 1 dp (frozen baseline §11 table convention).',
  },
  {
    file: '05-h4-vs-horse-a-pmu-profile.png',
    title: 'H4 vs Horse-A PMU efficiency profile',
    build: fig5PmuProfile,
    sourceFiles: ['pmu-cells-v1.json'],
    extraction:
      'EDIT_WRITE ∧ REPRESENTATIVE_PRE_OUTCOME cells, median per horse of A1_instructions_med, B2_mpki (L1D), A2_mpki (branch) — the frozen derive_pmu.py rule. Guard: reproduces the published PMU-EXPLANATION-v1 EDIT_WRITE table (23005 / 27998 instructions; 21.7 / 11.5 L1D MPKI; 13.7 / 16.8 branch MPKI). Display rounding: instructions exact integers, MPKI 1 dp.',
  },
  {
    file: '06-rq8-thinlto-vs-ltooff.png',
    title: 'ThinLTO vs LTO-off sensitivity (K1)',
    build: fig6Rq8Sensitivity,
    sourceFiles: ['rq8-sensitivity-facts-v1.json'],
    extraction:
      'primary_sealed.K1.{macro_H4,macro_HorseA,gap_pct} and second_profile.K1.{...}. Display rounding: 4 dp values (frozen baseline §5 form); gap percentages annotated ≈1.32% / ≈7.07%.',
  },
  {
    file: '07-pre-registered-weaknesses-vs-observed.png',
    title: 'Pre-registered weaknesses vs observed evidence',
    build: fig7WeaknessMatrix,
    sourceFiles: ['qualitative-verdicts.json', 'stage-b-facts-v1.json', 'regime-map.csv', 'r8-values.json'],
    extraction:
      'Verdict/priority text is a verbatim transcription of baseline record §13 (inputs/qualitative-verdicts.json; no numeric values in that file). Every number shown in the Observed-evidence cells (list 2.811x, blockquote 4.531x, E6 0.583x, allocation count 28) is injected from the same sealed extraction as Figures 3–4.',
  },
  {
    file: '08-weakness-to-next-mechanism-priority.png',
    title: 'Weakness → next-mechanism priority matrix',
    build: fig8NextMechanismPriority,
    sourceFiles: ['qualitative-verdicts.json', 'stage-b-facts-v1.json', 'regime-map.csv'],
    extraction:
      'P1–P4 ordering and basis text transcribed from baseline record §16 (inputs/qualitative-verdicts.json). Evidence numbers (E6 0.583x, bottom-decile 0.825x from regime-map.csv R1, E4 2.391x, 28 allocs) injected from sealed extraction. Bar length encodes rank position only (1 = longest); it is a visual ordering, not a quantity.',
  },
];

const manifest = {
  $comment: 'GENERATED by scripts/research/horse-a-v1-baseline-figures/generate-all.mjs — do not edit by hand. Regenerate with `npm run generate` in that directory.',
  capsuleSha256: '156ec1f3fbdfe1759ce81d17940360c2cda3adb773e7be7634b1ccc1e893d849',
  provenanceRule:
    'sealed raw / derived CSV or JSON -> deterministic extraction / plotting script -> figure artifact -> baseline report (baseline record §18)',
  rendering: {
    library: 'echarts 5 (SSR SVG) + @resvg/resvg-js rasterization',
    width: FIG_WIDTH,
    height: FIG_HEIGHT,
    background: '#ffffff',
    fonts: ['DejaVu Sans', 'DejaVu Sans Bold'],
    notes: 'animation disabled; no time-dependent content; fixed 1600x900 canvas',
  },
  generateCommand: 'cd scripts/research/horse-a-v1-baseline-figures && npm run generate',
  // No wall-clock timestamp: this manifest must be byte-identical across
  // reruns so that `git diff` is a determinism check. See README.
  nodeVersion: process.version,
  inputs: null,
  figures: [],
};

function main() {
  console.log('== Horse-A v1 baseline figure pipeline ==');
  console.log('[1/4] verifying sealed inputs + extracting data ...');
  const data = extract();
  manifest.inputs = data.inputSha256;

  mkdirSync(DERIVED_DIR, { recursive: true });
  const derivedPath = join(DERIVED_DIR, 'figures-data.json');
  writeFileSync(derivedPath, JSON.stringify(data, null, 1) + '\n');
  console.log(`      derived projection -> ${derivedPath}`);

  console.log('[2/4] rendering figures with ECharts SSR ...');
  mkdirSync(FIGURES_DIR, { recursive: true });
  for (const fig of FIGURES) {
    const option = fig.build(data);
    const png = renderChartToPng(option);
    const outPath = join(FIGURES_DIR, fig.file);
    writeFileSync(outPath, png);
    manifest.figures.push({
      file: fig.file,
      title: fig.title,
      sourceFiles: fig.sourceFiles,
      extraction: fig.extraction,
      inputSha256: Object.fromEntries(fig.sourceFiles.map((f) => [f, data.inputSha256[f]])),
      pngSha256: sha256(png),
      pngBytes: png.length,
      plottedValues: plottedValuesFor(fig.file, data),
    });
    console.log(`      ${fig.file}  (${png.length} bytes)`);
  }

  console.log('[3/4] writing manifest ...');
  writeFileSync(join(FIGURES_DIR, 'manifest.json'), JSON.stringify(manifest, null, 2) + '\n');

  console.log('[4/4] done.');
  process.exit(0);
}

// Full-precision source values behind each figure, kept for audit (the PNG
// labels use the uniform display rounding documented per figure).
function plottedValuesFor(file, d) {
  switch (file) {
    case '01-six-mechanism-edit-write-speedup.png':
      return { macro: d.macro };
    case '02-horse-a-size-regime-q1-q4.png':
      return { sizeRegime: d.sizeRegime, crossCheck: d.sizeRegimeCrossCheck };
    case '03-horse-a-edit-family-regimes.png':
      return { editFamily: d.editFamily };
    case '04-edit-latency-allocation-pareto.png':
      return { allocation: d.allocation };
    case '05-h4-vs-horse-a-pmu-profile.png':
      return { pmu: { H4: d.pmu.H4, HorseA: d.pmu.HorseA } };
    case '06-rq8-thinlto-vs-ltooff.png':
      return { rq8K1: d.rq8K1 };
    case '07-pre-registered-weaknesses-vs-observed.png':
      return {
        editFamily: { list: d.editFamily.list, blockquote: d.editFamily.blockquote, E6: d.editFamily.E6 },
        horseAllocCount: d.allocation.HorseA.count,
      };
    case '08-weakness-to-next-mechanism-priority.png':
      return {
        editFamily: { E6: d.editFamily.E6, E4: d.editFamily.E4 },
        bottomDecileHorseA: d.bottomDecile.HorseA,
        horseAllocCount: d.allocation.HorseA.count,
      };
    default:
      return null;
  }
}

main();
