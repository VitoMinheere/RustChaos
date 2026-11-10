//! The move list (#547): every move the player has, with what it's called, how it's done
//! and when it works, for the viewer's MOVES page and the sparring room
//! ([`crate::game::spar`]). Not in the original, whose moves are only in its manual
//! (pp. 11-17).
//!
//! How a move is done is written with the actions' names in braces (`{punch}`,
//! `{forward}+{kick}`), which the viewer swaps for the keys or buttons they're bound to.

use uc_world::character::anim;

/// A move in the list.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Tech {
    PunchCombo,
    KickCombo,
    SideKick,
    BackKick,
    GroinKick,
    JumpKick,
    SlideTackle,
    FootSweep,
    Stomp,
    Block,
    SideFlip,
    ArmGrab,
    Knee,
    Throw,
    Takedown,
    Arrest,
    KnifeCombo,
    BatCombo,
    Shoot,
    GunClub,
    NextWeapon,
    NextTarget,
    Run,
    Sprint,
    Jump,
    Crouch,
    Ladder,
    Strafe,
    BackFlip,
    WallHug,
    Can,
    LookRound,
    EnterCar,
    Handbrake,
    Siren,
}

/// The list's sections, in order.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Group {
    Fists,
    Grapples,
    Weapons,
    About,
    Stealth,
    Cars,
}

impl Group {
    pub const ALL: [Group; 6] = [Group::Fists, Group::Grapples, Group::Weapons, Group::About, Group::Stealth, Group::Cars];

    pub fn name(self) -> &'static str {
        match self {
            Group::Fists => "FISTS AND FEET",
            Group::Grapples => "GRAPPLES AND ARRESTS",
            Group::Weapons => "WEAPONS",
            Group::About => "GETTING ABOUT",
            Group::Stealth => "STEALTH",
            Group::Cars => "CARS",
        }
    }
}

/// A row of the list.
#[derive(Debug, Clone, Copy)]
pub struct Entry {
    pub tech: Tech,
    pub name: &'static str,
    pub group: Group,
    /// How it's done, the actions in braces.
    pub input: &'static str,
    /// When it works.
    pub when: &'static str,
    /// The animation that shows it.
    pub anim: usize,
    /// Whether the sparring room can tell it's been done, and ticks it off.
    pub sparred: bool,
}

const fn e(tech: Tech, name: &'static str, group: Group, input: &'static str, when: &'static str, anim: usize, sparred: bool) -> Entry {
    Entry { tech, name, group, input, when, anim, sparred }
}

/// The whole list, a section after another.
pub const MOVES: [Entry; 35] = [
    e(Tech::PunchCombo, "PUNCH COMBO", Group::Fists, "{punch} {punch} {punch}", "Each press as the last blow lands. The third puts them down.", anim::PUNCH_COMBO3, true),
    e(Tech::KickCombo, "KICK COMBO", Group::Fists, "{kick} {kick} {kick}", "Each press as the last blow lands. The third puts them down.", anim::KICK_COMBO3, true),
    e(Tech::SideKick, "SIDE KICK", Group::Fists, "{left}+{kick}  or  {right}+{kick}", "In a fight, at someone to her side: she faces the nearest, so it takes two.", anim::KICK_LEFT, true),
    e(Tech::BackKick, "BACK KICK", Group::Fists, "{back}+{kick}", "In a fight, at someone behind her: she faces the nearest, so it takes two.", anim::KICK_BEHIND, true),
    e(Tech::GroinKick, "GROIN KICK", Group::Fists, "{forward}+{kick}", "Stepping in on someone close. Darci's alone.", anim::KICK_NAD, true),
    e(Tech::JumpKick, "JUMP KICK", Group::Fists, "{jump}", "In a fight, standing still. Nothing lands on her while she's in the air.", anim::KICK_NS, true),
    e(Tech::SlideTackle, "SLIDE TACKLE", Group::Fists, "{run}+{forward}, then {kick}", "Running. Takes the legs from under anyone in the way.", anim::SLIDER_START, true),
    e(Tech::Block, "BLOCK", Group::Fists, "hold {block}", "Stops blows from the front. Stepping back from someone swinging at her blocks too.", anim::BLOCK_HIGH, true),
    e(Tech::FootSweep, "FOOT SWEEP", Group::Fists, "hold {block}, then {kick}", "From a block. Gets under a guard and puts them down.", anim::LEG_SWEEP, true),
    e(Tech::SideFlip, "SIDE FLIP", Group::Fists, "{left}+{jump}  or  {right}+{jump}", "In a fight. Out of the way of a blow.", anim::FLIP_LEFT, true),
    e(Tech::Stomp, "STOMP", Group::Fists, "{kick}", "Standing over someone knocked down.", anim::FIGHT_STOMP, true),
    e(Tech::ArmGrab, "ARM GRAB", Group::Grapples, "{forward}+{punch}", "Walking into someone close. She holds them for five seconds.", anim::GRAB_ARM, true),
    e(Tech::Knee, "KNEE", Group::Grapples, "{punch}, and {punch} again", "Holding someone. The second knee puts them down.", anim::GRAB_ARM_KNEE1, true),
    e(Tech::Throw, "THROW", Group::Grapples, "{kick}", "Holding someone.", anim::GRAB_ARM_THROW, true),
    e(Tech::Arrest, "ARREST", Group::Grapples, "{action}", "Standing over someone knocked down. Darci's alone.", anim::ARREST_CROUTCH, true),
    e(Tech::KnifeCombo, "KNIFE COMBO", Group::Weapons, "{punch} {punch} {punch}", "With the knife out.", anim::KNIFE_ATTACK3, true),
    e(Tech::BatCombo, "BAT COMBO", Group::Weapons, "{punch} {punch}", "With the bat out.", anim::BAT_HIT2, true),
    e(Tech::Shoot, "SHOOT", Group::Weapons, "{punch}", "With a gun out, at whoever the sight is on. Hold it down with the AK-47. Works on the run.", anim::PISTOL_SHOOT, true),
    e(Tech::GunClub, "GUN CLUB", Group::Weapons, "{punch}", "With the shotgun or the AK-47 out, once someone has landed a blow on her.", anim::SHOTGUN_WHIP1, true),
    e(Tech::NextWeapon, "NEXT WEAPON", Group::Weapons, "{weapon}", "Bare hands, knife or bat, then the guns.", anim::PISTOL_DRAW, false),
    e(Tech::NextTarget, "NEXT TARGET", Group::Weapons, "{target}", "With a gun out and more than one enemy about.", anim::PISTOL_AIM, false),
    e(Tech::Run, "RUN", Group::About, "{run}+{forward}", "", anim::RUN, false),
    e(Tech::Sprint, "SPRINT", Group::About, "hold {action}", "Running, for as long as her stamina lasts.", anim::YOMP, false),
    e(Tech::Jump, "JUMP AND CLIMB", Group::About, "{jump}", "She grabs a ledge in front and pulls herself up. Jump at a cable to ride it down.", anim::STANDING_JUMP, false),
    e(Tech::Crouch, "CROUCH AND CRAWL", Group::About, "hold {crouch}", "Move to crawl.", anim::CRAWL, false),
    e(Tech::Ladder, "LADDERS AND FENCES", Group::About, "{forward}", "Walk into a ladder or a fence to climb it. Run at a low fence to vault it.", anim::ON_LADDER, false),
    e(Tech::Strafe, "SIDESTEP", Group::About, "hold {strafe}, then {left} or {right}", "{back} walks her backwards.", anim::STEP_LEFT, false),
    e(Tech::BackFlip, "BACK FLIP", Group::About, "hold {strafe}, then {back}+{jump}", "With room behind her.", anim::BACK_FLIP, false),
    e(Tech::Takedown, "TAKEDOWN", Group::Stealth, "{punch}", "Close behind someone who hasn't seen her. Out cold, and alive.", anim::PISTOL_WHIP, true),
    e(Tech::WallHug, "WALL HUG", Group::Stealth, "{action}", "Facing a building's wall. Move along it, look round its end, {action} again to leap out.", anim::STAND_READY, false),
    e(Tech::Can, "THROW A CAN", Group::Stealth, "{action}, then {punch}", "Bare-handed by a can. Guards go and look where it lands.", anim::CAN_RELEASE, false),
    e(Tech::LookRound, "LOOK ROUND", Group::Stealth, "hold {look}", "Standing still, from close behind her head.", anim::BREATHE, false),
    e(Tech::EnterCar, "IN AND OUT OF A CAR", Group::Cars, "{action}", "By a car's door, or sat in one that's stopped.", anim::ENTER_CAR, false),
    e(Tech::Handbrake, "HANDBRAKE", Group::Cars, "{handbrake}", "Slides the back out.", anim::SIT_IDLE, false),
    e(Tech::Siren, "SIREN", Group::Cars, "{siren}", "In a police car. Traffic gets out of the way.", anim::SIT_IDLE, false),
];

/// The row for `tech`.
pub fn entry(tech: Tech) -> &'static Entry {
    MOVES.iter().find(|m| m.tech == tech).unwrap_or(&MOVES[0])
}

/// How many moves the sparring room ticks off.
pub fn sparred() -> usize {
    MOVES.iter().filter(|m| m.sparred).count()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Each move once, the sections one after another in their order, and no action named
    /// that the viewer wouldn't know.
    #[test]
    fn the_list_is_whole() {
        let mut seen = std::collections::BTreeSet::new();
        for m in &MOVES {
            assert!(seen.insert(m.tech), "{:?} twice", m.tech);
            assert!(!m.name.is_empty() && !m.input.is_empty());
        }
        let groups: Vec<usize> = MOVES.iter().map(|m| Group::ALL.iter().position(|&g| g == m.group).unwrap()).collect();
        assert!(groups.windows(2).all(|w| w[0] <= w[1]), "the sections are out of order");
        const ACTIONS: [&str; 16] = ["forward", "back", "left", "right", "run", "punch", "kick", "action", "jump", "block", "crouch", "strafe",
            "weapon", "target", "look", "handbrake"];
        for m in &MOVES {
            for text in [m.input, m.when] {
                for part in text.split('{').skip(1) {
                    let name = part.split('}').next().unwrap();
                    assert!(ACTIONS.contains(&name) || name == "siren", "{}: no action {name}", m.name);
                }
            }
        }
        assert_eq!(sparred(), 20);
    }
}
