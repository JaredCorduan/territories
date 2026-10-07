use serde::{Deserialize, Serialize};

/// A grid position as `(row, col)`.
pub type Cell = (usize, usize);

/// Largest supported grid; unit sets are stored as `u32` bitmasks.
pub const MAX_SIZE: usize = 32;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum UnitKind {
    Row,
    Col,
    Region,
}

impl UnitKind {
    pub const ALL: [UnitKind; 3] = [UnitKind::Row, UnitKind::Col, UnitKind::Region];

    pub fn index(self) -> usize {
        match self {
            UnitKind::Row => 0,
            UnitKind::Col => 1,
            UnitKind::Region => 2,
        }
    }
}

/// A row, column, or region: each must hold exactly one animal.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Unit {
    Row(usize),
    Col(usize),
    Region(usize),
}

impl Unit {
    pub fn new(kind: UnitKind, index: usize) -> Unit {
        match kind {
            UnitKind::Row => Unit::Row(index),
            UnitKind::Col => Unit::Col(index),
            UnitKind::Region => Unit::Region(index),
        }
    }

    pub fn kind(self) -> UnitKind {
        match self {
            Unit::Row(_) => UnitKind::Row,
            Unit::Col(_) => UnitKind::Col,
            Unit::Region(_) => UnitKind::Region,
        }
    }

    pub fn index(self) -> usize {
        match self {
            Unit::Row(i) | Unit::Col(i) | Unit::Region(i) => i,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PuzzleError {
    BadSize(usize),
    NotSquare,
    BadRegionIndex(usize),
    EmptyRegion(usize),
}

impl std::fmt::Display for PuzzleError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PuzzleError::BadSize(n) => write!(f, "size {n} is outside 1..={MAX_SIZE}"),
            PuzzleError::NotSquare => write!(f, "region grid is not square"),
            PuzzleError::BadRegionIndex(i) => write!(f, "region index {i} is out of range"),
            PuzzleError::EmptyRegion(i) => write!(f, "region {i} has no cells"),
        }
    }
}

impl std::error::Error for PuzzleError {}

/// An N×N grid partitioned into N regions.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Puzzle {
    size: usize,
    regions: Vec<usize>,
    region_cells: Vec<Vec<Cell>>,
}

impl Puzzle {
    /// Builds a puzzle from `grid[row][col] = region index`.
    pub fn new(grid: &[Vec<usize>]) -> Result<Puzzle, PuzzleError> {
        let size = grid.len();
        if size == 0 || size > MAX_SIZE {
            return Err(PuzzleError::BadSize(size));
        }
        if grid.iter().any(|row| row.len() != size) {
            return Err(PuzzleError::NotSquare);
        }
        let mut region_cells = vec![Vec::new(); size];
        for (r, row) in grid.iter().enumerate() {
            for (c, &id) in row.iter().enumerate() {
                if id >= size {
                    return Err(PuzzleError::BadRegionIndex(id));
                }
                region_cells[id].push((r, c));
            }
        }
        if let Some(i) = region_cells.iter().position(Vec::is_empty) {
            return Err(PuzzleError::EmptyRegion(i));
        }
        Ok(Puzzle {
            size,
            regions: grid.concat(),
            region_cells,
        })
    }

    pub fn size(&self) -> usize {
        self.size
    }

    pub fn region(&self, (r, c): Cell) -> usize {
        self.regions[r * self.size + c]
    }

    pub fn region_cells(&self, region: usize) -> &[Cell] {
        &self.region_cells[region]
    }

    pub fn grid(&self) -> Vec<Vec<usize>> {
        self.regions.chunks(self.size).map(<[usize]>::to_vec).collect()
    }

    pub fn cells(&self) -> impl Iterator<Item = Cell> + '_ {
        (0..self.size).flat_map(move |r| (0..self.size).map(move |c| (r, c)))
    }

    /// The unit of the given kind that contains `cell`.
    pub fn unit_of(&self, kind: UnitKind, cell: Cell) -> Unit {
        match kind {
            UnitKind::Row => Unit::Row(cell.0),
            UnitKind::Col => Unit::Col(cell.1),
            UnitKind::Region => Unit::Region(self.region(cell)),
        }
    }

    pub fn unit_cells(&self, unit: Unit) -> Vec<Cell> {
        match unit {
            Unit::Row(r) => (0..self.size).map(|c| (r, c)).collect(),
            Unit::Col(c) => (0..self.size).map(|r| (r, c)).collect(),
            Unit::Region(i) => self.region_cells[i].clone(),
        }
    }

    pub fn contains(&self, unit: Unit, cell: Cell) -> bool {
        self.unit_of(unit.kind(), cell) == unit
    }

    /// True if animals on `a` and `b` would conflict (shared row, column,
    /// region, or touching, including diagonally).
    pub fn attacks(&self, a: Cell, b: Cell) -> bool {
        a != b
            && (a.0 == b.0
                || a.1 == b.1
                || self.region(a) == self.region(b)
                || (a.0.abs_diff(b.0) <= 1 && a.1.abs_diff(b.1) <= 1))
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Mark {
    #[default]
    Empty,
    Cross,
    Animal,
}

/// The marks on a board, as a player (or the solver) has filled it in.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct State {
    size: usize,
    marks: Vec<Mark>,
}

impl State {
    pub fn new(size: usize) -> State {
        State {
            size,
            marks: vec![Mark::Empty; size * size],
        }
    }

    pub fn size(&self) -> usize {
        self.size
    }

    pub fn get(&self, (r, c): Cell) -> Mark {
        self.marks[r * self.size + c]
    }

    pub fn set(&mut self, (r, c): Cell, mark: Mark) {
        self.marks[r * self.size + c] = mark;
    }

    pub fn is_candidate(&self, cell: Cell) -> bool {
        self.get(cell) == Mark::Empty
    }

    pub fn animals(&self) -> Vec<Cell> {
        (0..self.marks.len())
            .filter(|&i| self.marks[i] == Mark::Animal)
            .map(|i| (i / self.size, i % self.size))
            .collect()
    }
}
