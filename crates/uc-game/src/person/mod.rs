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
            Weapon::Explosives | Weapon::Grenade => None,
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
/// `PCOM_AI_COP`.
pub const AI_COP: u8 = 5;
pub const AI_SUICIDE: u8 = 15;
/// `PCOM_MOVE_WANDER`.
pub const MOVE_WANDER: u8 = 4;
/// `PCOM_AI_HYPOCHONDRIA`: lies injured, waiting to be carried (`set_person_injured`).
pub const AI_INJURED: u8 = 20;
/// How far ahead of the one carrying them the carried person is put (`carry_running`).
const CARRY_AHEAD: f32 = 90.0;

/// Speeds in world units per second (a map square is 256). Chosen by eye.
pub const WALK_SPEED: f32 = 190.0;
/// Crawling: velocity 15 to walking's 16 (`SUB_STATE_CRAWLING` in `fn_person_moveing`).
pub const CRAWL_SPEED: f32 = WALK_SPEED * 15.0 / 16.0;
const RUN_SPEED: f32 = 520.0;
const SPRINT_SPEED: f32 = 700.0;
const FIGHT_STEP_SPEED: f32 = 140.0;
/// The ladder animation climbs this far per cycle (its hips rise 117 -> 245).
const RUNG_CYCLE: f32 = 128.0;
const GRAVITY: f32 = 2000.0;
/// Take-off speed of a jump: reaches about 0.4 squares.
const JUMP_SPEED: f32 = 640.0;
/// Ledges this low are jumped onto, not climbed (a standing jump clears them).
const JUMPABLE: f32 = 160.0;
/// While jumping, ledges this far above the feet can be grabbed.
const GRAB_REACH: f32 = 200.0;
const RADIUS: f32 = 36.0;
/// Opponents closer than this put a standing person into fight stance.
const FIGHT_RANGE: f32 = 400.0;
/// How close the attacker must be to grab or arrest.
const GRAB_RANGE: f32 = 120.0;
const ARREST_RANGE: f32 = 130.0;
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
    Footstep { player: bool },
    /// The effort of a jump, from whoever made it.
    Jump { kind: u8 },
    Land,
    Grab,
    /// Darci makes an arrest.
    Arrest,
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
    /// A car's tyres started sliding.
    Skid,
    /// Getting into or out of a car.
    CarDoor,
    Pickup,
    /// Something hidden turned up in a search (`S_HIDDENITEM`).
    Found,
    /// A bomb or a car going up.
    Explosion,
    /// The radio's beep before a message (`S_RADIO_MESSAGE`), heard wherever she is.
    Radio,
    /// A police siren (`S_SIREN_START`), from where it's given.
    Siren,
    /// One of the game's own sounds by id (`sound_list`), asked for by a mission.
    ById { id: usize, replace: bool },
    Won,
    Lost,
}

/// How fast someone heads for their `goal` (`PERSON_SPEED_*`, fallen/Headers/Person.h).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Pace {
    #[default]
    Walk,
    Run,
    Sprint,
}

impl Pace {
    /// The controls that move someone at this speed.
    fn controls(self) -> (bool, bool) {
        match self {
            Pace::Walk => (false, false),
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
    Knife1,
    Knife2,
    Knife3,
    Bat1,
    Bat2,
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
            // Weapons (damage from apply_hit_to_person, fallen/Source/combat.cpp).
            Knife1 => d(anim::KNIFE_ATTACK1, 0.45, 150.0, Dir::Front, 30, true, 0, Some(Knife2)),
            Knife2 => d(anim::KNIFE_ATTACK2, 0.45, 150.0, Dir::Front, 50, true, ko::COMBO_ANY, Some(Knife3)),
            Knife3 => d(anim::KNIFE_ATTACK3, 0.4, 160.0, Dir::Front, 70, false, ko::COMBO_ANY | ko::KNOCKDOWN, None),
            Bat1 => d(anim::BAT_HIT1, 0.5, 180.0, Dir::Front, 40, true, 0, Some(Bat2)),
            Bat2 => d(anim::BAT_HIT2, 0.5, 180.0, Dir::Front, 60, true, ko::COMBO_ANY | ko::KNOCKDOWN, None),
        }
    }
    /// Punches and weapon swings (the punch button).
    fn is_punch(self) -> bool {
        matches!(self, Move::Punch1 | Move::Punch2 | Move::Punch3 | Move::Knife1 | Move::Knife2 | Move::Knife3 | Move::Bat1 | Move::Bat2)
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
    /// Sliding tackle: phase 0 start, 1 slide, 2 get up.
    Slide { phase: u8, speed: f32, hit: u64 },
    Hurt,
    /// Lying on the ground: knocked down (getting up after a while) or knocked out.
    Down,
    GetUp,
    /// Guarding against blows from the front.
    Block,
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
    /// Sliding down a cable: `t` along it (0..1), moving towards `dir` (+1 or -1).
    /// `ridden`: how far along the cable so far (world units).
    Zip { cable: usize, t: f32, dir: f32, speed: f32, ridden: f32 },
    Fall { vy: f32, vel: Vec2 },
    /// Landing, still carrying some of the jump's speed.
    Land { vel: Vec2 },
    /// Pulling up onto a ledge (or off a ladder). The animation itself lifts the body, so
    /// `pos` moves from `start` to `end` while the drawn hips go from the edge to `to`.
    PullUp { start: Vec3, end: Vec3, to: Vec3 },
    /// Hanging by the hands from a ledge (`STATE_DANGLING`, `SUB_STATE_DANGLING`): `to` is
    /// where she'd stand on top after pulling up, `dir` the way she faces (into the wall).
    /// While going a hand along it (`SUB_STATE_TRAVERSE_LEFT`, `_RIGHT`), `shift` is the
    /// figure's start and end.
    Dangle { to: Vec3, dir: Vec2, shift: Option<(Vec3, Vec3)> },
    /// On a ladder: feet at `y0` plus one rung cycle per loop of the animation since `loops0`.
    Climb { ladder: usize, y0: f32, loops0: f32 },
    /// Grappling `victim`. `frame`: the last keyframe of Roper's grapples that did its damage.
    Grapple { victim: usize, grab: Grab, hit: bool, timer: f32, frame: Option<usize> },
    /// Taunting `at` (`ANIM_WANKER`), facing them all the while.
    Taunt { at: usize },
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
    /// Crouched, for as long as action is held (`set_person_croutch`): phase 0 going down,
    /// 1 down, 2 crawling (`SUB_STATE_CRAWLING`), 3 getting up (`SUB_STATE_IDLE_UNCROUCH`).
    Crouch { phase: u8 },
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
    /// How far off the noise they're making this frame is heard (`PCOM_oscillate_tympanum`'s
    /// radii, pcom.cpp:13124): running feet 0x280, a landing 0x200, a fight 0x900; 0 for none.
    pub noise: f32,
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
    /// `PCOM_MOVE_*`: 5 (follow) hunts the player down.
    pub mv: u8,
    /// What's in hand.
    pub weapon: Weapon,
    /// A knife or bat carried (drawn or not).
    pub melee: Weapon,
    /// The item (`IT_*`) they give up when searched, besides their weapons (`drop`).
    pub drop: Option<u8>,
    /// Searched already (`FLAG_PERSON_SEARCHED`): nothing more to find on them.
    pub searched: bool,
    /// Guns carried (bit per `Weapon::GUNS` slot), and rounds for each.
    pub guns: u8,
    pub ammo: [u32; 4],
    /// Out for good (health gone): won't get up.
    pub knocked_out: bool,
    /// Lying out cold but alive (the pistol whip's `STAY_ALIVE_PRONE`): down until cuffed.
    pub out_cold: bool,
    /// A mission has set them peeing (`FLAG_PERSON_PEEING`).
    pub peeing: bool,
    /// Charges carried (the special's `ammo`): four at most.
    /// Grenades carried (`SPECIAL_GRENADE`'s `ammo`).
    pub grenades: u8,
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
    /// Heard something there, and looking that way for this many seconds more
    /// (`PCOM_AI_STATE_INVESTIGATING`).
    pub heard: Option<(Vec3, f32)>,
    aggro: bool,
    attack_timer: f32,
    /// Sounds made this frame (collected by the game).
    pub cues: Vec<Sfx>,
    /// Animation phase at the last footstep check (half cycles are steps).
    step_phase: f32,
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
            arrested: false, removed: false, skill: 0, mv: 1, weapon: Weapon::None, melee: Weapon::None, drop: None, searched: false, guns: 0, ammo: [0; 4], knocked_out: false, out_cold: false, down_timer: 0.0,
            block_timer: 0.0, zip_cooldown: 0.0, dodge: None, taunt: None, wander: (None, 0.0), flash: 0.0, other: 0, follow: 0,
            invulnerable: false, foe: None, colour: 0, group: 0, goal: None, via: None, leap: None, steer: None, shot_wait: 1.0, aggro: false, attack_timer: 1.0, cues: Vec::new(), step_phase: 0.0,
            peeing: false, stamina: 128.0, max_stamina: 128.0, winded: false, sprinting: false, explosives: 0, grenades: 0, has_key: false, protected: false, guilty: false, fake: false, unseen: 0.0, scared: None, annoyed: false, hurt_by: None, blown_up: false, heard: None,
        }
    }

    /// A gun in hand is out: always for the player, only in a fight for the AI (it's
    /// holstered otherwise).
    pub fn gun_out(&self) -> bool {
        self.player || self.aggro || matches!(self.state, State::Shoot { .. })
    }

    /// Picks up weapon `w` (a gun comes loaded).
    pub fn give(&mut self, w: Weapon) {
        if let Some(s) = w.slot() {
            self.guns |= 1 << s;
            self.ammo[s] += w.clip();
        } else if w != Weapon::None {
            self.melee = w;
        }
    }

    pub fn has(&self, w: Weapon) -> bool {
        if w == Weapon::Explosives {
            return self.explosives > 0;
        }
        if w == Weapon::Grenade {
            return self.grenades > 0;
        }
        match w.slot() {
            Some(s) => self.guns & (1 << s) != 0,
            None => w == Weapon::None || self.melee == w,
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

    /// Everything carried besides what's in hand, with the rounds left in each gun: the
    /// original's panel lists these too (`PANEL_ADDWEAPON`), so a weapon picked up or handed
    /// over at the start can be seen without cycling to it.
    pub fn stowed(&self) -> Vec<(Weapon, Option<u32>)> {
        let melee = (self.melee != Weapon::None).then_some((self.melee, None));
        let guns = Weapon::GUNS.iter().enumerate()
            .filter(|(s, _)| self.guns & (1 << s) != 0)
            .map(|(s, &g)| (g, Some(self.ammo[s])));
        let charges = (self.explosives > 0).then_some((Weapon::Explosives, Some(self.explosives as u32)));
        let grenades = (self.grenades > 0).then_some((Weapon::Grenade, Some(self.grenades as u32)));
        melee.into_iter().chain(guns).chain(charges).chain(grenades).filter(|&(w, _)| w != self.weapon).collect()
    }

    /// Puts away what's in hand and takes out the next weapon carried (bare hands last).
    pub fn next_weapon(&mut self) {
        let carried: Vec<Weapon> = [Weapon::None, self.melee, Weapon::Pistol, Weapon::Shotgun, Weapon::Ak47, Weapon::Sniper, Weapon::Explosives, Weapon::Grenade]
            .into_iter()
            .enumerate()
            .filter(|&(k, w)| self.has(w) && (k == 0 || w != Weapon::None))
            .map(|(_, w)| w)
            .collect();
        let at = carried.iter().position(|&w| w == self.weapon).unwrap_or(0);
        self.weapon = carried[(at + 1) % carried.len()];
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

    pub fn is_dummy(&self) -> bool {
        self.ai == crate::script::PCOM_AI_FIGHT_TEST
    }

    /// Taunts at all (`PCOM_set_person_ai_taunt`): never cops, nor anyone helpless or held.
    fn can_taunt(&self) -> bool {
        self.kind != uc_world::character::person::COP && matches!(self.state, State::Move | State::Taunt { .. })
    }

    /// Attacks the player on sight (`PCOM_AI_GANG`, `_GUARD`, `_BULLY`, fight-test dummies...).
    pub fn hostile(&self) -> bool {
        !self.player && self.bent & BENT_ROBOT == 0 && matches!(self.ai, 2 | 3 | 4 | 6 | 12 | 13 | 17 | 18 | 19 | 21)
    }

    /// Closes in on its `foe` and swings: anyone hostile to the player, and a bodyguard who
    /// has someone to see off (`PCOM_find_bodyguard_victim` hands them a target).
    pub fn fights(&self) -> bool {
        self.hostile() || (self.ai == AI_BODYGUARD && self.bent & BENT_ROBOT == 0 && self.foe.is_some())
    }

    /// How close the player must come before this enemy notices (world units): guards watch
    /// their post, gangs and MIB look further, assassins and followers hunt from anywhere.
    fn aggro_range(&self) -> f32 {
        if self.mv == 5 {
            return 64.0 * 256.0;
        }
        match self.ai {
            // A bodyguard's sphere for someone going for their client (`0x800`).
            AI_BODYGUARD => 8.0 * 256.0,
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
            standing: !self.removed && matches!(self.state, State::Move | State::Attack { .. } | State::Hurt | State::Land { .. } | State::Block | State::Shoot { .. }),
            down: self.is_down(),
            arrested: self.arrested,
            arrestable: self.arrestable(),
            kind: self.kind,
            player: self.player,
            hostile: self.hostile(),
            fighter: self.hostile() || self.is_dummy(),
            dummy: self.is_dummy(),
            attacking: matches!(self.state, State::Attack { .. }),
            blocking: matches!(self.state, State::Block),
            in_car: matches!(self.state, State::InCar { .. }),
            injured: matches!(self.state, State::Injured),
            crouching: self.crouching(),
            doubled: self.doubled(),
            noise: match self.state {
                _ if self.removed => 0.0,
                State::Land { .. } => 0x200 as f32,
                State::Attack { .. } => 0x900 as f32,
                State::Move if matches!(self.fig.anim(), anim::RUN | anim::PISTOL_JOG | anim::AK_JOG) => 0x280 as f32,
                _ => 0.0,
            },
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
            State::Down => {
                // Settle a knockdown's height change onto the floor, at falling speed.
                let l = self.fig.lift;
                self.fig.lift = l.signum() * (l.abs() - 900.0 * dt).max(0.0);
            }
            _ => self.fig.lift = 0.0,
        }
        match self.state.clone() {
            State::Move => self.step_move(me, c, snaps, ground, map, dt, events),
            State::Float { floor } => {
                if self.fig.pos.y - floor < FLOAT_HEIGHT {
                    self.fig.pos.y = (self.fig.pos.y + FLOAT_SPEED * dt).min(floor + FLOAT_HEIGHT);
                }
            }
            State::Block => {
                if let Some(t) = self.nearest_opponent(me, snaps, FIGHT_RANGE) {
                    self.fig.turn_towards(flat(snaps[t].pos, self.fig.pos), 8.0, dt);
                }
                if !c.block {
                    self.state = State::Move;
                    self.idle(true);
                }
            }
            State::Injured => {
                self.fig.play(anim::INJURED_LOOP, Play::Loop);
            }
            State::Carried { .. } | State::Enter { .. } | State::Bike { .. } => {}
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
                    self.idle(self.nearest_opponent(me, snaps, FIGHT_RANGE).is_some());
                }
            }
            State::Shoot { target, shots } => self.step_shoot(target, shots, me, c, snaps, ground, map, dt, events),
            State::Fence { phase, at, over, base, top, from } => self.step_fence(phase, at, over, base, top, from, dt),
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
            State::Cuffed | State::Held { .. } | State::InCar { .. } => {}
            State::Fall { vy, vel } => self.step_fall(vy, vel, c, ground, map, dt),
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
