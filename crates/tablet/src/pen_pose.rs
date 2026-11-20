//! Windows Ink pen tilt and twist, from `POINTER_PEN_INFO` (`winuser.h`).
//!
//! The mask bits are the public `PEN_MASK_*` values. Tilt is degrees, −90..90, with the same
//! sign as Pointer Events: +x to the right, +y towards the user (Microsoft documents +tiltY as
//! towards the user). Rotation is degrees clockwise, 0..360. A packet that sets none of the
//! three bits carries no pose; the last one stays.

/// `PEN_MASK_ROTATION`.
pub const MASK_ROTATION: u32 = 0x0000_0002;
/// `PEN_MASK_TILT_X`.
pub const MASK_TILT_X: u32 = 0x0000_0004;
/// `PEN_MASK_TILT_Y`.
pub const MASK_TILT_Y: u32 = 0x0000_0008;

/// Tilt and twist from one pen packet, in degrees.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PenPose {
    pub tilt_x: f32,
    pub tilt_y: f32,
    pub rotation: f32,
}

/// The pose `mask` says this packet carries. `None` when it carries none of tilt or rotation.
pub fn windows_ink_pose(mask: u32, tilt_x: i32, tilt_y: i32, rotation: u32) -> Option<PenPose> {
    let has_x = mask & MASK_TILT_X != 0;
    let has_y = mask & MASK_TILT_Y != 0;
    let has_rotation = mask & MASK_ROTATION != 0;
    if !has_x && !has_y && !has_rotation {
        return None;
    }
    let degrees = |v: i32| (v as f32).clamp(-90.0, 90.0);
    let mut turn = if has_rotation { (rotation as f32).rem_euclid(360.0) } else { 0.0 };
    if turn >= 360.0 {
        turn = 0.0;
    }
    Some(PenPose { tilt_x: if has_x { degrees(tilt_x) } else { 0.0 }, tilt_y: if has_y { degrees(tilt_y) } else { 0.0 }, rotation: turn })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_packet_without_tilt_or_twist_has_no_pose() {
        assert_eq!(windows_ink_pose(0, 30, -15, 90), None);
        assert_eq!(windows_ink_pose(0x0000_0001, 30, -15, 90), None, "pressure alone is not a pose");
    }

    #[test]
    fn tilt_keeps_the_pointer_events_sign_and_stays_in_range() {
        let pose = windows_ink_pose(MASK_TILT_X | MASK_TILT_Y, 30, -15, 0).expect("both axes");
        assert_eq!(pose, PenPose { tilt_x: 30.0, tilt_y: -15.0, rotation: 0.0 });
        let pose = windows_ink_pose(MASK_TILT_X | MASK_TILT_Y, 200, -200, 0).expect("clamped");
        assert_eq!((pose.tilt_x, pose.tilt_y), (90.0, -90.0));
    }

    #[test]
    fn rotation_is_clockwise_degrees() {
        let pose = windows_ink_pose(MASK_ROTATION, 0, 0, 270).expect("twist");
        assert_eq!(pose, PenPose { tilt_x: 0.0, tilt_y: 0.0, rotation: 270.0 });
        let pose = windows_ink_pose(MASK_ROTATION, 0, 0, 360).expect("a full turn");
        assert_eq!(pose.rotation, 0.0);
    }
}
