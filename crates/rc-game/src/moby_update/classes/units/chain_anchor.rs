//! The chain anchors, class 1172: level09 0x303d10 (census U307; 10 created instances on Gaspar). The end of a chain
//! of links 1181 (`chain_link`, U303: the links find their ends by walking the moby list) that a lava platform hangs
//! from. It keeps a loop (class sound 1) going, takes weapon hits on its damage record (the creature resolver): broken
//! (or told by the chain, +0xbc = 1) it tells its partner (+0x78), then after 10 ticks its next link (+0x70, a 1172 /
//! 1181 / 1184), and goes up in a beam explosion. A set global flag deletes it at the start. Read from the level09
//! decomp and disassembly (0x303d10, 0x303b30). Native `f32`.
//!
//! **Pvar block**: +0x20 the damage record (+0x20 health, +0x26 s16 its cooldown), +0x60 the flash record
//! (`creature::flash`; +0x67 the flash's red), +0x70 the next chain moby, +0x74 s16 the break timer, +0x76 s16 the
//! global flag (−1 none), +0x78 the partner, +0x7c the loop's voice slot.
//!
//! | address | what | port |
//! |---|---|---|
//! | 0x303b30 | state ≠ 3: `MobyGetHitMessage(m, 0x330000, 0)` (a debug `printf`), `0x26f378(m, hit, record, 0, &out, 0, 0, 4)`; reactions 1 / 2 → health 0; out > 1: damage ≥ health → health 0, mode & ~0x1000, the next chain moby (1172 / 1181 / 1184, not deleted) +0xbc = 1, `SetDeathBits(m, 0, −1)` (0x26c250), flash red 0x78, `0x272318`, state 3; else health −= damage, flash red 0xfa, +0x26 = ticks(60), `0x272318`; +0xa4 = 0xff | [`hits`] (`damage::resolve`, `crate_::set_death_bits`, `flash::start`) |
//! | | every tick: the flash `0x2723f8` | [`hits`] (`flash::update`) |
//! | 0x303d10 state 0 | the flag `0x13d3c1[+0x76]` set → `DeleteMoby`; else slot −1, → 1 | [`update`] |
//! | state 1 | on its tick phase ((0x15f5cc & 7) = (address >> 8) & 7) with no voice (slot −1): `PlayClassSound(1, 4, m)` | [`update`] (`kalebo_traffic::slot_phase`) |
//! | | +0xbc = 1: the partner (+0x78) in state 1 → its +0xbc = 1; +0x74 = ticks(10); the loop released when it owns it; slot −1; → 2; +0x30 = 0xff | [`update`] |
//! | state 2 | `FastDecTimer_s16(+0x74)` done → the next chain moby (1172 / 1181 / 1184, not deleted): +0xbc = 1, +0x30 = 0xff; → 3 | [`update`] |
//! | state 3 | `SpawnBeamExplosion(0, 0, 10, 7, 20, 2, 40, m, camera − m, NULL → m's position, 20, 9, 32, sound 0, no shake, 1 debris)`, `DeleteMoby` | [`update`] (`fx::beam_explosion`) |
//! | | no light or particle of its own | n/a |

use crate::moby_runtime::MobyId;
use crate::moby_update::creature::{damage, dec_timer_pvar_s16, flash, fx, pf, pi32, pi16, set_pf, set_pi16, set_pi32, set_pu8};
use crate::moby_update::services::World;

/// The update in the level09 class table.
pub const UPDATE_FN: u32 = 0x30_3d10;
pub const REFERENCE_LEVEL: u32 = 9;
pub const CLASSES: [i16; 1] = [1172];
/// The chain classes the break passes along (1172, 1181 links, 1184 cores).
pub const CHAIN: [i16; 3] = [1172, 1181, 1184];

/// Pvar offsets.
pub mod pv {
    pub const RECORD: usize = 0x20;
    pub const FLASH: usize = 0x60;
    pub const NEXT: usize = 0x70;
    pub const TIMER: usize = 0x74;
    pub const FLAG: usize = 0x76;
    pub const PARTNER: usize = 0x78;
    pub const SLOT: usize = 0x7c;
}

/// The explosion (0x303f30).
pub const BLAST: fx::Beam = fx::Beam { damage_r: 0.0, damage: 0.0, flash: 10.0, flash2: 7.0, flash_dist: 20.0, scale: 2.0, light: 40.0, streaks: 20, sparks: 9, puffs: 32, debris: 1, sound: 0, shake: false };

/// The next chain moby (+0x70) when it is live and of a chain class.
fn next(w: &World, id: MobyId) -> Option<MobyId> {
    let n = usize::try_from(pi32(w, id, pv::NEXT)).ok()?;
    let m = w.table.mobys.get(n)?;
    (m.state != 0xfe && m.state != 0xfd && CHAIN.contains(&m.o_class)).then_some(n)
}

/// Level09 0x303b30: the damage and the flash (module doc).
pub fn hits(w: &mut World, id: MobyId) {
    if w.m(id).state != 3 {
        let hit = w.get_hit(id, 0x33_0000, false);
        let r = damage::resolve(w, id, hit, pv::RECORD, 0, 4);
        if matches!(r.reaction, 1 | 2) { set_pf(w, id, pv::RECORD, 0.0); }
        if 1 < r.out5 {
            let dmg = r.damage;
            let health = pf(w, id, pv::RECORD);
            if health <= dmg {
                set_pf(w, id, pv::RECORD, 0.0);
                w.mm(id).mode &= !crate::moby_runtime::mode::TARGETABLE;
                if let Some(n) = next(w, id) { w.mm(n).cmd = 1; }
                crate::moby_update::classes::crate_::set_death_bits(w, id, 0, -1);
                set_pu8(w, id, pv::FLASH + 7, 0x78);
                flash::start(w, id, pv::FLASH);
                w.mm(id).state = 3;
            } else {
                set_pf(w, id, pv::RECORD, health - dmg);
                set_pu8(w, id, pv::FLASH + 7, 0xfa);
                let t = w.ticks(0x3c);
                set_pi16(w, id, pv::RECORD + 6, t as i16);
                flash::start(w, id, pv::FLASH);
            }
        }
        w.mm(id).hit_slot = 0xff;
    }
    flash::update(w, id, pv::FLASH);
}

/// Level09 0x303d10 (module doc).
pub fn update(w: &mut World, id: MobyId) {
    if w.m(id).pvars.len() < 0x80 { return; }
    hits(w, id);
    match w.m(id).state {
        0 => {
            let f = pi16(w, id, pv::FLAG);
            if f != -1 && w.svc.interact.game.flags.get((f as i32 + 0x39) as usize).is_some_and(|&b| b != 0) {
                w.delete_moby(id);
                return;
            }
            set_pi32(w, id, pv::SLOT, -1);
            w.mm(id).state = 1;
        }
        1 => {
            if w.counter & 7 == super::kalebo_traffic::slot_phase(id) & 7 && pi32(w, id, pv::SLOT) == -1 {
                let s = w.play_sound(1, 4, id);
                set_pi32(w, id, pv::SLOT, s);
            }
            if w.m(id).cmd == 1 {
                let partner = pi32(w, id, pv::PARTNER);
                if let Some(p) = usize::try_from(partner).ok().filter(|&p| w.table.mobys.get(p).is_some_and(|m| m.state == 1)) { w.mm(p).cmd = 1; }
                let t = w.ticks(10);
                set_pi16(w, id, pv::TIMER, t as i16);
                let slot = pi32(w, id, pv::SLOT);
                if slot != -1 && w.sound_owner(slot) == Some(id) { w.release_sound(slot, id); }
                set_pi32(w, id, pv::SLOT, -1);
                let m = w.mm(id);
                m.state = 2;
                m.update_dist = 0xff;
            }
        }
        2 => {
            if dec_timer_pvar_s16(w, id, pv::TIMER) == 0 { return; }
            if let Some(n) = next(w, id) {
                let m = w.mm(n);
                m.cmd = 1;
                m.update_dist = 0xff;
            }
            w.mm(id).state = 3;
        }
        3 => {
            let p = w.m(id).position;
            fx::beam_explosion(w, &BLAST, Some(id), p);
            w.delete_moby(id);
        }
        _ => {}
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::moby_runtime::{Moby, MobyTable};

    #[test]
    fn told_by_the_chain_it_passes_the_break_on_and_blows_up() {
        let anchor = |cmd: u8| {
            let mut m = Moby { o_class: 1172, cmd, pvars: vec![0; 0x80], ..Moby::default() };
            crate::moby_update::services::pvar::set_i16(&mut m.pvars, pv::FLAG, -1);
            crate::moby_update::services::pvar::set_i32(&mut m.pvars, pv::NEXT, 2);
            crate::moby_update::services::pvar::set_i32(&mut m.pvars, pv::PARTNER, 1);
            m
        };
        let link = Moby { o_class: 1181, state: 1, ..Moby::default() };
        let mut t = MobyTable::new(vec![anchor(0), anchor(0), link], 8);
        t.mobys[1].state = 1;
        let hero = crate::hero::Hero::new();
        let mut rng = crate::rng::Rng::new();
        let classes = crate::moby_update::ClassTable::default();
        let mut svc = crate::moby_update::Services::new();
        let mut w = World::new(&mut t, &hero, &mut rng, &classes, &mut svc, 0);
        update(&mut w, 0);
        assert_eq!((w.m(0).state, pi32(&w, 0, pv::SLOT)), (1, -1));
        w.mm(0).cmd = 1;
        update(&mut w, 0);
        assert_eq!((w.m(0).state, w.m(1).cmd), (2, 1), "the partner told");
        for _ in 0..10 { update(&mut w, 0); }
        assert_eq!((w.m(0).state, w.m(2).cmd, w.m(2).update_dist), (3, 1, 0xff), "the next link told");
        update(&mut w, 0);
        assert!(w.m(0).state >= 0x80, "deleted after its explosion");
        assert!(w.svc.fx.flashes > 0);
    }
}
