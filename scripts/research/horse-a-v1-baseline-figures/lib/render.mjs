// ECharts SSR -> SVG -> PNG rendering helper. Rendering is deterministic:
// fixed canvas size, fixed fonts, animation disabled, no time-dependent
// content in the options.

import { Resvg } from '@resvg/resvg-js';
import * as echarts from 'echarts';

export const FIG_WIDTH = 1600;
export const FIG_HEIGHT = 900;

const FONT_FILES = [
  '/usr/share/fonts/truetype/dejavu/DejaVuSans.ttf',
  '/usr/share/fonts/truetype/dejavu/DejaVuSans-Bold.ttf',
];
const FONT_FAMILY = 'DejaVu Sans';

export function renderChartToPng(option) {
  const chart = echarts.init(null, null, {
    renderer: 'svg',
    ssr: true,
    width: FIG_WIDTH,
    height: FIG_HEIGHT,
  });
  chart.setOption(option);
  const svg = chart.renderToSVGString();
  chart.dispose();

  const resvg = new Resvg(svg, {
    font: {
      fontFiles: FONT_FILES,
      loadSystemFonts: false,
      defaultFontFamily: FONT_FAMILY,
    },
    background: 'white',
  });
  return resvg.render().asPng();
}
