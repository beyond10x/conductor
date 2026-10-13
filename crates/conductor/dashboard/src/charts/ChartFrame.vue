<script setup lang="ts">
// A chart drawn as inline SVG (story:dashboard-charts-ui): stacked bars for counts or one line per
// key for rates and durations, the days below and the values beside, a legend naming each key,
// and a tooltip for the day under the pointer or, with the keyboard, the day the arrow keys reach.
// No chart library and no style attribute: the server's policy allows neither; every colour comes
// from a class of the stylesheet.
import { computed, ref } from 'vue';
import { formatValue, shortDay, type ChartData, type Line } from '../api/series';
import { niceScale } from './scale';

const props = withDefaults(
  defineProps<{
    data: ChartData;
    label: string;
    kind: 'bars' | 'line';
    compact?: boolean;
    /** A target to draw across the chart, in the chart's unit. */
    target?: number | null;
  }>(),
  { compact: false, target: null },
);

const LEFT = 52;
const RIGHT = 10;
const TOP = 10;
const BOTTOM = 22;

const width = computed(() => (props.compact ? 320 : 640));
const height = computed(() => (props.compact ? 130 : 220));
const plotWidth = computed(() => width.value - LEFT - RIGHT);
const plotHeight = computed(() => height.value - TOP - BOTTOM);
const count = computed(() => Math.max(1, props.data.days.length));
const band = computed(() => plotWidth.value / count.value);

/** The highest value drawn: the highest day's stack for bars, the highest point for lines. */
const highest = computed(() => {
  let top = props.target ?? 0;
  props.data.days.forEach((_, day) => {
    if (props.kind === 'bars') {
      top = Math.max(top, props.data.lines.reduce((sum, line) => sum + (line.values[day] ?? 0), 0));
    } else {
      for (const line of props.data.lines) {
        top = Math.max(top, line.values[day] ?? 0);
      }
    }
  });
  return top;
});

const scale = computed(() => niceScale(highest.value, props.data.unit));
const shownTicks = computed(() => {
  const ticks = scale.value.ticks;
  return props.compact ? [ticks[0] ?? 0, ticks[ticks.length - 1] ?? 0] : ticks;
});

/** A coordinate to two places: the drawing does not need more, and the markup stays short. */
function round(at: number): number {
  return Math.round(at * 100) / 100;
}

function y(value: number): number {
  return round(TOP + plotHeight.value - (value / scale.value.max) * plotHeight.value);
}

function centre(day: number): number {
  return round(LEFT + band.value * (day + 0.5));
}

function left(day: number): number {
  return round(LEFT + band.value * day);
}

/** The class that colours a line: its tone, or its place. */
function colour(line: Line, index: number): string {
  return line.tone === undefined ? `s${index % 6}` : `tone-${line.tone}`;
}

interface Bar {
  key: string;
  x: number;
  y: number;
  width: number;
  height: number;
  colour: string;
}

const bars = computed<Bar[]>(() => {
  if (props.kind !== 'bars') {
    return [];
  }
  const drawn: Bar[] = [];
  const barWidth = Math.max(1, band.value * 0.7);
  props.data.days.forEach((day, at) => {
    let below = 0;
    props.data.lines.forEach((line, index) => {
      const value = line.values[at] ?? 0;
      if (value <= 0) {
        return;
      }
      const top = y(below + value);
      drawn.push({
        key: `${day}-${line.key}`,
        x: round(LEFT + band.value * at + (band.value - barWidth) / 2),
        y: top,
        width: round(barWidth),
        height: round(y(below) - top),
        colour: colour(line, index),
      });
      below += value;
    });
  });
  return drawn;
});

interface Path {
  key: string;
  d: string;
  colour: string;
  /** The points with no neighbour, which a path alone would not show. */
  alone: { x: number; y: number }[];
}

const paths = computed<Path[]>(() => {
  if (props.kind !== 'line') {
    return [];
  }
  return props.data.lines.map((line, index) => {
    let d = '';
    let drawing = false;
    const alone: { x: number; y: number }[] = [];
    line.values.forEach((value, at) => {
      if (value === null) {
        drawing = false;
        return;
      }
      const point = `${centre(at).toFixed(1)} ${y(value).toFixed(1)}`;
      d += `${drawing ? 'L' : 'M'}${point} `;
      const before = line.values[at - 1];
      const after = line.values[at + 1];
      if ((before === null || before === undefined) && (after === null || after === undefined)) {
        alone.push({ x: centre(at), y: y(value) });
      }
      drawing = true;
    });
    return { key: line.key, d: d.trim(), colour: colour(line, index), alone };
  });
});

/** The days the axis names: the first and the last, and between them two more on a wide chart. */
const dayLabels = computed(() => {
  const last = props.data.days.length - 1;
  const picks = props.compact || last < 3 ? [0, last] : [0, Math.round(last / 3), Math.round((2 * last) / 3), last];
  return [...new Set(picks)]
    .filter((at) => at >= 0)
    .map((at) => ({
      at,
      text: shortDay(props.data.days[at] ?? ''),
      x: centre(at),
      anchor: at === 0 ? 'start' : at === last ? 'end' : 'middle',
    }));
});

const active = ref<number | null>(null);

function point(day: number): void {
  active.value = day;
}

function leave(): void {
  active.value = null;
}

function focus(): void {
  active.value = props.data.days.length - 1;
}

function key(event: KeyboardEvent): void {
  const last = props.data.days.length - 1;
  const now = active.value ?? last;
  const next =
    event.key === 'ArrowLeft'
      ? Math.max(0, now - 1)
      : event.key === 'ArrowRight'
        ? Math.min(last, now + 1)
        : event.key === 'Home'
          ? 0
          : event.key === 'End'
            ? last
            : null;
  if (next !== null) {
    event.preventDefault();
    active.value = next;
  }
}

/** What the tooltip says for the day pointed at: the day, each key's value, and a stack's total. */
const tip = computed<string[]>(() => {
  const day = active.value;
  if (day === null || props.data.days[day] === undefined) {
    return [];
  }
  const unit = props.data.unit;
  const lines = [shortDay(props.data.days[day]!)];
  for (const line of props.data.lines) {
    lines.push(`${line.label}: ${formatValue(line.values[day] ?? null, unit)}`);
  }
  if (props.kind === 'bars' && props.data.lines.length > 1) {
    const total = props.data.lines.reduce((sum, line) => sum + (line.values[day] ?? 0), 0);
    lines.push(`total: ${formatValue(total, unit)}`);
  }
  if (props.data.lines.length === 0) {
    lines.push('nothing');
  }
  return lines;
});

const tipBox = computed(() => {
  const day = active.value ?? 0;
  const longest = Math.max(...tip.value.map((line) => line.length));
  const boxWidth = Math.ceil(longest * 6.6 + 16);
  const boxHeight = tip.value.length * 16 + 8;
  let x = centre(day) + 8;
  if (x + boxWidth > width.value - RIGHT) {
    x = Math.max(LEFT, centre(day) - 8 - boxWidth);
  }
  return { x, y: TOP, width: boxWidth, height: boxHeight };
});

const summary = computed(() => {
  const days = props.data.days;
  const what = props.kind === 'bars' ? 'stacked bar chart' : 'line chart';
  const range =
    days.length > 0 ? `, ${shortDay(days[0]!)} to ${shortDay(days[days.length - 1]!)}` : '';
  const keys = props.data.lines.map((line) => line.label).join(', ');
  return `${props.label}: ${what} of ${days.length} days${range}${keys ? `; ${keys}` : ''}. Arrow keys step through the days.`;
});

const targetY = computed(() => (props.target === null ? null : y(props.target)));
</script>

<template>
  <figure class="chart" :class="{ compact }">
    <svg
      :viewBox="`0 0 ${width} ${height}`"
      role="img"
      :aria-label="summary"
      tabindex="0"
      focusable="true"
      @mouseleave="leave"
      @focus="focus"
      @blur="leave"
      @keydown="key"
    >
      <g class="grid" aria-hidden="true">
        <line
          v-for="tick in shownTicks"
          :key="tick"
          :x1="LEFT"
          :x2="width - RIGHT"
          :y1="y(tick)"
          :y2="y(tick)"
        />
      </g>
      <g class="y-axis" aria-hidden="true">
        <text v-for="tick in shownTicks" :key="tick" :x="LEFT - 6" :y="y(tick) + 4" text-anchor="end">
          {{ formatValue(tick, data.unit) }}
        </text>
      </g>
      <g class="x-axis" aria-hidden="true">
        <line class="axis-line" :x1="LEFT" :x2="width - RIGHT" :y1="TOP + plotHeight" :y2="TOP + plotHeight" />
        <text
          v-for="label in dayLabels"
          :key="label.at"
          :x="label.x"
          :y="height - 6"
          :text-anchor="label.anchor"
        >
          {{ label.text }}
        </text>
      </g>
      <g class="hits">
        <rect
          v-for="(day, at) in data.days"
          :key="day"
          class="hit"
          :class="{ active: active === at }"
          :x="left(at)"
          :y="TOP"
          :width="round(band)"
          :height="plotHeight"
          @mouseenter="point(at)"
        />
      </g>
      <g class="marks" aria-hidden="true">
        <rect
          v-for="bar in bars"
          :key="bar.key"
          class="bar"
          :class="bar.colour"
          :x="bar.x"
          :y="bar.y"
          :width="bar.width"
          :height="bar.height"
        />
        <template v-for="path in paths" :key="path.key">
          <path class="line" :class="path.colour" :d="path.d" />
          <circle
            v-for="(dot, at) in path.alone"
            :key="at"
            class="dot"
            :class="path.colour"
            :cx="dot.x"
            :cy="dot.y"
            r="3"
          />
        </template>
        <line
          v-if="targetY !== null"
          class="target-line"
          :x1="LEFT"
          :x2="width - RIGHT"
          :y1="targetY"
          :y2="targetY"
        />
      </g>
      <g v-if="tip.length > 0" class="tip" :transform="`translate(${tipBox.x} ${tipBox.y})`">
        <rect :width="tipBox.width" :height="tipBox.height" rx="6" />
        <text
          v-for="(line, at) in tip"
          :key="at"
          :class="{ 'tip-day': at === 0 }"
          x="8"
          :y="18 + at * 16"
        >
          {{ line }}
        </text>
      </g>
    </svg>
    <ul v-if="data.lines.length > 0" class="legend">
      <li v-for="(line, index) in data.lines" :key="line.key">
        <span class="swatch" :class="colour(line, index)" aria-hidden="true"></span>{{ line.label }}
      </li>
    </ul>
    <p class="sr-only" aria-live="polite">{{ tip.join(', ') }}</p>
  </figure>
</template>

<style scoped>
figure.chart {
  margin: 0;
  display: flex;
  flex-direction: column;
  gap: var(--space-2);
  min-width: 0;
}
.target-line {
  stroke: var(--amber);
  stroke-width: 1.5;
  stroke-dasharray: 4 3;
}
</style>
