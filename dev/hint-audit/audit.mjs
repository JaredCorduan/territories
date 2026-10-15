// Ranks the two-animal hints that fall back on "picture each arrangement" by how many
// arrangements that asks the player to picture. Plays every `STEP`th stored puzzle
// through the hint engine and writes the worst of each rule to worst.json, which
// index.html shows. Run from anywhere: node dev/hint-audit/audit.mjs
import { readFileSync, writeFileSync } from 'fs';

const web = new URL('../../web/', import.meta.url);
const { default: init, hint } = await import(new URL('pkg/territories_wasm.js', web));
await init({ module_or_path: readFileSync(new URL('pkg/territories_wasm_bg.wasm', web)) });
globalThis.localStorage = { getItem: () => null, setItem() {} };
const { hintText, themeFor } = await import(new URL('common.js', web));

const STEP = 5;
const PER_RULE = 4;
// The sentences that leave the working to the player.
const FALLBACK = /ruled out (in )?every|ruled out wherever|ruled out however|no way to (hold|place)|Picture each|picked every way|part of every arrangement|used however/;
// Squeezes that count only touching, not other units' room.
const ROOMY = new Set(['small_squeeze', 'squeeze']);

// How many ways `k` animals fit in `cells`: none touching, no unit over its room (if
// `room` is given), and at most `most` of them in `limited`.
function ways(cells, k, n, regions, room, limited = [], most = Infinity) {
  const key = ([r, c]) => r * n + c;
  const inLimited = new Set(limited.map(key));
  const go = (from, picked, used, inside) => {
    if (picked.length === k) return 1;
    let total = 0;
    for (let i = from; i < cells.length; i++) {
      const [r, c] = cells[i];
      if (picked.some(([pr, pc]) => Math.abs(pr - r) <= 1 && Math.abs(pc - c) <= 1)) continue;
      const units = [r, n + c, 2 * n + regions[r][c]];
      if (room && units.some((u) => (used[u] || 0) >= room[u])) continue;
      const add = inLimited.has(key(cells[i])) ? 1 : 0;
      if (inside + add > most) continue;
      const next = { ...used };
      units.forEach((u) => { next[u] = (next[u] || 0) + 1; });
      total += go(i + 1, [...picked, cells[i]], next, inside + add);
    }
    return total;
  };
  return go(0, [], {}, 0);
}

const unitIndex = (u, n) => ('row' in u ? u.row : 'col' in u ? n + u.col : 2 * n + u.region);

const puzzles = JSON.parse(readFileSync(new URL('puzzles-2.json', web))).puzzles;
const byRule = {};
// Each puzzle's level and the most arrangements any of its hints asks for.
const perPuzzle = [];
let played = 0;
for (let i = 0; i < puzzles.length; i += STEP) {
  const p = puzzles[i], n = p.size;
  const m = Array(n * n).fill('.');
  played++;
  let most = 0;
  for (let step = 0; step < 500; step++) {
    const marks = m.join('');
    const h = JSON.parse(hint(JSON.stringify({ stars: 2, regions: p.regions, marks, solution: p.solution, level: p.level })));
    if (h.kind !== 'move') break;
    const text = hintText(h, themeFor(p.id, n, null, {}), 2, p.regions);
    const s = (byRule[h.rule] ||= { hints: 0, counts: [], worst: [] });
    s.hints++;
    if (FALLBACK.test(text)) {
      const room = Array(3 * n).fill(2);
      m.forEach((ch, j) => {
        if (ch !== 'A') return;
        const r = Math.floor(j / n), c = j % n;
        [r, n + c, 2 * n + p.regions[r][c]].forEach((u) => { room[u]--; });
      });
      // A capped squeeze pictures the squeezed unit; the rest picture the outlined cells.
      const capped = h.rule === 'leftover_cap' || h.rule === 'loose';
      const unit = capped && h.units[h.rule === 'loose' ? h.inner + 1 : h.inner];
      const arrangements = unit && h.squeezed.length
        ? ways(h.squeezed, room[unitIndex(unit, n)], n, p.regions, room, h.focus, h.count)
        : ways(h.focus, h.count, n, p.regions, ROOMY.has(h.rule) ? null : room);
      s.counts.push(arrangements);
      most = Math.max(most, arrangements);
      s.worst.push({ id: p.id, level: p.level, step, rule: h.rule, arrangements, regions: p.regions, marks, deduction: h });
      s.worst.sort((a, b) => b.arrangements - a.arrangements);
      // One per puzzle, so the list shows different boards.
      s.worst = s.worst.filter((w, j) => s.worst.findIndex((x) => x.id === w.id) === j).slice(0, PER_RULE);
    }
    h.animals.forEach(([r, c]) => { m[r * n + c] = 'A'; });
    h.crosses.forEach(([r, c]) => { m[r * n + c] = 'x'; });
  }
  perPuzzle.push({ level: p.level, most });
}

const at = (sorted, q) => sorted[Math.min(sorted.length - 1, Math.floor(q * sorted.length))];
const rules = Object.entries(byRule).filter(([, s]) => s.counts.length).map(([rule, s]) => {
  const sorted = [...s.counts].sort((a, b) => a - b);
  return { rule, hints: s.hints, fallback: sorted.length, median: at(sorted, 0.5), p90: at(sorted, 0.9), max: sorted[sorted.length - 1], over10: sorted.filter((x) => x > 10).length };
}).sort((a, b) => b.over10 - a.over10);
console.log(`${played} puzzles`);
console.table(rules);
// How many puzzles have a hint asking for more than each limit.
const LIMITS = [10, 20, 50, 100, 300];
console.table(['easy', 'medium', 'hard', 'brutal'].map((level) => {
  const mine = perPuzzle.filter((x) => x.level === level);
  const over = Object.fromEntries(LIMITS.map((t) => [`over${t}`, `${Math.round(100 * mine.filter((x) => x.most > t).length / mine.length)}%`]));
  return { level, puzzles: mine.length, ...over };
}));
const worst = Object.values(byRule).flatMap((s) => s.worst).sort((a, b) => b.arrangements - a.arrangements);
writeFileSync(new URL('worst.json', import.meta.url), JSON.stringify({ played, rules, worst }));
console.log(`wrote ${worst.length} hints to worst.json`);
