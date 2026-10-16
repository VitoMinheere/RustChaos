//! Searching props, in Auto-Destruct (`carbomb1`): once the bombers are arrested, point 181
//! stashes a UCNN video in one of the props at the Wildcat den, and finding it (point 187's
//! `item_held`) is what leads on. Five `make_searchable` points mark the den's other props,
//! which have nothing in them. Skipped without an install.

use glam::{vec3, Vec3};
use uc_formats::ucm::trigger;
use uc_game::{Controls, Search};

mod common;
use common::{Run, DT};

/// Auto-Destruct with the video stashed from the start: point 181 waits on the bombers being
/// arrested, which is a long way into the mission and nothing to do with searching.
fn start() -> Option<Run> {
    let mut level = common::load("carbomb1")?;
    level.mission.event_points.iter_mut().find(|e| e.index == 181).unwrap().trigger = trigger::NONE;
    let mut r = Run::new(level);
    assert!(r.g.unsupported().is_empty(), "unsupported: {:?}", r.g.unsupported());
    r.stop_at_outcome = false;
    r.play(1.0, Controls::default());
    Some(r)
}

impl Run {
    /// Darci a little way off prop `ob`, facing it.
    fn face(&mut self, ob: usize) {
        let o = self.level.map.objects[ob];
        let at = vec3(o.x as f32, o.y as f32, o.z as f32);
        let from = at + Vec3::new(110.0, 0.0, 0.0);
        let y = self.level.ground.surface(&self.level.map, from.x, from.z, from.y + 64.0);
        let i = self.g.player.unwrap();
        self.g.people[i].fig.pos = vec3(from.x, y, from.z);
        self.g.people[i].face_towards(at);
    }

    /// Presses action and keeps it held (the viewer passes "held" on as `sprint`).
    fn search(&mut self, secs: f32) {
        self.play(DT, Controls { action: true, sprint: true, ..Default::default() });
        self.play(secs, Controls { sprint: true, ..Default::default() });
    }

    fn video(&self) -> Option<Vec3> {
        self.g.items.iter().find(|it| it.way == 181).map(|it| it.pos)
    }
}

#[test]
fn the_video_turns_up_when_its_prop_is_searched() {
    let Some(mut r) = start() else { return };
    assert_eq!(r.video(), None, "the video should be hidden, not lying about");
    let &(ob, ref item) = r.g.hidden.iter().find(|(_, it)| it.way == 181).expect("the video, stashed");
    assert_eq!(item.kind, 19, "a video");
    assert!(r.g.searchable.len() >= 2, "the den's props should be searchable: {:?}", r.g.searchable);

    r.face(ob);
    r.search(Search::FOUND + 0.2);
    let at = r.video().expect("the search should turn up the video");
    assert!(r.g.hidden.is_empty(), "nothing else was stashed");

    // It is picked up like anything else, which is what point 187 is watching for.
    r.play(Search::DONE, Controls::default());
    assert!(r.g.search.is_none(), "the search should be over");
    let i = r.g.player.unwrap();
    r.g.people[i].fig.pos = at;
    r.play(1.0, Controls::default());
    assert!(r.seen.iter().any(|m| m.contains("UCNN video")), "the video was never lifted: {:?}", r.seen);
}

#[test]
fn letting_go_of_action_abandons_the_search() {
    let Some(mut r) = start() else { return };
    let ob = r.g.hidden[0].0;
    r.face(ob);
    r.search(Search::FOUND / 2.0);
    assert!(r.g.search.is_some(), "a search should be going on");
    r.play(DT * 2.0, Controls::default());
    assert!(r.g.search.is_none(), "letting go should stop it");
    assert_eq!(r.video(), None, "half a search finds nothing");

    // Nor does turning her back on the prop start one.
    let o = r.level.map.objects[ob];
    let i = r.g.player.unwrap();
    let away = 2.0 * r.g.people[i].fig.pos - vec3(o.x as f32, o.y as f32, o.z as f32);
    r.g.people[i].face_towards(away);
    r.search(0.5);
    assert!(r.g.search.is_none(), "searching a prop behind her");
}

/// A prop with nothing in it can be searched to the end, and gives nothing.
#[test]
fn an_empty_prop_gives_nothing() {
    let Some(mut r) = start() else { return };
    let stash = r.g.hidden[0].0;
    let Some(&empty) = r.g.searchable.iter().find(|&&o| o != stash) else { return };
    let items = r.g.items.len();
    r.face(empty);
    r.search(Search::DONE + 0.2);
    assert_eq!(r.g.items.len(), items, "an empty prop gave something up");
    assert_eq!(r.g.hidden.len(), 1, "the video should still be hidden");
}
