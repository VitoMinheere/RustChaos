//! The animations everyone's set takes in from other characters' files
//! (`setup_global_anim_array`, fallen/Source/Anim.cpp:1373): each slot from `ANIM_END` up
//! is filled, with frames of its own. Skipped without an install.

use uc_world::character::{anim, CARRY_ANIMS, EXTRA_ANIMS};
use uc_world::{Level, LoadOptions};

#[test]
fn every_extra_slot_is_filled() {
    let Some(game) = uc_formats::default_install() else { return };
    let level = Level::load(&game, "police1", &LoadOptions { fullbright: true, character: true, overrides: None }).unwrap();
    let darci = &level.people.as_ref().unwrap().darci;
    let slots = EXTRA_ANIMS.iter().map(|&(_, _, to)| to).chain(CARRY_ANIMS.iter().map(|&(_, to)| to)).chain([anim::ENTER_CAR]);
    for slot in slots {
        let start = darci.anims.get(slot).copied().flatten().unwrap_or_else(|| panic!("slot {slot} is empty"));
        assert!(darci.pose(start).is_some(), "slot {slot} has no pose");
    }
}
