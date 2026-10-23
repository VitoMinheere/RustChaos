//! Switches (`ANIM_PRIM_TYPE_SWITCH`, fallen/Source/Prim.cpp): anim prim 4 placed on the map,
//! which a `switch` point watches (`EWAY_COND_SWITCH`, eway.cpp:1293).
//!
//! The shipped game never turns one on. The player's action used to flip the nearest switch
//! (interfac.cpp:1617), but that's `#if 0`'d out ("We don't use anim_prim_switches any
//! more"), and nothing else plays a switch's animation, so a `switch` point never fires.
//! Only two unfinished missions use them (Takeover1 and subdock1), to open doors, start a
//! platform and film a camera shot. Here the cut action is back: action near a switch flips
//! it. Its model is gone (`anim004.all` became Bane), so it flips at once rather than at the
//! end of its animation, and nothing is drawn.

use glam::{vec3, Vec3};
use uc_world::Level;

/// `MAP_THING_TYPE_ANIM_PRIM` (Editor/Headers/Thing.h).
const ANIM_PRIM: u16 = 8;
/// The anim prim that's a switch (`get_anim_prim_type`, Prim.cpp:2219).
const SWITCH: u16 = 4;
/// How near a `switch` point a switch can be (`find_anim_prim`'s range, Manhattan).
const FIND_RANGE: f32 = 0x200 as f32;
/// How near the player's middle a switch they flip has to be (interfac.cpp:1624).
const REACH: f32 = 0x100 as f32;

/// A switch: where it is, and whether it's on (`FLAGS_SWITCHED_ON`).
#[derive(Debug, Clone, Copy)]
pub struct Switch {
    pub pos: Vec3,
    pub on: bool,
}

/// Every switch on the map.
#[derive(Debug, Clone, Default)]
pub struct Switches(pub Vec<Switch>);

/// `find_anim_prim`: the nearest of `switches` to `at` within `range`, Manhattan.
fn nearest(switches: &[Switch], at: Vec3, range: f32) -> Option<usize> {
    let dist = |s: &Switch| (s.pos - at).abs().element_sum();
    (0..switches.len()).filter(|&i| dist(&switches[i]) <= range).min_by(|&a, &b| dist(&switches[a]).total_cmp(&dist(&switches[b])))
}

impl Switches {
    pub fn new(level: &Level) -> Switches {
        let things = level.map.things.iter().filter(|t| t.kind == ANIM_PRIM && t.index_other == SWITCH);
        Switches(things.map(|t| Switch { pos: vec3(t.x as f32, t.y as f32, t.z as f32), on: false }).collect())
    }

    /// Someone standing at `feet` pressed action: the nearest switch within reach of their
    /// middle flips (`toggle_anim_prim_switch_state`). Returns whether one did.
    pub fn flip_near(&mut self, feet: Vec3) -> bool {
        let Some(i) = nearest(&self.0, feet + vec3(0.0, 0x40 as f32, 0.0), REACH) else { return false };
        self.0[i].on ^= true;
        true
    }

    /// Whether the switch a `switch` point at `pos` watches is on: the nearest within range.
    /// With none, never.
    pub fn on_near(&self, pos: Vec3) -> bool {
        nearest(&self.0, pos, FIND_RANGE).is_some_and(|i| self.0[i].on)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn flips_the_nearest_in_reach() {
        let mut s = Switches(vec![Switch { pos: vec3(0.0, 64.0, 0.0), on: false }, Switch { pos: vec3(200.0, 64.0, 0.0), on: false }]);
        assert!(!s.flip_near(vec3(300.0, 0.0, 300.0)), "out of reach");
        assert!(s.flip_near(vec3(150.0, 0.0, 0.0)));
        assert!(!s.0[0].on && s.0[1].on);
        // A point watches the nearest within its range, on or off.
        assert!(s.on_near(vec3(250.0, 0.0, 0.0)));
        assert!(!s.on_near(vec3(-50.0, 0.0, 0.0)));
        assert!(!s.on_near(vec3(5000.0, 0.0, 0.0)));
        s.flip_near(vec3(150.0, 0.0, 0.0));
        assert!(!s.0[1].on, "flips back off");
    }
}
