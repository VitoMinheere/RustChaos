//! What there is to do in a mission beyond winning it, read from its script: the badges
//! (`create_treasure` points; the original's end screen counts them as bonuses found and
//! missed) and the side objectives, the jobs that earn badges when done (a treasure point
//! with a trigger: arrest the extra mugger, find officer Miles' car...).

use std::collections::BTreeMap;
use uc_formats::ucm::{trigger, waypoint, EventPoint, Mission};

type Points<'a> = BTreeMap<usize, &'a EventPoint>;

/// A named street: the area of a `cuboid` message that names it (centre x, z; half sizes).
struct Street {
    name: String,
    x: f32,
    z: f32,
    hx: f32,
    hz: f32,
}

/// A side objective: done once the script creates its reward.
#[derive(Debug, Clone, PartialEq)]
pub struct SideGoal {
    /// The event point whose triggering completes it.
    pub id: usize,
    /// The badge points it creates.
    pub rewards: Vec<usize>,
    /// What to do, worked out from the script: "Arrest the Wildcat on a rooftop near
    /// Kleetus Ave" (or, failing that, what's said when it's done).
    pub label: String,
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct Goals {
    /// Every badge point.
    pub badges: Vec<usize>,
    pub sides: Vec<SideGoal>,
}

/// Which stat badge point `index` raises (stamina, strength, reflexes, constitution). The
/// original picks one of the four at random each time; fixing it per point makes a badge the
/// same badge on every run, so collecting them can be tracked.
pub fn badge_stat(index: usize) -> usize {
    ((index as u32).wrapping_mul(2_654_435_761) >> 16) as usize % 4
}

/// A mission's badges and side objectives.
pub fn goals(m: &Mission) -> Goals {
    let texts = m.point_texts();
    let points: BTreeMap<usize, _> = m.event_points.iter().filter(|e| e.used != 0).map(|e| (e.index, e)).collect();
    let streets: Vec<Street> = points
        .values()
        .filter(|e| e.kind == waypoint::MESSAGE && e.trigger == trigger::CUBOID)
        .filter_map(|e| {
            let name = texts.get(&e.index)?.trim().trim_end_matches('.').to_string();
            let r = e.radius as u32;
            Some(Street { name, x: e.x as f32, z: e.z as f32, hx: (r & 0xFFFF) as f32, hz: (r >> 16) as f32 })
        })
        .collect();
    let mut g = Goals::default();
    for e in points.values().filter(|e| e.kind == waypoint::CREATE_TREASURE) {
        g.badges.push(e.index);
        if e.trigger == trigger::NONE {
            continue;
        }
        let id = e.ep_ref as usize;
        match g.sides.iter_mut().find(|s| s.id == id) {
            Some(s) => s.rewards.push(e.index),
            None => {
                let label = describe(e, &points, &streets)
                    .or_else(|| label(id, &points, &texts, 0))
                    .unwrap_or_else(|| "Side objective".into());
                g.sides.push(SideGoal { id, rewards: vec![e.index], label });
            }
        }
    }
    g
}

/// What earns reward badge `e`, in words.
fn describe(e: &EventPoint, p: &Points, streets: &[Street]) -> Option<String> {
    let r = p.get(&(e.ep_ref as usize))?;
    if e.trigger == trigger::CONVERSATION_OVER {
        return talk(r, p, streets);
    }
    act(r, p, streets, 0)
}

/// The deed that sets off point `e`.
fn act(e: &EventPoint, p: &Points, streets: &[Street], depth: u32) -> Option<String> {
    let r = e.ep_ref as usize;
    match e.trigger {
        trigger::PERSON_ARRESTED => Some(format!("Arrest {}", who(r, p, streets))),
        trigger::KILLED | trigger::KILLED_NOT_ARRESTED => Some(format!("Take down {}", who(r, p, streets))),
        trigger::PERSON_USED => Some(format!("Talk to {}", who(r, p, streets))),
        trigger::CONVERSATION_OVER => talk(p.get(&r)?, p, streets),
        trigger::DEPENDENCY | trigger::BOOLEAN_AND if depth < 4 => act(p.get(&r)?, p, streets, depth + 1),
        _ if e.kind == waypoint::CONVERSATION => talk(e, p, streets),
        _ => None,
    }
}

/// "Talk to ..." for conversation point `c`: whoever in it isn't Darci (point 1 creates her).
fn talk(c: &EventPoint, p: &Points, streets: &[Street]) -> Option<String> {
    if c.kind != waypoint::CONVERSATION {
        return None;
    }
    let other = [c.data[1], c.data[2]].into_iter().map(|w| w as usize).find(|&w| w > 1 && p.contains_key(&w))?;
    Some(format!("Talk to {}", who(other, p, streets)))
}

/// The person event point `way` creates, and where: "the Wildcat on a rooftop near Kleetus
/// Ave".
fn who(way: usize, p: &Points, streets: &[Street]) -> String {
    let Some(e) = p.get(&way).filter(|e| e.kind == waypoint::CREATE_ENEMIES) else { return "someone".into() };
    // `ET_*` (fallen/Headers/Mission.h).
    let noun = match e.data[0] & 0xFFFF {
        3 | 4 => "woman",
        5 | 17 => "workman",
        6..=11 => "Wildcat",
        12..=15 => "cop",
        16 => "hostage",
        18 => "tramp",
        19..=21 => "MIB agent",
        _ => "civilian",
    };
    let (x, z) = (e.x as f32, e.z as f32);
    let high = if e.y >= 512 { " on a rooftop" } else { "" };
    // The nearest named street within six squares, else which part of town.
    let near = streets
        .iter()
        .map(|s| ((x - s.x).abs() - s.hx).max(0.0).hypot(((z - s.z).abs() - s.hz).max(0.0)))
        .zip(streets)
        .filter(|(d, _)| *d < 6.0 * 256.0)
        .min_by(|a, b| a.0.total_cmp(&b.0));
    let place = match near {
        Some((_, s)) => format!(" near {}", s.name),
        None => {
            // The map is 128 squares across, north (-z) up.
            let (dx, dz) = (x / 256.0 - 64.0, z / 256.0 - 64.0);
            let ns = if dz < -21.0 { "north" } else if dz > 21.0 { "south" } else { "" };
            let ew = if dx < -21.0 { "west" } else if dx > 21.0 { "east" } else { "" };
            match (ns, ew) {
                ("", "") => " in the middle of town".into(),
                (a, "") | ("", a) => format!(" in the {a}"),
                (a, b) => format!(" in the {a}-{b}"),
            }
        }
    };
    format!("the {noun}{high}{place}")
}

/// The first sentence said by point `i`, or by what it depends on.
fn label(i: usize, points: &BTreeMap<usize, &uc_formats::ucm::EventPoint>, texts: &BTreeMap<usize, String>,
         depth: u32) -> Option<String> {
    let e = points.get(&i)?;
    if let Some(t) = texts.get(&i) {
        return Some(first_sentence(t.split('|').next().unwrap_or("")));
    }
    if depth > 4 {
        return None;
    }
    label(e.ep_ref as usize, points, texts, depth + 1).or_else(|| label(e.ep_ref_bool as usize, points, texts, depth + 1))
}

/// Up to the first full stop, exclamation or question mark after a few words, and at most
/// about 48 characters.
fn first_sentence(t: &str) -> String {
    let t = t.trim();
    let end = t.char_indices().find(|&(k, c)| k >= 12 && matches!(c, '.' | '!' | '?')).map_or(t.len(), |(k, c)| k + c.len_utf8());
    let s = &t[..end];
    if s.chars().count() > 48 {
        let cut: String = s.chars().take(45).collect();
        format!("{}...", cut.trim_end())
    } else {
        s.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sentences_are_trimmed() {
        assert_eq!(first_sentence("Mugger arrested."), "Mugger arrested.");
        assert_eq!(first_sentence("Good work Stern - That'll be it. And more."), "Good work Stern - That'll be it.");
        assert!(first_sentence(&"x".repeat(80)).ends_with("..."));
    }

    #[test]
    fn badge_stats_cover_all_four() {
        let mut seen = [false; 4];
        for i in 0..40 {
            seen[badge_stat(i)] = true;
        }
        assert_eq!(seen, [true; 4]);
    }
}
