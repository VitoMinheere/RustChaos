//! Facts about the install's recorded mission lines (`talk2\`).

/// Mission lines whose recording isn't under their speech number (`Data[9]`), so that the
/// original says them in silence, but is in `talk2\` under the number the point had before
/// the mission was renumbered (#467): (the line's file, the recording's). Found by
/// transcribing the recordings no line plays and matching them to the missions' text.
pub const RENUMBERED: &[(&str, &str)] = &[
    // "Case clue No.5 found! Excellent work, D'arci" (point 347).
    ("MIB.ucm75.wav", "MIB.ucm400.wav"),
    // "Police building secure." (point 156).
    ("mission2.ucm114.wav", "mission2.ucm46.wav"),
    // "Dealer down." (point 264); `mission2.ucm68.wav` is another take.
    ("mission2.ucm112.wav", "mission2.ucm67.wav"),
    // "Way to go." (point 294); `mission2.ucm99.wav` is another take.
    ("mission2.ucm78.wav", "mission2.ucm90.wav"),
    // "I got me a GRENADE, hag! I'll blow us all to HELL..." (point 268's first line).
    ("Semtex.ucm118A.wav", "Semtex.ucm94.wav"),
];

#[cfg(test)]
mod tests {
    /// Each line still has no recording of its own, and the one it's said with is there.
    #[test]
    fn the_renumbered_lines_are_as_the_install_has_them() {
        let Some(game) = crate::default_install() else { return };
        for (line, under) in super::RENUMBERED {
            assert!(crate::resolve(&game, &format!("talk2\\{line}")).is_none(), "{line} has a recording of its own");
            assert!(crate::resolve(&game, &format!("talk2\\{under}")).is_some(), "{under} isn't there");
        }
    }
}
