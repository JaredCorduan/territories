// Shared by the game (app.js) and the moves page (help.js).

// Territory (region) colors with their names (for hint text), and each color's pool of
// animals (icons from game-icons.net, CC BY 3.0); one is shown on that region's animal.
// Near-duplicate animals never share a board because the pools hold one of each.
export const POOLS = [
  ['#d8604f', 'red', 'fox-head sad-crab scorpion rooster ladybug ant piranha'],
  ['#f0a04b', 'orange', 'tiger-head clownfish squirrel sea-star monkey kangaroo feline'],
  ['#efd66f', 'yellow', 'lion bee duck camel labrador-head toucan gold-scarab'],
  ['#a8c468', 'green', 'frog gecko snake chameleon-glyph dinosaur-rex hummingbird sloth'],
  ['#4fae9f', 'teal', 'turtle manta-ray dragonfly angler-fish swallow seahorse beaver'],
  ['#86b8ea', 'blue', 'dolphin sperm-whale penguin shark-fin heron juggling-seal butterfly'],
  ['#9670c0', 'purple', 'octopus giant-squid jellyfish bat hanging-spider raven leeching-worm'],
  ['#f3a9cf', 'pink', 'pig flamingo axolotl rabbit shrimp snail mouse'],
  ['#e6e2d8', 'gray', 'wolf-head elephant rhinoceros-horn sheep swan panda koala'],
  ['#94704e', 'brown', 'bear-head horse-head gorilla eagle-head barn-owl stag-head hedgehog'],
].map(([color, name, animals]) => ({ color, name, animals: animals.split(' ') }));

// A repeatable random source for `seed` (FNV-1a into mulberry32).
function rng(seed) {
  let h = 2166136261;
  for (const ch of String(seed)) h = Math.imul(h ^ ch.charCodeAt(0), 16777619);
  return () => {
    h = (h + 0x6d2b79f5) | 0;
    let t = Math.imul(h ^ (h >>> 15), h | 1);
    t ^= t + Math.imul(t ^ (t >>> 7), t | 61);
    return ((t ^ (t >>> 14)) >>> 0) / 4294967296;
  };
}

// The look of a board with `n` regions: per region, a { color, name, animal }.
// The same seed (a puzzle's stable id) always gives the same colors and animals.
// `pin`, a { region, animal }, forces that animal (in its own color) onto a region.
// `favorites` (from loadFavorites) replaces the random animal of each color it names.
export function themeFor(seed, n, pin, favorites = {}) {
  const rand = rng(seed);
  const pools = [...POOLS];
  for (let i = pools.length - 1; i > 0; i--) {
    const j = Math.floor(rand() * (i + 1));
    [pools[i], pools[j]] = [pools[j], pools[i]];
  }
  const picked = pools.map(({ color, name, animals }) => {
    const drawn = animals[Math.floor(rand() * animals.length)];
    return { color, name, animal: animals.includes(favorites[name]) ? favorites[name] : drawn };
  });
  if (pin) {
    const k = pools.findIndex((p) => p.animals.includes(pin.animal));
    picked[k].animal = pin.animal;
    [picked[k], picked[pin.region]] = [picked[pin.region], picked[k]];
  }
  return Array.from({ length: n }, (_, r) => picked[r % picked.length]);
}

const FAVORITES_KEY = 'territories:favorites:v1';

// The player's chosen animal per color name; colors left random are absent.
export function loadFavorites() {
  try {
    const f = JSON.parse(localStorage.getItem(FAVORITES_KEY));
    if (f && typeof f === 'object') return f;
  } catch { /* unavailable or corrupt */ }
  return {};
}

export function saveFavorites(favorites) {
  try { localStorage.setItem(FAVORITES_KEY, JSON.stringify(favorites)); } catch { /* ignore */ }
}

// A deep, saturated shade of a region color, for solved tiles.
export function deepen(hex) {
  const [r, g, b] = [1, 3, 5].map((i) => parseInt(hex.slice(i, i + 2), 16) / 255);
  const max = Math.max(r, g, b), min = Math.min(r, g, b);
  const d = max - min;
  let h = 0;
  if (d) {
    if (max === r) h = ((g - b) / d) % 6;
    else if (max === g) h = (b - r) / d + 2;
    else h = (r - g) / d + 4;
  }
  const s = d ? d / (1 - Math.abs(max + min - 1)) : 0;
  // Near-neutrals stay gray; already-dark colors drop further so the stamp still reads.
  const sat = d < 0.1 ? 0.06 : Math.min(1, s * 0.9 + 0.15);
  const light = Math.min(40, (max + min) * 50 - 16);
  return `hsl(${Math.round(h * 60 + 360) % 360} ${Math.round(sat * 100)}% ${Math.round(light)}%)`;
}

// Fills `board` with colored, walled cells for `regions`, styled by `theme`
// (from themeFor); returns the cells.
export function buildBoard(board, regions, theme) {
  const n = regions.length;
  board.style.setProperty('--n', n);
  board.replaceChildren();
  const cells = [];
  for (let r = 0; r < n; r++) {
    for (let c = 0; c < n; c++) {
      const el = document.createElement('div');
      const reg = regions[r][c];
      el.className = 'cell';
      el.dataset.i = r * n + c;
      const { color, animal } = theme[reg];
      el.style.background = color;
      el.style.setProperty('--icon', `url("animals/${animal}.svg")`);
      el.style.setProperty('--deep', deepen(color));
      const wall = (dr, dc) => {
        const rr = r + dr, cc = c + dc;
        return rr >= 0 && rr < n && cc >= 0 && cc < n && regions[rr][cc] !== reg;
      };
      el.style.boxShadow = [
        wall(-1, 0) ? 'inset 0 2px 0 var(--wall)' : 'inset 0 0.5px 0 rgb(0 0 0 / 0.25)',
        wall(1, 0) ? 'inset 0 -2px 0 var(--wall)' : 'inset 0 -0.5px 0 rgb(0 0 0 / 0.25)',
        wall(0, -1) ? 'inset 2px 0 0 var(--wall)' : 'inset 0.5px 0 0 rgb(0 0 0 / 0.25)',
        wall(0, 1) ? 'inset -2px 0 0 var(--wall)' : 'inset -0.5px 0 0 rgb(0 0 0 / 0.25)',
      ].join(', ');
      board.append(el);
      cells.push(el);
    }
  }
  return cells;
}

// Shows a hint (or example deduction) on the cells: striped units, solid
// focus outline, dashed outline on cells that change, red on mistakes.
export function showHighlights(cells, regions, h) {
  for (const el of cells) el.classList.remove('hl-unit', 'hl-focus', 'hl-target', 'hl-bad');
  const board = cells[0]?.parentElement;
  board?.querySelectorAll('.hl-block').forEach((el) => el.remove());
  if (!h) return;
  const n = regions.length;
  const at = ([r, c]) => cells[r * n + c];
  if (h.kind === 'mistake') h.cells.forEach((c) => at(c).classList.add('hl-bad'));
  if (h.kind === 'reveal') at(h.cell).classList.add('hl-target');
  if (h.kind !== 'move') return;
  for (const u of h.units) {
    cells.forEach((el, i) => {
      const r = Math.floor(i / n), c = i % n;
      if (('row' in u && u.row === r) || ('col' in u && u.col === c)
        || ('region' in u && u.region === regions[r][c])) el.classList.add('hl-unit');
    });
  }
  const side = SQUARE_SIDE[h.rule];
  if (side) {
    // focus lists each square's cells, top-left first: outline whole squares.
    for (let i = 0; i < h.focus.length; i += side * side) {
      const [r, c] = h.focus[i];
      const box = document.createElement('div');
      box.className = 'hl-block';
      box.style.gridArea = `${r + 1} / ${c + 1} / span ${side} / span ${side}`;
      board.append(box);
    }
  } else {
    h.focus.forEach((c) => at(c).classList.add('hl-focus'));
  }
  [...h.animals, ...h.crosses].forEach((c) => at(c).classList.add('hl-target'));
}

// Rules whose focus is a list of squares (2×2 blocks or a 3×3 window), by side.
const SQUARE_SIDE = { block_packing: 2, mixed_packing: 2, window_packing: 3 };

function unitNameOf(u, theme) {
  if ('row' in u) return `row ${u.row + 1}`;
  if ('col' in u) return `column ${u.col + 1}`;
  return `the ${theme[u.region].name} territory`;
}

const joinUnitsOf = (us, theme) => us.map((u) => unitNameOf(u, theme)).join(us.length === 2 ? ' and ' : ', ').replace(/, ([^,]*)$/, ', and $1');

const RULE_NAMES = {
  animal_shadow: 'Shadow',
  last_spot: 'Last spot',
  claimed_line: 'Claimed line',
  claimed_region: 'Claimed territory',
  l_corner: 'L-corner',
  squeeze: 'Squeeze',
  region_band_2: 'Territory band',
  line_band_2: 'Line band',
  region_band_3: 'Territory band',
  line_band_3: 'Line band',
  block_packing: '2×2 packing',
  mixed_packing: 'Mixed packing',
  mixed_band: 'Mixed band',
  window_packing: '3×3 packing',
};

// Explains a hint in the context of the specific board (`theme` names its territories).
export function hintText(h, theme) {
  switch (h.kind) {
    case 'mistake':
      return `Something's off: ${h.cells.length === 1 ? 'the highlighted mark is' : 'the highlighted marks are'} wrong.`;
    case 'reveal':
      return 'No clean move found here; revealing an animal.';
    case 'error':
      return `Hint engine error: ${h.message}`;
    case 'move': break;
    default: return '';
  }
  const us = h.units;
  const unitName = (u) => unitNameOf(u, theme);
  const joinUnits = (list) => joinUnitsOf(list, theme);
  const half = us.length / 2;
  const name = RULE_NAMES[h.rule] || h.rule;
  switch (h.rule) {
    case 'animal_shadow':
      return `${name}: this animal rules out every cell it can see: its row, column, territory, and neighbors.`;
    case 'last_spot':
      return `${name}: ${unitName(us[0])} has only one open cell left, so its animal goes there.`;
    case 'claimed_line':
      return `${name}: every open cell of ${unitName(us[0])} lies in ${unitName(us[1])}, so ${unitName(us[1])}'s animal belongs to it. Cross out the rest of ${unitName(us[1])}.`;
    case 'claimed_region':
      return `${name}: every open cell of ${unitName(us[0])} lies in ${unitName(us[1])}, so ${unitName(us[1])}'s animal is there. Cross out the rest of ${unitName(us[1])}.`;
    case 'l_corner':
      return `${name}: ${unitName(us[0])}'s open cells form an L. The cell completing the 2×2 touches all three, so it can't hold an animal.`;
    case 'squeeze':
      return `${name}: an animal on the outlined cell would rule out every open cell of ${unitName(us[0])}, so it can't hold an animal.`;
    case 'region_band_2':
    case 'line_band_2':
    case 'region_band_3':
    case 'line_band_3':
      return `${name}: ${joinUnits(us.slice(0, half))} fit entirely within ${joinUnits(us.slice(half))}, so those ${half} animals are spoken for. Cross out everything else in ${joinUnits(us.slice(half))}.`;
    case 'block_packing':
      return `${name}: ${joinUnits(us)} need ${us.length} animals, and their open cells fit inside the ${us.length} outlined 2×2 blocks. A 2×2 block holds at most one animal, so the rest of those blocks is empty.`;
    case 'mixed_packing': {
      // units lists the k chosen units, then the covering lines and territories;
      // each covering 2×2 block adds four focus cells.
      const blocks = h.focus.length / 4;
      const k = (us.length + blocks) / 2;
      const cover = us.slice(k);
      const places = [
        cover.length ? joinUnits(cover) : '',
        blocks ? `the ${blocks === 1 ? 'outlined 2×2 block' : `${blocks} outlined 2×2 blocks`}` : '',
      ].filter(Boolean).join(' plus ');
      return `${name}: ${joinUnits(us.slice(0, k))} need ${k} animals, and their open cells fit inside ${places}. Each of those holds at most one animal, so all ${k} are full. Cross out their other open cells.`;
    }
    case 'mixed_band':
      return `${name}: ${joinUnits(us.slice(0, half))} share no open cell, so they need ${half} different animals. Their open cells fit entirely within ${joinUnits(us.slice(half))}, so those are spoken for. Cross out everything else in ${joinUnits(us.slice(half))}.`;
    case 'window_packing':
      return `${name}: ${joinUnits(us)} share no open cell, so they need two animals, and their open cells fit inside the outlined 3×3 square. A 3×3 square holds at most two animals, so the rest of it is empty.`;
    default:
      return name;
  }
}
