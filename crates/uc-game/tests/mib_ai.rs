//! The Men in Black's AI (#242, `PCOM_AI_MIB`): they notice the player by seeing her, not
//! just by her being near, and when one does, everyone near him who'd fight comes for her
//! (`PCOM_alert_nearby_mib_to_attack`). Two of RTA's crowd made into a Man in Black and a
//! guard. Skipped without an install.

use glam::vec3;
use uc_game::person::{AI_CIV, AI_GUARD, AI_MIB};
use uc_game::Controls;
use uc_world::character::person as ptype;

mod common;
use common::Run;

impl Run {
    /// Person `i` made a still `ai` of `kind`, `ahead` in front of Darci, facing `towards`
    /// her or away.
    fn post(&mut self, i: usize, ai: u8, kind: u8, ahead: f32, side: f32, towards: bool) {
        let p = self.player();
        let me = self.g.people[p].fig.pos;
        let f = self.g.people[p].facing_dir();
        let at = me + vec3(f.x, 0.0, f.y) * ahead + vec3(-f.y, 0.0, f.x) * side;
        let y = self.level.ground.surface(&self.level.map, at.x, at.z, at.y + 64.0);
        let angle = self.g.people[p].fig.angle + if towards { std::f32::consts::PI } else { 0.0 };
        let q = &mut self.g.people[i];
        (q.ai, q.kind, q.mv, q.fake, q.bent) = (ai, kind, 1, false, 0);
        q.fig.pos = vec3(at.x, y, at.z);
        q.fig.angle = angle;
    }

    fn crowd(&self, n: usize) -> Vec<usize> {
        self.g.people.iter().enumerate().filter(|(_, q)| q.fake && q.ai == AI_CIV && !q.is_dead()).map(|(i, _)| i).take(n).collect()
    }
}

/// RTA, with Darci turned to look down an open stretch: clear 700 ahead, and 300 to the side
/// of that.
fn start() -> Option<Run> {
    let mut r = common::start("police1")?;
    r.play(1.0, Controls::default());
    let p = r.player();
    let me = r.g.people[p].fig.pos;
    let eye = vec3(0.0, 96.0, 0.0);
    let open = (0..16).map(|k| k as f32 * std::f32::consts::TAU / 16.0).find(|&a| {
        let f = glam::vec2(-a.sin(), -a.cos());
        let ahead = me + vec3(f.x, 0.0, f.y) * 700.0;
        let side = me + vec3(f.x, 0.0, f.y) * 600.0 + vec3(-f.y, 0.0, f.x) * 300.0;
        r.level.ground.line_of_sight(me + eye, ahead + eye) && r.level.ground.line_of_sight(me + eye, side + eye)
    }).expect("nowhere open round her");
    r.g.people[p].fig.angle = open;
    Some(r)
}

#[test]
fn a_man_in_black_with_his_back_to_her_doesnt_notice() {
    let Some(mut r) = start() else { return };
    let i = r.crowd(1)[0];
    r.post(i, AI_MIB, ptype::MIB1, 600.0, 0.0, false);
    r.play(1.0, Controls::default());
    assert!(!r.g.people[i].aggro(), "he can't see her behind him");
}

#[test]
fn one_who_sees_her_calls_the_others_in() {
    let Some(mut r) = start() else { return };
    let c = r.crowd(2);
    let (mib, guard) = (c[0], c[1]);
    r.post(mib, AI_MIB, ptype::MIB1, 600.0, 0.0, true);
    // Beside him, looking the other way: he wouldn't see her himself.
    r.post(guard, AI_GUARD, ptype::THUG_RED, 600.0, 300.0, false);
    r.play(0.5, Controls::default());
    assert!(r.g.people[mib].aggro(), "he should see her");
    assert!(r.g.people[guard].aggro(), "the guard beside him should be called in");
}
