// ECharts option builders for the eight Horse-A v1 baseline figures.
// All numeric inputs come from lib/extract.mjs (sealed derived artifacts);
// this module only shapes display. Display rounding rules are documented in
// the figure README and kept uniform across figures.
//
// Layout notes (from the 2026-09-29 visual acceptance pass):
// - long subtitles/notes are manually wrapped with \n (ECharts does not
//   auto-wrap title/subtext);
// - markLine parity labels use insideEndTop so they stay on-canvas;
// - multi-grid figures use fixed-width grids so panels cannot overlap;
// - scatter labels get deterministic per-point positions.

const FONT = 'DejaVu Sans';

const COLORS = {
  H0: '#8c8c8c',
  H1: '#c8b273',
  H2: '#6aaed6',
  H3: '#3f88b0',
  H4: '#205d8c',
  HorseA: '#b23a48',
  refLine: '#555555',
  frontier: '#9ab7c9',
};

const baseText = {
  fontFamily: FONT,
  color: '#1a1a1a',
};

function commonOption({ title, subtitle, bottomNote }) {
  return {
    backgroundColor: '#ffffff',
    animation: false,
    textStyle: baseText,
    title: {
      text: title,
      subtext: subtitle,
      left: 'center',
      top: 14,
      textBaseline: 'top',
      textStyle: { ...baseText, fontSize: 29, fontWeight: 'bold' },
      subtextStyle: { ...baseText, fontSize: 18, color: '#444444', lineHeight: 25 },
    },
    graphic: bottomNote
      ? [
          {
            type: 'text',
            left: 'center',
            bottom: 10,
            style: {
              ...baseText,
              fontSize: 15,
              fill: '#666666',
              textAlign: 'center',
              lineHeight: 21,
              text: bottomNote,
            },
          },
        ]
      : [],
  };
}

const mechColor = (mech) => COLORS[mech] ?? '#333333';

// Two-line axis labels keep six categories readable across the plot width.
const LABELS = {
  H0: 'H0\nfull rebuild',
  H1: 'H1\nblock-local',
  H2: 'H2\nfragment reuse',
  H3: 'H3\nsubtree reuse',
  H4: 'H4\nrestart/conv.',
  HorseA: 'Horse-A v1',
};

const PARITY_MARK = {
  symbol: 'none',
  silent: true,
  lineStyle: { color: COLORS.refLine, type: 'dashed', width: 1.5 },
  label: { ...baseText, fontSize: 16, position: 'insideEndTop', formatter: 'H0 parity (0%)' },
  data: [{ yAxis: 0 }],
};

// Latency-reduction display: "+58.7%" faster / "−71.5%" slower (U+2212).
const red = (v) =>
  v > 0 ? '+' + v.toFixed(1) + '%' : v < 0 ? '−' + Math.abs(v).toFixed(1) + '%' : '0.0%';
const redLabel = (v, speedup) => `${red(v)} (${speedup}×)`;

// Vertical y-axis name (horizontal names clip at the canvas edge given the
// 110px left grid margin).
const yAxisName = (text) => ({
  type: 'value',
  name: text,
  nameLocation: 'middle',
  nameRotate: 90,
  nameTextStyle: { ...baseText, fontSize: 18 },
  nameGap: 64,
  axisLabel: { ...baseText, fontSize: 19 },
  splitLine: { lineStyle: { color: '#dddddd' } },
});

// Greedy word wrap for graphic-cell text (fig 7/8). Deterministic. Breaks
// after hyphens and slashes (delimiter kept on the line) so long tokens like
// "semantic-dependency" or "container/locality" can wrap; inter-word spaces
// are preserved as tokens.
function wrap(text, maxChars) {
  const lines = [];
  let cur = '';
  const pushToken = (t) => {
    const cand = cur + t;
    if (cand.trimEnd().length > maxChars && cur.trimEnd() !== '') {
      lines.push(cur.trimEnd());
      cur = t;
    } else {
      cur = cand;
    }
  };
  const words = text.split(' ');
  words.forEach((word, wi) => {
    for (const p of word.split(/(?<=[-/])/)) {
      if (p !== '') pushToken(p);
    }
    if (wi < words.length - 1) pushToken(' ');
  });
  if (cur.trimEnd() !== '') lines.push(cur.trimEnd());
  return lines.join('\n');
}

// ---------------------------------------------------------------- Figure 1 --

export function fig1SixMechanismSpeedup(d) {
  const mechs = ['H0', 'H1', 'H2', 'H3', 'H4', 'HorseA'];
  const option = commonOption({
    title: 'EDIT_WRITE latency reduction vs H0 — six controlled mechanisms',
    subtitle:
      'Primary profile (ThinLTO) · project-macro aggregate, sealed stage-b facts\n' +
      `H4 (${red(d.latencyReduction.macro.H4)}) vs Horse-A (${red(d.latencyReduction.macro.HorseA)}) is a near-tie: not a forced universal winner`,
    bottomNote:
      'Source: R7 stage-b-facts-v1.json project-macro speedups · latency reduction = 1 − 1/speedup (baseline record §4.1)\n' +
      'labels: latency reduction with the frozen speedup in parentheses · H0 = full-rebuild reference (0% by definition)',
  });
  Object.assign(option, {
    grid: { top: 170, left: 110, right: 60, bottom: 130 },
    xAxis: {
      type: 'category',
      data: mechs.map((m) => LABELS[m]),
      axisLabel: { ...baseText, fontSize: 20, interval: 0, lineHeight: 26 },
      axisLine: { lineStyle: { color: '#333333' } },
      axisTick: { show: false },
    },
    yAxis: {
      ...yAxisName('latency reduction vs H0 (%)'),
      min: -10,
      max: 70,
    },
    series: [
      {
        type: 'bar',
        barWidth: 96,
        data: mechs.map((m) => ({
          value: d.latencyReduction.macro[m],
          speedup: d.macro[m].toFixed(4),
          itemStyle: { color: mechColor(m) },
        })),
        label: {
          show: true,
          position: 'top',
          ...baseText,
          fontSize: 20,
          fontWeight: 'bold',
          formatter: (p) => redLabel(p.value, p.data.speedup),
        },
        markLine: { ...PARITY_MARK, label: { ...PARITY_MARK.label, position: 'insideStartBottom' } },
      },
    ],
  });
  return option;
}

// ---------------------------------------------------------------- Figure 2 --

export function fig2SizeRegime(d) {
  const q = ['Q1', 'Q2', 'Q3', 'Q4'];
  const option = commonOption({
    title: 'Horse-A latency reduction across document-size quartiles (Q1 → Q4)',
    subtitle:
      'Descriptive quartile regime result (case-weighted geomean cut, primary profile) — not a claim that size alone causes the crossover\n' +
      'Horse-A overtakes H4 around Q2→Q3',
    bottomNote:
      'Source: R8 regime-map.csv R2–R5 (Q3/Q4 cross-checked at full precision against RQ8 rq8-sensitivity-facts-v1.json K3)\n' +
      'latency reduction = 1 − 1/speedup (§4.1) · frozen speedups (×): Horse-A Q1 1.414 · Q2 2.908 · Q3 5.276 · Q4 5.367 · H4 shown as frozen context',
  });
  Object.assign(option, {
    grid: { top: 190, left: 110, right: 70, bottom: 130 },
    legend: {
      top: 152,
      data: ['Horse-A v1', 'H4'],
      textStyle: { ...baseText, fontSize: 20 },
      itemWidth: 36,
      itemGap: 46,
    },
    xAxis: {
      type: 'category',
      data: q,
      axisLabel: { ...baseText, fontSize: 22 },
      axisLine: { lineStyle: { color: '#333333' } },
      axisTick: { show: false },
    },
    yAxis: {
      ...yAxisName('latency reduction vs H0 (%)'),
      min: 0,
      max: 95,
    },
    series: [
      {
        name: 'Horse-A v1',
        type: 'line',
        // per-point label positions: the two lines cross between Q2 and Q3,
        // so fixed positions must alternate around each marker
        data: q.map((k, i) => ({
          value: d.latencyReduction.sizeRegime.HorseA[k],
          // Q1 sits left of the rising outgoing segment: 'left' keeps the
          // label clear of the line; 'top' would be pierced by it.
          label: { show: true, position: ['left', 'right', 'top', 'top'][i], distance: 10 },
        })),
        symbol: 'circle',
        symbolSize: 14,
        lineStyle: { width: 4, color: COLORS.HorseA },
        itemStyle: { color: COLORS.HorseA },
        label: {
          ...baseText,
          fontSize: 20,
          fontWeight: 'bold',
          formatter: (p) => red(p.value),
        },
        labelLayout: { moveOverlap: 'shiftY', hideOverlap: false },
      },
      {
        name: 'H4',
        type: 'line',
        data: q.map((k, i) => ({
          value: d.latencyReduction.sizeRegime.H4[k],
          label: {
            show: true,
            position: ['bottom', 'top', 'bottom', 'bottom'][i],
            distance: 10,
            offset: i === 1 ? [-34, 0] : [0, 0],
          },
        })),
        symbol: 'triangle',
        symbolSize: 12,
        lineStyle: { width: 3, color: COLORS.H4, type: 'dashed' },
        itemStyle: { color: COLORS.H4 },
        label: {
          ...baseText,
          fontSize: 16,
          color: '#205d8c',
          formatter: (p) => red(p.value),
        },
        labelLayout: { moveOverlap: 'shiftY', hideOverlap: false },
      },
      {
        type: 'line',
        data: [],
        markLine: { ...PARITY_MARK, label: { ...PARITY_MARK.label, position: 'insideStartTop' } },
      },
    ],
  });
  return option;
}

// ---------------------------------------------------------------- Figure 3 --

export function fig3EditFamilies(d) {
  const rows = [
    ['E1', 'E1\ntext', COLORS.HorseA, 'top'],
    ['E2', 'E2\nparagraph', COLORS.HorseA, 'top'],
    ['E5', 'E5\ninline', COLORS.HorseA, 'top'],
    ['ATX', 'ATX\nheading', COLORS.HorseA, 'top'],
    ['list', 'list\ncontainer', '#7a6ea8', 'top'],
    ['blockquote', 'blockquote\ncontainer', '#7a6ea8', 'top'],
    ['E4', 'E4\nfence', '#c98b3f', 'top'],
    ['E6', 'E6\nreference', '#8f1d1d', 'bottom'],
  ];
  const option = commonOption({
    title: 'Horse-A latency reduction by edit family / regime',
    subtitle:
      'Positive = faster than H0; negative = slower · local / container / inline edits strong, semantic / global weak\n' +
      `E6 ${red(d.latencyReduction.editFamily.E6)} means ${Math.abs(d.latencyReduction.editFamily.E6)}% slower than a full rebuild — the clearest algorithmic weakness`,
    bottomNote:
      'E1/E2/E5/E4/E6/ATX = equal-weight FAMILY_MACRO aggregates (stage-b facts); list/blockquote = case-weighted E3 sub-stratum cuts (R8 regime map)\n' +
      'latency reduction = 1 − 1/speedup (§4.1) · frozen speedups (×): E1 3.314 · E2 3.451 · E5 3.920 · ATX 5.234 · list 2.811 · blockquote 4.531 · E4 2.391 · E6 0.583\n' +
      'colors: local (red) · container (purple) · fence (amber) · semantic/global (dark red)',
  });
  Object.assign(option, {
    grid: { top: 170, left: 110, right: 60, bottom: 140 },
    xAxis: {
      type: 'category',
      data: rows.map((r) => r[1]),
      axisLabel: { ...baseText, fontSize: 18, interval: 0, lineHeight: 24 },
      axisLine: { lineStyle: { color: '#333333' } },
      axisTick: { show: false },
    },
    yAxis: {
      ...yAxisName('latency reduction vs H0 (%)'),
      min: -85,
      max: 95,
    },
    series: [
      {
        type: 'bar',
        barWidth: 74,
        data: rows.map((r) => ({
          value: d.latencyReduction.editFamily[r[0]],
          itemStyle: { color: r[2] },
          label: { position: r[3] },
        })),
        label: {
          show: true,
          ...baseText,
          fontSize: 19,
          fontWeight: 'bold',
          formatter: (p) => red(p.value),
        },
        markLine: PARITY_MARK,
      },
    ],
  });
  return option;
}

// ---------------------------------------------------------------- Figure 4 --

export function fig4LatencyAllocationPareto(d) {
  // Deterministic layout (from two visual passes): the four top-cluster
  // points cannot carry full data labels without collisions, so points get
  // short name-only labels and the exact values live in a fixed annotation
  // block over the empty right side of the plot.
  const labelPos = {
    HorseA: 'top',
    H4: 'right',
    H3: 'bottom',
    H2: 'right',
    H0: 'top',
    H1: 'top',
  };
  const mechs = ['HorseA', 'H3', 'H2', 'H4', 'H0', 'H1'];
  const points = mechs.map((m) => ({
    name: m,
    value: [d.allocation[m].kb, d.latencyReduction.macro[m]],
    count: d.allocation[m].count,
    label: {
      show: true,
      position: labelPos[m],
      distance: 10,
      ...baseText,
      fontSize: 19,
      fontWeight: 'bold',
      formatter: (q) => (q.data.name === 'HorseA' ? 'Horse-A v1' : q.data.name),
    },
  }));
  const valueBlock = mechs
    .map((m) => {
      const nm = m === 'HorseA' ? 'Horse-A v1' : m;
      const a = d.allocation[m];
      return `${nm.padEnd(11)} ${String(a.kb).padStart(6)} KB · ${red(d.latencyReduction.macro[m])} · ${a.speedup.toFixed(4)}× · ${a.count} allocs`;
    })
    .join('\n');
  const option = commonOption({
    title: 'Edit latency × allocation — Horse-A is non-dominated',
    subtitle:
      'Latency shown as EDIT_WRITE latency reduction vs H0 (algebraic restatement of the §4 speedup: reduction = 1 − 1/speedup)\n' +
      'x = median allocated KB per edit (1000 B convention) · bubble area ∝ median allocation count',
    bottomNote:
      'Sources: r8-values.json edit_write/* (allocation medians) + stage-b-facts-v1.json project macro (speedups, same authority as Figure 1)\n' +
      'dashed envelope: non-dominated frontier Horse-A → H4 · shaded region: lower allocation + higher reduction = better',
  });
  Object.assign(option, {
    grid: { top: 185, left: 110, right: 70, bottom: 130 },
    xAxis: {
      type: 'value',
      name: 'allocated KB per edit (median)',
      nameTextStyle: { ...baseText, fontSize: 19 },
      nameGap: 26,
      nameLocation: 'middle',
      min: 0,
      max: 150,
      axisLabel: { ...baseText, fontSize: 19 },
      splitLine: { lineStyle: { color: '#eeeeee' } },
    },
    yAxis: {
      ...yAxisName('latency reduction vs H0 (%)'),
      min: -5,
      max: 70,
    },
    series: [
      {
        type: 'scatter',
        data: points,
        symbolSize: (val, params) => Math.round(5.5 * Math.sqrt(params.data.count)),
        itemStyle: { color: (p) => mechColor(p.data.name), opacity: 0.85 },
        markLine: {
          symbol: 'none',
          silent: true,
          lineStyle: { color: COLORS.frontier, width: 2.5, type: [4, 4] },
          label: { show: false },
          data: [
            [
              { coord: [d.allocation.HorseA.kb, d.latencyReduction.macro.HorseA] },
              { coord: [d.allocation.H4.kb, d.latencyReduction.macro.H4] },
            ],
          ],
        },
        markArea: {
          silent: true,
          itemStyle: { color: 'rgba(178, 58, 72, 0.05)' },
          label: { show: false },
          data: [[{ xAxis: 0, yAxis: d.latencyReduction.macro.HorseA }, { xAxis: 10.6, yAxis: 70 }]],
        },
      },
    ],
  });
  option.graphic.push({
    type: 'text',
    left: 800,
    top: 235,
    style: {
      ...baseText,
      fontSize: 16,
      fill: '#333333',
      lineHeight: 27,
      text: 'median per-mechanism values (sealed sources):\n\n' + valueBlock,
    },
  });
  return option;
}

// ---------------------------------------------------------------- Figure 5 --

export function fig5PmuProfile(d) {
  const h4 = d.pmu.H4;
  const horseA = d.pmu.HorseA;
  const labelOpt = { show: true, position: 'top', ...baseText, fontSize: 20, fontWeight: 'bold' };
  const option = commonOption({
    title: 'H4 vs Horse-A — different efficiency profiles on the PMU panel',
    subtitle:
      'EDIT_WRITE representative cells, median per horse · host-specific: Haswell-EP (not a universal hardware law)\n' +
      'H4 is instruction-minimal; Horse-A trades more instructions for ~2× lower L1D miss pressure',
    bottomNote:
      'Source: pmu-cells-v1.json (sealed PMU analysis cells), frozen derivation rule\nreproduces the published PMU-EXPLANATION-v1 EDIT_WRITE table',
  });
  Object.assign(option, {
    grid: [
      { top: 195, left: 130, width: 520, height: 540 },
      { top: 195, left: 950, width: 520, height: 540 },
    ],
    legend: {
      top: 212,
      right: 55,
      data: ['H4', 'Horse-A v1'],
      textStyle: { ...baseText, fontSize: 19 },
      itemWidth: 34,
      itemGap: 40,
    },
    xAxis: [
      {
        gridIndex: 0,
        type: 'category',
        data: ['H4', 'Horse-A v1'],
        axisLabel: { ...baseText, fontSize: 21 },
        axisLine: { lineStyle: { color: '#333333' } },
        axisTick: { show: false },
      },
      {
        gridIndex: 1,
        type: 'category',
        data: ['L1D MPKI', 'branch MPKI'],
        axisLabel: { ...baseText, fontSize: 19 },
        axisLine: { lineStyle: { color: '#333333' } },
        axisTick: { show: false },
      },
    ],
    yAxis: [
      {
        gridIndex: 0,
        type: 'value',
        name: 'median instructions',
        nameTextStyle: { ...baseText, fontSize: 17 },
        nameGap: 20,
        max: 32000,
        axisLabel: { ...baseText, fontSize: 16 },
        splitLine: { lineStyle: { color: '#dddddd' } },
      },
      {
        gridIndex: 1,
        type: 'value',
        name: 'MPKI (lower is better)',
        nameTextStyle: { ...baseText, fontSize: 17 },
        nameGap: 20,
        max: 26,
        axisLabel: { ...baseText, fontSize: 16 },
        splitLine: { lineStyle: { color: '#dddddd' } },
      },
    ],
    series: [
      {
        name: 'H4',
        type: 'bar',
        xAxisIndex: 0,
        yAxisIndex: 0,
        barWidth: 130,
        itemStyle: { color: COLORS.H4 },
        data: [
          { value: h4.instructionsMedian },
          { value: horseA.instructionsMedian, itemStyle: { color: COLORS.HorseA } },
        ],
        label: { ...labelOpt, formatter: (p) => p.value.toLocaleString('en-US') },
      },
      {
        name: 'H4',
        type: 'bar',
        xAxisIndex: 1,
        yAxisIndex: 1,
        barWidth: 52,
        itemStyle: { color: 'rgba(32, 93, 140, 0.9)' },
        data: [
          { value: Number(h4.l1dMpkiMedian.toFixed(1)) },
          { value: Number(h4.branchMpkiMedian.toFixed(1)), itemStyle: { color: 'rgba(32, 93, 140, 0.45)' } },
        ],
        label: { ...labelOpt, fontSize: 18, formatter: (p) => p.value.toFixed(1) },
      },
      {
        name: 'Horse-A v1',
        type: 'bar',
        xAxisIndex: 1,
        yAxisIndex: 1,
        barWidth: 52,
        itemStyle: { color: 'rgba(178, 58, 72, 0.9)' },
        data: [
          { value: Number(horseA.l1dMpkiMedian.toFixed(1)) },
          { value: Number(horseA.branchMpkiMedian.toFixed(1)), itemStyle: { color: 'rgba(178, 58, 72, 0.45)' } },
        ],
        label: { ...labelOpt, fontSize: 18, formatter: (p) => p.value.toFixed(1) },
      },
    ],
  });
  return option;
}

// ---------------------------------------------------------------- Figure 6 --

export function fig6Rq8Sensitivity(d) {
  const cats = ['H4', 'Horse-A v1'];
  const mkSeries = (profile) =>
    cats.map((c, i) => {
      const key = i === 0 ? 'H4' : 'HorseA';
      return d.latencyReduction.rq8K1[profile][key];
    });
  const option = commonOption({
    title: 'RQ8 sensitivity — primary ThinLTO vs LTO-off (H4 and Horse-A only)',
    subtitle:
      'K1 = OPTIMIZATION_SENSITIVE: the macro gap is compiler-profile conditioned (≈1.32% → ≈7.07% in speedup terms)\n' +
      'H4 numerically leads under both profiles — not a "flip", not a significance claim',
    bottomNote:
      'Source: rq8-sensitivity-facts-v1.json K1 (primary_sealed / second_profile) · exactly one factor changed: lto = "thin" → lto = false\n' +
      'latency reduction = 1 − 1/speedup (§4.1) · frozen speedups (×): H4 2.4535 → 2.5982 · Horse-A 2.4214 → 2.4265',
  });
  Object.assign(option, {
    grid: { top: 200, left: 130, right: 60, bottom: 130 },
    legend: {
      top: 158,
      data: ['primary ThinLTO', 'LTO-off'],
      textStyle: { ...baseText, fontSize: 20 },
      itemWidth: 36,
      itemGap: 60,
    },
    xAxis: {
      type: 'category',
      data: cats,
      axisLabel: { ...baseText, fontSize: 22 },
      axisLine: { lineStyle: { color: '#333333' } },
      axisTick: { show: false },
    },
    yAxis: {
      ...yAxisName('latency reduction vs H0 (%)'),
      min: 0,
      max: 70,
    },
    series: [
      {
        name: 'primary ThinLTO',
        type: 'bar',
        barWidth: 110,
        itemStyle: { color: '#205d8c' },
        data: mkSeries('primary').map((v) => ({ value: v })),
        label: { show: true, position: 'top', ...baseText, fontSize: 20, fontWeight: 'bold', formatter: (p) => red(p.value) },
      },
      {
        name: 'LTO-off',
        type: 'bar',
        barWidth: 110,
        itemStyle: { color: '#8f4a52' },
        data: mkSeries('secondProfile').map((v) => ({ value: v })),
        label: { show: true, position: 'top', ...baseText, fontSize: 20, fontWeight: 'bold', formatter: (p) => red(p.value) },
      },
    ],
  });
  return option;
}

// ---------------------------------------------------------------- Figure 7 --

const VERDICT_COLORS = {
  STRONGLY_CONFIRMED: '#8f1d1d',
  'PARTIALLY_CONFIRMED / REFINED': '#a3661d',
  'NOT_CONFIRMED AS PRIMARY WEAKNESS': '#4a7d4a',
};
const PRIORITY_COLORS = { HIGHEST: '#8f1d1d', MEDIUM: '#a3661d', LOW: '#4a7d4a' };

export function fig7WeaknessMatrix(d) {
  const { rows, columns } = d.qualitative.figure7;
  const inject = (tpl) =>
    tpl
      .replace('{listSpeedup}', d.editFamily.list.toFixed(3))
      .replace('{blockquoteSpeedup}', d.editFamily.blockquote.toFixed(3))
      .replace('{e6Speedup}', d.editFamily.E6.toFixed(3))
      .replace('{horseAllocCount}', String(d.allocation.HorseA.count));

  const fontSize = 14;
  const lineHeight = 20;
  // x, pixel width, wrap width (bold ≈ 0.62×fontSize px/char, regular ≈
  // 0.54×fontSize, minus 28px cell padding, then a safety margin)
  const cols = [
    { x: 50, w: 185, chars: 15, header: 'Weakness' },
    { x: 235, w: 350, chars: 38, header: columns[0] },
    { x: 585, w: 455, chars: 50, header: columns[1] },
    { x: 1040, w: 330, chars: 28, header: columns[2] },
    { x: 1370, w: 185, chars: 15, header: columns[3] },
  ];
  const headerY = 158;
  const rowTop = 212;
  const rowH = 225;

  const graphics = [];
  // header
  cols.forEach((c) => {
    graphics.push({
      type: 'text',
      x: c.x + 14,
      y: headerY,
      style: {
        ...baseText,
        fontSize: 19,
        fontWeight: 'bold',
        text: wrap(c.header, Math.floor(c.chars * 1.2)),
        lineHeight: 26,
      },
    });
  });

  rows.forEach((r, i) => {
    const y = rowTop + i * rowH;
    const cells = [
      { text: `${r.id}\n\n${wrap(r.title, cols[0].chars)}`, bold: true, fill: '#f2f2f2', color: '#1a1a1a' },
      { text: wrap(r.risk, cols[1].chars), color: '#1a1a1a' },
      { text: wrap(inject(r.observedTemplate), cols[2].chars), color: '#1a1a1a' },
      {
        text: `${wrap(r.verdict, cols[3].chars)}\n\n${wrap(r.verdictNote, cols[3].chars)}`,
        bold: true,
        color: VERDICT_COLORS[r.verdict] ?? '#1a1a1a',
      },
      {
        text: `${r.priority}\n\n${wrap(r.priorityNote, cols[4].chars)}`,
        bold: true,
        color: PRIORITY_COLORS[r.priority] ?? '#1a1a1a',
      },
    ];
    cells.forEach((cell, j) => {
      const c = cols[j];
      graphics.push({
        type: 'rect',
        shape: { x: c.x, y, width: c.w, height: rowH },
        style: { fill: cell.fill ?? '#ffffff', stroke: '#cccccc', lineWidth: 1 },
      });
      graphics.push({
        type: 'text',
        x: c.x + 14,
        y: y + 16,
        style: {
          ...baseText,
          fontSize,
          fontWeight: cell.bold ? 'bold' : 'normal',
          fill: cell.color,
          text: cell.text,
          lineHeight,
        },
      });
    });
  });

  const option = commonOption({
    title: 'Pre-registered weaknesses vs observed evidence',
    subtitle:
      'Frozen design/implementation/falsification records prohibited silently repairing these after results\n' +
      'verdicts follow the merged baseline record §13; evidence numbers come from the sealed extraction (Figures 3–4 authorities)',
  });
  option.graphic.push(...graphics);
  return option;
}

// ---------------------------------------------------------------- Figure 8 --

export function fig8NextMechanismPriority(d) {
  const items = [...d.qualitative.figure8.items].sort((a, b) => a.rank - b.rank);
  const inject = (tpl) =>
    tpl
      .replace('{e6Speedup}', d.editFamily.E6.toFixed(3))
      .replace('{bottomDecileSpeedup}', d.bottomDecile.HorseA.toFixed(3))
      .replace('{e4Speedup}', d.editFamily.E4.toFixed(3))
      .replace('{horseAllocCount}', String(d.allocation.HorseA.count));

  const barColors = ['#8f1d1d', '#b25b3a', '#a3661d', '#8c8c8c'];
  const top = 205;
  const rowH = 158;
  const barMax = 560;
  const barH = 56;

  const graphics = [];
  items.forEach((item, i) => {
    const y = top + i * rowH;
    const w = Math.round(barMax * (1 - (item.rank - 1) / 4.8));
    graphics.push({
      type: 'rect',
      shape: { x: 120, y, width: w, height: barH },
      style: { fill: barColors[i], stroke: 'none' },
    });
    graphics.push({
      type: 'text',
      x: 136,
      y: y + 13,
      style: {
        ...baseText,
        fontSize: 26,
        fontWeight: 'bold',
        fill: '#ffffff',
        text: item.id,
      },
    });
    // title sits right of the bar end — always fits, never clipped
    graphics.push({
      type: 'text',
      x: 120 + w + 24,
      y: y + 14,
      style: {
        ...baseText,
        fontSize: 21,
        fontWeight: 'bold',
        fill: '#1a1a1a',
        text: `${item.title}  (priority ${item.rank} of 4)`,
      },
    });
    graphics.push({
      type: 'text',
      x: 120,
      y: y + barH + 12,
      style: {
        ...baseText,
        fontSize: 17,
        fill: '#333333',
        text: `basis: ${item.basis}\nevidence: ${inject(item.evidenceTemplate)}`,
        lineHeight: 25,
      },
    });
  });

  const option = commonOption({
    title: 'Weakness → next-mechanism priority (evidence-backed ordering)',
    subtitle: d.qualitative.figure8.subtitle,
  });
  option.graphic.push(...graphics);
  return option;
}
