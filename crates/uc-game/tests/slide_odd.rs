//! The emergency jump (`SlideOdd`, pcom.cpp:12428): someone on their way somewhere who has
//! slid along a prop for 50 turns in a row takes a running jump. In RTA, a follower runs
//! head-on into a prop between them and Darci, too tall to step over but low enough to jump
//! (a jump at a lamp post doesn't get them past it). Skipped without an install.

use glam::{vec2, vec3, Vec2};
use uc_game::{Controls, Game, State};
use uc_world::mav::Mav;
use uc_world::{Level, LoadOptions};

const DT: f32 = 1.0 / 30.0;

#[test]
fn a_follower_stuck_on_a_lamp_post_jumps() {
    let Some(game) = uc_formats::default_install() else { return };
    let level = Level::load(&game, "police1", &LoadOptions { fullbright: true, character: true, overrides: None }).unwrap();
    let m = Mav::new(&level);
    let g0 = &level.ground;
    let sq = |v: Vec2| ((v.x / 256.0) as u8, (v.y / 256.0) as u8);
    // Props too tall to step over but low enough to jump (a bin, a bench), standing on the
    // street, with open street either side of them along x.
    let mut posts = Vec::new();
    for (_, r) in &g0.prop_walls {
        let ws = &g0.walls[r.clone()];
        let mid = ws.iter().map(|w| Vec2::from(w.a)).sum::<Vec2>() / ws.len() as f32;
        let y = level.map.ground_height(mid.x, mid.y);
        let (from, to) = (mid - vec2(600.0, 0.0), mid + vec2(800.0, 0.0));
        let level_street = [from, mid, to].iter().all(|p| (g0.surface(&level.map, p.x, p.y, y + 32.0) - y).abs() < 8.0);
        if ws[0].y0 > y + 16.0 || !(y + 100.0..y + 170.0).contains(&ws[0].y1) || !level_street || !m.can_walk(sq(from), sq(to)) {
            continue;
        }
        // Nothing else in the way.
        let eye = vec3(0.0, 100.0, 0.0);
        let clear = |a: Vec2, b: Vec2| g0.line_of_sight(vec3(a.x, y, a.y) + eye, vec3(b.x, y, b.y) + eye);
        if clear(from, mid - vec2(120.0, 0.0)) && clear(mid + vec2(120.0, 0.0), to) {
            posts.push((from, to, y));
        }
    }
    println!("{} posts", posts.len());
    assert!(!posts.is_empty());
    let (mut stuck, mut past) = (0, 0);
    for &(from, to, y) in &posts {
        let mut g = Game::new(&level, true);
        for _ in 0..(60.0 / DT) as usize {
            g.skip_cutscene();
            g.update(DT, &Controls::default(), &level);
            if g.time > 1.0 && !g.in_conversation() {
                break;
            }
        }
        let p = g.player.unwrap();
        let Some(i) = g.people.iter().position(|q| q.fake && !q.is_dead()) else { return };
        for (k, q) in g.people.iter_mut().enumerate() {
            q.removed |= k != p && k != i;
        }
        g.people[p].fig.pos = vec3(to.x, y, to.y);
        g.people[i].fig.pos = vec3(from.x, y, from.y);
        g.people[p].state = State::Move;
        g.people[i].state = State::Move;
        let q = &mut g.people[i];
        (q.fake, q.mv, q.follow) = (false, 5, 0);
        let (mut most, mut jumped, mut there) = (0, false, false);
        for _ in 0..(6.0 / DT) as usize {
            g.update(DT, &Controls::default(), &level);
            let q = &g.people[i];
            most = most.max(q.slide_odd);
            assert!(q.slide_odd <= 51, "slid {} turns without jumping", q.slide_odd);
            jumped |= most >= 50 && matches!(q.state, State::Fall { .. });
            // Past it: within the two squares a follower stops at.
            if (q.fig.pos - g.people[p].fig.pos).length() < 2.1 * 256.0 {
                there = true;
                break;
            }
        }
        println!("prop between {from} and {to}: slid for {most} turns, jumped {jumped}, got past {there}");
        if most >= 50 {
            stuck += 1;
            past += (jumped && there) as usize;
        }
    }
    assert!(stuck > 5, "only {stuck} got stuck on a prop");
    assert!(past * 4 >= stuck * 3, "{past} of {stuck} stuck jumped past");
}
