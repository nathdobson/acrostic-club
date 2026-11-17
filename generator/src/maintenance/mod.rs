// Tools that bring committed puzzles up to date after a change to the code that built them.
// Building a new puzzle never runs them: they exist so that a fix to an early stage can reach
// puzzles whose later stages are already built, without discarding those stages' work.
//
// A tool must leave each puzzle as the current code could have built it from scratch. Each stage
// file the tool rewrites should hold what that stage would build from the earlier stages' files;
// where a stage picks one of several valid outputs, such as the answers a search finds, the file
// may keep the pick it already holds. The rule serves the generator's aim to be deterministic,
// reproducible, and hermetic.

pub mod rebuild_cells;
