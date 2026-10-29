//! Guns: picking up weapons and ammo, switching, and a bot that fights through Combat Gold
//! with an AK-47 using the targeting (no cheating: the shots decide, though Darci is kept
//! healthy). Skipped without an install.

use glam::{vec2, Vec2};
use uc_game::{Controls, Game, Item, Outcome, State, Weapon};
use uc_world::{Level, LoadOptions};

const DT: f32 = 1.0 / 30.0;

fn load(mission: &str) -> Option<(Level, Game)> {
    let game = uc_formats::default_install()?;
    let level = Level::load(&game, mission, &LoadOptions { fullbright: true, character: true, overrides: None }).unwrap();
    let g = Game::new(&level, true);
    Some((level, g))
}

#[test]
fn pick_up_guns_and_ammo_and_switch() {
    let Some((level, mut g)) = load("fight2") else { return };
    for _ in 0..30 {
        g.skip_cutscene();
        g.update(DT, &Controls::default(), &level);
    }
    let pi = g.player.unwrap();
    let drop = |g: &mut Game, kind: u8| {
        let pos = g.people[pi].fig.pos;
        g.items.push(Item { way: 0, kind, pos, rounds: None, born: 0.0 });
        for _ in 0..5 {
            g.update(DT, &Controls::default(), &level);
        }
    };
    // A pistol: carried, loaded and drawn (the hands were empty).
    drop(&mut g, 2);
    let p = &g.people[pi];
    assert_eq!((p.weapon, p.rounds()), (Weapon::Pistol, Some(15)));
    // Pistol ammo adds a clip; a shotgun is carried but not drawn.
    drop(&mut g, 10);
    drop(&mut g, 4);
    let p = &g.people[pi];
    assert_eq!((p.weapon, p.rounds()), (Weapon::Pistol, Some(30)));
    assert!(p.has(Weapon::Shotgun));
    assert!(!g.items.iter().any(|it| matches!(it.kind, 2 | 4 | 10)));
    // Switching: pistol -> shotgun -> bare hands -> pistol.
    let switch = Controls { switch: true, ..Default::default() };
    let mut order = Vec::new();
    for _ in 0..3 {
        g.update(DT, &switch, &level);
        g.update(DT, &Controls::default(), &level);
        order.push(g.people[pi].weapon);
    }
    assert_eq!(order, [Weapon::Shotgun, Weapon::None, Weapon::Pistol]);
    // Once it's out, firing at nothing uses a round.
    while matches!(g.people[pi].state, uc_game::State::Draw) {
        g.update(DT, &Controls::default(), &level);
    }
    let fire = Controls { punch: true, fire: true, ..Default::default() };
    g.update(DT, &fire, &level);
    for _ in 0..20 {
        g.update(DT, &Controls::default(), &level);
    }
    assert_eq!(g.people[pi].rounds(), Some(29));
}

/// Bare-handed, the weapon key first takes out the best weapon carried
/// (`CONTROLS_get_best_item`): a loaded gun before a knife. Pressed again while the inventory
/// is still up, it goes on through the list; and an empty gun doesn't count as best.
#[test]
fn bare_handed_the_best_weapon_comes_out_first() {
    let Some((level, mut g)) = load("fight2") else { return };
    for _ in 0..30 {
        g.skip_cutscene();
        g.update(DT, &Controls::default(), &level);
    }
    let pi = g.player.unwrap();
    g.people[pi].give(Weapon::Knife);
    g.people[pi].give(Weapon::Pistol);
    g.people[pi].weapon = Weapon::None;
    let switch = |g: &mut Game| {
        g.update(DT, &Controls { switch: true, ..Default::default() }, &level);
        g.update(DT, &Controls::default(), &level);
        g.people[pi].weapon
    };
    let wait = |g: &mut Game| {
        for _ in 0..(3.5 / DT) as usize {
            g.update(DT, &Controls::default(), &level);
        }
    };
    // The pistol, though the knife comes next in the list.
    assert_eq!(switch(&mut g), Weapon::Pistol);
    assert!(g.people[pi].inventory > 2.5, "the inventory shows");
    // On through the list while it's up: bare hands, then the knife.
    assert_eq!(switch(&mut g), Weapon::None);
    assert_eq!(switch(&mut g), Weapon::Knife);
    wait(&mut g);
    assert_eq!(g.people[pi].inventory, 0.0);
    // Empty, the pistol isn't the best: bare-handed again, the knife.
    g.people[pi].ammo = [0; 4];
    assert_eq!(switch(&mut g), Weapon::Pistol);
    assert_eq!(switch(&mut g), Weapon::None);
    wait(&mut g);
    assert_eq!(switch(&mut g), Weapon::Knife);
}

#[test]
fn bot_shoots_through_combat_gold_with_an_ak() {
    let Some((level, mut g)) = load("fight2") else { return };
    let pi = g.player.unwrap();
    g.people[pi].give(Weapon::Ak47);
    g.people[pi].weapon = Weapon::Ak47;
    let (mut targeted, mut fired, mut downed_by_gun) = (0, 0, 0);
    let mut dropped = Vec::new();
    let mut seen = Vec::new();
    let (mut last_pos, mut stuck) = (Vec2::ZERO, 0.0f32);
    // The bot can't find its way round buildings, so it doesn't finish: two minutes is
    // plenty to see the waves of enemies shot down.
    for frame in 0..(120.0 / DT) as usize {
        g.skip_cutscene();
        // Darci is tough today, and never runs dry.
        let p = &mut g.people[pi];
        p.health = p.max_health;
        p.ammo[2] = p.ammo[2].max(30);
        let pos = vec2(p.fig.pos.x, p.fig.pos.z);
        let mut c = Controls::default();
        let last = std::mem::replace(&mut last_pos, pos);
        if g.target.is_some() {
            targeted += 1;
            // Bursts of fire.
            c.punch = frame % 20 == 0;
            c.fire = frame % 20 < 10;
        } else if let Some(e) = g.people.iter().filter(|e| !e.player && !e.is_dead() && !e.is_down() && e.hostile())
            .map(|e| vec2(e.fig.pos.x, e.fig.pos.z)).min_by(|a, b| (*a - pos).length().total_cmp(&(*b - pos).length()))
        {
            // Nobody in sight: walk towards the nearest enemy, jumping onto (and so climbing)
            // anything in the way.
            c.wish = (e - pos).normalize_or_zero();
            c.run = (e - pos).length() > 1500.0;
            stuck = if (pos - last).length() < 2.0 { stuck + DT } else { 0.0 };
            c.jump = stuck > 0.5 && frame % 20 == 0;
            // Arrest anyone shot down on the way.
            c.action = frame % 15 == 0;
        } else {
            c.action = frame % 15 == 0;
            // Walk to anyone lying down, to arrest them.
            if let Some(d) = g.people.iter().filter(|e| !e.player && e.is_down() && !e.is_dead())
                .map(|e| vec2(e.body_pos().x, e.body_pos().z)).min_by(|a, b| (*a - pos).length().total_cmp(&(*b - pos).length()))
            {
                c.wish = (d - pos).normalize_or_zero() * if (d - pos).length() < 60.0 { 0.0 } else { 1.0 };
            }
        }
        let before: Vec<bool> = g.people.iter().map(|e| e.is_down() || e.is_dead()).collect();
        let ammo = g.people[pi].ammo[2];
        g.update(DT, &c, &level);
        fired += (ammo - g.people[pi].ammo[2]) as usize;
        for (i, e) in g.people.iter().enumerate() {
            if i < before.len() && !before[i] && e.is_down() && matches!(e.state, State::Down) {
                downed_by_gun += 1;
            }
        }
        for it in &g.items {
            if Weapon::from_item(it.kind).is_some_and(|w| w.is_gun()) && !dropped.contains(&it.kind) {
                dropped.push(it.kind);
            }
        }
        for m in &g.messages {
            if !seen.contains(&m.text) {
                println!("[{:6.1}s] {}", g.time, m.text);
                seen.push(m.text.clone());
            }
        }
        if g.outcome.is_some() {
            break;
        }
    }
    let p = &g.people[pi];
    println!("targeted {targeted} frames, fired {fired}, downed {downed_by_gun}, guns dropped {dropped:?}, carrying {:?} {:?}, outcome {:?} at {:.0}s",
             p.guns, p.ammo, g.outcome, g.time);
    assert!(targeted > 30 && fired > 10, "the gun was hardly used");
    assert!(downed_by_gun >= 5, "only {downed_by_gun} enemies shot down");
    // Combat Gold's gunmen keep their guns when they go down, until they're searched (the
    // bot never holds action long enough to search anyone).
    let armed_down = g.people.iter().filter(|e| !e.player && (e.is_down() || e.is_dead()) && e.weapon.is_gun()).count();
    assert!(armed_down > 0, "no gunman went down still holding his gun");
    assert_ne!(g.outcome, Some(Outcome::Lost("Darci was knocked out".into())));
}
