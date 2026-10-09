//! Gemlik's breakable props, class 1805 (level 13, 8 created instances): level13 0x30cdb8 (census U438). A prop with
//! health that flashes red when hit and breaks when a hit's damage exceeds what is left: class sound 0, a bolt
//! burst, both death bits (it stays broken), eight pieces (classes 0x709–0x70c twice) scattered about 1.7 up and a
//! beam explosion. Breaking the last of its group earns a skill point. One already broken on an earlier
//! visit is deleted at once. Read from the level13 decomp of 0x30cdb8 and the disassembly of the explosion call (its
//! stack arguments). Native `f32`; the `rand` draws in the game's order.
//!
//! **Pvar block**: +0x20 the damage record (health +0x20), +0x60 the flash record (+0x67 the red).
//!
//! | address | what | port |
//! |---|---|---|
//! | 0x30cdb8 | the persistent death bit (`0x14c190 + level·0x100`, spawn id +0xb2) set → `DeleteMoby` | [`update`] (`Services::save`) |
//! | | `MobyGetHitMessage(m, 0x10000, 0)` (0x2664a8 = L01 0x26f320); `0x266500(m, hit, +0x20, 0, &out, 0, 0, 4)` (L01 0x26f378) | [`update`] (`damage::resolve`) |
//! | state 0 | → 1 | [`update`] |
//! | state 1 | a hit: health < its damage (+0x2c) → 2; else red +0x67 = 0xfa, health −= damage, the flash `0x2694a0` (L01 0x272318); +0xa4 = 0xff; every tick the flash `0x269580` (L01 0x2723f8) | [`update`] (`flash::start` / `update`) |
//! | state 2 | group ≠ −1, `MobyGroupCount(group, −1)` (0x264f78 = L01 0x26e008) = 1 and the skill point byte 0x13d41e clear → set, `PlayLevelSoundAtMoby(1, 0, 0)`, `ShowBanner(0x53d6, −1)` | [`update`] (`story::award_skill_point`) |
//! | | `PlayClassSound(0, 0, m)`; `BreakFxA(m)` (0x26f828 = L01 0x2787a0: 4..7 bolts); both death bits set (persistent, and this visit's 0x1ba7d0) | [`update`] (`breakables::bolts`) |
//! | | twice, classes 0x709..0x70c: p = (0, 0, 0) jittered by `randf(−1.5, 1.5)` per axis (0x26b928 = L01 0x2747a0), z + 1.7, + position; `BreakFxB(0, m, class, p, rotation, 0, 0, 0, 0)` (0x26f990 = L01 0x278ad8) | [`update`] (`fx::jitter`, `fx::break_piece`) |
//! | | `SpawnBeamExplosion(0, 0, 6, 3, 9, 1.1, 15, m, position, NULL, 10, 3, 16, sound 0, shake, 3 debris, −1, 0)` (0x26a498 = L01 0x273310); `DeleteMoby` | [`update`] (`fx::beam_explosion`) |

use crate::moby_update::classes::breakables;
use crate::moby_update::creature::{self as c, damage, flash, fx};
use crate::moby_runtime::MobyId;
use crate::moby_update::services::World;

/// The update in the level13 class table.
pub const UPDATE_FN: u32 = 0x30_cdb8;
pub const REFERENCE_LEVEL: u32 = 13;
pub const CLASSES: [i16; 1] = [1805];
pub const RECORD: usize = 0x20;
pub const FLASH: usize = 0x60;
/// The pieces (`BreakFxB`), each made twice.
pub const PIECES: [i16; 4] = [0x709, 0x70a, 0x70b, 0x70c];
pub const BLAST: fx::Beam = fx::Beam { damage_r: 0.0, damage: 0.0, flash: 6.0, flash2: 3.0, flash_dist: 9.0, scale: 1.1, light: 15.0, streaks: 10, sparks: 3, puffs: 0x10, debris: 3, sound: 0, shake: true };

fn dead(w: &World, id: MobyId) -> bool {
    let b2 = w.m(id).spawn_id;
    b2 >= 0 && w.svc.save.death.contains(&(w.svc.level, b2))
}

/// Level13 0x30cdb8 (module doc).
pub fn update(w: &mut World, id: MobyId) {
    if dead(w, id) {
        w.delete_moby(id);
        return;
    }
    if w.m(id).pvars.len() < FLASH + 0x10 { return; }
    let hit = w.get_hit(id, 0x1_0000, false);
    let res = damage::resolve(w, id, hit, RECORD, 0, 4);
    match w.m(id).state {
        0 => w.mm(id).state = 1,
        1 => {
            if let Some(h) = res.hit {
                let dmg = h.damage.to_f32();
                let hp = c::pf(w, id, RECORD);
                if hp < dmg {
                    w.mm(id).state = 2;
                } else {
                    c::set_pu8(w, id, FLASH + 7, 0xfa);
                    c::set_pf(w, id, RECORD, hp - dmg);
                    flash::start(w, id, FLASH);
                }
                w.mm(id).hit_slot = 0xff;
            }
            flash::update(w, id, FLASH);
        }
        2 => {
            // The last of its group (itself still counted): skill point 0x13d41e.
            let g = w.m(id).group;
            if g != -1 && crate::moby_update::scheduler::group_count(w, g as i32, -1) == 1 {
                crate::moby_update::story::award_skill_point(w, crate::moby_update::story::skill_index(0x13_d41e));
            }
            w.play_sound(0, 0, id);
            breakables::bolts(w, id);
            let (lvl, b2) = (w.svc.level, w.m(id).spawn_id);
            if b2 >= 0 {
                w.svc.save.death.insert((lvl, b2));
                w.svc.save.death_level.insert(b2);
            }
            let (pos, rot) = (w.m(id).position, w.m(id).rotation);
            for _ in 0..2 {
                for cl in PIECES {
                    let mut p = [0.0, 0.0, 0.0, 0.0];
                    fx::jitter(w, 1.5, &mut p);
                    p[2] += 1.7;
                    let p = c::add(p, pos);
                    fx::break_piece(w, id, cl, p, rot, 0, 0);
                }
            }
            fx::beam_explosion(w, &BLAST, Some(id), pos);
            w.delete_moby(id);
        }
        _ => {}
    }
}
