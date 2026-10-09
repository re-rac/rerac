//! **The Glove of Doom's canister** (class 230 = 0xe6; one update on every level: level01 `0x2de650`), its create
//! `0x2dde80`, release `0x2ddf58`, landing preview `0x2de0a8` (the decoy's `0x2d9760` over its own pvars, drawn with
//! the same reticle `0x2dda60`) and its surface pop `0x2ddce0`. It lets out the bots 186
//! ([`super::doom_bot`]). The glove's side is the shared glove update (`crate::hero::gloves`, item 20). docs/plan/
//! hero_gameplay.md §18.
//!
//! **States** (+0x20):
//! * **0** → 1 (the puff timer 4 ticks, the bot counter and the timer cleared).
//! * **1 held** (the glove places it every tick; hidden in first person): grows toward a quarter of the class scale
//!   (5 % of it a tick); every 4 ticks a type-32 glow (`0x283d88`, not in first person). Deleted once the hand no
//!   longer holds the Glove of Doom (or Ratchet is not on foot).
//! * **2 flying**: grows to half the class scale; a type-32 glow every 4 ticks; gravity 9·dt²; the move tested by a
//!   sphere (0.3, at the next point 0.2 up, flags 4, Ratchet ignored) and then a line from its bottom (0.15 of its
//!   size under it) to the next bottom (flags 0): nothing → it moves on. A hit: it goes to the sphere's pushed centre
//!   (0.2 down) or the line's point (the bottom offset back up), the velocity is reflected off the face ×0.5, and
//!   slower than 0.01 a tick it **opens** (state 3) — on the world or a moby's mesh at once, on a moby's primitive after
//!   240 slow ticks; then it moves by the velocity and a surface of kinds 0, 1, 3, 8, 0xb, 0xc, 0xd (water, lava…)
//!   pops it (`0x2ddce0`: 20 type-5 puffs, state 4).
//! * **3 open**: every 8 ticks one bot (`0x2d5a70`), at most 8 alive: the bots leave at the quarter turns
//!   `(cos, sin)(k·π/2)·0.05` and 0.02 up from it, with that as their velocity; the fourth one → 4 (15 ticks later
//!   when 8 are alive). A glow every 4 ticks while fewer than two are out.
//! * **4 fading**: alpha −4 and scale ×0.92 a tick; deleted at alpha 0.
//! * Out of the world box [2, 1021]³: deleted.
//!
//! **Pvars** (0x40): +0x00 velocity (the glove's aim writes it; the create copies the hand point into it), +0x10 /
//! +0x20 the preview's point and normal, +0x30 the owner (the glove: 1 here), +0x34 the glow timer, +0x38 s16 the slow
//! ticks / the bot timer (and the preview's budget in flight: 0), +0x3a s16 a new canister in the glove (the preview
//! stops), +0x3c the bots made.
//!
//! Native `f32`; the rand draws are the game's. **Inferred [L]**: the owner is "the hand holds the Glove of Doom"
//! (the glove is not a table moby); objects on moving platforms do not ride them (`0x275290` / `0x2752c0`, 0x13f64c:
//! never set in the port); the pop's type-5 puffs are records only (type 5: G-PRT-001); the gold canister
//! (0x13e534: half the class scale × 2, a sphere of 0.6) reads the mirrored gold table (`Weapons::gold`).

use crate::moby_runtime::{MobyId, MobyTable};
use crate::moby_update::creature::{self as c, fx, V};
use crate::moby_update::services::{pvar as p, World};
use crate::particles::type32;
use crate::ps2v::Pf;

pub const UPDATE_FN: u32 = 0x2de650;
pub const CLASS: i16 = 0xe6;
pub const CLASSES: [i16; 1] = [CLASS];
/// The Glove of Doom (item 20) and its hand class.
pub const GLOVE_OF_DOOM: i32 = 20;
pub const GLOVE_CLASS: i16 = 0xe5;

/// States (+0x20).
pub const HELD: u8 = 1;
pub const FLYING: u8 = 2;
pub const OPEN: u8 = 3;
pub const FADING: u8 = 4;

/// Pvar offsets.
pub mod pv {
    pub const VEL: usize = 0x00;
    pub const POINT: usize = 0x10;
    pub const NORMAL: usize = 0x20;
    pub const OWNER: usize = 0x30;
    pub const PUFF_T: usize = 0x34;
    pub const COUNT: usize = 0x38;
    pub const NEW_HELD: usize = 0x3a;
    pub const BOTS: usize = 0x3c;
    pub const SIZE: usize = 0x40;
}

/// At most this many bots alive for a canister to let one out.
pub const MAX_BOTS: usize = 8;

const DT2: f32 = c::DT2;

/// The velocity the glove's aim solved (`0x2dcf88` into +0x00).
pub fn set_velocity(m: &mut crate::moby_runtime::Moby, v: [f32; 3]) {
    if m.pvars.len() < pv::SIZE { m.pvars.resize(pv::SIZE, 0); }
    p::set_v4f(&mut m.pvars, pv::VEL, [v[0], v[1], v[2], 0.0]);
}

/// `0x2dde80(glove, point, point)`: the new canister (the caller's `CreateMoby(0xe6)`): update / draw distances 0xff,
/// visible, state 0 at the hand point, the velocity word = the point (the aim writes it before its next update), the
/// owner, the counter and the glow timer 0, a hundredth of the class scale; hidden in first person or with the hand
/// hidden.
pub fn init_held(table: &mut MobyTable, id: MobyId, point: [f32; 3], hidden: bool) {
    let m = &mut table.mobys[id];
    m.draw_dist = 0xff;
    m.update_dist = 0xff;
    m.visible = 1;
    m.state = 0;
    m.position = [point[0], point[1], point[2], m.position[3]];
    if m.pvars.len() < pv::SIZE { m.pvars.resize(pv::SIZE, 0); }
    let pvs = &mut m.pvars;
    p::set_v4f(pvs, pv::VEL, [point[0], point[1], point[2], 0.0]);
    p::set_i32(pvs, pv::OWNER, 1);
    p::set_i16(pvs, pv::COUNT, 0);
    p::set_i32(pvs, pv::PUFF_T, 0);
    m.scale *= 0.01;
    if hidden { m.mode |= 0x41; }
}

/// `0x2ddf58(point, canister, vel)`: the release from the hand point, the velocity kept; the line from Ratchet at the
/// point's height to it (flags 0, the world and the mobys, Ratchet ignored: `hit`, its point and raw normal) moves it
/// onto the wall, `9·dt² − 0.1·dt²` out along the face, the velocity reflected off it ×0.6. State 2 either way.
pub fn release(m: &mut crate::moby_runtime::Moby, point: [f32; 3], hit: Option<([f32; 3], [f32; 3])>) {
    m.position = [point[0], point[1], point[2], m.position[3]];
    if let Some((hp, n)) = hit {
        let n4 = [n[0], n[1], n[2], 0.0];
        let off = c::set_len3(n4, DT2 * 9.0 - DT2 * 0.1);
        m.position = [hp[0] + off[0], hp[1] + off[1], hp[2] + off[2], m.position[3]];
        let v = super::decoy::reflect(p::v4f(&m.pvars, pv::VEL), n4);
        p::set_v4f(&mut m.pvars, pv::VEL, c::scale(v, 0.6));
    }
    m.state = FLYING;
}

/// Whether the hand holds the Glove of Doom (`0x1403e0`'s class is 0xe5).
fn glove_in_hand(w: &World) -> bool { w.hero.items.slot.item.as_ref().is_some_and(|m| m.o_class == GLOVE_CLASS) }

fn in_box(q: V) -> bool { (0..3).all(|k| (2.0..=1021.0).contains(&q[k])) }

fn pv4(v: V) -> [Pf; 4] { v.map(Pf::f) }

/// `0x2de650`.
pub fn update(w: &mut World, id: MobyId) {
    step(w, id);
    // The type-32 glows follow the canister (their update reads its position and scale): the port's anchor.
    let m = w.m(id);
    let (alive, q, s) = (!m.is_deleted(), [m.position[0], m.position[1], m.position[2]], m.scale);
    if let Some(ps) = w.particles.as_deref_mut() {
        if alive {
            ps.anchors.insert(id, q);
            ps.anchor_scales.insert(id, s);
        } else {
            ps.anchors.remove(&id);
            ps.anchor_scales.remove(&id);
        }
    }
}

fn step(w: &mut World, id: MobyId) {
    if w.m(id).pvars.len() < pv::SIZE { w.mm(id).pvars.resize(pv::SIZE, 0); }
    if !in_box(c::pos(w, id)) {
        w.delete_moby(id);
        return;
    }
    {
        let fp = w.hero.f13f5 != 0;
        let m = w.mm(id);
        if fp && m.state <= 1 { m.mode |= 0x41; } else { m.mode &= !0x41; }
    }
    // The landing preview (the owner is the glove in hand, no new canister in it, a velocity); never draws in the game
    // (hero_gameplay.md §13: the first update's "flying" branch marks the canister in the glove as a new one).
    if glove_in_hand(w) && c::pi16(w, id, pv::NEW_HELD) == 0 && 0.0 < c::len3(c::pv4(w, id, pv::VEL)) { preview(w, id); }
    match w.m(id).state {
        0 => {
            w.mm(id).state = HELD;
            let t4 = w.ticks(4);
            c::set_pi32(w, id, pv::PUFF_T, t4);
            c::set_pi32(w, id, pv::BOTS, 0);
            c::set_pi16(w, id, pv::COUNT, 0);
        }
        HELD => {
            c::set_pi16(w, id, pv::COUNT, 0);
            let cs = w.class_scale(w.m(id).o_class).to_f32();
            if w.m(id).scale < cs * 0.25 { w.mm(id).scale += cs * 0.05; }
            if c::dec_timer_pvar_i32(w, id, pv::PUFF_T) != 0 {
                let t4 = w.ticks(4);
                c::set_pi32(w, id, pv::PUFF_T, t4);
                if w.hero.f13f5 == 0 { glow(w, id); }
            }
            if w.body() == 0 && glove_in_hand(w) { return; }
            w.delete_moby(id);
        }
        FLYING => fly(w, id),
        OPEN => open(w, id),
        FADING => {
            let m = w.mm(id);
            m.alpha = m.alpha.wrapping_sub(4);
            m.scale *= 0.92;
            if m.alpha == 0 { w.delete_moby(id); }
        }
        _ => w.mm(id).state = 0,
    }
}

/// `0x283d88(canister)`: a type-32 glow at the canister (two draws with a record; the same draws without a particle
/// system).
fn glow(w: &mut World, id: MobyId) {
    *w.svc.fx.part_spawns.entry(type32::TYPE).or_default() += 1;
    let at = w.m(id).position;
    match w.particles.as_deref_mut() {
        Some(ps) => {
            if type32::spawn(ps, w.rng, id, at).is_none() { w.svc.fx.part_failed += 1; }
        }
        None => {
            w.rng.randf(0.0, 1.0);
            w.rng.randf(50000.0, 50000.0);
        }
    }
}

/// State 2 (module doc).
fn fly(w: &mut World, id: MobyId) {
    // 0x13e534 + 1: the gold canister grows to the full class scale and tests a sphere of 0.6.
    let gs = w.hero.weapons.gold.get(20).copied().unwrap_or(0) as f32 + 1.0;
    let cs = w.class_scale(w.m(id).o_class).to_f32();
    if w.m(id).scale < cs * 0.5 * gs { w.mm(id).scale += cs * 0.05 * gs; }
    if c::dec_timer_pvar_i32(w, id, pv::PUFF_T) != 0 {
        let t4 = w.ticks(4);
        c::set_pi32(w, id, pv::PUFF_T, t4);
        glow(w, id);
    }
    let pos = c::pos(w, id);
    let mut vel = c::pv4(w, id, pv::VEL);
    vel[2] -= DT2 * 9.0;
    let next = c::add(pos, vel);
    let bottom = 0.15 * w.m(id).scale / cs;
    let from = [pos[0], pos[1], pos[2] - bottom, pos[3]];
    let a = [next[0], next[1], next[2] - bottom, next[3]];
    let b = [next[0], next[1], next[2] + gs * 0.2, next[3]];
    let hero = w.hero_moby;
    let hit = match w.coll_sphere(pv4(b), Pf::f(gs * 0.3), 4, hero) {
        Some(o) => {
            let q = o.pushed_centre.unwrap_or([b[0], b[1], b[2]]);
            c::set_pos(w, id, [q[0], q[1], q[2] - gs * 0.2, pos[3]]);
            o
        }
        None => match w.coll_line(pv4(from), pv4(a), 0, hero) {
            None => {
                c::set_pos(w, id, next);
                c::set_pv4(w, id, pv::VEL, vel);
                return;
            }
            Some(o) => {
                let off = 0.15 * w.m(id).scale / cs;
                c::set_pos(w, id, [o.point[0], o.point[1], o.point[2] + off, pos[3]]);
                o
            }
        },
    };
    let n = [hit.normal[0], hit.normal[1], hit.normal[2], 0.0];
    vel = c::scale(super::decoy::reflect(vel, n), 0.5);
    if c::len3(vel) < 0.01 {
        let k = c::pi16(w, id, pv::COUNT).wrapping_add(1);
        c::set_pi16(w, id, pv::COUNT, k);
        if hit.moby.is_none() || 0 < hit.kind || (w.ticks(0xf0) as i16) < k {
            w.mm(id).state = OPEN;
            c::set_pi16(w, id, pv::COUNT, 0);
        }
    }
    c::set_pv4(w, id, pv::VEL, vel);
    let q = c::add(c::pos(w, id), vel);
    c::set_pos(w, id, q);
    pop(w, id, hit.surface_id());
}

/// `0x2ddce0`: on a surface of kind 0, 1, 3, 8, 0xb, 0xc or 0xd (the last query's) twenty type-5 puffs about it
/// (3 jitter draws ±0.3, 3 colour draws, a size 100000..800000, a life `ticks(rand % 40 + 10)`) and state 4.
fn pop(w: &mut World, id: MobyId, surface: i32) {
    if !matches!(surface, 0 | 1 | 3 | 8 | 0xb | 0xc | 0xd) { return; }
    for _ in 0..20 {
        let mut at = w.m(id).position;
        for x in &mut at[..3] { *x += w.rng.randf(f32::from_bits(0xbe99_999a), f32::from_bits(0x3e99_999a)); }
        let (r, g, b) = (w.rng.rand(), w.rng.rand(), w.rng.rand());
        let grow = w.rng.randf(f32::from_bits(0x47c3_5000), f32::from_bits(0x4943_5000));
        let n = w.rng.rand();
        let life = w.ticks(n % 0x28 + 10);
        fx::part05(w, grow, 0.0, at, [(r + 0x30) as u32 & 0x3f, (g + 0x20) as u32 & 0x3f, b as u32 & 0x2f], life);
    }
    w.mm(id).state = FADING;
}

/// State 3 (module doc): the bots come out.
fn open(w: &mut World, id: MobyId) {
    if c::dec_timer_pvar_s16(w, id, pv::COUNT) != 0 {
        let alive = w.table.mobys.iter().take_while(|m| m.state != crate::moby_runtime::state::END).filter(|m| m.o_class == super::doom_bot::CLASS && m.state & 0xf0 != 0xf0).count();
        if alive < MAX_BOTS {
            let k = c::pi32(w, id, pv::BOTS);
            let a = k as f32 * std::f32::consts::TAU * 0.25;
            let v = [a.cos() * 0.05, a.sin() * 0.05, 0.02, 0.0];
            let at = c::add(v, c::pos(w, id));
            let owner = c::pi32(w, id, pv::OWNER);
            super::doom_bot::create(w, owner, at, v);
            let t8 = w.ticks(8) as i16;
            c::set_pi16(w, id, pv::COUNT, t8);
            c::set_pi32(w, id, pv::BOTS, k + 1);
            if 3 < k + 1 { w.mm(id).state = FADING; }
        } else {
            let t15 = w.ticks(0xf) as i16;
            c::set_pi16(w, id, pv::COUNT, t15);
        }
    }
    if c::pi32(w, id, pv::BOTS) < 2 && c::dec_timer_pvar_i32(w, id, pv::PUFF_T) != 0 {
        let t4 = w.ticks(4);
        c::set_pi32(w, id, pv::PUFF_T, t4);
        glow(w, id);
    }
}

/// `0x2de0a8(pvars, canister, 1)`: the landing preview (`super::decoy::short_glove_preview`): held, for 300 ticks from
/// the launch point; flying, for the +0x38 ticks (0: no landing) after marking a new canister in the glove (+0x3a).
fn preview(w: &mut World, id: MobyId) -> bool {
    if w.hero.state == 0x72 || w.svc.game_mode != 0 { return false; }
    let held = w.m(id).state == HELD;
    let v = c::pv4(w, id, pv::VEL);
    let budget = if held {
        w.ticks(300)
    } else {
        if w.hero.weapons.glove.held.is_some() { c::set_pi16(w, id, pv::NEW_HELD, 1); }
        c::pi16(w, id, pv::COUNT) as i32
    };
    let Some((point, normal)) = super::decoy::short_glove_preview(w, id, held, [v[0], v[1], v[2]], budget) else { return false };
    c::set_pv4(w, id, pv::POINT, [point[0], point[1], point[2], 0.0]);
    c::set_pv4(w, id, pv::NORMAL, [normal[0], normal[1], normal[2], 0.0]);
    true
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::moby_update::classes::bomb_water::tests::{pool, Bench};
    use crate::moby_update::classes::decoy::tests::floor;
    use crate::moby_update::classes::doom_bot;

    fn thrown(b: &mut Bench, at: [f32; 3], vel: [f32; 3]) -> MobyId {
        let c = b.create(CLASS);
        init_held(&mut b.table, c, at, false);
        set_velocity(&mut b.table.mobys[c], vel);
        release(&mut b.table.mobys[c], at, None);
        c
    }

    /// `0x2de650` states 2 → 3 → 4: a canister rolled onto a floor (surface 2) glows every 4 ticks in flight (type 32,
    /// its anchor written), stops and opens, lets out one bot every 8 ticks at the quarter turns (4 in all, each thrown
    /// out at 0.05 and 0.02 up), then fades (alpha −4 a tick) and is deleted (its anchor removed).
    #[test]
    fn opens_and_lets_out_four_bots() {
        let mut b = Bench::new(floor(8.0, 0x22), &[CLASS, doom_bot::CLASS], [40.0, 40.0, 8.0]);
        let c = thrown(&mut b, [20.0, 20.0, 8.6], [0.03, 0.0, 0.0]);
        let mut opened = None;
        let mut made: Vec<(u64, usize)> = Vec::new();
        let mut glows_in_flight = 0;
        for _ in 0..400 {
            let g0 = b.parts(type32::TYPE);
            b.tick(None);
            let m = &b.table.mobys[c];
            if m.state == FLYING { glows_in_flight += b.parts(type32::TYPE) - g0; }
            if !m.is_deleted() { assert!(b.parts.anchors.contains_key(&c) && b.parts.anchor_scales.contains_key(&c), "anchor kept"); }
            if m.state == OPEN && opened.is_none() { opened = Some(b.counter); }
            for (i, x) in b.table.mobys.iter().enumerate() {
                if x.o_class == doom_bot::CLASS && x.state < 0xfd && !made.iter().any(|m| m.1 == i) { made.push((b.counter, i)); }
            }
            if b.table.mobys[c].is_deleted() { break; }
        }
        let o = opened.expect("opened");
        assert!(glows_in_flight > 0, "glows in flight");
        assert_eq!(made.len(), 4, "four bots: {made:?}");
        let ticks: Vec<u64> = made.iter().map(|m| m.0).collect();
        assert_eq!(ticks, [o + 1, o + 9, o + 17, o + 25], "one every 8 ticks from the update after the opening");
        assert!(b.table.mobys[c].is_deleted(), "faded away");
        assert!(!b.parts.anchors.contains_key(&c), "anchor removed");
    }

    /// `0x2ddce0`: a canister coming down on a pop surface (the pool's floor, surface 1) pops: 20 type-5 records (live, `particles::type05`) and
    /// state 4 (it fades), no bots.
    #[test]
    fn pops_on_a_pop_surface() {
        let mut b = Bench::new(pool(8.0, 4.0), &[CLASS, doom_bot::CLASS], [40.0, 40.0, 9.0]);
        let c = thrown(&mut b, [20.0, 20.0, 9.0], [0.02, 0.0, 0.0]);
        let p5 = b.parts(5);
        for _ in 0..200 {
            b.tick(None);
            if b.table.mobys[c].state == FADING { break; }
        }
        assert_eq!(b.table.mobys[c].state, FADING, "popped");
        assert_eq!(b.parts(5) - p5, 20, "twenty type-5 puffs");
        // The puffs live their game lives (`particles::type05`): 20 records, faded and gone within 50 updates, none
        // killed unported.
        assert_eq!(b.parts.live_by_type()[5], 20);
        let mut rng = crate::rng::Rng::new();
        for _ in 0..50 { b.parts.update_parts(&mut rng); }
        assert_eq!((b.parts.live_by_type()[5], b.parts.stats.unported_kills[5]), (0, 0));
        for _ in 0..40 { b.tick(None); }
        assert!(b.table.mobys[c].is_deleted() && b.alive(doom_bot::CLASS) == 0, "gone without bots");
    }

    /// State 3 with 8 bots alive: no bot, the timer set to 15 ticks.
    #[test]
    fn waits_while_eight_bots_are_out() {
        let mut b = Bench::new(floor(8.0, 0x22), &[CLASS, doom_bot::CLASS], [40.0, 40.0, 8.0]);
        for k in 0..8 {
            let x = b.create(doom_bot::CLASS);
            b.table.mobys[x].position = [4.0 + 4.0 * k as f32, 40.0, 8.5, 1.0];
        }
        let c = thrown(&mut b, [20.0, 20.0, 8.0], [0.0; 3]);
        b.table.mobys[c].state = OPEN;
        b.tick(None);
        assert_eq!(b.alive(doom_bot::CLASS), 8, "no ninth bot");
        assert_eq!(p::i16(&b.table.mobys[c].pvars, pv::COUNT), 15, "tries again in 15 ticks");
        assert_eq!(p::i32(&b.table.mobys[c].pvars, pv::BOTS), 0);
    }
}
