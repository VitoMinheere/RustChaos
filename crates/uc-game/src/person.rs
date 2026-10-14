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
}

impl Weapon {
    pub const GUNS: [Weapon; 3] = [Weapon::Pistol, Weapon::Shotgun, Weapon::Ak47];

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
        }
    }

    /// The weapon item `IT_*` `kind` is.
    pub fn from_item(kind: u8) -> Option<Weapon> {
        [Weapon::Pistol, Weapon::Shotgun, Weapon::Knife, Weapon::Ak47, Weapon::Bat].into_iter().find(|w| w.item() == Some(kind))
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
            Weapon::Explosives => None,
        }
    }

    /// Standing with the gun up (two-handed guns all use the shotgun's, `set_person_aim`).
    fn aim_anim(self) -> usize {
        match self {
            Weapon::Shotgun | Weapon::Ak47 => anim::SHOTGUN_AIM,
            _ => anim::PISTOL_AIM,
        }
    }

    fn fire_anim(self) -> usize {
        match self {
            Weapon::Shotgun => anim::SHOTGUN_FIRE,
            Weapon::Ak47 => anim::AK_FIRE,
            _ => anim::PISTOL_SHOOT,
        }
    }
}

/// `PCOM_BENT_ROBOT`: ignores everything and everyone.
pub const BENT_ROBOT: u8 = 1 << 5;
/// `PCOM_BENT_FIGHT_BACK`: hits back when set on.
pub const BENT_FIGHT_BACK: u8 = 1 << 3;
/// `PCOM_BENT_PLAYERKILL`: only the player can hurt them.
pub const BENT_PLAYERKILL: u8 = 1 << 7;
/// `PCOM_AI_BODYGUARD`: guards the person their `other` point created, and follows them.
pub const AI_BODYGUARD: u8 = 8;

/// Speeds in world units per second (a map square is 256). Chosen by eye.
pub const WALK_SPEED: f32 = 190.0;
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
/// Pistols: shoot from this far, close in from further.
const SHOOT_RANGE: f32 = 6.0 * 256.0;

/// Something worth hearing. The game reports these with a position; the app decides which
/// recordings to play.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Sfx {
    /// An attack swings.
    Swing,
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
}

impl Grab {
    /// (attacker, victim) animations.
    pub fn anims(self) -> (usize, usize) {
        match self {
            Grab::Grab | Grab::Hold => (anim::GRAB_ARM, anim::GRAB_ARMV),
            Grab::Knee1 => (anim::GRAB_ARM_KNEE1, anim::GRAB_ARM_KNEE1V),
            Grab::Knee2 => (anim::GRAB_ARM_KNEE2, anim::GRAB_ARM_KNEE2V),
            Grab::Throw => (anim::GRAB_ARM_THROW, anim::GRAB_ARM_THROWV),
        }
    }
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
    /// Sliding down a cable: `t` along it (0..1), moving towards `dir` (+1 or -1).
    /// `ridden`: how far along the cable so far (world units).
    Zip { cable: usize, t: f32, dir: f32, speed: f32, ridden: f32 },
    Fall { vy: f32, vel: Vec2 },
    /// Landing, still carrying some of the jump's speed.
    Land { vel: Vec2 },
    /// Pulling up onto a ledge (or off a ladder). The animation itself lifts the body, so
    /// `pos` moves from `start` to `end` while the drawn hips go from the edge to `to`.
    PullUp { start: Vec3, end: Vec3, to: Vec3 },
    /// On a ladder: feet at `y0` plus one rung cycle per loop of the animation since `loops0`.
    Climb { ladder: usize, y0: f32, loops0: f32 },
    Grapple { victim: usize, grab: Grab, hit: bool, timer: f32 },
    Held { by: usize },
    Arrest { victim: usize },
    Cuffed,
    /// Drawn up into the air by Bane (`STATE_FLOAT`): a quarter of a unit a tick until the
    /// body is 200 above the ground at `floor`, and there it hangs. Nothing in the original
    /// ever lets them down again (`set_person_float_down` has no callers).
    Float { floor: f32 },
}

/// What a person did this frame that affects somebody else.
#[derive(Debug, Clone)]
pub enum Event {
    Hit { attacker: usize, victim: usize, damage: i32, ko: u8, high: bool, low: bool, weapon: Weapon },
    /// A shot at `victim` (or at nothing); whether it hits is decided by the game.
    Shot { attacker: usize, victim: Option<usize>, weapon: Weapon },
    Grab { attacker: usize, victim: usize },
    /// The attacker changed grapple move; the victim follows.
    GrabMove { victim: usize, grab: Grab },
    Release { victim: usize },
    Arrest { attacker: usize, victim: usize },
    Arrested { victim: usize },
    Punch { attacker: usize },
    Kick { attacker: usize },
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
    pub attacking: bool,
    /// Guarding (an attacker can see it).
    pub blocking: bool,
    /// In a car: out of reach of fists and bullets.
    pub in_car: bool,
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
    /// Guns carried (bit per `Weapon::GUNS` slot), and rounds for each.
    pub guns: u8,
    pub ammo: [u32; 3],
    /// Out for good (health gone): won't get up.
    pub knocked_out: bool,
    /// A mission has set them peeing (`FLAG_PERSON_PEEING`).
    pub peeing: bool,
    /// Charges carried (the special's `ammo`): four at most.
    pub explosives: u8,
    down_timer: f32,
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
    pub pace: Pace,
    /// Getting out of a car's way: run this way for this many seconds.
    pub dodge: Option<(Vec2, f32)>,
    /// `PCOM_MOVE_WANDER`: walking to this spot, giving up after this many seconds (or
    /// pausing, with no spot).
    wander: (Option<Vec2>, f32),
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
            arrested: false, removed: false, skill: 0, mv: 1, weapon: Weapon::None, melee: Weapon::None, guns: 0, ammo: [0; 3], knocked_out: false, down_timer: 0.0,
            block_timer: 0.0, zip_cooldown: 0.0, dodge: None, wander: (None, 0.0), flash: 0.0, other: 0, follow: 0,
            invulnerable: false, foe: None, colour: 0, group: 0, goal: None, aggro: false, attack_timer: 1.0, cues: Vec::new(), step_phase: 0.0,
            peeing: false, explosives: 0, protected: false, guilty: false,
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
        melee.into_iter().chain(guns).chain(charges).filter(|&(w, _)| w != self.weapon).collect()
    }

    /// Puts away what's in hand and takes out the next weapon carried (bare hands last).
    pub fn next_weapon(&mut self) {
        let carried: Vec<Weapon> = [Weapon::None, self.melee, Weapon::Pistol, Weapon::Shotgun, Weapon::Ak47, Weapon::Explosives]
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

    pub fn is_dummy(&self) -> bool {
        self.ai == crate::script::PCOM_AI_FIGHT_TEST
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

    pub fn snap(&self) -> Snap {
        Snap {
            // Where the body is: a knockdown carries it away from the figure's origin (standing,
            // the origin is steadier: hit reactions only lean the body).
            pos: if self.is_down() { self.body_pos() } else { self.fig.pos },
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
            attacking: matches!(self.state, State::Attack { .. }),
            blocking: matches!(self.state, State::Block),
            in_car: matches!(self.state, State::InCar { .. }),
        }
    }

    /// Whose blows can land on whom: the player can hit anybody, the AI only its foe.
    fn opponents(&self, j: usize, other: &Snap) -> bool {
        if self.player { !other.player } else { self.fights() && self.foe == Some(j) }
    }

    /// The nearest standing opponent within `range`.
    fn nearest_opponent(&self, me: usize, snaps: &[Snap], range: f32) -> Option<usize> {
        snaps
            .iter()
            .enumerate()
            .filter(|&(j, s)| j != me && s.standing && self.opponents(j, s) && (!self.player || s.fighter)
                && (s.pos.y - self.fig.pos.y).abs() < 128.0)
            .map(|(j, s)| (j, flat(s.pos, self.fig.pos).length()))
            .filter(|&(_, d)| d < range)
            .min_by(|a, b| a.1.total_cmp(&b.1))
            .map(|(j, _)| j)
    }

    /// AI: decide this frame's controls, against its `foe` (see `Person::foe`).
    pub fn think(&mut self, me: usize, snaps: &[Snap], ground: &Ground, dt: f32,
                 rand: &mut impl FnMut() -> u32) -> Controls {
        let player = self.foe;
        let mut c = Controls { target: player, ..Default::default() };
        // A car coming: run out of its way (`PCOM_find_runover_thing`).
        if let Some((dir, left)) = self.dodge {
            self.dodge = (left > dt).then_some((dir, left - dt));
            c.wish = dir;
            c.run = true;
            return c;
        }
        // Nobody to fight: walk the patrol, follow whoever is being followed, or stroll.
        // Dropping `aggro` matters as much as the walking does: the game only gives someone
        // somewhere to go while they aren't in a fight, so a bodyguard whose man went down
        // would otherwise stand there for good.
        let Some(p) = player.map(|p| snaps[p]) else {
            self.aggro = false;
            return self.wander(c, ground, dt, rand);
        };
        let d = flat(p.pos, self.fig.pos);
        let dist = d.length();
        let dy = (p.pos.y - self.fig.pos.y).abs();
        // PCOM_AI_FLEE_PLAYER keeps its distance (until hit: then it fights, see take_hit).
        if self.ai == 16 && p.present && !p.down {
            if dist < 6.0 * 256.0 {
                c.wish = -d.normalize_or_zero();
                c.run = true;
            }
            return c;
        }
        if !self.fights() || !p.present || p.down || p.in_car {
            self.aggro = false;
            return self.wander(c, ground, dt, rand);
        }
        let range = self.aggro_range();
        let hunter = range > 32.0 * 256.0;
        if !self.aggro && dist < range && (dy < 128.0 || hunter) {
            self.aggro = true;
            self.cues.push(Sfx::Alert { kind: self.kind });
            self.attack_timer = 0.8;
        } else if self.aggro && !hunter && (dist > range * 2.0 + AGGRO_RANGE || dy > 256.0) {
            self.aggro = false;
        }
        if !self.aggro {
            return self.wander(c, ground, dt, rand);
        }
        // Block a blow that's coming (better fighters more often).
        self.block_timer -= dt;
        if self.block_timer > 0.0 {
            c.block = true;
            return c;
        }
        let swinging = snaps.iter().any(|s| s.player && s.attacking) && dist < 220.0;
        if swinging && self.block_timer <= -0.4 && rand() % 100 < self.skill as u32 * 5 {
            self.block_timer = 0.6;
            c.block = true;
            return c;
        }
        // Guns shoot from range when they can see the player, and close in otherwise.
        let skill = self.skill as f32;
        if self.weapon.is_gun() && dist > 260.0 {
            self.attack_timer -= dt;
            let eye = vec3(0.0, 140.0, 0.0);
            let seen = dist < SHOOT_RANGE && ground.line_of_sight(self.fig.pos + eye, p.pos + eye);
            if seen && self.attack_timer <= 0.0 {
                c.punch = true;
                self.attack_timer = (2.4 - skill * 0.1).max(0.8) + (rand() % 100) as f32 / 100.0;
            } else if !seen || dist > SHOOT_RANGE * 0.8 {
                c.wish = d / dist;
                c.run = dist > SHOOT_RANGE;
            }
            return c;
        }
        // Keep out of each other's way: hold back while another opponent is closer to the player.
        let crowded = snaps.iter().enumerate().any(|(j, s)| {
            j != me && s.standing && s.hostile && flat(p.pos, s.pos).length() + 20.0 < dist
                && flat(s.pos, self.fig.pos).normalize_or_zero().dot(d.normalize_or_zero()) > 0.8
        });
        if dist > 140.0 && !crowded {
            c.wish = d / dist;
        } else if dist < 80.0 {
            c.wish = -d / dist.max(1.0) * 0.5;
        }
        c.run = hunter && dist > 600.0;
        self.attack_timer -= dt;
        // Take turns, like the original's gang attacks: nobody swings while another does.
        let busy = snaps.iter().enumerate().any(|(j, s)| j != me && s.hostile && s.attacking);
        if dist < 170.0 && self.attack_timer <= 0.0 && p.standing && !busy {
            // Armed people use their weapon; others punch or kick.
            if self.weapon != Weapon::None || rand().is_multiple_of(2) { c.punch = true } else { c.kick = true }
            self.attack_timer = (1.8 - skill * 0.07).max(0.6) + (rand() % 100) as f32 / 70.0;
        }
        c
    }

    /// `PCOM_MOVE_WANDER` (move 4): strolls from spot to spot nearby, pausing in between.
    /// Spots are in plain sight on the same level; walking never steps off a ledge.
    fn wander(&mut self, mut c: Controls, ground: &Ground, dt: f32, rand: &mut impl FnMut() -> u32) -> Controls {
        // Off to the goal the game set: a patrol's next point, or whoever is being followed.
        if let (Some(goal), false) = (self.goal, self.is_down()) {
            let d = flat(goal, self.fig.pos);
            if d.length() > 40.0 {
                c.wish = d.normalize();
                (c.run, c.sprint) = self.pace.controls();
            }
            return c;
        }
        if self.mv != 4 || self.is_down() {
            return c;
        }
        let here = vec2(self.fig.pos.x, self.fig.pos.z);
        let (spot, left) = self.wander;
        let left = left - dt;
        match spot {
            Some(s) if left > 0.0 && (s - here).length() > 40.0 => {
                c.wish = (s - here).normalize_or_zero();
                self.wander = (Some(s), left);
            }
            // There (or stuck): pause for a few seconds.
            Some(_) => self.wander = (None, 1.0 + (rand() % 400) as f32 / 100.0),
            None if left > 0.0 => self.wander = (None, left),
            None => {
                let angle = (rand() % 628) as f32 / 100.0;
                let dist = 2.0 * 256.0 + (rand() % 1024) as f32;
                let s = here + vec2(angle.sin(), angle.cos()) * dist;
                let waist = vec3(0.0, 80.0, 0.0);
                if ground.line_of_sight(self.fig.pos + waist, vec3(s.x, self.fig.pos.y, s.y) + waist) {
                    // Long enough to get there at a walk, with some to spare.
                    self.wander = (Some(s), dist / WALK_SPEED * 1.5 + 1.0);
                }
            }
        }
        c
    }

    fn idle(&mut self, fight: bool) {
        let a = match (fight, self.weapon) {
            (_, w) if w.is_gun() && self.gun_out() => w.aim_anim(),
            (true, Weapon::Knife) => anim::KNIFE_FIGHT_READY,
            (true, Weapon::Bat) => anim::BAT_IDLE,
            (true, _) => anim::FIGHT,
            (false, _) => anim::BREATHE,
        };
        self.fig.play(a, Play::Loop);
        self.fig.set_speed(1.0);
    }

    fn start_attack(&mut self, me: usize, mv: Move, events: &mut Vec<Event>) {
        self.fig.restart(mv.def().anim, Play::Once);
        self.state = State::Attack { mv, hit: false, queued: None };
        self.cues.push(Sfx::Swing);
        events.push(if mv.is_punch() { Event::Punch { attacker: me } } else { Event::Kick { attacker: me } });
    }

    /// Moves horizontally by `delta`, sliding along walls and following the floor. Returns
    /// false if it stepped off an edge (the caller falls). Without `drop_ok`, edges stop the
    /// person instead: the AI, and anyone fighting, never walks off a roof.
    fn walk(&mut self, delta: Vec2, ground: &Ground, map: &Map, drop_ok: bool) -> bool {
        let p = &mut self.fig.pos;
        let to = ground.slide(vec2(p.x, p.z), p.y, delta, RADIUS);
        let floor = ground.surface(map, to.x, to.y, p.y);
        if floor - p.y > STEP_HEIGHT || (!drop_ok && p.y - floor > STEP_HEIGHT) {
            return true; // a wall without collision data, or an edge: just stop
        }
        *p = vec3(to.x, p.y, to.y);
        if p.y - floor > STEP_HEIGHT {
            return false;
        }
        p.y = floor;
        true
    }

    /// Where a person would stand after climbing a ledge in direction `dir`, if there is one
    /// between a step and `reach` above the feet (a raised walkway, a platform rim, a roof).
    fn ledge_ahead(&self, dir: Vec2, reach: f32, ground: &Ground, map: &Map) -> Option<Vec3> {
        let p = self.fig.pos;
        let probe = vec2(p.x, p.z) + dir * (RADIUS + 40.0);
        let top = ground.surface(map, probe.x, probe.y, p.y + reach - STEP_HEIGHT);
        if top - p.y <= STEP_HEIGHT || top - p.y > reach {
            return None;
        }
        // Room to stand: no wall between the edge and the spot, at the new height.
        let spot = ground.slide(probe, top, dir * 30.0, RADIUS);
        Some(vec3(spot.x, ground.surface(map, spot.x, spot.y, top), spot.y))
    }

    /// Plays a climbing animation `a` that ends standing on `to`. The animation carries the
    /// body up by itself (a pull-up raises the hips 263 units), so the figure's position is
    /// moved so that the hips start where they are and end at standing height on `to`.
    /// `from_standing`: turn to face `to` first (else keep facing, e.g. on a ladder).
    fn climb_onto(&mut self, a: usize, to: Vec3, from_standing: bool) {
        let f = &mut self.fig;
        if from_standing {
            f.angle = uc_world::character::angle_of(vec2(to.x - f.pos.x, to.z - f.pos.z));
        }
        let stand = f.stand_root();
        let (Some(first), Some(last)) = (f.anim_root(a, false), f.anim_root(a, true)) else { return };
        // Switch poses at once (blending in from the old pose would leave the body behind),
        // keeping the hips where they are.
        // From where the hips are drawn now (a jump's height correction included).
        let start = f.drawn_pos() + f.to_world(f.root() - first);
        f.cancel_rise = false;
        f.lift = 0.0;
        let end = to - f.to_world(last - stand);
        f.restart_now(a, Play::Once);
        f.set_speed(1.3);
        f.pos = start;
        self.state = State::PullUp { start, end, to };
    }

    /// Starts climbing over a fence whose near face is at `at` (xz), top at `top`.
    fn climb_fence(&mut self, at: Vec2, over: Vec2, top: f32) {
        let from = self.fig.hips();
        self.fig.angle = uc_world::character::angle_of(over);
        self.fig.pin_root = true;
        self.fig.restart_now(anim::LAND_ON_FENCE, Play::Once);
        self.fig.set_speed(1.5);
        self.state = State::Fence { phase: 0, at, over, base: self.fig.pos.y, top, from };
        self.place_hips(from);
    }

    /// With the root pinned, puts the figure so its hips are drawn at `h`.
    fn place_hips(&mut self, h: Vec3) {
        let stand = self.fig.stand_root();
        self.fig.pos = h - self.fig.to_world(stand);
    }

    fn pull_up(&mut self, to: Vec3) {
        self.climb_onto(anim::PULL_UP, to, true);
    }

    fn jump(&mut self, vel: Vec2) {
        self.cues.push(Sfx::Jump { kind: self.kind });
        self.state = State::Fall { vy: JUMP_SPEED, vel };
        self.fig.restart(if vel.length() > 1.0 { anim::RUN_JUMP } else { anim::STANDING_JUMP }, Play::Once);
        self.fig.set_speed(1.5);
    }

    fn fall(&mut self, vel: Vec2) {
        self.state = State::Fall { vy: 0.0, vel };
        self.fig.play(anim::FALLING, Play::Loop);
    }

    /// Advances the animation, then places anything that follows it: a climber's height on a
    /// ladder goes up a rung cycle as the climbing animation loops back down.
    pub fn advance(&mut self, dt: f32, ground: &Ground) {
        self.fig.advance(dt);
        self.flash = (self.flash - dt).max(0.0);
        // Footsteps: two per walk or run cycle.
        let walking = matches!(self.state, State::Move | State::Land { .. })
            && matches!(self.fig.anim(), anim::WALK | anim::RUN | anim::PISTOL_JOG | anim::AK_JOG | anim::FIGHT_STEP_N | anim::FIGHT_STEP_S | anim::FIGHT_STEP_E | anim::FIGHT_STEP_W);
        let phase = self.fig.loops().floor() + self.fig.progress();
        if walking && (phase * 2.0).floor() != (self.step_phase * 2.0).floor() {
            self.cues.push(Sfx::Footstep { player: self.player });
        }
        self.step_phase = phase;
        if let State::Climb { ladder, y0, loops0 } = self.state {
            self.fig.pos = ground.on_ladder(ladder, y0 + (self.fig.loops() - loops0) * RUNG_CYCLE);
        }
    }

    /// One frame. `snaps` is everybody's state at the start of the frame.
    #[allow(clippy::too_many_arguments)]
    pub fn step(&mut self, me: usize, c: &Controls, snaps: &[Snap], ground: &Ground, map: &Map, dt: f32, events: &mut Vec<Event>) {
        if self.removed {
            return;
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
            State::GetUp => {
                if self.fig.finished() {
                    self.state = State::Move;
                    self.idle(self.nearest_opponent(me, snaps, FIGHT_RANGE).is_some());
                }
            }
            State::Shoot { target, mut shots } => {
                let target = target.filter(|&t| snaps[t].present && !snaps[t].down);
                if let Some(t) = target {
                    self.fig.turn_towards(flat(snaps[t].pos, self.fig.pos), 12.0, dt);
                } else if let Some(at) = c.aim {
                    self.fig.turn_towards(flat(at, self.fig.pos), 12.0, dt);
                }
                let w = self.weapon;
                if w == Weapon::Ak47 {
                    // Automatic: a round each time round the firing loop, while the trigger is
                    // held (the AI fires bursts of three).
                    let more = if self.player { c.fire } else { shots < 3 };
                    if self.fig.loops() >= shots as f32 {
                        if more && self.fire_round(me, target, events) {
                            shots += 1;
                        } else {
                            self.state = State::Move;
                            self.idle(false);
                            return;
                        }
                    }
                } else if shots == 0 && self.fig.progress() > 0.25 {
                    shots = 1;
                    self.fire_round(me, target, events);
                } else if shots > 0 && (self.fig.finished() || (c.punch && self.fig.progress() > 0.6)) {
                    // Done (or fire again straight away).
                    self.state = State::Move;
                    self.idle(false);
                    if c.punch {
                        self.step_move(me, c, snaps, ground, map, dt, events);
                    }
                    return;
                }
                self.state = State::Shoot { target, shots };
            }
            State::Fence { mut phase, at, over, base, top, from } => {
                // Hips just in front of the fence, hands up on it.
                let near = vec3(at.x, 0.0, at.y) - vec3(over.x, 0.0, over.y) * 28.0;
                let grip = |y: f32| vec3(near.x, y, near.z);
                let (low, high) = (base + 150.0, (top - 30.0).max(base + 150.0));
                match phase {
                    0 => {
                        let t = self.fig.progress();
                        self.place_hips(from.lerp(grip(low), t));
                        if self.fig.finished() {
                            phase = 1;
                            self.fig.restart(anim::CLIMB_UP_FENCE, Play::Loop);
                        }
                    }
                    1 => {
                        let y = (self.fig.hips().y + 220.0 * dt).min(high);
                        self.place_hips(grip(y));
                        if y >= high {
                            phase = 2;
                            self.fig.restart(anim::CLIMB_OVER_FENCE, Play::Once);
                        }
                    }
                    _ => {
                        // Over the top in an arc, ending just past the fence.
                        let t = self.fig.progress();
                        let far = vec3(at.x, 0.0, at.y) + vec3(over.x, 0.0, over.y) * 40.0;
                        let xz = near.lerp(far, t);
                        let y = high + (top + 60.0 - high) * (t * std::f32::consts::PI).sin() - (high - top + 60.0) * t * t;
                        self.place_hips(vec3(xz.x, y, xz.z));
                        if self.fig.finished() {
                            // Drop down on the far side.
                            let h = self.fig.hips();
                            self.fig.pin_root = false;
                            self.fig.pos = h - vec3(0.0, self.fig.stand_root().y, 0.0);
                            self.fall(over * 60.0);
                            return;
                        }
                    }
                }
                self.state = State::Fence { phase, at, over, base, top, from };
            }
            State::Zip { cable, mut t, dir, mut speed, mut ridden } => {
                let cb = &ground.cables[cable];
                let len = cb.length().max(1.0);
                // Speed up downhill to a top speed.
                let ahead = cb.at(t + dir * 0.01);
                let slope = (cb.at(t).y - ahead.y) / (ahead - cb.at(t)).length().max(1.0);
                speed = (speed + GRAVITY * 0.6 * slope.max(0.1) * dt).min(1100.0);
                t += dir * speed * dt / len;
                ridden += speed * dt;
                let tangent = (cb.at(t + dir * 0.01) - cb.at(t)).normalize_or_zero();
                self.fig.angle = uc_world::character::angle_of(vec2(tangent.x, tangent.z));
                let end = !(0.0..=1.0).contains(&t);
                self.fig.pos = cb.at(t) - vec3(0.0, HANG, 0.0);
                // Near the bottom, sliding into a roof (its top between the feet and the waist):
                // step off onto it. The Jump's zip line ends just above a roof that's above
                // Darci's feet. (Earlier on, the legs pass over whatever's below.)
                let p = self.fig.pos;
                let left = if dir > 0.0 { 1.0 - t } else { t } * len;
                let roof = ground.surface(map, p.x, p.z, p.y + HANG * 0.5);
                if roof > p.y - 8.0 && ridden > 300.0 && left < 600.0 {
                    self.fig.pos.y = roof;
                    self.zip_cooldown = 1.0;
                    self.state = State::Land { vel: vec2(tangent.x, tangent.z) * speed * 0.3 };
                    self.cues.push(Sfx::Land);
                    self.fig.restart_now(anim::LAND_VERT, Play::Once);
                    self.fig.set_speed(2.0);
                    return;
                }
                if end || c.jump || c.action {
                    // Let go: drop from where the hanging body is, and don't grab straight back.
                    let hips = self.fig.hips();
                    self.zip_cooldown = 1.0;
                    self.fall(vec2(tangent.x, tangent.z) * speed * 0.6);
                    self.fig.restart_now(anim::FALLING, Play::Loop);
                    // Falling draws the hips at standing height above the feet.
                    self.fig.pos.y = hips.y - self.fig.stand_root().y;
                    return;
                }
                self.state = State::Zip { cable, t, dir, speed, ridden };
            }
            State::Attack { mv, mut hit, mut queued } => {
                let def = mv.def();
                let p = self.fig.progress();
                if !hit && p >= def.hit_at {
                    hit = true;
                    let dir = match def.dir {
                        Dir::Front => self.fig.facing(),
                        Dir::Back => -self.fig.facing(),
                        Dir::Left => { let f = self.fig.facing(); vec2(f.y, -f.x) }
                        Dir::Right => { let f = self.fig.facing(); vec2(-f.y, f.x) }
                    };
                    for (j, s) in snaps.iter().enumerate() {
                        if j == me || !s.standing || !self.opponents(j, s) || (s.pos.y - self.fig.pos.y).abs() > 100.0 {
                            continue;
                        }
                        let d = flat(s.pos, self.fig.pos);
                        if d.length() < def.reach && d.normalize_or_zero().dot(dir) > 0.5 {
                            // The AI hits less hard, more so the less skilled.
                            let damage = if self.player { def.damage } else { def.damage * (8 + self.skill as i32) / 30 };
                            let weapon = if matches!(mv, Move::Knife1 | Move::Knife2 | Move::Knife3 | Move::Bat1 | Move::Bat2) { self.weapon } else { Weapon::None };
                            events.push(Event::Hit { attacker: me, victim: j, damage, ko: def.ko, high: def.high, low: false, weapon });
                        }
                    }
                }
                // Combos: the same button again in the second half of the move continues the
                // combo; the other button (or the same one after a finisher) starts afresh.
                if queued.is_none() && p > 0.4 && (c.punch || c.kick) {
                    let same = if mv.is_punch() { c.punch } else { c.kick };
                    queued = match (same, def.next) {
                        (true, Some(next)) => Some(next),
                        _ if c.punch => Some(Move::punch_with(self.weapon)),
                        _ => Some(Move::Kick1),
                    };
                }
                self.state = State::Attack { mv, hit, queued };
                if self.fig.finished() {
                    match queued {
                        // A punch while walking at someone close is a grab, not another punch.
                        Some(Move::Punch1) if c.wish.length_squared() > 0.25 => {
                            self.state = State::Move;
                            let c = Controls { punch: true, kick: false, ..*c };
                            self.step_move(me, &c, snaps, ground, map, dt, events);
                        }
                        Some(next) => self.start_attack(me, next, events),
                        None => {
                            self.state = State::Move;
                            self.idle(self.nearest_opponent(me, snaps, FIGHT_RANGE).is_some());
                        }
                    }
                }
            }
            State::Slide { mut phase, mut speed, mut hit } => {
                let f = self.fig.facing();
                if phase < 2 {
                    for (j, s) in snaps.iter().enumerate() {
                        if j == me || j >= 64 || hit & (1 << j) != 0 || !s.standing || !self.opponents(j, s) {
                            continue;
                        }
                        let d = flat(s.pos, self.fig.pos);
                        if d.length() < 110.0 && d.normalize_or_zero().dot(f) > 0.3 {
                            hit |= 1 << j;
                            events.push(Event::Hit { attacker: me, victim: j, damage: 30, ko: ko::SLIDE, high: false, low: true, weapon: Weapon::None });
                        }
                    }
                }
                speed = (speed - 500.0 * dt).max(0.0);
                if !self.walk(f * speed * dt, ground, map, self.player) {
                    self.fall(f * speed);
                    return;
                }
                match phase {
                    0 if self.fig.finished() => {
                        phase = 1;
                        self.fig.play(anim::SLIDER_HOLD, Play::Loop);
                    }
                    1 if speed < 120.0 => {
                        phase = 2;
                        self.fig.restart(anim::SLIDER_END, Play::Once);
                    }
                    2 if self.fig.finished() => {
                        self.state = State::Move;
                        self.idle(false);
                        return;
                    }
                    _ => {}
                }
                self.state = State::Slide { phase, speed, hit };
            }
            State::Land { mut vel } => {
                // Keep running out of a landing; otherwise skid to a stop.
                let held = c.wish.length_squared() > 0.25;
                vel *= if held { 1.0 } else { (-6.0 * dt).exp() };
                if held && vel.length() > 150.0 {
                    self.fig.turn_towards(c.wish, 6.0, dt);
                    vel = c.wish.normalize() * vel.length();
                }
                if !self.walk(vel * dt, ground, map, self.player) {
                    self.fall(vel);
                    return;
                }
                self.state = State::Land { vel };
                if self.fig.finished() || (held && self.fig.progress() > 0.5) {
                    self.state = State::Move;
                    self.step_move(me, c, snaps, ground, map, dt, events);
                }
            }
            State::Hurt => {
                // The player can cut a hit reaction short by fighting back or jumping away.
                if self.player && self.fig.progress() > 0.35 && (c.punch || c.kick || c.jump) {
                    self.state = State::Move;
                    self.step_move(me, c, snaps, ground, map, dt, events);
                    return;
                }
                if self.fig.finished() {
                    self.state = State::Move;
                    self.idle(self.nearest_opponent(me, snaps, FIGHT_RANGE).is_some());
                }
            }
            State::Down => {
                // Knocked down, not out: get up after a while (unless arrested meanwhile).
                self.down_timer += dt;
                let lying = matches!(self.fig.anim(), anim::KD_FRONT_LAND | anim::KD_BACK_LAND);
                if !self.knocked_out && lying && self.fig.finished() && self.down_timer > 2.5 {
                    self.fig.restart_locked(anim::QUICK_GET_UP, Play::Once);
                    self.fig.lift = 0.0;
                    self.state = State::GetUp;
                    return;
                }
                // A knockdown is two animations: the fall, then hitting the floor (on the back
                // if knocked over from the front). Chain them keeping the hips in place.
                let land = match self.fig.anim() {
                    anim::KD_FRONT_MID | anim::KD_FRONT_LOW | anim::HIT_FRONT_HI => Some(anim::KD_BACK_LAND),
                    anim::KD_BACK_MID | anim::KD_BACK_LOW => Some(anim::KD_FRONT_LAND),
                    _ => None,
                };
                if let (Some(a), true) = (land, self.fig.finished()) {
                    self.fig.restart_locked(a, Play::Once);
                    let p = self.fig.pos;
                    self.fig.pos.y = ground.surface(map, p.x, p.z, p.y);
                }
            }
            State::Cuffed | State::Held { .. } | State::InCar { .. } => {}
            State::Fall { mut vy, mut vel } => {
                vy -= GRAVITY * dt;
                let running = vel.length() > 100.0;
                if self.fig.finished() {
                    self.fig.play(if running { anim::MID_AIR } else { anim::FALLING }, Play::Loop);
                }
                // A little air control.
                if self.player && c.wish.length_squared() > 0.25 {
                    let want = c.wish.normalize() * vel.length().max(WALK_SPEED);
                    vel = vel.lerp(want, (2.0 * dt).min(1.0));
                }
                // Grab a cable overhead.
                self.zip_cooldown -= dt;
                let grab = if self.zip_cooldown <= 0.0 { ground.cable_near(self.fig.pos + vec3(0.0, HANG - 60.0, 0.0), 110.0) } else { None };
                // Slide towards the lower end; anywhere but right at that end will do (The
                // Jump's zip line can only be reached at its very top).
                let lower = |cable: usize| if ground.cables[cable].at(1.0).y < ground.cables[cable].at(0.0).y { 1.0 } else { -1.0 };
                let room = |cable: usize, t: f32| if lower(cable) > 0.0 { 1.0 - t } else { t };
                if let Some((cable, t)) = grab.filter(|&(cable, t)| room(cable, t) > 0.03 && t > 0.0 && t < 1.0) {
                    let dir = lower(cable);
                    self.fig.restart(anim::DEATH_SLIDE, Play::Once);
                    self.cues.push(Sfx::Zip);
                    self.state = State::Zip { cable, t, dir, speed: vel.length() * 0.5, ridden: 0.0 };
                    return;
                }
                // Grab a ledge in front while in the air.
                let dir = if c.wish.length_squared() > 0.25 { c.wish.normalize() } else { vel.normalize_or_zero() };
                if dir != Vec2::ZERO {
                    // Only ledges too high to jump onto are grabbed; lower ones are landed on.
                    let p = self.fig.pos;
                    let floor = ground.surface(map, p.x, p.z, p.y);
                    if let Some(to) = self.ledge_ahead(dir, GRAB_REACH, ground, map).filter(|to| to.y - floor > JUMPABLE) {
                        self.pull_up(to);
                        return;
                    }
                }
                let p = self.fig.pos;
                let to = ground.slide(vec2(p.x, p.z), p.y, vel * dt, RADIUS);
                let y = p.y + vy * dt;
                let floor = ground.surface(map, to.x, to.y, p.y);
                if y <= floor {
                    self.fig.pos = vec3(to.x, floor, to.y);
                    self.fig.lift = (floor - p.y).max(0.0);
                    self.state = State::Land { vel };
                    self.cues.push(Sfx::Land);
                    self.fig.restart(if running { anim::LAND_RIGHT } else { anim::LAND_VERT }, Play::Once);
                    self.fig.set_speed(if running { 1.5 } else { 2.0 });
                    // Landing hard hurts, and from high enough kills (fallen/Source/Darci.cpp).
                    let damage = fall_damage(-vy);
                    if damage > 0 {
                        self.fig.lift = 0.0;
                        let ko = if damage >= self.health { ko::KNOCKDOWN } else { 0 };
                        self.take_hit(self.fig.pos + vec3(0.0, 0.0, 1.0), damage, ko, false, true, Weapon::None);
                    }
                } else {
                    self.fig.pos = vec3(to.x, y, to.y);
                    self.state = State::Fall { vy, vel };
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
            State::Climb { ladder, y0, loops0 } => {
                let l = ground.ladders[ladder];
                self.fig.angle = uc_world::character::angle_of(-l.out);
                if c.wish.dot(l.out) > 0.7 || c.action && self.fig.pos.y > l.base + 64.0 {
                    self.fall(l.out * 60.0);
                    return;
                }
                // The animation moves the body up a rung cycle and loops back, so the feet
                // move up a cycle each time it loops.
                let moving = c.wish.dot(-l.out) > 0.3;
                self.fig.set_speed(if moving { 1.3 } else { 0.0 });
                let y = y0 + (self.fig.loops() - loops0) * RUNG_CYCLE;
                self.fig.pos = ground.on_ladder(ladder, y);
                // Near the top, at the point of the cycle where the hands reach the edge,
                // climb off onto the roof.
                let off_rise = self.fig.anim_root(anim::OFF_LADDER_TOP, true).map_or(261.0, |r| r.y - self.fig.stand_root().y);
                if y + off_rise >= l.top - 24.0 && self.fig.root().y >= 220.0 {
                    let spot = ground.on_ladder(ladder, l.top) - vec3(l.out.x, 0.0, l.out.y) * 90.0;
                    let to = vec3(spot.x, ground.surface(map, spot.x, spot.z, l.top), spot.z);
                    self.climb_onto(anim::OFF_LADDER_TOP, to, false);
                }
            }
            State::Grapple { victim, grab, mut hit, mut timer } => {
                let v = snaps[victim];
                if !v.present {
                    self.state = State::Move;
                    return;
                }
                timer += dt;
                let next = match grab {
                    Grab::Grab if self.fig.finished() => Some(Grab::Hold),
                    Grab::Hold if c.punch => Some(Grab::Knee1),
                    Grab::Hold if c.kick => Some(Grab::Throw),
                    Grab::Knee1 if c.punch && self.fig.progress() > 0.4 => Some(Grab::Knee2),
                    Grab::Knee1 if self.fig.finished() => Some(Grab::Hold),
                    _ => None,
                };
                if grab == Grab::Hold && timer > 5.0 {
                    // The victim wriggles free.
                    events.push(Event::Release { victim });
                    self.state = State::Move;
                    self.idle(true);
                    return;
                }
                if let Some(g) = next {
                    if g != Grab::Hold {
                        self.fig.restart(g.anims().0, Play::Once);
                        events.push(Event::GrabMove { victim, grab: g });
                        events.push(if g == Grab::Throw { Event::Kick { attacker: me } } else { Event::Punch { attacker: me } });
                    }
                    self.state = State::Grapple { victim, grab: g, hit: false, timer: 0.0 };
                    return;
                }
                if matches!(grab, Grab::Knee2 | Grab::Throw) {
                    if !hit && self.fig.progress() > 0.55 {
                        hit = true;
                        events.push(Event::Hit { attacker: me, victim, damage: 60, ko: ko::GRAPPLE, high: false, low: false, weapon: Weapon::None });
                    }
                    if self.fig.finished() {
                        self.state = State::Move;
                        self.idle(true);
                        return;
                    }
                }
                self.state = State::Grapple { victim, grab, hit, timer };
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

    #[allow(clippy::too_many_arguments)]
    fn step_move(&mut self, me: usize, c: &Controls, snaps: &[Snap], ground: &Ground, map: &Map, dt: f32, events: &mut Vec<Event>) {
        if c.switch && self.player {
            self.next_weapon();
            self.cues.push(Sfx::Pickup);
        }
        let moving = c.wish.length_squared() > 0.01;
        let target = self.nearest_opponent(me, snaps, FIGHT_RANGE);
        // A gun drawn: no fighting stance (punch fires).
        let fight = target.is_some() && !c.run && !self.weapon.is_gun();
        let f = self.fig.facing();
        // Wish direction relative to facing: x = right, y = forward.
        let rel = vec2(c.wish.dot(vec2(-f.y, f.x)), c.wish.dot(f));

        // Only Darci arrests people: the original asks for `PERSON_DARCI` before it even
        // looks for someone to cuff (`set_person_croutch`, fallen/Source/Person.cpp), so
        // Nitro Car's thug can't.
        if c.action && self.can_arrest() {
            // Arrest a downed opponent.
            // The best of those in reach: the nearest, but Roper and hostages only if there's
            // nobody else (`find_arrestee` takes 250 off their score of 500 less the distance).
            let downed = snaps.iter().enumerate()
                .filter(|&(j, s)| {
                    j != me && s.present && s.arrestable && !s.player
                        && flat(s.pos, self.fig.pos).length() < ARREST_RANGE && (s.pos.y - self.fig.pos.y).abs() < 64.0
                })
                .map(|(j, s)| {
                    use uc_world::character::person as ptype;
                    let last = matches!(s.kind, ptype::ROPER | ptype::HOSTAGE);
                    (j, s, 500.0 - flat(s.pos, self.fig.pos).length() - if last { 250.0 } else { 0.0 })
                })
                .max_by(|a, b| a.2.total_cmp(&b.2))
                .map(|(j, s, _)| (j, s));
            if let Some((j, s)) = downed {
                self.fig.angle = uc_world::character::angle_of(flat(s.pos, self.fig.pos));
                self.fig.restart(anim::ARREST_CROUTCH, Play::Once);
                self.state = State::Arrest { victim: j };
                self.cues.push(Sfx::Arrest);
                events.push(Event::Arrest { attacker: me, victim: j });
                return;
            }
        }
        // Climb a ladder by walking into it, or with action next to it.
        if let Some(l) = ground.ladder_near(self.fig.pos, 90.0) {
            let into = c.wish.dot(-ground.ladders[l].out) > 0.7;
            if into || c.action {
                self.fig.pos = ground.on_ladder(l, self.fig.pos.y);
                self.fig.restart(anim::ON_LADDER, Play::Loop);
                self.state = State::Climb { ladder: l, y0: self.fig.pos.y, loops0: self.fig.loops() };
                return;
            }
        }
        if c.block {
            self.fig.restart(anim::BLOCK_HIGH, Play::Once);
            self.state = State::Block;
            return;
        }
        if c.punch && self.weapon.is_gun() {
            if self.rounds() == Some(0) {
                self.cues.push(Sfx::Click);
                return;
            }
            let target = c.target.filter(|&t| t != me && snaps[t].present);
            if let Some(t) = target {
                self.face(snaps[t].pos);
            } else if let Some(at) = c.aim {
                self.face(at);
            } else if !self.player {
                return;
            }
            let looped = self.weapon == Weapon::Ak47;
            if !self.fig.restart(self.weapon.fire_anim(), if looped { Play::Loop } else { Play::Once }) {
                self.fig.restart(anim::PISTOL_SHOOT, if looped { Play::Loop } else { Play::Once });
            }
            self.state = State::Shoot { target, shots: 0 };
            return;
        }
        if c.punch {
            // Punch while moving towards a close opponent: grab his arm.
            if let Some(t) = target {
                let d = flat(snaps[t].pos, self.fig.pos);
                if self.player && d.length() < GRAB_RANGE && moving && c.wish.dot(d.normalize_or_zero()) > 0.6 {
                    self.fig.angle = uc_world::character::angle_of(d);
                    self.fig.restart(anim::GRAB_ARM, Play::Once);
                    self.state = State::Grapple { victim: t, grab: Grab::Grab, hit: false, timer: 0.0 };
                    self.cues.push(Sfx::Grab);
                    events.push(Event::Grab { attacker: me, victim: t });
                    events.push(Event::Punch { attacker: me });
                    return;
                }
            }
            if let Some(t) = target {
                self.face(snaps[t].pos);
            }
            self.start_attack(me, Move::punch_with(self.weapon), events);
            return;
        }
        if c.kick {
            if moving && c.run {
                let speed = if c.sprint { SPRINT_SPEED * 1.15 } else { RUN_SPEED * 1.1 };
                self.fig.restart(anim::SLIDER_START, Play::Once);
                self.state = State::Slide { phase: 0, speed, hit: 0 };
                events.push(Event::Kick { attacker: me });
                return;
            }
            let mv = if fight && moving && rel.y < -0.5 {
                Move::KickBack
            } else if fight && moving && rel.x.abs() > 0.5 {
                if rel.x > 0.0 { Move::KickRight } else { Move::KickLeft }
            } else {
                if let Some(t) = target {
                    self.face(snaps[t].pos);
                }
                Move::Kick1
            };
            self.start_attack(me, mv, events);
            return;
        }

        if fight {
            // Fight stance: face the opponent and step without turning.
            let t = snaps[target.unwrap()].pos;
            let d = flat(t, self.fig.pos);
            self.fig.turn_towards(d, 8.0, dt);
            if moving {
                let step = if rel.y.abs() >= rel.x.abs() {
                    if rel.y > 0.0 { anim::FIGHT_STEP_N } else { anim::FIGHT_STEP_S }
                } else if rel.x > 0.0 { anim::FIGHT_STEP_E } else { anim::FIGHT_STEP_W };
                self.fig.play(step, Play::Loop);
                if !self.walk(c.wish.normalize_or_zero() * FIGHT_STEP_SPEED * dt, ground, map, false) {
                    self.fall(c.wish * FIGHT_STEP_SPEED);
                }
            } else {
                self.idle(true);
            }
        } else if moving {
            let dir = c.wish.normalize();
            self.fig.turn_towards(dir, 10.0, dt);
            let (speed, a, rate) = match (c.run, c.sprint) {
                (true, true) => (SPRINT_SPEED, anim::RUN, 1.35),
                (true, false) => (RUN_SPEED, anim::RUN, 1.0),
                _ => (WALK_SPEED, anim::WALK, 1.0),
            };
            if c.jump {
                self.jump(dir * speed);
                return;
            }
            // Running with a gun: the jog that keeps it up.
            let jog = match self.weapon {
                Weapon::Pistol => Some(anim::PISTOL_JOG),
                Weapon::Shotgun | Weapon::Ak47 => Some(anim::AK_JOG),
                _ => None,
            };
            if !(c.run && !c.sprint && jog.is_some_and(|j| self.fig.play(j, Play::Loop))) {
                self.fig.play(a, Play::Loop);
            }
            self.fig.set_speed(rate);
            let before = self.fig.pos;
            // Patrols go where their points are, even off a roof (The Jump's jumper does).
            if !self.walk(dir * speed * dt, ground, map, self.player || self.goal.is_some()) {
                self.fall(dir * speed);
                return;
            }
            // Blocked while walking into a fence: climb over. (Ledges are jumped onto.)
            let moved = vec2(self.fig.pos.x - before.x, self.fig.pos.z - before.z).length();
            if moved < speed * dt * 0.5 && self.fig.facing().dot(dir) > 0.7 {
                if let Some((i, at, over)) = ground.fence_ahead(self.fig.pos, dir, RADIUS + 30.0) {
                    let f = ground.fences[i];
                    self.climb_fence(at, over, f.y1);
                }
            }
        } else if c.jump {
            self.jump(Vec2::ZERO);
        } else {
            self.idle(false);
            // A gun drawn: keep it on the target.
            if let (true, Some(t)) = (self.weapon.is_gun(), c.target) {
                self.fig.turn_towards(flat(snaps[t].pos, self.fig.pos), 8.0, dt);
            }
            // Stand on whatever is underneath (something may have moved us).
            let p = self.fig.pos;
            let floor = ground.surface(map, p.x, p.z, p.y);
            if p.y - floor > STEP_HEIGHT {
                self.fall(Vec2::ZERO);
            } else {
                self.fig.pos.y = floor;
            }
        }
    }

    /// Fires one round from the gun in hand; false when it's empty.
    fn fire_round(&mut self, me: usize, target: Option<usize>, events: &mut Vec<Event>) -> bool {
        let Some(s) = self.weapon.slot() else { return false };
        if self.player {
            if self.ammo[s] == 0 {
                self.cues.push(Sfx::Click);
                return false;
            }
            self.ammo[s] -= 1;
        }
        self.cues.push(Sfx::Shot(self.weapon));
        self.flash = 0.07;
        events.push(Event::Shot { attacker: me, victim: target, weapon: self.weapon });
        true
    }

    fn face(&mut self, at: Vec3) {
        let d = flat(at, self.fig.pos);
        if d.length_squared() > 1.0 {
            self.fig.angle = uc_world::character::angle_of(d);
        }
    }

    /// Takes a blow from someone at `from`. Fight-test dummies only go down to the moves in
    /// their `ko_moves`; everyone else loses health.
    #[allow(clippy::too_many_arguments)]
    pub fn take_hit(&mut self, from: Vec3, damage: i32, ko_flag: u8, high: bool, low: bool, weapon: Weapon) {
        if self.removed || self.invulnerable || self.is_down() || matches!(self.state, State::InCar { .. }) {
            return;
        }
        let front = flat(from, self.fig.pos).dot(self.fig.facing()) > 0.0;
        // A guard stops blows from the front; weapons still hurt a little.
        let blocking = matches!(self.state, State::Block) && !weapon.is_gun();
        if blocking && front && !low {
            self.cues.push(Sfx::Block);
            if weapon != Weapon::None {
                self.health -= damage / 4;
            }
            if self.health > 0 {
                return;
            }
        }
        let damage = if blocking { damage / 4 } else { damage };
        // Fight-test dummies only go down to their moves; everyone else loses health, and
        // finishing moves knock them down even with some left.
        let (knocked_out, knocked_down) = if self.is_dummy() {
            let ko = self.ko_moves & ko_flag != 0;
            (ko, ko)
        } else {
            self.health -= damage;
            (self.health <= 0, self.health <= 0 || ko_flag & ko::FINISHERS != 0)
        };
        if self.ai == 16 {
            self.ai = 6; // a cornered fleer fights back
        }
        self.aggro = true;
        self.attack_timer = self.attack_timer.max(0.8);
        // A bullet landing makes no thump of its own: all the original adds is a spark
        // (`PYRO_hitspang`), and it keeps the ricochets for shots that miss. The gun going
        // off and the victim's own voice carry it, so a shot mustn't sound like a punch.
        if !weapon.is_gun() {
            self.cues.push(match weapon {
                Weapon::Knife => Sfx::Stab,
                Weapon::Bat => Sfx::BatHit,
                _ => Sfx::Hit { heavy: knocked_down || damage >= 25 },
            });
        }
        self.cues.push(if knocked_out { Sfx::KnockedOut { kind: self.kind } } else { Sfx::Hurt { kind: self.kind } });
        if knocked_down {
            if knocked_out {
                self.health = 0;
                self.knocked_out = true;
            }
            self.down_timer = 0.0;
            self.state = State::Down;
            let a = match (front, low) {
                (true, true) => anim::KD_FRONT_LOW,
                (false, true) => anim::KD_BACK_LOW,
                (true, false) => anim::KD_FRONT_MID,
                (false, false) => anim::KD_BACK_MID,
            };
            self.fig.restart(a, Play::Once);
        } else {
            self.state = State::Hurt;
            let a = match (front, high) {
                (true, true) => anim::HIT_FRONT_HI,
                (true, false) => anim::HIT_FRONT_MID,
                (false, true) => anim::HIT_BACK_HI,
                (false, false) => anim::HIT_BACK_MID,
            };
            self.fig.restart(a, Play::Once);
            self.fig.set_speed(if self.player { 1.8 } else { 1.2 });
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
