//! People: movement, fighting, grapples, arrests, ladders, falling and a simple fight AI.
//!
//! This is a compact re-design rather than a port of Person.cpp / combat.cpp / pcom.cpp: one
//! state machine for everybody, driven by [`Controls`] that come from the keyboard (player)
//! or from [`Person::think`] (AI). Moves and their effects are data (see [`Move::def`]).
//! The rules the combat tutorial checks are kept: fight-test dummies only go down to the
//! moves their event point names (`PCOM_COMBAT_*`, fallen/Headers/pcom.h).

use glam::{vec2, vec3, Vec2, Vec3};
use uc_formats::iam::Map;
use uc_world::character::{anim, facing, Figure, Play};
use uc_world::ground::{Ground, STEP_HEIGHT};

mod ai;
mod carry;
mod climb;
mod fight;
mod hug;
pub use hug::Hug;
mod moving;

/// `PCOM_COMBAT_*`: which finishing moves knock out a fight-test dummy.
pub mod ko {
    pub const SLIDE: u8 = 1 << 0;
    pub const COMBO_PPP: u8 = 1 << 1;
    pub const COMBO_KKK: u8 = 1 << 2;
    pub const COMBO_ANY: u8 = 1 << 3;
    pub const GRAPPLE: u8 = 1 << 4;
    pub const SIDE_KICK: u8 = 1 << 5;
    pub const BACK_KICK: u8 = 1 << 6;
    /// Not a `PCOM_COMBAT_*` bit: the blow knocks people down (they get up again unless
    /// it also takes their last health).
    pub const KNOCKDOWN: u8 = 1 << 7;
    /// Finishing moves that knock down.
    pub const FINISHERS: u8 = SLIDE | COMBO_PPP | COMBO_KKK | GRAPPLE | KNOCKDOWN;
}

/// What a person fights with.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Weapon {
    #[default]
    None,
    Knife,
    Bat,
    Pistol,
    Shotgun,
    Ak47,
    /// Charges to set down and run from (`SPECIAL_EXPLOSIVES`): action sets one.
    Explosives,
    /// The sniper rifle: not in the shipped game, whose sniper mode (snipe.cpp) nothing can
    /// turn on. Here it's a gun of its own, picked up like the others, whose scope is the
    /// original's sniper mode.
    Sniper,
    /// Grenades (`SPECIAL_GRENADE`): action primes one, action again throws it.
    Grenade,
    /// The grappling hook (#109, hook.cpp; switched off in the original): attack throws it,
    /// and attack again climbs it once it's caught on a roof.
    Hook,
}

impl Weapon {
    pub const GUNS: [Weapon; 4] = [Weapon::Pistol, Weapon::Shotgun, Weapon::Ak47, Weapon::Sniper];

    /// From an enemy's `HAS_*` flags (fallen/Headers/Mission.h).
    pub fn from_has(has: u8) -> Weapon {
        if has & 4 != 0 {
            Weapon::Ak47
        } else if has & 2 != 0 {
            Weapon::Shotgun
        } else if has & 1 != 0 {
            Weapon::Pistol
        } else if has & 0x40 != 0 {
            Weapon::Bat
        } else if has & 0x20 != 0 {
            Weapon::Knife
        } else {
            Weapon::None
        }
    }

    /// The item (`IT_*`) it is when lying on the ground.
    pub fn item(self) -> Option<u8> {
        match self {
            Weapon::None => None,
            Weapon::Pistol => Some(2),
            Weapon::Shotgun => Some(4),
            Weapon::Knife => Some(5),
            Weapon::Ak47 => Some(6),
            Weapon::Bat => Some(8),
            Weapon::Explosives => Some(crate::EXPLOSIVES),
            Weapon::Sniper => Some(crate::SNIPER),
            Weapon::Grenade => Some(crate::game::grenades::IT_GRENADE),
            Weapon::Hook => None,
        }
    }

    /// The weapon item `IT_*` `kind` is.
    pub fn from_item(kind: u8) -> Option<Weapon> {
        [Weapon::Pistol, Weapon::Shotgun, Weapon::Knife, Weapon::Ak47, Weapon::Bat, Weapon::Sniper].into_iter().find(|w| w.item() == Some(kind))
    }

    pub fn is_gun(self) -> bool {
        self.slot().is_some()
    }

    /// Guns: which ammo count is theirs.
    pub fn slot(self) -> Option<usize> {
        Weapon::GUNS.iter().position(|&g| g == self)
    }

    /// Rounds that come with a gun or a box of its ammo (`IT_AMMO_*`).
    pub fn clip(self) -> u32 {
        match self {
            Weapon::Pistol => 15,
            Weapon::Shotgun => 8,
            Weapon::Ak47 => 30,
            Weapon::Sniper => 10,
            _ => 0,
        }
    }

    pub fn name(self) -> &'static str {
        match self {
            Weapon::None => "",
            Weapon::Knife => "KNIFE",
            Weapon::Bat => "BASEBALL BAT",
            Weapon::Pistol => "PISTOL",
            Weapon::Shotgun => "SHOTGUN",
            Weapon::Ak47 => "AK-47",
            Weapon::Explosives => "EXPLOSIVES",
            Weapon::Sniper => "SNIPER RIFLE",
            Weapon::Grenade => "GRENADE",
            Weapon::Hook => "GRAPPLING HOOK",
        }
    }

    /// As held: the prim (`255 + PersonID >> 5`), the body part it's drawn in (the knife
    /// goes in the other hand) and the muzzle flash prim (fallen/DDEngine/Source/figure.cpp).
    pub fn held(self) -> Option<(u16, usize, Option<u16>)> {
        match self {
            Weapon::None => None,
            Weapon::Pistol => Some((256, 7, Some(261))),
            Weapon::Knife => Some((257, 10, None)),
            Weapon::Shotgun => Some((258, 7, Some(262))),
            Weapon::Bat => Some((259, 7, None)),
            Weapon::Ak47 => Some((260, 7, Some(263))),
            // The AK-47's, drawn longer.
            Weapon::Sniper => Some((260, 7, Some(263))),
            Weapon::Explosives | Weapon::Grenade | Weapon::Hook => None,
        }
    }

    /// Standing with the gun up (two-handed guns all use the shotgun's, `set_person_aim`).
    fn aim_anim(self) -> usize {
        match self {
            Weapon::Shotgun | Weapon::Ak47 | Weapon::Sniper => anim::SHOTGUN_AIM,
            _ => anim::PISTOL_AIM,
        }
    }

    fn fire_anim(self) -> usize {
        match self {
            Weapon::Shotgun | Weapon::Sniper => anim::SHOTGUN_FIRE,
            Weapon::Ak47 => anim::AK_FIRE,
            _ => anim::PISTOL_SHOOT,
        }
    }
}

/// `PCOM_BENT_ROBOT`: ignores everything and everyone.
pub const BENT_ROBOT: u8 = 1 << 5;
/// `PCOM_BENT_RESTRICTED`: won't climb, jump and so on, on a route.
pub const BENT_RESTRICTED: u8 = 1 << 6;
/// `PCOM_BENT_FIGHT_BACK`: hits back when set on.
pub const BENT_FIGHT_BACK: u8 = 1 << 3;
/// `PCOM_BENT_PLAYERKILL`: only the player can hurt them.
pub const BENT_PLAYERKILL: u8 = 1 << 7;
/// `PCOM_AI_BODYGUARD`: guards the person their `other` point created, and follows them.
pub const AI_BODYGUARD: u8 = 8;
/// `PCOM_AI_CIV`: a member of the public.
pub const AI_CIV: u8 = 1;
/// `PCOM_AI_DRIVER`: finds a car, gets in and drives it.
pub const AI_DRIVER: u8 = 9;
/// `PCOM_AI_COP_DRIVER`: a cop who drives. Its "stop and get out if he sees trouble" is
/// commented out of the shipped game (pcom.cpp:11290), so it drives as a driver does.
pub const AI_COP_DRIVER: u8 = 14;
/// `PCOM_AI_GUARD`, `PCOM_AI_GANG`, `PCOM_AI_SUICIDE` (fallen/Headers/pcom.h).
pub const AI_GUARD: u8 = 2;
pub const AI_GANG: u8 = 6;
/// `PCOM_AI_FIGHT_TEST`: the combat tutorial's dummies.
pub const AI_FIGHT_TEST: u8 = 12;
/// `PCOM_AI_MIB`: the Men in Black.
pub const AI_MIB: u8 = 18;
/// `PCOM_AI_KILL_COLOUR`: goes for everyone of one colour.
pub const AI_KILL_COLOUR: u8 = 17;
/// `PCOM_AI_BULLY`: picks on whoever's about, the player or a passer-by.
pub const AI_BULLY: u8 = 13;
/// `PCOM_MOVE_*` that pose at home: warming their hands at a fire, dancing, hands up, tied up.
pub const MOVE_WARM_HANDS: u8 = 6;
pub const MOVE_DANCE: u8 = 8;
pub const MOVE_HANDS_UP: u8 = 9;
pub const MOVE_TIED_UP: u8 = 10;
/// `PCOM_BENT_LAZY`: walks rather than runs, doesn't keep taunting, and sits down.
pub const BENT_LAZY: u8 = 1 << 0;
/// `PCOM_BENT_GANG`: one of a gang, who backs up the rest of their colour.
pub const BENT_GANG: u8 = 1 << 2;
/// `PCOM_BENT_ONLY_KILL_PLAYER`: a bully who only picks on the player.
pub const BENT_ONLY_KILL_PLAYER: u8 = 1 << 4;
/// `PCOM_AI_COP`.
pub const AI_COP: u8 = 5;
pub const AI_SUICIDE: u8 = 15;
/// `PCOM_MOVE_WANDER`.
pub const MOVE_WANDER: u8 = 4;
/// `PCOM_MOVE_FOLLOW`.
pub const MOVE_FOLLOW: u8 = 5;
/// `PCOM_MOVE_FOLLOW_ON_SEE`: stands still until they're near whom they'd follow and can see
/// them, then follows.
pub const MOVE_FOLLOW_ON_SEE: u8 = 7;
/// `PCOM_AI_HYPOCHONDRIA`: lies injured, waiting to be carried (`set_person_injured`).
pub const AI_INJURED: u8 = 20;
/// How far ahead of the one carrying them the carried person is put (`carry_running`).
const CARRY_AHEAD: f32 = 90.0;

/// Speeds in world units per second (a map square is 256). Chosen by eye.
pub const WALK_SPEED: f32 = 190.0;
/// Crawling: velocity 15 to walking's 16 (`SUB_STATE_CRAWLING` in `fn_person_moveing`).
pub const CRAWL_SPEED: f32 = WALK_SPEED * 15.0 / 16.0;
const RUN_SPEED: f32 = 520.0;
/// How often the injured might struggle: 32 of the original's turns.
const INJURED_BEAT: f32 = 32.0 / 20.0;
/// How often someone sat down shifts about: 64 of the original's turns, 20 a second.
const SIT_FIDGET: f32 = 64.0 / 20.0;
const SPRINT_SPEED: f32 = 700.0;
const FIGHT_STEP_SPEED: f32 = 140.0;
/// The ladder animation climbs this far per cycle (its hips rise 117 -> 245).
const RUNG_CYCLE: f32 = 128.0;
const GRAVITY: f32 = 2000.0;
/// Take-off speed of a jump: reaches about 0.4 squares.
const JUMP_SPEED: f32 = 640.0;
/// Ledges this low are jumped onto, not climbed (a standing jump clears them).
const JUMPABLE: f32 = 160.0;
/// While jumping, ledges this far above the feet can be grabbed. The original grabs within 40
/// of the raised hands (`grab_ledge`, Person.cpp:12771); this much lets a jump's top reach a
/// van's roof, 292 up, as there.
const GRAB_REACH: f32 = 224.0;
const RADIUS: f32 = 36.0;
/// Opponents closer than this put a standing person into fight stance.
const FIGHT_RANGE: f32 = 400.0;
/// How close the attacker must be to grab or arrest.
const GRAB_RANGE: f32 = 120.0;
pub(crate) const ARREST_RANGE: f32 = 130.0;
/// AI: notices the player within 2 squares (`PCOM_get_dist_between(...) < 0x200`).
const AGGRO_RANGE: f32 = 512.0;
/// The hands of someone hanging from a cable are this far above their feet position.
const HANG: f32 = 300.0;
/// `SUB_OBJECT_LEFT_HAND`.
const LEFT_HAND: usize = 7;
/// Pistols: shoot from this far, close in from further.
const SHOOT_RANGE: f32 = 6.0 * 256.0;

/// Something worth hearing. The game reports these with a position; the app decides which
/// recordings to play.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Sfx {
    /// An attack swings.
    Swing,
    /// One of the Men in Black going up (`S_MIB_EXPLODE`).
    MibExplode,
    /// A blow lands.
    Hit { heavy: bool },
    /// Someone (of `PERSON_*` type `kind`) is hurt but stays up.
    Hurt { kind: u8 },
    KnockedOut { kind: u8 },
    /// A foot coming down: the player's, and the run of sound ids for what it's on
    /// ([`crate::steps`]), which the game fills in.
    Footstep { player: bool, sound: Option<(usize, usize)> },
    /// The effort of a jump, from whoever made it.
    Jump { kind: u8 },
    Land,
    Grab,
    /// Darci makes an arrest.
    Arrest,
    /// Darci draws on someone who might be a villain: "Freeze!" (0, 1) or "Stop, police!"
    /// (2) (`S_DARCI_FREEZE_START`, `_END`, `S_DARCI_STOP_POLICE`).
    Freeze(u8),
    /// The player knocked someone out and crows about it.
    Taunt { kind: u8 },
    /// An enemy spots the player.
    Alert { kind: u8 },
    /// A mission message with recording number `n` (if it was recorded).
    Voice(usize),
    /// Line `line` of the conversation with recording number `n`.
    ConversationLine(usize, usize),
    /// A blow blocked.
    Block,
    /// A cop goes after a felon (`S_COP_ARREST_START` to `_END`).
    CopArrest,
    /// A gang member set to guard heard something (`SOUND_Curious`): "Huh?", "Who's there?".
    Curious { kind: u8 },
    /// Someone plunging to their death screams (`ScreamFallSound`, Sound.cpp:600).
    ScreamFall { kind: u8 },
    Stab,
    BatHit,
    Shot(Weapon),
    /// Pulled the trigger with no ammo.
    Click,
    /// Grabbed a cable.
    Zip,
    /// A car hit something hard.
    Crash,
    /// A car knocked a cone or barrel.
    Cone,
    /// A barrel knocked (`S_BARREL_START` to `_END`, barrel.cpp:75): anything but a cone.
    Barrel,
    /// The crackle of a spark (`S_ELEC_START + i % 5`, spark.cpp:657).
    Elec,
    /// Someone run over (`S_THUMP_SQUISH`, vehicle.cpp:3339).
    Squish,
    /// A car rubbing along a wall (`S_CAR_SCRAPE_START` to `_END`, vehicle.cpp:2350).
    Scrape,
    /// A kick in the nuts (`S_KICK_IN_THE_NUTS_START`, `_END`, combat.cpp:2289).
    Nuts,
    /// A camera fly-by that holds the player starting (`S_CUTSCENE_STING`, eway.cpp:5389),
    /// heard wherever she is.
    Sting,
    /// A car's tyres started sliding.
    Skid,
    /// A driver held up by someone in the road sounds the horn (`S_CAR_HORN_START` and the
    /// two after).
    Horn,
    /// Getting into or out of a car.
    CarDoor,
    Pickup,
    /// Something hidden turned up in a search (`S_HIDDENITEM`).
    Found,
    /// A bomb or a car going up.
    Explosion,
    /// The radio's beep before a message (`S_RADIO_MESSAGE`), heard wherever she is.
    Radio,
    /// The danger stings, heard wherever she is (`context_music`, Controls.cpp:1389): a thug
    /// come within 0x400 of her (`S_TUNE_DANGER_RED`, `near`), or the nearest now two to three
    /// times that off (`S_TUNE_DANGER_GREEN`).
    Danger { near: bool },
    /// A police siren (`S_SIREN_START`), from where it's given.
    Siren,
    /// One of the game's own sounds by id (`sound_list`), asked for by a mission.
    ById { id: usize, replace: bool },
    Won,
    Lost,
    /// Lost with the player still standing (`MUSIC_MODE_CHAOS`).
    Chaos,
}

/// How fast someone heads for their `goal` (`PERSON_SPEED_*`, fallen/Headers/Person.h).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Pace {
    #[default]
    Walk,
    Run,
    Sprint,
    /// Crawling (`PERSON_SPEED_CRAWL`): a follower keeping down with whoever they follow.
    Crawl,
}

impl Pace {
    /// The controls that move someone at this speed.
    fn controls(self) -> (bool, bool) {
        match self {
            Pace::Walk | Pace::Crawl => (false, false),
            Pace::Run => (true, false),
            Pace::Sprint => (true, true),
        }
    }
}

/// Bane's floating bodies rise `64 * TICK_RATIO >> TICK_SHIFT` in 256ths a tick at 20 a
/// second, to 200 above the ground (`fn_person_float`).
const FLOAT_SPEED: f32 = 64.0 / 256.0 * 20.0;
const FLOAT_HEIGHT: f32 = 200.0;

/// One frame of intent.
#[derive(Debug, Clone, Copy, Default)]
pub struct Controls {
    /// Desired move direction in world xz (length 0..1).
    pub wish: Vec2,
    pub run: bool,
    /// Held "action" while running.
    pub sprint: bool,
    pub punch: bool,
    pub kick: bool,
    /// Pressed "action": arrest, climb.
    pub action: bool,
    pub jump: bool,
    /// Held: guard against blows from the front.
    pub block: bool,
    /// Held punch: automatic guns keep firing.
    pub fire: bool,
    /// Pressed: the next weapon.
    pub switch: bool,
    /// Pressed: target the next enemy (the game's targeting uses it).
    pub cycle_target: bool,
    /// Who guns shoot at (chosen by the game for the player, the player for the AI).
    pub target: Option<usize>,
    /// Somewhere else to aim the gun when `target` is nobody: a creature the player has in
    /// their sights.
    pub aim: Option<Vec3>,
    /// Driving, when in a car.
    pub drive: crate::vehicle::Drive,
    /// Pressed: the sniper rifle's scope, in or out.
    pub scope: bool,
    /// Taunt `target` (`ANIM_WANKER`), standing.
    pub taunt: bool,
    /// Held: crouch (and crawl when moving), as action held standing still does.
    pub crouch: bool,
    /// The strafe key held (#309), with the way to keep facing (the camera's). The ways to
    /// go are then hers, as the original's were: to the side a step that way, back walking
    /// backwards, jump with back a back flip; standing, she turns on the spot to face it.
    pub strafe: Option<Vec2>,
    /// Looking round (`FLAG2_PERSON_LOOK`): the way she's turned to (her `fig.angle`) and
    /// how far up she's looking (radians, down negative). She turns with the view, and her
    /// gun picks out whoever's at that height (`apply_button_input_first_person`,
    /// interfac.cpp:7459).
    pub look: Option<(f32, f32)>,
}

impl Controls {
    /// Anything asked of the player: a way to go or a button.
    pub fn any(&self) -> bool {
        self.wish.length_squared() > 1e-4 || self.run || self.sprint || self.punch || self.kick || self.action
            || self.jump || self.block || self.fire || self.switch || self.scope || self.taunt || self.crouch || self.strafe.is_some()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Move {
    Punch1,
    Punch2,
    Punch3,
    Kick1,
    Kick2,
    Kick3,
    KickBack,
    KickLeft,
    KickRight,
    /// Darci's kick between the legs (`ANIM_KICK_NAD`).
    KickNad,
    /// The jump kick: jump in a fight, no way held (`ANIM_KICK_NS`, interfac.cpp:5109).
    KickJump,
    /// A kick at someone within 120 (`set_person_kick_near`, combat.cpp:3495).
    KickNear,
    /// The foot sweep: kick from a block (`set_person_leg_sweep`, Person.cpp:9545).
    LegSweep,
    /// A kick at someone lying knocked down is a stamp (`set_person_stomp`, Person.cpp:9928).
    Stomp,
    Knife1,
    Knife2,
    Knife3,
    Bat1,
    Bat2,
    /// Clubbing with a two-handed gun in fight mode (#346).
    Whip1,
    Whip2,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Dir {
    Front,
    Back,
    Left,
    Right,
}

pub struct MoveDef {
    anim: usize,
    /// When in the animation (0..1) the blow lands.
    hit_at: f32,
    reach: f32,
    dir: Dir,
    damage: i32,
    high: bool,
    /// What it counts as for fight-test dummies.
    ko: u8,
    /// The next move of the combo for the same button.
    next: Option<Move>,
}

impl Move {
    pub fn def(self) -> MoveDef {
        use Move::*;
        let d = |anim, hit_at, reach, dir, damage, high, ko, next| MoveDef { anim, hit_at, reach, dir, damage, high, ko, next };
        match self {
            Punch1 => d(anim::PUNCH_COMBO1, 0.5, 130.0, Dir::Front, 10, true, 0, Some(Punch2)),
            Punch2 => d(anim::PUNCH_COMBO2, 0.5, 130.0, Dir::Front, 15, true, ko::COMBO_ANY, Some(Punch3)),
            Punch3 => d(anim::PUNCH_COMBO3, 0.45, 140.0, Dir::Front, 25, false, ko::COMBO_PPP | ko::COMBO_ANY, None),
            Kick1 => d(anim::KICK_COMBO1, 0.5, 150.0, Dir::Front, 15, false, 0, Some(Kick2)),
            Kick2 => d(anim::KICK_COMBO2, 0.5, 150.0, Dir::Front, 20, false, ko::COMBO_ANY, Some(Kick3)),
            Kick3 => d(anim::KICK_COMBO3, 0.45, 160.0, Dir::Front, 30, true, ko::COMBO_KKK | ko::COMBO_ANY, None),
            KickBack => d(anim::KICK_BEHIND, 0.45, 160.0, Dir::Back, 25, false, ko::BACK_KICK, None),
            KickLeft => d(anim::KICK_LEFT, 0.45, 160.0, Dir::Left, 25, false, ko::SIDE_KICK, None),
            KickRight => d(anim::KICK_RIGHT, 0.45, 160.0, Dir::Right, 25, false, ko::SIDE_KICK, None),
            // The combat table's extra (combat.cpp:1894) comes on top, by who's kicked.
            KickNad => d(anim::KICK_NAD, 0.45, 130.0, Dir::Front, 10, false, 0, None),
            // Its blow is in the animation's fight data, not the source: a side kick's.
            // 40, whatever else (combat.cpp:1970).
            KickNear => d(anim::KICK_NEAR, 0.45, 130.0, Dir::Front, 40, false, 0, None),
            KickJump => d(anim::KICK_NS, 0.45, 150.0, Dir::Front, 25, true, ko::SIDE_KICK, None),
            // `sweep_feet` (Person.cpp:1936): 49 off and down, low so no guard stops it. Its
            // reach and timing are in the animation's fight data, not the source: these are
            // the kicks'.
            LegSweep => d(anim::LEG_SWEEP, 0.45, 150.0, Dir::Front, 49, false, ko::KNOCKDOWN, None),
            // 50 on top of the blow (combat.cpp:1967); it lands only on someone lying down,
            // the foot within 0x53 of their pelvis, head or shin (combat.cpp:1296).
            Stomp => d(anim::FIGHT_STOMP, 0.5, 130.0, Dir::Front, 50, false, 0, None),
            // Weapons (damage from apply_hit_to_person, fallen/Source/combat.cpp).
            Knife1 => d(anim::KNIFE_ATTACK1, 0.45, 150.0, Dir::Front, 30, true, 0, Some(Knife2)),
            Knife2 => d(anim::KNIFE_ATTACK2, 0.45, 150.0, Dir::Front, 50, true, ko::COMBO_ANY, Some(Knife3)),
            Knife3 => d(anim::KNIFE_ATTACK3, 0.4, 160.0, Dir::Front, 70, false, ko::COMBO_ANY | ko::KNOCKDOWN, None),
            Bat1 => d(anim::BAT_HIT1, 0.5, 180.0, Dir::Front, 40, true, 0, Some(Bat2)),
            Bat2 => d(anim::BAT_HIT2, 0.5, 180.0, Dir::Front, 60, true, ko::COMBO_ANY | ko::KNOCKDOWN, None),
            // Fight-tree node 1's blow, a jab's (`set_person_punch`'s `node = 1`).
            Whip1 => d(anim::SHOTGUN_WHIP1, 0.5, 130.0, Dir::Front, 10, true, 0, None),
            Whip2 => d(anim::SHOTGUN_WHIP2, 0.5, 130.0, Dir::Front, 10, true, 0, None),
        }
    }
    /// Back out of a move of a combo that wasn't followed up (the `fight_tree` node it goes
    /// to when it's finished, combat.cpp:79): the last move of a combo has none.
    fn ret(self) -> Option<usize> {
        use Move::*;
        Some(match self {
            Punch1 => anim::PUNCH_RETURN1,
            Punch2 => anim::PUNCH_RETURN2,
            Kick1 => anim::KICK_RETURN1,
            Kick2 => anim::KICK_RETURN2,
            Knife1 => anim::KNIFE_ATTACK1_RET,
            Knife2 => anim::KNIFE_ATTACK2_RET,
            Bat1 => anim::BAT_HIT1_RET,
            _ => return None,
        })
    }

    /// Punches and weapon swings (the punch button).
    fn is_punch(self) -> bool {
        matches!(self, Move::Punch1 | Move::Punch2 | Move::Punch3 | Move::Knife1 | Move::Knife2 | Move::Knife3 | Move::Bat1 | Move::Bat2
            | Move::Whip1 | Move::Whip2)
    }
    /// The first move of the punch button's combo with `weapon`.
    fn punch_with(weapon: Weapon) -> Move {
        match weapon {
            Weapon::Knife => Move::Knife1,
            Weapon::Bat => Move::Bat1,
            _ => Move::Punch1,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Grab {
    Grab,
    Hold,
    Knee1,
    Knee2,
    Throw,
    /// Roper's grapples (`grapples[]`, combat.cpp:179, only his): strangling, and a headbutt.
    Strangle,
    Headbutt,
    /// The player's takedown from behind (`grapples[0]`, `ANIM_PISTOL_WHIP`): out cold.
    PistolWhip,
}

impl Grab {
    /// (attacker, victim) animations.
    pub fn anims(self) -> (usize, usize) {
        match self {
            Grab::Grab | Grab::Hold => (anim::GRAB_ARM, anim::GRAB_ARMV),
            Grab::Knee1 => (anim::GRAB_ARM_KNEE1, anim::GRAB_ARM_KNEE1V),
            Grab::Knee2 => (anim::GRAB_ARM_KNEE2, anim::GRAB_ARM_KNEE2V),
            Grab::Throw => (anim::GRAB_ARM_THROW, anim::GRAB_ARM_THROWV),
            Grab::Strangle => (anim::STRANGLE, anim::STRANGLE_VICTIM),
            Grab::Headbutt => (anim::HEADBUTT, anim::HEADBUTT_VICTIM),
            Grab::PistolWhip => (anim::PISTOL_WHIP, anim::PISTOL_WHIP_TAKE),
        }
    }

    /// How far in front of the one grappling the victim is held (`set_grapple_pos`).
    pub fn reach(self) -> f32 {
        match self {
            Grab::Strangle | Grab::Headbutt => 100.0,
            Grab::PistolWhip => 70.0,
            _ => 90.0,
        }
    }

    /// Whether the victim is held facing the one grappling them (else facing away: the
    /// pistol whip's `Angle` of 1024 turns them round).
    pub fn face_to_face(self) -> bool {
        self != Grab::PistolWhip
    }
}

/// A gang member taunting the player (`PCOM_AI_STATE_TAUNT`).
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Taunt {
    /// About to make the gesture.
    Now,
    /// Making it.
    Doing,
    /// Pausing this many seconds before the next one.
    Pause(f32),
}

#[derive(Debug, Clone, PartialEq)]
pub enum State {
    Move,
    Attack { mv: Move, hit: bool, queued: Option<Move> },
    /// Coming back from a move of a combo that wasn't followed up (`ANIM_PUNCH_RETURN1` and
    /// the rest): a press meanwhile starts afresh once it's done.
    Recover { queued: Option<Move> },
    /// Sliding tackle: phase 0 start, 1 slide, 2 get up.
    Slide { phase: u8, speed: f32, hit: u64 },
    Hurt,
    /// Lying on the ground: knocked down (getting up after a while) or knocked out.
    Down,
    GetUp,
    /// Guarding against blows from the front: with the block key, or, `auto`, stepping back
    /// from someone swinging at her (the manual's auto-block).
    Block { auto: bool },
    /// Firing a gun at `target` (or straight ahead): `shots` fired so far.
    Shoot { target: Option<usize>, shots: u32 },
    /// Climbing over a fence: phase 0 grabbing on, 1 climbing up, 2 swinging over. The hips
    /// follow `from` -> up the near side -> over the top at `top` -> the far side.
    Fence { phase: u8, at: Vec2, over: Vec2, base: f32, top: f32, from: Vec3 },
    /// Driving (or sitting in) vehicle `vehicle`: not drawn, moved with the car.
    InCar { vehicle: usize },
    /// Getting into vehicle `vehicle` (`SUB_STATE_ENTERING_VEHICLE`): lined up at its door,
    /// in once the animation's done.
    Enter { vehicle: usize },
    /// On motorbike `bike` (`FLAG_PERSON_BIKING`): phase 0 getting on
    /// (`SUB_STATE_MOUNTING_BIKE`), 1 riding, 2 getting off.
    Bike { bike: usize, phase: u8 },
    /// Hands up to the gun person `by` is aiming (`PCOM_AI_STATE_HANDS_UP`), for `time`
    /// seconds so far.
    HandsUp { by: usize, time: f32 },
    /// Sat on a bench, a sofa or an armchair (`ANIM_SIT_DOWN`, then held:
    /// `SUB_STATE_SIMPLE_ANIM_OVER`), until something gets them up.
    Sit,
    /// Doing what their move has them do at home, until something needs them
    /// (`PCOM_set_person_move_animation` with `FLAG_PERSON_NO_RETURN_TO_NORMAL`): dancing,
    /// `anim` on a loop; or hands up, for good.
    Pose { anim: usize },
    /// Sliding down a cable: `t` along it (0..1), moving towards `dir` (+1 or -1).
    /// `ridden`: how far along the cable so far (world units).
    Zip { cable: usize, t: f32, dir: f32, speed: f32, ridden: f32 },
    Fall { vy: f32, vel: Vec2 },
    /// Landing, still carrying some of the jump's speed.
    Land { vel: Vec2 },
    /// Pulling up onto a ledge (or off a ladder). The animation itself lifts the body, so
    /// `pos` moves from `start` to `end` while the drawn hips go from the edge to `to`.
    PullUp { start: Vec3, end: Vec3, to: Vec3 },
    /// Sliding down ground too steep to stand on (`SUB_STATE_SLIPPING`), until it levels out.
    Slip,
    /// A flip to the side, out of a bullet's way (`SUB_STATE_FLIPING`).
    Flip { left: bool },
    /// Vaulting a low fence at a run (`SUB_STATE_RUNNING_VAULT`), from `from` to `to` on the
    /// far side as the animation goes.
    Vault { from: Vec3, to: Vec3 },
    /// Hanging by the hands from a ledge (`STATE_DANGLING`, `SUB_STATE_DANGLING`): `to` is
    /// where she'd stand on top after pulling up, `dir` the way she faces (into the wall).
    /// While going a hand along it (`SUB_STATE_TRAVERSE_LEFT`, `_RIGHT`), `shift` is the
    /// figure's start and end.
    Dangle { to: Vec3, dir: Vec2, shift: Option<(Vec3, Vec3)> },
    /// Stepping onto `ladder` at its foot (`SUB_STATE_MOUNT_LADDER`).
    Mount { ladder: usize },
    /// Climbing down onto `ladder` from the roof at its top: climbing off it backwards
    /// (`SUB_STATE_CLIMB_DOWN_ONTO_LADDER`), `pos` going from `end` back to `start` on the
    /// ladder as [`State::PullUp`]'s does the other way.
    OntoLadder { ladder: usize, start: Vec3, end: Vec3 },
    /// A step to the side (`SUB_STATE_STEP_LEFT`, `_RIGHT`): the animation carries her.
    Step,
    /// On a ladder: feet at `y0` plus one rung cycle per loop of the animation since `loops0`.
    Climb { ladder: usize, y0: f32, loops0: f32 },
    /// Grappling `victim`. `frame`: the last keyframe of Roper's grapples that did its damage.
    Grapple { victim: usize, grab: Grab, hit: bool, timer: f32, frame: Option<usize> },
    /// Taunting `at` (`ANIM_WANKER`), facing them all the while.
    Taunt { at: usize },
    /// Getting a weapon out, stood still (`SUB_STATE_DRAW_ITEM`, `_DRAW_GUN`): it's in hand
    /// from the fourth frame.
    Draw,
    Held { by: usize },
    Arrest { victim: usize },
    Cuffed,
    /// Drawn up into the air by Bane (`STATE_FLOAT`): a quarter of a unit a tick until the
    /// body is 200 above the ground at `floor`, and there it hangs. Nothing in the original
    /// ever lets them down again (`set_person_float_down` has no callers).
    Float { floor: f32 },
    /// Lying injured (`STATE_DEAD`, `SUB_STATE_DEAD_INJURED`): only Roper can do anything
    /// with them, and that's pick them up.
    Injured,
    /// Carried by person `by`, who puts them where they are and picks their animation.
    Carried { by: usize },
    /// Carrying `victim` (`STATE_CARRY`, `FLAG2_PERSON_CARRYING`): phase 0 picking them up,
    /// 1 standing, 2 walking, 3 putting them down.
    Carry { victim: usize, phase: u8 },
    /// Throwing a grenade (`STATE_CANNING`, `SUB_STATE_CANNING_RELEASE`): whether it's left
    /// her hand yet.
    Throw { released: bool },
    /// The grappling hook: spinning it (`windup`), then letting it go (whether it's gone), or
    /// winding it back in (`released` with no wind-up).
    Hook { released: bool, windup: bool },
    /// Climbing the hook's line up a wall, as up a ladder: standing off it at `at`, facing
    /// `dir` (into the wall), up to the edge at `top` and onto the roof at `to`. Feet at `y0`
    /// plus a rung cycle per loop of the animation since `loops0`.
    Rope { at: Vec2, dir: Vec2, top: f32, to: Vec3, y0: f32, loops0: f32 },
    /// Crouched, for as long as action is held (`set_person_croutch`): phase 0 going down,
    /// 1 down, 2 crawling (`SUB_STATE_CRAWLING`), 3 getting up (`SUB_STATE_IDLE_UNCROUCH`).
    Crouch { phase: u8 },
    /// Bending down for a can (`SUB_STATE_CANNING_PICKUP`): whether it's in her hand yet.
    PickUp { got: bool },
    /// Pressing a switch, turning a valve or getting up from a seat
    /// (`set_person_do_a_simple_anim`), then on as before: the animations to play in turn.
    Press { then: Option<usize> },
    /// Back to a wall (`STATE_HUG_WALL`, #205): the wall from `a` to `b`, `out` its normal
    /// (the way she faces), and what she's doing against it.
    HugWall { a: Vec2, b: Vec2, out: Vec2, hug: Hug },
}

/// What a person did this frame that affects somebody else.
#[derive(Debug, Clone)]
pub enum Event {
    Hit { attacker: usize, victim: usize, damage: i32, ko: u8, high: bool, low: bool, weapon: Weapon },
    /// A shot at `victim` (or at nothing); whether it hits is decided by the game.
    Shot { attacker: usize, victim: Option<usize>, weapon: Weapon },
    Grab { attacker: usize, victim: usize, grab: Grab },
    /// A frame of Roper's strangle or his headbutt landing on `victim`: the damage is taken
    /// straight off their health, by how skilled they are (Person.cpp:18969).
    Squeeze { victim: usize, headbutt: bool },
    /// The pistol whip's done: the victim lies out cold, alive (`PERSON_DEATH_TYPE_STAY_ALIVE_
    /// PRONE`, Person.cpp:19090), until cuffed.
    OutCold { attacker: usize, victim: usize },
    /// The attacker changed grapple move; the victim follows.
    GrabMove { victim: usize, grab: Grab },
    Release { victim: usize },
    Arrest { attacker: usize, victim: usize },
    /// `carrier` has picked up `victim` (`set_person_carry`).
    Carry { carrier: usize, victim: usize },
    Arrested { victim: usize },
    Punch { attacker: usize },
    Kick { attacker: usize },
    /// The throw's reached the frame where the grenade leaves `thrower`'s hand.
    Throw { thrower: usize },
    /// The hook's let go from `thrower`'s hand.
    Hook { thrower: usize },
    /// The pickup reached the frame where the can in reach goes into `who`'s hand.
    PickUp { who: usize },
}

/// Looking into a sound (`PCOM_AI_STATE_INVESTIGATING`, pcom.cpp:7768): where it came from,
/// how far along they are, and for how long.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Heard {
    pub at: Vec3,
    pub phase: Investigate,
    pub time: f32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Investigate {
    /// `PCOM_AI_SUBSTATE_SUPRISED`: turned to it for a second.
    Surprised,
    /// `_WALKOVER`: walking over to where it came from.
    WalkOver,
    /// `_LOOK`: looking left and right for trouble.
    Look,
    /// The look round over, off to somewhere someone might be hiding: the game finds it
    /// (`PCOM_find_hiding_place`, [`crate::Game::hiding_places`]).
    Hide,
}

/// What everyone else needs to know about a person this frame.
#[derive(Debug, Clone, Copy)]
pub struct Snap {
    pub pos: Vec3,
    pub facing: Vec2,
    pub present: bool,
    /// Can be hit or grabbed.
    pub standing: bool,
    pub down: bool,
    pub arrested: bool,
    /// Lying knocked down and still alive, and someone who can be cuffed (see
    /// [`Person::arrestable`]). The game also clears it for the player's own bodyguards.
    pub arrestable: bool,
    /// `PERSON_*` type.
    pub kind: u8,
    pub player: bool,
    pub hostile: bool,
    /// Worth a fighting stance: hostile, or a fight-test dummy.
    pub fighter: bool,
    /// A fight-test dummy (`PCOM_AI_FIGHT_TEST`).
    pub dummy: bool,
    pub attacking: bool,
    /// Guarding (an attacker can see it).
    pub blocking: bool,
    /// In a car: out of reach of fists and bullets.
    pub in_car: bool,
    /// Lying injured, to be carried.
    pub injured: bool,
    /// Crouched or crawling: harder to see.
    pub crouching: bool,
    /// Doubled over from a kick between the legs: another won't do it again (`not_nad`).
    pub doubled: bool,
    /// A gun in hand and drawn (`PCOM_person_has_gun_in_hand`).
    pub gun_out: bool,
    /// Lying knocked down, alive and not cuffed: a kick stamps on them
    /// (`is_person_ko_and_lay_down`, Person.cpp:2862).
    pub floored: bool,
    /// How far off the noise they're making this frame is heard (`PCOM_oscillate_tympanum`'s
    /// radii, pcom.cpp:13124): running feet 0x280, a landing 0x200 to 0x600, a fight 0x900; 0
    /// for none.
    pub noise: f32,
    /// That noise is their footsteps (`PCOM_SOUND_FOOTSTEP`).
    pub footsteps: bool,
    /// The zones of the square they're on (bit 0 for `ZF_ZONE1`), set by the game.
    pub zone: u8,
}

/// A noise the game makes for the AI to hear (`PCOM_SOUND_*`), beyond the ones people make
/// as they go (footsteps, landings, fights).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Noise {
    /// "Hey!": a guard who's spotted someone, or someone being arrested (0x600).
    Hey,
    /// A killer's "what you looking at" to someone paying them no attention (0x400).
    LookingAtMe,
    /// A grenade hitting the ground (0x300).
    GrenadeHit,
    /// A car coming fast, made two squares ahead of it (`PCOM_SOUND_VAN`, 0x180).
    Van,
}

impl Noise {
    /// How far off it's heard.
    pub fn radius(self) -> f32 {
        match self {
            Noise::Hey => 0x600 as f32,
            Noise::LookingAtMe => 0x400 as f32,
            Noise::GrenadeHit => 0x300 as f32,
            Noise::Van => 0x180 as f32,
        }
    }
}

/// What someone is running from: a person (followed as they move) or a place.
#[derive(Debug, Clone, Copy)]
pub struct Scare {
    pub by: Option<usize>,
    pub at: Vec3,
    /// Seconds spent running.
    pub time: f32,
    /// Where they're running to (`PCOM_set_person_move_runaway`), set by the game.
    pub goal: Option<Vec3>,
}

pub struct Person {
    pub fig: Figure,
    /// The event point that created this person (0 = none).
    pub way: usize,
    pub player: bool,
    /// `PERSON_*` type.
    pub kind: u8,
    pub ai: u8,
    pub bent: u8,
    pub ko_moves: u8,
    pub health: i32,
    pub max_health: i32,
    pub state: State,
    pub arrested: bool,
    pub removed: bool,
    /// 0..15: how well an enemy fights (attack rate, blocking, damage).
    pub skill: u8,
    /// The zones of the mission's zone map they're kept to (`pcom_zone`, bit 0 for
    /// `ZF_ZONE1`), 0 for none: they only go after someone standing in one of them.
    pub zone: u8,
    /// `PCOM_MOVE_*`: 5 (follow) hunts the player down.
    pub mv: u8,
    /// What's in hand.
    pub weapon: Weapon,
    /// A knife carried (drawn or not). One at most, and a bat besides
    /// (`should_person_get_item`, Special.cpp:284).
    pub knife: bool,
    /// A bat carried (drawn or not), one at most.
    pub bat: bool,
    /// The item (`IT_*`) they give up when searched, besides their weapons (`drop`).
    pub drop: Option<u8>,
    /// Rounds the AI has fired from the gun in hand: they have its one load and no more
    /// ([`Person::out_of_rounds`]).
    pub(crate) ai_fired: u32,
    /// Searched already (`FLAG_PERSON_SEARCHED`): nothing more to find on them.
    pub searched: bool,
    /// Guns carried (bit per `Weapon::GUNS` slot), and rounds for each.
    pub guns: u8,
    pub ammo: [u32; 4],
    /// The player's rounds fired from each gun's clip since it was last changed: at a full
    /// clip's worth ([`Weapon::clip`]) the next pull of the trigger changes it instead
    /// (`HAD_TO_CHANGE_CLIP`, Person.cpp:6096).
    pub spent: [u32; 4],
    /// Out for good (health gone): won't get up.
    pub knocked_out: bool,
    /// Lying out cold but alive (the pistol whip's `STAY_ALIVE_PRONE`): down until cuffed.
    pub out_cold: bool,
    /// A mission has set them peeing (`FLAG_PERSON_PEEING`).
    pub peeing: bool,
    /// Charges carried (the special's `ammo`): four at most.
    /// Grenades carried (`SPECIAL_GRENADE`'s `ammo`).
    pub grenades: u8,
    /// Carrying the grappling hook.
    pub hook: bool,
    /// Breath for sprinting (`Genus.Person->Stamina`), the player's only: up to
    /// `max_stamina`, which is 128 and the stamina badges (Person.cpp:4319).
    pub stamina: f32,
    pub max_stamina: f32,
    /// Ran out of breath, and still holding action: no sprinting until it's let go.
    winded: bool,
    /// Sprinting this frame.
    pub sprinting: bool,
    pub explosives: u8,
    /// Carrying a key (`SPECIAL_KEY`): cars locked with a coloured key open for them.
    pub has_key: bool,
    /// The dirt slot of the can in her hand (`FLAG_PERSON_CANNING`, `Hold`).
    pub can: Option<usize>,
    /// A can lies within reach of her feet to pick up (`DIRT_get_nearest_can_or_head_dist`
    /// under 0x80), as the game finds each turn.
    pub can_near: bool,
    /// Seconds left of the inventory panel shown after a weapon change
    /// (`CONTROLS_inventory_mode`, 3000 ms, fallen/Source/Controls.cpp:2386).
    pub inventory: f32,
    pub(crate) down_timer: f32,
    block_timer: f32,
    /// Seconds before a cable can be grabbed again after letting go.
    zip_cooldown: f32,
    /// Seconds left of the muzzle flash after a shot.
    pub flash: f32,
    /// `PCOM_MOVE_FOLLOW`: the event point that created the person to keep up with
    /// (0 = the player), from the point's `Data[1]`.
    pub follow: usize,
    /// Out of reach: a bodyguard more than twenty blocks from the player can't be hurt, so a
    /// mission isn't lost to a scuffle nobody saw (`PCOM_AI_BODYGUARD`, pcom.cpp).
    pub invulnerable: bool,
    /// Nothing hurts them, for good (`FLAG2_PERSON_INVULNERABLE`): people a mission makes
    /// that way, and the person the player was once a mission has moved the player on to
    /// someone else.
    pub protected: bool,
    /// Guilty of something (`FLAG2_PERSON_GUILTY`): the one sort of cop who can be arrested.
    pub guilty: bool,
    /// `ai_other`: for assassins, the event point that created the person to kill (0 = the
    /// player).
    pub other: usize,
    /// Who the AI fights (a person index), set by the game each frame: an assassin's mark,
    /// otherwise the player.
    pub foe: Option<usize>,
    /// Won't start a fight with their foe on seeing them (a capture-the-flag bot breaking
    /// off one), though a blow still makes them fight back.
    pub(crate) truce: bool,
    /// `pcom_colour` and `pcom_group`: patrols follow the event points of their colour
    /// and group.
    pub colour: u8,
    pub group: u8,
    /// Where to go next when there's nobody to fight (set by the game each frame), and how
    /// fast to get there.
    pub goal: Option<Vec3>,
    /// Where to head for on the way to whoever they're fighting, when there's no walking
    /// straight there (set by the game each frame from the navigation map).
    pub via: Option<Vec3>,
    /// On a route, what to do at its next edge (set by the game each frame): where, which
    /// way, and whether to jump there (a gap or a ledge) or walk on (into a ladder).
    pub leap: Option<(Vec3, Vec2, bool)>,
    /// Where a game mode wants them to head next, in place of the navigation map's way
    /// (endless mode's arena routes).
    pub steer: Option<Vec3>,
    /// `SlideOdd`: how many turns in a row they've slid along something that isn't a wall
    /// or a fence (a prop, a person, a car). After 50 on a route, they jump.
    pub slide_odd: u8,
    /// Whether they slid along such a thing this turn: `None` if they didn't move.
    pub(crate) slid_odd: Option<bool>,
    /// Times the wait between their shots: the difficulty's (1 on Normal).
    pub shot_wait: f32,
    pub pace: Pace,
    /// Getting out of a car's way: run this way for this many seconds.
    pub dodge: Option<(Vec2, f32)>,
    /// Taunting the player, rather than going for them just yet.
    pub taunt: Option<Taunt>,
    /// `PCOM_MOVE_WANDER`: walking to this spot, giving up after this many seconds (or
    /// pausing, with no spot).
    wander: (Option<Vec2>, f32),
    /// The patrol point (its waypoint) being walked to, kept from reaching one point until
    /// reaching the next (`PCOM_process_patrol`).
    pub(crate) patrol: Option<usize>,
    /// Seconds walking (the AI) without getting anywhere, against a wall, an edge or a
    /// parked car: they stand a while, rather than walk on the spot, and a wanderer gives up
    /// where it was going. Measured over half a second at a time, from where they were then.
    pub(crate) blocked: f32,
    pub(crate) walk_check: (Vec3, f32),
    /// A Man in Black has seen or heard the player: everyone nearby who'd fight is to come
    /// for her, round this point (`PCOM_alert_nearby_mib_to_attack`). The game takes it.
    pub(crate) alert: Option<Vec3>,
    /// Who they're out to get until they're down (`PCOM_set_person_ai_kill_person`): a bully's
    /// pick, or whoever a gang member's mate is fighting. And for a bully, the time to the
    /// next look round for someone.
    pub(crate) victim: Option<usize>,
    /// Where they were made and which way they faced (`HomeX`, `HomeZ`, `HomeYaw`): where
    /// someone who stands still goes back to.
    pub home: Option<(Vec3, f32)>,
    /// How far off the last landing is heard (0 for one too soft to hear), for the moment
    /// after it that it's heard.
    pub(crate) landed: f32,
    pub(crate) landed_for: f32,
    pub(crate) bully_clock: f32,
    /// Standing about: seconds to the next fidget (`Timer1` in `fn_person_idle`), and the
    /// dice it's rolled with.
    pub(crate) fidget: Option<f32>,
    fidget_roll: u32,
    /// Fighting someone hand to hand: who, and the quarter round them that's theirs to
    /// fight from (`AttackAngle` in the target's `gang_attacks` entry), kept by
    /// [`crate::Game::gang_slots`].
    pub(crate) slot: Option<(usize, u8)>,
    /// Someone else holds a place round who they're fighting too, so they keep to theirs.
    pub(crate) ganged: bool,
    /// The player's fight mode with a two-handed gun in hand (`PERSON_MODE_FIGHT`): hit in
    /// a fight, until she runs or gets a gun out. A punch then clubs with it (#346).
    gun_fight: bool,
    /// Knocked down over an edge: falling at this speed (up), lying as they went over
    /// (`SUB_STATE_DYING_PRONE`).
    topple: Option<f32>,
    /// Their death or arrest has had its chance of leaving something behind
    /// (`generate_bonus_item`).
    pub(crate) bonus_rolled: bool,
    /// Seconds before the gun can go off again firing on the run (`Timer1`, set by
    /// `set_person_running_shoot`).
    run_shot: f32,
    /// Off to pick up the weapon lying here (`PCOM_AI_STATE_GETITEM`).
    pub(crate) fetch: Option<Vec3>,
    /// Cuffed: seconds to the next chance to struggle (`PTIME` coming round to 0).
    pub(crate) struggle: f32,
    /// The player: seconds stood about with nothing asked of her (`pcom_move_counter` while
    /// `STATE_IDLE`, `PCOM_process_person`), counted to three a time.
    pub(crate) idle_for: f32,
    /// This breath's puff is out (`FLAG2_SYNC_SOUNDFX` for `MagicFrameCheck`).
    pub(crate) breathed: bool,
    /// Plunging to their death: they've screamed. Landed from it: the blood to splash
    /// (`PYRO_SPLATTERY`), for the game to lay.
    pub(crate) screamed: bool,
    /// Kicking in the air off a running jump: whether it's landed on someone yet.
    pub(crate) fly_kick: Option<bool>,
    /// Has looked, since going for someone, at whether they've a gun on them
    /// (`PCOM_set_person_ai_navtokill`).
    pub(crate) sized_up: bool,
    /// Wanted by the police (`FLAG_PERSON_FELON`).
    pub felon: bool,
    pub(crate) splattered: bool,
    /// One of the city's extra people, made by the level rather than the mission
    /// (`FLAG2_PERSON_FAKE_WANDER`): they keep to the pavements, and are moved somewhere
    /// near the player once out of sight for a while.
    pub fake: bool,
    /// Seconds out of sight, towards being moved (`InsideRoom`, or `sewerbits` for drivers).
    pub unseen: f32,
    /// Running from something (`PCOM_AI_STATE_FLEE_PERSON`, `_FLEE_PLACE`).
    pub scared: Option<Scare>,
    /// Somebody has given them a fright (`pcom_ai_memory`): they don't forget it, and say so.
    pub annoyed: bool,
    /// Who last hurt them (a blow, a shot, a blast they're blamed for, a car they drove), to
    /// know whose doing a death was.
    pub hurt_by: Option<usize>,
    /// Gone up in lightning (one of the Men in Black): removed, but dead all the same.
    pub blown_up: bool,
    /// Seconds to the next crackle of lightning round a downed Man in Black.
    pub(crate) crackle: f32,
    /// Heard something there, and looking that way for this many seconds more
    /// (`PCOM_AI_STATE_INVESTIGATING`).
    pub heard: Option<Heard>,
    aggro: bool,
    attack_timer: f32,
    /// Sounds made this frame (collected by the game).
    pub cues: Vec<Sfx>,
    /// Animation phase at the last footstep check (half cycles are steps).
    step_phase: f32,
    /// A foot came down this turn, for the game to lay a print (`person_splash`).
    pub(crate) footfall: Option<crate::tracks::Foot>,
    /// What's on her feet, and how fresh (`muckyfootprint`).
    pub(crate) mucky: u16,
}

/// Damage from landing at `speed` (world units per second, downwards), after
/// `person_hit_ground` (fallen/Source/Darci.cpp): nothing below a fall of about 2.4 squares,
/// up to 100 more, and fatal (250) from about 5.4 squares. The original measures the fall
/// speed per game tick (20 a second, positions in 1/256 units): `-DY` 20000 and 30000.
pub fn fall_damage(speed: f32) -> i32 {
    let dy = speed / 20.0 * 256.0;
    if dy >= 30000.0 {
        250
    } else if dy > 20000.0 {
        ((dy - 20000.0) / 100.0) as i32
    } else {
        0
    }
}

/// Standing about: breathing, or the stand their kind or what they hold gives them.
pub fn idle_anim(a: usize) -> bool {
    matches!(a, anim::BREATHE | anim::STAND_HIP | anim::SHOTGUN_IDLE)
}

/// Walking, whoever's doing it.
pub fn walking_anim(a: usize) -> bool {
    matches!(a, anim::WALK | anim::THUG_WALK | anim::THUG_WALK_SHOTGUN)
}

/// Running (`SUB_STATE_RUNNING`): a sprint, a run or a jog, or pushing off into one.
pub fn running_anim(a: usize) -> bool {
    matches!(a, anim::RUN | anim::YOMP | anim::YOMP_BAT | anim::PISTOL_JOG | anim::AK_JOG
        | anim::YOMP_START | anim::YOMP_START_AK | anim::YOMP_START_PISTOL | anim::YOMP_START_BAT)
}

/// `(a - b)` in the ground plane.
fn flat(a: Vec3, b: Vec3) -> Vec2 {
    vec2(a.x - b.x, a.z - b.z)
}

impl Person {
    pub fn new(fig: Figure, kind: u8, player: bool, way: usize) -> Person {
        let health = uc_world::character::person::HEALTH[kind.min(14) as usize];
        Person {
            fig, way, player, kind, ai: 0, bent: 0, ko_moves: 0, health, max_health: health, state: State::Move,
            pace: Pace::Walk,
            arrested: false, removed: false, skill: 0, mv: 1, weapon: Weapon::None, knife: false, bat: false, drop: None, ai_fired: 0, searched: false, guns: 0, ammo: [0; 4], spent: [0; 4], knocked_out: false, out_cold: false, down_timer: 0.0,
            block_timer: 0.0, zip_cooldown: 0.0, dodge: None, taunt: None, wander: (None, 0.0), flash: 0.0, other: 0, follow: 0,
            invulnerable: false, foe: None, truce: false, colour: 0, group: 0, zone: 0, goal: None, via: None, leap: None, steer: None, slide_odd: 0, slid_odd: None, patrol: None, blocked: 0.0, walk_check: (Vec3::ZERO, 0.0), alert: None, victim: None, home: None, landed: 0.0, landed_for: 0.0, bully_clock: 0.0, fidget: None, struggle: 0.0, idle_for: 0.0, breathed: false, screamed: false, fly_kick: None, gun_fight: false, topple: None, bonus_rolled: false, run_shot: 0.0, fetch: None, splattered: false, sized_up: false, felon: false, slot: None, ganged: false, fidget_roll: (way as u32).wrapping_mul(2654435761) ^ kind as u32, shot_wait: 1.0, aggro: false, attack_timer: 1.0, cues: Vec::new(), step_phase: 0.0, footfall: None, mucky: 0,
            peeing: false, stamina: 128.0, max_stamina: 128.0, winded: false, sprinting: false, explosives: 0, grenades: 0, hook: false, has_key: false, can: None, can_near: false, inventory: 0.0, protected: false, guilty: false, fake: false, unseen: 0.0, scared: None, annoyed: false, hurt_by: None, blown_up: false, crackle: 0.0, heard: None,
        }
    }

    /// A gun in hand is out: always for the player, only in a fight for the AI (it's
    /// holstered otherwise).
    pub fn gun_out(&self) -> bool {
        self.player || self.aggro || matches!(self.state, State::Shoot { .. })
    }

    /// Steps up to a switch (`PRIM_OBJ_SWITCH_OFF`, `switch`) or a valve at `ob` facing `yaw`
    /// (0..2047), and presses it or turns it (interfac.cpp:1665): if she's within 0x40 of
    /// where it's worked from, a quarter or three quarters of a square in front of it, and
    /// faces it to within 200 of 2048. Returns whether she did.
    pub fn work_switch(&mut self, ob: Vec3, yaw: i32, switch: bool) -> bool {
        if !matches!(self.state, State::Move) {
            return false;
        }
        let a = yaw as f32 * std::f32::consts::TAU / 2048.0;
        let out = vec3(a.sin(), 0.0, a.cos());
        let want = ob + out * if switch { 64.0 } else { 256.0 - 64.0 };
        let (dx, dz) = ((want.x - self.fig.pos.x).abs(), (want.z - self.fig.pos.z).abs());
        // `QDIST2`: the longer side and half the shorter.
        if dx.max(dz) + dx.min(dz) / 2.0 >= 0x40 as f32 {
            return false;
        }
        let mine = (self.fig.angle / std::f32::consts::TAU * 2048.0).round() as i32;
        let mut dangle = (mine - yaw).rem_euclid(2048);
        if dangle > 1024 {
            dangle -= 2048;
        }
        if dangle.abs() >= 200 {
            return false;
        }
        self.fig.pos = vec3(want.x, self.fig.pos.y, want.z);
        self.fig.angle = a;
        if switch {
            self.fig.restart(anim::BUTTON, Play::Once);
            self.state = State::Press { then: None };
        } else {
            self.fig.restart(anim::VALVE_LOOP, Play::Once);
            self.state = State::Press { then: Some(anim::VALVE_END) };
        }
        true
    }

    /// Picks up weapon `w` (a gun comes loaded).
    pub fn give(&mut self, w: Weapon) {
        if w == Weapon::Hook {
            self.hook = true;
        } else if let Some(s) = w.slot() {
            self.ai_fired = 0;
            if self.guns & (1 << s) == 0 {
                self.spent[s] = 0;
            }
            self.guns |= 1 << s;
            self.ammo[s] += w.clip();
        } else if w == Weapon::Knife {
            self.knife = true;
        } else if w == Weapon::Bat {
            self.bat = true;
        }
    }

    pub fn has(&self, w: Weapon) -> bool {
        if w == Weapon::Explosives {
            return self.explosives > 0;
        }
        if w == Weapon::Grenade {
            return self.grenades > 0;
        }
        if w == Weapon::Hook {
            return self.hook;
        }
        match w.slot() {
            Some(s) => self.guns & (1 << s) != 0,
            None => match w {
                Weapon::Knife => self.knife,
                Weapon::Bat => self.bat,
                _ => w == Weapon::None,
            },
        }
    }

    /// Rounds left in the gun in hand (the AI never runs out).
    pub fn rounds(&self) -> Option<u32> {
        if self.weapon == Weapon::Explosives {
            return Some(self.explosives as u32);
        }
        if self.weapon == Weapon::Grenade {
            return Some(self.grenades as u32);
        }
        self.weapon.slot().map(|s| if self.player { self.ammo[s] } else { u32::MAX })
    }

    /// Rounds left for weapon `w`, carried or in hand: a gun's, the charges' or the grenades'.
    pub fn rounds_of(&self, w: Weapon) -> Option<u32> {
        match w {
            Weapon::Explosives => Some(self.explosives as u32),
            Weapon::Grenade => Some(self.grenades as u32),
            _ => w.slot().map(|s| self.ammo[s]),
        }
    }

    /// Everything carried besides what's in hand, with the rounds left in each gun: the
    /// original's panel lists these too (`PANEL_ADDWEAPON`), so a weapon picked up or handed
    /// over at the start can be seen without cycling to it.
    pub fn stowed(&self) -> Vec<(Weapon, Option<u32>)> {
        let melee = [(self.knife, Weapon::Knife), (self.bat, Weapon::Bat)].into_iter().filter(|&(c, _)| c).map(|(_, w)| (w, None));
        let guns = Weapon::GUNS.iter().enumerate()
            .filter(|(s, _)| self.guns & (1 << s) != 0)
            .map(|(s, &g)| (g, Some(self.ammo[s])));
        let charges = (self.explosives > 0).then_some((Weapon::Explosives, Some(self.explosives as u32)));
        let grenades = (self.grenades > 0).then_some((Weapon::Grenade, Some(self.grenades as u32)));
        let hook = self.hook.then_some((Weapon::Hook, None));
        melee.chain(guns).chain(charges).chain(grenades).chain(hook).filter(|&(w, _)| w != self.weapon).collect()
    }

    /// What can be taken out, in the order the weapon key goes through them: bare hands
    /// first, as the original's panel lists them (`PANEL_inventory`).
    pub fn carried(&self) -> Vec<Weapon> {
        [Weapon::None, Weapon::Knife, Weapon::Bat, Weapon::Pistol, Weapon::Shotgun, Weapon::Ak47, Weapon::Sniper, Weapon::Explosives, Weapon::Grenade, Weapon::Hook]
            .into_iter()
            .filter(|&w| self.has(w))
            .collect()
    }

    /// The best weapon to take out bare-handed (`CONTROLS_get_best_item`,
    /// fallen/Source/Controls.cpp:1140): the AK-47, then the shotgun, then the pistol, each
    /// with rounds left, then the knife, the bat and a grenade.
    fn best_weapon(&self) -> Option<Weapon> {
        let loaded = |w: Weapon| self.has(w) && w.slot().is_some_and(|s| self.ammo[s] > 0);
        [Weapon::Ak47, Weapon::Shotgun, Weapon::Pistol].into_iter().find(|&w| loaded(w))
            .or_else(|| [Weapon::Knife, Weapon::Bat, Weapon::Grenade].into_iter().find(|&w| self.has(w)))
    }

    /// The player's best weapon with rounds left, to go to when the one in hand is out
    /// (`get_persons_best_weapon_with_ammo`, Person.cpp:6649): the AK-47, the shotgun, the
    /// pistol, then the bat and the knife.
    fn best_with_ammo(&self) -> Option<Weapon> {
        let loaded = |w: Weapon| self.has(w) && w.slot().is_some_and(|s| self.ammo[s] > 0);
        [Weapon::Ak47, Weapon::Shotgun, Weapon::Pistol].into_iter().find(|&w| loaded(w))
            .or_else(|| [Weapon::Bat, Weapon::Knife].into_iter().find(|&w| self.has(w)))
    }

    /// Puts away what's in hand and takes out the next weapon carried (bare hands last), and
    /// shows the inventory. Bare-handed with it not on show, the best weapon comes out
    /// instead (Controls.cpp:2370).
    pub fn next_weapon(&mut self) {
        // Not with a can in hand.
        if self.can.is_some() {
            return;
        }
        let carried = self.carried();
        let best = (self.weapon == Weapon::None && self.inventory <= 0.0).then(|| self.best_weapon()).flatten();
        let at = carried.iter().position(|&w| w == self.weapon).unwrap_or(0);
        self.weapon = best.unwrap_or(carried[(at + 1) % carried.len()]);
        self.inventory = 3.0;
    }

    /// The weapon key: on to the next weapon, and getting it out. Pressed while getting one
    /// out, it goes on to the next, as the inventory takes the key whatever she's doing
    /// (`CONTROLS_set_inventory`, Controls.cpp:1004). Whether she's getting one out.
    pub(super) fn switch_weapon(&mut self) -> bool {
        let was = self.weapon;
        self.next_weapon();
        self.cues.push(Sfx::Pickup);
        if self.weapon != was {
            self.draw_weapon();
        }
        matches!(self.state, State::Draw)
    }

    /// Gets the weapon now in hand out, stood still (`set_person_draw_item`,
    /// Person.cpp:19630): the pistol's draw unless it's the shotgun's or the AK's (one), the
    /// knife's or the bat's. Putting something away has no animation.
    pub(super) fn draw_weapon(&mut self) {
        // "You can't be in fight mode with a gun out!" (`set_person_draw_gun`, Person.cpp:8132).
        self.gun_fight = false;
        let a = match self.weapon {
            Weapon::None => {
                if matches!(self.state, State::Draw) {
                    self.state = State::Move;
                    self.idle(false);
                }
                return;
            }
            Weapon::Shotgun | Weapon::Ak47 => anim::SHOTGUN_DRAW,
            Weapon::Knife => anim::KNIFE_DRAW,
            Weapon::Bat => anim::BAT_DRAW,
            _ => anim::PISTOL_DRAW,
        };
        if self.fig.restart(a, Play::Once) {
            self.state = State::Draw;
        }
    }

    /// The player hit in a fight goes into fight mode (combat.cpp:2183), gun or no gun.
    pub(crate) fn hit_in_a_fight(&mut self) {
        self.gun_fight |= self.player;
    }

    /// Whether what they're holding shows in their hand: not until the fourth frame of
    /// getting it out (`FrameIndex == 3`, Person.cpp:18483).
    pub fn in_hand(&self) -> bool {
        !matches!(self.state, State::Draw) || self.fig.frame().is_some_and(|f| f >= 3)
    }

    /// An enemy that has noticed the player and is fighting.
    pub fn aggro(&self) -> bool {
        self.aggro && !self.is_down() && !self.removed
    }

    /// Arrests are Darci's own: `set_person_croutch` only looks for someone to cuff when the
    /// person doing it is `PERSON_DARCI`.
    pub fn can_arrest(&self) -> bool {
        self.player && self.kind == uc_world::character::person::DARCI
    }

    /// Can be arrested, as `find_arrestee` (fallen/Source/Person.cpp) has it:
    /// - knocked down and waiting to get up, not dead (`is_person_ko_and_lay_down`). Someone
    ///   out of health is `STATE_DEAD` in the original, which it never cuffs. The one
    ///   exception is a fight-test dummy, which its lesson move "knocks out" and the tutorial
    ///   then asks for an arrest of;
    /// - not invulnerable (dummies again excepted, which the original makes invulnerable);
    /// - not a Man In Black;
    /// - not a cop, unless a guilty one.
    ///
    /// The player's own bodyguards are ruled out by the game, which knows whose they are.
    pub fn arrestable(&self) -> bool {
        use uc_world::character::person as ptype;
        if !self.is_down() || self.arrested || self.removed || matches!(self.state, State::Cuffed) {
            return false;
        }
        if self.is_dummy() {
            return true;
        }
        !self.knocked_out && !self.invulnerable && !self.protected
            && !matches!(self.kind, ptype::MIB1 | ptype::MIB2 | ptype::MIB3)
            && (self.kind != ptype::COP || self.guilty)
    }

    /// `is_person_guilty` (fallen/Source/Person.cpp:2763): marked guilty, a Wildcat, one of
    /// the Men In Black, or a guard or gang member.
    pub fn is_guilty(&self) -> bool {
        use uc_world::character::person as ptype;
        self.guilty
            || matches!(self.kind, ptype::THUG_RASTA | ptype::THUG_RED | ptype::THUG_GREY | ptype::MIB1 | ptype::MIB2 | ptype::MIB3)
            || matches!(self.ai, AI_GUARD | AI_GANG)
    }

    /// `PCOM_person_doing_nothing_important` (pcom.cpp:1404), as far as this engine's people
    /// go: on their feet and about their own business, not fighting, running, driving,
    /// carrying or being carried.
    pub fn doing_nothing_important(&self) -> bool {
        matches!(self.state, State::Move) && !self.removed && !self.is_dead() && !self.aggro && self.scared.is_none()
    }

    /// Hands up to `by`'s gun (`PCOM_set_person_ai_hands_up`, pcom.cpp:4677): turned to face
    /// them, `ANIM_HANDS_UP`.
    pub fn hands_up(&mut self, by: usize, at: Vec3) {
        self.face(at);
        self.fig.restart(anim::HANDS_UP, Play::Once);
        self.state = State::HandsUp { by, time: 0.0 };
    }

    /// Hands down again (`PCOM_set_person_ai_normal`).
    pub fn hands_down(&mut self) {
        self.state = State::Move;
        self.idle(false);
    }

    /// Stops going for anyone (capture the flag's bot getting on with the game).
    pub(crate) fn calm_down(&mut self) {
        self.aggro = false;
    }

    /// Goes for `by` (`PCOM_set_person_ai_navtokill`): those who fight anyway notice them at
    /// once; anyone else guilty takes up the fight as a gang member would.
    pub fn go_for(&mut self) {
        if !self.fights() {
            self.ai = AI_GANG;
        }
        if !self.aggro {
            self.aggro = true;
            self.cues.push(Sfx::Alert { kind: self.kind });
            self.attack_timer = 0.8;
        }
    }

    /// Looks for a car and drives it (`PCOM_AI_DRIVER`, and `PCOM_AI_COP_DRIVER` which falls
    /// through to the same code, pcom.cpp:11313), unless something has frightened them.
    pub fn drives(&self) -> bool {
        matches!(self.ai, AI_DRIVER | AI_COP_DRIVER) && self.scared.is_none()
    }

    /// `might_i_be_a_villain` (Person.cpp:17917): anyone but Darci, Roper, a cop or a hostage.
    pub fn might_be_a_villain(&self) -> bool {
        use uc_world::character::person as ptype;
        !matches!(self.kind, ptype::DARCI | ptype::ROPER | ptype::COP | ptype::HOSTAGE)
    }

    /// The panel's stamina marks: a mark a 25 of breath, up to five
    /// (DDEngine/Source/panel.cpp:5021).
    pub fn stamina_marks(&self) -> usize {
        (self.stamina.max(0.0) as usize / 25).min(5)
    }

    /// `am_i_a_thug`: one of the three gangs.
    pub fn thug(&self) -> bool {
        use uc_world::character::person as ptype;
        matches!(self.kind, ptype::THUG_RASTA | ptype::THUG_GREY | ptype::THUG_RED)
    }

    pub fn is_dummy(&self) -> bool {
        self.ai == crate::script::PCOM_AI_FIGHT_TEST
    }

    /// Taunts at all (`PCOM_set_person_ai_taunt`): never cops, nor anyone helpless or held.
    fn can_taunt(&self) -> bool {
        self.kind != uc_world::character::person::COP && matches!(self.state, State::Move | State::Taunt { .. })
    }

    /// Attacks the player on sight (`PCOM_AI_GANG`, `_GUARD`, `_BULLY`, fight-test dummies...).
    pub fn hostile(&self) -> bool {
        // A robot ignores everything, but an assassin still goes for its mark (`PCOM_AI_ASSASIN`
        // and `_SHOOT_DEAD` in `PCOM_process_person` don't look at `PCOM_BENT_ROBOT`): The
        // Fallen's Wildcat holding Pete at gunpoint is one.
        !self.player && (matches!(self.ai, 3 | 21) || self.bent & BENT_ROBOT == 0 && matches!(self.ai, 2 | 4 | 6 | 12 | 13 | 17 | 18 | 19))
    }

    /// Closes in on its `foe` and swings: anyone hostile to the player, and a bodyguard who
    /// has someone to see off (`PCOM_find_bodyguard_victim` hands them a target).
    pub fn fights(&self) -> bool {
        self.hostile() || (self.ai == AI_BODYGUARD && self.bent & BENT_ROBOT == 0 && self.foe.is_some())
            // A cop after a felon (`PCOM_AI_STATE_ARREST`).
            || (self.ai == AI_COP && self.victim.is_some())
    }

    /// How close the player must come before this enemy notices (world units): guards watch
    /// their post, gangs and MIB look further, assassins and followers hunt from anywhere.
    fn aggro_range(&self) -> f32 {
        if self.mv == 5 {
            return 64.0 * 256.0;
        }
        match self.ai {
            // A bodyguard's sphere for someone going for their client (`0x800`).
            AI_BODYGUARD => crate::GUARDS * 256.0,
            2 => 4.0 * 256.0,
            6 | 13 => 5.0 * 256.0,
            4 | 18 | 19 => 8.0 * 256.0,
            3 | 17 | 21 => 64.0 * 256.0,
            _ => AGGRO_RANGE,
        }
    }

    pub fn is_down(&self) -> bool {
        matches!(self.state, State::Down | State::Cuffed)
    }

    /// Where the body is on the ground: below the hips (for lying people, away from the
    /// figure's origin).
    pub fn body_pos(&self) -> Vec3 {
        let h = self.fig.hips();
        vec3(h.x, self.fig.pos.y, h.z)
    }

    /// Knocked out for good and bleeding: how far the blood has spread (0..1).
    pub fn blood(&self) -> Option<f32> {
        (self.knocked_out && !self.arrested && !self.player).then(|| (self.down_timer / 3.0).min(1.0))
    }

    /// Out of the fight for good: knocked out, arrested or gone.
    pub fn is_dead(&self) -> bool {
        self.knocked_out || self.arrested || self.removed || matches!(self.state, State::Cuffed)
    }

    pub fn grabbing(&self) -> bool {
        matches!(self.state, State::Grapple { grab: Grab::Hold, .. })
    }

    /// Lying knocked down and waiting to get up (or out cold), not dead, cuffed or lying
    /// with their hands behind their head.
    pub fn floored(&self) -> bool {
        matches!(self.state, State::Down) && !self.knocked_out && !self.arrested && !self.removed
            && self.fig.anim() != anim::HANDS_UP_LIE
    }

    /// Doubled over from a kick between the legs, or straightening up again.
    pub fn doubled(&self) -> bool {
        matches!(self.state, State::Hurt) && matches!(self.fig.anim(), anim::KICK_NAD_TAKE | anim::KICK_NAD_STUNNED | anim::KICK_NAD_RECOVER)
    }

    /// Has what a kick between the legs hurts most (`SOUND_Gender` 1): all but Darci, the
    /// women and the hostage.
    pub fn male(&self) -> bool {
        !uc_world::character::person::uses_darci_meshes(self.kind)
    }

    pub fn snap(&self) -> Snap {
        Snap {
            // Where the body is: a knockdown carries it away from the figure's origin (standing,
            // the origin is steadier: hit reactions only lean the body).
            pos: if self.is_down() || matches!(self.state, State::Injured) { self.body_pos() } else { self.fig.pos },
            facing: self.fig.facing(),
            present: !self.removed,
            standing: !self.removed && matches!(self.state, State::Move | State::Attack { .. } | State::Recover { .. } | State::Hurt | State::Land { .. } | State::Block { .. } | State::Shoot { .. } | State::HugWall { .. } | State::Draw | State::Step),
            down: self.is_down(),
            arrested: self.arrested,
            arrestable: self.arrestable(),
            kind: self.kind,
            player: self.player,
            hostile: self.hostile(),
            fighter: self.hostile() || self.is_dummy(),
            dummy: self.is_dummy(),
            attacking: matches!(self.state, State::Attack { .. }),
            blocking: matches!(self.state, State::Block { .. }),
            in_car: matches!(self.state, State::InCar { .. }),
            injured: matches!(self.state, State::Injured),
            crouching: self.crouching(),
            doubled: self.doubled(),
            gun_out: self.weapon.is_gun() && self.gun_out(),
            floored: self.floored(),
            noise: match self.state {
                _ if self.removed => 0.0,
                // A landing, as hard as they came down (`PCOM_SOUND_DROP`, `_MED`, `_BIG`,
                // Darci.cpp:444), whatever it left them doing.
                _ if self.landed_for > 0.0 => self.landed,
                State::Attack { .. } => 0x900 as f32,
                State::Move if running_anim(self.fig.anim()) => 0x280 as f32,
                _ => 0.0,
            },
            zone: 0,
            footsteps: !self.removed && matches!(self.state, State::Move) && running_anim(self.fig.anim()),
        }
    }

    /// Back on their feet somewhere else, as if new: one of the city's extra people moved
    /// on by `PCOM_do_regen`, which clears being knocked out or arrested.
    /// On her feet and doing nothing, wherever she was (after a teleport).
    pub fn stand_still(&mut self) {
        self.state = State::Move;
        self.idle(false);
    }

    pub fn respawn(&mut self, pos: Vec3) {
        self.fig.pos = pos;
        self.state = State::Move;
        (self.knocked_out, self.arrested, self.removed, self.out_cold) = (false, false, false, false);
        self.health = self.max_health;
        (self.scared, self.dodge, self.aggro) = (None, None, false);
        self.hurt_by = None;
        self.wander = (None, 0.0);
        self.down_timer = 0.0;
        self.unseen = 0.0;
        self.idle(false);
    }

    /// One frame. `snaps` is everybody's state at the start of the frame.
    #[allow(clippy::too_many_arguments)]
    pub fn step(&mut self, me: usize, c: &Controls, snaps: &[Snap], ground: &Ground, map: &Map, dt: f32, events: &mut Vec<Event>) {
        if self.removed {
            return;
        }
        self.sprinting = false;
        if self.player {
            self.breathe(c, dt);
            self.inventory = (self.inventory - dt).max(0.0);
            // Dancing along with someone (`FLAG_PERSON_NO_RETURN_TO_NORMAL`): until she's
            // asked to do something else.
            if matches!(self.state, State::Pose { .. }) && c.any() {
                self.state = State::Move;
                self.idle(false);
            }
        }
        // Jumps, landings and knockdowns draw the body offset from where the animation puts it.
        self.fig.cancel_rise = matches!(self.state, State::Fall { .. } | State::Land { .. });
        self.fig.pin_root = matches!(self.state, State::Fence { .. });
        match self.state {
            State::Fall { .. } => self.fig.lift = 0.0,
            State::Land { .. } => {
                // Landing a little below a ledge's top: rise onto it over a few frames.
                let l = self.fig.lift;
                self.fig.lift = (l - 600.0 * dt).max(0.0);
            }
            State::Down | State::Cuffed => {
                // Settle a knockdown's height change onto the floor, at falling speed.
                let l = self.fig.lift;
                self.fig.lift = l.signum() * (l.abs() - 900.0 * dt).max(0.0);
            }
            _ => self.fig.lift = 0.0,
        }
        if !matches!(self.state, State::Fall { .. }) {
            self.fly_kick = None;
        }
        if !matches!(self.state, State::Down) {
            self.topple = None;
        }
        match self.state.clone() {
            State::Move if self.slipping(ground, map).is_some() => {
                self.state = State::Slip;
                self.fig.play(anim::FALLING, Play::Loop);
            }
            State::Move => self.step_move(me, c, snaps, ground, map, dt, events),
            State::Slip => self.step_slip(ground, map, dt),
            State::Flip { left } => self.step_flip(left, ground, map, dt),
            State::Float { floor } => {
                if self.fig.pos.y - floor < FLOAT_HEIGHT {
                    self.fig.pos.y = (self.fig.pos.y + FLOAT_SPEED * dt).min(floor + FLOAT_HEIGHT);
                }
            }
            State::Block { auto } => {
                let target = self.nearest_opponent(me, snaps, FIGHT_RANGE);
                if let Some(t) = target {
                    self.fig.turn_towards(flat(snaps[t].pos, self.fig.pos), 8.0, dt);
                }
                // Kick from a guard: the foot sweep (Person.cpp:18720).
                if self.player && c.kick {
                    if let Some(t) = target {
                        self.face(snaps[t].pos);
                    }
                    self.start_attack(me, Move::LegSweep, events);
                    return;
                }
                // The auto-block holds while she keeps backing off (`continue_blocking`,
                // interfac.cpp:8424) and someone close is still swinging; the other while the
                // key is held.
                let swinging = snaps.iter().enumerate()
                    .any(|(j, s)| j != me && s.attacking && self.opponents(j, s) && flat(s.pos, self.fig.pos).length() < 220.0);
                let held = if auto { c.wish.dot(-self.fig.facing()) > 0.5 && swinging } else { c.block };
                if !held {
                    self.state = State::Move;
                    self.idle(true);
                }
            }
            // Every 32 turns, one time in five, a struggle (`SUB_STATE_DEAD_INJURED`,
            // Person.cpp:17479); not a fight-test dummy, who'd flip over doing it.
            State::Injured => {
                if self.fig.anim() == anim::INJURED_LOOP_STRUGGLE && !self.fig.finished() {
                    return;
                }
                let left = self.fidget.map_or(INJURED_BEAT, |t| t.min(INJURED_BEAT)) - dt;
                self.fidget = Some(if left > 0.0 { left } else { INJURED_BEAT });
                self.fidget_roll = self.fidget_roll.wrapping_mul(69069).wrapping_add(1);
                if left <= 0.0 && (self.fidget_roll >> 16) & 0xff < 50 && !self.is_dummy()
                    && self.fig.restart(anim::INJURED_LOOP_STRUGGLE, Play::Once)
                {
                    return;
                }
                self.fig.play(anim::INJURED_LOOP, Play::Loop);
            }
            State::Carried { .. } | State::Enter { .. } | State::Bike { .. } => {}
            // Sitting: down, and stays down, shifting every 64 turns (`ANIM_SIT_IDLE`,
            // Person.cpp:16489). They get up again in `decide` when something needs them; the
            // player when pushing any way (interfac.cpp:4727), standing up first
            // (`set_person_unsit`, Person.cpp:15328).
            State::Sit => {
                if self.player && c.wish.length() > 0.1 {
                    self.fidget = None;
                    if self.fig.restart(anim::SIT_TO_STAND, Play::Once) {
                        self.state = State::Press { then: None };
                    } else {
                        self.state = State::Move;
                        self.idle(false);
                    }
                } else if matches!(self.fig.anim(), anim::SIT_DOWN | anim::SIT_IDLE) && self.fig.finished() {
                    let left = self.fidget.map_or(SIT_FIDGET, |t| t.min(SIT_FIDGET)) - dt;
                    self.fidget = Some(if left > 0.0 { left } else { SIT_FIDGET });
                    if left <= 0.0 {
                        self.fig.restart(anim::SIT_IDLE, Play::Once);
                    }
                }
            }
            State::Pose { anim: a } => {
                if a == anim::HANDS_UP {
                    if self.fig.anim() != anim::HANDS_UP_LOOP && (self.fig.anim() != anim::HANDS_UP || self.fig.finished()) {
                        self.fig.play(anim::HANDS_UP_LOOP, Play::Loop);
                    }
                } else {
                    self.fig.play(a, Play::Loop);
                }
            }
            State::HandsUp { by, time } => {
                // `ANIM_HANDS_UP`, then held there (`ANIM_HANDS_UP_LOOP`), facing the gun.
                if snaps[by].present {
                    self.face(snaps[by].pos);
                }
                if self.fig.anim() != anim::HANDS_UP_LOOP && (self.fig.anim() != anim::HANDS_UP || self.fig.finished()) {
                    self.fig.play(anim::HANDS_UP_LOOP, Play::Loop);
                }
                self.state = State::HandsUp { by, time: time + dt };
            }
            State::Dangle { to, dir, shift } => self.step_dangle(to, dir, shift, c, ground, map),
            State::Carry { victim, phase } => self.step_carry(victim, phase, c, ground, map, dt),
            State::HugWall { a, b, out, hug } => self.step_hug(a, b, out, hug, c, ground, map, dt),
            State::Crouch { phase } => {
                // Up from a crouch or a crawl straight into the takedown, if someone's back
                // is to her.
                if matches!(phase, 1 | 2) && c.punch && self.takedown(me, snaps, events) {
                    return;
                }
                self.step_crouch(phase, c, ground, map, dt)
            }
            State::GetUp => {
                if self.fig.finished() {
                    self.state = State::Move;
                    self.ready(self.nearest_opponent(me, snaps, FIGHT_RANGE).is_some(), false);
                }
            }
            State::Shoot { target, shots } => self.step_shoot(target, shots, me, c, snaps, ground, map, dt, events),
            State::Fence { phase, at, over, base, top, from } => self.step_fence(phase, at, over, base, top, from, c, dt),
            State::Zip { cable, t, dir, speed, ridden } => self.step_zip(cable, t, dir, speed, ridden, c, ground, map, dt),
            State::Attack { mv, hit, queued } => self.step_attack(mv, hit, queued, me, c, snaps, ground, map, dt, events),
            State::Slide { phase, speed, hit } => self.step_slide(phase, speed, hit, me, snaps, ground, map, dt, events),
            State::Land { vel } => self.step_land(vel, me, c, snaps, ground, map, dt, events),
            State::Hurt => {
                // The player can cut a hit reaction short by fighting back or jumping away.
                if self.player && self.fig.progress() > 0.35 && (c.punch || c.kick || c.jump) {
                    self.state = State::Move;
                    self.step_move(me, c, snaps, ground, map, dt, events);
                    return;
                }
                if self.fig.finished() {
                    // Kicked between the legs: bent double for a while, then straightening up
                    // (`fn_person_recoil`).
                    let next = match self.fig.anim() {
                        anim::KICK_NAD_TAKE => Some(anim::KICK_NAD_STUNNED),
                        anim::KICK_NAD_STUNNED => Some(anim::KICK_NAD_RECOVER),
                        _ => None,
                    };
                    if !next.is_some_and(|a| self.fig.restart(a, Play::Once)) {
                        self.state = State::Move;
                        self.idle(self.nearest_opponent(me, snaps, FIGHT_RANGE).is_some());
                    }
                }
            }
            State::Down => self.step_down(ground, map, dt),
            State::Cuffed => self.step_cuffed(dt),
            State::Held { .. } | State::InCar { .. } => {}
            State::Fall { vy, vel } => self.step_fall(vy, vel, me, c, snaps, ground, map, dt, events),
            State::Vault { from, to } => {
                self.fig.pos = from.lerp(to, self.fig.progress());
                if self.fig.finished() {
                    let floor = ground.surface(map, to.x, to.z, to.y + STEP_HEIGHT);
                    self.fig.pos = vec3(to.x, floor, to.z);
                    self.state = State::Move;
                    self.fig.play(self.run_anims(false, false).0, Play::Loop);
                }
            }
            State::PullUp { start, end, to } => {
                self.fig.pos = start.lerp(end, self.fig.progress());
                if self.fig.finished() {
                    // The last frame is a standing pose drawn `end - to` lower: swap both at once.
                    self.fig.pos = to;
                    self.fig.restart_now(anim::BREATHE, Play::Loop);
                    self.state = State::Move;
                    self.idle(false);
                }
            }
            State::Climb { ladder, y0, loops0 } => self.step_climb(ladder, y0, loops0, c, ground, map),
            State::Mount { ladder } => self.step_mount(ladder, ground),
            State::OntoLadder { ladder, start, end } => self.step_onto_ladder(ladder, start, end),
            State::Step => {
                if self.fig.finished() {
                    self.state = State::Move;
                    self.ready(false, true);
                }
            }
            State::Grapple { victim, grab: grab @ (Grab::Strangle | Grab::Headbutt), hit, timer, frame } => {
                // `SUB_STATE_STRANGLE` and `_HEADBUTT` (Person.cpp:18932): the damage comes on
                // certain frames, and when it's over the victim is left lying (or dead).
                let now = self.fig.frame();
                if now != frame {
                    let strangle = grab == Grab::Strangle;
                    let landed = match now {
                        Some(f) if strangle => f == 27 || (f % 2 == 0 && (14..=24).contains(&f)),
                        Some(f) => f == 11,
                        None => false,
                    };
                    if landed && snaps[victim].present {
                        events.push(Event::Squeeze { victim, headbutt: !strangle });
                    }
                }
                if self.fig.finished() {
                    events.push(Event::Hit { attacker: me, victim, damage: 0, ko: ko::GRAPPLE, high: false, low: false, weapon: Weapon::None });
                    self.state = State::Move;
                    self.idle(true);
                    return;
                }
                self.state = State::Grapple { victim, grab, hit, timer, frame: now };
            }
            State::Grapple { victim, grab: Grab::PistolWhip, .. } => {
                // `SUB_STATE_GRAPPLE`: when it's played out, the one whipped goes down out cold.
                if self.fig.finished() {
                    events.push(Event::OutCold { attacker: me, victim });
                    self.state = State::Move;
                    self.idle(false);
                }
            }
            State::Grapple { victim, grab, hit, timer, frame } => self.step_grapple(victim, grab, hit, timer, frame, me, c, snaps, dt, events),
            State::Hook { released, windup } => {
                // The wind-up spins it round her head; then the release (or a grenade's throw),
                // which lets it go a few frames in. Winding it back in is the pick-up, and nothing to let go.
                if windup {
                    if self.fig.finished() {
                        if self.fig.restart_now(crate::game::hook::RELEASE, Play::Once)
                            || self.fig.restart_now(crate::game::grenades::ANIM_CAN_RELEASE, Play::Once)
                        {
                            self.state = State::Hook { released: false, windup: false };
                        } else {
                            events.push(Event::Hook { thrower: me });
                            self.state = State::Move;
                            self.idle(false);
                        }
                    }
                } else {
                    if !released && self.fig.frame().is_some_and(|f| f >= 3) {
                        events.push(Event::Hook { thrower: me });
                        self.state = State::Hook { released: true, windup: false };
                    }
                    if self.fig.finished() {
                        if !released {
                            events.push(Event::Hook { thrower: me });
                        }
                        self.state = State::Move;
                        self.idle(false);
                    }
                }
            }
            State::Rope { at, dir, top, to, y0, loops0 } => self.step_rope(at, dir, top, to, y0, loops0, c),
            State::Press { then } => {
                if self.fig.finished() {
                    match then {
                        Some(a) => {
                            self.fig.restart(a, Play::Once);
                            self.state = State::Press { then: None };
                        }
                        None => {
                            self.state = State::Move;
                            self.idle(false);
                        }
                    }
                }
            }
            State::PickUp { got } => {
                // The fourth frame picks it up (`FrameIndex == 3`, Person.cpp:20722).
                if !got && self.fig.frame().is_some_and(|f| f >= 3) {
                    events.push(Event::PickUp { who: me });
                    self.state = State::PickUp { got: true };
                }
                if self.fig.finished() {
                    self.state = State::Move;
                    self.idle(false);
                }
            }
            State::Throw { released } => {
                // The fourth frame lets it go (`FrameIndex == 3`).
                if !released && self.fig.frame().is_some_and(|f| f >= 3) {
                    events.push(Event::Throw { thrower: me });
                    self.state = State::Throw { released: true };
                }
                if self.fig.finished() {
                    if !released {
                        events.push(Event::Throw { thrower: me });
                    }
                    self.state = State::Move;
                    self.idle(false);
                }
            }
            State::Recover { mut queued } => {
                if queued.is_none() && (c.punch || c.kick) {
                    queued = Some(if c.punch { Move::punch_with(self.weapon) } else { Move::Kick1 });
                }
                self.state = State::Recover { queued };
                if self.fig.finished() {
                    match queued {
                        Some(mv) => self.start_attack(me, mv, events),
                        None => {
                            self.state = State::Move;
                            self.idle(self.nearest_opponent(me, snaps, FIGHT_RANGE).is_some());
                        }
                    }
                }
            }
            State::Draw => {
                if c.switch && self.player && self.switch_weapon() {
                    return;
                }
                if self.fig.finished() {
                    self.state = State::Move;
                    self.idle(false);
                }
            }
            State::Taunt { at } => {
                // `PCOM_process_taunt`: always facing who they're taunting.
                if snaps[at].present {
                    self.fig.turn_towards(flat(snaps[at].pos, self.fig.pos), 8.0, dt);
                }
                if self.fig.finished() {
                    self.state = State::Move;
                    self.idle(false);
                }
            }
            State::Arrest { victim } => {
                if self.fig.finished() {
                    events.push(Event::Arrested { victim });
                    self.state = State::Move;
                    self.idle(false);
                }
            }
        }
    }

    fn face(&mut self, at: Vec3) {
        let d = flat(at, self.fig.pos);
        if d.length_squared() > 1.0 {
            self.fig.angle = uc_world::character::angle_of(d);
        }
    }

    /// Makes facing `(-sin, -cos)` point at `p` (used by the grapple and arrest set-ups).
    pub fn face_towards(&mut self, p: Vec3) {
        self.face(p);
    }

    pub fn facing_dir(&self) -> Vec2 {
        facing(self.fig.angle)
    }
}
