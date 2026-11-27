import { describe, expect, it } from 'vitest';
import { drawerBack, drawerClose, drawerOpen, type DrawerItem } from './drawer';

const task: DrawerItem = { type: 'task', plan: 'P', task: 'TASK-001' };
const topic: DrawerItem = { type: 'topic', tag: 'tech/jwt' };

describe('drawer', () => {
  it('drawer_push_back_close', () => {
    let s = drawerOpen([], task);
    expect(s).toEqual([task]);
    s = drawerOpen(s, topic, { push: true });
    expect(s).toEqual([task, topic]);
    s = drawerBack(s);
    expect(s).toEqual([task]);
    s = drawerBack(s);
    expect(s).toEqual([]);
    expect(drawerClose()).toEqual([]);
  });

  it('open without push replaces the stack', () => {
    expect(drawerOpen([task, topic], { type: 'activity' })).toEqual([{ type: 'activity' }]);
  });
});
