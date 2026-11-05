//! Inverted event points (`WPT_FLAGS_INVERSE`, #231): a point with the flag counts when its
//! trigger is false. Nitro Car's point 13 is "Mako is not in the car" (`person_in_vehicle`,
//! active while), which with point 5 tells him to get in; Transmission Terminated's 81 has no
//! trigger, so inverted it never goes off. Skipped without an install.

use uc_game::Controls;

mod common;

#[test]
fn an_inverted_point_holds_while_its_trigger_doesnt() {
    let Some(mut r) = common::start("Bankbomb1") else { return };
    r.play(1.0, Controls::default());
    assert!(r.g.point_active(13), "Mako isn't in the car, so 'not in the car' should hold");
    assert!(r.get_in(2), "Mako should get into the car bomb");
    r.play(0.5, Controls::default());
    assert!(!r.g.point_active(13), "once he's in, 'not in the car' should stop holding");
}

#[test]
fn an_inverted_point_with_no_trigger_never_goes_off() {
    let Some(mut r) = common::start("factory1") else { return };
    r.play(2.0, Controls::default());
    assert!(!r.g.point_active(81), "an inverted 'always' is 'never'");
}
