//! Browser bindings. JSON in, JSON out, so the JS side stays trivial.

use serde::Deserialize;
use wasm_bindgen::prelude::*;

use territories_core::{Level, Mark, Puzzle, State, hint};

#[derive(Deserialize)]
struct HintRequest {
    /// `regions[row][col]` = region index.
    regions: Vec<Vec<usize>>,
    /// Row-major, one char per cell: `.` empty, `x` cross, `A` animal.
    marks: String,
    /// `solution[row]` = animal's column.
    solution: Vec<usize>,
    /// Hardest rule level to suggest (the puzzle's own level).
    level: Level,
}

/// Takes a `HintRequest` as JSON and returns a `territories_core::Hint` as JSON,
/// or `{"kind":"error","message":...}` for bad input.
#[wasm_bindgen]
pub fn hint(request: &str) -> String {
    match run(request) {
        Ok(json) => json,
        Err(e) => serde_json::json!({ "kind": "error", "message": e }).to_string(),
    }
}

fn run(request: &str) -> Result<String, String> {
    let req: HintRequest = serde_json::from_str(request).map_err(|e| e.to_string())?;
    let puzzle = Puzzle::new(&req.regions).map_err(|e| e.to_string())?;
    let n = puzzle.size();
    if req.solution.len() != n || req.solution.iter().any(|&c| c >= n) {
        return Err("solution does not match the grid".into());
    }
    let marks: Vec<char> = req.marks.chars().collect();
    if marks.len() != n * n {
        return Err(format!("expected {} marks, got {}", n * n, marks.len()));
    }
    let mut state = State::new(n);
    for (i, ch) in marks.into_iter().enumerate() {
        let mark = match ch {
            '.' => Mark::Empty,
            'x' => Mark::Cross,
            'A' => Mark::Animal,
            other => return Err(format!("unknown mark {other:?}")),
        };
        state.set((i / n, i % n), mark);
    }
    let h = hint::hint(&puzzle, &state, &req.solution, req.level);
    serde_json::to_string(&h).map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    const REGIONS: &str = "[[0,0,1,1],[0,0,0,1],[2,2,3,1],[2,2,3,3]]";

    #[test]
    fn move_hint_is_flat_json() {
        let req = format!(
            r#"{{"regions":{REGIONS},"marks":"................","solution":[1,3,0,2],"level":"hard"}}"#
        );
        let out = hint(&req);
        assert!(out.starts_with(r#"{"kind":"move","rule":"#), "{out}");
    }

    #[test]
    fn bad_input_is_an_error_not_a_panic() {
        let req = format!(r#"{{"regions":{REGIONS},"marks":"..","solution":[1,3,0,2],"level":"easy"}}"#);
        assert!(hint(&req).contains(r#""kind":"error""#));
    }
}
