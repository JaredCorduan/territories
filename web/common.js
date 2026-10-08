// Shared by the game (app.js) and the moves page (help.js).

// Territory (region) colors with their names (for hint text), and each color's pool of
// animals (icons from game-icons.net, CC BY 3.0); one is shown on that region's animal.
// Near-duplicate animals never share a board because the pools hold one of each.
// Boards with up to BASE_POOLS territories use only the first BASE_POOLS colors, so
// colors added after them never change how those boards look.
const BASE_POOLS = 10;
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
  ['#4f545c', 'black', 'mole badger tapir vulture bison hyena-head ostrich'],
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
  const pools = POOLS.slice(0, Math.max(n, BASE_POOLS));
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

// Whether a region color is dark enough to need light marks on it.
export function isDark(hex) {
  const [r, g, b] = [1, 3, 5].map((i) => parseInt(hex.slice(i, i + 2), 16));
  return 0.299 * r + 0.587 * g + 0.114 * b < 110;
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
      el.dataset.i = r * n + c;
      const { color, animal } = theme[reg];
      el.className = isDark(color) ? 'cell dark' : 'cell';
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
  // `filled` (two-animal crowded squeezes) and `others` (loose leftovers) list more
  // units that matter.
  for (const u of [...h.units, ...(h.filled || []), ...(h.others || [])]) {
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
// `stars` is the game: animals per row, column, and territory.
export function hintText(h, theme, stars = 1) {
  if (stars === 2 && h.kind === 'move') return twoAnimalText(h, theme);
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

// The two-animal game's moves. Its rule ids are its own, even where a name repeats.
const TWO_ANIMAL_NAMES = {
  animal_shadow: 'Shadow',
  full_unit: 'Full',
  last_spots: 'Last spots',
  claimed_line: 'Claimed line',
  claimed_region: 'Claimed territory',
  small_squeeze: 'Small squeeze',
  squeeze: 'Squeeze',
  crowded_squeeze: 'Crowded squeeze',
  region_band_2: 'Territory band',
  line_band_2: 'Line band',
  region_band_3: 'Territory band',
  line_band_3: 'Line band',
  leftover: 'Leftover',
  wide_leftover: 'Wide leftover',
  mixed_band: 'Mixed band',
  leftover_cap: 'Leftover cap',
  loose: 'Loose leftover',
};

function twoAnimalText(h, theme) {
  const us = h.units;
  const name = TWO_ANIMAL_NAMES[h.rule] || h.rule;
  const join = (list) => joinUnitsOf(list, theme);
  const animals = (k) => (k === 1 ? 'one animal' : `${k} animals`);
  // Units come as one kind (lines or territories) and then the other.
  const isLine = (u) => !('region' in u);
  const split = us.findIndex((u) => isLine(u) !== isLine(us[0]));
  let [first, rest] = split < 0 ? [us, []] : [us.slice(0, split), us.slice(split)];
  const cap = (text) => text[0].toUpperCase() + text.slice(1);
  const num = (k) => (k === 1 ? 'one' : k);
  const need = (list) => (list.length === 1 ? 'still needs' : 'still need');
  const it = h.crosses.length === 1;
  // A leftover's cells: `first` and `rest` are lines then the territories they touch (the
  // leftover is the rest of those territories), or territories then the lines they lie
  // within (the rest of those lines).
  const leftover = () => (!rest.length
    ? `${join(first)} ${need(first)} ${animals(h.count)}${first.length > 1 ? ' between them' : ''}, all in the outlined cells.`
    : isLine(first[0])
      ? `every open cell of ${join(first)} lies in ${join(rest)}. ${cap(join(rest))} ${need(rest)} ${animals(h.line_need + h.count)} and ${join(first)} ${first.length === 1 ? 'takes' : 'take'} ${num(h.line_need)} of them, so the rest of ${join(rest)} (outlined) holds exactly ${num(h.count)}.`
      : `${join(first)} ${first.length === 1 ? 'lies' : 'lie'} entirely inside ${join(rest)}. ${cap(join(rest))} ${need(rest)} ${animals(h.line_need)} and ${join(first)} ${first.length === 1 ? 'takes' : 'take'} ${num(h.line_need - h.count)} of them, so the rest of ${join(rest)} (outlined) holds exactly ${num(h.count)}.`);
  // What squeezing `unit` gives when the outlined cells hold at most `h.count`.
  const capped = (unit) => [
    `Picture each arrangement of the animals ${join([unit])} still needs, with at most ${num(h.count)} in the outlined cells.`,
    h.animals.length ? `Every arrangement uses ${h.animals.length === 1 ? 'one cell, so it gets an animal' : `${h.animals.length} cells, so they get animals`}.` : '',
    h.crosses.length ? `The dashed ✕ ${it ? 'cell is' : 'cells are'} ruled out in every arrangement, so cross ${it ? 'it' : 'them'} out.` : '',
  ];
  // What a squeeze concludes, for the cells it marks.
  const outcome = () => [
    h.animals.length ? `Every arrangement uses ${h.animals.length === 1 ? 'one cell, so it gets an animal' : `${h.animals.length} cells, so they get animals`}.` : '',
    h.crosses.length ? `The dashed ✕ ${h.crosses.length === 1 ? 'cell is' : 'cells are'} ruled out in every arrangement, so cross ${h.crosses.length === 1 ? 'it' : 'them'} out.` : '',
  ].filter(Boolean).join(' ');
  switch (h.rule) {
    case 'animal_shadow':
      return `${name}: animals never touch, so cross out the cells around ${h.focus.length === 1 ? 'this animal' : 'these animals'}.`;
    case 'full_unit':
      return `${name}: ${join(us)} already ${us.length === 1 ? 'has both its' : 'have both their'} animals. Cross out the rest.`;
    case 'last_spots':
      return `${name}: ${join(us)} still needs ${animals(h.count)} and has exactly that many open cells.`;
    case 'claimed_line':
    case 'claimed_region':
    case 'region_band_2':
    case 'line_band_2':
    case 'region_band_3':
    case 'line_band_3':
      return `${name}: every open cell of ${join(first)} lies in ${join(rest)}. ${first.length === 1 ? 'It still needs' : 'They still need'} ${animals(h.count)}, which is all ${join(rest)} ${rest.length === 1 ? 'has' : 'have'} left. Cross out everything else in ${join(rest)}.`;
    case 'small_squeeze':
    case 'squeeze':
      return `${name}: ${join(us)} still needs ${animals(h.count)} in its outlined cells, with none touching. ${outcome()}`;
    case 'crowded_squeeze': {
      // units: the squeezed unit, then the units too short of room for some arrangements.
      const short = us.slice(1);
      const every = short.length ? 'every arrangement left' : 'every arrangement';
      const it = h.crosses.length === 1;
      // The lines those animals fill, and how many of them it takes.
      const lines = h.filled.map((u) => unitNameOf(u, theme)).join(' or ');
      const path = h.filling === 2 ? `two animals in ${lines}`
        : h.filling === 1 ? `an animal that fills ${lines}` : `animals that fill ${lines}`;
      return [
        `${name}: ${join(us.slice(0, 1))} still needs ${animals(h.count)} in its outlined cells, with none touching.`,
        short.length ? `${join(short)} ${short.length === 1 ? 'has' : 'each have'} room for only one more, which rules some arrangements out.` : '',
        h.animals.length ? `${every[0].toUpperCase()}${every.slice(1)} uses ${h.animals.length === 1 ? 'one cell, so it gets an animal' : `${h.animals.length} cells, so they get animals`}.` : '',
        h.filled.length
          ? `In ${every}, the dashed ✕ ${it ? 'cell' : 'cells'} either ${it ? 'touches' : 'touch'} one of those animals, or ${it ? 'is' : 'are'} in the path of ${path}. Either way, cross ${it ? 'it' : 'them'} out.`
          : h.crosses.length ? `The dashed ✕ ${it ? 'cell is' : 'cells are'} ruled out in ${every}, so cross ${it ? 'it' : 'them'} out.` : '',
      ].filter(Boolean).join(' ');
    }
    case 'leftover':
    case 'wide_leftover': {
      return [
        `${name}: ${leftover()}`,
        h.count === 1
          ? 'Picture each place that animal can go in the outlined cells, with no row, column, or territory taking more than it has room for.'
          : `Picture each arrangement of those ${h.count} animals in the outlined cells, with none touching, and no row, column, or territory taking more than it has room for.`,
        h.animals.length ? `Every arrangement uses ${h.animals.length === 1 ? 'one cell, so it gets an animal' : `${h.animals.length} cells, so they get animals`}.` : '',
        h.crosses.length ? `The dashed ✕ ${it ? 'cell is' : 'cells are'} ruled out ${h.count === 1 ? 'wherever it goes' : 'in every arrangement'}, so cross ${it ? 'it' : 'them'} out.` : '',
      ].filter(Boolean).join(' ');
    }
    case 'mixed_band': {
      // units: the `inner` units that fit, then the units they fit within.
      const [fits, within] = [us.slice(0, h.inner), us.slice(h.inner)];
      const crossing = h.crosses.some(([r, c]) => h.focus.some(([fr, fc]) => fr === r && fc === c));
      return [
        `${name}: every open cell of ${join(fits)} lies in ${join(within)}${fits.length > 1 ? `, and ${join(fits)} share no open cell` : ''}.`,
        `${fits.length === 1 ? 'It still needs' : 'They still need'} ${animals(h.count)}${fits.length > 1 ? ' between them' : ''}, which is all ${join(within)} ${within.length === 1 ? 'has' : 'have'} left.`,
        `Cross out everything else in ${join(within)}.`,
        crossing ? `An animal where ${within.length === 2 ? '' : 'two of '}${join(within)} cross would use up room in both, so cross that out too.` : '',
      ].filter(Boolean).join(' ');
    }
    case 'leftover_cap': {
      // units: a leftover's units (`inner` of them), then the unit squeezed.
      const mine = us.slice(0, h.inner);
      const cut = mine.findIndex((u) => isLine(u) !== isLine(mine[0]));
      [first, rest] = cut < 0 ? [mine, []] : [mine.slice(0, cut), mine.slice(cut)];
      return [`${name}: ${leftover()}`, ...capped(us[h.inner])].filter(Boolean).join(' ');
    }
    case 'loose': {
      // units: `inner` lines, the territory that must take some of their animals, and the
      // unit squeezed. filling: the least that territory takes. count: the most its
      // outlined rest then holds. others: the other territories crossing the lines.
      const lines = us.slice(0, h.inner);
      const taker = [us[h.inner]];
      const others = h.line_need - h.filling;
      const named = h.others || [];
      const several = named.length > 1;
      return [
        `${name}: ${join(lines)} ${need(lines)} ${animals(h.line_need)}.`,
        others ? `${named.length ? cap(join(named)) : 'The other territories there'} can fit at most ${animals(others)} in ${join(lines)}${several ? ' between them' : ''}, so ${join(taker)} must have at least ${num(h.filling)} there.`
          : `Only ${join(taker)} has open cells there, so it takes ${h.filling === 1 ? 'it' : 'them all'}.`,
        h.count ? `${cap(join(taker))} still needs ${animals(h.filling + h.count)}, so the rest of it (outlined) holds at most ${num(h.count)}.`
          : `That is every animal ${join(taker)} still needs, so cross out the rest of it.`,
        ...(h.count ? capped(us[h.inner + 1]) : []),
      ].filter(Boolean).join(' ');
    }
    default:
      return name;
  }
}
