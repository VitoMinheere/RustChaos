//! Fly-bys and conversations, and the camera they take over.

use crate::*;

impl Game {
    /// A filmed conversation is going on: the player is held still until it's over.
    pub fn in_conversation(&self) -> bool {
        self.convo.as_ref().is_some_and(|c| c.filmed())
    }

    /// The fly-by camera, or a filmed conversation's, if one is running: (position, looking
    /// at), world units.
    pub fn camera(&self) -> Option<(Vec3, Vec3)> {
        self.cutscene.as_ref().map(|c| (c.pos, self.look_at(c.shot.target)))
            .or_else(|| self.convo.as_ref().filter(|c| c.filmed()).and_then(|c| c.cam))
    }

    /// Where a fly-by's target is now (`EWAY_process_camera`, fallen/Source/eway.cpp).
    pub(crate) fn look_at(&self, look: script::Look) -> Vec3 {
        use script::{Look, Query};
        match look {
            Look::Place(at) => at,
            // A person's chest rather than their feet.
            Look::Thing { way, at } => self.thing(way).map_or(at, |t| t.pos + Vec3::new(0.0, 160.0, 0.0)),
            Look::Near(at) => self.people.iter()
                .filter(|p| !p.removed && !p.is_dead() && (p.fig.pos - at).length() < 1280.0)
                .min_by(|a, b| (a.fig.pos - at).length().total_cmp(&(b.fig.pos - at).length()))
                .map_or(at, |p| p.fig.pos),
        }
    }

    /// Skips the rest of a fly-by (everything it would have triggered still happens).
    pub fn skip_cutscene(&mut self) {
        let Some(c) = self.cutscene.take() else { return };
        self.keep_skip();
        if let Some(mut script) = self.script.take() {
            for &(way, ..) in &c.shot.waypoints[c.leg.min(c.shot.waypoints.len())..] {
                script.camera_reached(way, self);
            }
            self.script = Some(script);
        }
    }

    /// What to call the player.
    pub fn player_name(&self) -> &'static str {
        self.player_name
    }

    /// Fly-bys and conversations move on.
    /// `person_ok_for_conversation`: on their feet, not driving, climbing or in the air.
    pub(crate) fn can_talk(&self, i: usize) -> bool {
        let p = &self.people[i];
        !p.removed && !p.is_dead() && !p.is_down()
            && !matches!(p.state, State::InCar { .. } | State::Climb { .. } | State::Fall { .. } | State::Zip { .. } | State::Fence { .. } | State::PullUp { .. })
    }

    /// Person `i`'s face (`PANEL_new_face`).
    pub(crate) fn face(&self, i: usize) -> u8 {
        let p = &self.people[i];
        talk::face_of(p.kind, p.fig.model.person.saturating_sub(6), self.estate)
    }

    pub(crate) fn update_scenes(&mut self, dt: f32, level: &Level, pressed: bool) {
        if let Some(c) = self.cutscene.as_mut() {
            let (reached, done) = c.update(dt);
            if done {
                self.cutscene = None;
            }
            if let Some(mut script) = self.script.take() {
                for way in reached {
                    script.camera_reached(way, self);
                }
                self.script = Some(script);
            }
        }
        if let Some(mut cv) = self.convo.take() {
            // Anything happening to either of them breaks it off; it counts as said.
            if let (Some((i, j)), true) = (cv.pair, cv.filmed()) {
                // Sitting or posing (hands up, a hostage) they can still talk.
                let busy = |k: usize| !matches!(self.people[k].state, State::Move | State::Sit | State::Pose { .. }) || !self.can_talk(k);
                if busy(i) || busy(j) {
                    self.wide = None;
                    if let Some(s) = &mut self.script {
                        s.conversation_over(cv.way);
                    }
                    return;
                }
            }
            cv.left -= dt;
            cv.skip -= dt;
            // A press cuts the line short, once the line has been up for half a second (a
            // second for the first); a press before that means the next one does.
            if pressed && cv.filmed() {
                if cv.skip <= 0.0 {
                    cv.left = 0.0;
                } else {
                    cv.skip = 0.0;
                }
            }
            if cv.left <= 0.0 {
                cv.line = cv.line.wrapping_add(1);
                match cv.lines.get(cv.line).cloned() {
                    Some((who, text)) => {
                        // Two seconds, and a twenty-fifth for every letter (`EWAY_conv_timer`).
                        cv.left = 2.0 + text.chars().count() as f32 * 0.04;
                        cv.skip = 0.5;
                        let speaker = if who == 0 { self.player } else { self.by_way.get(&who).copied() };
                        let face = speaker.map_or(talk::face::RADIO, |i| self.face(i));
                        let at = self.player().map_or(Vec3::ZERO, |p| p.fig.pos);
                        self.sounds.push((Sfx::ConversationLine(cv.speech, cv.line), at));
                        match (cv.pair, cv.filmed()) {
                            (Some((i, j)), true) => {
                                let (top, bottom) = (i.min(j), i.max(j));
                                let top_talking = speaker == Some(top);
                                self.messages.push(Message { text: text.clone(), left: cv.left, face: Some(face), filmed: true });
                                self.wide = Some(Wide { top: Some(self.face(top)), bottom: Some(self.face(bottom)), text, top_talking });
                                // The camera on whoever has the line, over the other one.
                                let (s, l) = if speaker == Some(j) { (j, i) } else { (i, j) };
                                let (sp, sa, lp) = (self.people[s].fig.pos, self.people[s].fig.angle, self.people[l].fig.pos);
                                let mut rng = self.rngs.main;
                                let mut rand = || rand(&mut rng);
                                let eye = Vec3::new(0.0, 0.0, 0.0);
                                cv.cam = Some(talk::converse_camera(sp, sa, lp, |a, b| level.ground.line_of_sight(a + eye, b), &mut rand));
                                self.rngs.main = rng;
                            }
                            _ => self.messages.push(Message { text, left: cv.left, face: Some(face), filmed: false }),
                        }
                    }
                    None => {
                        self.wide = None;
                        if let Some(s) = &mut self.script {
                            s.conversation_over(cv.way);
                        }
                        return;
                    }
                }
            }
            self.convo = Some(cv);
        }
    }
}
