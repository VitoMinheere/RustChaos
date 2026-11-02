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

#[test]
fn mr_mcintyres_lines() {
    let Some(lines) = added("gang2") else { return };
    // Mcintyre (point 122) gives in, and despatch says he's in.
    assert_eq!(lines, [("Okay okay! I SURRENDER!!".to_string(), 901, 122),
                       ("You got him! Mr Mcintyre's gonna be our guest for a while.".to_string(), 902, 0)]);
}

#[test]
fn carbomb2s_debrief_is_put_back() {
    let Some(lines) = added("carbomb2") else { return };
    assert_eq!(lines, [("Whooa, what's this? \"UCNN video\"?".to_string(), 901, 0),
                       ("Stern, what the hell you up to? We ain't got all day to wait for you to get back here - ain't ya found anythin' yet?".to_string(), 902, 19)]);
    let game = uc_formats::default_install().unwrap();
    let mut m = Mission::parse(&std::fs::read(uc_formats::resolve(&game, "levels/carbomb2.ucm").unwrap()).unwrap()).unwrap();
    uc_world::level::finish_unfinished("carbomb2", &mut m);
    // Nothing waits for a point that isn't there any more: the debriefs, the Sarge, the win.
    let there: std::collections::BTreeSet<usize> = m.event_points.iter().filter(|e| e.used != 0).map(|e| e.index).collect();
    use uc_formats::ucm::trigger;
    for e in m.event_points.iter().filter(|e| matches!(e.trigger, trigger::DEPENDENCY | trigger::BOOLEAN_AND | trigger::BOOLEAN_OR | trigger::ITEM_HELD | trigger::COUNTDOWN)) {
        for r in [e.ep_ref as usize, if matches!(e.trigger, trigger::BOOLEAN_AND | trigger::BOOLEAN_OR) { e.ep_ref_bool as usize } else { e.ep_ref as usize }] {
            assert!(r == 0 && e.trigger == trigger::COUNTDOWN || there.contains(&r), "point {} waits for {r}, which isn't there", e.index);
        }
    }
    // The tape is stashed at the hideout, and the debriefs keep their own lines.
    let tape = m.event_points.iter().find(|e| e.index == 181).unwrap();
    assert_eq!((tape.kind, tape.data[0], tape.data[2]), (waypoint::CREATE_ITEM, 19, 2));
    let texts = m.point_texts();
    assert!(texts[&172].starts_with("Well? Come up with anything?"), "{}", texts[&172]);
    assert!(texts[&182].starts_with("You find anything?"), "{}", texts[&182]);
    // Twice changes nothing.
    let n = m.event_points.len();
    uc_world::level::finish_unfinished("carbomb2", &mut m);
    assert_eq!(m.event_points.len(), n);
}
