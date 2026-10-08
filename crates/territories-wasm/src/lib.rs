//! Browser bindings. JSON in, JSON out, so the JS side stays trivial.

use serde::Deserialize;
use wasm_bindgen::prelude::*;

use territories_core::{Level, Mark, Puzzle, State, one_star, two_star};

#[derive(Deserialize)]
struct HintRequest {
    /// Animals per row, column, and territory: 1 (if omitted) or 2.
    #[serde(default = "one")]
    stars: usize,
    /// `regions[row][col]` = region index.
    regions: Vec<Vec<usize>>,
    /// Row-major, one char per cell: `.` empty, `x` cross, `A` animal.
    marks: String,
    /// `solution[row]` = the animal's column, or with two animals, both columns.
    solution: serde_json::Value,
    /// Hardest rule level to suggest (the puzzle's own level).
    level: Level,
}

fn one() -> usize {
    1
}

/// Takes a `HintRequest` as JSON and returns that game's `Hint` as JSON,
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
    let mismatch = || "solution does not match the grid".to_string();
    let hint = match req.stars {
        1 => {
            let solution: Vec<usize> = serde_json::from_value(req.solution).map_err(|e| e.to_string())?;
            if solution.len() != n || solution.iter().any(|&c| c >= n) {
                return Err(mismatch());
            }
            serde_json::to_string(&one_star::hint::hint(&puzzle, &state, &solution, req.level))
        }
        2 => {
            let solution: two_star::Solution = serde_json::from_value(req.solution).map_err(|e| e.to_string())?;
            if n > two_star::MAX_SIZE || solution.len() != n || solution.iter().flatten().any(|&c| c >= n) {
                return Err(mismatch());
            }
            serde_json::to_string(&two_star::hint::hint(&puzzle, &state, &solution, req.level))
        }
        other => return Err(format!("no game with {other} animals")),
    };
    hint.map_err(|e| e.to_string())
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
    fn two_animal_hints_use_their_own_rules() {
        let rows: Vec<String> = (0..9).map(|r| format!("[{}]", vec![r.to_string(); 9].join(","))).collect();
        let req = format!(
            r#"{{"stars":2,"regions":[{}],"marks":"{}","solution":[[0,2],[4,6],[1,8],[3,5],[0,7],[2,4],[6,8],[1,3],[5,7]],"level":"easy"}}"#,
            rows.join(","),
            "A".to_string() + &".".repeat(80),
        );
        let out = hint(&req);
        assert!(out.starts_with(r#"{"kind":"move","rule":"animal_shadow""#), "{out}");
    }

    #[test]
    fn bad_input_is_an_error_not_a_panic() {
        let req = format!(r#"{{"regions":{REGIONS},"marks":"..","solution":[1,3,0,2],"level":"easy"}}"#);
        assert!(hint(&req).contains(r#""kind":"error""#));
    }
}
