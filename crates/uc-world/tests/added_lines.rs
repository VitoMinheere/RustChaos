//! The lines `finish_unfinished` adds have recordings of their own, numbered from 901
//! (`talk2\<mission>.ucm<speech>.wav`), whatever the points saying them are numbered.
//! Skipped without an install.

use uc_formats::ucm::{waypoint, Mission};

/// Each added line's text, with its recording's number and who says it (0 for despatch).
fn added(mission: &str) -> Option<Vec<(String, i32, i32)>> {
    let game = uc_formats::default_install()?;
    let mut m = Mission::parse(&std::fs::read(uc_formats::resolve(&game, &format!("levels/{mission}.ucm"))?).ok()?).ok()?;
    uc_world::level::finish_unfinished(mission, &mut m);
    let texts = m.point_texts();
    let mut lines: Vec<_> = m.event_points.iter().filter(|e| e.used != 0 && e.kind == waypoint::MESSAGE && e.data[9] > 900)
        .map(|e| (texts[&e.index].clone(), e.data[9], e.data[2])).collect();
    lines.sort_by_key(|l| l.1);
    lines.dedup();
    Some(lines)
}

#[test]
fn twenty_fourth_street_shootings_lines() {
    let Some(lines) = added("Crimescene") else { return };
    let said = |n: i32| lines.iter().find(|l| l.1 == n).unwrap_or_else(|| panic!("no {n}: {lines:?}"));
    assert!(said(901).0.starts_with("Nothin', huh?") && said(901).2 == 16, "{:?}", said(901));
    assert!(said(902).0.contains("our only lead") && said(902).2 == 0, "{:?}", said(902));
    assert!(said(903).0.starts_with("You're the geek") && said(903).2 == 16, "{:?}", said(903));
    assert_eq!(lines.len(), 3, "{lines:?}");
}

#[test]
fn one_recording_for_each_line() {
    // Takeover1's for either hostage, police5's for any Wildcat killed.
    for (mission, n) in [("Takeover1", 1), ("police5", 2)] {
        let Some(lines) = added(mission) else { return };
        assert_eq!(lines.len(), n, "{mission}: {lines:?}");
        assert!(lines.iter().enumerate().all(|(k, l)| l.1 == 901 + k as i32), "{mission}: {lines:?}");
    }
}
