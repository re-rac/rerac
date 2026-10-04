//! The hero state registry: every state id 0..=0x82 of the game's hero state machine, the game's own movement
//! group (the byte SetState writes to `0x1413dc`), the port module that owns it, and whether the port implements
//! it; plus the three dispatchers the driver calls — SetState's per-state entry (`0x23cf98`), the per-state
//! physics (`0x2370b8`) and the per-state transitions (`0x242930`).
//!
//! Inventory, sources and the port packages: `docs/plan/hero_states.md`. The table is the union of the 19
//! levels' state machines (each overlay compiles its own SetState / physics / transitions; level00 has almost
//! every state, 0x3e and 0x6b..0x6f only exist on the levels that use them).
//!
//! **Adding a state:** its package ports the entry / physics / transitions in its own module (the stubs below are
//! already routed), then sets `ported: true` in [`STATES`]. Nothing else in the dispatch needs to change. The
//! hero freezes (`HeroTick::Unimplemented`) in every state whose row says `ported: false`.
#![allow(clippy::neg_cmp_op_on_partial_ord, clippy::assign_op_pattern)]

use super::anim::AnimCtl;
use super::physics::Env;
use super::states::Ctx;
use super::Hero;
use crate::rng::Rng;

/// The port module that owns a state (one file each; docs/plan/hero_states.md "Packages").
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Module {
    /// `ground.rs`: idle 0, stop 3, crouch 4.
    Ground,
    /// `walk.rs`: walk / run 2, wading 0x73.
    Walk,
    /// `air.rs`: fall 6.
    Air,
    /// `jump.rs`: the jump group (the ids the port has: 7, 9, 0xb, 0xe, 0x12; 0xc has no owner yet).
    Jump,
    /// `melee.rs`: the wrench (group 6).
    Melee,
    /// `swim.rs`: the water states.
    Swim,
    /// `ledge.rs` (package P3): ledge grab / hang / shimmy / climb, wall jump.
    Ledge,
    /// `packs.rs` (P4): Heli-Pack and Thruster-Pack.
    Packs,
    /// `boots.rs` (P5): Magneboots, Grind Boots, the cable slide (spline riders).
    Boots,
    /// `swingshot.rs` (P6): Swingshot pull and swing.
    Swingshot,
    /// `surface.rs` (P1): surface-driven states (slippery walk, sinking floors).
    Surface,
    /// `damage.rs` (P2): hurt, knockback, deaths.
    Damage,
    /// `stance.rs` (P2): look stance, fidget state, scripted walk-to-point.
    Stance,
    /// `weapons.rs`: the weapon states (the glove throw 0x23; the weapon stances 0x17 / 0x27 / 0x2e / 0x30 have no
    /// SetState caller in any level); the Hydrodisplacer's use poses 0x38..0x3a (`hydrodisplacer.rs`, set by its item
    /// update on a pad 341).
    Weapons,
    /// `crank.rs`: the bolt crank 0x3b (set and driven by the crank class 280).
    Crank,
    /// `scripted.rs`: the scripted / cutscene control states the classes set and end (0x72: the cinematics' hold; 0x1d:
    /// the Visibomb's flight; 0x1f: the mouse's summon; 0x32: turrets and vehicles; 0x78: the Umbris boss; 99 / 100:
    /// the scene body).
    Scripted,
    /// `bodies/clank.rs` (body 1: 0x43..0x52, 0x7d), `bodies/giant.rs` (body 2: 0x5a..0x62) and `bodies/disguise.rs` (the
    /// Hologuise disguise, body 3: 0x53..0x59).
    Bodies,
    /// `hoverboard.rs`: the Hoverboard (levels 5 and 16): 0x6b..0x6f (level 16's 0x3e is not ported).
    Hoverboard,
    /// No state with this id exists in any level's SetState.
    Unused,
}

/// One row of the registry.
#[derive(Clone, Copy, Debug)]
pub struct StateInfo {
    /// Our name (the game has none).
    pub name: &'static str,
    /// The game's movement group SetState writes to `0x1413dc` (−1: none / left unchanged / unknown).
    pub group: i8,
    pub module: Module,
    /// The port implements the state (entry, physics and transitions).
    pub ported: bool,
}

const fn s(name: &'static str, group: i8, module: Module, ported: bool) -> StateInfo { StateInfo { name, group, module, ported } }

use Module::*;

/// Every hero state id 0..=0x82 (`0x1413d4`). Names and groups: docs/plan/hero_states.md (confidence there).
pub static STATES: [StateInfo; 0x83] = [
    s("idle", 0, Ground, true),                                  // 0x00
    s("look stance (L1/L2 first-person)", 0, Stance, true),       // 0x01
    s("walk / run", 1, Walk, true),                               // 0x02
    s("stop / skid", 1, Ground, true),                            // 0x03
    s("crouch", 0xc, Ground, true),                               // 0x04
    s("(none)", -1, Unused, false),                               // 0x05
    s("fall", 2, Air, true),                                      // 0x06
    s("jump", 4, Jump, true),                                     // 0x07
    s("pack glide / hover (Heli-Pack ✕ held)", 5, Packs, true),   // 0x08
    s("running jump", 4, Jump, true),                             // 0x09
    s("Heli-Pack long jump", 4, Packs, true),                     // 0x0a
    s("side / back flip", 4, Jump, true),                         // 0x0b
    s("jump variant (defaults only; source unknown)", 4, Jump, false), // 0x0c
    s("Thruster-Pack high jump", 4, Packs, true),                 // 0x0d
    s("double jump", 4, Jump, true),                              // 0x0e
    s("Heli-Pack high jump", 4, Packs, true),                     // 0x0f
    s("Thruster-Pack long jump", 4, Packs, true),                 // 0x10
    s("wall jump", 4, Ledge, true),                               // 0x11
    s("jump out of the water", 4, Jump, true),                    // 0x12
    s("wrench combo", 6, Melee, true),                            // 0x13
    s("wrench jump attack", 6, Melee, true),                      // 0x14
    s("comet strike (crouch + □)", 6, Melee, true),               // 0x15
    s("hurt (knockback)", 7, Damage, true),                       // 0x16
    s("weapon fire stance", 8, Weapons, false),                   // 0x17
    s("ledge grab", 3, Ledge, true),                              // 0x18
    s("ledge hang", 3, Ledge, true),                              // 0x19
    s("ledge shimmy left", 3, Ledge, true),                       // 0x1a
    s("ledge shimmy right", 3, Ledge, true),                      // 0x1b
    s("ledge climb / jump up", 4, Ledge, true),                   // 0x1c
    s("steering the Visibomb (no control)", 9, Scripted, true),   // 0x1d
    s("look stance (set by mobys)", 0, Stance, true),             // 0x1e
    s("held by a class, control kept (the mouse's summon)", 9, Scripted, true), // 0x1f
    s("gadget lunge (the Walloper)", 6, Melee, true),             // 0x20
    s("wrench rebound", 10, Melee, true),                         // 0x21
    s("Thruster-Pack stomp (R1 in the air)", 0xb, Packs, true),   // 0x22
    s("glove throw (hand item)", 6, Weapons, true),                // 0x23
    s("Swingshot fire", 0xd, Swingshot, true),                   // 0x24
    s("Swingshot pull", 0xd, Swingshot, true),                   // 0x25
    s("Swingshot arrive", 0xd, Swingshot, true),                 // 0x26
    s("weapon stance 2", 8, Weapons, false),                      // 0x27
    s("grind", 0xf, Boots, true),                                // 0x28
    s("grind jump", 0xf, Boots, true),                           // 0x29
    s("grind rail-switch jump", 0xf, Boots, true),               // 0x2a
    s("grind wrench swing", 0xf, Boots, true),                   // 0x2b
    s("Swingshot swing", 0xe, Swingshot, true),                  // 0x2c
    s("fall after a swing", 2, Swingshot, true),                 // 0x2d
    s("weapon draw walk", 1, Weapons, false),                     // 0x2e
    s("slippery-floor walk (surface 7)", 1, Surface, true),       // 0x2f
    s("weapon fire stance 3", 8, Weapons, false),                 // 0x30
    s("sinking floor, surface 4", 0x10, Surface, true),           // 0x31
    s("mounted: a turret / vehicle class holds him (frozen)", 9, Scripted, true), // 0x32
    s("underwater stroke", 0x11, Swim, true),                     // 0x33
    s("underwater drift", 0x11, Swim, true),                      // 0x34
    s("Hydro-Pack thrust", 0x11, Swim, true),                     // 0x35
    s("surface swim", 0x12, Swim, true),                          // 0x36
    s("tread water", 0x12, Swim, true),                           // 0x37
    s("Hydrodisplacer use pose: in", 0x13, Weapons, true),                     // 0x38
    s("Hydrodisplacer use pose: use", 0x13, Weapons, true),                     // 0x39
    s("Hydrodisplacer use pose: out", 0x13, Weapons, true),                     // 0x3a
    s("bolt crank (turning a bolt with the wrench)", 9, Crank, true), // 0x3b
    s("burn bounce (surface 1)", 4, Damage, true),                // 0x3c
    s("death", 0x14, Damage, true),                               // 0x3d
    s("level 16 group-0x15 state (unknown)", 0x15, Hoverboard, false), // 0x3e
    s("Magneboots walk", 1, Boots, true),                        // 0x3f
    s("fidget state", -1, Stance, true),                          // 0x40
    s("fidget state end", -1, Stance, true),                      // 0x41
    s("grind hurt", 0xf, Boots, true),                           // 0x42
    s("Clank idle", 0, Bodies, true),                            // 0x43
    s("Clank walk", 1, Bodies, true),                            // 0x44
    s("Clank fall", 2, Bodies, true),                            // 0x45
    s("Clank hurt", 7, Bodies, true),                            // 0x46
    s("Clank death", 0x14, Bodies, true),                        // 0x47
    s("(Clank, physics no-op only)", -1, Bodies, true),          // 0x48
    s("Clank jump", 4, Bodies, true),                            // 0x49
    s("Clank ledge grab", 3, Bodies, true),                      // 0x4a
    s("Clank ledge hang", 3, Bodies, true),                      // 0x4b
    s("Clank ledge climb", 4, Bodies, true),                          // 0x4c
    s("Clank ledge shimmy left", 3, Bodies, true),               // 0x4d
    s("Clank ledge shimmy right", 3, Bodies, true),              // 0x4e
    s("Clank glide", 5, Bodies, true),                           // 0x4f
    s("Clank walk variant (no entry)", -1, Bodies, true),        // 0x50
    s("Clank kick (□)", 6, Bodies, true),                         // 0x51
    s("Clank pit fall (surface 8 / 0xc)", 2, Bodies, true),                          // 0x52
    s("Hologuise idle", 0, Bodies, true),                        // 0x53
    s("Hologuise walk", 1, Bodies, true),                        // 0x54
    s("Hologuise fall", 2, Bodies, true),                        // 0x55
    s("Hologuise hurt", 7, Bodies, true),                        // 0x56
    s("Hologuise death", 0x14, Bodies, true),                    // 0x57
    s("Hologuise pit fall", 2, Bodies, true),                    // 0x58
    s("Hologuise ○ action", 0, Bodies, true),                    // 0x59
    s("Giant Clank idle", 0, Bodies, true),                      // 0x5a
    s("Giant Clank walk", 1, Bodies, true),                      // 0x5b
    s("Giant Clank fall", 2, Bodies, true),                      // 0x5c
    s("Giant Clank hurt", 7, Bodies, true),                      // 0x5d
    s("Giant Clank jump", 4, Bodies, true),                      // 0x5e
    s("Giant Clank missiles (○)", 6, Bodies, true),                  // 0x5f
    s("Giant Clank punches (□, three in a row)", 6, Bodies, true),                  // 0x60
    s("Giant Clank head beam (△)", 6, Bodies, true),                  // 0x61
    s("Giant Clank death", 0x14, Bodies, true),                  // 0x62
    s("cutscene control", 0x18, Scripted, true),                  // 0x63
    s("cutscene control / respawn (scene body)", 0x18, Scripted, true), // 0x64
    s("walk to point", 1, Stance, true),                          // 0x65
    s("walk to point: stop", 1, Stance, true),                    // 0x66
    s("walk to point: turn", 0, Stance, true),                    // 0x67
    s("sinking liquid, surface 3", 0x19, Surface, true),          // 0x68
    s("jump out of the sinking liquid", 0x19, Surface, true),     // 0x69
    s("drowned", 0x14, Swim, true),                               // 0x6a
    s("Hoverboard ride", 0x16, Hoverboard, true),                 // 0x6b
    s("Hoverboard ramp jump", 0x16, Hoverboard, true),            // 0x6c
    s("Hoverboard crash", -1, Hoverboard, true),                  // 0x6d
    s("Hoverboard into water", -1, Hoverboard, true),             // 0x6e
    s("Hoverboard into a wall", -1, Hoverboard, true),            // 0x6f
    s("Magneboots wrench swing", 6, Boots, true),                // 0x70
    s("Magneboots hop", 1, Boots, true),                        // 0x71
    s("scripted hold (cinematics)", 9, Scripted, true),           // 0x72
    s("wade", 1, Walk, true),                                     // 0x73
    s("cable slide (spline)", 0x1a, Boots, true),                // 0x74
    s("hurt on the water surface", 7, Damage, true),              // 0x75
    s("hurt under water", 7, Damage, true),                       // 0x76
    s("death fall (below the level's death height)", 2, Damage, true),  // 0x77
    s("held by the Umbris boss 1106", 9, Scripted, true),         // 0x78
    s("pit fall (surface 8 / 0xc)", 2, Damage, true),             // 0x79
    s("pack jump wall rebound", 10, Packs, true),                 // 0x7a
    s("sinking liquid, no health (surface 3)", 0x19, Surface, true), // 0x7b
    s("burn death (surface 1)", 0x14, Damage, true),              // 0x7c
    s("Clank burn (surface 1)", 0x14, Bodies, true),             // 0x7d
    s("(walk-case label only)", -1, Unused, false),               // 0x7e
    s("sinking death (surface 0xd)", 0x14, Damage, true),         // 0x7f
    s("death by a hazard moby (0x4eb / 0x558)", 0x14, Damage, true),  // 0x80
    s("Thruster-Pack hover", 1, Packs, true),                     // 0x81
    s("eaten in the water (class 0x28f)", 0x14, Damage, true),    // 0x82
];

/// The registry row of `id` (None outside 0..=0x82).
pub fn info(id: i32) -> Option<&'static StateInfo> { usize::try_from(id).ok().and_then(|i| STATES.get(i)) }

/// The module that owns `id` ([`Module::Unused`] outside the table).
pub fn module_of(id: i32) -> Module { info(id).map_or(Unused, |s| s.module) }

/// Whether the port implements `id` (entry, physics and transitions).
pub fn implemented(id: i32) -> bool { info(id).is_some_and(|s| s.ported) }

impl Hero {
    /// SetState's per-state entry (`0x23cf98`'s switch). `None`: continue with the epilogue (group change, timers);
    /// `Some(r)`: SetState returns `r` at once. A state whose module does not port its entry does nothing.
    pub(super) fn dispatch_entry(&mut self, c: &mut Ctx, id: i32, play: bool, old_sub: i32) -> Option<bool> {
        match module_of(id) {
            Ground => self.ground_entry(c, id, play),
            Walk => self.walk_entry(c, id, play),
            Air => self.fall_entry(c, play),
            Jump if implemented(id) => self.jump_group_entry(c, id, play, old_sub),
            Melee | Weapons if matches!(id, 0x13 | 0x14 | 0x15 | 0x20 | 0x23 | 0x51) => {
                self.melee_entry(c, id, play);
                None
            }
            Weapons if (0x38..=0x3a).contains(&id) => super::hydrodisplacer::entry(self, c, id, play, old_sub),
            Melee if id == 0x21 => {
                self.rebound_entry(c, play);
                None
            }
            Swim => {
                self.swim_entry(c, id, play);
                None
            }
            Ledge => super::ledge::entry(self, c, id, play, old_sub),
            Packs => super::packs::entry(self, c, id, play, old_sub),
            Boots => super::boots::entry(self, c, id, play, old_sub),
            Swingshot => super::swingshot::entry(self, c, id, play, old_sub),
            Surface => super::surface::entry(self, c, id, play, old_sub),
            Damage => super::damage::entry(self, c, id, play, old_sub),
            Stance => super::stance::entry(self, c, id, play, old_sub),
            Crank => super::crank::entry(self, c, id, play, old_sub),
            Scripted => super::scripted::entry(self, c, id, play, old_sub),
            Bodies if implemented(id) => {
                if (0x5a..=0x62).contains(&id) {
                    super::bodies::giant::entry(self, c, id, play, old_sub)
                } else if (0x53..=0x59).contains(&id) {
                    super::bodies::disguise::entry(self, c, id, play, old_sub)
                } else {
                    super::bodies::clank::entry(self, c, id, play, old_sub)
                }
            }
            Hoverboard if implemented(id) => super::hoverboard::entry(self, c, id, play, old_sub),
            Jump | Melee | Weapons | Bodies | Hoverboard | Unused => None,
        }
    }

    /// The per-state physics (`0x2370b8`'s switch). False: the state has no ported physics (the caller freezes
    /// the hero).
    pub(super) fn dispatch_physics(&mut self, env: &Env, anim: &mut dyn AnimCtl, rng: &mut Rng) -> bool {
        let s = self.state;
        match module_of(s) {
            Ground => {
                self.phys_ground(env, anim, rng);
                // Port-only: the Port Options strafe keeps Ratchet facing the camera in idle and stop (super::strafe).
                if self.strafe && matches!(s, 0 | 3) { super::strafe::face_camera(self, env); }
            }
            Walk => self.phys_walk(env),
            Air => self.phys_fall(env),
            Jump if implemented(s) => {
                // The deep-water jump's splash and bubbles (super::swim::effects::water_jump).
                if s == super::swim::id::WATER_JUMP { super::swim::effects::water_jump(self, anim, rng); }
                self.phys_jump(env)
            }
            Swim => match s {
                0x33..=0x35 => self.phys_underwater(env, anim, rng),
                0x36 | 0x37 => self.phys_surface(env, anim, rng),
                0x6a => self.phys_drown(anim, rng),
                _ => return false,
            },
            Melee => match s {
                0x13 => self.phys_combo(env, anim),
                0x14 => self.phys_jump_attack(env, anim),
                0x15 => return super::comet::physics(self, env, anim),
                0x20 => return super::walloper::physics(self, env, anim),
                0x21 => super::packs::rebound_physics(self),
                _ => return false,
            },
            Weapons if s == 0x23 => return super::weapons::physics(self, env),
            Weapons if (0x38..=0x3a).contains(&s) => return super::hydrodisplacer::physics(self, env),
            Ledge => return super::ledge::physics(self, env, anim, rng),
            Packs => return super::packs::physics(self, env, anim, rng),
            Boots => return super::boots::physics(self, env, anim, rng),
            Swingshot => return super::swingshot::physics(self, env, anim, rng),
            Surface => return super::surface::physics(self, env, anim, rng),
            Damage => return super::damage::physics(self, env, anim, rng),
            Stance => return super::stance::physics(self, env, anim, rng),
            Crank => return super::crank::physics(self),
            Scripted => return super::scripted::physics(self, env, anim, rng),
            Bodies if implemented(s) => {
                return if (0x5a..=0x62).contains(&s) {
                    super::bodies::giant::physics(self, env, anim, rng)
                } else if (0x53..=0x59).contains(&s) {
                    super::bodies::disguise::physics(self, env, anim, rng)
                } else {
                    super::bodies::clank::physics(self, env, anim, rng)
                };
            }
            Hoverboard if implemented(s) => return super::hoverboard::physics(self, env, anim, rng),
            Jump | Weapons | Bodies | Hoverboard | Unused => return false,
        }
        true
    }

    /// The per-state transitions (`0x242930`'s switch, after its prologue).
    pub(super) fn dispatch_transitions(&mut self, c: &mut Ctx) {
        let s = self.state;
        match module_of(s) {
            Ground => match s {
                0 => self.tr_idle(c),
                3 => self.tr_stop(c),
                _ => self.tr_crouch(c),
            },
            Walk => self.tr_walk(c),
            Air => self.tr_fall(c),
            Jump if implemented(s) => self.tr_jump(c),
            Melee => {
                if matches!(s, 0x13..=0x15) {
                    self.tr_melee(c)
                } else if s == 0x20 {
                    super::walloper::transitions(self, c)
                } else if s == 0x21 && c.anim.view().flags & 2 != 0 {
                    // 0x21 (with 0x7a): idle once the anim wraps.
                    self.set_state(c, 0, true);
                }
            }
            Weapons if s == 0x23 => super::weapons::transitions(self, c),
            Weapons if (0x38..=0x3a).contains(&s) => super::hydrodisplacer::transitions(self, c),
            Swim => match s {
                0x33 | 0x35 => self.tr_underwater(c),
                0x34 => self.tr_underwater_idle(c),
                0x36 => self.tr_surface_swim(c),
                0x37 => self.tr_surface_idle(c),
                _ => self.tr_drown(c),
            },
            Ledge => super::ledge::transitions(self, c),
            Packs => super::packs::transitions(self, c),
            Boots => super::boots::transitions(self, c),
            Swingshot => super::swingshot::transitions(self, c),
            Surface => super::surface::transitions(self, c),
            Damage => super::damage::transitions(self, c),
            Stance => super::stance::transitions(self, c),
            Crank => super::crank::transitions(self, c),
            Scripted => super::scripted::transitions(self, c),
            Bodies if implemented(s) => {
                if (0x5a..=0x62).contains(&s) {
                    super::bodies::giant::transitions(self, c)
                } else if (0x53..=0x59).contains(&s) {
                    super::bodies::disguise::transitions(self, c)
                } else {
                    super::bodies::clank::transitions(self, c)
                }
            }
            Hoverboard if implemented(s) => super::hoverboard::transitions(self, c),
            Jump | Weapons | Bodies | Hoverboard | Unused => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The restructure is behaviour-neutral: the implemented set is exactly the one before it.
    #[test]
    fn implemented_set_unchanged() {
        let got: Vec<i32> = (0..0x100).filter(|&s| implemented(s)).collect();
        let mut want = vec![0, 2, 3, 4, 6, 7, 9, 0xb, 0xe, 0x12, 0x13, 0x14, 0x33, 0x34, 0x35, 0x36, 0x37, 0x6a, 0x73];
        // Package P3 (ledge.rs).
        want.extend([0x11, 0x18, 0x19, 0x1a, 0x1b, 0x1c]);
        // Package P2 (damage.rs, stance.rs).
        want.extend([0x16, 0x3c, 0x3d, 0x75, 0x76, 0x77, 0x79, 0x7c, 0x7f, 0x80, 0x82, 1, 0x1e, 0x40, 0x41, 0x65, 0x66, 0x67]);
        // Package P1 (surface.rs).
        want.extend([0x2f, 0x31, 0x68, 0x69, 0x7b]);
        // Package P4 (packs.rs).
        want.extend([8, 10, 0xd, 0xf, 0x10, 0x22, 0x7a, 0x81]);
        // Package P5 (boots.rs).
        want.extend([0x28, 0x29, 0x2a, 0x2b, 0x3f, 0x42, 0x70, 0x71, 0x74]);
        // Package P6 (swingshot.rs).
        want.extend([0x24, 0x25, 0x26, 0x2c, 0x2d]);
        // Weapons + first person (comet.rs, weapons.rs).
        want.extend([0x15, 0x23]);
        // The gadget lunge (walloper.rs).
        want.push(0x20);
        // The Hydrodisplacer's use poses (hydrodisplacer.rs, G-WPN-006).
        want.extend([0x38, 0x39, 0x3a]);
        // The wrench rebound (melee.rs; the physics and transitions of 0x7a).
        want.push(0x21);
        // The bolt crank (crank.rs).
        want.push(0x3b);
        // The scripted hold (scripted.rs, the cinematics) and the scene body 99 / 100.
        want.extend([0x72, 0x63, 0x64]);
        // The Visibomb's flight (scripted.rs).
        want.push(0x1d);
        // The class holds 0x1f / 0x32 / 0x78 (scripted.rs, G-HERO-002).
        want.extend([0x1f, 0x32, 0x78]);
        // The other bodies (G-HERO-005): Clank (bodies/clank.rs) and Giant Clank (bodies/giant.rs).
        want.extend([0x43, 0x44, 0x45, 0x46, 0x47, 0x48, 0x49, 0x4a, 0x4b, 0x4c, 0x4d, 0x4e, 0x4f, 0x50, 0x51, 0x52, 0x7d]);
        want.extend(0x5a..=0x62);
        // The Hologuise disguise (bodies/disguise.rs, G-WPN-006).
        want.extend(0x53..=0x59);
        // The Hoverboard (hoverboard.rs, G-HERO-008).
        want.extend(0x6b..=0x6f);
        want.sort_unstable();
        assert_eq!(got, want);
        assert!(!implemented(-1) && !implemented(0x83));
    }

    /// Every module's ported states route to the module's own code (no state is ported in a stub module).
    #[test]
    fn stubs_own_no_ported_state() {
        for (id, s) in STATES.iter().enumerate() {
            if s.ported {
                assert!(matches!(s.module, Ground | Walk | Air | Jump | Melee | Swim | Ledge | Damage | Stance | Surface | Boots | Packs | Swingshot | Weapons | Crank | Scripted | Bodies), "state {id:#x} ported in {:?}", s.module);
            }
        }
    }
}
