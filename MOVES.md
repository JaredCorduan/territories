# Moves: Territories, a Star Battle puzzle

Every move is a pattern you can spot directly on the board: no guessing, and no "suppose an animal went here…". A puzzle's level is the hardest move the grader needed. After every move, the grader goes back to the easiest tier.

Terms:
- **territory**: a colored region; each has its own animal.
- **unit**: a row, a column, or a territory. Each unit gets exactly one animal.
- **line**: a row or a column.
- **open cell**: a cell that is not yet X'd and has no animal.
- **attacks**: two cells attack each other if they share a unit or if they touch (including diagonally).

Diagram key: letters are territories, `x` is a crossed-out cell, `*` marks the cells the move X's.

## Easy

**Shadow** (`animal_shadow`): an animal X's every cell it attacks. The UI does this automatically.

**Last spot** (`last_spot`): a unit with one open cell gets its animal there.

**Claimed line** (`claimed_line`): if all of a territory's open cells lie in one line, X the rest of that line.

```
A A B B B B      A lives only in row 0:
x x B B B B      A A * * * *
```

**Claimed territory** (`claimed_region`): if all of a line's open cells lie in one territory, X the rest of that territory.

**L-corner** (`l_corner`): if a territory's open cells are three cells of a 2×2 (an L), X the fourth cell, which touches all three.

```
A A .            A = the L. The fourth cell (*) touches
A * .            every A cell.
```

## Medium

**Squeeze** (`squeeze`): if an animal on cell `c` would attack every open cell of some other unit, X `c`. L-corner is the most common case of this.

**Territory band, 2** (`region_band_2`): if two territories' open cells lie within the same two lines, X everything else in those lines.

```
A A A B B        A and B live only in rows 0-1:
A C C C B        X the C cells in rows 0-1.
```

**Line band, 2** (`line_band_2`): if two lines' open cells lie within the same two territories (or two lines of the other direction), X everything else in those territories (or lines).

## Hard

**Territory band / Line band, 3+** (`region_band_3`, `line_band_3`): the same with three or more units. The grader reports the smaller of the two equivalent views: k territories locked into k rows means the other rows are locked into the other territories.

## Brutal

All four are the same counting idea: k units that share no open cell need k animals. If their open cells fit inside places that hold at most k animals in all, those places are full, so X every other open cell in them. The grader tries 2×2 packing first, then, for each k, mixed packing, mixed band, and 3×3 packing. Brutal puzzles come from the farm's dedicated brutal threads.

**2×2 packing** (`block_packing`): a 2×2 block never holds two animals. If k units (all of one kind) need k animals and their open cells fit inside k 2×2 blocks, those blocks are full. Almost never needed.

**3×3 packing** (`window_packing`): a 3×3 window holds at most two animals, since three would need all three of its rows and columns, and the middle one would touch another. If two units that share no open cell have all their open cells inside one 3×3 window, X the rest of the window.

**Mixed packing** (`mixed_packing`): 2×2 packing with any mix of lines, territories, and 2×2 blocks as the k places.

```
A * * * * B      A lives in row 0 or the 2×2 block, and so does B.
. . . . . .      Row 0 holds one of them and the block the other:
. . . . . .      X the rest of row 0 and of the block.
. . . A * .
. . . * B .
```

**Mixed band** (`mixed_band`): the k units are of different kinds (say a row and a territory) and share no open cell, so they still need k different animals. If their open cells fit inside k lines or territories, X everything else in those.

## Deliberately excluded

- Contradiction, trial and error, and "if this, then …" chains (including "both options lead to Z").
- Uniqueness arguments.

## Two animals

The two-animal game (two animals in every row, column, and territory) has its own moves and its own levels. A unit is **full** once it has both animals, and two cells **touch** if they are neighbors, including diagonally. Sharing a unit is no longer a clash by itself.

Most of these are one idea. Take some cells known to hold exactly k animals, and picture every way k animals fit there without touching. A cell every way uses is an animal. A cell where an animal would leave no way is crossed out.

### Easy

**Shadow** (`animal_shadow`): an animal X's the cells touching it. The UI does this automatically.

**Full** (`full_unit`): a full unit's other cells are X'd. The UI does this automatically.

**Last spots** (`last_spots`): a unit with as many open cells as animals it still needs gets them all.

**Claimed line** (`claimed_line`): if all of a territory's open cells lie in one line, and it needs as many animals as the line does, X the rest of the line. Two territories needing one each can share a line needing two.

**Claimed territory** (`claimed_region`): if all of a line's open cells lie in one territory, and the line needs as many animals as the territory does, X the rest of the territory.

**Small squeeze** (`small_squeeze`): the idea above, on one unit with at most four open cells.

```
A A A            A needs two: they take the ends,
                 and the middle is X'd.

. * .            A needs one of its two cells (A A).
. A A            The cells touching both are X'd.
. * *
```

### Medium

**Squeeze** (`squeeze`): the same on a unit with any number of open cells. Four open cells in a line X the cells on both sides of all four.

**Crowded squeeze** (`crowded_squeeze`): a squeeze that also counts the room other units have left. A row that already holds an animal takes only one more.

**Territory band, 2** (`region_band_2`): Claimed line with two lines: territories inside two lines that need all the animals those lines do. X everything else in those lines.

**Line band, 2** (`line_band_2`): Claimed territory with two lines: two lines whose open cells lie in territories needing just as many animals. X everything else in those territories.

### Hard

**Territory band / Line band, 3+** (`region_band_3`, `line_band_3`): the same with three or more lines.

**Leftover** (`leftover`): one line and the territories it touches leave a set of cells holding a known number of animals. If a row's open cells all lie in two territories, the row takes two of their four animals, so the rest of those territories holds exactly two. Then squeeze those cells as if they were a unit.

### Brutal

**Wide leftover** (`wide_leftover`): the leftover across a run of two or more neighboring lines.

**Mixed band** (`mixed_band`): a band whose units are a mix of kinds. Units that share no open cell (say a row and a territory), with all their open cells inside units needing just as many animals (say two columns, or a row and a column): X everything else in those. Where two of the outer units cross, X that cell too.

**Leftover cap** (`leftover_cap`): a leftover holds a known number of animals, so no other unit takes more than that from its cells. Squeeze a unit counting that limit. In practice the leftover holds one and the unit needs two, so at least one of the unit's animals goes in its open cells outside the leftover. If those all touch, they hold exactly one, the unit's other animal is the leftover's only one, and the rest of the leftover is X'd.

**Loose leftover** (`loose`): a run of one to three lines shares its animals among the territories crossing it, and each takes only what fits in its part of the run. A territory the others can't cover for must take at least what remains, so the rest of it holds at most what it has left: X it if that is none, or else squeeze another unit counting that limit.

Most brutal puzzles need only the wide leftover. The grader uses the other three only when a puzzle can't be solved without them, and the farm publishes two such puzzles for every three of the others, in a shuffled order.

Still excluded: trial and error (placing a mark and following the rules until something breaks) and uniqueness arguments.

## JSON output

Each puzzle carries `level` (`easy` / `medium` / `hard` / `brutal`) and a `moves` tally:

```json
{ "id": "medium-8-004", "level": "medium", "size": 8,
  "regions": [[0,0,1, ...], ...], "solution": [3,0,6, ...],
  "moves": { "last_spot": 5, "animal_shadow": 8, "squeeze": 2 } }
```

Two-animal puzzles have ids starting with `2star-`, and `solution` lists both columns for each row (`[[1, 4], [6, 8], ...]`).
