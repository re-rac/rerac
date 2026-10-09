//! **Gemlik's target switches, class 231** (level13 `0x2e4448`, its hits `0x2e4300`; census U462; 4 placed, one moby
//! group). The group's first member is its master (+0x80); the others are marked followers (+0x79). Each switch shot
//! down to no health turns green (sound 3) and counts on the master (+0x7a; the first starts the master's timer
//! `ticks(+0x7c)` when one is set). While the master waits (state 2) with switches still standing, a timed group
//! beeps (sound 0) faster as its time runs out; out of time: sound 2, the group to state 3 (grey again, health back,
//! 1). All shot: the group to state 4 (sound 1), the master's time +0x84; done switches blink blue / green every
//! `ticks(20)`, and a master with a time left keeps counting it down the same way.
//!
//! **Pvars**: +0x20 the damage record, +0x60 the flash, +0x70 s16 the time, +0x72 s16 the beep timer, +0x74 s16 hit,
//! +0x76 s16 the blink timer, +0x78 the blink, +0x79 follower, +0x7a the count, +0x7b the group's size, +0x7c the time
//! (ticks, −1 none), +0x80 the master (the moby + 1 here), +0x84 s16 the time after.
//!
//! | address | what | port |
//! |---|---|---|
//! | `0x2e4448` | the switch (module doc); colours through `0x25bf78` (= L01 `0x2650d0`: the ambient bytes) | [`update`] |
//! | `0x2e4300` | the hits (0x330000, column 4): health spent → untargetable, flash 0x78, 1; else flash 0xfa, cooldown | [`hits`] |
//!
//! Read from the level13 decomp. The game's debug prints are left out. Native `f32`.

use crate::moby_runtime::{mode, MobyId};
use crate::moby_update::creature::{self as c, damage, flash};
use crate::moby_update::scheduler::{group_state, group_walk, GroupWalk};
use crate::moby_update::services::World;
use crate::moby_update::story;

pub const REFERENCE_LEVEL: u32 = 13;
pub const UPDATE_FN: u32 = 0x2e_4448;
pub const CLASSES: [i16; 1] = [231];

mod pv_ {
    pub const D: usize = 0x20;
    pub const COOLDOWN: usize = 0x26;
    pub const F: usize = 0x60;
    pub const TIME: usize = 0x70;
    pub const BEEP: usize = 0x72;
    pub const HIT: usize = 0x74;
    pub const BLINK_T: usize = 0x76;
    pub const BLINK: usize = 0x78;
    pub const FOLLOWER: usize = 0x79;
    pub const COUNT: usize = 0x7a;
    pub const SIZE_: usize = 0x7b;
    pub const LIMIT: usize = 0x7c;
    pub const MASTER: usize = 0x80;
    pub const AFTER: usize = 0x84;
    pub const SIZE: usize = 0x88;
}
use pv_ as o;

/// `0x25bf78(m, r, g, b)`: the ambient colour.
fn colour(w: &mut World, id: MobyId, r: u8, g: u8, b: u8) { w.mm(id).ambient = [r, g, b, 0]; }
fn master(w: &World, id: MobyId) -> Option<MobyId> { usize::try_from(c::pi32(w, id, o::MASTER) - 1).ok().filter(|&m| m < w.table.mobys.len()) }

/// `0x2e4300(m, P, D)`: the hits (module doc). True: shot down.
fn hits(w: &mut World, id: MobyId) -> bool {
    let hit = w.get_hit(id, 0x33_0000, false);
    let res = damage::resolve(w, id, hit, o::D, 0, 4);
    if matches!(res.reaction, 1 | 2) { c::set_pf(w, id, o::D, 0.0); }
    let mut down = false;
    if let Some(h) = res.hit.filter(|_| 1 < res.out5) {
        let dmg = h.damage.to_f32();
        let hp = c::pf(w, id, o::D);
        if hp <= dmg {
            c::set_pf(w, id, o::D, 0.0);
            down = true;
            w.mm(id).mode &= !mode::TARGETABLE;
            c::set_pu8(w, id, o::F + 7, 0x78);
        } else {
            c::set_pf(w, id, o::D, hp - dmg);
            c::set_pu8(w, id, o::F + 7, 0xfa);
            let t = w.ticks(0x3c);
            c::set_pi16(w, id, o::COOLDOWN, t as i16);
        }
        flash::start(w, id, o::F);
    }
    w.mm(id).hit_slot = 0xff;
    flash::update(w, id, o::F);
    down
}

/// The countdown with its beeps (states 2 and 4). True: out of time (the group reset, → 3).
fn countdown(w: &mut World, id: MobyId) -> bool {
    if c::dec_timer_pvar_s16(w, id, o::TIME) == 0 {
        if c::dec_timer_pvar_s16(w, id, o::BEEP) != 0 {
            w.play_sound(0, 0, id);
            let (t45, t5, t40) = (w.ticks(0x2d), w.ticks(5), w.ticks(0x28).max(1));
            let v = t5 + c::pi16(w, id, o::TIME) as i32 / t40;
            c::set_pi16(w, id, o::BEEP, if t45 < v { t45 } else { v } as i16);
        }
        return false;
    }
    c::set_pi16(w, id, o::HIT, 0);
    c::set_pu8(w, id, o::COUNT, 0);
    w.play_sound(2, 0, id);
    let g = w.m(id).group;
    group_state(w, g, 3);
    w.mm(id).state = 3;
    true
}

/// Level13 `0x2e4448` (module doc).
pub fn update(w: &mut World, id: MobyId) {
    story::pvars(w, id, o::SIZE);
    let st = w.m(id).state;
    let down = c::pi16(w, id, o::HIT) == 0 && (st == 1 || st == 2) && hits(w, id);
    match st {
        0 => {
            if c::pu8(w, id, o::FOLLOWER) == 0 {
                c::set_pu8(w, id, o::SIZE_, 0);
                c::set_pi32(w, id, o::MASTER, id as i32 + 1);
                let g = w.m(id).group as u8 as i32;
                for m in group_walk(w, g, GroupWalk::of(false, false)) {
                    if m != id && w.m(m).o_class == 0xe7 && w.m(m).pvars.len() >= o::SIZE {
                        c::set_pu8(w, m, o::FOLLOWER, 1);
                        c::set_pi32(w, m, o::MASTER, id as i32 + 1);
                    }
                    let n = c::pu8(w, id, o::SIZE_).wrapping_add(1);
                    c::set_pu8(w, id, o::SIZE_, n);
                }
            }
            c::set_pi16(w, id, o::BLINK_T, 0);
            c::set_pi16(w, id, o::TIME, 0);
            c::set_pi16(w, id, o::BEEP, 0);
            c::set_pu8(w, id, o::COUNT, 0);
            c::set_pi16(w, id, o::HIT, 0);
            colour(w, id, 0x80, 0x80, 0x80);
            w.mm(id).state = 1;
        }
        1 | 2 => {
            if c::pi16(w, id, o::HIT) == 0 && down {
                if let Some(m) = master(w, id).filter(|&m| w.m(m).pvars.len() >= o::SIZE) {
                    if w.m(m).state == 1 {
                        c::set_pu8(w, id, o::COUNT, 0);
                        w.mm(m).state = 2;
                        let lim = c::pi32(w, m, o::LIMIT);
                        if 0 < lim {
                            let own = c::pi32(w, id, o::LIMIT);
                            let t = w.ticks(own);
                            c::set_pi16(w, m, o::TIME, t as i16);
                        }
                    }
                    let n = c::pu8(w, m, o::COUNT).wrapping_add(1);
                    c::set_pu8(w, m, o::COUNT, n);
                }
                w.play_sound(3, 0, id);
                colour(w, id, 0, 0xff, 0);
                c::set_pi16(w, id, o::HIT, 1);
            }
            if w.m(id).state == 2 {
                if c::pu8(w, id, o::COUNT) < c::pu8(w, id, o::SIZE_) {
                    if -1 < c::pi32(w, id, o::LIMIT) { countdown(w, id); }
                } else {
                    let g = w.m(id).group;
                    group_state(w, g, 4);
                    w.play_sound(1, 0, id);
                    let t = c::pi16(w, id, o::AFTER);
                    c::set_pi16(w, id, o::TIME, t);
                    w.mm(id).state = 4;
                }
            }
        }
        3 => {
            colour(w, id, 0x80, 0x80, 0x80);
            c::set_pu8(w, id, o::COUNT, 0);
            c::set_pi16(w, id, o::HIT, 0);
            let hp = c::pi16(w, id, o::D + 4) as f32;
            c::set_pf(w, id, o::D, hp);
            w.mm(id).state = 1;
        }
        4 => {
            if c::pu8(w, id, o::FOLLOWER) == 0 && c::pi16(w, id, o::TIME) != -1 { countdown(w, id); }
            if c::dec_timer_pvar_s16(w, id, o::BLINK_T) != 0 {
                let t = w.ticks(0x14);
                c::set_pi16(w, id, o::BLINK_T, t as i16);
                if c::pu8(w, id, o::BLINK) == 0 {
                    colour(w, id, 0, 0, 0xff);
                    c::set_pu8(w, id, o::BLINK, 1);
                } else {
                    colour(w, id, 0, 0xff, 0);
                    c::set_pu8(w, id, o::BLINK, 0);
                }
            }
        }
        _ => {}
    }
}
