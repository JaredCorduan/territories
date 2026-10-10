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

// The two-animal game's moves. Its rule ids are its own, even where a name repeats.
const TWO_ANIMAL_MOVES = {
  easy: [
    ['animal_shadow', 'Shadow', 'Animals never touch, so an animal rules out the eight cells around it. The board crosses these out for you as soon as you place an animal.'],
    ['full_unit', 'Full', 'Once a row, column, or territory has both its animals, the rest of it is ruled out. The board crosses these out for you too.'],
    ['last_spots', 'Last spots', 'When a row, column, or territory has exactly as many open cells as animals it still needs, they all get animals.'],
    ['claimed_line', 'Claimed line', "When all of a territory's open cells lie in one row (or column), and the territory still needs as many animals as that line does, the line's animals are all the territory's. Cross out the rest of the line.\nUsually both need two. It also works when both need one, or when two territories that need one each share a line that needs two."],
    ['claimed_region', 'Claimed territory', "The reverse: when all of a row's (or column's) open cells lie inside one territory, and the line still needs as many animals as the territory does, the territory's animals are all in that line. Cross out the rest of the territory."],
    ['small_squeeze', 'Small squeeze', "When a row, column, or territory is down to four open cells or fewer, there are only a few arrangements left for its animals, since they can't touch. Picture each arrangement and look for what they all have in common.\nIf a cell holds an animal in every arrangement, put an animal there. If a cell holds an animal in none of them, cross it out. And if a cell outside the unit touches an animal in every arrangement, cross it out too.\nFor example, when three open cells in a line need two animals, the only arrangement is the two ends. They get animals and the middle is crossed out.\nOr when two touching open cells need one animal, it goes in one or the other, so every cell that touches both is crossed out.\nA shortcut for any squeeze: cells that all touch hold one animal at most. When the open cells fall into as many such groups as there are animals to place, each group gets exactly one."],
  ],
  medium: [
    ['squeeze', 'Squeeze', 'The small squeeze with any number of open cells. Long thin shapes and shapes that fall into two clumps are the ones to look for: four open cells in a line rule out the cells on both sides of all four.'],
    ['crowded_squeeze', 'Crowded squeeze', "A squeeze that also remembers a row, column, or territory holds only two animals. Picture each arrangement of the unit's animals as before, but now an arrangement can also fill up a line.\nIf a cell is ruled out in every arrangement, either because it touches an animal or because its row or column is already full, cross it out."],
    ['region_band_2', 'Territory band (2)', "When territories have all their open cells inside the same two rows (or columns), and together still need as many animals as those two rows do, they use up both rows' animals. Cross out everything else in those rows.\nWith nothing placed yet, that is two territories inside two rows: four animals either way."],
    ['line_band_2', 'Line band (2)', "The reverse: when two rows' (or columns') open cells all lie inside territories that together still need as many animals as the two rows do, those territories' animals are all in these rows. Cross out the rest of those territories."],
  ],
  hard: [
    ['region_band_3', 'Territory band (3+)', 'The territory band with three or more rows (or columns). Scan for clusters of territories sharing a band of the board.'],
    ['line_band_3', 'Line band (3+)', 'The line band with three or more rows (or columns).'],
    ['leftover', 'Leftover', "When a row's (or column's) open cells all lie inside a few territories, the row takes its share of those territories' animals, and the rest of those territories holds exactly what is left over.\nWith two territories, the row takes two of their four animals, so their cells outside the row hold exactly two. Treat those leftover cells as a unit that needs two and squeeze it: picture each arrangement and look for what they all have in common."],
  ],
  brutal: [
    ['wide_leftover', 'Wide leftover', "The leftover, counting two or more neighboring rows (or columns) together.\nIt works in two directions. If every open cell of those rows lies in a few territories, the rows take their share and the rest of those territories holds what is left over. Or if some territories lie entirely inside those rows, they take their share and the rest of the rows holds what is left over.\nCount only the animals still to place: a row that already has one needs just one more. Then squeeze the leftover cells as a unit of their own."],
    ['mixed_band', 'Mixed band', "A band whose units are a mix of kinds. A row and a territory that share no open cell need their animals separately. If all their open cells lie inside two columns, they need everything those columns have left, so cross out the rest of the columns.\nThe units they fit inside can be a mix too, such as a row and a column. Then the cell where the two cross is also crossed out: an animal there would use up room in both."],
    ['leftover_cap', 'Leftover cap', "A leftover tells you exactly how many animals its cells hold. That number is also a limit for any row, column or territory that overlaps those cells: it can't place more of its own animals there than the leftover holds.\nSay the leftover cells hold exactly one animal, and a territory that needs two has some of its open cells among them. At most one of the territory's animals is in the leftover, so at least one goes in its other open cells.\nIf those other cells all touch, they hold exactly one. The territory's second animal is then in the leftover, and it is the only animal the leftover holds, so cross out the leftover cells outside the territory.\nEither way, cross out any cell where an animal would leave the territory no way to place both: one that touches all of its other open cells, for instance, since that would push both of its animals into the leftover."],
    ['loose', 'Loose leftover', "A leftover with a limit instead of an exact count.\nEvery animal in a row belongs to one of the territories crossing it, and a territory can supply only as many as fit in its part of the row. Pick one territory and add up the most that all the others could supply. If that falls short of what the row needs, the territory you picked must make up the difference inside the row. Whatever it places there it can't place elsewhere, so the rest of that territory holds at most what it has left.\nIf that is none, cross out the rest of the territory. Otherwise use the limit as in Leftover cap: squeeze another unit, knowing how few it can take from those cells. It works with two or three neighboring rows (or columns) as well."],
  ],
};

// What differs between the two games, by animals per row, column, and territory.
const GAMES = {
  1: {
    rules: 'Each color is a territory. Place one animal in every row, every column, and every territory. Animals never touch, not even diagonally.',
    auto: "Placing an animal auto-crosses everything it rules out: its row, column, territory, and neighbors. The automatic ✕'s disappear when you remove the animal.",
    moves: MOVES,
    examples: 'examples.json',
    legend: 'In examples: striped cells are the units involved, a solid outline marks the key cells, and dashed outlines are the cells the move changes, with a faint ✕ or animal for what goes there.',
  },
  2: {
    rules: 'Each color is a territory. Place two animals in every row, every column, and every territory. Animals never touch, not even diagonally.',
    auto: "Placing an animal auto-crosses the cells touching it, and the second animal in a row, column, or territory auto-crosses the rest of it. The automatic ✕'s disappear when you remove the animal.",
    moves: TWO_ANIMAL_MOVES,
    examples: 'examples-2.json',
    legend: "In examples: striped cells are the units involved, a solid outline marks the key cells, and dashed outlines are the cells the move changes, with a faint ✕ or animal for what goes there. Key cells sharing one box are a group of touching cells, which gets exactly one animal. A dotted box marks a unit's open cells outside the key cells, where at least one of its animals must go.",
  },
};

const NOTES = {
  easy: 'Easy puzzles need only these moves.',
  medium: 'Medium puzzles add these to the easy moves.',
  hard: 'Hard puzzles add these to the medium moves.',
  brutal: 'Brutal puzzles add these to the hard moves.',
};

// Moves whose example shows a chosen animal (on its first animal's territory).
const FEATURED = {
  1: { animal_shadow: 'turtle', last_spot: 'dinosaur-rex' },
  2: {
    animal_shadow: 'manta-ray',
    full_unit: 'beaver',
    last_spots: 'feline',
    small_squeeze: 'octopus',
    wide_leftover: 'squirrel',
    leftover_cap: 'duck',
    loose: 'dolphin',
  },
};

function themeOf(ex) {
  const i = afterMarks(ex).indexOf('A');
  const animal = FEATURED[ex.stars][ex.rule];
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

function render(stars, level, examples) {
  document.querySelectorAll('.games button').forEach((b) =>
    b.setAttribute('aria-selected', String(Number(b.dataset.stars) === stars)));
  document.querySelectorAll('.levels button').forEach((b) =>
    b.setAttribute('aria-selected', String(b.dataset.level === level)));
  document.getElementById('rules').textContent = GAMES[stars].rules;
  document.getElementById('auto').textContent = GAMES[stars].auto;
  document.getElementById('level-note').textContent = NOTES[level];
  document.getElementById('legend').textContent = GAMES[stars].legend;
  const root = document.getElementById('moves');
  root.replaceChildren(...GAMES[stars].moves[level].map(([rule, title, text]) => {
    const sec = document.createElement('section');
    sec.className = 'move';
    const h = document.createElement('h3');
    h.textContent = title;
    // A description's lines are its paragraphs.
    const paras = text.split('\n').map((line) => {
      const p = document.createElement('p');
      p.textContent = line;
      return p;
    });
    sec.append(h, ...paras);
    const ex = examples.find((e) => e.rule === rule);
    if (ex) {
      const row = document.createElement('div');
      row.className = 'examples';
      row.append(figure('Before', ex, ex.marks, true), figure('After', ex, afterMarks(ex), false));
      const why = document.createElement('p');
      why.className = 'example-text';
      why.textContent = `Example: ${hintText({ kind: 'move', ...ex.deduction }, themeOf(ex), stars)}`;
      sec.append(row, why);
    }
    return sec;
  }));
}

async function main() {
  drawCycle();
  // A game without examples yet shows its moves without pictures.
  const load = async (stars) => {
    const res = await fetch(GAMES[stars].examples, { cache: 'no-cache' }).catch(() => null);
    return (res?.ok ? await res.json() : []).map((ex) => ({ ...ex, stars }));
  };
  const examples = { 1: await load(1), 2: await load(2) };
  // The two-animal game shows up here once the site has puzzles for it.
  const published = await fetch('puzzles-2.json', { method: 'HEAD' }).catch(() => null);
  document.getElementById('games').hidden = !published?.ok;
  // The hash names the level, with "2-" in front for the two-animal game.
  const fromHash = () => {
    const [, two, level] = location.hash.match(/^#(2-)?(\w*)$/) || [];
    return { stars: two && published?.ok ? 2 : 1, level: level in MOVES ? level : 'easy' };
  };
  const go = (stars, level) => { location.hash = `${stars === 2 ? '2-' : ''}${level}`; };
  document.querySelectorAll('.levels button').forEach((b) =>
    b.addEventListener('click', () => go(fromHash().stars, b.dataset.level)));
  document.querySelectorAll('.games button').forEach((b) =>
    b.addEventListener('click', () => go(Number(b.dataset.stars), fromHash().level)));
  const show = () => { const { stars, level } = fromHash(); render(stars, level, examples[stars]); };
  window.addEventListener('hashchange', show);
  show();
}

main().catch((err) => {
  document.getElementById('moves').textContent = `Failed to load examples: ${err}`;
});
