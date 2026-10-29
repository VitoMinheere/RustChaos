//! The "Mucky Time" on the win screen (`ScoresDraw`'s `mucky_times`, fallen/Source/Attract.cpp:
//! 868, the live `#else` table): a time for each mission set by one of the team, and a code for
//! anyone who beats it fairly (`hash`, Attract.cpp:930), to send in.

/// Mission file, the time in minutes and seconds, and who set it. The training missions have
/// none (no `who`).
const MUCKY_TIMES: [(&str, u32, u32, Option<&str>); 35] = [
    ("ftutor1", 0, 0, None),
    ("assault1", 0, 0, None),
    ("testdrive1a", 0, 0, None),
    ("fight1", 0, 0, None),
    ("police1", 1, 5, Some("Alex Hood")),
    ("testdrive2", 0, 0, None),
    ("fight2", 0, 0, None),
    ("police2", 1, 32, Some("Joe Neate")),
    ("testdrive3", 0, 0, None),
    ("bankbomb1", 5, 0, Some("Kenny Tang")),
    ("police3", 1, 37, Some("Alex Hood")),
    ("gangorder2", 3, 14, Some("David Babajee")),
    ("police4", 1, 24, Some("Kaoski")),
    ("bball2", 0, 45, Some("Tom Forsyth")),
    ("wstores1", 4, 3, Some("Joe Neate")),
    ("e3", 5, 54, Some("Kwesi Moodie")),
    ("westcrime1", 6, 4, Some("Alex Hood")),
    ("carbomb1", 4, 18, Some("David Babajee")),
    ("botanicc", 6, 29, Some("Kwesi Moodie")),
    ("semtex", 16, 3, Some("Kaoski")),
    ("awol1", 10, 30, Some("Kenny Tang")),
    ("mission2", 7, 49, Some("Alex Hood")),
    ("park2", 6, 45, Some("Tom Forsyth")),
    ("mib", 26, 53, Some("Phil Maskell")),
    ("skymiss2", 9, 28, Some("David Babajee")),
    ("factory1", 8, 35, Some("Kwesi Moodie")),
    ("estate2", 8, 14, Some("Phil Maskell")),
    ("stealtst1", 5, 28, Some("Kaoski")),
    ("snow2", 2, 26, Some("Kaoski")),
    ("gordout1", 5, 42, Some("Kenny Tang")),
    ("baalrog3", 2, 38, Some("Mike Diskett")),
    ("finale1", 40, 0, Some("Beat me")),
    ("gangorder1", 5, 21, Some("Joe Neate")),
    ("freecd1", 40, 0, Some("Beat me")),
    ("album1", 15, 38, Some("Tom Forsyth")),
];

/// The line for a mission won in `secs`: its Mucky Time and who set it, and, beaten fairly (no
/// cheats: `fair`), the code (`: CODE <hash>`). A mission without one says so, as it did.
pub(crate) fn line(mission: &str, secs: f32, fair: bool) -> (String, String) {
    let total = secs.max(0.0) as u32;
    let (h, m, s) = (total / 3600, total / 60 % 60, total % 60);
    let name = mission.to_ascii_lowercase();
    let found = MUCKY_TIMES.iter().enumerate().find(|(_, t)| name == t.0 && t.3.is_some());
    let Some((i, &(_, mins, ss, who))) = found else {
        return (format!("({h:02}:{m:02}:{s:02})"), "No time yet. Email TomF your time!".to_string());
    };
    let beaten = fair && total < mins * 60 + ss;
    let code = if beaten { format!("   CODE <{:X}>", code(i as u32, m, s)) } else { String::new() };
    (format!("({h:02}:{m:02}:{s:02}){code}"), format!("(00:{mins:02}:{ss:02})  {}", who.unwrap_or_default()))
}

/// `hash` (Attract.cpp:930): from the mission's place in the table and the minutes and
/// seconds of the time.
fn code(i: u32, m: u32, s: u32) -> u32 {
    (i + 1) * (m + 1) * (s + 1) * 3141 % 12345 + 0x9a2f
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn gun_hunt_beaten_and_not() {
        // Gun Hunt (police3, the eleventh) has Alex Hood's 1:37.
        let (t, par) = line("police3", 90.0, true);
        assert_eq!(par, "(00:01:37)  Alex Hood");
        assert_eq!(t, format!("(00:01:30)   CODE <{:X}>", (11 * 2 * 31 * 3141) % 12345 + 0x9a2f));
        assert_eq!(line("police3", 120.0, true).0, "(00:02:00)");
        assert_eq!(line("police3", 90.0, false).0, "(00:01:30)", "no code after cheating");
        assert_eq!(line("FTutor1", 300.0, true).1, "No time yet. Email TomF your time!");
    }
}
