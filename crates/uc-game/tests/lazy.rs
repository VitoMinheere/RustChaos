//! The lazy (#246, `PCOM_BENT_LAZY`): standing about at home, they find a bench, a sofa or
//! an armchair within 0x200 and sit on it, and get up again for a fight; the rest don't sit.
//! One of RTA's crowd by one of its benches, with Darci well away. Skipped without an install.

use glam::vec3;
use uc_game::person::{AI_CIV, BENT_LAZY};
use uc_game::{Controls, State};
use uc_world::character::anim;

mod common;
use common::Run;

/// RTA with one of its crowd standing still beside a park bench (`PRIM_OBJ_PARK_BENCH`), with
/// personality `bent`, and Darci thirty squares off. Returns the person and the bench.
fn by_a_bench(bent: u8) -> Option<(Run, usize, glam::Vec3)> {
    let mut r = common::start("police1")?;
    r.play(1.0, Controls::default());
    let bench = r.level.map.objects.iter().find(|o| o.prim == 89).expect("RTA has benches");
    let at = vec3(bench.x as f32, bench.y as f32, bench.z as f32);
    let i = r.g.people.iter().position(|q| q.fake && q.ai == AI_CIV && !q.is_dead()).expect("crowd");
    let y = r.level.ground.surface(&r.level.map, at.x + 150.0, at.z, at.y + 64.0);
    let q = &mut r.g.people[i];
    (q.mv, q.fake, q.bent) = (1, false, bent);
    q.fig.pos = vec3(at.x + 150.0, y, at.z);
    let pi = r.player();
    r.g.people[pi].fig.pos = at + vec3(30.0 * 256.0, 0.0, 0.0);
    Some((r, i, at))
}

#[test]
fn the_lazy_sit_down_on_a_bench() {
    let Some((mut r, i, bench)) = by_a_bench(BENT_LAZY) else { return };
    r.play(4.0, Controls::default());
    let q = &r.g.people[i];
    assert!(matches!(q.state, State::Sit), "{:?}", q.state);
    assert_eq!(q.fig.anim(), anim::SIT_DOWN);
    assert!((q.fig.pos - bench).length() < 128.0, "sat {} from the bench", (q.fig.pos - bench).length());
    // A fight gets them up.
    let from = q.fig.pos + vec3(50.0, 0.0, 0.0);
    r.g.people[i].take_hit(from, 1, 0, false, false, uc_game::Weapon::None);
    r.play(1.0, Controls::default());
    assert!(!matches!(r.g.people[i].state, State::Sit), "still sitting after being hit");
}

#[test]
fn the_rest_stay_on_their_feet() {
    let Some((mut r, i, _)) = by_a_bench(0) else { return };
    r.play(4.0, Controls::default());
    assert!(!matches!(r.g.people[i].state, State::Sit));
}
