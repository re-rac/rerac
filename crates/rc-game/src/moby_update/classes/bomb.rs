//! **The Bomb Glove's bomb** (class 121, level01 `0x2c3300`) and **its fireballs** (class 122, `0x2c4d88`), plus
//! the glove-side writers of a bomb's pvars: `0x2c2640` (the bomb created in the glove, [`create_held`]) and
//! `0x2c27a8` (its release, [`release`]). The glove's own update is `crate::hero::weapons` (hand item 10).
//!
//! **The bomb** (moby +0x20): **0 held** — the glove places it every tick (collision off, +0x98 = 1), a pulsing
//! glow; hidden while the first-person camera is up (0x1413f5, the hero's `f13f5`); deleted when the hand no
//! longer holds the glove. **1 flying** — the glow fades from red to grey over its 30-tick timer (+0x6a), it spins
//! (x 120°/s, y 450°/s), moves by its velocity (+0x00) under gravity 11 u/s² (1.1 under water), and tests its path
//! (`CollLine_Fix(old, new, 0x10, bomb, tmpl)`: template flags 0x830000 (| 1 after its first 10 ticks), damage 2,
//! so a crate on the path gets its hit): a hit explodes it — except Ratchet or the glove in its first 10 ticks;
//! water (surface 0) while falling: it sinks at 1.5 u/s until its 300-tick fuse (+0x54) runs out; the fuse ends in
//! the air too. **The explosion** (moby +0xbc = 1): the sphere 0.5 lists the mobys (`coll_sphere_mobys`, flags
//! 0x10) and hits them (`0x26f8f8`: damage 2, flags 0x830000) → state **2**; the fireballs (10 low ones, 4 + 1 high,
//! the last toward the camera), the type-11 smoke rings (1..4 by the camera distance), the flashes (class 1192:
//! two more when the camera is farther than 9), the class sound 0, the camera shake along up `0.4 − 0.0175·d`
//! (0.05 beyond 20) for 25 ticks. **2 exploding**: hidden, a sphere growing from 0.5 to 2.5 over 15 ticks hits
//! every moby it touches (damage 2, 0x830000), then the bomb is deleted.
//!
//! **Fireball** (class 122): moves by its velocity, gravity 14.6 u/s², spins by its two rates, shrinks to 0 over
//! the last quarter of its life (random 60..120 / 60..90 ticks), deleted when it runs out or leaves the positive
//! octant.
//!
//! The flight's trail (type-2 blobs every 8th tick, [`trail`]), the high fireballs' smoke (a type-4 puff a tick,
//! [`smoke`]) and the explosion light (class 0x27f, template 0x20a930) are ported with their draws at the game's point,
//! so a dry explosion leaves the one `rand` stream where the game leaves it (`tests/explosion_novalis.rs`).
//!
//! The water branch (entry splash, ripple and drops, the sinking bubbles, the deep burst and its sound, the
//! shallow-water scorch and sparks) is [`super::bomb_water`], with the game's draws.
//!
//! **One blast, three callers** ([`blast`], [`BlastRow`], [`spawn_fireball`] / [`FireballRow`]; docs/plan/explosions.md
//! §B): the fireballs / rings / flashes of the explosion are the same compiled code in this update, in the Suck
//! Cannon's burst `0x304798` (`creature::react::burst`) and in Gemlik's tanks `0x3073c8` (`units::explosive_tank`),
//! with other constants (size k, fireball divisor, colour tables, fireball class).
//!
//! Native `f32`. **Not ported** (counted in `Services::fx.unported`): the aim-preview `0x2c2be0`, the pass-through
//! classes of the hit test (update `0x160770`), the gold glove (0x13e52a: k = 2, the colour shifts `0x270fa8` /
//! `0x270f48` [`blast`] takes, the gold smoke of the fireballs, class sound 1; not mirrored, G-WPN gold).

use crate::moby_runtime::MobyId;
use crate::moby_update::services::{self as sv, pvar as p, HitTemplate, World};
use crate::ps2v::Pf;
use std::f32::consts::PI;

pub const UPDATE_FN: u32 = 0x2c3300;
pub const FIREBALL_UPDATE_FN: u32 = 0x2c4d88;
/// The bomb (`CreateMoby(0x79)`) and its fireball (`0x7a`).
pub const CLASSES: [i16; 1] = [121];
pub const FIREBALL_CLASSES: [i16; 1] = [122];
pub const BOMB_CLASS: i16 = 121;
pub const FIREBALL_CLASS: i16 = 122;
/// The explosion's flash class (`0x309a68`: `CreateMoby(0x4a8)`, the shared `FlashUpdate`).
pub const FLASH_CLASS: i16 = 1192;
/// The glove (hand item 10's class).
pub const GLOVE_CLASS: i16 = 0xc0;

/// Bomb states (+0x20).
pub const HELD: u8 = 0;
pub const FLYING: u8 = 1;
pub const EXPLODING: u8 = 2;

/// Pvar offsets of the bomb (the game's layout; the owner +0x50 holds 1 for "the glove" instead of its pointer).
pub mod pv {
    pub const VEL: usize = 0x00;
    pub const OWNER: usize = 0x50;
    pub const LIFE: usize = 0x54;
    pub const FLAG56: usize = 0x56;
    pub const SPIN_X: usize = 0x58;
    pub const SPIN_Y: usize = 0x5c;
    pub const TARGET: usize = 0x60;
    pub const WATER: usize = 0x68;
    pub const FLASH: usize = 0x6a;
    pub const YAW: usize = 0x6c;
}

fn dt() -> f32 { sv::DT.to_f32() }
fn dt2() -> f32 { sv::DT2.to_f32() }
fn wrap(a: f32) -> f32 {
    let mut x = a;
    if x >= PI { x -= 2.0 * PI; }
    if x < -PI { x += 2.0 * PI; }
    x
}
fn sub3(a: [f32; 4], b: [f32; 4]) -> [f32; 4] { [a[0] - b[0], a[1] - b[1], a[2] - b[2], 0.0] }
fn add3(a: [f32; 4], b: [f32; 4]) -> [f32; 4] { [a[0] + b[0], a[1] + b[1], a[2] + b[2], a[3]] }
fn len3(a: [f32; 4]) -> f32 { (a[0] * a[0] + a[1] * a[1] + a[2] * a[2]).sqrt() }
fn setlen(a: [f32; 4], l: f32) -> [f32; 4] {
    let n = len3(a);
    if n == 0.0 { [0.0; 4] } else { [a[0] * l / n, a[1] * l / n, a[2] * l / n, 0.0] }
}
fn dot3(a: [f32; 4], b: [f32; 4]) -> f32 { a[0] * b[0] + a[1] * b[1] + a[2] * b[2] }
fn pv4(a: [f32; 4]) -> [Pf; 4] { a.map(Pf::f) }

/// `FUN_002c2640(glove, point)`: a bomb in the glove at `point` (the table's `CreateMoby(0x79)` is the caller's;
/// this writes the new moby): update and draw distances 0xff, visible, state 0, owner the glove, the timers 0, the
/// yaw of Ratchet's facing, a random rotation (3 draws `randf(−π, π)`) and spin rates (2 draws `randf(60°, 180°)·dt`).
pub fn init_held(m: &mut crate::moby_runtime::Moby, rng: &mut crate::rng::Rng, point: [f32; 3], facing_yaw: f32) {
    let r = [rng.randf(-PI, PI), rng.randf(-PI, PI), rng.randf(-PI, PI)];
    m.update_dist = 0xff;
    m.draw_dist = 0xff;
    m.visible = 1;
    m.state = HELD;
    if m.pvars.len() < 0x80 { m.pvars.resize(0x80, 0); }
    let pv = &mut m.pvars;
    p::set_i32(pv, pv::OWNER, 1);
    p::set_i16(pv, pv::FLAG56, 0);
    p::set_i16(pv, pv::FLASH, 0);
    p::set_i32(pv, pv::TARGET, 0);
    p::set_ff(pv, pv::YAW, facing_yaw);
    p::set_ff(pv, 0x64, 0.0);
    m.position = [point[0], point[1], point[2], m.position[3]];
    p::set_v4f(pv, pv::VEL, [0.0; 4]);
    m.rotation = [r[0], r[1], r[2], 0.0];
    let a = rng.randf(dt() * (PI / 3.0), dt() * PI);
    let b = rng.randf(dt() * (PI / 3.0), dt() * PI);
    p::set_ff(pv, pv::SPIN_X, a);
    p::set_ff(pv, pv::SPIN_Y, b);
}

/// `FUN_002c27a8(point, bomb)`: the release — the flash timer 30, state 1, the velocity the glove's aim left in the
/// pvars (plus Ratchet's velocity while grinding: `grind_vel`), the 300-tick fuse, at `point`; a wall between
/// Ratchet (`hero_pos`, at the point's height) and the point explodes it there at once. Returns the line to test
/// `(from, to)`: the caller runs it (it needs the collision) and passes the hit point to [`release_hit`].
pub fn release(m: &mut crate::moby_runtime::Moby, point: [f32; 3], grind_vel: Option<[f32; 3]>) {
    if m.pvars.len() < 0x80 { m.pvars.resize(0x80, 0); }
    let pv = &mut m.pvars;
    p::set_i16(pv, pv::FLASH, 30);
    m.state = FLYING;
    if let Some(g) = grind_vel {
        let v = p::v4f(pv, pv::VEL);
        p::set_v4f(pv, pv::VEL, [v[0] + g[0], v[1] + g[1], v[2] + g[2], v[3]]);
    }
    p::set_i16(pv, pv::LIFE, 300);
    m.position = [point[0], point[1], point[2], m.position[3]];
}

/// The release's wall test hit: explode at `hit` (+0xbc = 1).
pub fn release_hit(m: &mut crate::moby_runtime::Moby, hit: [f32; 3]) {
    m.cmd = 1;
    m.position = [hit[0], hit[1], hit[2], m.position[3]];
}

/// The bomb's template: `dir` (xy of the unit velocity, z 1), flags 0x830000 (| 1 once 10 ticks old), damage 2,
/// +0x18 = 2 / 1, +0x1a = the class.
fn template(id: MobyId, vel: [f32; 4], life: i16) -> HitTemplate {
    let n = setlen(vel, 1.0);
    let mut flags = 0x83_0000;
    if (life as i32) < 300 - 10 { flags |= 1; }
    HitTemplate { dir: [Pf::f(n[0]), Pf::f(n[1]), Pf::ONE, Pf::b(0x45af_df66)], attacker: Some(id), flags, b18: 2, b19: 1, h1a: BOMB_CLASS as u16, damage: Pf::f(2.0), w20: 1 }
}

/// `0x2c3300`.
pub fn update(w: &mut World, id: MobyId) {
    if w.m(id).pvars.len() < 0x80 { w.mm(id).pvars.resize(0x80, 0); }
    let hand_glove = w.hero.items.slot.item.as_ref().is_some_and(|m| m.o_class == GLOVE_CLASS);
    let st = w.m(id).state;
    if (!hand_glove && st == HELD) || (w.body() != 0 && st == HELD) {
        w.delete_moby(id);
        return;
    }
    // Hidden while held in first person (0x1413f5), shown otherwise.
    {
        let fp = w.hero.f13f5 != 0;
        let m = w.mm(id);
        if fp && m.state == HELD { m.mode |= 0x41; } else { m.mode &= !0x41; }
    }
    // The glove's bomb (owner +0x50 = the glove) with a velocity and +0x56 clear: the landing preview and its reticle
    // (`0x2c2be0(pvars, bomb, 1)`; the moving-platform variants 0 / 2 need 0x13f64c, which the port never sets).
    {
        let pv = &w.m(id).pvars;
        let v = p::v4f(pv, pv::VEL);
        if p::i32(pv, pv::OWNER) == 1 && p::i16(pv, pv::FLAG56) == 0 && 0.0 < len3(v) { aim_preview(w, id); }
    }
    match st {
        FLYING => fly(w, id),
        EXPLODING => exploding(w, id),
        _ => {
            // Held: the glow pulse (+0x6a counts up).
            let m = w.mm(id);
            let c = p::i16(&m.pvars, pv::FLASH).wrapping_add(1);
            p::set_i16(&mut m.pvars, pv::FLASH, c);
            let a = sv::normalize_angle(Pf::f(c as f32 * 0.068)).to_f32();
            let v = ((a.sin() * 96.0) as i32 + 0x9f) as u32 & 0xff;
            m.glow = 0xff00_0000 | v << 16 | v << 8 | v;
        }
    }
}

/// `0x2c2be0(pvars, bomb, 1)`: the landing preview (`crate::targeting::arc_landing`) from the launch point (held:
/// the velocity the glove's aim left, plus Ratchet's displacement while grinding, 300 ticks) or the bomb (flying: its
/// fuse; a new bomb in the glove sets +0x56, which ends the flying bomb's previews); a moby it snaps to is tracked in
/// +0x30 (its position) / +0x40 (its motion since) / +0x60 (the moby) / +0x64 (ticks on it), and let go once dead,
/// untargetable or past 30 ticks without a snap; the point (+0x10, pulled 5 % toward the camera) and the normal
/// (+0x20) are what the reticle draw (`0x2c23c0`, list 1: `Services::reticles`) reads. Returns whether it drew.
fn aim_preview(w: &mut World, id: MobyId) -> bool {
    use crate::targeting::{self as tg, ArcStart, LineHit};
    if w.hero.state == 0x72 || w.svc.game_mode != 0 { return false; }
    let v4 = |a: [f32; 4]| [a[0], a[1], a[2]];
    let held = w.m(id).state == HELD;
    let vel = v4(p::v4f(&w.m(id).pvars, pv::VEL));
    let (from, vel, budget) = if held {
        let h = w.hero;
        let from = crate::hero::weapons::launch_point(crate::hero::physics::to_f32x3(h.pos), crate::hero::physics::to_f32x3(h.moby_rows[0]));
        let d = crate::hero::physics::to_f32x3(h.disp);
        let vel = if h.group == 0xf { [vel[0] + d[0], vel[1] + d[1], vel[2] + d[2]] } else { vel };
        (from, vel, 300)
    } else {
        if w.hero.weapons.glove.held.is_some() { p::set_i16(&mut w.mm(id).pvars, pv::FLAG56, 1); }
        (v4(w.m(id).position), vel, p::i16(&w.m(id).pvars, pv::LIFE) as i32)
    };
    let hero_pos = w.hero_moby.and_then(|h| w.table.mobys.get(h)).map_or(from, |m| [m.position[0], m.position[1], m.position[2]]);
    let start = ArcStart {
        from,
        vel,
        budget,
        gravity: dt2() * 11.0,
        first: [hero_pos[0], hero_pos[1], from[2]],
        first_ignore: w.hero_moby,
        ignore: Some(id),
        // Ratchet (the glove is not a table moby in the port).
        own: [w.hero_moby, None],
        gold: w.hero.weapons.gold.get(10).is_some_and(|&g| g != 0) && w.hero.state == 1,
        prims: tg::Prims::Snap { max_rise: Some(2.0) },
        first_test: true,
    };
    let landing = {
        let ww: &World = w;
        tg::arc_landing(
            &start,
            |a, b, ignore| {
                let h = ww.coll_line(pv4([a[0], a[1], a[2], 0.0]), pv4([b[0], b[1], b[2], 0.0]), 0x10, ignore)?;
                Some(LineHit { point: h.point, normal: h.normal, moby: h.moby, kind: h.kind, surface: h.surface_id() })
            },
            |m| { let q = ww.m(m).position; [q[0], q[1], q[2]] },
            |a, b| ww.coll_line(pv4([a[0], a[1], a[2], 0.0]), pv4([b[0], b[1], b[2], 0.0]), 2, None).map(|h| h.normal),
        )
    };
    let Some(l) = landing else { return false };
    // The lock on the snapped moby (+0x60 holds the moby + 1; the game's pointer).
    let locked = p::i32(&w.table.mobys[id].pvars, pv::TARGET);
    if let Some(m) = l.snapped {
        let mp = w.table.mobys[m].position;
        let pvars = &mut w.table.mobys[id].pvars;
        if locked != m as i32 + 1 {
            p::set_i32(pvars, 0x64, 0);
            p::set_i32(pvars, pv::TARGET, m as i32 + 1);
        } else {
            let c = p::i32(pvars, 0x64) + 1;
            p::set_i32(pvars, 0x64, c);
            let old = p::v4f(pvars, 0x30);
            p::set_v4f(pvars, 0x40, [mp[0] - old[0], mp[1] - old[1], mp[2] - old[2], mp[3] - old[3]]);
        }
        p::set_v4f(pvars, 0x30, mp);
    } else if locked != 0 {
        let t = (locked - 1) as usize;
        let gone = w.table.mobys.get(t).is_none_or(|m| m.state == 0xfe || m.state == 0xfd || m.mode & crate::moby_runtime::mode::TARGETABLE == 0);
        let pvars = &mut w.table.mobys[id].pvars;
        if gone || 30 < p::i32(pvars, 0x64) {
            p::set_i32(pvars, 0x64, 0);
            p::set_i32(pvars, pv::TARGET, 0);
        }
    }
    let cam = [w.camera[0].to_f32(), w.camera[1].to_f32(), w.camera[2].to_f32()];
    let point = tg::pull_toward(cam, l.point);
    let pvars = &mut w.table.mobys[id].pvars;
    p::set_v4f(pvars, 0x10, [point[0], point[1], point[2], 0.0]);
    p::set_v4f(pvars, 0x20, [l.normal[0], l.normal[1], l.normal[2], 0.0]);
    w.svc.reticles.register(w.counter, tg::Reticle { point, normal: l.normal, style: &tg::BOMB_RETICLE });
    true
}

/// State 1.
fn fly(w: &mut World, id: MobyId) {
    // The glow: (255, 64, 64) → (128, 128, 128) over the 30-tick timer.
    {
        let m = w.mm(id);
        let mut t = p::i16(&m.pvars, pv::FLASH);
        sv::fast_dec_timer_s16(&mut t);
        p::set_i16(&mut m.pvars, pv::FLASH, t);
        let k = t as f32 / 30.0;
        let r = ((128.0 - 255.0) * k + 255.0) as i32 as u32;
        let g = ((128.0 - 64.0) * k + 64.0) as i32 as u32;
        let b = ((128.0 - 64.0) * k + 64.0) as i32 as u32;
        m.glow = 0xff00_0000 | (b & 0xff) << 16 | (g & 0xff) << 8 | (r & 0xff);
    }
    if p::i16(&w.m(id).pvars, pv::WATER) == 0 && w.counter & 7 == 0 { trail(w, id); }
    let (old, vel, water, life);
    {
        let m = w.mm(id);
        m.rotation[0] = wrap(m.rotation[0] + dt() * 2.094_395_2);
        m.rotation[1] = wrap(m.rotation[1] + dt() * 7.853_981_5);
        old = m.position;
        let v = p::v4f(&m.pvars, pv::VEL);
        m.position = add3(m.position, v);
        let wt = p::i16(&m.pvars, pv::WATER);
        let mut v2 = v;
        if wt == 0 { v2[2] -= dt2() * 11.0; } else { v2[2] -= dt2() * 11.0 / 10.0; }
        p::set_v4f(&mut m.pvars, pv::VEL, v2);
        vel = v2;
        water = wt;
        life = p::i16(&m.pvars, pv::LIFE);
        if wt != 0 { p::set_i16(&mut m.pvars, pv::WATER, wt.wrapping_add(1)); }
    }
    if water != 0 { super::bomb_water::sink_bubble(w, id, vel); }
    let tmpl = template(id, vel, life);
    let pos = w.m(id).position;
    let hit = sv::line_hit_in(w.table, w.svc, w.classes, w.coll, pv4(old), pv4(pos), 0x10, Some(id), &tmpl);
    let mut drift = [0.0f32; 4];
    let mut normal = [0.0f32; 4];
    match hit {
        None => {
            let m = w.mm(id);
            let mut t = p::i16(&m.pvars, pv::LIFE);
            if sv::fast_dec_timer_s16(&mut t) != 0 {
                m.cmd = 1;
                drift = vel;
            }
            p::set_i16(&mut m.pvars, pv::LIFE, t);
        }
        Some(h) => {
            let water_face = h.moby.is_none() && h.surface_id() == 0;
            if water == 0 && water_face && vel[2] < 0.0 {
                // Into water: sink slowly; the ripple, splash and drops (`bomb_water::entry`).
                let m = w.mm(id);
                p::set_i16(&mut m.pvars, pv::WATER, 1);
                p::set_v4f(&mut m.pvars, pv::VEL, [0.0, 0.0, dt() * -1.5, 0.0]);
                super::bomb_water::entry(w, id, h.point[2]);
            } else if !(water == 0 && water_face) {
                let young = (life as i32) >= 300 - 10;
                let own = h.moby.is_some() && (h.moby == w.hero_moby);
                if !(own && young) {
                    let m = w.mm(id);
                    m.cmd = 1;
                    m.position = [h.point[0], h.point[1], h.point[2], m.position[3]];
                    let n = [h.normal[0], h.normal[1], h.normal[2], 0.0];
                    let nn = setlen(n, 1.0);
                    let refl = sub3(vel, [nn[0] * 2.0 * dot3(vel, nn), nn[1] * 2.0 * dot3(vel, nn), nn[2] * 2.0 * dot3(vel, nn), 0.0]);
                    drift = setlen(refl, dt() + dt());
                    normal = nn;
                }
            }
        }
    }
    if w.m(id).cmd != 0 { explode(w, id, drift, normal); }
}

/// The flight's trail (0x2c35c8..0x2c3788, every 8th tick while not in water): [`TRAIL_COUNT`] type-2 blobs
/// (`crate::particles::type02`) from the gp block 0x1613f0: velocity 1 = half the bomb's velocity plus `rand_vec(0,
/// dt)`, velocity 2 = 0 falling by 4·dt² per tick of phase B, sizes `randf(0.25, 0.15)` / `randf(0.15, 0.05)`, colours
/// tweened at `randf(0, 1)` between 0x804080ff / 0x8040ffff and 0x80104040 / 0x80002080, phases `1 + randf(0, 1)`,
/// `30·(1 + randf(±0.5))` and `10·(1 + randf(±0.5))` ticks, texture `def[25]`, additive. 10 draws per blob, plus the
/// spawner's one.
fn trail(w: &mut World, id: MobyId) {
    for _ in 0..TRAIL_COUNT {
        let pos = w.m(id).position;
        let vel = p::v4f(&w.m(id).pvars, pv::VEL);
        let rv = w.rng.rand_vec(0.0, TRAIL_SPREAD * dt());
        let mut v1 = [vel[0] * TRAIL_VEL + rv[0], vel[1] * TRAIL_VEL + rv[1], vel[2] * TRAIL_VEL + rv[2], 0.0];
        let mut v2 = [0.0f32; 4];
        v1[3] = w.rng.randf(0.25, 0.15);
        v2[3] = w.rng.randf(0.15, 0.05);
        let f = w.rng.randf(0.0, 1.0);
        let c1 = crate::particles::tween_color(f.to_bits(), 0x8040_80ff, 0x8040_ffff);
        let f = w.rng.randf(0.0, 1.0);
        let c2 = crate::particles::tween_color(f.to_bits(), 0x8010_4040, 0x8000_2080);
        let r = w.rng.randf(0.0, 1.0);
        let ta = w.svc.timing.scale(Pf::f(r + 1.0)).to_f32() as i32;
        let r = w.rng.randf(-0.5, 0.5);
        let tb = w.svc.timing.scale(Pf::f(30.0 * (r + 1.0))).to_f32() as i32;
        let r = w.rng.randf(-0.5, 0.5);
        let tc = w.svc.timing.scale(Pf::f(10.0 * (r + 1.0))).to_f32() as i32;
        v2[2] -= 4.0 * dt2() * tb as f32;
        let a = crate::particles::type02::Spawn { pos, v1, v2, c1, c2, t: [ta, tb, tc], def: 0x1_0019 };
        crate::moby_update::creature::fx::part02(w, &a);
    }
}

/// The trail's gp block (level01 0x1613f0..0x161430): blobs per spawn tick, velocity scale, spread (× dt).
const TRAIL_COUNT: usize = 1;
const TRAIL_VEL: f32 = 0.5;
const TRAIL_SPREAD: f32 = 1.0;

/// The fireballs' smoke colours (0x161438, `randi(4)`); the end colour is 0x4fff (alpha 0).
const SMOKE: [u32; 4] = [0x2fff_ffff, 0x2f00_ffff, 0x2f00_7fff, 0x2f00_4fff];

/// The smoke rings' colours (`0x20a980`, `0x20a998`: `randi(6)` each).
pub(crate) const RING_C1: [u32; 6] = crate::moby_update::creature::fx::SPARK_A;
pub(crate) const RING_C2: [u32; 6] = crate::moby_update::creature::fx::SPARK_B;

/// The explosion (+0xbc ≠ 0; `drift` = the reflected direction ·2·dt (or the velocity at the fuse's end),
/// `normal` = the unit normal of the face it hit).
fn explode(w: &mut World, id: MobyId, drift: [f32; 4], normal: [f32; 4]) {
    let k = 1.0f32;
    let drift = drift.map(|x| x * k);
    let pos = w.m(id).position;
    let cam = w.camera.map(|x| x.to_f32());
    let to_cam = sub3(cam, pos);
    let dcam = len3(to_cam);
    let water = p::i16(&w.m(id).pvars, pv::WATER);
    let dry = (water as i32) < w.ticks(20);
    if dry {
        // The sphere lists the mobys; 0x26f8f8 hits them (damage 2, flags 0x830000).
        let tmpl = HitTemplate { dir: [Pf::ZERO, Pf::ZERO, Pf::ONE, Pf::b(0x45af_df66)], attacker: Some(id), flags: 0x83_0000, b18: 2, b19: 1, h1a: BOMB_CLASS as u16, damage: Pf::f(2.0), w20: 1 };
        w.sphere_mobys(Pf::f(k * 0.5), pv4(pos), 0x10, Some(id), Some(&tmpl));
        w.mm(id).state = EXPLODING;
    }
    // The fireballs, rings and flashes (the explosion code the Suck Cannon's burst and Gemlik's tanks share).
    blast(w, &GLOVE_BLAST, id, pos, drift, normal, k, 1, dry, 0);
    if !dry { super::bomb_water::deep_burst(w, id, k); }
    p::set_ff(&mut w.mm(id).pvars, pv::VEL, 0.0);
    w.mm(id).cmd = 0;
    if dry {
        w.play_sound(0, 0, id);
    } else {
        w.play_sound_as(0x16, 0, id, 0);
    }
    if water != 0 { super::bomb_water::shallow_scorch(w, id); }
    // The camera shake along up (0x167260) for 25 ticks.
    let amp = if dcam < 20.0 { 0.4 - dcam * 0.0175 } else { f32::from_bits(0x3d4c_ccd0) };
    let t = w.ticks(25);
    w.shake_camera(crate::follow_camera::ShakeRequest { axis: crate::follow_camera::ShakeAxis::Up, amp, ticks: t });
    // The explosion light (class 0x27f, template 0x20a930).
    crate::moby_update::creature::fx::light_spawn(w, &crate::moby_update::creature::fx::LIGHT_BOMB, pos);
    if w.m(id).state != EXPLODING {
        w.delete_moby(id);
        return;
    }
    exploding(w, id);
}

/// A fireball spawner: `0x2c4c20` (class 122, every level) and its level13 copy `0x30c138` (class 1634, Gemlik's
/// tanks) are one code with these constants (docs/plan/explosions.md §B).
#[derive(Clone, Copy, Debug)]
pub struct FireballRow {
    pub class: i16,
    /// The alpha: `rand_range(lo, hi)` (drawn first), or 0x80 when `None`.
    pub alpha: Option<(i32, i32)>,
    /// The ambient colour (`0x2650d0`).
    pub ambient: [u8; 3],
    /// A last scale factor `randf(lo, hi)` (drawn after the shrink) when `Some`.
    pub grow: Option<(f32, f32)>,
    /// +0xbc carries the gold flag in bit 1 (`type | gold << 1`); else the type alone.
    pub gold_bits: bool,
}

/// `0x2c4c20`: the Bomb Glove's fireball.
pub const GLOVE_FIREBALL: FireballRow = FireballRow { class: FIREBALL_CLASS, alpha: None, ambient: [0x7f, 0x7f, 0x7f], grow: None, gold_bits: true };

/// `FUN_002c4c20(pos, vel, life, type, gold)` (with [`GLOVE_FIREBALL`]): a fireball (class 122).
pub(crate) fn fireball(w: &mut World, pos: [f32; 4], vel: [f32; 4], life: i32, ty: u8) { spawn_fireball(w, &GLOVE_FIREBALL, pos, vel, life, ty, 0); }

/// The fireball spawners ([`FireballRow`]): `CreateMoby(class)`, drawn, distances 0xff, the alpha (the tank's
/// `rand_range(0x40, 0x80)`), the ambient, the position (the whole quadword), the velocity (+0x00), two spins
/// `randf(2π, 4π)·dt` (+0x10 / +0x14), the life (+0x18 timer, +0x1a start), type 0: scale ·`randf(0.5, 0.75)`;
/// +0x1c = the scale; the tank's scale ·`randf(2, 5)`; +0xbc = the type (| gold << 1); the matrix.
#[allow(clippy::too_many_arguments)]
pub fn spawn_fireball(w: &mut World, row: &FireballRow, pos: [f32; 4], vel: [f32; 4], life: i32, ty: u8, gold: u8) {
    let Some(m) = w.create_moby(row.class) else { return };
    let alpha = match row.alpha { Some((lo, hi)) => w.rng.rand_range(lo, hi) as u8, None => 0x80 };
    let a = w.rng.randf(dt() * 2.0 * PI, dt() * 4.0 * PI);
    let b = w.rng.randf(dt() * 2.0 * PI, dt() * 4.0 * PI);
    let shrink = if ty == 0 { Some(w.rng.randf(0.5, 0.75)) } else { None };
    let grow = row.grow.map(|(lo, hi)| w.rng.randf(lo, hi));
    let mo = w.mm(m);
    mo.visible = 1;
    mo.alpha = alpha;
    mo.draw_dist = 0xff;
    mo.update_dist = 0xff;
    mo.ambient = [row.ambient[0], row.ambient[1], row.ambient[2], mo.ambient[3]];
    mo.position = pos;
    if mo.pvars.len() < 0x20 { mo.pvars.resize(0x20, 0); }
    p::set_v4f(&mut mo.pvars, 0, vel);
    p::set_ff(&mut mo.pvars, 0x10, a);
    p::set_ff(&mut mo.pvars, 0x14, b);
    p::set_i16(&mut mo.pvars, 0x1a, life as i16);
    p::set_i16(&mut mo.pvars, 0x18, life as i16);
    if let Some(s) = shrink { mo.scale *= s; }
    let sc = mo.scale;
    p::set_ff(&mut mo.pvars, 0x1c, sc);
    mo.cmd = if row.gold_bits { ty | gold << 1 } else { ty };
    if let Some(g) = grow { mo.scale *= g; }
    w.build_matrix(m);
}

/// One of a blast's five flashes: size (× k), duration T (ticks), colour and alpha.
#[derive(Clone, Copy, Debug)]
pub struct BlastFlash {
    pub size: f32,
    pub t: i32,
    pub rgba: [u8; 4],
}

/// The explosion code the Bomb Glove's bomb (inline in `0x2c3300`), the Suck Cannon's fired creature (`0x304798`)
/// and Gemlik's tanks (level13 `0x3073c8`) were compiled from, as data (docs/plan/explosions.md §B): the fireball
/// spawner, the rings' colour tables, the five flashes (class 1192, `debris::FLASH_BOMB`; the first two only when the
/// camera is farther than 9).
#[derive(Clone, Copy, Debug)]
pub struct BlastRow {
    pub fireball: FireballRow,
    pub ring_c1: [u32; 6],
    pub ring_c2: [u32; 6],
    pub flashes: [BlastFlash; 5],
}

const fn bf(size: f32, t: i32, r: u8, g: u8, b: u8, a: u8) -> BlastFlash { BlastFlash { size, t, rgba: [r, g, b, a] } }

/// The Bomb Glove's (and the Suck Cannon burst's: its tables 0x20b820 / 0x20b838 hold the same words as 0x20a980 /
/// 0x20a998, its flashes the same constants).
pub const GLOVE_BLAST: BlastRow = BlastRow {
    fireball: GLOVE_FIREBALL,
    ring_c1: RING_C1,
    ring_c2: RING_C2,
    flashes: [bf(4.0, 15, 0x7f, 0x7f, 0x7f, 0x20), bf(4.0, 24, 0x7f, 0x20, 0, 0x20), bf(4.0, 20, 0x7f, 0x3f, 0, 0x30), bf(3.5, 27, 0x60, 0x10, 0, 0x40), bf(3.0, 29, 0x20, 0, 0, 0x20)],
};

/// The blast (row, the moby `id`, at `pos`; `base` the velocity every piece carries (the bomb's drift, the others'
/// `(0, 0, 2·dt)·k`), `normal` the face the spread keeps to, `k` the size (the gold glove / cannon: 2; the tank 1.5),
/// `n` the fireball divisor (the non-gold Suck Cannon: 2), `dry` false: only the low fireballs (the bomb under water),
/// `shift` the colour shift `0x270fa8` / `0x270f48` of the rings and the last four flashes (the gold glove / cannon)).
/// In the game's order: 10/n low fireballs (a random xy direction flattened onto the face, pushed `randf(0, 1)` off
/// it, `randf(3.5, 6.5)·k·dt` fast, plus `base`, `rand_range(ticks(60), ticks(120))`), 4/n high ones (`randf(6.5,
/// 10)`, `(ticks(60), ticks(90))`), one toward the camera (a random `(d/5)·dt` plus `2d·dt` toward it, 0.5·d up, at
/// most `10·dt`), the rings (type 11: `trunc(d) + 1` within 6 else 4; `randf(8, 10)·k·dt`, less `(7 − d)·dt` within
/// 7; sprite `k·400000`), the flashes.
#[allow(clippy::too_many_arguments)]
pub fn blast(w: &mut World, row: &BlastRow, id: MobyId, pos: [f32; 4], base: [f32; 4], normal: [f32; 4], k: f32, n: i32, dry: bool, shift: u8) {
    use crate::moby_update::creature::{add, clamp_len3, len3, set_len3, sub};
    use crate::moby_update::creature::fx::colour_shift;
    let cam = w.camera.map(|x| x.to_f32());
    let to_cam = sub(cam, pos);
    let dcam = len3(to_cam);
    let spread = |w: &mut World, lo: f32, hi: f32| -> [f32; 4] {
        let x = w.rng.randf(-1.0, 1.0);
        let y = w.rng.randf(-1.0, 1.0);
        let mut v = [x, y, 0.0, 0.0];
        let d = dot3(v, normal);
        v = sub(v, set_len3(normal, d));
        let u = w.rng.randf(0.0, 1.0);
        v = add(v, set_len3(normal, u));
        let s = w.rng.randf(lo, hi);
        add(set_len3(v, k * s * dt()), base)
    };
    for _ in 0..10 / n {
        let v = spread(w, 3.5, 6.5);
        let (a, b) = (w.ticks(60), w.ticks(120));
        let life = w.rng.rand_range(a, b);
        spawn_fireball(w, &row.fireball, pos, v, life, 0, shift);
    }
    if !dry { return; }
    for _ in 0..4 / n {
        let v = spread(w, 6.5, 10.0);
        let (a, b) = (w.ticks(60), w.ticks(90));
        let life = w.rng.rand_range(a, b);
        spawn_fireball(w, &row.fireball, pos, v, life, 1, shift);
    }
    let x = w.rng.randf(-1.0, 1.0);
    let y = w.rng.randf(-1.0, 1.0);
    let z = w.rng.randf(-1.0, 1.0);
    let mut c = to_cam;
    c[2] += dcam * 0.5;
    let v = add(set_len3([x, y, z, 0.0], (dcam / 5.0) * dt()), set_len3(c, (dcam + dcam) * dt()));
    let v = clamp_len3(v, dt() * 10.0);
    let (a, b) = (w.ticks(60), w.ticks(90));
    let life = w.rng.rand_range(a, b);
    spawn_fireball(w, &row.fireball, pos, v, life, 1, shift);
    let rings = if dcam < 6.0 { dcam as i32 + 1 } else { 4 };
    let slow = if dcam < 7.0 { 7.0 - dcam } else { 0.0 };
    for _ in 0..rings {
        let s = w.rng.randf(8.0, 10.0);
        let speed = k * s * dt() - slow * dt();
        let c1 = colour_shift(row.ring_c1[w.rng.randi(6) as usize], shift);
        let c2 = colour_shift(row.ring_c2[w.rng.randi(6) as usize], shift);
        let (a, b) = (w.ticks(15), w.ticks(20));
        let life = w.rng.rand_range(a, b);
        let (a, b) = (w.ticks(25), w.ticks(30));
        let t1 = w.rng.rand_range(a, b);
        w.part11(Pf::f(k * 400_000.0), Pf::f(speed), pv4(pos), pv4(base), c1, c2, life, t1, 0, 0);
    }
    for (i, f) in row.flashes.iter().enumerate() {
        if i < 2 && dcam <= 9.0 { continue; }
        let [r, g, b, a] = f.rgba;
        // `0x270f48(&r, &g, &b, shift)`: every flash but the first.
        let c = if i == 0 { r as u32 | (g as u32) << 8 | (b as u32) << 16 } else { colour_shift(r as u32 | (g as u32) << 8 | (b as u32) << 16, shift) };
        let t = w.ticks(f.t);
        super::debris::flash_spawn_as(w, &super::debris::FLASH_BOMB, k * f.size, id, pos, base, t, c as u8, (c >> 8) as u8, (c >> 16) as u8, a);
    }
}

/// `0x2c4d88`: the fireball.
pub fn fireball_update(w: &mut World, id: MobyId) {
    if w.m(id).pvars.len() < 0x20 { w.mm(id).pvars.resize(0x20, 0); }
    if w.m(id).cmd & 1 != 0 { smoke(w, id); }
    let m = w.mm(id);
    let v = p::v4f(&m.pvars, 0);
    m.rotation[0] = wrap(m.rotation[0] + p::ff(&m.pvars, 0x10));
    m.rotation[1] = wrap(m.rotation[1] + p::ff(&m.pvars, 0x14));
    m.position = add3(m.position, v);
    p::set_ff(&mut m.pvars, 8, v[2] - dt2() * 14.6);
    if m.position[0] < 0.0 || m.position[1] < 0.0 || m.position[2] < 0.0 {
        w.delete_moby(id);
        return;
    }
    let life0 = p::i16(&m.pvars, 0x1a) as i32;
    let q = if life0 < 0 { (life0 + 3) >> 2 } else { life0 >> 2 };
    let timer = p::i16(&m.pvars, 0x18);
    if (timer as i32) < q {
        m.scale = p::ff(&m.pvars, 0x1c) * timer as f32 / q as f32;
    }
    let mut t = timer;
    let done = sv::fast_dec_timer_s16(&mut t) != 0;
    p::set_i16(&mut m.pvars, 0x18, t);
    if done { w.delete_moby(id); }
}

/// The fireball's smoke (0x2c4d88 while +0xbc bit 0 is set, i.e. the high fireballs): every tick one type-4 puff
/// (`crate::particles::type04`) at the fireball, moving `dt` in a random direction (3 × `randf(−1, 1)`, normalised),
/// colour [`SMOKE`]`[randi(4)]` fading to 0x4fff over `ticks(45)`, size 50 growing by 120 (×1000), additive. The gold
/// glove's colour shift (+0xbc > 1, `0x270fa8`) is not ported (counted).
fn smoke(w: &mut World, id: MobyId) {
    let x = w.rng.randf(-1.0, 1.0);
    let y = w.rng.randf(-1.0, 1.0);
    let z = w.rng.randf(-1.0, 1.0);
    let v = setlen([x, y, z, 0.0], dt());
    let k = w.rng.randi(4) as usize;
    if w.m(id).cmd & 0xfe != 0 { w.svc.unported("fireball smoke: gold colours 0x270fa8"); }
    let a = crate::particles::type04::Spawn { pos: w.m(id).position, vel: v, c1: SMOKE[k], c2: 0x4fff, life: w.ticks(45), base: 0x32, growth: 0x78, additive: true };
    crate::moby_update::creature::fx::part04(w, &a);
}

/// State 2: the growing sphere.
fn exploding(w: &mut World, id: MobyId) {
    let k = 1.0f32;
    let pos = w.m(id).position;
    let c = w.m(id).cmd;
    {
        let m = w.mm(id);
        m.mode |= 0x41;
    }
    let r = 0.5 + ((k * 3.0 - 0.5) - 0.5) * (c as f32 / 15.0);
    let tmpl = HitTemplate { dir: [Pf::ZERO, Pf::ZERO, Pf::ONE, Pf::b(0x45af_df66)], attacker: Some(id), flags: 0x83_0000, b18: 2, b19: 1, h1a: BOMB_CLASS as u16, damage: Pf::f(2.0), w20: 1 };
    w.sphere_mobys(Pf::f(r), pv4(pos), 0x10, Some(id), Some(&tmpl));
    let m = w.mm(id);
    m.cmd = m.cmd.wrapping_add(1);
    if m.cmd as i32 > 15 { w.delete_moby(id); }
}

/// `CreateMoby(o_class)` 0x263390 from the hero's code (the glove's bomb): [`World::create_moby`]'s work on the
/// moby system's parts (the hero's hit sink holds them, not a `World`).
pub fn create_from_hero(table: &mut crate::moby_runtime::MobyTable, svc: &mut sv::Services, classes: &dyn sv::ClassData, o_class: i16, counter: u64) -> Option<MobyId> {
    let info = classes.info(o_class);
    let id = table.create(o_class, info.as_ref(), counter)?;
    if svc.snapshots.len() <= id { svc.snapshots.resize(id + 1, None); }
    svc.snapshots[id] = None;
    Some(id)
}

/// `DeleteMoby` 0x2636c0 from the hero's code: [`World::delete_moby`]'s work (state, reuse tick, grid removal).
pub fn delete_from_hero(table: &mut crate::moby_runtime::MobyTable, svc: &mut sv::Services, id: MobyId, counter: u64) {
    table.delete(id, counter);
    std::sync::Arc::make_mut(&mut svc.grid).remove(&mut table.mobys[id]);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::hero::Hero;
    use crate::moby_runtime::{ClassInfo, Moby, MobyTable};
    use crate::moby_update::scheduler::{self, Scheduler};
    use crate::moby_update::services::{ClassTable, Services};
    use crate::rng::Rng;

    fn classes() -> ClassTable {
        let mut t = ClassTable::default();
        for (slot, oc) in [(1u8, BOMB_CLASS), (2, FIREBALL_CLASS), (3, FLASH_CLASS), (4, crate::moby_update::creature::fx::LIGHT_CLASS)] {
            let info = ClassInfo { slot, update_fn: scheduler::port_update_fn(oc), scale: 1.0, ..Default::default() };
            t.classes.insert(oc, (info, None));
        }
        t
    }

    /// A released bomb whose fuse runs out in the air: it explodes (state 2: hidden, the 10 + 5 fireballs and the 3
    /// flashes of a close camera), grows its sphere for 15 ticks and is deleted; the fireballs fall and run out.
    #[test]
    fn fuse_explosion_and_fireballs() {
        let mut h = Moby::init_instance(0, 0, Some(&ClassInfo { scale: 1.0, ..Default::default() }));
        h.mode |= crate::moby_runtime::mode::NO_UPDATE;
        let mut table = MobyTable::new(vec![h], 64);
        let ct = classes();
        let mut hero = Hero::new();
        hero.pos = crate::hero::physics::v4(100.0, 100.0, 50.0);
        let mut rng = Rng::new();
        let mut svc = Services::new();
        let mut sched = Scheduler::new();
        let b = table.create(BOMB_CLASS, ct.classes.get(&BOMB_CLASS).map(|c| &c.0), 0).unwrap();
        init_held(&mut table.mobys[b], &mut rng, [102.0, 100.0, 51.0], 0.0);
        p::set_v4f(&mut table.mobys[b].pvars, pv::VEL, [0.1, 0.0, 0.0, 0.0]);
        release(&mut table.mobys[b], [102.0, 100.0, 51.0], None);
        p::set_i16(&mut table.mobys[b].pvars, pv::LIFE, 3);
        let mut states = Vec::new();
        let mut fireballs = 0;
        let mut flashes = 0;
        for counter in 0..200u64 {
            table.free_slot_pass(counter);
            let mut w = World::new(&mut table, &hero, &mut rng, &ct, &mut svc, counter);
            sched.tick(&mut w);
            states.push(table.mobys[b].state);
            fireballs = fireballs.max(table.mobys.iter().filter(|m| m.o_class == FIREBALL_CLASS && m.state < 0xfd).count());
            flashes = flashes.max(table.mobys.iter().filter(|m| m.o_class == FLASH_CLASS && m.state < 0xfd).count());
        }
        let boom = states.iter().position(|&s| s == EXPLODING).expect("never exploded");
        let gone = states.iter().position(|&s| s >= 0xfd).expect("never deleted");
        assert!(boom <= 4, "exploded at {boom}");
        assert_eq!(gone - boom, 15, "15 ticks of the growing sphere");
        assert_eq!(fireballs, 15);
        assert_eq!(flashes, 3);
        assert_eq!(svc.camera_shakes.len(), 1);
        assert!(table.mobys.iter().all(|m| m.o_class != FIREBALL_CLASS || m.state >= 0xfd), "fireballs outlived their timers");
    }

    /// Draws between two stream states.
    fn draws(from: u32, to: u32) -> usize {
        let mut r = Rng { state: from };
        for n in 0..1_000_000 {
            if r.state == to { return n; }
            r.rand();
        }
        panic!("state not reached");
    }

    /// The rand stream through a bomb's flight and dry explosion, tick by tick, against a ledger of the draws the
    /// game's code makes (per call, from the level01 disassembly):
    /// * flight (0x2c35c8): on ticks with `counter & 7 == 0`, one trail blob: `rand_vec` (3), 2 sizes, 2 colour tweens,
    ///   3 phase lengths = 10, plus `PartType02Spawn`'s `randf(0, 255)` = **11**;
    /// * explosion: 10 low fireballs × (spread 4 + `rand_range` + `0x2c4c20`'s 2 spins + 1 shrink) = 80, 4 high × (4 +
    ///   1 + 2) = 28, the one toward the camera 3 + 1 + 2 = 6, n smoke rings × (5 + `PartType11Spawn`'s 5), 3 per flash
    ///   (`0x309a68`), the light spawn 0;
    /// * every update of a high fireball (+0xbc = 1): the smoke (3 `randf` + `randi(4)` + `PartType04Spawn`'s `rand()`)
    ///   = **5**; low fireballs and flashes 0;
    /// * every update of the explosion light past its 3-tick delay: `randf(min, max)` for the radius (flag 0x80000) = **1**.
    ///
    /// The particle updates run on a stream of their own here (their draws are the type modules' tests: types 2, 4
    /// draw none, 11 its splits), so every moby-side draw is accounted for. Nothing is left unported on a dry explosion.
    #[test]
    fn dry_explosion_rand_stream_matches_the_game_ledger() {
        let mut h = Moby::init_instance(0, 0, Some(&ClassInfo { scale: 1.0, ..Default::default() }));
        h.mode |= crate::moby_runtime::mode::NO_UPDATE;
        let mut table = MobyTable::new(vec![h], 96);
        let ct = classes();
        let mut hero = Hero::new();
        hero.pos = crate::hero::physics::v4(100.0, 100.0, 50.0);
        let mut rng = Rng::new();
        rng.srand(crate::rng::LEVEL_SEED);
        let mut part_rng = Rng::new();
        let mut svc = Services::new();
        let mut sched = Scheduler::new();
        let mut parts = crate::particles::Particles::new(None, Vec::new());
        let b = table.create(BOMB_CLASS, ct.classes.get(&BOMB_CLASS).map(|c| &c.0), 0).unwrap();
        init_held(&mut table.mobys[b], &mut rng, [102.0, 100.0, 51.0], 0.0);
        p::set_v4f(&mut table.mobys[b].pvars, pv::VEL, [0.1, 0.0, 0.02, 0.0]);
        release(&mut table.mobys[b], [102.0, 100.0, 51.0], None);
        p::set_i16(&mut table.mobys[b].pvars, pv::LIFE, 20);
        // The camera 30 units away: 4 rings, 5 flashes.
        let camera = [Pf::f(130.0), Pf::f(100.0), Pf::f(51.0), Pf::ONE];
        let (mut trail_ticks, mut boom, mut smoke_updates, mut light_updates) = (0, None, 0, 0);
        for counter in 1..200u64 {
            table.free_slot_pass(counter);
            // Observables before the tick: the high fireballs' and the lights' timers.
            let timers = |t: &MobyTable| -> Vec<(usize, i16, u8, i32)> {
                t.mobys.iter().enumerate().filter(|(_, m)| m.state < 0xfd).map(|(i, m)| match m.o_class {
                    FIREBALL_CLASS => (i, p::i16(&m.pvars, 0x18), m.cmd, 0),
                    crate::moby_update::creature::fx::LIGHT_CLASS if m.pvars.len() >= 0x80 => (i, 0, 0xff, p::i32(&m.pvars, 0x48)),
                    _ => (i, 0, 0xfe, 0),
                }).collect()
            };
            let before = timers(&table);
            let flying = table.mobys[b].state == FLYING;
            let s0 = rng.state;
            {
                let mut w = World::new(&mut table, &hero, &mut rng, &ct, &mut svc, counter);
                w.camera = camera;
                w.particles = Some(&mut parts);
                sched.tick(&mut w);
            }
            parts.counter = counter;
            parts.update_parts(&mut part_rng);
            let made = draws(s0, rng.state);
            let mut want = 0;
            if flying && counter & 7 == 0 && table.mobys[b].state == FLYING { want += 11; trail_ticks += 1; }
            if flying && table.mobys[b].state != FLYING {
                // 4 rings (camera ≥ 6 away), 5 flashes (camera > 9 away); the trail blob when this tick was one.
                want += 80 + 28 + 6 + 4 * (5 + 5) + 5 * 3;
                if counter & 7 == 0 { want += 11; }
                boom = Some(counter);
            }
            // Updates this tick: a high fireball whose timer moved, a light whose life moved (both existed before).
            for (i, t0, cmd, life0) in &before {
                let m = &table.mobys[*i];
                if *cmd == 1 && m.o_class == FIREBALL_CLASS && (m.state >= 0xfd || p::i16(&m.pvars, 0x18) != *t0) { want += 5; smoke_updates += 1; }
                if *cmd == 0xff && m.state < 0xfd && p::i32(&m.pvars, 0x48) != *life0 {
                    want += 1;
                    light_updates += 1;
                }
            }
            // A light created this tick runs its first update (state 0, its delay) in the same pass: no draw.
            assert_eq!(made, want, "tick {counter}: {made} draws, the game's code makes {want}");
        }
        assert!(trail_ticks >= 2, "trail ticks {trail_ticks}");
        assert!(boom.is_some(), "never exploded");
        assert!(smoke_updates >= 5 * 50, "smoke updates {smoke_updates}");
        assert!(light_updates > 20, "light updates {light_updates}");
        assert!(svc.fx.unported.is_empty(), "unported on a dry explosion: {:?}", svc.fx.unported);
        let spawned = &svc.fx.part_spawns;
        assert_eq!(spawned.get(&2).copied().unwrap_or(0), trail_ticks as u64);
        assert_eq!(spawned.get(&4).copied().unwrap_or(0), smoke_updates as u64);
        // The light took a point-light slot and gave it back when it ended.
        assert!(svc.point_lights.active().next().is_none());
    }

    // ---------------------------------------------------------------------------------------------------------
    // The shared blast (docs/plan/explosions.md §B): one test per caller's row, and the flash kinds.

    const TANK_BALL: i16 = crate::moby_update::classes::units::explosive_tank::FIREBALL;

    /// A world with the blast's classes (the glove's fireball 122, the tank's 1634, the flash 1192) and a source moby 0
    /// at (100, 100, 50); the camera `cam` units away along x.
    fn blast_run(row: &BlastRow, k: f32, n: i32, dry: bool, shift: u8, cam: f32) -> (MobyTable, Services, usize) {
        let mut ct = ClassTable::default();
        for (slot, oc) in [(1u8, FIREBALL_CLASS), (2, TANK_BALL), (3, FLASH_CLASS)] {
            ct.classes.insert(oc, (ClassInfo { slot, scale: 1.0, ..Default::default() }, None));
        }
        let mut src = Moby::init_instance(0, 0, Some(&ClassInfo { scale: 1.0, ..Default::default() }));
        src.position = [100.0, 100.0, 50.0, 1.0];
        let mut table = MobyTable::new(vec![src], 96);
        let hero = Hero::new();
        let mut rng = Rng::new();
        rng.srand(crate::rng::LEVEL_SEED);
        let mut svc = Services::new();
        let s0 = rng.state;
        {
            let mut w = World::new(&mut table, &hero, &mut rng, &ct, &mut svc, 1);
            w.camera = [Pf::f(100.0 + cam), Pf::f(100.0), Pf::f(50.0), Pf::ONE];
            let base = [0.0, 0.0, 2.0 * dt() * k, 0.0];
            blast(&mut w, row, 0, [100.0, 100.0, 50.0, 1.0], base, [0.0, 0.0, 1.0, 0.0], k, n, dry, shift);
        }
        let d = draws(s0, rng.state);
        (table, svc, d)
    }

    fn alive(t: &MobyTable, oc: i16) -> Vec<&Moby> { t.mobys.iter().filter(|m| m.o_class == oc && m.state < 0x80).collect() }

    /// The Bomb Glove's row (`0x2c3300`, k = 1): 10 low + 4 high + 1 camera fireballs of class 122 (alpha 0x80, ambient
    /// 0x7f, +0xbc = the type), 4 rings, 5 flashes of class 1192 beyond 9 (sizes 4 / 4 / 4 / 3.5 / 3, T 15 / 24 / 20 /
    /// 27 / 29, the timer at T and the size at 0: no head start), 3 within 9; under water (dry = false) only the 10 low
    /// ones. Draws: 10·(4 + 1 + 3) + 4·(4 + 1 + 2) + (3 + 1 + 2) + 4·5 (+ the type-11 spawner's own with particles) + 5·3.
    #[test]
    fn blast_glove_row() {
        let (t, svc, d) = blast_run(&GLOVE_BLAST, 1.0, 1, true, 0, 30.0);
        let balls = alive(&t, FIREBALL_CLASS);
        assert_eq!(balls.len(), 15);
        assert!(balls.iter().all(|m| m.alpha == 0x80 && m.ambient[..3] == [0x7f, 0x7f, 0x7f] && m.cmd <= 1));
        assert_eq!(balls.iter().filter(|m| m.cmd == 0).count(), 10);
        let fl = alive(&t, FLASH_CLASS);
        let got: Vec<(f32, i32, i16, [u8; 3], u8)> = fl.iter().map(|m| (p::ff(&m.pvars, 0x18), p::i32(&m.pvars, 0x14), p::i16(&m.pvars, 0x1c), [m.ambient[0], m.ambient[1], m.ambient[2]], m.alpha)).collect();
        assert_eq!(got, vec![(4.0, 15, 15, [0x7f, 0x7f, 0x7f], 0x20), (4.0, 24, 24, [0x7f, 0x20, 0], 0x20), (4.0, 20, 20, [0x7f, 0x3f, 0], 0x30), (3.5, 27, 27, [0x60, 0x10, 0], 0x40), (3.0, 29, 29, [0x20, 0, 0], 0x20)]);
        assert!(fl.iter().all(|m| m.scale == 0.0));
        assert_eq!(svc.fx.flashes, 5);
        // Without a particle system the ring spawns make only their throttle's draws (none at load 0).
        assert_eq!(d, 10 * 8 + 4 * 7 + 6 + 4 * 5 + 5 * 3);
        let (t, _, _) = blast_run(&GLOVE_BLAST, 1.0, 1, true, 0, 5.0);
        assert_eq!(alive(&t, FLASH_CLASS).len(), 3, "within 9: the last three flashes");
        let (t, _, d) = blast_run(&GLOVE_BLAST, 1.0, 1, false, 0, 30.0);
        assert_eq!((alive(&t, FIREBALL_CLASS).len(), alive(&t, FLASH_CLASS).len(), d), (10, 0, 80), "under water: the low fireballs only");
    }

    /// The Suck Cannon burst's row (`0x304798`): the glove's tables at k = 1 with the fireballs halved (5 + 2 + 1);
    /// the gold cannon (k = 2, n = 1, shift 1): all 15, the flashes twice the size, the last four with red and green
    /// swapped (`0x270f48`), the first unshifted, and the fireballs' +0xbc carrying the gold bit.
    #[test]
    fn blast_suck_cannon_row() {
        let (t, _, _) = blast_run(&GLOVE_BLAST, 1.0, 2, true, 0, 30.0);
        assert_eq!(alive(&t, FIREBALL_CLASS).len(), 5 + 2 + 1);
        assert_eq!(alive(&t, FLASH_CLASS).len(), 5);
        let (t, _, _) = blast_run(&GLOVE_BLAST, 2.0, 1, true, 1, 30.0);
        let balls = alive(&t, FIREBALL_CLASS);
        assert_eq!(balls.len(), 15);
        assert_eq!(balls.iter().filter(|m| m.cmd == 2).count(), 10, "low: 0 | gold << 1");
        assert_eq!(balls.iter().filter(|m| m.cmd == 3).count(), 5, "high: 1 | gold << 1");
        let fl: Vec<(f32, [u8; 3])> = alive(&t, FLASH_CLASS).iter().map(|m| (p::ff(&m.pvars, 0x18), [m.ambient[0], m.ambient[1], m.ambient[2]])).collect();
        assert_eq!(fl, vec![(8.0, [0x7f, 0x7f, 0x7f]), (8.0, [0x20, 0x7f, 0]), (8.0, [0x3f, 0x7f, 0]), (7.0, [0x10, 0x60, 0]), (6.0, [0, 0x20, 0])]);
    }

    /// Gemlik's tank row (level13 `0x3073c8` / `0x30c138`): k = 1.5, 15 fireballs of class 1634 (alpha
    /// `rand_range(0x40, 0x80)`, ambient (0x20, 0x60, 0x10), +0x1c the scale before the `randf(2, 5)` growth, +0xbc
    /// the type alone), the green flashes 6 / 6 / 6 / 5.25 / 4.5. Draws: two more per fireball than the glove's.
    #[test]
    fn blast_tank_row() {
        use crate::moby_update::classes::units::explosive_tank::TANK_BLAST;
        let (t, _, d) = blast_run(&TANK_BLAST, 1.5, 1, true, 0, 30.0);
        let balls = alive(&t, TANK_BALL);
        assert_eq!((balls.len(), alive(&t, FIREBALL_CLASS).len()), (15, 0));
        for m in &balls {
            assert!((0x40..=0x80).contains(&m.alpha) && m.ambient[..3] == [0x20, 0x60, 0x10] && m.cmd <= 1);
            let g = m.scale / p::ff(&m.pvars, 0x1c);
            assert!((2.0..=5.0).contains(&g), "growth {g}");
        }
        let fl: Vec<(f32, [u8; 3])> = alive(&t, FLASH_CLASS).iter().map(|m| (p::ff(&m.pvars, 0x18), [m.ambient[0], m.ambient[1], m.ambient[2]])).collect();
        assert_eq!(fl, vec![(6.0, [0x7f, 0x7f, 0x7f]), (6.0, [0x20, 0x7f, 0]), (6.0, [0x3f, 0x7f, 0]), (5.25, [0x10, 0x60, 0]), (4.5, [0, 0x20, 0])]);
        assert_eq!(d, 10 * 10 + 4 * 9 + 8 + 4 * 5 + 5 * 3);
    }
}

