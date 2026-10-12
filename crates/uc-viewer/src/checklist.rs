//! A mission's checklist: its objectives (from the briefing), side objectives and badges,
//! with boxes ticked as they're done. Shown on the menu and, holding Tab, in a mission.

use uc_render::Overlay;

pub struct Item {
    pub text: String,
    pub done: bool,
}

/// Splits a briefing into the story and its objectives: the lines starting with "-" (story
/// missions; the lines are separated by bare carriage returns), or else its last sentence
/// (training missions).
pub fn split_briefing(briefing: &str) -> (String, Vec<String>) {
    let lines: Vec<&str> = briefing.split(['\r', '\n']).map(str::trim).filter(|l| !l.is_empty()).collect();
    let objectives: Vec<String> = lines
        .iter()
        .filter(|l| l.starts_with('-'))
        .map(|l| l.trim_start_matches('-').trim().to_string())
        .collect();
    let story = lines.iter().filter(|l| !l.starts_with('-')).copied().collect::<Vec<_>>().join(" ");
    if !objectives.is_empty() {
        return (story, objectives);
    }
    // The last sentence.
    let body = story.trim_end_matches(['.', '!']);
    let start = body.rfind(['.', '!', '?']).map_or(0, |k| k + 1);
    let last = story[start..].trim().to_string();
    let rest = story[..start].trim().to_string();
    (rest, if last.is_empty() { Vec::new() } else { vec![last] })
}

/// The checklist for a mission: its objectives (done once it's won), each side objective,
/// and the badges (done when all are found).
pub fn items(objectives: &[String], won: bool, goals: &uc_game::goals::Goals, badges: usize,
             side_done: impl Fn(usize) -> bool) -> Vec<Item> {
    let mut v: Vec<Item> = objectives.iter().map(|o| Item { text: o.clone(), done: won }).collect();
    for g in &goals.sides {
        v.push(Item { text: format!("Side: {}", g.label), done: side_done(g.id) });
    }
    if !goals.badges.is_empty() {
        v.push(Item { text: format!("Badges found: {badges} of {}", goals.badges.len()), done: badges == goals.badges.len() });
    }
    v
}

/// A tick box at (x, y), `size` pixels across.
fn checkbox(o: &mut Overlay, x: f32, y: f32, size: f32, s: f32, done: bool) {
    let (edge, gold) = ([200, 200, 190, 230], [255, 210, 70, 255]);
    let t = s.max(1.0);
    o.rect(x, y, size, t, edge);
    o.rect(x, y + size - t, size, t, edge);
    o.rect(x, y, t, size, edge);
    o.rect(x + size - t, y, t, size, edge);
    if done {
        // A tick.
        o.rect(x + 1.5 * t, y + size * 0.5, t * 1.5, t * 1.5, gold);
        o.rect(x + 2.5 * t, y + size * 0.65, t * 1.5, t * 1.5, gold);
        o.rect(x + 3.5 * t, y + size * 0.45, t * 1.5, t * 1.5, gold);
        o.rect(x + 4.5 * t, y + size * 0.25, t * 1.5, t * 1.5, gold);
    }
}

/// Lines `items` would take in `width` pixels at scale `s`.
pub fn height(items: &[Item], width: f32, s: f32) -> f32 {
    items.iter().map(|i| Overlay::wrap(&i.text, width - 14.0 * s, s).len()).sum::<usize>() as f32 * 11.0 * s
}

/// Draws `items` from (x, y), each with its box, wrapping to `width`. Returns the height used.
pub fn draw(o: &mut Overlay, x: f32, y: f32, width: f32, s: f32, items: &[Item]) -> f32 {
    let line = 11.0 * s;
    let mut at = y;
    for i in items {
        checkbox(o, x, at, 8.0 * s, s, i.done);
        let col = if i.done { [120, 230, 120, 255] } else { [215, 215, 205, 255] };
        for l in Overlay::wrap(&i.text, width - 14.0 * s, s) {
            o.text(x + 14.0 * s, at, s, col, &l);
            at += line;
        }
    }
    at - y
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn briefings_split_into_story_and_objectives() {
        // As in urban.sty: the bullet lines are separated by bare carriage returns.
        let (story, o) = split_briefing("That's all. \r  \r - Bring the car back to the district station.\r - Arrest at least one mugger.");
        assert_eq!(story, "That's all.");
        assert_eq!(o, ["Bring the car back to the district station.", "Arrest at least one mugger."]);
        let (_, o) = split_briefing("hang our asses out to dry. \r \r -Bring down the suicide threat.");
        assert_eq!(o, ["Bring down the suicide threat."]);
        let (story, o) = split_briefing("The Silver award pits you against waves. You must defeat each wave within the time limits.");
        assert_eq!((story.as_str(), o[0].as_str()), ("The Silver award pits you against waves.", "You must defeat each wave within the time limits."));
        let (story, o) = split_briefing("Complete 3 laps within the allotted time.");
        assert_eq!((story.as_str(), o[0].as_str()), ("", "Complete 3 laps within the allotted time."));
    }
}
