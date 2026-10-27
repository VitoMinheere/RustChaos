//! Animated people: a set of body-part meshes posed by keyframe animation.
//!
//! Posing follows `FIGURE_draw` / `FIGURE_draw_prim_tween` (DDEngine/Source/figure.cpp):
//!   world = position + R(angle) * (tween(rotation_a, rotation_b) * point + lerp(offset_a, offset_b))
//! where R is `FIGURE_rotate_obj2(0, angle, 0)` and the rotation tween is a quaternion slerp.
//!
//! Every person type shares Darci's animations (`global_anim_array`, fallen/Source/Anim.cpp);
//! only the meshes differ. So a [`Figure`] pairs an animation set with a [`Model`].

use crate::geometry::{Batch, UNIT};
use glam::{vec2, vec3, Mat3, Quat, Vec2, Vec3};
use std::collections::BTreeMap;
use std::sync::Arc;
use uc_formats::all::{AnimChunk, Element};
use uc_formats::lgt::Lighter;

/// Animation ids from fallen/Headers/animate.h.
pub mod anim {
    pub const STAND_READY: usize = 3;
    pub const ON_LADDER: usize = 14;
    pub const LAND_RIGHT: usize = 53;
    pub const WALK: usize = 20;
    pub const FALLING: usize = 51;
    pub const CLIMB_UP_FENCE: usize = 45;
    pub const LAND_ON_FENCE: usize = 57;
    pub const CLIMB_OVER_FENCE: usize = 58;
    /// A flip to the side, out of a bullet's way (`ANIM_FLIP_LEFT`, `_RIGHT`).
    pub const FLIP_LEFT: usize = 91;
    pub const FLIP_RIGHT: usize = 92;
    /// Over a low fence at a run (`ANIM_VAULT`).
    pub const VAULT: usize = 16;
    pub const DEATH_SLIDE: usize = 68;
    pub const OFF_LADDER_TOP: usize = 55;
    pub const RUN: usize = 119;
    pub const MID_AIR: usize = 122;
    pub const BREATHE: usize = 124;
    pub const IDLE_SCRATCH1: usize = 77;
    pub const IDLE_SCRATCH2: usize = 78;
    pub const CROUTCH_DOWN: usize = 143;
    pub const CRAWL: usize = 144;
    pub const IDLE_CROUTCH: usize = 222;
    pub const CROUTCH_GETUP: usize = 278;
    pub const PISTOL_DUCK: usize = 309;
    pub const UNCROUTCH_PISTOL: usize = 311;
    pub const SHOTGUN_DUCK: usize = 312;
    pub const UNCROUTCH_AK: usize = 314;
    pub const SNIPER_CRAWL: usize = 315;
    pub const PUNCH_COMBO1: usize = 146;
    pub const PUNCH_COMBO2: usize = 147;
    pub const PUNCH_COMBO3: usize = 148;
    pub const KICK_COMBO1: usize = 154;
    pub const KICK_COMBO2: usize = 155;
    pub const KICK_COMBO3: usize = 156;
    pub const FIGHT: usize = 159;
    pub const KICK_LEFT: usize = 162;
    pub const KICK_BEHIND: usize = 164;
    /// Darci's kick between the legs, stepping in close (`set_person_kick_near`), and the
    /// victim doubling over, staying bent and straightening up (`fn_person_recoil`).
    pub const KICK_NAD: usize = 394;
    pub const KICK_NAD_TAKE: usize = 395;
    pub const KICK_NAD_STUNNED: usize = 396;
    pub const KICK_NAD_RECOVER: usize = 397;
    pub const BLOCK_HIGH: usize = 165;
    /// `ANIM_LEG_SWEEP`: the foot sweep from a block.
    pub const LEG_SWEEP: usize = 219;
    pub const HIT_FRONT_HI: usize = 166;
    pub const HIT_FRONT_MID: usize = 167;
    pub const HIT_BACK_HI: usize = 168;
    pub const HIT_BACK_MID: usize = 169;
    /// Sitting down on a bench, a sofa or an armchair, and staying there (`ANIM_SIT_DOWN`).
    pub const SIT_DOWN: usize = 337;
    /// Knocked dead by a blast, a car or fire, standing on the ground
    /// (`PERSON_DEATH_TYPE_OTHER`): from the front, onto the back; from behind, flat out.
    pub const KO_BACK: usize = 33;
    pub const KO_BEHIND_BIG: usize = 109;
    /// A fall that kills (`PERSON_DEATH_TYPE_LAND`, `ANIM_BIGLAND_DIE`).
    pub const BIGLAND_DIE: usize = 388;
    /// Falling to your death: the dive, the fall head first, and the landing face down.
    pub const PLUNGE_START: usize = 277;
    /// Pressing a switch, and turning a valve then letting go of it.
    pub const BUTTON: usize = 134;
    pub const VALVE_LOOP: usize = 398;
    pub const VALVE_END: usize = 399;
    pub const PLUNGE_FORWARDS: usize = 129;
    pub const PLUNGE_FRONT_SLAM: usize = 141;
    /// Shot dead (`ANIM_SHOT_DEAD_*`, fallen/Headers/animate.h:514): from behind, in the
    /// head, the chest or the gut; from the front, thrown onto the back.
    pub const SHOT_DEAD_HEAD: usize = 304;
    pub const SHOT_DEAD_CHEST: usize = 305;
    pub const SHOT_DEAD_GUT: usize = 306;
    pub const SHOT_DEAD_BACK: usize = 307;
    pub const KD_FRONT_MID: usize = 170;
    pub const KD_BACK_MID: usize = 172;
    pub const KD_BACK_LOW: usize = 174;
    pub const KD_FRONT_LOW: usize = 175;
    pub const KD_FRONT_LAND: usize = 176;
    pub const KD_BACK_LAND: usize = 177;
    pub const FIGHT_STEP_N: usize = 178;
    pub const FIGHT_STEP_S: usize = 179;
    pub const FIGHT_STEP_E: usize = 180;
    pub const FIGHT_STEP_W: usize = 181;
    pub const FIGHT_STOMP: usize = 184;
    pub const FIGHT_STOMPED_BACK: usize = 188;
    pub const FIGHT_STOMPED_FRONT: usize = 189;
    pub const STANDING_JUMP: usize = 203;
    pub const LAND_VERT: usize = 204;
    pub const BAT_HIT1: usize = 199;
    pub const BAT_HIT2: usize = 200;
    pub const BAT_IDLE: usize = 201;
    pub const RUN_JUMP: usize = 54;
    pub const PULL_UP: usize = 332;
    /// `ANIM_DANGLE`: hanging from a ledge by the hands.
    pub const DANGLE: usize = 223;
    /// `ANIM_TRAVERSE_LEFT` and `_RIGHT`: a hand along the ledge either way.
    pub const TRAVERSE_LEFT: usize = 130;
    pub const TRAVERSE_RIGHT: usize = 131;
    pub const SLIDER_START: usize = 216;
    pub const SLIDER_END: usize = 217;
    pub const SLIDER_HOLD: usize = 218;
    pub const GRAB_ARM: usize = 226;
    pub const GRAB_ARMV: usize = 227;
    pub const PISTOL_WHIP: usize = 391;
    pub const PISTOL_WHIP_TAKE: usize = 392;
    pub const GRAB_ARM_THROW: usize = 228;
    pub const GRAB_ARM_THROWV: usize = 229;
    pub const GRAB_ARM_KNEE1: usize = 230;
    pub const GRAB_ARM_KNEE1V: usize = 231;
    pub const GRAB_ARM_KNEE2: usize = 232;
    pub const GRAB_ARM_KNEE2V: usize = 233;
    pub const KICK_RIGHT: usize = 262;
    pub const QUICK_GET_UP: usize = 264;
    pub const KNIFE_ATTACK1: usize = 271;
    pub const KNIFE_ATTACK2: usize = 272;
    pub const KNIFE_ATTACK3: usize = 273;
    pub const KNIFE_FIGHT_READY: usize = 276;
    pub const PISTOL_SHOOT: usize = 297;
    pub const PISTOL_AIM: usize = 298;
    /// The pistol pointed off to the left and right of ahead, and the shotgun's.
    pub const POINT_GUN_LEFT: usize = 290;
    pub const POINT_GUN_RIGHT: usize = 292;
    pub const POINT_SHOTGUN_LEFT: usize = 294;
    pub const POINT_SHOTGUN_RIGHT: usize = 295;
    pub const PISTOL_JOG: usize = 267;
    pub const SHOTGUN_FIRE: usize = 210;
    pub const SHOTGUN_AIM: usize = 293;
    pub const AK_FIRE: usize = 254;
    pub const AK_JOG: usize = 266;
    /// `ANIM_BACK_FLIP`: a standing jump backwards, over and round.
    pub const BACK_FLIP: usize = 105;
    /// `ANIM_WALL_KICK`: a foot against a wall in a jump, to kick off it.
    pub const WALL_KICK: usize = 117;
    /// `ANIM_FLYKICK_*`: a flying kick out of a running jump, the fall after it, and the
    /// landing.
    pub const FLYKICK_START: usize = 279;
    pub const FLYKICK_FALL: usize = 280;
    pub const FLYKICK_LAND: usize = 281;
    pub const ARREST_CROUTCH: usize = 282;
    pub const ARREST_BE_CUFFED: usize = 283;
    /// `ANIM_INJURED_LOOP`: lying hurt (`set_person_injured`).
    pub const INJURED_LOOP: usize = 386;
    /// `ANIM_END` + n (fallen/Headers/animate.h): carrying someone, and `_V` being carried.
    pub const PUTDOWN_CARRY_V: usize = 408;
    pub const STAND_CARRY: usize = 409;
    pub const STAND_CARRY_V: usize = 410;
    pub const PICKUP_CARRY: usize = 427;
    pub const START_WALK_CARRY: usize = 428;
    pub const WALK_CARRY: usize = 429;
    pub const WALK_STOP_CARRY: usize = 430;
    pub const PUTDOWN_CARRY: usize = 431;
    pub const PICKUP_CARRY_V: usize = 432;
    pub const START_WALK_CARRY_V: usize = 433;
    pub const WALK_CARRY_V: usize = 434;
    pub const WALK_STOP_CARRY_V: usize = 435;
    /// `ANIM_BIKE_DISMOUNT`: off a motorbike. The rest of the bike set (`ANIM_BIKE_MOUNT`,
    /// `_RIDE`, `_LEAN*`) isn't in the shipped `darci1.all`.
    pub const BIKE_DISMOUNT: usize = 360;
    /// `ANIM_ENTER_CAR`: getting in at the driver's door.
    pub const ENTER_CAR: usize = 89;
    /// `ANIM_CAN_PICKUP` and `ANIM_CAN_RELEASE`: picking up a can and throwing it (or a
    /// grenade).
    pub const CAN_PICKUP: usize = 64;
    pub const CAN_RELEASE: usize = 65;
    /// The rest of the `ANIM_END` + n slots, taken from the civilians' and Roper's files.
    pub const WANKER: usize = 403;
    /// `ANIM_ENTER_TAXI`: getting in at the passenger's door.
    pub const ENTER_TAXI: usize = 407;
    pub const DANCE_BOOGIE: usize = 416;
    pub const DANCE_WOOGIE: usize = 417;
    pub const DANCE_HEADBANG: usize = 418;
    /// Empty in the original: see [`super::EXTRA_ANIMS`].
    pub const WARM_HANDS: usize = 419;
    pub const HANDS_UP: usize = 420;
    pub const STRANGLE: usize = 421;
    pub const STRANGLE_VICTIM: usize = 422;
    pub const HEADBUTT: usize = 423;
    pub const HEADBUTT_VICTIM: usize = 424;
    pub const HANDS_UP_LOOP: usize = 426;
    pub const HANDS_UP_LIE: usize = 436;
}

/// Roper's file's carrying animations (`NROPER_*`, fallen/Source/Anim.cpp:137) and the
/// slots everyone's set takes them into (Anim.cpp:1398): (from, to).
pub const CARRY_ANIMS: [(usize, usize); 12] = [
    (34, anim::PICKUP_CARRY),
    (35, anim::START_WALK_CARRY),
    (36, anim::WALK_CARRY),
    (37, anim::WALK_STOP_CARRY),
    (38, anim::PUTDOWN_CARRY),
    (39, anim::PICKUP_CARRY_V),
    (40, anim::START_WALK_CARRY_V),
    (41, anim::WALK_CARRY_V),
    (42, anim::WALK_STOP_CARRY_V),
    (43, anim::PUTDOWN_CARRY_V),
    (44, anim::STAND_CARRY),
    (45, anim::STAND_CARRY_V),
];

/// The civilians' animations everyone's set takes in (Anim.cpp:1373): (file, from, to). The
/// civilians' set is `rthug.all` (`load_anim_system(&game_chunk[ANIM_TYPE_CIV], "rthug.all",
/// 2)`, Anim.cpp:957, after `bossprtg.all`), with `newciv.all`'s appended from
/// `CIV_M_START` (100), so its hands-up animations are `newciv.all`'s 16 to 18. Roper's
/// second file gives the strangle and headbutt and their victims' halves.
///
/// `ANIM_WARM_HANDS` is left out: it's `CIV_ANIM_WARM_HANDS` (53), which `rthug.all` doesn't
/// have, so the original's slot is empty too.
pub const EXTRA_ANIMS: [(&str, usize, usize); 12] = [
    ("rthug.all", 13, anim::WANKER),
    ("rthug.all", 27, anim::ENTER_TAXI),
    ("rthug.all", 52, anim::DANCE_BOOGIE),
    ("rthug.all", 58, anim::DANCE_WOOGIE),
    ("rthug.all", 55, anim::DANCE_HEADBANG),
    ("newciv.all", 16, anim::HANDS_UP),
    ("newciv.all", 17, anim::HANDS_UP_LOOP),
    ("newciv.all", 18, anim::HANDS_UP_LIE),
    ("roper2.all", 268, anim::STRANGLE),
    ("roper2.all", 269, anim::STRANGLE_VICTIM),
    ("roper2.all", 270, anim::HEADBUTT),
    ("roper2.all", 271, anim::HEADBUTT_VICTIM),
];

/// One game tick is NORMAL_TICK_TOCK = 1000/15 ms; the tween advances TweenStep*2 per tick.
const TICKS_PER_SECOND: f32 = 15.0;

/// Which meshes to draw: `body` picks the multi-object (`MeshID`), `person` the part table
/// (`PersonID`) of `chunk`.
#[derive(Clone)]
pub struct Model {
    pub chunk: Arc<AnimChunk>,
    pub body: usize,
    pub person: usize,
    /// Part i is the body's mesh i, as an anim prim's are (`object_offset = i` in
    /// `ANIM_obj_draw`, figure.cpp:8225), not looked up in the person types' table.
    pub in_order: bool,
}

impl Model {
    /// The mesh drawn for animation element `part`.
    pub fn mesh(&self, part: usize) -> Option<&uc_formats::prm::Prim> {
        if self.in_order {
            return self.chunk.bodies.get(self.body)?.parts.get(part);
        }
        self.chunk.part_mesh(self.body, self.person, part)
    }
}

/// `PERSON_*` types (fallen/Headers/person.h).
pub mod person {
    pub const DARCI: u8 = 0;
    pub const ROPER: u8 = 1;
    pub const COP: u8 = 2;
    pub const CIV: u8 = 3;
    pub const THUG_RASTA: u8 = 4;
    pub const THUG_GREY: u8 = 5;
    pub const THUG_RED: u8 = 6;
    pub const SLAG_TART: u8 = 7;
    pub const SLAG_FATUGLY: u8 = 8;
    pub const HOSTAGE: u8 = 9;
    pub const MECHANIC: u8 = 10;
    pub const TRAMP: u8 = 11;
    pub const MIB1: u8 = 12;
    pub const MIB2: u8 = 13;
    pub const MIB3: u8 = 14;

    /// `health[]` (fallen/Source/Person.cpp).
    pub const HEALTH: [i32; 15] = [200, 400, 200, 130, 200, 200, 200, 130, 130, 200, 200, 200, 700, 700, 700];
    /// `mesh_type[]`: which multi-object of the person's chunk. Roper's entry is for the thug
    /// file he falls back to; his own body is the only one in `roper.all`.
    pub const MESH: [usize; 15] = [0, 0, 4, 7, 0, 1, 2, 1, 2, 3, 3, 6, 5, 5, 5];
    /// Types whose meshes come from Darci's file (`anim_type[]` == ANIM_TYPE_DARCI).
    pub fn uses_darci_meshes(t: u8) -> bool {
        matches!(t, DARCI | SLAG_TART | SLAG_FATUGLY | HOSTAGE)
    }

    /// What to call someone of this type, for the HUD and for who is saying a line. The
    /// original's panel has no name on it (it just picks Darci's or Roper's face): these are
    /// this engine's own, so a mission played as somebody else reads right.
    pub const NAME: [&str; 15] = [
        "Darci", "Roper", "Cop", "Civilian", "Thug", "Thug", "Thug", "Woman", "Woman", "Hostage",
        "Workman", "Tramp", "MIB", "MIB", "MIB",
    ];

    pub fn name(t: u8) -> &'static str {
        NAME.get(t as usize).copied().unwrap_or("")
    }
}

/// The loaded character files.
#[derive(Clone)]
pub struct People {
    /// darci1.all: Darci's meshes and everybody's animations.
    pub darci: Arc<AnimChunk>,
    /// rthug.all: meshes of most other people.
    pub thug: Arc<AnimChunk>,
    /// trainer.all: replaces the first three thug meshes in the combat tutorial.
    pub trainer: Option<Arc<AnimChunk>>,
    /// roper.all: Roper's own body. The original keeps him in a chunk of his own
    /// (`ANIM_TYPE_ROPER`) while still animating him from Darci's set, like everybody else.
    pub roper: Option<Arc<AnimChunk>>,
}

impl People {
    /// The model for a person of `PERSON_*` type `t`; `variant` picks a civilian's look.
    pub fn model(&self, t: u8, variant: u32) -> Model {
        let t = t.min(person::MIB3);
        let body = person::MESH[t as usize];
        if person::uses_darci_meshes(t) {
            return Model { chunk: self.darci.clone(), body, person: 0, in_order: false };
        }
        // Roper's own body, which is the only one in his file. Without it he'd be a thug.
        if let (person::ROPER, Some(r)) = (t, &self.roper) {
            return Model { chunk: r.clone(), body: 0, person: 0, in_order: false };
        }
        if let (Some(tr), true) = (&self.trainer, body <= 2) {
            return Model { chunk: tr.clone(), body: 0, person: 0, in_order: false };
        }
        let person = if t == person::CIV { 6 + (variant % 4) as usize } else { 0 };
        Model { chunk: self.thug.clone(), body, person, in_order: false }
    }

    pub fn figure(&self, t: u8, variant: u32, pos: Vec3, angle: f32) -> Option<Figure> {
        Figure::new(self.model(t, variant), self.darci.clone(), pos, angle)
    }
}

/// An event point's `Direction` byte as an engine angle in radians.
pub fn ep_angle(direction: u8) -> f32 {
    direction as f32 / 256.0 * std::f32::consts::TAU
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Play {
    Loop,
    /// Play once and hold the last frame.
    Once,
}

/// Plays one animation of a keyframe chain at a time.
#[derive(Debug, Clone)]
pub struct Animator {
    id: usize,
    play: Play,
    start: usize,
    cur: usize,
    next: usize,
    /// 0..256 between `cur` and `next`.
    tween: f32,
    /// Position of `cur` in the animation; None while blending in from the previous one.
    index: Option<usize>,
    len: usize,
    done: bool,
    /// How many times a looping animation has wrapped (never reset).
    loops: u32,
    /// Playback speed multiplier.
    pub speed: f32,
}

/// Number of keyframes in animation `id` (following `next` until it ends or loops).
fn chain_len(chunk: &AnimChunk, start: usize) -> usize {
    let (mut f, mut n) = (start, 1);
    while let Some(next) = chunk.keyframes[f].next {
        if next == start || n >= 256 {
            break;
        }
        f = next;
        n += 1;
    }
    n
}

impl Animator {
    fn new(chunk: &AnimChunk, id: usize) -> Option<Animator> {
        let start = chunk.anims.get(id).copied().flatten()?;
        Some(Animator {
            id, play: Play::Loop, start, cur: start, next: start, tween: 0.0, index: Some(0),
            len: chain_len(chunk, start), done: false, loops: 0, speed: 1.0,
        })
    }

    /// Starts `id` from its first frame, blending in from the current pose. Returns false
    /// (and changes nothing) if the animation doesn't exist.
    fn restart(&mut self, chunk: &AnimChunk, id: usize, play: Play) -> bool {
        let Some(start) = chunk.anims.get(id).copied().flatten() else { return false };
        // Blend from the pose shown now: snap to whichever frame is nearer.
        if self.tween >= 128.0 {
            self.cur = self.next;
        }
        *self = Animator { id, play, start, cur: self.cur, next: start, tween: 0.0, index: None,
                           len: chain_len(chunk, start), done: false, loops: self.loops, speed: 1.0 };
        true
    }

    /// Starts `id` at its first frame straight away, without blending from the current pose.
    fn restart_now(&mut self, chunk: &AnimChunk, id: usize, play: Play) -> bool {
        if !self.restart(chunk, id, play) {
            return false;
        }
        self.cur = self.start;
        self.next = if self.len > 1 { chunk.keyframes[self.start].next.unwrap_or(self.start) } else { self.start };
        self.index = Some(0);
        true
    }

    /// Moves on by `dt`; returns how many keyframes it passed and whether it came to the end
    /// of the animation, wrapping round or holding its last frame (`advance_keyframe`'s "anim
    /// over", which keeps on saying so while a finished animation holds).
    fn advance(&mut self, chunk: &AnimChunk, dt: f32) -> (u32, bool) {
        if self.done {
            return (0, true);
        }
        let (mut passed, mut ended) = (0, false);
        // Blending in from another animation runs at the new animation's pace (an idle
        // pose's slow step would make every move start sluggishly).
        let pace = if self.index.is_none() { self.next } else { self.cur };
        let step = chunk.keyframes[pace].tween_step.max(1) as f32 * 2.0;
        self.tween += step * TICKS_PER_SECOND * dt * self.speed;
        while self.tween >= 256.0 {
            self.tween -= 256.0;
            self.cur = self.next;
            passed += 1;
            let i = match self.index {
                None => 0,
                Some(_) if self.cur == self.start => {
                    self.loops += 1;
                    ended = true;
                    0
                }
                Some(i) => i + 1,
            };
            self.index = Some(i);
            let at_end = i + 1 >= self.len;
            match (at_end, self.play) {
                (false, _) => self.next = chunk.keyframes[self.cur].next.unwrap_or(self.start),
                (true, Play::Loop) => self.next = self.start,
                (true, Play::Once) => {
                    self.next = self.cur;
                    self.tween = 0.0;
                    self.done = true;
                    ended = true;
                    break;
                }
            }
        }
        (passed, ended)
    }

    /// 0 at the start of the animation, 1 at its last frame.
    fn progress(&self) -> f32 {
        match self.index {
            None => 0.0,
            Some(_) if self.done => 1.0,
            Some(i) => ((i as f32 + self.tween / 256.0) / (self.len.max(2) - 1) as f32).min(1.0),
        }
    }
}

/// A posed, animated person in the world.
#[derive(Clone)]
pub struct Figure {
    pub model: Model,
    /// The keyframes (Darci's animation set for everybody).
    pub anims: Arc<AnimChunk>,
    /// Feet position in world units.
    pub pos: Vec3,
    /// Engine angle in radians (2048 units per turn in the original); facing is (-sin, -cos).
    pub angle: f32,
    /// Drawn this much lower than `pos`.
    pub lift: f32,
    /// Also cancel any rise of the hips above standing height (jump animations raise the
    /// body themselves; the physics does that instead).
    pub cancel_rise: bool,
    /// Pin the hips at standing height above `pos`: the animation only moves the limbs, and
    /// the caller moves `pos` along its own path (fence climbing).
    pub pin_root: bool,
    /// Which set of clothes (0 = the model's own, 1-3 = alternatives): the creating event
    /// point's colour & 3 (`tex_page_offset = pcom_colour & 3`, DDEngine/Source/figure.cpp).
    pub clothes: u8,
    /// A gang member's jacket shows how good a fighter they are: skill / 4 (0-3).
    pub jacket: u8,
    /// The colour of the faces with no texture (`POLY_FLAG_TEXTURED` not set), which are then
    /// drawn untextured in it, lit. Only the dog (#110) has such faces: their palette index
    /// (`Col2`) is into `tex01.pal`, which didn't ship. None draws every face textured.
    pub plain: Option<[u8; 3]>,
    animator: Animator,
}

/// `jacket_lookup[][]` (DDEngine/Source/figure.cpp): the four parts of a gang jacket, each
/// in four colours by skill / 4 (mustard Wildcats, green, green, brown). Faces drawn from
/// any of these pages are jacket faces (`TEXTURE_fix_prim_textures` flags them
/// `FACE_FLAG_THUG_JACKET`, DDEngine/Source/texture.cpp).
const JACKETS: [[u32; 4]; 4] = [
    [64 + 21, 10 * 64 + 2, 10 * 64 + 2, 10 * 64 + 32],
    [64 + 22, 10 * 64 + 3, 10 * 64 + 3, 10 * 64 + 33],
    [64 + 24, 10 * 64 + 4, 10 * 64 + 4, 10 * 64 + 36],
    [64 + 25, 10 * 64 + 5, 10 * 64 + 5, 10 * 64 + 37],
];

/// The page drawn for a jacket face modelled on `page`, for a fighter of jacket `level`.
pub fn jacket_page(page: u32, level: u8) -> u32 {
    let raw = page.saturating_sub(8 * 64);
    match JACKETS.iter().find(|row| row.contains(&raw)) {
        Some(row) => row[level.min(3) as usize] + 8 * 64,
        None => page,
    }
}

/// `alt_texture[]` (DDEngine/Source/texture.cpp): person pages that come in other colours,
/// as (page, first alternative). A page `FACE_PAGE_OFFSET + 640 + k` is drawn from
/// `21 * 64 + base + clothes - 1` instead.
const ALT_CLOTHES: [(u32, u32); 10] = [
    (108, 12), // legs
    (109, 0),  // crotch
    (111, 3),  // front
    (112, 6),  // hat, side
    (113, 9),  // hat, front
    (114, 15), // female: seat
    (115, 21), // seam
    (116, 24), // shoe
    (117, 18), // chest
    (118, 27), // back
];

/// The page actually drawn for person texture `page` in clothes set `clothes`.
pub fn clothes_page(page: u32, clothes: u8) -> u32 {
    if clothes == 0 {
        return page;
    }
    match ALT_CLOTHES.iter().find(|&&(k, _)| page == 8 * 64 + 10 * 64 + k) {
        Some(&(_, base)) => 21 * 64 + base + clothes.min(3) as u32 - 1,
        None => page,
    }
}

fn mat3_rows(r: &[[f32; 3]; 3]) -> Mat3 {
    Mat3::from_cols_array_2d(r).transpose()
}

/// The facing direction (xz) of engine angle `a`.
pub fn facing(a: f32) -> Vec2 {
    vec2(-a.sin(), -a.cos())
}

/// The engine angle that faces direction `d` (xz).
pub fn angle_of(d: Vec2) -> f32 {
    (-d.x).atan2(-d.y)
}

impl Figure {
    pub fn new(model: Model, anims: Arc<AnimChunk>, pos: Vec3, angle: f32) -> Option<Figure> {
        let animator = Animator::new(&anims, anim::STAND_READY)?;
        let mut f = Figure { model, anims, pos, angle, lift: 0.0, cancel_rise: false, pin_root: false, clothes: 0, jacket: 0, plain: None, animator };
        f.play(anim::BREATHE, Play::Loop);
        Some(f)
    }

    pub fn anim(&self) -> usize {
        self.animator.id
    }

    pub fn facing(&self) -> Vec2 {
        facing(self.angle)
    }

    /// Plays `id` unless it is already playing. Returns whether `id` is now playing.
    pub fn play(&mut self, id: usize, play: Play) -> bool {
        if self.animator.id == id {
            return true;
        }
        self.animator.restart(&self.anims, id, play)
    }

    /// Plays `id` from the start even if it is already playing.
    pub fn restart(&mut self, id: usize, play: Play) -> bool {
        self.animator.restart(&self.anims, id, play)
    }

    /// Like `restart`, but moves the figure (in xz) so the hips stay where they are, as the
    /// original's `locked_anim_change` does: animations that carry the body away (a knockdown
    /// ends a body length from where it started) chain on without a jump.
    /// The switch is immediate; any change in hip height goes into `lift` (draw offset) for
    /// the caller to settle.
    pub fn restart_locked(&mut self, id: usize, play: Play) -> bool {
        let before = self.root();
        let Some(after) = self.anim_root(id, false) else { return false };
        let d = self.to_world(before - after);
        self.pos += vec3(d.x, 0.0, d.z);
        self.lift += after.y - before.y;
        self.restart_now(id, play)
    }

    /// Plays `id` from its first frame with no blend from the current pose.
    pub fn restart_now(&mut self, id: usize, play: Play) -> bool {
        self.animator.restart_now(&self.anims, id, play)
    }

    /// How many times looping animations have wrapped, with the fraction of a wrap in
    /// progress (blending from the last frame back to the first).
    pub fn loops(&self) -> f32 {
        let a = &self.animator;
        let wrapping = a.index.is_some() && a.next == a.start && a.cur != a.start;
        a.loops as f32 + if wrapping { a.tween / 256.0 } else { 0.0 }
    }

    /// Where the figure is drawn (its position less `lift` and any cancelled rise).
    pub fn drawn_pos(&self) -> Vec3 {
        if self.pin_root {
            return self.pos - self.to_world(self.root() - self.stand_root());
        }
        let rise = if self.cancel_rise { (self.root().y - self.stand_root().y).max(0.0) } else { 0.0 };
        self.pos - vec3(0.0, self.lift + rise, 0.0)
    }

    /// Where the hips are drawn, in world units.
    pub fn hips(&self) -> Vec3 {
        self.drawn_pos() + self.to_world(self.root())
    }

    /// The hips (part 0) offset in the current pose, in character space.
    pub fn root(&self) -> Vec3 {
        let a = &self.animator;
        let off = |f: usize| self.anims.pose(f).map_or(Vec3::ZERO, |p| Vec3::from(p[0].offset));
        off(a.cur).lerp(off(a.next), (a.tween / 256.0).clamp(0.0, 1.0))
    }

    /// The hips offset at the first (or `last`) frame of animation `id`.
    pub fn anim_root(&self, id: usize, last: bool) -> Option<Vec3> {
        let start = self.anims.anims.get(id).copied().flatten()?;
        let mut f = start;
        if last {
            for _ in 1..chain_len(&self.anims, start) {
                f = self.anims.keyframes[f].next?;
            }
        }
        Some(Vec3::from(self.anims.pose(f)?[0].offset))
    }

    /// The hips offset when standing still.
    pub fn stand_root(&self) -> Vec3 {
        self.anim_root(anim::BREATHE, false).unwrap_or(vec3(0.0, 96.0, 0.0))
    }

    /// Where body part `part` is in the world: (rotation, position of its origin).
    pub fn part_world(&self, part: usize) -> Option<(Mat3, Vec3)> {
        let (rot, off) = *self.pose()?.get(part)?;
        let (s, c) = self.angle.sin_cos();
        let r = mat3_rows(&[[c, 0.0, s], [0.0, 1.0, 0.0], [-s, 0.0, c]]);
        Some((r * rot, self.drawn_pos() + r * off))
    }

    /// Character space to world space (rotation only).
    pub fn to_world(&self, v: Vec3) -> Vec3 {
        let (s, c) = self.angle.sin_cos();
        vec3(c * v.x + s * v.z, v.y, -s * v.x + c * v.z)
    }

    pub fn set_speed(&mut self, speed: f32) {
        self.animator.speed = speed;
    }

    /// A `Once` animation has reached its last frame.
    pub fn finished(&self) -> bool {
        self.animator.done
    }

    /// The keyframe the animation is on, counting from 0 (`Draw.Tweened->FrameIndex`); None
    /// while blending in from the one before.
    pub fn frame(&self) -> Option<usize> {
        self.animator.index
    }

    /// How many keyframes the current animation has.
    pub fn frames(&self) -> usize {
        self.animator.len
    }

    /// How far through the current animation (0..1).
    pub fn progress(&self) -> f32 {
        self.animator.progress()
    }

    /// The first keyframe of animation `id`, if this figure has it.
    pub fn first_keyframe(&self, id: usize) -> Option<usize> {
        self.anims.anims.get(id).copied().flatten()
    }

    /// The pose shown: the two keyframes it's between and how far from the first (0..256).
    /// Keyframes are numbered in the game's own animation data, so they mean the same in
    /// every build.
    pub fn keyframes(&self) -> (usize, usize, f32) {
        let a = &self.animator;
        (a.cur, a.next, a.tween)
    }

    /// Shows the pose between keyframes `cur` and `next` (a ghost's, #186). Keyframes the
    /// animations don't have are left as they were.
    pub fn show_keyframes(&mut self, cur: usize, next: usize, tween: f32) {
        let n = self.anims.keyframes.len();
        if cur < n && next < n {
            let a = &mut self.animator;
            (a.cur, a.next, a.tween) = (cur, next, tween.clamp(0.0, 256.0));
        }
    }

    pub fn advance(&mut self, dt: f32) {
        self.animator.advance(&self.anims, dt);
    }

    /// Like [`Figure::advance`], for things that count keyframes: how many it passed, and
    /// whether the animation is over (wrapped, or holding its last frame).
    pub fn advance_frames(&mut self, dt: f32) -> (u32, bool) {
        self.animator.advance(&self.anims, dt)
    }

    /// A figure whose meshes and animations come from one chunk, as an anim prim's do (the
    /// Balrog, Bane, gargoyles: `anim%03d.all`), starting on animation `id`.
    pub fn anim_prim(chunk: Arc<AnimChunk>, pos: Vec3, angle: f32, id: usize) -> Option<Figure> {
        let animator = Animator::new(&chunk, id)?;
        let model = Model { chunk: chunk.clone(), body: 0, person: 0, in_order: true };
        let mut f = Figure { model, anims: chunk, pos, angle, lift: 0.0, cancel_rise: false, pin_root: false, clothes: 0, jacket: 0, plain: None, animator };
        f.animator.play = if f.loops_round(id) { Play::Loop } else { Play::Once };
        Some(f)
    }

    /// Whether animation `id`'s keyframes come back round to its first, or stop at the end.
    pub fn loops_round(&self, id: usize) -> bool {
        let Some(start) = self.anims.anims.get(id).copied().flatten() else { return false };
        let mut f = start;
        for _ in 0..512 {
            match self.anims.keyframes[f].next {
                Some(n) if n == start => return true,
                Some(n) => f = n,
                None => return false,
            }
        }
        false
    }

    /// Turns towards direction `d` at `rate` (fraction per second, 10 = snappy).
    pub fn turn_towards(&mut self, d: Vec2, rate: f32, dt: f32) {
        if d.length_squared() < 1e-6 {
            return;
        }
        let mut delta = angle_of(d) - self.angle;
        delta = (delta + std::f32::consts::PI).rem_euclid(std::f32::consts::TAU) - std::f32::consts::PI;
        self.angle += delta * (dt * rate).min(1.0);
    }

    /// Texture pages this figure's meshes may use, in any clothes (for loading).
    pub fn pages(&self) -> Vec<u32> {
        let m = &self.model;
        (0..self.anims.element_count)
            .filter_map(|part| m.mesh(part))
            .flat_map(|mesh| mesh.faces.iter())
            .filter(|f| self.plain.is_none() || f.textured())
            .flat_map(|f| {
                let p = f.texture().0;
                (0..4).flat_map(move |c| [jacket_page(p, c), clothes_page(p, c)])
            })
            .collect()
    }

    /// The current pose of every part: (rotation, offset) in character space.
    fn pose(&self) -> Option<Vec<(Mat3, Vec3)>> {
        let a = &self.animator;
        let (pa, pb) = (self.anims.pose(a.cur)?, self.anims.pose(a.next)?);
        let t = (a.tween / 256.0).clamp(0.0, 1.0);
        Some(pa.iter().zip(pb).map(|(ea, eb): (&Element, &Element)| {
            let qa = Quat::from_mat3(&mat3_rows(&ea.rotation)).normalize();
            let mut qb = Quat::from_mat3(&mat3_rows(&eb.rotation)).normalize();
            if qa.dot(qb) < 0.0 {
                qb = -qb;
            }
            (Mat3::from_quat(qa.slerp(qb, t)), Vec3::from(ea.offset).lerp(Vec3::from(eb.offset), t))
        }).collect())
    }

    /// Adds this frame's triangles (in square units) to `out`, one batch per texture page.
    pub fn append_batches(&self, out: &mut BTreeMap<u32, Batch>, lighter: Option<&Lighter>) {
        let Some(pose) = self.pose() else { return };
        let (s, c) = self.angle.sin_cos();
        // FIGURE_rotate_obj2(pitch 0, yaw, roll 0): rows [c 0 s] [0 1 0] [-s 0 c].
        let r = mat3_rows(&[[c, 0.0, s], [0.0, 1.0, 0.0], [-s, 0.0, c]]);
        // Light the whole body from its chest position (the original lights per point).
        let chest = self.pos + vec3(0.0, 120.0, 0.0);
        let pos = self.drawn_pos();
        let m = &self.model;
        for (part, (rot, off)) in pose.iter().enumerate() {
            let Some(mesh) = m.mesh(part) else { continue };
            let world: Vec<Vec3> = mesh
                .points
                .iter()
                .map(|p| (pos + r * (*rot * vec3(p[0] as f32, p[1] as f32, p[2] as f32) + *off)) / UNIT)
                .collect();
            for f in &mesh.faces {
                let (page, uv) = f.texture();
                let page = clothes_page(jacket_page(page, self.jacket), self.clothes);
                let pts: Vec<Vec3> = f.points.iter().map(|&i| world[i as usize]).collect();
                let n = (pts[1] - pts[0]).cross(pts[2] - pts[0]);
                let mut col = match lighter {
                    Some(l) => {
                        let [cr, cg, cb] = l.light(chest.to_array(), n.to_array());
                        [(cr * 255.0) as u8, (cg * 255.0) as u8, (cb * 255.0) as u8, 255]
                    }
                    None => [255; 4],
                };
                let plain = self.plain.filter(|_| !f.textured());
                let page = match plain {
                    Some(tint) => {
                        // Palette entry 0, on the flat faces, is taken to be black: drawn darker.
                        let k = if f.colour == 0 { 0.4 } else { 1.0 };
                        for i in 0..3 {
                            col[i] = (col[i] as f32 * tint[i] as f32 / 255.0 * k) as u8;
                        }
                        crate::UNTEXTURED
                    }
                    None => page,
                };
                let b = out.entry(page).or_default();
                if pts.len() == 4 {
                    b.quad([pts[0], pts[1], pts[2], pts[3]], [uv[0], uv[1], uv[2], uv[3]], [col; 4]);
                } else {
                    b.tri([pts[0], pts[1], pts[2]], [uv[0], uv[1], uv[2]], [col; 3]);
                }
            }
        }
    }
}
