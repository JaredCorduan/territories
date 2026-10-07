import init, { hint as wasmHint } from './pkg/territories_wasm.js';
import { buildBoard as buildCells, showHighlights, hintText, themeFor } from './common.js';

const LEVELS = ['easy', 'medium', 'hard', 'brutal'];
const STORE_KEY = 'territories:v1';

const $ = (id) => document.getElementById(id);

// ---------- persistence ----------

function loadStore() {
  try {
    const s = JSON.parse(localStorage.getItem(STORE_KEY));
    if (s && typeof s === 'object') return { progress: s.progress || {}, last: s.last || null };
  } catch { /* unavailable or corrupt */ }
  return { progress: {}, last: null };
}

function saveStore() {
  try { localStorage.setItem(STORE_KEY, JSON.stringify(store)); } catch { /* ignore */ }
}

// ---------- state ----------

let puzzles = [];        // all puzzles from puzzles.json
let store = loadStore();
let sel = { level: 'easy', size: 8, index: 0 };
let game = null;         // { p, n, theme, animal[], manualX[], history[], secs, solved }
let activeHint = null;   // last hint response, while shown

function list() {
  return puzzles.filter((p) => p.level === sel.level && p.size === sel.size);
}

const count = (level, size) => puzzles.filter((p) => p.level === level && p.size === size).length;

// With no puzzles for the level, show only the picker and a note.
function showEmpty(empty) {
  $('empty').hidden = !empty;
  for (const id of ['nav', 'board', 'controls']) $(id).hidden = empty;
  if (empty) $('hint-panel').hidden = $('win').hidden = true;
}

function openPuzzle() {
  // Prefer a size that has puzzles at this level.
  if (count(sel.level, sel.size) === 0) {
    const size = sizes().find((s) => count(sel.level, s) > 0);
    if (size) sel = { ...sel, size, index: 0 };
  }
  refreshSizes();
  document.querySelectorAll('.levels button').forEach((b) =>
    b.setAttribute('aria-selected', String(b.dataset.level === sel.level)));
  const ps = list();
  if (ps.length === 0) {
    game = null;
    clearHint();
    showEmpty(true);
    $('empty').textContent = `No ${sel.level} puzzles yet. They're rare; check back after the next puzzle build.`;
    return;
  }
  showEmpty(false);
  sel.index = Math.max(0, Math.min(sel.index, ps.length - 1));
  const p = ps[sel.index];
  const n = p.size;
  const saved = store.progress[p.id] || {};
  const animal = new Array(n * n).fill(false);
  const manualX = new Array(n * n).fill(false);
  for (const i of saved.animals || []) animal[i] = true;
  for (const i of saved.xs || []) manualX[i] = true;
  game = { p, n, theme: themeFor(p.id, n), animal, manualX, history: [], secs: saved.secs || 0, solved: !!saved.solved };
  store.last = { ...sel };
  saveStore();
  clearHint();
  buildBoard();
  render();
}

function persist() {
  const { p, animal, manualX, secs, solved } = game;
  const idx = (arr) => arr.flatMap((v, i) => (v ? [i] : []));
  store.progress[p.id] = { animals: idx(animal), xs: idx(manualX), secs, solved };
  saveStore();
}

// ---------- rules helpers ----------

const regionOf = (i) => game.p.regions[Math.floor(i / game.n)][i % game.n];

// Auto-X: the animal's whole shadow (every cell it attacks).
function autoX() {
  const { n, animal } = game;
  const out = new Array(n * n).fill(false);
  animal.forEach((q, i) => {
    if (!q) return;
    for (let j = 0; j < n * n; j++) if (attacks(i, j)) out[j] = true;
  });
  return out;
}

function attacks(a, b) {
  const n = game.n;
  const ar = Math.floor(a / n), ac = a % n, br = Math.floor(b / n), bc = b % n;
  return a !== b && (ar === br || ac === bc || regionOf(a) === regionOf(b)
    || (Math.abs(ar - br) <= 1 && Math.abs(ac - bc) <= 1));
}

// 'animal' | 'x' | 'empty' as the player sees it.
function shown(i, auto) {
  if (game.animal[i]) return 'animal';
  return game.manualX[i] || auto[i] ? 'x' : 'empty';
}

function isSolved() {
  const { n, animal, p } = game;
  return animal.filter(Boolean).length === n && p.solution.every((c, r) => animal[r * n + c]);
}

// ---------- editing ----------

function snapshot() {
  game.history.push({ animal: [...game.animal], manualX: [...game.manualX] });
  if (game.history.length > 500) game.history.shift();
}

function changed() {
  clearHint();
  const justSolved = !game.solved && isSolved();
  if (justSolved) game.solved = true;
  persist();
  render();
  if (justSolved) celebrate();
}

// Stamp the animals' tiles row by row, popping each animal.
function celebrate() {
  const board = $('board');
  const step = 0.08;
  cells.forEach((el, i) => el.style.setProperty('--delay', `${Math.floor(i / game.n) * step}s`));
  board.classList.remove('celebrate');
  void board.offsetWidth; // restart the animation if it was already applied
  board.classList.add('celebrate');
  clearTimeout(celebrate.timer);
  celebrate.timer = setTimeout(() => board.classList.remove('celebrate'), (game.n * step + 1) * 1000);
}

function cycle(i) {
  const s = shown(i, autoX());
  if (s === 'empty') game.manualX[i] = true;
  else if (s === 'x') { game.animal[i] = true; game.manualX[i] = false; }
  else game.animal[i] = false;
}

// ---------- board rendering ----------

let cells = [];

function buildBoard() {
  cells = buildCells($('board'), game.p.regions, game.theme);
}

function render() {
  const auto = autoX();
  const animals = game.animal.flatMap((q, i) => (q ? [i] : []));
  cells.forEach((el, i) => {
    const s = shown(i, auto);
    el.classList.toggle('animal', s === 'animal');
    el.classList.toggle('x', s === 'x');
    el.classList.toggle('auto', s === 'x' && !game.manualX[i]);
    el.classList.toggle('clash', s === 'animal' && animals.some((j) => attacks(i, j)));
  });
  renderHint();
  $('board').classList.toggle('won', game.solved);
  if (!game.solved) $('board').classList.remove('celebrate');

  const ps = list();
  const done = store.progress[game.p.id]?.solved;
  $('title').textContent = `#${sel.index + 1} of ${ps.length}${done ? ' ✓' : ''}`;
  $('prev').disabled = sel.index === 0;
  $('next').disabled = sel.index >= ps.length - 1;
  $('undo').disabled = game.history.length === 0;
  $('win').hidden = !game.solved;
  $('win-text').textContent = `Solved in ${fmt(game.secs)}!`;
  $('hint').disabled = game.solved;
  $('timer').textContent = fmt(game.secs);
}

const fmt = (s) => `${Math.floor(s / 60)}:${String(s % 60).padStart(2, '0')}`;

// ---------- pointer input: click cycles, drag paints ----------

let drag = null; // { start, mode: 'cross' | 'erase', moved, visited }

function cellAt(e) {
  const el = document.elementFromPoint(e.clientX, e.clientY);
  return el && el.classList.contains('cell') && el.parentElement === $('board') ? Number(el.dataset.i) : null;
}

function paint(i) {
  if (drag.visited.has(i)) return;
  drag.visited.add(i);
  if (drag.mode === 'erase') game.manualX[i] = false;
  else if (shown(i, drag.auto) === 'empty') game.manualX[i] = true;
}

function onDown(e) {
  if (game.solved || e.button > 0) return;
  const i = cellAt(e);
  if (i === null) return;
  e.preventDefault();
  $('board').setPointerCapture(e.pointerId);
  const mode = game.manualX[i] && !game.animal[i] ? 'erase' : 'cross';
  drag = { start: i, mode, moved: false, visited: new Set(), auto: autoX() };
  snapshot();
}

function onMove(e) {
  if (!drag) return;
  const i = cellAt(e);
  if (i === null || (i === drag.start && !drag.moved)) return;
  if (!drag.moved) { drag.moved = true; paint(drag.start); }
  paint(i);
  render();
}

function onUp() {
  if (!drag) return;
  if (!drag.moved) cycle(drag.start);
  drag = null;
  changed();
}

// ---------- hints ----------

function askHint() {
  const { n, p } = game;
  const auto = autoX();
  const marks = Array.from({ length: n * n }, (_, i) => ({ animal: 'A', x: 'x', empty: '.' })[shown(i, auto)]).join('');
  let h;
  try {
    h = JSON.parse(wasmHint(JSON.stringify({ regions: p.regions, marks, solution: p.solution, level: p.level })));
  } catch (err) {
    h = { kind: 'error', message: String(err) };
  }
  if (h.kind === 'solved') return;
  activeHint = h;
  render();
}

function clearHint() {
  activeHint = null;
}

function renderHint() {
  const h = activeHint;
  showHighlights(cells, game.p.regions, h);
  $('hint-panel').hidden = !h;
  if (!h) return;
  $('hint-text').textContent = hintText(h, game.theme);
  $('hint-apply').hidden = h.kind === 'error';
  $('hint-apply').textContent = h.kind === 'mistake' ? 'Fix' : 'Apply';
}

function applyHint() {
  const h = activeHint;
  if (!h) return;
  const n = game.n;
  const idx = ([r, c]) => r * n + c;
  snapshot();
  if (h.kind === 'mistake') {
    for (const c of h.cells) { game.animal[idx(c)] = false; game.manualX[idx(c)] = false; }
  } else if (h.kind === 'reveal') {
    game.animal[idx(h.cell)] = true;
  } else if (h.kind === 'move') {
    for (const c of h.animals) { game.animal[idx(c)] = true; game.manualX[idx(c)] = false; }
    const auto = autoX();
    for (const c of h.crosses) if (shown(idx(c), auto) === 'empty') game.manualX[idx(c)] = true;
  }
  changed();
}

// ---------- wiring ----------

const sizes = () => [...new Set(puzzles.map((p) => p.size))].sort((a, b) => a - b);

// Size options for the current level; sizes with no puzzles are disabled.
function refreshSizes() {
  $('size').replaceChildren(...sizes().map((s) => {
    const o = new Option(`${s}×${s}`, s);
    o.disabled = count(sel.level, s) === 0;
    return o;
  }));
  $('size').value = sel.size;
}

function wire() {
  const board = $('board');
  board.addEventListener('pointerdown', onDown);
  board.addEventListener('pointermove', onMove);
  board.addEventListener('pointerup', onUp);
  board.addEventListener('pointercancel', onUp);
  board.addEventListener('contextmenu', (e) => e.preventDefault());

  document.querySelectorAll('.levels button').forEach((b) => b.addEventListener('click', () => {
    sel = { ...sel, level: b.dataset.level, index: 0 };
    openPuzzle();
  }));
  $('size').addEventListener('change', (e) => { sel = { ...sel, size: Number(e.target.value), index: 0 }; openPuzzle(); });
  $('prev').addEventListener('click', () => { sel.index--; openPuzzle(); });
  $('next').addEventListener('click', () => { sel.index++; openPuzzle(); });
  $('win-next').addEventListener('click', () => {
    if (sel.index < list().length - 1) { sel.index++; openPuzzle(); }
  });
  $('undo').addEventListener('click', () => {
    if (!game) return;
    const prev = game.history.pop();
    if (!prev) return;
    Object.assign(game, prev);
    game.solved = isSolved();
    clearHint();
    persist();
    render();
  });
  $('clear').addEventListener('click', () => {
    if (!game) return;
    snapshot();
    game.animal.fill(false);
    game.manualX.fill(false);
    game.solved = false;
    game.secs = 0;
    changed();
  });
  $('hint').addEventListener('click', askHint);
  $('hint-apply').addEventListener('click', applyHint);
  $('hint-close').addEventListener('click', () => { clearHint(); render(); });

  setInterval(() => {
    if (!game || game.solved || document.hidden) return;
    game.secs++;
    $('timer').textContent = fmt(game.secs);
    if (game.secs % 5 === 0) persist();
  }, 1000);
}

async function main() {
  const [, res] = await Promise.all([init(), fetch('puzzles.json')]);
  puzzles = (await res.json()).puzzles;
  if (store.last && LEVELS.includes(store.last.level)) sel = { ...sel, ...store.last };
  if (!sizes().includes(sel.size)) sel.size = sizes()[0];
  wire();
  openPuzzle();
}

main().catch((err) => {
  document.querySelector('main').insertAdjacentHTML('beforeend',
    `<p style="color:var(--bad)">Failed to load: ${String(err)}. Serve this folder over HTTP (e.g. <code>python3 -m http.server -d web</code>).</p>`);
});
