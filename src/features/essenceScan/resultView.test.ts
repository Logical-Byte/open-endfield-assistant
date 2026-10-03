import { describe, expect, it } from 'vitest';
import { deriveSummary, filterItems } from './resultView';
import type { Decision, EssenceCatalog, ScannedItem } from './types';

const catalog: EssenceCatalog = {
  stats: [
    { id: 'strength', name: '力量提升', kind: 'attribute' },
    { id: 'attack', name: '攻击力提升', kind: 'secondary' },
    { id: 'skill', name: '技力恢复', kind: 'skill' },
  ],
  weapons: [{ id: 'sword', name: '白夜新星', rarity: 6, stats: ['strength', 'attack', 'skill'] }],
};

function item(sequence: number, decision: Decision): ScannedItem {
  return {
    sequence,
    page: 1,
    row: 1,
    column: sequence,
    essence: {
      stats: ['strength', 'attack', 'skill'],
      levels: [1, 1, 1],
      rarity: 'five',
      locked: false,
      abandoned: false,
    },
    evaluation: { decision, reason: 'weaponMatch', matchedWeaponIds: ['sword'] },
    image: null,
  };
}

describe('scan result view', () => {
  it('counts stored decisions without reevaluating recognition or weapon matches', () => {
    const items = [item(1, 'keep'), item(2, 'discard'), item(3, 'skip'), item(4, 'review')];
    items.push(item(5, 'keep'));
    items[4]!.essence.stats = [null, null, null];

    expect(deriveSummary(items)).toEqual({ total: 5, keep: 2, discard: 1, skip: 1, review: 1 });
    expect(deriveSummary([])).toEqual({ total: 0, keep: 0, discard: 0, skip: 0, review: 0 });
  });

  it('combines decision filtering with stat, weapon, and sequence searches', () => {
    const items = [item(1, 'keep'), item(2, 'discard'), item(30, 'review')];
    items[1]!.essence.stats = [null, null, null];
    items[1]!.evaluation.matchedWeaponIds = [];

    expect(filterItems(items, catalog, { decision: 'all', query: '  力量  ' })).toEqual([
      items[0],
      items[2],
    ]);
    expect(filterItems(items, catalog, { decision: 'review', query: '白夜' })).toEqual([items[2]]);
    expect(filterItems(items, catalog, { decision: 'all', query: '30' })).toEqual([items[2]]);
    expect(filterItems(items, catalog, { decision: 'discard', query: '白夜' })).toEqual([]);
    expect(filterItems(items, catalog, { decision: 'discard', query: '  ' })).toEqual([items[1]]);
    expect(deriveSummary(items).total).toBe(3);
  });

  it('can search recognized IDs while the catalog is unavailable', () => {
    const items = [item(7, 'keep')];
    expect(filterItems(items, null, { decision: 'all', query: 'STRENGTH' })).toEqual(items);
    expect(filterItems(items, null, { decision: 'all', query: 'SWORD' })).toEqual(items);
    expect(filterItems(items, null, { decision: 'all', query: 'unknown' })).toEqual([]);
  });
});
