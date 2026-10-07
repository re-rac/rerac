//! U106 (census 2026-10-02): classes 735 and 736, Aridia's swinging gate halves (level02 0x2df948, one each). Read from
//! the level02 decomp. `Pf` for the quaternion steps (the VU0 code), `f32` elsewhere.
//!
//! A gate half holds its placed pose until something sets its `cmd` (+0xbc), then swings to its target pose (pvar
//! +0x10..+0x18, Euler degrees) over a couple of seconds by a normalised quaternion blend. When its mission is done at
//! load it stands in the target pose at once. In a scene (game mode 2) it is hidden until its `cmd` is set.
//!
//! **Pvars**: +0x00 the placed pose's quaternion, +0x10 the target Euler (degrees; then its quaternion), +0x20 the blend's
//! velocity, +0x24 the blend.
//!
//! | address | what | port |
//! |---|---|---|
//! | every tick | game mode 2 (0x15f5c4) and `cmd` 0 → not drawn (+0x31 = 0), mode \|= 1; else drawn, mode &= ~1 | [`update`] |
//! | 0 | +0x00 = the Euler (w 0); mission done → Euler = the target · π/180, 3; else 1, mode \|= 0x100 (its rows kept), +0x10 = q(x)·q(y)·q(z) of the target, +0x00 = the same of the placed Euler (`0x221e38`, `0x221d78`) | [`update`] (`services::axis_quat`, `quat_mul`) |
//! | 1 | `cmd` ≠ 0 → 2, blend and velocity 0 | [`update`] |
//! | 2 | `0x270cc0(1, 0.5·dt², 0.5·dt², dt, blend, velocity)`; the rows = `quat_rows(nlerp(blend, +0x00, +0x10))` (`0x221db8`, `0x221ef8`) | [`update`] (`turn::turn_toward`, `services::quat_nlerp`, `quat_rows`) |

#![allow(clippy::needless_range_loop)] // the game's per-lane VU writes, spelled out.

use crate::moby_runtime::{mode, MobyId};
use crate::moby_update::creature::{self as c, turn, DT, DT2};
use crate::moby_update::services::{self as sv, pvar as p, World};
use crate::ps2v::Pf;

pub const REFERENCE_LEVEL: u32 = 2;
pub const UPDATE_FN: u32 = 0x2d_f948;
pub const CLASSES: [i16; 2] = [735, 736];

const Q0: usize = 0x00;
const TARGET: usize = 0x10;
const VEL: usize = 0x20;
const BLEND: usize = 0x24;
const SIZE: usize = 0x28;
const DEG: f32 = 0.017_453_292;

/// q(x)·q(y)·q(z) of the Euler `e` (radians).
fn euler_quat(e: [f32; 3]) -> crate::hero::physics::V4 {
    let qx = sv::axis_quat(Pf::f(e[0]), 0);
    let qy = sv::axis_quat(Pf::f(e[1]), 1);
    let qz = sv::axis_quat(Pf::f(e[2]), 2);
    sv::quat_mul(sv::quat_mul(qx, qy), qz)
}

/// Level02 0x2df948 (module doc).
pub fn update(w: &mut World, id: MobyId) {
    if w.m(id).pvars.len() < SIZE { return; }
    let hide = w.svc.game_mode == 2 && w.m(id).cmd == 0;
    {
        let m = w.mm(id);
        if hide {
            m.visible = 0;
            m.mode |= mode::HIDDEN;
        } else {
            m.visible = 1;
            m.mode &= !mode::HIDDEN;
        }
    }
    match w.m(id).state {
        0 => {
            let r = w.m(id).rotation;
            c::set_pv4(w, id, Q0, [r[0], r[1], r[2], 0.0]);
            let t = [c::pf(w, id, TARGET), c::pf(w, id, TARGET + 4), c::pf(w, id, TARGET + 8)];
            let level = w.svc.level;
            if w.mission_done(level, w.m(id).mission) == 0xff {
                let m = w.mm(id);
                m.rotation[0] = t[0] * DEG;
                m.rotation[1] = t[1] * DEG;
                m.rotation[2] = t[2] * DEG;
                m.state = 3;
            } else {
                w.mm(id).state = 1;
                w.mm(id).mode |= mode::KEEP_ROWS;
                let q1 = euler_quat([t[0] * DEG, t[1] * DEG, t[2] * DEG]);
                let q0 = euler_quat([r[0], r[1], r[2]]);
                let m = w.mm(id);
                p::set_v4(&mut m.pvars, TARGET, q1);
                p::set_v4(&mut m.pvars, Q0, q0);
            }
        }
        1 => {
            if w.m(id).cmd != 0 {
                w.mm(id).state = 2;
                c::set_pf(w, id, BLEND, 0.0);
                c::set_pf(w, id, VEL, 0.0);
            }
        }
        2 => {
            let mut b = c::pf(w, id, BLEND);
            let mut v = c::pf(w, id, VEL);
            turn::turn_toward(1.0, DT2 * 0.5, DT2 * 0.5, DT, &mut b, &mut v);
            c::set_pf(w, id, BLEND, b);
            c::set_pf(w, id, VEL, v);
            let m = w.mm(id);
            let q = sv::quat_nlerp(Pf::f(b), p::v4(&m.pvars, Q0), p::v4(&m.pvars, TARGET));
            let r = sv::quat_rows(q);
            for k in 0..3 { m.rows[k] = r[k].map(|x| x.to_f32()); }
        }
        _ => {}
    }
}
