//! Replays (#184): every turn's inputs kept as they're played, for `uc_net::replay` to save.
//! A turn is kept in [`Game::step`], after whatever made its inputs (the keyboard, a pad, a
//! co-op session's turn), so it doesn't matter where they came from. A cutscene skipped
//! between two turns goes with the next one, and every so often the state hash goes with a
//! turn, to tell a replay that has drifted.

use crate::*;

/// A turn as it was played.
#[derive(Debug, Clone, Default)]
pub struct Played {
    /// Every seat's input, seat 0 first.
    pub inputs: Vec<Controls>,
    /// A cutscene was skipped just before it.
    pub skip: bool,
    /// [`Game::state_hash`] after it, on every turn the keeping asked for.
    pub hash: Option<u64>,
}

/// The turns played since they were last taken.
#[derive(Debug, Clone, Default)]
pub(crate) struct Keeping {
    turns: Vec<Played>,
    /// A cutscene skipped since the last turn.
    skip: bool,
    /// The state hash goes with every turn that leaves this many played, and its multiples.
    hash_every: u64,
}

impl Game {
    /// Starts keeping every turn played from now on, with the state hash after every
    /// `hash_every` turns.
    pub fn keep_played(&mut self, hash_every: u64) {
        self.keeping = Some(Keeping { hash_every: hash_every.max(1), ..Default::default() });
    }

    /// Whether turns are being kept.
    pub fn keeping_played(&self) -> bool {
        self.keeping.is_some()
    }

    /// The turns played since the last call, in order.
    pub fn take_played(&mut self) -> Vec<Played> {
        self.keeping.as_mut().map_or(Vec::new(), |k| std::mem::take(&mut k.turns))
    }

    /// A cutscene is being skipped: the next turn kept says so.
    pub(crate) fn keep_skip(&mut self) {
        if let Some(k) = &mut self.keeping {
            k.skip = true;
        }
    }

    /// The turn just played, with `inputs`.
    pub(crate) fn keep_turn(&mut self, inputs: &[Controls]) {
        let Some(every) = self.keeping.as_ref().map(|k| k.hash_every) else { return };
        let hash = self.turns.is_multiple_of(every).then(|| self.state_hash());
        let seats = self.seats();
        let k = self.keeping.as_mut().unwrap();
        let inputs = (0..seats).map(|s| inputs.get(s).copied().unwrap_or_default()).collect();
        k.turns.push(Played { inputs, skip: std::mem::take(&mut k.skip), hash });
    }
}
