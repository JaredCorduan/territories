import { buildBoard, showHighlights, hintText, themeFor } from './common.js';

// [rule, heading, description] per level; each level lists only its new moves.
const MOVES = {
  easy: [
    ['animal_shadow', 'Shadow', "An animal rules out every cell it could see: its row, its column, its territory, and the eight cells around it. The board crosses these out for you as soon as you place an animal."],
    ['last_spot', 'Last spot', 'When a row, column, or territory is down to one open cell, its animal goes there.'],
    ['claimed_line', 'Claimed line', "When all of a territory's open cells lie in one row (or column), that line's animal has to be this territory's animal. Cross out the rest of the line."],
    ['claimed_region', 'Claimed territory', "The reverse: when all of a row's (or column's) open cells lie inside one territory, that territory's animal has to be in this line. Cross out the rest of the territory."],
    ['l_corner', 'L-corner', 'When a territory is down to three open cells forming an L inside a 2×2 square, the fourth cell of the square touches all three. An animal there would leave the territory no room, so cross it out.'],
  ],
  medium: [
    ['squeeze', 'Squeeze', "If an animal on some cell would rule out every open cell of a row, column, or territory, that unit would have no room left, so the cell gets crossed out. Count everything the animal rules out: its row, column, territory, and the cells touching it. The L-corner is the simplest case."],
    ['region_band_2', 'Territory band (2)', "When two territories' open cells all lie inside the same two rows (or columns), those territories use up both rows' animals. Cross out everything else in those rows."],
    ['line_band_2', 'Line band (2)', "The reverse: when two rows' (or columns') open cells all lie inside the same two territories, those territories' animals are in these rows. Cross out the rest of those territories. Two rows fitting inside two columns works the same way."],
  ],
  hard: [
    ['region_band_3', 'Territory band (3+)', 'The territory band with three or more: three territories huddled inside three rows (or columns) claim them. Cross out everything else in those rows. Scan for clusters of territories sharing a band of the board.'],
    ['line_band_3', 'Line band (3+)', "The line band with three or more: three rows (or columns) whose open cells come from just three territories claim those territories' animals. Cross out the rest of those territories."],
  ],
  brutal: [
    ['block_packing', '2×2 packing', "A 2×2 square can never hold two animals, because they'd touch. So when a group of territories (or rows, or columns) has all its open cells inside the same number of 2×2 squares, such as three territories inside three squares, each square holds exactly one of their animals. Every other open cell inside the squares gets crossed out."],
    ['window_packing', '3×3 packing', "A 3×3 square holds at most two animals: three would need a row and a column each, and the one in the middle row would touch another. So when two territories (or any two rows, columns, or territories that share no open cell) have all their open cells inside one 3×3 square, the square is full. Cross out its other open cells."],
    ['mixed_packing', 'Mixed packing', "Bands and 2×2 packing are the same idea: a group of territories (or rows, or columns) needs one animal each, and their open cells fit inside the same number of places that hold at most one animal each. The places don't have to match. They can be any mix of rows, columns, territories, and 2×2 squares, such as one row plus one 2×2 square. Every one of those places is then full, so cross out their other open cells."],
    ['mixed_band', 'Mixed band', "The units being counted don't have to match either. A row and a territory that share no open cell need two different animals. If the open cells of both fit inside the same two columns, those columns are spoken for. Cross out everything else in them. The same goes for any mix of rows, columns, and territories that share no open cells."],
  ],
};

const NOTES = {
  easy: 'Easy puzzles need only these moves.',
  medium: 'Medium puzzles add these to the easy moves.',
  hard: 'Hard puzzles add these to the medium moves.',
  brutal: 'Brutal puzzles add these to the hard moves.',
};

// Moves whose example shows a chosen animal (on its first animal's territory).
const FEATURED = { animal_shadow: 'turtle', last_spot: 'dinosaur-rex' };

function themeOf(ex) {
  const i = afterMarks(ex).indexOf('A');
  const animal = FEATURED[ex.rule];
  const pin = animal && i >= 0 ? { region: ex.regions[Math.floor(i / ex.size)][i % ex.size], animal } : undefined;
  return themeFor(ex.rule, ex.size, pin);
}

function setMarks(cells, marks) {
  cells.forEach((el, i) => {
    el.classList.toggle('animal', marks[i] === 'A');
    el.classList.toggle('x', marks[i] === 'x');
  });
}

function afterMarks(ex) {
  const n = ex.size;
  const m = [...ex.marks];
  for (const [r, c] of ex.deduction.animals) m[r * n + c] = 'A';
  for (const [r, c] of ex.deduction.crosses) if (m[r * n + c] === '.') m[r * n + c] = 'x';
  return m.join('');
}

function figure(caption, ex, marks, highlight) {
  const fig = document.createElement('figure');
  const cap = document.createElement('figcaption');
  cap.textContent = caption;
  const board = document.createElement('div');
  board.className = 'board mini';
  const cells = buildBoard(board, ex.regions, themeOf(ex));
  setMarks(cells, marks);
  if (highlight) showHighlights(cells, ex.regions, { kind: 'move', ...ex.deduction });
  fig.append(cap, board);
  return fig;
}

// The tap cycle drawn with real tiles: blank → ✕ → animal → blank.
function drawCycle() {
  const { color, animal } = themeFor('cycle', 1, { region: 0, animal: 'lion' })[0];
  const root = document.getElementById('cycle');
  ['', 'x', 'animal', ''].forEach((state, i) => {
    if (i) root.append('→');
    const el = document.createElement('div');
    el.className = `cell ${state}`;
    el.style.background = color;
    el.style.setProperty('--icon', `url("animals/${animal}.svg")`);
    root.append(el);
  });
}

function render(level, examples) {
  document.querySelectorAll('.levels button').forEach((b) =>
    b.setAttribute('aria-selected', String(b.dataset.level === level)));
  document.getElementById('level-note').textContent = NOTES[level];
  const root = document.getElementById('moves');
  root.replaceChildren(...MOVES[level].map(([rule, title, text]) => {
    const sec = document.createElement('section');
    sec.className = 'move';
    const h = document.createElement('h3');
    h.textContent = title;
    const p = document.createElement('p');
    p.textContent = text;
    sec.append(h, p);
    const ex = examples.find((e) => e.rule === rule);
    if (ex) {
      const row = document.createElement('div');
      row.className = 'examples';
      row.append(figure('Before', ex, ex.marks, true), figure('After', ex, afterMarks(ex), false));
      const why = document.createElement('p');
      why.className = 'example-text';
      why.textContent = `Example: ${hintText({ kind: 'move', ...ex.deduction }, themeOf(ex))}`;
      sec.append(row, why);
    }
    return sec;
  }));
}

async function main() {
  drawCycle();
  const examples = await (await fetch('examples.json')).json();
  const levelFromHash = () => (location.hash.slice(1) in MOVES ? location.hash.slice(1) : 'easy');
  document.querySelectorAll('.levels button').forEach((b) =>
    b.addEventListener('click', () => { location.hash = b.dataset.level; }));
  window.addEventListener('hashchange', () => render(levelFromHash(), examples));
  render(levelFromHash(), examples);
}

main().catch((err) => {
  document.getElementById('moves').textContent = `Failed to load examples: ${err}`;
});
