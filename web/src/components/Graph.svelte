<script lang="ts">
  import type { Edge } from '../lib/generated/Edge';
  import type { PlanSummary } from '../lib/generated/PlanSummary';
  import {
    fitBox,
    layoutGraph,
    pickEdges,
    zoomAt,
    type Box,
    type LayoutEdge,
    type LayoutNode,
  } from '../lib/graph';
  import { t } from '../lib/i18n';
  import { projectColor } from '../lib/projectColor';

  /**
   * Static plan graph: laid out once per change of the graph's shape, drawn as SVG.
   * Zoom with the wheel or the buttons, pan by dragging the background, drag a circle
   * to move it; a click (or Enter) on a circle calls `onopen`.
   */
  let {
    plans,
    edges,
    highlight = null,
    showShared = true,
    topicLabel,
    onopen,
  }: {
    plans: PlanSummary[];
    edges: Edge[];
    /** Plans of the selected topic; the others are dimmed. */
    highlight?: Set<string> | null;
    showShared?: boolean;
    topicLabel: (tag: string) => string;
    onopen: (planId: string) => void;
  } = $props();

  // Layout only when the set of plans or edges changes; progress updates reuse positions.
  const shape = $derived(
    `${plans.map((p) => `${p.id}:${p.task_count}`).join(',')}|${edges
      .map((e) => `${e.from}>${e.to}:${e.kind.type}:${String(e.kind.value)}`)
      .join(',')}`,
  );
  let lastShape = '';
  let lastLayout: { nodes: LayoutNode[]; edges: LayoutEdge[] } = { nodes: [], edges: [] };
  const layout = $derived.by(() => {
    if (shape === lastShape) return lastLayout;
    const picked = pickEdges(edges, new Set(plans.map((p) => p.id)));
    // Start from where known plans were, so a new plan does not reshuffle the map.
    const seed = new Map(lastLayout.nodes.map((n) => [n.id, { x: n.x, y: n.y }]));
    lastShape = shape;
    lastLayout = { nodes: layoutGraph(plans, picked, seed), edges: picked };
    return lastLayout;
  });

  let width = $state(800);
  let height = $state(520);
  /** Positions of circles the user dragged (reset by a new layout). */
  let moved = $state<Record<string, { x: number; y: number }>>({});
  let view = $state<Box | null>(null);
  $effect(() => {
    void layout;
    moved = {};
    view = null;
  });

  const byId = $derived(new Map(plans.map((p) => [p.id, p])));
  const nodes = $derived(
    layout.nodes.map((n) => ({ ...n, ...moved[n.id], plan: byId.get(n.id) ?? n.plan })),
  );
  const pos = $derived(new Map(nodes.map((n) => [n.id, n])));
  // Never scale a small map above 1:1, so labels keep their normal size.
  const home = $derived(fitBox(layout.nodes, undefined, { w: width, h: height }));
  const vb = $derived(view ?? home);

  let svg = $state<SVGSVGElement>();
  /** SVG units per screen pixel (preserveAspectRatio="xMidYMid meet"). */
  const unitsPerPx = () => {
    const r = svg?.getBoundingClientRect();
    return r && r.width && r.height ? Math.max(vb.w / r.width, vb.h / r.height) : 1;
  };
  /** Screen point → SVG point. */
  function toSvg(clientX: number, clientY: number) {
    const r = svg!.getBoundingClientRect();
    const s = unitsPerPx();
    return {
      x: vb.x - (r.width * s - vb.w) / 2 + (clientX - r.left) * s,
      y: vb.y - (r.height * s - vb.h) / 2 + (clientY - r.top) * s,
    };
  }
  const clampZoom = (factor: number) => {
    const w = vb.w * factor;
    const min = home.w / 8;
    const max = home.w * 3;
    return w < min ? min / vb.w : w > max ? max / vb.w : factor;
  };
  function zoom(factor: number, at = { x: vb.x + vb.w / 2, y: vb.y + vb.h / 2 }) {
    view = zoomAt(vb, at, clampZoom(factor));
  }

  $effect(() => {
    const el = svg;
    if (!el) return;
    // Non-passive, so the page does not scroll while zooming.
    const onWheel = (e: WheelEvent) => {
      e.preventDefault();
      zoom(Math.exp(e.deltaY * 0.0015), toSvg(e.clientX, e.clientY));
    };
    el.addEventListener('wheel', onWheel, { passive: false });
    return () => el.removeEventListener('wheel', onWheel);
  });

  // Pointer: drag a circle to move it, drag the background to pan, a short tap opens.
  let drag: {
    id: string | null;
    x: number;
    y: number;
    start: { x: number; y: number };
    far: boolean;
  } | null = null;
  function down(e: PointerEvent) {
    if (e.button !== 0) return;
    const id = (e.target as Element).closest<SVGGElement>('[data-plan]')?.dataset.plan ?? null;
    const n = id ? pos.get(id) : undefined;
    drag = {
      id,
      x: e.clientX,
      y: e.clientY,
      start: n ? { x: n.x, y: n.y } : { x: vb.x, y: vb.y },
      far: false,
    };
    svg?.setPointerCapture?.(e.pointerId);
  }
  function move(e: PointerEvent) {
    if (!drag) return;
    const dx = e.clientX - drag.x;
    const dy = e.clientY - drag.y;
    if (!drag.far && Math.hypot(dx, dy) < 4) return;
    drag.far = true;
    const s = unitsPerPx();
    if (drag.id)
      moved = { ...moved, [drag.id]: { x: drag.start.x + dx * s, y: drag.start.y + dy * s } };
    else view = { ...vb, x: drag.start.x - dx * s, y: drag.start.y - dy * s };
  }
  function up() {
    if (drag?.id && !drag.far) onopen(drag.id);
    drag = null;
  }
  function key(e: KeyboardEvent, id: string) {
    if (e.key === 'Enter' || e.key === ' ') {
      e.preventDefault();
      onopen(id);
    }
  }

  const lit = (id: string) => !highlight || highlight.has(id);
  /** Labels get noisy when zoomed far out on a big map; selected plans keep theirs. */
  const labels = $derived(nodes.length <= 40 || vb.w / Math.max(width, 1) < 1.8);
  const short = (s: string) => (s.length > 32 ? `${s.slice(0, 31)}…` : s);

  function line(e: LayoutEdge) {
    const a = pos.get(e.from);
    const b = pos.get(e.to);
    if (!a || !b) return null;
    const dx = b.x - a.x;
    const dy = b.y - a.y;
    const d = Math.hypot(dx, dy) || 1;
    // Declared arrows stop at the target's ring so the arrowhead stays visible.
    const back = e.kind === 'shared' ? 0 : b.r + 6;
    return { x1: a.x, y1: a.y, x2: b.x - (dx / d) * back, y2: b.y - (dy / d) * back };
  }
  const drawn = $derived(layout.edges.filter((e) => showShared || e.kind !== 'shared'));
</script>

<div class="gwrap" bind:clientWidth={width} bind:clientHeight={height}>
  <svg
    bind:this={svg}
    viewBox="{vb.x} {vb.y} {vb.w} {vb.h}"
    preserveAspectRatio="xMidYMid meet"
    role="group"
    aria-label={$t('map.graphLabel', { n: nodes.length })}
    onpointerdown={down}
    onpointermove={move}
    onpointerup={up}
    onpointercancel={() => (drag = null)}
  >
    <defs>
      <marker
        id="pa-arrow"
        viewBox="0 0 10 10"
        refX="9"
        refY="5"
        markerWidth="7"
        markerHeight="7"
        orient="auto"
      >
        <path class="arrowhead" d="M0,0L10,5L0,10z" />
      </marker>
    </defs>
    {#each drawn as e (`${e.from}>${e.to}:${e.kind}`)}
      {@const l = line(e)}
      {#if l}
        <line
          class="edge {e.kind}"
          class:dim={!(lit(e.from) && lit(e.to))}
          {...l}
          stroke-width={e.kind === 'shared' ? Math.min(e.topics.length, 4) : 2}
          marker-end={e.kind === 'shared' ? undefined : 'url(#pa-arrow)'}
        >
          {#if e.kind === 'shared'}
            <title>{$t('map.sharedTopics', { topics: e.topics.map(topicLabel).join(', ') })}</title>
          {/if}
        </line>
      {/if}
    {/each}
    {#each nodes as n (n.id)}
      {@const ring = 2 * Math.PI * (n.r + 4)}
      {@const done = n.plan.task_count ? n.plan.done_count / n.plan.task_count : 0}
      <g
        class="node"
        class:dim={!lit(n.id)}
        transform="translate({n.x},{n.y})"
        data-plan={n.id}
        role="button"
        tabindex="0"
        aria-label={$t('map.openPlan', { title: n.plan.title })}
        onkeydown={(e) => key(e, n.id)}
      >
        <title>{n.plan.title} · {n.plan.project} — {$t('map.clickToView')}</title>
        <circle class="track" r={n.r + 4} />
        <circle
          class="ring"
          r={n.r + 4}
          stroke-dasharray="{ring * done} {ring}"
          transform="rotate(-90)"
        />
        <circle class="body" r={n.r} style="fill:{$projectColor(n.plan.project)}" />
        <text class="count" y="4">{n.plan.task_count}</text>
        {#if labels || (highlight && highlight.has(n.id))}
          <text class="label" y={n.r + 22}>{short(n.plan.title)}</text>
        {/if}
      </g>
    {/each}
  </svg>
  <div class="zoom">
    <button onclick={() => zoom(0.75)} aria-label={$t('map.zoomIn')} title={$t('map.zoomIn')}>
      +
    </button>
    <button onclick={() => zoom(1 / 0.75)} aria-label={$t('map.zoomOut')} title={$t('map.zoomOut')}>
      −
    </button>
    <button onclick={() => ((view = null), (moved = {}))} title={$t('map.fit')}>
      {$t('map.fit')}
    </button>
  </div>
</div>

<style>
  .gwrap {
    position: relative;
    background: var(--panel);
    border: 1px solid var(--line);
    border-radius: 12px;
    overflow: hidden;
  }
  svg {
    display: block;
    width: 100%;
    height: 520px;
    touch-action: none;
    cursor: grab;
    user-select: none;
  }
  .zoom {
    position: absolute;
    right: 10px;
    top: 10px;
    display: flex;
    gap: 4px;
  }
  .zoom button {
    min-width: 30px;
    padding: 3px 8px;
  }
  .edge {
    stroke: var(--accent);
  }
  .edge.relates_to {
    stroke-dasharray: 2 4;
  }
  .edge.shared {
    stroke: var(--faint);
    stroke-opacity: 0.5;
    stroke-dasharray: 5 5;
  }
  .arrowhead {
    fill: var(--accent);
  }
  .node {
    cursor: pointer;
  }
  .node:focus {
    outline: none;
  }
  .node:focus-visible .body {
    stroke: var(--ink);
    stroke-width: 3;
  }
  .track,
  .ring {
    fill: none;
    stroke-width: 3;
  }
  .track {
    stroke: var(--line);
  }
  .ring {
    stroke: var(--ok);
  }
  .count {
    fill: #fff;
    font-weight: 700;
    font-size: 12px;
    text-anchor: middle;
    pointer-events: none;
  }
  .label {
    font-size: 12px;
    fill: var(--ink);
    text-anchor: middle;
    paint-order: stroke;
    stroke: var(--panel);
    stroke-width: 4;
    pointer-events: none;
  }
  .dim {
    opacity: 0.12;
  }
  @media (max-width: 900px) {
    svg {
      height: 380px;
    }
  }
</style>
