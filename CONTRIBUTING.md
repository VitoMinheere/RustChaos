# Contributing

## Before a pull request

```
cargo clippy --workspace --all-targets -- -D warnings
cargo test --release --workspace
```

CI fails on any Clippy warning. It can't run the tests that need the game's data, because there's no Urban Chaos install on CI, so they skip themselves and pass. That includes every mission play-through in `crates/uc-game/tests`. **Run the tests locally, with the game installed, before merging**, and after resolving merge conflicts in `uc-game`: a merge that builds can still break a play-through.

The install is found from `UC_GAME_DIR`, or the default GOG location.

## Mission tests

`crates/uc-game/tests/common/mod.rs` has what the mission tests share:
- `common::start(mission)` loads a mission and checks it uses nothing unsupported;
- `common::load(mission)` with `Run::new(level)` does the same without the check, for a mission or level that needs changing first;
- `Run::play` plays with the cutscenes skipped, logs every new message to `seen`, keeps the player's health topped up (`heal`), stops at a win or loss (`stop_at_outcome`), and runs `each_turn` before every update, for example to take off the enemies who'd get in the way;
- `until`, `said`, `point`, `who`, `player`, `stand` and `look_at`.

A test file starts with `mod common;` and adds its own helpers in an `impl Run` block of its own. A helper only one mission needs belongs in that mission's file. One that a second file would copy belongs in `common`.

## Worktrees

Working on several branches at once with `git worktree` is fine, but give each worktree its own `target` directory, which is cargo's default. A `CARGO_TARGET_DIR` shared between worktrees can reuse crates compiled from another worktree's files, and the build then tests code that isn't on the branch.

## Line endings and style

- `.gitattributes` checks text files out with LF on every platform.
- Doc comments cite the original source (`Person.cpp:1234`, `PCOM_set_person_move_runaway`) where the code follows it. Keep the citation when moving code.
