import {
  forceCollide,
  forceLink,
  forceManyBody,
  forceSimulation,
  forceX,
  forceY,
  type SimulationNodeDatum,
} from 'd3-force';
import type { Edge } from './generated/Edge';
import type { PlanSummary } from './generated/PlanSummary';
import type { RelationKind } from './generated/RelationKind';

export type LayoutNode = { id: string; plan: PlanSummary; r: number; x: number; y: number };
export type LayoutEdge = {
  from: string;
  to: string;
  kind: RelationKind | 'shared';
  /** Shared topic tags (shared edges only). */
  topics: string[];
};
export type Box = { x: number; y: number; w: number; h: number };
type Point = { x: number; y: number };

/** Circle radius: grows with the number of tasks, capped so big plans stay readable. */
export const nodeRadius = (p: PlanSummary) => Math.min(12 + p.task_count * 3, 40);

/**
 * Edges worth drawing and laying out: declared relations between known plans, plus
 * shared-topic edges, strongest first, kept only while one of the two plans has
 * fewer than `perPlan` of them (bounds a large graph to about `perPlan × plans` lines).
 */
export function pickEdges(edges: Edge[], ids: Set<string>, perPlan = 3): LayoutEdge[] {
  const known = edges.filter((e) => ids.has(e.from) && ids.has(e.to) && e.from !== e.to);
  const declared: LayoutEdge[] = [];
  const shared: LayoutEdge[] = [];
  for (const e of known) {
    if (e.kind.type === 'declared')
      declared.push({ from: e.from, to: e.to, kind: e.kind.value, topics: [] });
    else shared.push({ from: e.from, to: e.to, kind: 'shared', topics: e.kind.value });
  }
  const degree = new Map<string, number>();
  const deg = (id: string) => degree.get(id) ?? 0;
  const kept = shared
    .map((e, i) => ({ e, i }))
    .sort((a, b) => b.e.topics.length - a.e.topics.length || a.i - b.i)
    .filter(({ e }) => {
      if (deg(e.from) >= perPlan && deg(e.to) >= perPlan) return false;
      degree.set(e.from, deg(e.from) + 1);
      degree.set(e.to, deg(e.to) + 1);
      return true;
    })
    .map(({ e }) => e);
  return [...declared, ...kept];
}

type SimNode = SimulationNodeDatum & { id: string; plan: PlanSummary; r: number };
type SimLink = { source: string | SimNode; target: string | SimNode; shared: boolean; w: number };

/**
 * Static force layout, run to convergence off-screen. Seeds are deterministic
 * (a sunflower spiral in input order, or `seed` positions from an earlier layout),
 * and d3-force's own jitter is seeded, so the same input gives the same picture.
 */
export function layoutGraph(
  plans: PlanSummary[],
  edges: LayoutEdge[],
  seed?: Map<string, Point>,
  ticks = plans.length > 150 ? 150 : 300,
): LayoutNode[] {
  const big = plans.length > 150;
  const nodes: SimNode[] = plans.map((p, i) => {
    const s = seed?.get(p.id);
    const d = 40 * Math.sqrt(i + 0.5);
    return {
      id: p.id,
      plan: p,
      r: nodeRadius(p),
      x: s?.x ?? Math.cos(i * 2.4) * d,
      y: s?.y ?? Math.sin(i * 2.4) * d * 0.8,
    };
  });
  const links: SimLink[] = edges.map((e) => ({
    source: e.from,
    target: e.to,
    shared: e.kind === 'shared',
    w: e.topics.length,
  }));
  const sim = forceSimulation(nodes)
    .force('charge', forceManyBody<SimNode>().strength(-900).distanceMax(1200))
    .force(
      'link',
      forceLink<SimNode, SimLink>(links)
        .id((d) => d.id)
        .distance((l) => (l.shared ? 280 : 200))
        .strength((l) => (l.shared ? Math.min(0.03 * l.w, 0.15) : 0.5)),
    )
    // Room for the progress ring and the label under each circle.
    .force('collide', forceCollide<SimNode>((d) => d.r + 44).iterations(big ? 1 : 3))
    .force('x', forceX<SimNode>(0).strength(0.03))
    // A stronger vertical pull gives a wide picture that suits the wide map panel.
    .force('y', forceY<SimNode>(0).strength(0.12))
    // Cool down to d3's default alphaMin in exactly `ticks` steps.
    .alphaDecay(1 - Math.pow(0.001, 1 / ticks))
    .stop();
  sim.tick(ticks);
  return nodes.map((n) => ({ id: n.id, plan: n.plan, r: n.r, x: n.x ?? 0, y: n.y ?? 0 }));
}

/**
 * Bounding box of the circles plus padding (wider at the sides for labels, room
 * below for the label under the lowest circle), at least `min` in size so a small
 * map is not blown up.
 */
export function fitBox(nodes: LayoutNode[], pad = { x: 110, y: 50 }, min = { w: 0, h: 0 }): Box {
  if (!nodes.length) return { x: -200, y: -150, w: 400, h: 300 };
  const x0 = Math.min(...nodes.map((n) => n.x - n.r)) - pad.x;
  const y0 = Math.min(...nodes.map((n) => n.y - n.r)) - pad.y;
  const x1 = Math.max(...nodes.map((n) => n.x + n.r)) + pad.x;
  const y1 = Math.max(...nodes.map((n) => n.y + n.r)) + pad.y;
  const w = Math.max(x1 - x0, min.w);
  const h = Math.max(y1 - y0, min.h);
  return { x: (x0 + x1 - w) / 2, y: (y0 + y1 - h) / 2, w, h };
}

/** Scale `box` by `factor` (< 1 zooms in) around `at`, which stays at the same spot. */
export function zoomAt(box: Box, at: Point, factor: number): Box {
  return {
    x: at.x - (at.x - box.x) * factor,
    y: at.y - (at.y - box.y) * factor,
    w: box.w * factor,
    h: box.h * factor,
  };
}
