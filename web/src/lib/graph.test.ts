import { describe, expect, it } from 'vitest';
import type { Edge } from './generated/Edge';
import type { PlanSummary } from './generated/PlanSummary';
import { fitBox, layoutGraph, nodeRadius, pickEdges, zoomAt } from './graph';

const plan = (id: string, tasks = 3): PlanSummary => ({
  id,
  project: id.split('-')[1],
  title: `Plan ${id}`,
  tags: [],
  created: null,
  task_count: tasks,
  done_count: 0,
  structured: true,
  warnings_count: 0,
  tasks: [],
});
const nodes = [
  'PLAN-a-001',
  'PLAN-a-002',
  'PLAN-b-001',
  'PLAN-b-002',
  'PLAN-c-001',
  'PLAN-c-002',
].map((id, i) => plan(id, i + 1));
const edges: Edge[] = [
  { from: 'PLAN-a-002', to: 'PLAN-a-001', kind: { type: 'declared', value: 'depends_on' } },
  { from: 'PLAN-b-001', to: 'PLAN-a-001', kind: { type: 'declared', value: 'extends' } },
  { from: 'PLAN-c-001', to: 'PLAN-b-002', kind: { type: 'declared', value: 'relates_to' } },
  { from: 'PLAN-c-002', to: 'PLAN-gone', kind: { type: 'declared', value: 'relates_to' } },
  { from: 'PLAN-a-001', to: 'PLAN-c-002', kind: { type: 'shared_topics', value: ['tech/jwt'] } },
];

describe('graph layout', () => {
  it('nodes_do_not_overlap', () => {
    const laid = layoutGraph(nodes, pickEdges(edges, new Set(nodes.map((n) => n.id))));
    expect(laid).toHaveLength(nodes.length);
    for (let i = 0; i < laid.length; i++)
      for (let j = i + 1; j < laid.length; j++) {
        const a = laid[i];
        const b = laid[j];
        const d = Math.hypot(a.x - b.x, a.y - b.y);
        // Circles (plus progress ring) never touch; small slack for the solver.
        expect(d, `${a.id} ↔ ${b.id}`).toBeGreaterThan(a.r + b.r + 8);
      }
  });

  it('is deterministic and keeps linked plans closer than unlinked ones', () => {
    const picked = pickEdges(edges, new Set(nodes.map((n) => n.id)));
    const a = layoutGraph(nodes, picked);
    const b = layoutGraph(nodes, picked);
    expect(a.map((n) => [n.x, n.y])).toEqual(b.map((n) => [n.x, n.y]));
    const at = (id: string) => a.find((n) => n.id === id)!;
    const dist = (x: string, y: string) => Math.hypot(at(x).x - at(y).x, at(x).y - at(y).y);
    const pairs = a.flatMap((x, i) => a.slice(i + 1).map((y) => dist(x.id, y.id)));
    const mean = pairs.reduce((s, d) => s + d, 0) / pairs.length;
    expect(dist('PLAN-a-002', 'PLAN-a-001')).toBeLessThan(mean);
    expect(dist('PLAN-b-001', 'PLAN-a-001')).toBeLessThan(mean);
  });

  it('pickEdges drops dangling edges and caps shared-topic edges per plan', () => {
    const ids = new Set(nodes.map((n) => n.id));
    const picked = pickEdges(edges, ids);
    expect(picked.map((e) => e.kind)).toEqual(['depends_on', 'extends', 'relates_to', 'shared']);
    // A clique: every pair shares topics; weight = number of shared topics.
    const clique: Edge[] = [];
    nodes.forEach((a, i) =>
      nodes.slice(i + 1).forEach((b, j) =>
        clique.push({
          from: a.id,
          to: b.id,
          kind: { type: 'shared_topics', value: Array(((i + j) % 3) + 1).fill('t') },
        }),
      ),
    );
    const capped = pickEdges(clique, ids, 2);
    expect(capped.length).toBeLessThanOrEqual(2 * nodes.length);
    expect(capped.length).toBeLessThan(clique.length);
    for (const n of nodes) expect(capped.some((e) => e.from === n.id || e.to === n.id)).toBe(true);
    // Strongest first.
    expect(capped[0].topics.length).toBe(3);
  });

  it('nodeRadius grows with tasks and is capped', () => {
    expect(nodeRadius(plan('x', 0))).toBe(12);
    expect(nodeRadius(plan('x', 2))).toBe(18);
    expect(nodeRadius(plan('x', 100))).toBe(40);
  });

  it('fitBox pads the bounds; zoomAt keeps the point under the cursor', () => {
    const box = fitBox(
      [
        { id: 'a', plan: nodes[0], r: 10, x: 0, y: 0 },
        { id: 'b', plan: nodes[1], r: 10, x: 100, y: 50 },
      ],
      { x: 10, y: 10 },
    );
    expect(box).toEqual({ x: -20, y: -20, w: 140, h: 90 });
    // A minimum size keeps the box centred on the circles.
    expect(
      fitBox([{ id: 'a', plan: nodes[0], r: 10, x: 0, y: 0 }], { x: 0, y: 0 }, { w: 100, h: 40 }),
    ).toEqual({ x: -50, y: -20, w: 100, h: 40 });
    const z = zoomAt({ x: 0, y: 0, w: 100, h: 100 }, { x: 25, y: 75 }, 0.5);
    expect(z).toEqual({ x: 12.5, y: 37.5, w: 50, h: 50 });
  });
});
