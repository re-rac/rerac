# Hero-facing gameplay: crates, pickups, health, ammo, the weapon arm

Audit of the hero-facing gameplay systems (what the player picks up, spends and sees), with the game addresses
(level01 unless marked), the port status and the plan; then what was built on 2026-09-28 (§2–§5). Companion of
`hero_states.md` (the states), `moby_update_catalogue.md` (the crates and bolts), `hud_text.md` (the HUD slots),
`interaction.md` / `menus.md` (the vendor), `particles.md` and `moby_animation.md` §6.4 (pose layers).

Confidence: **H** read from the decompiler C / disassembly of the function itself; **M** from the function and its
callers; **L** inferred.

## 1. Audit

Status: **P** ported, **P\*** ported in this pass, **part** partly, **–** not ported.

### 1.1 Crates

| system | game | status | notes / plan |
|---|---|---|---|
| Bolt crate 500 (and 502 reinforced, 505 TNT): break → `SetDeathBits` → `BoltBurst` → `BoltSpawn` | `CrateUpdate` 0x2ea178, `CrateDropBolts` 0x2eb498, 0x26c250, 0x275988, 0x2bcdb8 | P | `classes/crate_.rs`. A spawner-counted dropper (+0xb1 ≥ 0) pays at least 1 bolt even with +0xb4 = 0 (H). |
| **Nanotech crate 501** | same update; init `CrateSpawnIconMoby` 0x300528 → class 806 | P (crate) / **P\*** (806) | The catalogue's names were swapped (H, from `CrateDropBolts` and 0x300de0): **501 is the nanotech crate**, its drop only clears +0xb4 and sets the death bits; the health comes from the cluster 806. The "ammo-crate death gate" (`MissionState::ammo_crate_gate`, pvar +0xf8) is therefore the *nanotech* crates appearing after N deaths to a mission's enemies. |
| **Ammo crate 511** (init: pvar +0xc6 = 100) | `CrateDropBolts` ammo branch, `0x26bff8`, `0x2daf10` | P (drop) / **P\*** (pickup) | 1 (4 in 5) or 2 pickups of pvar +0xcb's item, re-picked among the owned vendor-list items weighted to those below max. |
| Multi-hit crates 0x1fb–0x1fd, crates 503/504/506–510, crates on moving platforms | `FUN_0026f378`, `FUN_00275290` | crates on carriers: `crate_::attach_platform` / `ride_platform` (2026-09-30) | The multi-hit classes are on no level's class table (all 19 map only 500/501/502/505/511, some without 502): dead code, not ported (G-CLS-009 closed). Crates on carriers: triggers.md §5b. |
| Crate respawn (planet 0x12, pvar +0xc8) | crate state 6 | P | Reads the ammo tables (now filled, §2.3). |

### 1.2 Pickups

| system | game | status | notes / plan |
|---|---|---|---|
| Bolts 13–16: idle spin / glint, fall, settle, fly-to-hero, collect, despawn | `BoltUpdate` 0x2bb758, `CollectBolt` 0x2bc4f0 | P | Complete for Novalis. The bolt grabber (item 34, `0x13d4e2`) widens the pickup volume to 12 / 4.5: `Services::bolt_grabber` is now synced from the owned table (§2.3). |
| **Ammo pickups** (classes 204, 213, 214, 222, 223, 225, 226, 1006, 1438, 1447, 1449) | `AmmoPickupUpdate` 0x2db028 (Lombyte `FxDebrisGroupUpdateB`), states via the jump table 0x20ace0 | **P\*** | §2.1. |
| **Nanotech orbs** (cluster 806 + particle type 62, healing ring type 59, glints type 60) | `NanotechUpdate` 0x300de0, 0x300c40, 0x300cf8, 0x300d70, 0x3005a8, 0x300900, 0x300808; `PartType62Spawn` 0x288bb0 / update 0x288ca8; `PartType59Spawn` 0x288410 / 0x288530 | **P\*** | §2.2. The cluster's draw callback 0x301c00 (a glowing mesh drawn straight to the GS: 3 × `FUN_0021fda8` fans + 32 quads over effect textures) is **not ported** (counted `nanotech draw callback 0x301c00`). |
| The pickup banner (`ShowBannerf(text, n, −1)`: "+n bombs") | 0x278a50 | **P\*** | `HudState::show_bannerf`; texts from the item records +0x34 / +0x36. |
| Gold bolt 1134 (bob / spin, glow, glints, pickup cutaway) | 0x307ca0 | **P\*** | §6 (2026-09-28). |
| Infobot 750 (paths, pickup, scene → movie → scene, planet unlock) | 0x2fbf80 | **P\*** | §6. On Novalis the Infobot is the Water Pump Worker's talk-table sale. |
| Gold-weapon offers 304 | 0x2e1ac0 | P | (dialog not ported). |

### 1.3 Health

| system | game | status | notes / plan |
|---|---|---|---|
| HP 0x1415f8, max HP 0x15eda0 (4; 5 / 8 with Premium / Ultra Nanotech), HP = max at the hero init / death reload | hero init 0x226b70 | P | `Hero::health`, `GameState::global.max_hp`; the moby loop reads max HP from `GameCounters::max_hp` (synced each tick). |
| **Heal**: each nanotech cluster heals **1 HP** when its first orb reaches Ratchet (HP + clusters already on their way < max HP; 4 as Clank) | 0x300de0 state 3 | **P\*** | §2.2. Written through `HeroFields::health`. |
| Damage / knockback / death | `damage.rs` (`HeroTakeDamage` 0x226fa8) | P | |
| HUD health: shown by damage (0x226fa8), the select button (0x2aba68), menu close (0x28c6c8) and state transitions; the slot's update 0x24e238 re-arms 120 ticks on any HP change, always shown at 1 HP; init 210 ticks | `HudShowHealth` 0x24a498 | P | The port also queues the slot on a gain [L]: the game does not (the slot stays assigned, and a heal re-arms it only while it is up); kept, it is the result the player sees when the slot was shown at the level start. Verified on screen (§6). |
| Nanotech upgrades (Premium / Ultra, the Orxon seller 1326, records 40 / 41) | talk system | – | Level 10; the purchase must set flags [4]/[5] and max HP 5 / 8. Queue with Orxon. |
| Clank's own health 0x1415fc | | – | With the Clank levels. |

### 1.4 Ammo

| system | game | status | notes / plan |
|---|---|---|---|
| Per-item ammo 0x13d428, max ammo (record +0xe), pickup amount (+0xc) | item records 0x1c4530 | P | Mirrored in `Weapons::ammo` before the tick, copied back after. |
| Use on fire (`0x249450`, used stat 0x13dea0) | | P (Bomb Glove) | The other weapons' fire code is not ported (§1.6). |
| **`AddAmmo` 0x2494d8** from pickups, picked-up stat 0x13de08 | | **P\*** | `pickup::add_ammo`, `HeroFields::ammo` / `ammo_picked`, `GameState::global.ammo_picked_up`. |
| Vendor ammo purchase (AddAmmo, bought stat 0x13dd70) | vendor menu | P | `menus/vendor.rs`. |
| HUD weapon / ammo slot (`%d/%d`, red at 0) | 0x24f9c0, 0x2519c0 | P | Follows the game state, so pickups and purchases show. |
| Ammo in the crates' pick (owned list, max) | 0x26bff8 | **P\*** | The engine's moby loop had an empty inventory (every ammo crate would pick item 10 blind); now `ItemTables` from the price records, the vendor list and Ratchet's mirrors. |

### 1.5 The weapon arm (upper-body layer)

| system | game | status | notes / plan |
|---|---|---|---|
| Pose-layer nodes (6 × 0x40 at 0x18efc0), alloc / free, start, advance | `FUN_00263e08`, `FUN_00263ec8`, `FUN_002641c0`, `FUN_00263f70` | **P\*** | `hero::anim::AnimLayer`. |
| Layer decode and blend in the evaluator (`MobyProc` +0x60 list) and in the chain evaluator | `FUN_00267770`, `MobyAnimEvalChain` 0x268ee8 | **P\*** | `rc_formats::moby_anim::{evaluate_layered, evaluate_chains_layered}`. |
| The weapon draw (arm layer on joint list 12, + list 13 when 0x1413fb = 2), upkeep, put away, holster | `0x22ee08`, `0x22f068`, `0x22efd8`, `0x2405f8` | **P\*** | `hero::weapons`. §2.4. |
| Persistent-arm weapons (item +0x30 ≠ 0: the Blaster-type weapons keep the arm up): layer re-creation, standing pose, idle rules | `0x22eca0`, `0x242858`, idle transitions 0x242930 | **P\*** (§7) | `weapons::{arm_on_state_change, gun_stance, stance_kept, idle_stance}`. Fixed 2026-09-28 (§11): `0x242858` is called by the stop 3 (SetState and physics) and the walk's slow stop only, not by the walk's SetState, so Ratchet runs off from the stance while firing. |
| **The glove-holding layers** (0x140050 / 0x140054, while the hand item's def +0x18 = 0x1413fb is set: list 12, and list 13 with 2): Ratchet's key replayed from the holding classes 1 / 2 on his arms | `0x22e660` (in `0x22f390`), `FUN_00264220` (the retargeted key), `0x22df10` (the special poses) | **P** (§11) | `weapons::hold_update`, `anim::{HoldClass, pose_layers_with}`, `rc_formats::moby_anim::{AltKeys, retarget_map}`. Not ported: the joint records 8 / 9 angles, the weapon lowered at walls `0x22b700` (G-WPN-011). The face layer `0x22e3a8` (list 0x18): `weapons::face_update`. |
| Comet-Strike (0x15) | `0x2be1c0`, 0x236da0 | P | A full-body state: the wrench code never calls 0x22ee08 (xrefs: HeroPdaGadget and the weapon updates 0x2c7d68, 0x2cd458, 0x2ce448, 0x2d2450, 0x2e4e60, 0x303000). No layer to wire. |

### 1.6 Other hero-facing gaps (listed, not done)

| gap | game | plan |
|---|---|---|
| **Pyrocitor 16** (class 176, 0x2cd458; sold on Novalis): **ported (§7)**. **Blaster 15** (168, 0x2ca610), **Devastator 11** (157, 0x2c7d68), **R.Y.N.O. 23** (454, 0x2e4e60), **Tesla Claw 19** (177, 0x2ce448): **ported (§9)**. The other weapons (ids from the level01 item table 0x179f40, §8): Suck Cannon 9 (849, 0x303000), Visibomb Gun 13 (163, **ported §17**), Morph-o-Ray 21 (185, 0x2d2450), Mine Glove 17 (190), Glove of Doom 20 (229), Drone Device 24 (483), Decoy Glove 25 (562) …: their `HAND_ITEMS` rows, fire cases, projectiles and arm sequences | weapon updates listed above, `HeroPdaGadget` 0x240ed8 | One weapon at a time on the arm layer built here. |
| ~~Auto-aim target list 0x1abe80~~ | | Done: §8 (the list and the glove's search), §9 (the Blaster's, the Devastator's, the R.Y.N.O.'s and the Tesla Claw's searches). |
| The melee aim assist 0x22e238 over the target list | | With the wrench's combat pass. |
| 0x20 (Walloper lunge, item 0x12), 0x21 (wrench rebound off flag-2 targets) | hero_states.md §1.2 | |
| ~~Gold bolt 1134, Infobot 750~~ | 0x307ca0, 0x2fbf80 | Done: §6. |
| The nanotech cluster's glowing mesh (draw callback 0x301c00) | 0x301c00, `FUN_0021fda8` | A small custom renderer (effect textures 0xb and gp−0x4dbc, 290-vertex fans). |
| Cheat 6 (0x15edb6) nanotech extras (0x13f510 timer, pickups at full health) | 0x300de0 | Only with the cheats menu. |
| Nanotech upgrades, Clank's health | §1.3 | With their levels. |

## 2. In the port (2026-09-28)

Code: `crates/rc-game/src/moby_update/classes/pickup.rs` (registry `classes/mod.rs`), `particles/type59.rs`,
`particles/type62.rs`, `hero/anim.rs` (`AnimLayer`, `AnimCtl::{key_rate, eval_chains_with}`), `hero/weapons.rs`,
`rc_formats::moby_anim::{PoseLayer, evaluate_layered, evaluate_chains_layered}`; engine wiring in `gameplay.rs`
(inventory, max HP, bolt grabber, stats, the layered palette) and `hud_render.rs` (the banner). Written on standard
`f32` (the pickups, the layer bookkeeping); the evaluator's layer blend uses the evaluator's existing PS2 float helpers
(consistent with the code around it). RNG draws are the game's, in its order.

### 2.1 Ammo pickups 0x2db028

Pvars +0x00 amount, +0x04 (dropper's +0xc4), +0x06 dim timer, +0x08 item, +0x0c, +0x10 velocity, +0x20 pickup delay
(`ticks(16)`), +0x24 rise speed, +0x28 spawn z. **H** (disassembly + jump table 0x20ace0: 0 → 0x2db798, 1 → 0x2db1ec,
2 → 0x2db4d0, 3 → 0x2db65c, 4 → 0x2db7a4).
* Physics (states 1, 3, 4): moving → `coll_sphere(pos + 0.5 up, 0.4, flags 2)` pushes it out (`CollOutput +0x30`,
  then −0.5); gravity `4.9·dt²` before and after `pos + vel` (the midpoint rule); `CollLine_Fix(pos + 0.1 up → next,
  flags 2)`: a hit lands it (velocity 0, position = hit point).
* State 1: alpha → 0x80 by `128/ticks(30)`; after the delay, at rest: collected when Ratchet's pickup volume (the
  bolts' `0x1415d8` / `0x1415dc`, [`World::bolt_radii`]) holds it and the ammo is below max, or when it fell 2 below its
  spawn height; never with 0 HP. Then `AddAmmo`, the banner (`+0x36` text for 1, else `+0x34`), the picked-up stat,
  state 2, scale ×1.2, rise `8·dt`, sound 0 of class 213 (at most every `ticks(10)`, gp−0x5398). Otherwise alpha
  +1 (≤ 0xff) and, when Ratchet's bounding sphere overlaps its own by 0.2 (`FUN_00265210`: the gap of the ×1024
  spheres), state 3 with `ticks(60)` at +0x06.
* State 2: rise speed → 0 by `24·dt²`; the flight speed `|vel|` → `64·dt` by `24·dt²` toward the body point
  0x13f420; deleted at scale 0 or within one step; scale → class scale × `clamp(d / (ticks(30)·speed + 0.01), 0.3,
  1)` by 10 % of the class scale a tick.
* State 3: alpha pulses to `10 + trunc(5 + 5·sin(2π·(counter % 60)/60 − π))`; back to 1 when the spheres no longer
  overlap or the ammo drops below max. State 4: scale grows 3 % a tick to the class scale, then state 1 with its
  collision.

### 2.2 The nanotech cluster 806 (0x300de0)

**H** for the states and constants (decompiler C + .lit 0x161d60..0x161e5c: turn 4°, bob 0.06 at 2°/tick, radius
0.25 ± 0.15 with the steps 0x1fbe00, orb 60000 / trail 10000, colour 0x7f7f4040, heal ring hues / sizes / heights,
glint colours); **M** for the rotation direction of `FUN_00274ac8` (right-handed axis-angle, a mirror would only
mirror the orbit), see the module doc of `pickup.rs` for the state machine. Particle types: 62 (orbs kind 1 with no
update, trails kind 0 / kind 2 attached through `Particles::anchors`), 59 (the ring, attached to the hero), 60.

### 2.3 The engine's item tables

The moby loop's `World::inventory` was the empty default, so the ammo crates picked blind and the level-18 crate
respawn rule saw no items. Now `pickup::ItemTables` (price records `0x1c4530` from the talk tables, the vendor list
0x15edd0, Ratchet's owned / ammo mirrors); max HP 0x15eda0 into `GameCounters::max_hp`; the bolt grabber flag
(owned item 34 = 0x13d4e2) into `Services::bolt_grabber`; the pickups' ammo and picked-up stat back through
`HeroFields` → `Weapons::{ammo, picked}` → `GameState`.

### 2.4 The weapon arm layer

`0x22ee08` (moving, not standing): for list 12 (and 13 with 0x1413fb = 2) a node with weight 1, key A = Ratchet's
current key B, key B = (`def +0x28`, or `+0x2c` crouched, frame 0) over `ticks(10)` / `ticks(11)`, advanced once. The
upkeep `0x22f068` (first thing in `HeroItemsUpdate`): re-blend over 8 when the item's layer sequence changed, advance
(t += speed · rate; key steps take the new key A's rate; the wrap sets flag 2), weight → 1 by 0.2; on the wrap with
0x1413fa clear: 0x1413f8 = 0, speed 0, fade 0.07 a tick (0.25 after the holster's 0x140064 = 2) and free at 0. The Bomb
Glove: moving sequence 66, crouched 44 (standing throws use the state 0x23 with the full-body 0x2c). The evaluator
blends each listed joint `q = nlerp_flip(q, q_layer, 1 − w, w)`, the translation (the layer's rest where its keys have
none) and a layer inherited scale by `w` (§6.4), in `MobyProc` and in `MobyAnimEvalChain` (the hand item follows the
arm: `AnimCtl::eval_chains_with` in the hand attach and the wrench's hand point). The main animation is untouched.

## 3. Tests

* Unit: `particles::type62::tests`, `particles::type59::tests`, `pickup::tests` (the cubic, the rotation sense, the
  item tables), the existing `weapons::tests` (the arm while running, now on the layer).
* `crates/rc-game/tests/weapons/hero_gameplay_novalis.rs` (full ticks: the scheduler, particles, moby collision, the hero with
  the wrench and the glove, the hit path): the bolt crate 376 pays bolts; the nanotech crate 533 (on 377) shows 8 orbs,
  heals 2 → 3 and 3 → 4, and does nothing at 4 / 4 (the free cluster waits); the ammo crate 552 fills the glove 38 → 40
  (capped) and leaves its pickup at 40 / 40; a glove throw while running at the spawn plays sequence 66 on the arm
  layer at full weight while the hero stays in the run state on the walk / run animation, uses one bomb, fades and
  frees the layer; the layered palette differs from the plain one only in the arm list's joints and their children.
  Every scenario runs twice with identical rows (RNG included).
* Guards: `novalis_hero_digest` unchanged (the digest cuts the hero dump before `owned`; no hero state or RNG changes
  on those scripts); `cargo test --workspace` green.

## 4. Verified vs inferred

* Verified (H): the crate identities (501 nanotech, 511 ammo), every state and constant of 0x2db028 and 0x300de0,
  `AddAmmo`, the pickup volume shared with the bolts, the heal of 1 per cluster up to max HP (4 as Clank), the layer
  node layout and its start / advance / upkeep / fade rules, the layer joints = the second byte list of Ratchet's
  joint list 12 (count byte `pb[2]`), both evaluators applying the +0x60 list.
* Inferred: the quaternion rotation sense of `FUN_00274ac8` (M); the layer's scale channel default (L: the decode
  fills 0x7f7f; a layer without an inherited scale leaves the main one); the HUD health re-show on a gain (the port's
  extra queue, [L] as before).

## 6. Gold bolts and Infobots (2026-09-28)

Code: `crates/rc-game/src/moby_update/classes/gold_bolt.rs` (1134, `GoldBoltUpdate` 0x307ca0 with 0x308380 / 0x308470 /
0x308550 / 0x3087e0 / 0x3089f0), `classes/infobot.rs` (750, `InfobotUpdate` 0x2fbf80 with 0x2fbb20 / 0x2fcd88 /
0x2fce68); the module docs carry the pvar layouts and state machines. Both are registered by class number and run on
every level whose class table names the same function (`LevelPorts`): the gold bolt on levels 1–18 (17 overlays byte
for byte, Gemlik's up to relocations), the infobot on 0, 1, 3–8, 10, 12–15, 17 (checked by
`tests/classes/gold_bolt_infobot_novalis.rs::registered_on_the_levels_with_identical_code`). Placement comes from each
level's instances: 40 gold bolts on the disc (none on 00), pvar +0x00 = the index in the level's 4 bytes.

**Reused**: the script camera and the cinematic call layer (`CameraScript` mode 1 snap + targets, `CameraScript2(0)`,
`HeroTeleport(…, 0x72)`, the letterbox 0x15f404), the hero hold state 0x72 and `HeroCall::SetAnim`, particle types 59
(glow) and 60 (glint) with the nanotech ring's glow pattern, `creature::{turn::spring_turn, ground::key_time}`,
`spline::nearest` (the infobot's path aim), the checkpoint record, the talk system (the Novalis Infobot sale), the
scene / movie hand-offs, the HUD banner (`HudState::show_banner`), `GameState::unlock_planet`, the saved-game write
channel `interact::GameWrite`. **New**: the two classes; `cinematic::{fade_to_black, show_banner, show_planet_banner,
unlock_planet, save}` with `Cinematic::banner` (`ShowBanner` / `ShowPlanetBanner` 0x277c38, message table level01
0x20a0c0 = `PLANET_BANNERS`, `ticks(1180)`); `EngineRequest::FadeToBlack` and the engine's `scene_render::FadeHold`
(the blocking `FadeToBlack(n)` 0x21b438 of a gameplay class: the n frames after the asking tick fade the last view to
black, tick suspended); `HudState::show_banner_msg`; the hand-item hide 0x1413ff (`Hero::f13ff`, set through
`HeroFields::hide_hand` after the hero calls, cleared by `SetState` on foot, read by `moby_attach`).

**Game state (in memory, laid out as the save).** There is no save/load flow, menu or travel yet; everything goes into
`GameState` (Persistent), in the save chunks' own layout, so a future save writer only serialises it:

| field | game address | save chunk | written by |
|---|---|---|---|
| `levels[l].gold_bolts[i]` (u8, 1 = collected) | 0x14bec0 + l·4 + i | 3003 (level slot) | gold bolt pickup: `GameWrite::GoldBolt { level, index }` |
| gold bolt count | Σ of those bytes | – | derived (`TalkGame::gold_bolts`; the gold-weapon offers' condition 6 spends it: `spendable_gold_bolts`) |
| `global.planet_unlocked[p]` | 0x13dd40 | 14 | `cinematic::unlock_planet` → `GameWrite::UnlockPlanet` → `GameState::unlock_planet` |
| `global.map_order[n]` | 0x13d510 | 20 | same (appended) |
| `levels[l].missions[m]` (0xff done) | 0x14c050 | 3004 | `SetMissionDone` (infobot +0xb0, talkers, mission NPC) |
| `global.landmarks[slot].flags` (talked) | 0x13d5bc | 15 | talk system; the infobot's `FUN_0027b438(1)` on its path start |
| `global.bolts` | 0x15ed98 | 7 | the purchase (500 for the Novalis Infobot) |

The moby loop reads the mirror `TalkGame` (`gold_bolt_bits`, `planet_unlocked`, …, synced before every tick and
before the load pass), so "stays collected" holds for the session: a revisit or reload runs the bolt's init, which
deletes a collected one. `memcard_Save` is logged (`EngineRequest::Save`). Rewards: the gold-weapon offers (class 304,
kind 6 nodes) already read the count; spending is the offer's existing `GoldWeapon` write.

**The flows (level01).**
* Gold bolt: idle (bob 1 ± 0.3, spins 120 / 40 / 20 °/s, 4 soft glows, glints 1 in 40) → Ratchet within 3 (xy) / 2
  (z) → `FadeToBlack(ticks(10))`, Ratchet 2.5 in front facing it (0x72), sound 0, the camera snapped beside it, the
  bolt on Ratchet (sequence 1), his animation 0x82, hand item hidden, letterbox → the camera orbits 40° about him
  (or moves between the two cuboids of pvar +4 / +8) while sequence 1 plays → 60 ticks → banner 21427 "Gold Bolt
  acquired" (180 ticks), the byte, control back, `CameraScript2(0)`, save, deleted. pvar +0x0c ≠ 0 (one bolt on
  level 16): banner, byte, save at once.
* Infobot: see `infobot.rs`; state 8 = `UnlockPlanet(+4)` + `SetMissionDone` + checkpoint + scene +0xc → movie +0xd →
  scene +0xe (each after the last ends) → save + `ShowPlanetBanner(+4)`. Novalis' instance 874 is inert (hidden, no
  path, no scenes). **Novalis' Infobot** is the Water Pump Worker 774's: "△ Buy Infobot for 500 bolts" (talk kind 1,
  item 37, purchase + chain) → scene 1 → movie 2 (`mpegs[4]`) → scene 2 → `UnlockPlanet(2)` + `ShowPlanetBanner(2)`
  ("Infobot for Planet Aridia acquired", 1180 ticks), the worker leaves. The mission NPC's Kerwan reward
  (`UnlockPlanet(3)` after scene 3 → movie 3 → scene 4) now goes the same way.

**Tests** (`crates/rc-game/tests/classes/gold_bolt_infobot_novalis.rs`, headless, game state of a first arrival): both
Novalis camera variants (959 orbit, 958 cuboids 0x51 → 0x50): picked up on the tick Ratchet is in reach, idle bob
within spawn + 1 ± 0.3, FadeToBlack(10), 0x72 + animation 0x82 + hand hidden, bolt on him, letterbox and script
camera for the whole 398-tick cutaway, then control back, banner 21427 / 180, save request, count +1, chunk-3003 byte
set, glows and glints spawned; a revisit with that state has no bolt 0 (the others idle); determinism; the Infobot
purchase: hand-offs scene 0, scene 1, movie 2, scene 2 in order, bolts 5000 → 4500, planet 2 unlocked and in the map
order, the planet banner; the Novalis 750 inert; the per-level registry. Updated: `hero_gameplay_novalis`'s ammo-crate
check no longer assumes one pickup (the crate's `randi(5)` moves with the RNG stream now that the gold bolts draw).

**Engine** (frame-exact, `RC_SCENE=0`, scratchpad `goldbolt_infobot/`): `RC_HERO_AT=244.05,107.90,87.5,-2.101
"RC_PLAY_SCRIPT=40-80:stick 0 -1"`: idle glow (frame 50), pickup on frame 89, fade frames 89–98 (94: the last view
at ~60 % black), cutaway (150: the bolt over Ratchet, bars, no HUD; 300: he holds it, no wrench), banner "Gold Bolt
acquired" with control back (520); two runs identical (frames 150, 520). Infobot: `RC_HERO_AT=253.13,186.67,95.57,
2.5066 RC_GIVE_BOLTS=5000 "RC_PLAY_SCRIPT=2470-2472:press TRIANGLE" RC_MOVIE_SKIP=120`: scene 0 → △ at 2470 →
scene 1 (2473) → movie 2 (3336, holofilm) → scene 2 (3462) → UnlockPlanet(2) at tick 4207, the banner "Infobot for
Planet Aridia acquired" with 4500 bolts (4420).

**Verified vs inferred.** [H] every state, constant (.lit 0x161f10..0x161f74, 0x161c80) and call order of the two
updates and their helpers; the planet banner table and `ticks(1180)`; `ShowBanner(−1)` = `ticks(180)`; the Novalis
data (surveyed). [M] level 13's gold bolt = the port (relocation match, not byte identical); `DialogStreamStart` /
`DialogStreamUpdate` taking the pvar bytes +0xc / +0xd / +0xe (argument registers lost in the decompile, menus.md
§6). Port choices: the fade hold shows the last view with the asking tick's world under it (Ratchet already
teleported, the bolt on him) and holds its HUD / bars; the infobot ride's platform-matrix carry (0x2752c0 with a
mode-0x20 ride of class 822) is counted, not ported; the gold bolt's cheat mirror (0x15edb5) and control mode 3's
`FUN_00231450` are not ported. Not engine-checked: other levels' gold bolts / infobots (headless smoke and registry
only), the infobot's path / ride states (no Novalis instance uses them).

## 7. Joint modifiers, the Pyrocitor, the scene body (2026-09-28)

**The runtime joint-modifier list (moby +0x64).** [H: `MobyProc` 0x267fc0 at 0x268b00, `MobyAnimEvalChain` 0x268ee8,
`AttachManipulator` 0x264370, `DetachManipulator` 0x2643e8, `FUN_00227050`, `FUN_00227590`; RAM at the Novalis idle
savestate] A node (`u8 mode +3, target record +4, next +8, f32 weight +0xc, quat +0x10, scale +0x20, trans +0x30`) is
linked in front of the list for one of the class's joint lists; it acts on that list's **second byte list's first
joint** (`pb[pb[0] + 4]`). After the pose layers: mode 0 `q ← q ⊗ q_m`, scale xyz `∘ s_m` (its presence word kept;
the chain evaluator marks it present), translation xyz `+ t_m`; mode ≠ 0 a weighted nlerp (and lerps) toward the node.
MobyProc skips nodes at or past the class's joint count. Ratchet's users on foot:
* **the joint records 0x17ab00** (31 × 0xb0; `0x2273d0` → `0x227050` every idle update): a record whose targets / scale
  are set or whose angles have not settled is (re)attached, mode 0, quaternion `FUN_0026ee30` from its Euler angles
  (`FUN_00221e38` = rotation by −a about one axis, composed x ⊗ y ⊗ z), scale = the tick's +0xac; otherwise detached.
  Their writers: the head look (records 1 / 3 / 12), the idle secondaries (13..16), record 17's head scale (0.92),
  and **`HeroLean` 0x235638** (records 0..3, 13..16 from the turn residual 0x13f4d8: walk / run, fall and jumps, the
  pack glide 8 and the Thruster hover 0x81; its spring constants for records 0..3);
* **the blink's eyelid nodes 0x140080** (7 × 0x40, `0x227590`): lists 15..21 (0x17c640), mode 1, fixed poses
  (0x17c4c0 / 0x17c530 / 0x17c5a0), weight `BLINK[frame]`, all attached during a blink (node 0 first, so node 6 heads the
  list), all detached at its end.
In the port: `rc_formats::moby_anim::{JointModifier, evaluate_posed, evaluate_chains_posed, list_target}` (native `f32`
on the record values), `Moby::joint_mods` (the list, written by the owner; every pose evaluation reads it: the palette,
the dynamic mobys, the shadows, the hand / back attachments), `hero::idle::{Manip, Idle::manips, Idle::modifiers,
BLINK_NODES, axis_quat, euler_quat, Hero::lean}`, `Hero::joint_targets` (the loader's list targets) and the
write-back. **RAM match** (`shadow_volume_novalis.rs`, the fixture now carries the savestate's 11 nodes): the port's
nodes equal RAM's (quaternions to 2e-6), and **all 21 posed shadow records** match to **3.05e-5** units (the 10 under
the list 1.53e-5; 0.178 without the list), radii 1.5e-8. Not ported: the swim lean (0x2370b8 cases 0x31, 0x33..0x35),
the Magneboots aim lean `0x2352e0`, the pack record 18 `0x235e60`, `HeroScanTargets`' look target, the cheats' big
head `0x24a1d0`, Clank's eyelids `0x2278c0` (Clank's pose has no modifier input yet), and the NPCs' own manipulators
(vendor 11, talking NPC 774: their classes compute the angles; `Moby::joint_mods` is where they would go).

**The Pyrocitor** (item 16; `hero::pyrocitor`, the `HAND_ITEMS` row; module doc for the whole update) [H: disassembly
of 0x2cd458, 0x2cde98, 0x2ce0a0, `PartType12Spawn` 0x280138 / `PartType12Update` 0x280408, 0x2d0fc8, 0x2d1068]: no case
in `HeroPdaGadget`; its update fires on ○ held: `0x22ee08` (the stance 51, full body; def +0x28 = −1, so no moving arm
layer: running, Ratchet keeps his run and the flame leaves the hand along his facing), flames (particle type 12,
`particles::type12`: 2 glow puffs + 2 flames a tick, 12 units, cut 0.75 before walls), the kept flames' hit spheres
and embers (type 2), two nozzle spheres, one ammo per 10 ticks, the flickering point light (7.5), the loop sound, the
pilot flame (class 179, `classes::pyro_glow`), first person from below the eye along the view. The persistent-arm rules
(def +0x30): `0x242858` (the stop 3's SetState and physics, or the walk's slow stop, turn into the stance; §11), `0x22eca0` (arm layers when
leaving idle), the idle keep / stance, `0x22efd8`'s return to idle. The hand item is not a table moby in the port:
hits are Ratchet's, the pilot flame reads its owner's state from pvars; an item update's standing `SetAnim` is made
right after the slot loop (`Weapons::pending_*`). Gold weapons (0x13e520) are not mirrored (0). Not ported: the hold
statistics / "hold ○" help (`0x2cde98`), the moby texture scroll of class 179, the grind aim rows (0x22f068 group 0xf).

**The scene body 99 / 100** (`hero::scripted`) [H: SetState case 99/100, `0x2370b8` case 99/100, no `0x242930` case]:
group 0x18, 0x1413fc = 1, the weapon put away, with `play` the idle sequence on eased curve 1; the physics zeroes the
velocity; nothing leaves it but `SetState(0, 1)`. The gameplay Ratchet is never visible in it (the scene hides him and
his items, `FUN_002486c0`, and draws its own actor; the vendor hides him too), so this is all of it.

**Tests**: `moby_anim::modifier_tests` (3), `idle::tests::{modifier_list_order_and_blink, euler_node_quaternions,
running_lean_targets}`, `type12::tests` (3), `pyrocitor::tests`, `pyro_glow::tests`, `scripted::tests`,
`shadow_volume_novalis` (the RAM match), `hero_pyrocitor_novalis.rs` (the purchase's `GiveItem`, standing / running
at a crate (broken by the flames; not without them) / first person, each twice identical). `novalis_hero_digest`: with
`RC_HERO_DIGEST_NO_IDLE=1` (the idle block cut) byte-identical to before; the full digest differs only in the idle
block (the new records 0 / 2, the list, the lean's targets and angles while moving).

## 8. Targeting, the Bomb Glove's reticle, one hand item on screen (2026-09-28)

**Item ids (correction to §1.6).** The name ids of the item definitions (+0x00, strings 20034..) give: 9 Suck Cannon
(class 849), 10 Bomb Glove (192), 11 Devastator (157), 12 Swingshot (208), 13 Visibomb (163), 14 Taunter (175), 15
Blaster (168), 16 Pyrocitor (176), 17 Mine Glove (190), 18 Walloper (180), 19 Tesla Claw (177), 20 Glove of Doom (229),
21 Morph-o-Ray (185), 22 Hydrodisplacer (1251), 23 R.Y.N.O. (454), 24 Drone Device (483), 25 Decoy Glove (562), 26
Trespasser (188), 27 Metal Detector (585), 31 Hologuise (483, the Drone Device's class), 32 PDA (619). §1.6's "Suck
Cannon 11 (157)" / "Devastator 19 (177)" are the Devastator and the Tesla Claw. **H** (level01 item table 0x179f40,
name ids against the string order).

**Two hand items on screen.** Root cause (engine side): `rc-engine` keeps an entity set for every gadget class (and
the three packs) and shows the one the game's slot holds; the vendor's exit (`interact_render::hide_hero`) and a
scene's end (`scene_render`) put `Visibility::Inherited` on every `AttachedTo` entity, and `moby_attach` only
re-inserted visibility when its own flag changed, so after the first vendor visit or cutscene every gadget class stood
at the hand (the Hologuise / Drone Device model, class 483, the most visible) and the unworn Thruster- / Hydro-Pack on
the back. The game has no such entities: `HeroItemsCreate` 0x22f3c0 creates only the slot's moby and `0x2305e8`
deletes it on a swap; `VendorExit` / `FUN_002487a8` clear the hide bits of the mobys that exist. Fix:
`moby_attach::keep_absent_hidden` hides, every frame, the entities of every item the game does not have
(`absent_entities`), after those show-again passes; the hand's choice is `hand_shows` (the slot's class only). Not a
game-data issue: the item → class table is right (items 24 and 31 share class 483 in the game's own table).

**Targeting** (`rc_game::targeting`, module doc for the table of readers). **H** from the level01 decompiler output
and disassembly of 0x265548, 0x2711f8, 0x2d8330, 0x2c2be0 (disassembled in full), 0x2c23c0, 0x2c3300, 0x22c080,
0x22e238 / 0x22dff0 / 0x2351d0, 0x2c7d68:
* `0x1abe80` = the targetable (mode 0x1000) mobys of the run list, in order; each target's record = the pvar
  block's first word (mode 0x20), `+0x10` the aim point's height. In the port: `targeting::target_list` over the
  moby system's run list (`tick::Game::target_list`, handed to the items as `ItemEnv::targets`), `aim_height`.
  Crates are targets too (they have records): the glove aims at crates as the game does.
* Every reader has its own selection code; the glove's (`targeting::BOMB_GLOVE`): range 15 (2D, from the launch
  point), 45° of the reference yaw (90° within 4), elevation < 55°, rise < 2, a clear world line from the camera
  (flags 6); greedy (an accepted target becomes the reference yaw and the new range). Only without L1 / L2 (or with
  0x1413fc); in first person the camera aim of before. The chosen target (0x13fda0, `Weapons::aim`) replaces the
  point 8.5 ahead as the arc's target (`launch_velocity` unchanged), and SetState 0x23 turns Ratchet to it.
* The landing preview 0x2c2be0 runs in the bomb's update (`bomb::aim_preview`, `targeting::arc_landing`) while held
  and while flying (until the glove holds a new bomb, +0x56): the first line from Ratchet to the start, then 10-tick
  steps (`p += v; v.z −= 11·dt²`) with a line test each (`0x10`), at most 300 ticks (the fuse when flying); a world
  face ends it; a moby's collision primitive snaps it to the moby's position (Ratchet only after 10 ticks; only while
  the arc is less than 2 above its start); water only while falling. A snap's `GroundHeight(0.5, pos + 0.5 up)` is
  called for its side effect only: its line leaves the ground face's normal in the collision output the preview stores,
  so a snapped reticle lies flat under the enemy. The lock bookkeeping (+0x30 / +0x40 / +0x60 / +0x64, let go dead,
  untargetable or after 30 ticks without a snap) is kept in the bomb's pvars. The point is pulled 5 % toward the
  camera and the reticle registered (`Services::reticles`, list 1).
* The draw 0x2c23c0 (`targeting::reticle_quads`, `rc-engine` `reticle_render.rs` on crate::fx_draw's material): two
  2×2 quads, FX 0x11 (three arcs) and 0x12 (three ticks), colour 0x80808080, ALPHA 0x48, turned by `(0, ±spin, 0)` then
  by the Euler angles `(π/2 − atan2(n.y, √(n.x² + n.z²)), atan2(n.x, n.z), 0)` of the normal, spin = 2π·(counter mod
  180)/180, opposite ways; not in game mode 2.
* **Which weapons**: the reticle is the Bomb Glove's only (0x2c2be0's one caller is the bomb 0x2c3300, gated on the
  owner's class 0xc0; nothing else registers 0x2c23c0). **Corrected 2026-09-28 (§13):** the mine 74 (`0x2bfa78` +
  draw `0x2bf420`), the decoy 203 and the Glove of Doom's canister 230 (`0x2d9760` / `0x2de0a8` + draw `0x2d8e28`) run
  copies of the preview and of the draw (same quads and constants); the decoy's and the canister's never draw in
  practice (their first update marks them as replaced), the mine's does. The Drone Device has none; the Pyrocitor
  reads no target list. Plug-in rows for the other readers (`AimRules` or their own
  search over the same list): the melee aim assist (range 11, cone 50°, score `d + diff·d`, +7 for a record flag), the
  Devastator 0x2c7d68 (hard lock within 2.5 at 60° / 45°, else a 10° cone (+40° gold) narrowed by the record's
  radius byte +0x0a, camera line flags 6), the head look 0x22c080 (range record +0x38, 110°, 60°, priority +0x39), and
  0x2cc830 / 0x2cf138 / 0x2d2018 / 0x2d49f8 / 0x30d308 / 0x2bfe40 (lures, a two-target scorer, the mines of class
  190) — not ported.

**Native, not emulated.** `f32` with `atan2` for `FastArcTan` and the moby Euler rows for `fun_001fa050`; the
reticle is two ordinary textured quads through the shared callback material; no PS2 arithmetic. **Inferred**: the
target list is rebuilt from the run list after the moby loop (the game builds it at the loop's start: a moby that
toggles 0x1000 in its own update is seen one tick early); the glove is not a table moby in the port, so the preview's
"owner" exemption covers Ratchet only; the GroundHeight line's endpoints' x/y (the decompiler dropped them: taken as the
moby's); the moving-platform variants (0x13f64c, never set in the port) are not wired.

**Tests**: `targeting::tests` (6: the list and record, the glove's rules, landing, snapping, the reticle's plane /
size / spin, the pull and the per-tick list), `moby_attach::tests` (2: one hand entity set per equipped item, absent
items hidden after a show-again across swaps), `hero_targeting_novalis.rs` (the glove targets a critter, the reticle
snaps onto it, the bomb lands where it was; twice identical). `novalis_hero_digest` is unaffected by construction (its
runs have no hand item data, and the new `Weapons` fields sit after the digest's cut).

## 9. The gun-family weapons: Blaster, Devastator, R.Y.N.O., Tesla Claw (2026-09-28)

**Which update is which** (level01 class table `lvl.vtbl` against the item table 0x179f40; the other levels run the same
code by `LevelPorts`): Blaster **15** / class 168 → `0x2ca610` (search `0x2ca310`, shot class **305** `0x2e1cc8` /
`0x2e2170` / `0x2e2a18`); Devastator **11** / 157 → `0x2c7d68` (missile class **153** `0x2c5440` / `0x2c5b70`; the
gold re-target `0x2c5778`); R.Y.N.O. **23** / 454 → `0x2e4e60` (search `0x2e4bb8`, missile class **457** `0x2e5738` /
`0x2e5a48`); Tesla Claw **19** / 177 → `0x2ce448` (`0x2cef78`, `0x2cf138`, draw callback `0x2d05d8` / `0x2d0748` /
`0x2d0d18`). The other xrefs of `0x22ee08`: `0x2d2450` is the Morph-o-Ray (21 / 185), `0x303000` the Suck Cannon
(9 / 849). The item defs (+0x18 / +0x24 / +0x28 / +0x2c / +0x30): Blaster 1 / 56 / 57 / 62 / 1; Devastator 2 / 54 / 73 /
73 / 0; R.Y.N.O. 2 / 81 / −1 / −1 / 1; Tesla Claw 1 / 51 / −1 / −1 / 1. Only the Blaster has a weapon-check case
(`HeroPdaGadget` case 0xf); the others fire from their updates, like the Pyrocitor. **H** (decompiler output, the
disassembly where the decompiler dropped arguments; data read from the overlay).

**Code**: `crates/rc-game/src/hero/{guns, blaster, devastator, ryno, tesla}.rs` (the item updates, their `HAND_ITEMS`
rows), `moby_update/classes/{blaster_shot, devastator_missile, ryno_missile, missile}.rs` (the projectiles),
`particles/{type21, type27, type44, type72}.rs`, `targeting.rs` (`cone_miss`, `polar`, the record readers, the screen
markers); engine `marker_render.rs` (the 2D markers, into the HUD pass) and `tesla_render.rs` (the beam, on
crate::fx_draw). Each module's doc carries its pvar layout, state machine and constants with their addresses.

**What each does** (module docs for the detail):
* **Blaster**: ○ draws (the persistent arm: stance 56 / layer 57 / crouched 62); while the arm is out and 5 ticks after
  the draw, a shot every 6 ticks (one ammo each), 40 u/s from the item's joint list 0 (first person: 0.15 right, 0.15
  down, 0.75 ahead of the eye along the view); its search takes the nearest targetable moby within 20 whose aim point is
  within 9° of the aim (less the record's radius), with clear camera and muzzle lines; a target's pitch is clamped to
  ±10° of the aim, the shot homes on its aim point (spring turns, 270°/s). A 0.42 hit sphere at the muzzle, five
  sparks and a flash (type 27), a grey light 15 ticks; the item's firing sequence 4 whose loop sound (class sound 1) is
  the gun's sound. The shot (305): a glow (type 26) and a 17-sprite trail (type 72) it places each tick; damage 0.25
  (flags 0x10001) through the path test; sparks and smoke (types 27 / 23) where it ends; 35 ticks, 64 from the camera.
  Markers: green (FX 0x26) over the target; red crosshair in first person.
* **Devastator**: ○ held (35-tick lockout, one ammo a missile); standing (or walking slowly) Ratchet plays the shot 54,
  else the two arm layers 73; its search takes creatures (class type 5) within 10° (+40° gold) of the aim (less the
  radius), a creature within 2.5 and 60° of Ratchet at once; a creature no missile holds becomes the lock (20 ticks).
  First person: a ray 70 along the view locks what it hits, the lock-on crosshair (FX 0x27, turning) and the green
  marker over the lock. Muzzle smoke and burst (types 44 / 21), a warm light (radius 7). The missile (153): speeds up to
  20 u/s, leads its target (`missile::intercept_time`), spring turns 360°/s; damage 3 on the path and a 2-radius blast
  (flags 0x830000); the blast: 10 streaks, a fireball (`bomb::fireball`, 0x2c4c20), 3 pairs of rings, 10 puffs, the
  flashes (`FlashSpawn`), the shake, the explosion light (the Bomb Glove's template); 300 ticks / 70 (140) from Ratchet.
* **R.Y.N.O.**: its search follows the camera (creatures, 97°, 80, score `d/5` near, `yaw²·pitch²·d + d` far); the green
  marker (FX 0x23). ○ fires a salvo: 7 missiles 9 ticks apart from its 9 barrels for one ammo (item swaps locked), each
  taking 1 off the target's health left, then the next target (the salvo's own list of taken ones; 180°, 100) or a taken
  one at random. The missile (457) wobbles about its centre line, leads its target, trails smoke (type 4), damage 3
  (flags 0x830000) on its path and the beam explosion (`SpawnBeamExplosion`, 10 / 3 / 9).
* **Tesla Claw**: ○ held after a 16-tick warm-up: a 20-point chain up to 12 long (grows at 7 % a tick) waving (three
  phases), pulled to its target (creatures within 15, 32° of the camera, 45° of Ratchet; held at most 50 ticks), hit
  tests every other point (a creature is passed through, anything else stops the chain and is hit), the target hit
  (damage 2, flags 0x210000) every 5 ticks, a second chain, four flickering arcs, sparks (type 53), a bluish light, the
  hum (class sound 4 looping), one ammo per 10 ticks. The draw: FX 14 / 16 strips across the view, the claw's glow
  (FX 0xb), additive.

**Shared pieces (built once)**: `guns::{item_point, first_person, camera_point, aim_angles, hero_point}` (all four, and
`item_point` is the Pyrocitor's nozzle too); `targeting::cone_miss` (Blaster, Devastator), `targeting::polar`
(`FUN_00277b50`: every gun and missile), `targeting::{aim_point, record_radius, record_health}`, `targeting::Markers` /
`marker_render` (Blaster, Devastator, R.Y.N.O.); `missile::intercept_time` / `lead_point` (both missiles: the same
code in `0x2c5b70` and `0x2e5a48`); `creature::turn::spring_turn` (the shot and both missiles); the weapon arm and
persistent stance of §2.4 / §7 (`draw_weapon` via `Weapons::pending_draw`, `put_away`, `gun_stance`, `idle_stance`,
`stance_kept`, `arm_on_state_change`) with `Weapons::pending_anim` (the Devastator's standing shot `0x247a90`) and a
put-away now dropping a draw the same update asked for earlier (the R.Y.N.O.'s last missile); the hand item's sequence
loop sound (`items::refresh_seq_loop`, `FUN_002637d8` for the hand item: the Blaster's firing sequence); `HitSink::
{probe_moby, class_type}` (the guns' rays and creature filter); `SoundCmd::MobySound` / `HeroSounds::moby_sound` (a
sound owned by a moby the hand item created: the R.Y.N.O.'s missile); `fx::create_one` (the hero's queued spawns and
the shot's end share one record builder); `Particles::links` (records a hero-created moby owns by pointer: the shot's
trail). Per weapon, as the game: each update, each search, each constant set, each projectile update.

**Enemy-side reactions deferred** (the hits reach the targets through the existing hit path; their reactions are the
classes' own): the missiles' `record +0x1e |= 0x80` (the target learns a missile is on it) is written but no ported
class reads it; the Tesla Claw's temporary collision-off of the creatures its chain passes (+0x94) is done; the
record +0x04 = 1 ("quick" targets: 5-tick hold, 1-tick cooldown), +0x0b (no lead) and +0x0c (the missile's top
speed) are read.

**Native, not emulated**: standard `f32` with `atan2` / `sin` / `cos` for the fast trig; the markers are 2D HUD
primitives (i32 corners: the game's 1/16-pixel positions rounded [L]); the beam is ordinary textured quads; no PS2
arithmetic. **Inferred [L]**: 0x140600 / 0x14060c (a forced aim yaw: never set in the level code) are 0; the camera's
x axis is right and y down (the view rows 0x167100 as `BSphereView` has them) for the first-person muzzles; the hand
item's loop-sound phase is 0 (it has no table address); game mode 2 never reaches a hand-item update in the port; the
R.Y.N.O. missile blocked at its barrel explodes on its first update (the game at once); the Tesla draw's scroll
steps per tick, its claw glow has no jitter and its first strip width is the call's `f12`; `sqrt` of the intercept's
discriminant is of its magnitude; the Devastator's blocked spawn explodes one step out; the Tesla chain's lines
ignore Ratchet (the game: the item). **M**: the camera angles of the R.Y.N.O. search from the camera's forward row;
`FUN_0026e7b0` as `has_collision`; `0x15ed64` as 1.0.

**Not ported**: the gold versions (0x13e52b / 0x13e52f / 0x13e533 not mirrored: the Blaster's ricochets, the gold
missile's re-targets and triple blast, the Tesla's second target); the shot statistics and the Tesla's "hold ○"
help; the R.Y.N.O.'s item state 3 (no writer in the level code); the Blaster's `fun_0020d580` in the hold state 0x72;
0x141618 branches.

**Tests** (`crates/rc-game/tests/weapons/hero_guns_novalis.rs`, headless on the arrival state with `GiveItem(id, equip)`,
every run twice identical): per weapon the def row and ammo; Blaster standing (draw, one shot per 6 ticks per ammo,
40 u/s, firing sequence 4, breaks crate 376, put away), running (arm layer 57), a critter targeted (green marker)
and hit, first person (0x1e, red crosshair, shots below the eye); Devastator at the critters (shot 54, one missile /
35 ticks, a lock, the blast, a critter hit), running (arm layers 73), first person (lock-on crosshair); R.Y.N.O. salvo
(lock + marker, one ammo, 7 missiles 9 ticks apart, trails, a critter hit), running, first person; Tesla Claw at the
critters (stance 51, ammo per 10 ticks, a lock, a hit, stop and put away) and at a crate (the chain stops there, the
crate breaks). Unit tests in each module (searches, the shot's aim, the intercept, the trail curve, the strips, the
particle types). The engine sees the markers and the beam (screenshots, below). `novalis_hero_digest`: unchanged.

**Follow-up (user)**: a PCSX2 trace of each weapon firing on Novalis (`docs/workflows/pcsx2.md`: a savestate with the
weapon bought, then `rc-trace`'s RAM reads of the item pvars, the projectiles' pvars and the markers 0x1694c0 per
tick) would pin the search results, the salvo's target order and the missiles' wobble against the game.

## 10. How weapons act on creatures; the Suck Cannon and the Taunter (2026-09-28)

**Ids** (the item table 0x179f40 against `lvl.vtbl` 0x20bb00, level01): Suck Cannon **9** / class 849 → `0x303000`;
Taunter **14** / 175 → `0x2ccb78`; Morph-o-Ray **21** / 185 → `0x2d2450` (its chicken: class **270** `0x2df448`, the
morph `0x2defb0`). Neither the Suck Cannon nor the Taunter has a `HeroPdaGadget` case; both fire from their updates
(`HAND_ITEMS` rows with `ItemUpdate::Slot`). **H** (decompiler output; the disassembly where the decompiler lost a
comparison: the fire key `0.0 ≤ MobyAnimKeyTime` at 0x303a70; the class tables read from the overlays).

**The enemy-reaction "interface" in the game — what is shared and what is not** (`creature::react`'s module doc):
* **Shared by every weapon**: the hit records and the resolver (damage, knockback, burn: ported before, `creature::damage`).
* **One real per-class dispatch, used by one weapon**: the third word of each `lvl.vtbl` entry (0x20bb00 + 12·i + 8) is a
  six-slot **class reaction table** the loader puts in class header +0x2c. Only the Suck Cannon calls it (slot +0x00
  from `0x3028c8`, +0x04 from the carried update, +0x08 from its fire, +0x0c from the carried update's let-go, +0x10 by
  every handler, +0x14 at a landing). The default table (level01 0x20c3ac: `return 0` ×5 + a `DeleteMoby` wrapper)
  makes a class unsuckable. Per class the slots are three-line wrappers; the work is the shared handlers `0x304168`,
  `0x304390`, `0x3044a0`, `0x304690`, `0x305260`, `0x3051a8`, `0x304798` over the class's **suck record** (the creature
  header's +0x14 record). Tables on the disc (all 19 overlays): 270 everywhere; 577 (01); 572 / 866 (01, 05, 11); 749,
  580, 340, 827, 252, 193, 1246, 238, 63, 1445, 1382, 568, 1906 on other levels (not ported). 865, 459 and every other
  Novalis class: the default.
* **Per weapon, poking a shared record**: the Taunter writes the damage record's **+0x18** (the lure) of the creatures
  in front (`0x2cc830`); the classes read and clear it (577: 240-tick alert, range 24; 572 family: alert; 459: 600-tick
  alert). The Morph-o-Ray reads the damage record's health / +0x04 and replaces the target with a chicken
  (`0x2defb0`: `CreateMoby(0x10e)`, death bits, `DeleteMoby` unless +0x0e = 1). The Decoy Glove's decoys and gold
  chickens are the enemies' own target search's (`0x274b78`, ported).
* **Not in this game**: stun, freeze.

**In the port**: `moby_update/creature/react.rs` (the table resolution `tables_from_overlay` / `table`, the wrappers
`CRITTER` / `AMOEBOID_866` / `AMOEBOID_572` / `CHICKEN` with the class sequence tables 0x161a70 / 0x161a50 / 0x161a40 /
0x161870, the handlers, the carried update, the burst, the Suck Cannon's globals); the classes' held states call
`react::carried` (577 state 7 → 0xe after a landing, 572 / 866 state 0xe → 1); `hero/suck_cannon.rs`, `hero/taunter.rs`,
`hero/reactive.rs` (their pvars, `Weapons::reactive`); `HitSink::world` (a hand item's calls into class code run on the
moby world; the engine's `CellHits` and the tests implement it); the engine resolves the level's tables once
(`gameplay::level_reactions`) and fills the Taunter's whistle cycle from the level's sound defs.

**The Suck Cannon** (module doc for the states): ○ held pulls creatures within 15 in a 12° cone (110° within 3; the
same in pitch) with a clear line from the mouth: record 1 (turn away) → 2 (rise) → 3 (pulled at up to 0.75 a tick,
drifting sideways in Ratchet's aim frame, shrinking with the distance, oriented toward the cannon's rows) → 4
(swallowed: hidden, no update, scale 0, death bits, the cannon's sound 5); up to 5 held (10 gold). Released, anything
still approaching is let go (record 7: it falls and walks, `0x26d610`, until it lands; the class resumes). ○ again
fires one: at the muzzle, 0.4462 a tick along the aim plus Ratchet's velocity, homing on the fire search's creature
(2.5-unit lock or a 26° cone with the bounding radius, clear camera lines) or the first-person ray's; on its path it
hits mobys (damage 2, flags 0x430000, type 1 / 1, class 0x351), bounces off floors (record 6: rolling), and bursts on a
wall (normal more than 50° from vertical), after 300 ticks, 60 from Ratchet or too slow: the Bomb Glove's explosion
effects at half the fireballs (area hits only gold).

**The Taunter**: ○ plays a whistle (the item's sequence 3; the whistles cycle while the next sound def has near = far =
3.0) and lures the creatures within 30 and 55° of Ratchet's facing (every 4th frame while it sounds); the crates
500 / 501 / 505 / 511 within 12 in the same cone are counted and one of them (`randi(count of the last whistle)`) gets a
hit of 1 — the whistle breaks a crate.

**Native, not emulated / inferred [L]**: standard `f32`; the orientation blend is a slerp (`fun_001fa400`) and the
target rows are `Euler(−π/2, 0, −π/2) · cannon rows` [L: the product order]; the hand items are not table mobys, so the
cannon's position, mouth and rows reach the creatures through `react::Cannon` (as the game's moby loop reads the
matrix the hero update built), its sequence-4 request and sound 5 are made by its next update, the swap lock by the
next update [L: one tick]; the lure moby is Ratchet's (the classes only test it for non-zero); a moby-only hit on the
flight uses (0, 0, 1) for the stale normal [L]; the whistle's aliveness is the previous flush's
(`HeroFx::item_loop_alive`). The chicken's suck record lives in its own pvars (+0xe0; the game: a global table).

**Not ported** (gaps): ~~particle type 18~~ (the flight trail: ported 2026-09-29, `particles::type18`, 5 draws a record); the held-count HUD
element (G-UI-011); the mines' lure list 0x1b0c30 (batch 2); the gold cannon (G-WPN-009); the stats (G-SAV-009). (The
vortex, the vacuum and the rings: §10.1; crates, the pickup sound and the full coverage table: §10.2.)

**Tests** (`crates/rc-game/tests/weapons/hero_reactive_novalis.rs`, headless, every run twice identical): Novalis — a critter
pulled (records 1 → 7 on the release → 2 → 3 → 4), held and hidden (held 1), fired (record 5 → 6) and burst at a wall
more than 10 units on; the Taunter lures both pit critters (their +0x18 set, cleared by them the next tick) and knocks
the first of four crates in front (a hit record, broken). Rilgar (level 05) — the level's tables resolve to {270, 572,
866} (865 default) and a small amoeboid 866 is pulled and swallowed (held state 0xe). Unit: `react::tests` (the
quaternion round trip, the class fallback), `suck_cannon::tests` (the groups). `novalis_hero_digest` with
`RC_HERO_DIGEST_NO_IDLE=1`: unchanged by construction (the new hero fields sit after the digest's cut).

**Engine** (`RC_SCENE=0 RC_AUDIO=0 RC_GIVE_ITEMS=9`, frame-exact dumps, two runs byte-identical): Novalis
`RC_HERO_AT=135.6956,177.82448,40.578,0 RC_PLAY_SCRIPT='40-200:press CIRCLE,260-262:press CIRCLE,330-336:press CIRCLE'`
— frame 112 a critter pulled in, 116 swallowed (its bolts burst out: the swallow's `SetDeathBits`), 268 fired out;
Rilgar `RC_LEVEL=5 RC_HERO_AT=161.52519,326.2638,26.5,1.5708 RC_PLAY_SCRIPT='30-160:press CIRCLE'` — frames 50 / 54 /
62: the small amoeboid in front, pulled, swallowed. The Taunter has no visible effect of its own yet (its rings are
G-WPN-017).

### 10.1 The hold fix, the vortex, the vacuum, the rings (2026-09-28, the user's play-test)

**The report**: "it sucks once, for like a millisecond … you have to stand REAL close … you can't suck more than one
enemy; the Taunter triggers once for a millisecond". **The cause** (found with a per-tick trace of the engine): the
hold logic was the game's and ran continuously (state 3 for the whole hold, two critters held in sequence), but both
items kept their looping sound on hand-item sound channel 0, which the Pyrocitor's `item_gone` releases every tick
whenever another item is in the hand (`hero/items.rs`): the Suck Cannon's suction stopped a tick after it started, and
the Taunter's whistle ended a tick after it started — so its update saw "not sounding", stopped (sequence 1) and could
only whistle again after 17 ticks of ○: one-tick lures. **Fix**: a channel of their own (`fx::LOOP_ITEM`). And nothing
was visible: the vortex and the rings were not ported.

**Reach and strength (the game's, unchanged)**: 15 from the cannon, a 12° cone (squared radians 0.04386491; 110°
within 3), the same in pitch, a clear line from the mouth; pulled at up to 0.75 a tick (+0.11 a tick); 5 held (10
gold). Headless: an amoeboid 14.2 away is pulled; two are swallowed in sequence while ○ stays held.

**The vortex** (`hero/suck_vortex.rs`, module doc): a 10-node path of 1.7-unit segments (reach 17) bending after
the aim with a spring lag (≤ 18°) and a slow wobble, rings of 20 points (radius 0.1 + 0.3·i), node strengths from a
line along the aim (the tube stops at a wall), up to 200 smoke strands (type-23 puffs) spiralling down the tube into
the mouth and 2 loose puffs a tick; drawn as the tube's camera-facing quads (FX 0x15, additive) on draw list 2; it
fades when not sucking and collapses (the strands flung out) at the stop. **The vacuum** (`suck_cannon::vacuum`,
`0x307a50`): the pull's walk of the run list gives every other live moby a `randi(10)`; on 0 a bolt or ammo pickup
(classes 0xcc, 0xd5, 0xd6, 0xde, 0xdf, 0xe1, 0xe2, 0x3ee) within 30 of Ratchet near a strong node is taken
(`bolt::start_fly`, `pickup::collect` = `0x2db850`, factored out of the pickup's idle).

**The rings** (`taunter::Rings`): while the whistle sequence plays, one every 10 ticks from the horn along the item's
−row 1 at 20 u/s for 30 ticks, 0.3 → 10 wide, alpha 0x14 → 0 (faint by the game's constants), FX 8, colour 0x7f5050,
additive, draw list 1. Engine: `rc-engine/src/reactive_render.rs` (both, on crate::fx_draw); the engine's hit sink now
hands the particles to `HitSink::world` (the strands).

**Tests** (`hero_reactive_novalis.rs`, a stand-in sound layer (`FakeSounds`) gives the whistle a length): the cannon
sucks for the whole 360-tick hold with the vortex drawn every tick, node 0 at full strength, up to 200 strands, the
vacuum taking the swallowed critters' bolts, then fades out; Rilgar: the far (14.2) and the near amoeboid swallowed in
sequence; the Taunter whistles and lures in every 30-tick window of a 240-tick hold (the whistle sounding 200+ of the
240 ticks), 3 rings out at a time. Unit: `suck_vortex::tests` (the circle table, the spring, the rotation sense, the
vacuum's reach). Engine frames (two runs identical): `novalis_vortex_150.png` (the tube and the vacuumed bolts),
`novalis_taunter_rings_078.png`.

**Native / [L]**: standard `f32`; the rotation sense of `FUN_00274ac8` (turn toward the aim), the lerp form, the
first-person second row, the scroll advanced at registration once a tick; the run list the pull walks is rebuilt in
the hand item's update (the game uses the moby loop's).

### 10.2 Crates, the bolt pickup sound, and the coverage audit (2026-09-28, the user's play-test)

**"The Suck Cannon can't suck crates."** Not a port bug: the game's cannon never takes a crate. The pull `0x302bd0`
takes a targetable moby only when its class reaction table's slot +0x00 is not the default `0x3040b8` (asm
0x302c70..0x302c7c: `lw v1, 0x2c(class)`, `lw a0, 0(v1)`, `beq a0, s8(=0x3040b8)` → the vacuum instead). The loader
`MobyClassRegister` 0x279218 copies the third word of each `lvl.vtbl` entry to class +0x2c; the crate classes 500 / 501
/ 502 / 505 / 511 have the default table on **all 19 overlays** (scanned from the overlay ELFs; only 270, 572, 577, 866
and the 13 other-level enemy classes of §10 have one). The vacuum `0x307a50` (the same code on 17 overlays, relocated on
13 and 16) takes only class type 0x13 (bolts) or the 8 ammo pickup classes of `0x2732b8`. `CrateUpdate` 0x2ea178 has no
suck path. **What the cannon does to crates in the game**: a fired creature's flight hits the mobys on its path (template
flags 0x430000, damage 2); the crate's hit mask 0x1830000 takes it (0x30000), so the crate breaks (`CrateBreakFx`,
`CrateDropBolts`) and the next suck vacuums its bolts (or an ammo crate's pickups). The port does exactly that (tests
below).

**"No bolt pickup sound when the cannon sucks an enemy or bolts."** The game plays one sound for every bolt pickup:
`FUN_002bcb90` (`bolt::start_fly`) `PlayClassSound(0, 0x20, bolt)` at most once per `ticks(3)` (`CollectBolt` 0x2bc4f0
plays none); the vacuum calls the same `0x2bcb90`, and the bolts a swallowed enemy drops (`SetDeathBits` in the swallow
`0x304390`) are picked up by it or by their own idle — the burst drops none. **Cause** (engine): the hand item's calls
into class code run on a moby world the engine's hit sink builds (`CellHits::world`, `rc-engine/src/gameplay.rs`), and
that world had **no sound layer** (every class sound refused: the vacuumed bolts' pickup sound, the ammo pickups' class-213
sound, 577's bounce sound 2 and the fire-out sound at the fire), **no item tables** (`pickup::collect`'s `AddAmmo` read
ammo 0 and max 0: a vacuumed ammo pickup *set* the ammo to its amount, uncapped) and no missions. **Fix**: the hand world
gets the moby loop's item tables (with Ratchet's ammo), missions and a `ClassSoundSink` with the hero update's listener.
With it: the vacuum's `AddAmmo` goes to Ratchet in the tick of the take (the game stores 0x13d428 at once; the port
applied the hero-block write a tick late and lost it when a class wrote the block first), and `collect` follows
`0x2db850` (nothing happens when the ammo did not rise: a full weapon leaves the pickup lying).

**Found by the audit and fixed** (all small, inside the Suck Cannon): `0x302840` gives the swallowed moby the cannon's
rows turned by `gp−0x4d60` = (−π/2, 0, −π/2) and the fired one by `gp−0x4d50` = (π/2, 0, π/2) (the port only built
the matrix: `react::cannon_frame`); every handler's matrix call is `fun_0020e098` = `MobyAnimSphereLerp` 0x265d78, which
keeps the stored rows — the port's `MobyBuildMatrix` rebuilt them from the Euler angles, so the pulled creature's turn
toward the cannon, the flight's roll and both rows above were thrown away the same tick (`react::sphere_lerp`); the
fire search turns its cone to each accepted creature and takes one only within 10° of the cone's yaw (best distance and
pitch always follow a candidate; `FastDiffRots` < 0.17453292); the burst's fireballs, rings and flashes carry the base
velocity (0, 0, 2·dt·k) (sp+0x60); the handlers' "gone" test is state −2 / −3 only (the port: any state ≥ 0x80); the
homing intercept roots the discriminant's magnitude (VU0 `vsqrt`): the port's `sqrt` gave NaN for a target faster than
the shot and the fired creature's position went NaN (seen when a critter fired right after a knock-down homed on the
other, knocked critter).

**Coverage** (`0x303000` and every function it reaches in the suck code; P ported with the port's function, N not ported
with its gap, n/a with the reason; **F** = found missing or wrong by this audit and fixed). Level01 addresses.

| address | what it does | status |
|---|---|---|
| 0x303000 top | `queue_animation_update(4, 0x753f, 0x24ce30/…50/…d8, &held 0x1413c8, 5)` every tick: the held-count HUD element | N (G-UI-011) |
| 0x303000 top | states ≠ 0 / 3: the vortex fade `0x307850` | P `suck_cannon::update` → `Vortex::fade` |
| 0x303000 top | `0x302ed0` recount | P `recount` |
| 0x303000 st 0 | the held slots' collision off (+0x94 = 0) and untargetable (limit 5 / 10), swap lock 0; none held → timer 0, st 1; else lock 1, seq 6 (4) → st 4; `0x306510` vortex reset | P `update` |
| 0x303000 st 1 | wrap → seq 1 (0); timer; 0 → put away `0x22efd8`; held → seq 6 → st 4; the start gate (fire mask, timer 0, !0x1413f7, !0x1413fc, ready > `ticks(15)`, `0x3027e8`) → arm out `0x22ee08`, class sound 2 flags 4 (slot in +0x04), timer `ticks(10)` → st 2 | P `update` |
| 0x303000 st 1 / 4 | stats: 0x1416c8 + 1 (unless −1), 0x1416ca play-time minutes, 0x1416cc \|= 1 << level \| 0x80000000 | N (G-SAV-009) |
| 0x303000 st 2 | wrap → seq 3 (5); blocked group / 0x1413fc / 0x1413f7 → release the loop, put away, swap 0, → st 1 (seq 1, timer 0) / st 4 (seq 6); timer → `ticks(20)`, st 3 | P `update`, `stop` |
| 0x303000 st 3 | seq 3 unless (seq 3 / seq 0 not wrapped); timer; nothing coming and (full, group, ○ up with the timer out, 0x1413fc, 0x1413f7) → stop + collapse `0x3078b8` (+ timer 0, lock 1 when held); else wrap → seq 3 (2), the pull `0x302bd0` | P `update`, `stop` |
| 0x303000 st 4 | timer 0 → put away; wrap → seq 6 (0) + put away; ○ up → lock 0; target timer / a dead target cleared; hero state 1 → SetState 0x1e; first person (Ratchet hidden, !0x1413fc): ray 50 (CollLine flags 0, the cannon ignored) → a targetable moby is the target (+0x10, `ticks(20)`, +0x18 hit height); marker FX 0x21 red / green | P `update` |
| 0x303000 st 4 | 0x140600 forced target / 0x14060c forced yaw | n/a (never set in the level code, §9) |
| 0x303000 st 4 | fire gate (lock 0, held, timer 0, ○, group ∉ {3, 7, 0x11, 0x12}, !0x1413fc, !0x1413f7, ready > `ticks(22)`) → arm out, seq 5 (2), swap 2, timer `ticks(25)`, stats → st 5 | P `update` (stats: N G-SAV-009) |
| 0x303000 st 5 | key time ≥ 0: first held slot emptied; muzzle (first person: camera point (0.15, 0.3, 1), view · 0.4462; else joint list 0, aim row · 0.4462); position, `fun_0020e098`; + Ratchet's velocity 0x13f450 | P `update` (**F**: the sphere build) |
| 0x303000 st 5 | the fire search (rows below); `0x302840(cannon, moby, gp−0x4d50)`: rows = Euler(π/2, 0, π/2) in the cannon's | P `fire_search`, **F** `react::cannon_frame` |
| 0x303000 st 5 | slot +0x08 (height +0x18, moby, velocity, target +0x10); held − 1; none → timer `ticks(20)`, swap 0, st 1; else swap 0, st 4 | P `update` → `react::slot_fire` |
| 0x303000 end | slot going away (0x140404 = 3): release the loop | P `update` |
| 0x303000 end | … and the pad's released mask \|= 5 (0x13cae8) when L1 / L2 are held | N (G-HERO-025) |
| 0x303000 all | the gold cannon 0x13e529 (10 slots, the gold flight / burst) | N (G-WPN-009) |
| fire search | run list, skip state ≥ 0x80 and the fired moby; the point = position + damage record +0x10 (0.5 without) | P `fire_search` |
| fire search | **filter**: targetable, a class, class type 5 (creatures only) | P |
| fire search | within 2.5, Ratchet's aim yaw within 60°, elevation below 45° → taken at once | P |
| fire search | nearer than the best; the 26° cone about (yaw, pitch) widened by the bounding radius; a clear camera line (flags 6) else the search ends | P |
| fire search | best and pitch follow the candidate; the yaw turns to it and it is taken only within 10° of the cone's yaw | **F** `fire_search` |
| 0x302ac0 | the mouth (joint list 0); look stance: 1.2 along the camera rows 0x167450 at the item's height, the camera's yaw / −pitch; else Ratchet's aim 0x13f9d8 / 0x13f9d4 | P `aim` |
| 0x302ac0 | 0x141618 = 1 (weapon lowered at a wall): the item's +0x44 pitch | N (G-WPN-011) |
| 0x3027e8 | groups 0, 1, 5, 0xc; 2 / 4 outside state 0x3c | P `group_ok` |
| 0x302bd0 | `0x302ac0`, the vortex `0x3067d0`, then the run list 0x15ffe4 | P `update`, `pull` |
| 0x302bd0 | **filter (suckable)**: class ≠ 0, targetable 0x1000, reaction table slot +0x00 ≠ the default → the pull; the rest → the vacuum. Suckable on the disc: 577 (01), 572 / 866 (01, 05, 11; the wrapper takes only 866, not in its state 8), 270 (every level), 749, 580, 340, 827, 252, 193, 1246, 238, 63, 1445, 1382, 568, 1906 (G-ENM-001). **Not**: crates 500 / 501 / 502 / 505 / 511, bolts, pickups, Ratchet | P `pull` + `react::table` |
| 0x302bd0 | record null → skip; record state > 5 → skip | P |
| 0x302bd0 | ≥ 15 from the cannon, a blocked group, held ≥ 5 (10 gold) → let go (slot +0x0c) unless state 0 | P |
| 0x302bd0 | state 4 / 5 skip; state 3 → take `0x3028c8` | P |
| 0x302bd0 | state < 3: slot −1; point + 0.4 up; yaw² and pitch² below (12°)² ((110°)² within 3); a clear line from the mouth (flags 2, the moby ignored) → take; else let go when state > 0 | P |
| 0x3028c8 | slot +0x00 (moby, mouth, cannon); a 2 not yet pulled with room (held + coming < limit): seq 3 (10), state 3, t 0, speed 1, distance to the mouth, first free slot, coming + 1 | P `react::take` |
| 0x302ed0 | held / coming from the slots (states 3 / 4 with a slot index; else the slot emptied; the debug print) | P `recount` (print n/a) |
| 0x302a88 / 0x302a50 | slot +0x04 / +0x0c through the class table (none: 1) | P `slot_swallow` / `slot_let_go` |
| 0x304100 | slot +0x10 (the record) unless deleted, no pvars, no class | P `react::record` |
| 0x3040b8.. | the default table: 0 ×5 + `DeleteMoby` | P (`table` = None) |
| 577 0x2f1c78.. | wrappers: state 7 / back to 1; slot +0x08's bounce sound 2 when the record is in state 6; let go → state 7 | P `CRITTER`, `slot_*` |
| 572 / 866 0x2efa88.. | wrappers: state 0xe; slot +0x00 takes only 866 and not in its state 8 | P `AMOEBOID_*` |
| 270 0x2e0a28.. | wrappers: state 5; the bounce sound 1 | P `CHICKEN` (§12) |
| every table +0x14 | `DeleteMoby` | P `slot_delete` |
| 0x304168 | the record's point / cannon; by state: 1 (t < 1: seq 1, else seq 2, state 2), 2 (t < 1), 3, 4 (3), 5..7 (seq 3, state 3, speed 1, distance to the cannon), else (seq 1, state 1); 8 → 0; no record → print | P `react::approach` (print n/a) |
| 0x304390 | at the mouth; `0x302840(cannon, moby, gp−0x4d60)`: rows = Euler(−π/2, 0, −π/2) in the cannon's; `fun_0020e098` | **F** `react::swallow`, `cannon_frame`, `sphere_lerp` |
| 0x304390 | the cannon's sound 5 when the hand holds class 0x351 | P (made by its next update [L]) |
| 0x304390 | `SetDeathBits(moby, 0, −1)`: save bits, **the enemy's bolts** (`BoltBurst`) | P `crate_::set_death_bits` |
| 0x304390 | state 4, seq 4 (10), scale 0 | P |
| 0x3044a0 | visible, mode &= ~3; 1.23456 → seq 6 (20), collision bits +0x88, state 6; else seq 5 (0), state 5; velocity, timer `ticks(300)`, target (its damage record +0x1e \|= 0x80, its point + height), within 1 of Ratchet a step ahead, the fire sound +0x9c | P `react::fire_out` |
| 0x304690 | let go: state 3 leaves its slot (coming − 1); 1..3 → seq 7 (10), state 7; 4 → 1 | P `react::let_go` |
| 0x305260 | deleted (−2 / −3) → 0 | **F** `react::carried` (`dead`) |
| 0x305260 | no record → `fun_0020e098` with mode 4 | **F** `tail_none` (`sphere_lerp`) |
| 0x305260 | +0x30 = position | P |
| 0x305260 st 1 | turn away from the mouth (`0x26d058` on knock +0x44), within 0.01 → state 2; velocity 0 | P |
| 0x305260 st 2 | speed t + 1, t += 1/`ticks(1)`, z += t/100, the quaternion; velocity 0 | P |
| 0x305260 st 3 | visible; t < 1: rows = slerp(start, Euler(−π/2, 0, −π/2) in the cannon's) over `ticks(30)`, mode 4 | P `pulled` (**F** kept by `sphere_lerp`) |
| 0x305260 st 3 | the mouth (joint list 0); scale by the distance; pull speed +0.11 ≤ 0.75; within 70 × speed the cannon's seq 4 (2); within 3 the swap lock 2 and the collision off | P |
| 0x305260 st 3 | the drift in Ratchet's aim frame 0x13f990, the step, `fun_0020e098`; `FUN_00218828` | P (**F** `sphere_lerp`); `0x218828` n/a (empty) |
| 0x305260 st 3 | arrived: coming − 1, held + 1, slot +0x04, the slot's moby untargetable | P |
| 0x305260 st 4 | hidden, no update, mode 4 off, collision off; wrap → seq 4 (3), scale 0.3, rotation 0 | P |
| 0x305260 st 5 / 6 | z < 2 → `0x3051a8` without the burst; scale regrowth; homing on the target (not −2 / −3, after `ticks(5)`): the intercept's root is the VU0 `vsqrt` (0x2210f0) of the magnitude — a target faster than the shot gives a number, not NaN | P `flight` (**F** the target test; **F** the root: the port's NaN froze the shot) |
| 0x305260 st 5 / 6 | pos += velocity; the hits `0x26e808` (2, flags 0x430000, xy 1, z 1, type bytes 1 / 1, class 0x351) with the flight sphere: **other mobys** — creatures take damage 2, **crates break** (mask 0x1830000), anything with a matching mask | P (`sphere_mobys`) |
| 0x305260 st 5 / 6 | the world sphere, pushed centre, contact; a hit on Ratchet ignored | P |
| 0x305260 st 6 | roll: collision on, contact (0, 0, 1, 0.01), the quaternion about the axis across the motion; `FUN_002731d0` (result unused) | P (**F** kept by `sphere_lerp`); `0x2731d0` n/a |
| 0x305260 st 5 / 6 | a hit: seq 6 (20), bits +0x88, state 6; normal > 50° from vertical → bounce sound +0x9e, land with the burst | P |
| 0x305260 st 5 / 6 | gold and a hit moby of type > 4: the sound, land with the burst (and no return) | n/a (gold not mirrored: G-WPN-009) |
| 0x305260 st 5 / 6 | reflect `0x221570`; timer / 60 from Ratchet / speed < 0.01 → land with the burst; gravity 0.00925 | P |
| 0x305260 st 5 / 6 | the trail `PartType18Spawn` 0x281430 | N (G-PRT-001; its 3 draws kept) |
| 0x305260 st 7 | visible, gravity 0.013, z < 2 → land with the burst; `0x26d8b0` (0.8, the fall sphere); rotation / 1.5; landed → speed 1, return 1; wrap → speed +0x78 | P |
| 0x305260 st 8 | debug print | n/a |
| 0x305260 tail | `fun_0020e098` with mode 4; below state 4 the let-go countdown (slot +0x0c at ≤ 0) | P (**F** `sphere_lerp`) |
| 0x3051a8 | the burst when asked; collision bits +0x84; rotation 0 and its rows; `fun_0020e098`; mode 4 off; class scale; state 0; slot +0x14 (`DeleteMoby`) — no bolts (they fell at the swallow) | P `react::land` |
| 0x304798 | 10 / n low and 4 / n high fireballs `0x2c4c20` (the spread about up), one toward the camera | P `react::burst` |
| 0x304798 | the base velocity (0, 0, 2·dt·k) on the low / high fireballs, the rings and the flashes | **F** `burst` |
| 0x304798 | rings (type 11, `0x27f8f8`), flashes `0x309a68` (two more beyond 9), the camera shake 0x167260 / 0x167268, the light `0x2f3570` (template 0x20b7d0) | P |
| 0x304798 | colour shifts `0x270f48` / `0x270fa8`; area hits radius 3 (`0x26f8f8`, damage 2, flags 0x830000) | n/a (gold only: G-WPN-009) |
| 0x304798 | sound | n/a (none in the game) |
| 0x307a50 | **filter (vacuum)**: not deleted; `randi(10)` = 0; class type 0x13 (bolts 13..16) or `0x2732b8` (0xe2, 0xcc, 0xde, 0x3ee, 0xd6, 0xe1, 0xd5, 0xdf); within 30 of Ratchet; a node above 7 within max(2, 0.1 + 0.3·k) | P `suck_cannon::vacuum`, `Vortex::in_vacuum` |
| 0x307a50 → 0x2bcb90 | a bolt flies to Ratchet: **the bolt pickup sound** `PlayClassSound(0, 0x20, bolt)` once per `ticks(3)` (else a random bend) | P `bolt::start_fly` (**F** audible: the hand world's sound layer) |
| 0x307a50 → 0x2db850 | a pickup: `AddAmmo` (ammo 0x13d428, max from the item table) at once | **F** (item tables in the hand world; the write applied in the tick) |
| 0x2db850 | only when the ammo rose: the banner `0x278a50`, the stat 0x13de08, state 2, scale ×1.2, rise 8·dt, class 213's sound 0 once per `ticks(10)` | **F** `pickup::collect` (taken when full; silent) |
| 0x306510 / 0x306528 / 0x3067d0 / 0x307850 / 0x3078b8 / 0x306158 | the vortex: reset, start, path, strengths (the line flags 0x14), rings, strands (type 23 spawn / `KillPart`), the draw on list 2, the fade, the collapse | P `suck_vortex` (§10.1) |
| 0x3067d0 | 0x141618 = 1: the aim from the item's row 1 | N (G-WPN-011) |
| 0x2e8e88 (class 479) | the drones' target search skips a moby whose suck record is in state ≥ 1 | P `classes::drone` |

**Counts**: 81 rows — 69 ported (16 fixes in 15 rows found wrong or missing now, **F**), 7 not ported (G-UI-011,
G-SAV-009 ×2 rows, G-HERO-025, G-WPN-009, G-WPN-011 ×2 rows, G-PRT-001), 5 n/a (the forced aim, never set; the gold
flight and burst; the burst's sound, which the game does not have; a debug print); inside 4 ported rows a debug print,
the empty `0x218828` and the unused `0x2731d0` are n/a.

**Tests** (`hero_reactive_novalis.rs`; its harness now gives the moby loop and the hand item's world the item tables
and a logging sound layer that refuses the slot, so the runs behave as before): `novalis_suck_cannon_leaves_crates`
(the filter: the four crates in the cone stay for a 160-tick suck; no crate class has a table or is a bolt);
`novalis_suck_cannon_fired_critter_breaks_a_crate` (a critter swallowed at the pit — its bolts vacuumed with the pickup
sound through the hand item's call; Ratchet moved in front of crate 376; the critter fired: hit and broken within 30
ticks; ○ again: the crate's bolts vacuumed with the sound); `novalis_bolt_pickup_sound_on_a_normal_pickup` (crate 376
broken next to Ratchet: the bolts' own pickup plays the same sound in the moby loop); `novalis_suck_cannon_vacuums_an_ammo_pickup`
(ammo crate 552 broken in front of the vortex: the Bomb Glove pickup taken mid-air, ammo 10 → 13 in the tick of the take,
class 213's sound). Unit: `react::tests::swallow_and_fire_take_the_cannons_rows`, `burst_carries_the_base_velocity`;
`pickup::tests::collect_only_when_the_ammo_rises`. Every run twice identical.

**Engine** (Novalis, frame-exact, `RC_SCENE=0 RC_GIVE_ITEMS=9 RC_HERO_AT=147.42233,130.81314,57.0,-1.5708
RC_PLAY_SCRIPT='20-300:press CIRCLE' RC_DEBUG_HIT=376@120`): with `RC_AUDIO_TRACE=1` the suction loop starts at tick 21
(class 849 sound 2), crate 376 breaks at 121 (class 500 sound 0) and **class 14's sound 0, flags 0x20 plays at 121 and
130 with a voice slot** — at 121 the bolts are still at the crate, 5 from Ratchet, out of their own pickup's reach: the
vacuum's (before the fix every class sound of the hand item's calls was refused). Frames (`RC_AUDIO=0`, two runs
byte-identical, 100..140): `frame_00100.png` (the vortex over the crates, which stay), `frame_00124.png` /
`frame_00130.png` (crate 376 broken, its bolts taken).

**Native / [L]**: the rows' product order as the pull's target (§10); the hand world's listener is the camera the hero
update hears with (the previous tick's), as Ratchet's own sounds.

**The knock-down at the first fire press (checked 2026-09-28): faithful.** In `novalis_suck_cannon_pulls_and_fires_a_critter`
the second critter (moby 592, class 577) walks up to Ratchet and bites him at tick 258. This is `GroundCritterUpdate`
0x2efc60 state 4: at key frame 13 of sequence 3, within 2 xy and 0.5 height and facing within 15°, it sends a hit with
damage 1 and flags 1, and the 0.2 push goes to Ratchet. The hit intake 0x231580 then puts him in hurt 0x16, so the
press at 260 finds a blocked group and the press at 330 fires. At HEAD 0b43851 the same critter also walked at him
(state 3), but it was caught in the vortex before it could bite.

The runs split at tick 51, the first tick of the vortex: one extra RNG draw on each tick. The source is the Novalis
help director 1341 (`HelpHintDirectorUpdate` 0x30acb8, ported by the cheap-win class batch in
`classes/units/help_director.rs`). Its state 0 stores 0xff into the update distance (`li v0, 0xff; sb v0, 0x30(s0)`
at 0x30ad18..0x30ad20), so it is on the run list every tick. The pull 0x302bd0 walks that list (`DAT_0015ffe4`, then
+0x28). It passes each untargetable or default-table moby to the vacuum 0x307a50, which draws `FUN_0026c930(10)` for
every moby whose state is not −2 or −3, before it tests the class. In the game the director makes that draw too; while
it was unported, the port's director stayed at state 0 out of update range. The shifted stream changes the critters'
side-offset rolls (0x2efc60: `randi(256)` for the sign, `randf(30, 90)` for the flip timer), so 592 now reaches the
bite.

The reaction audit's creature fixes (§14.4) do not cause it: the rows are identical up to tick 50, and at tick 51 the only
extra call site in the draw trace is `suck_cannon::vacuum` on moby 910 (class 1341).
The test accepts either press (the 330 one only after a hurt 0x16 at the first) and runs 700 ticks. **No fix**: this
is RNG order, and it now includes a moby the game really updates.

## 11. Holding the weapon: the glove-holding layers, standing → running fire, the R.Y.N.O. crash (2026-09-28, the user's play-test)

**What the user saw** (recordings 16.35.21 / 16.47.33 of the original, 16.37.25 of the port): every weapon swung with
Ratchet's one running arm (the Devastator bobbing up and down, pointing at the ground and the sky), where the original
holds it level, the two-handed ones in both hands; firing a continuous weapon standing, he could not start running; the
R.Y.N.O. crashed the game.

**The holding layers `0x22e660`** [H: decompiler output of `0x22e660`, `FUN_00263e08` / `FUN_00263ec8` (alloc / free:
the node is zeroed), `FUN_00263f70` (the advance: key flags +6 / +7 and the frame pointers +0x38 / +0x3c),
`FUN_00264220` (the retargeted key), `0x22df10`, `0x22def8`, `HeroItemsCreate` 0x22f3c0 (0x1413fb = def +0x18); data:
the level's moby classes 1 and 2]. Called first in `0x22f390` (the items' upkeep in `HeroItemsUpdate`, then `0x22f068`,
then the face layer `0x22e3a8`). Per layer i (0 → node 0x140050 on joint list 12, the right arm with the hand item;
1 → 0x140054 on list 13, the left arm):
* **wanted** while 0x1413fb ≠ 0 (and for list 13, 0x1413fb = 2), not while the hand item is hidden (0x1413ff: the
  node's +0x34 set, faded out for good). Made at weight 0 with node +0x18 = `0x197780[0x198040[i + 1]]`: **moby class
  1 or 2**. Not wanted: weight → 0 by 0.1·[0x15ed60] a tick, then freed — but the common tail below brings an
  unwanted node's weight back to 1 in the same tick, so it only really goes when +0x34 is set (the hand swap, §11.1).
* **weight** → 1 by 0.1 a tick, but → 0 while the weapon arm's layer 0x140058 is out and not fading (0x140064 = 0):
  the firing arm replaces the holding arm, and comes back as the arm fades.
* **keys**: it copies Ratchet's key (seq A / B +0x52 / +0x53, frames +0x50 / +0x51, t +0x54; key A only while he is
  not blending, `FUN_0022def8` would map a 0xff key A to his +0xa5 but that never happens). When he blends into a new
  sequence while the node is itself between two sequences, it first finishes its own blend at 0.15 a tick (+0x30).
  In the special poses (`0x22df10`: his key B 0x1c / 0x1d / 0x1f, 0x31 / 0x32, 0x37, 0x4a / 0x4b, 0x50, 0x60, 0x6d /
  0x6e) it blends to sequence 0 frame 0 over `ticks(15)` and plays it (speed 1); out of them it blends back to his key
  B from frame 0 at 0.1 a tick (+0x32 / +0x33).
* **the data**: a key on a sequence below 0x17 is read from the node's class (`FUN_00263f70` sets +6 / +7 and calls
  `FUN_00264220`), any other from Ratchet's own. Classes 1 and 2 are animation-only (no mesh), 20 joints each, 23
  sequences with the frame counts and key times of Ratchet's sequences 0..0x16 (seq 17: 27 frames against his 25); the
  byte table after their joint-list word (class +0x1c + 4) retargets their joint k to Ratchet's joint 52 + k (class 1,
  list 12) and 72 + k (class 2, list 13); their translation records already carry Ratchet's joint numbers (52 / 54 /
  56). So on his idle, walk, run, jumps … the arm joints play the **holding** versions of the same frames: the gun stays
  in his hand(s), level with his chest, while his legs and body run.
* **which items** (def +0x18 of the level01 item table): 2 (both arms): Suck Cannon 9, Devastator 11 (and 36), Visibomb
  13, Pyrocitor 16, Hydrodisplacer 22, R.Y.N.O. 23; 1 (the hand-item arm): Swingshot 12, Taunter 14, Blaster 15, Tesla
  Claw 19, Morph-o-Ray 21, Trespasser 26, Metal Detector 27, Hologuise 31, PDA 32; 0: the wrench, the gloves (10, 17,
  20, 24, 25), the Walloper 18, the packs, boots and helmets.

**Aim steadiness.** The holding classes are the whole mechanism: there is no aim joint modifier or spine correction in
the running path (the joint-record writers are the head look, the idle secondaries and `HeroLean`, §7). The guns' line
of fire does not come from the muzzle's orientation either: the Blaster `0x2ca610`, the Devastator `0x2c7d68`, the
R.Y.N.O. `0x2e4e60` and the Tesla Claw aim along Ratchet's moby rows (his facing; the camera in first person), only the
spawn point is the muzzle (`FUN_002645a8` on the item's joint list). With the arm swinging, that spawn point went from
the hip to above the head; now it stays in the holding pose. The Devastator's running shot still kicks the barrel up
about 38° for a few ticks: that is its arm sequence 73 (the recoil, on lists 12 / 13 at full weight while the holding
layers fade out), not the aim.

**Standing → running fire.** `0x242858` (the persistent-arm stance: a weapon with def +0x30 out → `SetState(0, 0)` in
its standing sequence) has three callers [H: xrefs; SetState 0x23cf98 case 3, `HeroStatePhysics` 0x2370b8 for state
3, `HeroStateTransitions` 0x242930 state 2's slow stop]: the stop 3's SetState, the stop's physics, and the walk's
slow stop. The walk's SetState (case 2 / 0x3f / 0x73) does **not** call it. The port called it in the walk entry, so
the stick's `SetState(2)` out of the stance was turned back into the stance every tick: Ratchet could not run while
holding ○. Now the walk starts (0x22eca0 brings the moving arm layer up: the Blaster's 57), he runs firing, and the
stop / the slow stop / the stop's physics return him to the stance (the physics call added: `ground.rs`).

**The R.Y.N.O. crash.** Reproduced in the engine (`RC_GIVE_ITEMS=23`, `RC_HERO_AT` 9 units from the critters, a
scripted salvo): `attempt to multiply with overflow` at `rc_game::audio::Spu::mix`, the master volume stage
`x · (reg << 1)` in `i32` on the exact sum of the 48 voices. A salvo's seven missiles each play class sound 1 and
end in a beam explosion with sound 0: with three or more near full-scale voices the sum passes 65 538 and the product
passes `i32` (a panic in the dev build, a full-scale wrap-around click in the release build). Fix: the product in 64
bits (`audio::master_out`), then the clamp as before. The same class checked in the other guns: they share the mixer
(fixed once); the Devastator's and the R.Y.N.O.'s missiles both wrote the target record's +0x1e..+0x20 while
`targeting::record` only guarantees +0x14: now `targeting::mark_missile` (bounded) for both. No other panic path found
in a stress run (the R.Y.N.O. standing, running, first person, at 17 places near and far, 520 ticks each).

**In the port**: `rc_formats::moby_anim::{AltKeys, retarget_map}` (a pose layer's key from another class, remapped;
`PoseLayer::alt`), `hero::anim::{HoldClass, HOLD_CLASSES, HOLD_SEQS, PoseNodes, pose_layers_with}` and
`AnimLayer::{alt, own_blend, special, back, kill, born}` (a node's place in the +0x60 list: the newest first),
`hero::weapons::{hold_update, hold_special, HOLD}` (`Weapons::layers` now holds the four nodes), the call in
`items::items_update`; the engine loads classes 1 / 2 (`gameplay.rs` `hold_classes`, `RatchetAnim::hold`) for the
palette and the shadows; the hand item and the joint points follow the holding arm through `eval_chains_with`.
Native `f32`; no hardware modelling.

**Inferred [L]**: the node order when a holding layer is remade while an arm layer is out (by creation stamp, as
`FUN_00263e08` links); 0x15ed60 = 1 (NTSC).

**Tests**: `weapons::tests::holding_layers` (per 0x1413fb, the fade under the arm layer, the special pose, the hidden
hand), `targeting::tests::missile_mark_is_bounded`, `audio::tests::loud_mix_clamps_without_overflow`;
`hero_guns_novalis`: `novalis_holding_layers_per_item` (Devastator / R.Y.N.O. two layers, Blaster one, Bomb Glove none,
following the run at weight 1), `novalis_weapon_steady_while_running` (running 60 ticks: barrel pitch spread 13.1°,
yaw 7.2°, height 0.16 with the holding classes against 97.9° / 124.9° / 0.30 without), `novalis_standing_fire_then_run`
(Blaster and Tesla Claw: standing fire → running fire → the stance again; fails with the old walk entry),
`novalis_ryno_salvo_sounds_mix_without_overflow` (the salvo's sounds through the audio system; panics with the old
arithmetic). `novalis_blaster_standing_breaks_a_crate` now stands 6 units off (was 9): with the Blaster held up in the
idle too, every shot from 9 units passes over that low crate (it sits in a dip 0.86 below him); it broke only because
the first shot left from the lower no-weapon idle arm during the stance blend — the port's old pose, not the game's.
`novalis_hero_digest` (`RC_HERO_DIGEST_NO_IDLE=1`) is byte-identical to HEAD's.

**Frames** (scratchpad `weapon_hold/`): `orig_dev_run.png`, `orig_dev_fire.png` (the original, running with the
Devastator), `ours_dev_run.png` (the port before, from the user's recording), `ours_after_dev_zoom.png`,
`ours_after_ryno_run.png`, `ours_after_blaster_run.png` (after), `orig_stand_to_run.png` / `ours_after_stand_to_run.png`
(standing fire → running fire).

### 11.1 Hand swaps: the holding layers released, the take-out sounds (2026-09-28, the user's play-test)

**Stuck two-hand arm.** After a two-handed weapon, the left arm stayed in the holding pose with the wrench or any
one-handed item. Cause [H: disassembly of `0x22e660` 0x22e7ec..0x22e8d0; `UpdateWrenchSelected` 0x2307e0]: an unwanted
holding node is faded by 0.1 and then, in the function's common tail, faded back toward 1 (only +0x34 skips that), so it
never leaves by itself; the game releases them in the swap: when 0x2307e0 commits a change (0x140400 = 3, `0x22b8e8`,
the fidget timer, `0x230720` / `0x2306c0`, **`0x22efd8` when 0x1413f8 is set**, the wrench flag, the request cleared,
**0x140050 / 0x140054 +0x34 = 1**, the item's blend to sequence 2 over 2 ticks, slot state 3). The port had left that
part out (and the put-away of a weapon that is out). Now `weapons::swap_commit` from `items::update_hand_selected`.
So a swap fades the old layers out over 10 ticks, frees them, and only then makes the new item's (weight 0, 10 ticks
in; 0x1413fb is written by `HeroItemsCreate` 0x22f3c0 when the new item is made).

**No slot animation to port for the hand.** The slot loop `0x231088` deletes slot 0 on its first tick in state 3
(`iVar7 == 0`, gate `0x15f3f8 + 2`), wrapped or not; the back slot waits for the put-away's wrap (the packs' path). So
on a swap the hand item blends toward its sequence 2 for one tick, is deleted, the slot is empty about two ticks, the new
item is made on its sequence 0 (its take-out: the gun unfolding) — the wrench starts on sequence 1 — and Ratchet's arms
move through the holding layers' 10 + 10 tick weight blends and the weapon arm's fade. The "snap" was the stuck layers.

**Sounds.** No sound call in 0x2307e0 (except voice 0x18 on the request 0x1f path, `FUN_00230770`: not on foot),
0x231088, 0x2305e8, 0x22f3c0, `0x22b8e8` or the quick select (0x24d238). The swap sounds are the new item's: `CreateMoby`
→ `InitMobyInstance` 0x263488 → `update_moby_animation_state` 0x263718 sets +0x7e from sequence 0, so the take-out's
trigger words play through `MobyAnimAdvance` in `HeroItemsAttach` (e.g. Blaster 168: sounds 2 / 3 at key times 16 / 88;
R.Y.N.O. 454: 0 / 1; Devastator 157: 2 / 3; Pyrocitor 176: 2, 3, 3; the gloves do not advance, the wrench starts on
sequence 1). The port's hand item was spawned with +0x7e = 0: now `moby_update::anim_sound::init_state` (the table
mobys' `init` uses it too). The put-away sequence 2 has one frame and no triggers.

**Tests**: `hero_guns_novalis` `novalis_swap_releases_the_holding_layers` (two-hand → wrench, two-hand → one-hand,
one-hand → two-hand, quick-select repeats 11 → 15 → 11 → 8 → 23 → 8; the fade 1.0, 0.9 … 0; fails without the
+0x34), `novalis_swap_take_out_sounds` (per tick: the Blaster's 2, 3, the R.Y.N.O.'s 0, 1, none for the wrench, the
Devastator's 2, 3). Frames: scratchpad `weapon_hold/swap_dev_to_wrench.png`, `swap_wrench_to_dev.png`.
Not ported: the item def +0x20 = 0x20 write of the request path (unknown reader).

## 12. The Morph-o-Ray and its chicken (2026-09-28)

**Ids** (the item table 0x179f40 against `lvl.vtbl`): Morph-o-Ray **21** / class 185 → `0x2d2450` on every level
(the same code in all 19 tables); its chicken **270** (0x10e) → `0x2df448` and the feathers **428** (0x1ac) →
`0x2e2f68`, both on every level. Def +0x18 = 1: the hand-item arm's holding class. No ammo, no weapon-check case.
**H** (decompiler output; the disassembly for `0x2df448` (5.6 KB, no Ghidra function), the draw callback `0x2d38b8`
and the pulse loop; an R5900-aware disassembly of lq / sq / por, scratch `morph/mdis2.py`).

**What the game does** (module docs of `hero/morph_ray.rs` and `moby_update/classes/chicken.rs` for the detail):
* **Fire** (`0x2d2450`): ○ held in hero states 0 / 1 / 4 / 5 / 0xc, past the draw's key time 6: the arm up (`0x22ee08`),
  the look stance → first person (`SetState(0x1e, 1)`), the aim from Ratchet's facing (the camera in first person).
  The search `0x2d2018`: creatures (type 5) with a target record, not 0x58e / 0x452, within 8 of the muzzle in a 10°
  cone (the Devastator's cone test with the record's radius), or within 2.5 / 60° outright, clear lines.
* **Eligibility and the meter are the damage record, not the reaction table and not per-class code**: the meter
  starts at the record's health and runs down `3·dt·(1 + 0.33·gold)`; the scale is the record's s16 +0x04 (a scale
  ≤ 1 morphs at once). Every Novalis and Rilgar creature has 1 / 1 (instant); level 07's 871 3 / 3, 13's 111 9 / 9,
  10's 1229 20 / 20, 07's 1106 120 (the boss, excluded as 0x452). The HUD meter is `10000 − 10000·meter/scale`.
* **The morph is a spawn and a delete** (`0x2defb0`): `CreateMoby(0x10e)` at the target's place facing away from
  Ratchet with its lighting, class sound 1, the target's `SetDeathBits` (bolts) and `DeleteMoby` (unless record +0x0e
  = 1 → 2), a type-5 flash; a ring of 20 chickens (0x1dd580) with a knockback and a suck record per slot; the gold
  chicken (0x13e535) has 4 health, is a decoy (+0xbc) the enemies' search prefers, and grows to 4×.
* **The chicken** (`0x2df448`): pecks (1), turns from Ratchet (2), runs (3, 1·size u/s) and sprints (4, 2·size) while
  he is within 5 / 6, every step through the shared move `0x26d270` (`creature::walker::move_ground` / `settle`: the
  ground-step limit, the wall slide, six push-outs); any hit → health; below 1 it bursts (20·size type-22 puffs, or
  the gold burst: `SpawnBeamExplosion` with the colour shift and a damage-1 sphere of 3) into 5–8 feathers and hides
  (state 6); in the Suck Cannon (state 5) through its reaction table `react::CHICKEN` and `react::carried`, a feather
  one tick in four. Clucks (class sound 0 / 1) every 2–5 s.
* **The beam** (`0x2d2d08`, draw `0x2d38b8`): 12 points bending from the last tick's shape to the aim and the target,
  rings of 20, two helix strands, type-53 sparkle pairs at their ends, type-78 sparks (not ported), three pulses (8 u/s,
  0.1 → 1.5); drawn as the tube (FX 13, two counter-scrolled layers, back faces culled), the strands (FX 14 / 16), the
  pulses (FX 8), the muzzle glow (FX 0xb); additive, 0x407f207f (gold 0x60007f00). The light: radius 7.5,
  (1, 1, 0.25)·127/128, a 20-tick fade after the firing. The sound: class sound 0 looping (flags 4).

**Shared pieces (built once) and their users**: `creature::walker::move_ground` / `settle` (`0x26d270`; the chicken;
census consumers 749, 238, 1023, 294, 695: G-ENM-009), `creature::attack::area_hit` (`0x26f8f8` with the per-target
push; the gold chicken; the same code as the Bomb Glove's and the Devastator's blasts), `creature::fx::colour_shift`
/ `beam_explosion_shift` / `LIGHT_BEAM_GOLD` (`0x270fa8`, `param_18` of `SpawnBeamExplosion`; the gold chicken; the
Bomb Glove's colour shift of G-WPN-002 is the same function), `react::morph_target` (`0x2defb0`), the chicken's
reaction table `react::CHICKEN` (existing), `targeting::cone_miss` (the Blaster, the Devastator, now the Morph-o-Ray),
the hero's `PartSpawn::Sparkle` (type 53), `rc-engine/src/reactive_render.rs` (the vortex, the rings, now the beam).

**The loop sound's channel** (the one-tick bug of §10.1): the game keeps each item's loop slot in the item's own
storage (the Pyrocitor +0x4a, the Tesla Claw +0x48, the Suck Cannon / Taunter +0x04, the Morph-o-Ray the global
0x1617f8), so no item releases another's. The port's channels follow that: `fx::LOOP_MORPH` (5) is the Morph-o-Ray's
alone; nothing but its own update and its `item_gone` releases it.

**Native, not emulated**: standard `f32`; the chicken's knockback and suck records live in its own pvars (+0x80 /
+0xe0; the game: global tables by ring slot). **Inferred [L]**: a targeted beam's end point x / y (the decompiler kept
only z); the pulse index past the last point clamped; the search's lines ignore Ratchet (the game: the item for the
cone line); `FUN_002731d0` as an angle wrap; a `GroundHeight` miss keeps the chicken's z (the game reads stale output).

**Not ported** (gaps): particle types 5 (the morph flash) and 78 (the beam's sparks) — G-PRT-001; the HUD meter element
(`queue_animation_update(4, 0x7533)`) — G-UI-011; the skill point for morphing class 625 on level 5 — G-SAV-007; the
shot statistics 0x141728.. — G-SAV-009; the gold Morph-o-Ray (not mirrored) — G-WPN-009; the big-head cheat
manipulator on the chicken (`0x278720`).

**Tests**: `crates/rc-game/tests/weapons/hero_morph.rs` (headless, twice identical): Novalis — a critter 577 morphed (the
beam every tick of the hold, the critter deleted, the chicken at its place changing state, the light faded after);
a big amoeboid 572 with level 07's 3 / 3 record held 60 ticks then morphed; the Suck Cannon (asked for after the
morph) swallowing the chicken (its record 1 → 2 → 3 → 4, held state 5); Rilgar — a small amoeboid 866 morphed.
Unit: `morph_ray::tests` (the beam's shape, the pulses, the quads' culling and the far ring's fade),
`creature::tests` (the morph spawns and deletes, the chicken runs from Ratchet, a hit bursts it into 20 puffs and
5–8 feathers that fall and end, the gold chicken's decoy / size, a displaced chicken goes when out of view,
`move_ground` on a floor and refusing a drop). `novalis_hero_digest` (NO_IDLE) unchanged (md5 596306d7…).

**Frames** (scratch `morph/shots/`, two runs byte-identical): Novalis `RC_GIVE_ITEMS=21
RC_HERO_AT=136.6956,177.82448,40.578,0 RC_PLAY_SCRIPT='40-300:press CIRCLE'` frames 130 / 150 / 175 (the beam at a
hovering critter), 250 (the chicken by the pit); Rilgar `RC_LEVEL=5 RC_HERO_AT=161.52519,326.2638,26.5,1.5708
RC_PLAY_SCRIPT='30-200:press CIRCLE'` frames 32..70 (`grid_r1.png`: the beam, the amoeboid morphed, its bolts).

## 13. The throw gloves: one shared glove update, the Mine Glove, the Decoy Glove, the Drone Device (2026-09-28)

**Ids** (the item table 0x179f40 against `lvl.vtbl`, the same code on all 19 levels): Bomb Glove **10** / class 192 →
`0x2d8330`; Mine Glove **17** / 190 → `0x2d7738` (the mine class **74** `0x2bfe40`); Glove of Doom **20** / 229 →
`0x2dd6c0` (the canister **230** `0x2de650`, which releases the bots **186** `0x2d5c10`); Decoy Glove **25** / 562 →
`0x2ed5d8` (the decoys **203** / gold **1900** `0x2d9fe8`); Drone Device **24** / 483 (no update: the drones **479**
`0x2e92b8`, launched by `0x2e8c20` from `UpdateWrenchSelected` when 0x141345 is set). Every glove's def +0x18 = 0 (no
holding layer), +0x24 / +0x28 / +0x2c = 0x2c / 0x42 / 0x2c, +0x30 = 0 (the Bomb Glove's). **H** (decompiler output,
the disassembly where the decompiler lost arguments or stack copies; data read from the overlay).

**One shared glove update** (`crate::hero::gloves`, module doc): the four updates are copies of one function —
warm-up (Bomb, Mine), the hand point in the glove's frame, the held object's check, `SetState(0x1e, 1)` from the look
stance, the fire trigger (the arm out 17 ticks, ○ in 0x1e with ammo, 0x23 at tick 16 → voice 0x1a, one ammo, state 3)
after the 20-tick lockout, the aim of the held object, the state machine 0..6 and the tail (a new object, or it
follows the hand). A `GloveRow` per glove carries what differs: the hand offset (Bomb / Mine (0, −0.09, −0.02), Doom
(0.01, −0.14, −0.05), Decoy (0, −0.19, −0.11)), the warm-up, the empty throw (Bomb: create; Doom / Decoy: click sound
0; Mine: nothing), the refill rule (Bomb / Decoy: ammo or the arm; Doom: ammo; Mine: ammo after a 10-tick delay),
state 6 (Bomb / Mine delete their object; Doom / Decoy keep it), and the glove's own aim / create / release functions.
The aims share two pieces: the launch point (0.859 ahead-left, 0.49082 up; Doom / Decoy 0.39082) and the first-person
target (`gloves::first_person_target`: the camera's reach `2r²(1 − slope)/g`, the camera's line); the Bomb, Doom and
Decoy velocity solvers are one function (`weapons::launch_velocity`: `0x2d80f0` = `0x2dcf88` = `0x2ece98` with gravity
11 / 9 and the reach 8.5 / 2.5 (3.5 with L1 / L2)); the Mine Glove's lob is its own (`gloves::mine_velocity`,
`0x2d7590`: 45°, speed `√min(g·h²/(2(h − dz)), g·k/2)`, reach 4 / 6). The Bomb Glove's tests and behaviour are
unchanged (`novalis_hero_digest` NO_IDLE md5 596306d7…, identical).

**The landing previews and the reticle are shared too**: `targeting::arc_landing` now takes the copies' rules
(`Prims::Snap { max_rise }` — the bomb's 2-unit rule, none for the decoy / canister; `Prims::Ignore` — the mine: only a
face ends it; `first_test`), and the draws `0x2bf420` (mine) / `0x2d8e28` (decoy, canister) are the bomb's `0x2c23c0`
(`targeting::BOMB_RETICLE`). A quirk reproduced: the decoy's and the canister's first update (state 0) takes the
preview's "flying" branch and marks "a new object in the glove" on themselves, so they never show a reticle; the mine's
held state is 0, so it does (held, and for its first 10 ticks out).

**The Mine Glove's mine** (`moby_update::classes::mine`, module doc for the states, pvars and constants): thrown as a
45° lob, bounces (×0.6, half the vertical, a bob that fades in), lands on ground flatter than 45° once slower than 0.5
u/s and arms at once (the arming timer is cleared on the ground; a mine still out after 120 ticks goes off). Armed and
landed it watches the target list: any target within 1.5 sets it off; the nearest non-crate within `4 + record
radius/8` (×3 while the Taunter lures it) and 2 in height is **sought** — it hops at 4 u/s after it, its 0.35 sphere
setting it off on contact, for at most 600 ticks. Ratchet standing on it pushes it away (`walker::move_ground`, the
shared `0x26d270`). A hit (mask 0x830000) or more than 8 mines (the oldest) set it off. **The explosion**: an area hit
of radius 2, damage 3 (`creature::attack::area_hit`, the shared `0x26f8f8`), `SpawnBeamExplosion` in the air or
`trunc(d · 15)` type-16 puffs on the ground (`d` the xy camera distance), class sound 2, the shake, the Bomb Glove's
explosion light (0x20a890 = 0x20a930). **In
water** it uses the bomb's water code (`classes::bomb_water`, written for both: the entry splash, the sinking bubbles,
now also `bubbles` and `scorch` with the mine's counts). A bug fixed on the way: the water entry makes **16** drops,
not 15 (the loop counts 0xf down to −1, like the other loops' 0x95 → 150, 99 → 100); the bomb's ledger test updated.

**The Decoy Glove's decoy** (`moby_update::classes::decoy`, module doc): flies under gravity 9 through the shared move
`walker::move_collide` (`0x26d610`), bounces (×0.2), lands on ground less than 35° from flat and inflates (sequence 1),
wobbles (sequence 2 at a random speed), falls to the ground, and pops on water / lava-like surfaces; a Ratchet stuck on
top pushes it away. At most 6 (the oldest shrinks away). **How creatures go for it**: nothing in the decoy talks to
them — the creatures' own target search (`creature::target::acquire` `0x274b78`, ported before) takes a decoy in
state 3 nearer than its range instead of Ratchet, so 577 (Novalis), 572 / 865 / 866 (Novalis, Rilgar) and 459 walk to
it and attack it with their usual hit; the decoy takes the hits (mask 0x30001, one attacker every 30 ticks) off its
4 health with a red flash (`creature::flash`), and bursts below 0 (`SpawnBeamExplosion`: flashes 4 / 3, light 12,
sound 2, shake).

**The Drone Device's drones** (`moby_update::classes::drone`, module doc): the Drone Device is not thrown. Picking it
in the quick select (`0x24d238`), buying it (`0x2af7e8`) or granting it (`RC_GIVE_ITEMS=24`) sets 0x141345
(`SessionState::drone` → `ItemGlobals::drone`); the next `UpdateWrenchSelected` clears it and the request and
launches (`0x2e8c20`, `hero::gadgets::launch_drones` on the moby world): with ammo and fewer than 6 drones, one ammo,
the missing drones one up and one ahead of Ratchet, thrown out at 0.25 a tick. Each drone **orbits** him (its slot
angle plus a phase that turns 0.05 a round, radius 1, a slow tilt, at most 10 u/s through `walker::move_collide`),
**attacks** the target the round gives it (the nearest enemy of class type 5 / 7 / 8 — a creature must be targetable
— drawn last frame, within 4 / 5.3 / 6 of Ratchet's feet, not held by the Suck Cannon, with a clear line: `0x2e8e88`),
homing on its aim point led by its motion, and on contact gives it a hit (damage 1, flags 0x10000); on a creature it
blows up with it (`SpawnBeamExplosion`: flashes 0.4 / 0.2, light 8, a spark pair and a puff, sound 1, the shake,
debris 1) and fades out (alpha −4 a tick). An enemy touching an orbiting drone is struck the same way; an enemy's hit
kills a drone. Crates are not targets (class type 0).

**The Taunter lures mines** (its second loop, `0x2cc830` over the list 0x1b0c30, now ported in `hero::taunter::lure`):
mines within the reach (xy) and 55° of Ratchet's facing get +0x78; the mine triples its seek reach while lured.

**Shared pieces and their users**: `hero::gloves::update` (Bomb, Mine, Decoy Gloves); `gloves::wall_probe` (the mine's
and the decoy's release lines through the hit sink: the world and the mobys); `gloves::first_person_target`
(the three); `weapons::launch_velocity` (Bomb, Decoy); `targeting::arc_landing` + `Prims` (bomb, mine, decoy) and the
reticle (the three); `classes::bomb_water::{entry, sink_bubble, bubbles, scorch, part34/35/64}` (bomb, mine);
`creature::walker::{move_collide, move_ground}` (decoy, mine; before: the chicken, 577, 572); `creature::attack::area_hit`
(mine; before: the gold chicken); `creature::target::acquire` (the decoy's effect on 577 / 572 / 866 / 459);
`creature::flash` (decoy; before: 577); `creature::fx::{beam_explosion, death_explosion, light_spawn, LIGHT_BOMB}`
(mine, decoy, drones); `creature::turn::spring_turn` (drones); `creature::react::rec_state` (the drones' "not in the
Suck Cannon"); `Services::targets` (0x1abe80 for class updates: the mine; the scheduler fills it); `Services::drones`
(the drones' globals 0x141344..).

**Coverage** (every call and branch of the ported functions; status: **P** ported (file::fn), **G** not ported (gap
id), **n/a** (reason)). Level01 addresses.

*The shared glove update* `0x2d8330` (Bomb) / `0x2d7738` (Mine) / `0x2dd6c0` (Doom) / `0x2ed5d8` (Decoy):

| address / branch | what it does | status |
|---|---|---|
| +0x58 (Mine +0x5c) < 3 | warm-up (Bomb, Mine) | P `gloves::update` step 1 |
| `fun_001fa2b8` / `fun_001fa2d8` (0x167100) | the camera matrix for the first-person aim | P `ItemEnv::camera` |
| `FUN_002645a8(glove, 0)`, `fun_0020cca8`, `fun_001f9d20`, `fun_001f9a10` | the hand point +0x40: joint list 0 + the row's offset in the glove's frame | P `gloves::update` step 2 |
| +0x50 held: +0x98 = 1; state −2 / −3 → 0 | the object in the glove | P step 3 (the port also drops an object of another class) |
| 0x1413d4 == 1 → `FUN_0023cf98(0x1e, 1)` | look stance → first person | P `weapons::after_items` |
| `FastDecTimer` +0x54 (Mine +0x58) | the fire lockout | P step 5 |
| 0x1413fc; arm 0x1413f8 && 0x13f50c == ticks(17); ○ (0x13cae4 & 0x1403f0) in 0x1e with ammo; 0x23 at 0x13f4e8 == ticks(16) | the fire trigger | P step 5 |
| `FUN_00236738(0x1a, 0)` | Ratchet's throw voice | P (`SoundCmd::Voice`); test `novalis_*` (heard (0, 0x1a)) |
| `FUN_00249450(−1, 1)` | one ammo (and the used stat 0x13dea0) | P `Weapons::use_ammo`; tests (ammo) |
| +0x20 = 3 | the throw | P |
| the aim (inline / `0x2ed0c0` / `0x2dd1b0`) | see the rows below | P per glove |
| case 0 / 1 (+0x70 & 2) | the glove's animation wrap → 2 | P |
| case 3, no object | Bomb: `FUN_002c2640` (create); Doom / Decoy: `fun_0022da68(0, 0, glove)` click; Mine: nothing | P (`Empty`); test `novalis_decoy_glove_clicks_without_a_decoy` (the object lost before tick 16: the click, nothing thrown, state 4) |
| case 3: the release (`0x2c27a8` / `0x2bf7d8` / `0x2ddf58` / `0x2d94f0`) | see below | P per glove |
| case 3: `s16 0x1416d0++`, 0x1416d2 (minutes), 0x1416d4 level bits (Mine 0x141708.., Doom 0x141720.., Decoy 0x141748..) | the throw statistics | **G-SAV-009** (counted in `Weapons::throws`) |
| case 3 Mine: `FUN_00274640(mine, 0x1b0c30, 0x3f)` | the mines' list | P (the table's mines; the Taunter's loop walks them) |
| case 3 Mine: +0x54 = ticks(10) | no new mine for 10 ticks | P (`Refill::AmmoAfterDelay`); test `novalis_mine_seeks_a_critter` (9 ticks after the throw tick) |
| case 3: +0x20 = 4, lockout ticks(20) | | P |
| case 4 | → 2 unless 0x23 or the arm | P |
| case 5 / 6 | 6; Bomb deletes (kept pointer), Mine deletes and clears, Doom / Decoy return | P (`OnDelete`) |
| tail | a new object: Bomb / Decoy ammo or the arm, Doom ammo, Mine +0x54 == 0 and ammo; else it follows the hand | P (`Refill`); tests (the decoy at once, the mine 9 ticks later) |
| Decoy: gold byte 0x13e539 and the held decoy not 0x76c → list out, `DeleteMoby`, re-create | the gold decoy swap | **G-WPN-009** |
| gold reach (0x13e52a, 0x13e531, 0x13e534, 0x13e539: 8.5 + 2.5·gold, 12, 3.5 …) | the gold gloves | **G-WPN-009** |

*The aims and solvers*: `0x2d80f0` = `0x2dcf88` = `0x2ece98` (P `weapons::launch_velocity`), `0x2d7590` (P
`gloves::mine_velocity`), the first-person reach (P `gloves::first_person_target`; `CollLine_Fix(camera, …, 2)` with
Ratchet's hit ignored — the port tests the world alone [L], as the Bomb Glove did), the launch point 0.49082 / 0.39082
(P `gloves::launch_at`), 0x13fda0 = 0 without L1 / L2 (P).

*The mine* `0x2bfe40` (+ `0x2bf6a0` create, `0x2bf7d8` release, `0x2bfa78` preview, `0x2bf420` draw):

| address / branch | what it does | status |
|---|---|---|
| `0x2bf6a0`: `CreateMoby(0x4a)`, +0x30 0xff, +0x32 0x7f, +0x31 1, scale ×0.5, state 0, owner, +0x36 / +0x78 = 0, hand point, vel 0, 2 × `randf(60°·dt, 180°·dt)` +0x38 / +0x3c, +0x68 −1, +0x6c, +0x5c = frame, +0x60 / +0x64 / +0x70 / +0x74 = 0, +0x94 = 0, `fun_0020def8` | the mine in the glove | P `mine::init_held` (the rotation build is the renderer's: n/a) |
| `0x2bf7d8`: 3 × `randf(±1)` + `randf(0.01·dt, 0.1·dt)` nudge, state 1, scale ÷0.65, +0x34 = ticks(120), > 7 other mines → oldest +0x5c = −1, at the launch point, `CollLine_Fix((Ratchet.xy, z), point, 0, Ratchet)` → on the wall 9.7·dt² off, vel reflected ×0.6 | the release | P `mine::release` / `release_hit` + `gloves::wall_probe` |
| 0x13f64c / `FUN_00275290` / `FUN_002752c0` | moving platforms (release, preview, landing, riding) | n/a (0x13f64c never set in the port: G-WPN-008 lists the platform previews) |
| 0x1413f5 / 0x1413ff | hidden in first person or with the hand hidden (held) | P |
| hand not 0xbe or 0x1413f4 ≠ 0, held → `DeleteMoby` | | P |
| owner alive, class 0xbe, +0x36 == 0, vel ≠ 0 → `0x2bfa78(…, 1)` | the preview | P `mine::preview` (the reticle every held tick: test) |
| held: Euler(π, 0, 0) × `MobyAttachToJoint(glove, 0)` → rows; +0x94 = 0; mode \| 0x100 | the held mine's frame | P (the glove's joint-0 matrix kept by `gloves::update` step 2 as `Glove::hand_mtx`, then turned by π) |
| out: pos += vel; gravity 9.8·dt² (in water ÷10) | | P |
| in water: `randi(ticks(6) − 1)` → `SetWaterLevel`, `PartType34Spawn(randf(0.05, 0.1)·210000, level, pos, vel/4)` | sinking bubbles | P `bomb_water::sink_bubble` |
| the path's template (damage 3, 0x830000, type 2 / 1, the class) | hits on its path (creatures, crates) | P; test `novalis_mine_breaks_a_crate` (the crate broken, its bolts) |
| −0.4 and the bob's undo | | P |
| `CollLine_Fix(old, pos, 0x10, mine, tmpl)` | the path | P (`line_hit_in`) |
| water face (CollType 0), vz < 0 → `RippleDisturb`, +0x50 = 1, vel (0, 0, −1.5·dt), `FUN_002ff768(2, …)` +0x23 = 0x70, 16 × `PartType35Spawn` | the water entry | P `bomb_water::entry` (16 drops: the bomb's 15 corrected) |
| world face, not water, vz ≥ −9.8·dt → the bounce (9.7·dt² off, ×0.6, vz ×0.5, the bob's start `+0x6c/70/74`), floor > 45° and falling → z ≥ ground, vz flipped, slow → landed (+0x94 = collision) | | P `mine::ground_bounce`; tests (landed) |
| the tilt (`FUN_00222420`, 3°) | | P `mine::orient` |
| a crate (`FUN_00273278`: 500..540) or mode 0x5000, not a mine / Ratchet / the glove → explode (not state 1: at the hit, on the ground) | | P; test `novalis_mine_breaks_a_crate` |
| Ratchet / the glove (kind < 1: nothing) or another moby → the landing test; another mine → bounce ×0.4 or explode | | P |
| else → explode (the drift = reflected ·2·dt, the normal) | | P |
| landed: `GroundHeight`; 40·dt² above → out again (loop released, vz 0, +0x68 −1, +0x34 ticks(120)); else snapped, +0x34 = 0 (so it arms at once) | | P; test (armed on landing) |
| target gone / untargetable while seeking → out again, `MobyAnimBlend(0, 0, 10)` | | P `mine::stop_seek` |
| seeking: seq 1 → 2 on the wrap; `SoundIsAlive` / `PlayClassSound(1, 4)`; speed +15·dt² ≤ 4·dt; the hop (0.05, `ticks(7)`); `FUN_0026e808` (damage 1, 0x10000) + `coll_sphere_mobys(0.35)` → any moby but 0x363 / 0x4a / 0x458 / 0x365 / 0x367 sets it off; `coll_sphere(0.25)` push-out; +0x58 `ticks(600)` → off | | P `mine::landed`; test (sounds (74, 1, 4)) |
| `FUN_00277a80(0.525, 30°·dt, 17°·dt)` | the wobble | P |
| landed: vel = pos − old | | P |
| unarmed and `FastDecTimer`(+0x34) → armed, +0x54 = 0 | | P |
| `MobyGetHitMessage(0x830000)`: an attacker not a mine → off; +0xa4 = 0xff | | P |
| armed, not landed → off; seeking and Ratchet within 0.75 (xy) and 0.25 (z) → off | | P |
| Ratchet ≥ 0.75 (3-D): the search over 0x1abe80 (≤ 30; < 1.5 → off; crates skipped; `4 + record +0x0a/8` (×3 lured, 12 gold); xy; \|dz\| < 2; the nearest) → seeking (blend 1, `PlayClassSound(0)`, +0x40 = 0, +0x58, +0x52, +0x48) | the proximity search (targets: the target list's mobys; crates set it off but are not sought) | P `mine::seek_search`; tests (577 sought, 866 within 1.5, the crate not sought) |
| Ratchet < 0.75: `FUN_0026d270(0.4, 0.25, 0.25, 0.525)` toward 0.75 out | pushed from under him | P (`walker::move_ground`) |
| +0x78 = 0; +0x5c < 0 or `0x1613d0` → `FUN_00273f50(0.33, 13, mine, pos, −1)`, list out, `DeleteMoby` | the oldest-mine rule / all mines off | P (0x1613d0 not read: its writer is not ported, 0 in the data) |
| the bob: ×(1 − 0.012) to 0.05, the fade over 6 ticks, phase +2π·dt, +0.4 | | P |
| +0xbc == 0 → `FUN_0026eec8(0.25)` | the blob shadow | **G-REN-025** |
| explode: `coll_sphere_mobys(2)` + `FUN_0026f8f8(3, 0.25, 1.5, …, 0x810000, 4, 1)` | the area hit (creatures, crates, …) | P `attack::area_hit`; tests (the critter killed) |
| `FUN_0026e690`; 2 up → `SpawnBeamExplosion(0, 0, 2, 1, 4, 1, 7, mine, 0, 0, 3, 3, 5, 2, 0, 1, −1, gold)`; else `trunc(d · 15)` × `PartType16Spawn` (4 kinds, colours through `FUN_00270fa8`; `d` the xy camera distance) | | P `fx::beam_explosion` / `mine::smoke`; test (the puffs or the beam) |
| in water: 150 × `PartType34Spawn`, 150 × `PartType64Spawn`, 20 × `PartType15Spawn` from `GroundHeight(pos + 3.5)` | | P `bomb_water::{bubbles, scorch}`; test `mine::tests::explodes_in_water` (a mine entering water goes off the next tick: its path starts at its shown position, 0.4 up, and meets the surface again — the game's code, reproduced) |
| `PlayClassSound(2 (3 gold))`, the shake (0x167260 / 0x167268), `FUN_002f3570(0x20a890 (0x20a8e0 gold))`, list out, `DeleteMoby` | | P (the light = `LIGHT_BOMB`, identical data); test (sound 2, light, shake) |
| `0x2bfa78` / `0x2bf420` | the preview (gravity 9.8, 120 ticks, faces only) and the reticle | P `mine::preview` |

*The decoy* `0x2d9fe8` (+ `0x2d9228` create, `0x2d94f0` release, `0x2d9760` preview, `0x2d8e28` draw, `0x2d9d08` push,
`0x2d9f90` / `0x2d90a8` pop):

| address / branch | what it does | status |
|---|---|---|
| `0x2d9228`: class 0xcb (0x76c gold), +0x30 / +0x32 0xff, +0x31 1, state 0, the hand point, +0x10 = the point, owner, +0x44 / +0x54 = 0, scale × 0.5, health (gold + 1)·4, seq-0 blend (a no-op on a new moby), +0x94 = 0, +0x98 = −1, yaw `randf(±π)`, +0x58 / +0x5c = 0, the cap (> 5 listed: the oldest below state 5 → 6), `fun_0020def8`, `FUN_00272078` (Ratchet's +0x38 lighting), the flash record (0x80, 4, 15), list in, hidden in first person | | P `decoy::init_held` / `cap_count` |
| `0x2d94f0`: the release from the hand point, the line (Ratchet → 0.2 past, flags 0, the mobys too) → state 2, or on the wall: vel 0, state 4, `ticks(30)`, the tilt | | P `decoy::release` + `gloves::wall_probe` |
| out of [2, 1021]³ → list out, delete | | P |
| hand not 0x232 / 0x1413f4 ≠ 0, held → delete | | P |
| +0x54++ ; hidden in first person (state ≤ 1) | | P |
| the preview (owner 0x232, +0x46 == 0, vel ≠ 0); the first update's "flying" branch sets +0x46 on itself | | P `decoy::preview` (never draws: the game's quirk, reproduced) |
| 0 → 1 (+0x44, +0x98 −1, +0x6e, +0x6c) | | P |
| 1: owner alive and in hand → nothing; else delete | | P |
| 2: grow 0.02 a tick; −9·dt²; apart from the listed decoys (1.8, ×0.015); Ratchet's +0x94 off in 0x1e; `FUN_0026d610(0.2, 0.2, 0)`; walls → `FUN_00212960(0.23)`; pushed from Ratchet when falling; reflected ×0.2 (×1.05 off a primitive); slow on < 35° → +0x44++ and land (a primitive after 240); land: blend 1 (ticks(10)), +0x98 0, +0x94 collision, vel 0, state 3, +0x44 ticks(600), z, the tilt | | P `decoy::fly` / `land`; tests (flew, landed) |
| `0x2d9f90` → `0x2d90a8` on surfaces 0, 1, 3, 8, 0xb, 0xc, 0xd: 10 × (3 `randf` + 3 `rand` + `randf` + `rand`) `PartType05Spawn`, list out, delete | the pop | P `decoy::pop_surface` (type 5 records, live since 2026-09-29 `particles::type05`; was **G-PRT-001**); test `decoy::tests::pops_on_a_pop_surface` (surface 1: 10 puffs, deleted; surface 2: stands) |
| `FUN_00220ed8`(+0x6f): the bounce sound 0 every 20 ticks, else `FUN_002747a0(dt)` jitter | | P |
| 3 / 4: `MobyGetHitMessage(0x30001)`; the tilt; the push; +0x5c, +0x6c; one attacker per 30 ticks; type 0x101 → +0x6c ticks(100); `fun_0022da68(0)`; +0x07 = 0xfa; health −= damage; `FUN_00272318` flash | the hits (creatures' bites and strikes, bombs) | P `decoy::stand`; tests (bites, sound 0, the red flash) |
| health < 0: `FUN_002747a0(0.5)` jitter, `SpawnBeamExplosion(0, 0, 4, 3, 9, 1, 12, decoy, drift, 0, 0, 0, 0, 2, 1, 1, −1, gold)`, gold: `FUN_0026e7d8` + sphere hits, list out, delete | the burst | P (gold: **G-WPN-009**); test (sound 2, light, shake) |
| +0xa4 0xff; `FUN_002723f8`; the wrap → blend 2 (ticks(10)) and speed `randf(0.7, 1.05)` | | P |
| 4: grow; +0x44 → blend 0 (ticks(60)), list out, 5 | | P |
| 3: `FUN_0026e618`, the pop test, the fall (9.8·dt²) and the tilt; vel 0 on the ground; the platform ride | | P (the ride: n/a, 0x13f64c) |
| 5: the wrap → speed 0; +0x44 or the wrap → 6 | | P |
| 6: shrink 2 % a tick; below a tenth → delete | | P |
| `0x2d9d08`: 0x13f532 (Ratchet stuck on top) within 1 / 1.3 → the spring push (`0x270780`), `FUN_0026d610(0.5, 0.4, 0)`, the fall 24·dt² | | P `decoy::push`; test `decoy::tests::slides_from_under_ratchet` (pushed to about 1.4, the push ends when he is off) |
| effect on creatures | `0x274b78` / `0x274df8` take a state-3 decoy nearer than their range | P (ported before); tests 577, 866 |

*The drones* `0x2e92b8` (+ `0x2e8c20` launch, `0x2e9150` create, `0x2e8e88` search, `0x2e8dd8` trails):

| address / branch | what it does | status |
|---|---|---|
| 0x141345 set by the quick select `0x24d238`, the vendor `0x2af7e8`, the Gadgets page `0x28f260` (+ 0x141347 += ammo) | the request | P quick select / vendor / `RC_GIVE_ITEMS`; the Gadgets page **G-WPN-002** |
| `UpdateWrenchSelected`: 0x141345 → `0x2e8c20`, 0x141408 = 0 | | P `items::update_hand_selected` + `gadgets::launch_drones` |
| `0x2e8c20`: 0x141345 = 0; ammo; cos / sin of 0x13f3e8 ·0.25; count < 6 → `FUN_00249450(0x18, 1)`, stats 0x141740.., feet + rows 2 + 0, `0x2e9150` × (6 − count), vel | | P `drone::launch` (stats **G-SAV-009**); test (6 drones, one ammo) |
| `0x2e9150`: `CreateMoby(0x1df)`, distances, visible, state 2, +0xbc 0, +0x44 prev, count++, `fun_0020def8`, lighting, +0x28 / +0x24 / +0x20, slot, +0x56 100, +0x3c, +0x94 0 | | P `drone::create` |
| game modes 2 / 5 / 6 or hero state 0x32 → hidden, return | | P |
| the round: counter, slots rebuilt (deleted / other class freed), taken list, live count, phase +0.05; `0x2e8e88`; the nearest free drone → state 3, `fun_0022da68(rand % 3 → 0 / 2 / 3, 0, this drone)`, taken | | P `drone::round`; test (the sound on the updating drone) |
| `0x2e8e88`: the moby list's class types 5 / 7 / 8, not taken, type 5 targetable, drawn (+0x31) or class 0x350 / 0x31, 0x4d6's own radius, 4 / 5.3 / 6 from the feet, `FUN_00304100` suck record +0x68 < 1, `CollLine_Fix(feet, aim + 0.05, 6, it)` | the drones' targets (enemies only; crates are type 0) | P `drone::search` (0x4d6: **G-WPN-002**) |
| the slot angle +0x40, the tilt goal +0x34 | | P |
| `MobyGetHitMessage(1)`: an attacker of class type 5..8 (0x4d6: +0x56 budget) → 4 | | P (0x4d6: G-WPN-002) |
| 3: collision on, scale → class, trails, the target check, the lead ×3.1, 20 % a tick, `FUN_00212960(0.25)`: the target → `FUN_0026e7d8` + `FUN_0026e968` (damage 1, 0x10000, type 1 / 2), a creature → `SpawnBeamExplosion(0, 0, 0.4, 0.2, 4, 1, 8, drone, vel, 0, 0, 1, 1, 1, 1, 1, −1)` → 4; else the push-out; the facing | | P `drone::attack` / `strike`; tests (the critter killed, sound 1, light, shake) |
| 2: collision off, trails, scale, radius → 1, the angle and the tilt, the circle turned by the tilt, `FUN_0026d610(0, 0.18, 0)`, stuck unseen → behind the camera (0x167240 + 0x167470 − 0x167450), `FUN_00212960(0.2)`: Ratchet skipped, type 5 / 7 / 8 → the strike; vel; `SpringTurn` ×2 | | P `drone::orbit`; test `drone::tests::stuck_out_of_view_jumps_behind_the_camera` (camera + up − forward; a drawn drone stays) |
| 4: following, alpha −4 → 5 | | P `drone::fade`; test (the drone gone after its strike) |
| 5: count −1, slot freed, delete | | P |
| `FUN_0026eec8(0.17)` | the blob shadow | **G-REN-025** |
| `0x2e8dd8`: 2 × `PartType55Spawn(0.05, 0.01, drone, list 0 / 1, …)` | the trails | P as records (type 55: **G-PRT-001**); test (12 records) |

**A mine in water**: the mine's path test starts at its position at the update's start — the shown one, 0.4 above
the physical (the disassembly's quadword copy before the move and the −0.4) — so the tick after a water entry its path
meets the surface again while in water, and it goes off in its water burst at once; the sinking branch never lasts.
Reproduced (`mine::tests::explodes_in_water`).

**Native, not emulated**: standard `f32`; the rand draws are the game's, at its points. **Inferred [L]** (module docs):
the gloves' pvars start clear with the hand moby's creation; the hit templates' unwritten bytes; the held mine's
orientation (the glove's joint-0 frame turned by π) is written (`Glove::hand_mtx`); the decoy / mine lists 0x1b0cb0 / 0x1b0c30 are the table's
objects; objects on moving platforms do not ride them (0x13f64c never set); the gold gloves are not mirrored.

**Tests** (`crates/rc-game/tests/weapons/hero_gloves.rs`, headless on the arrival state with `GiveItem(id, equip)`, every run
twice identical; the rows carry the objects, their class sounds, the hero-side sounds, particle counts, lights, the
shake and the bolts): Novalis — the decoy thrown from the throw state (voice 0x1a, one ammo, the next decoy at once),
held → flying → standing, a critter 577 comes to it and bites it 4 → 3 → 2 → 1 → 0 (a class sound 0 and the red flash
a bite) → burst (sound 2, a light, the shake) while Ratchet is untouched, then goes for him; the mine thrown (voice,
one ammo, the reticle every held tick, the next mine 9 ticks later), out → landed → armed / seeking (sounds 0 and the
loop 1), the critter blown up (sound 2, the ground's type-16 puffs or the air's beam explosion, a light, the shake); a mine thrown
at crate 376 goes off on its way, breaks it and its bolts fly (crates are not sought); the Taunter (swapped in) lures a
landed mine (+0x78); the Drone Device launches six drones at once for one ammo (12 trail records), one takes the
critter coming for Ratchet (its assignment sound), strikes it (sound 1, light, shake) and goes, five orbit on.
Rilgar — the decoy struck twice by the small amoeboid 866 (Ratchet untouched); a mine landing by an 866 goes off at
once and deletes it; the drones strike an 866. The Decoy Glove with its decoy lost before the trigger clicks and throws
nothing. Unit (`classes::{mine, decoy, drone}::tests`, on a small moby bench in `bomb_water::tests`): a mine into water
(16 drops, the water burst the next tick: 150 bubbles, 150 scorch, 20 sparks, sound 2, the light); a decoy popping on
surface 1 (10 type-5 records) and standing on surface 2; a decoy sliding from under a stuck Ratchet; a stuck drone
out of view jumping behind the camera. The Bomb Glove's unit / integration tests, the guns, reactive, morph,
creature and gadgets tests: green; `novalis_hero_digest` (NO_IDLE) md5 596306d7…, unchanged.

**Frames** (scratch `gloves/shots/`, two runs byte-identical; `RC_SCENE=0 RC_AUDIO=0`; windows from frame 1 — a window
starting later is not frame-exact before it): Novalis decoy `RC_GIVE_ITEMS=25 RC_HERO_AT=148.6956,177.82448,40.5,3.14159
RC_PLAY_SCRIPT='40-41:press CIRCLE' RC_DUMP_FRAMES=60..240` (`grid_decoy.png`: frames 75 / 100 / 150 / 210, the
inflated decoy with a critter at it); Novalis mine `RC_GIVE_ITEMS=17`, same place and script, frames 20..200
(`grid_mine.png`: 30 / 100 / 150 / 175, the mine landed, the explosion and the critter's bolts); Novalis drones
`RC_GIVE_ITEMS=24`, same place, frames 1..200 (`drone_a/`, `grid_drone.png`: 40 / 118 / 130 / 160, six drones round
Ratchet, one striking a critter); Rilgar decoy `RC_LEVEL=5 RC_GIVE_ITEMS=25 RC_HERO_AT=161.52519,325.2638,26.5,1.5708
RC_PLAY_SCRIPT='30-31:press CIRCLE'` frames 20..300 (`grid_rdecoy.png`: the decoy with the amoeboid at it); Rilgar
drones `RC_LEVEL=5 RC_GIVE_ITEMS=24 RC_HERO_AT=161.52519,327.7638,26.5,1.5708` frames 20..200 (`grid_rdrone.png`).

## 14. The enemy-reaction layer: a coverage audit (2026-09-28)

**Question** (the user's): which reactions work, and for which enemies. **Method**: the static call graph of every
level overlay (scratch `wvr/cg.py`, `wvr/audit.py`: `jal` targets, the class table `lvl.vtbl`, the level copies of
`MobyGetHitMessage` 0x26f320 and the resolver `0x26f378` found by masked code identity), the call sites' mask
arguments read from the disassembly (`wvr/masks.py`), and the census's placed / created counts
(`work/census/classes.tsv`). Hit-record readers are the class updates that reach `0x26f320` directly or through their
own helpers (the hero's SetState, which reads Ratchet's record, excluded). **H** for every mask and call below.

### 14.1 The hit records: writers (the weapons' templates)

| writer | game | flags | type / sub | damage | port |
|---|---|---|---|---|---|
| wrench swing (combo, jump attack) | `0x2be1c0` lines + sphere | 0x10000 | 0 / 1, class 0x47 | 1 (jump 2) | `melee::wrench_update` |
| jump attack shockwave | `0x2370b8` case 0x14 | 0x10000 | 0 / 1 | 2 | `melee::jump_attack_shockwave` |
| thrown wrench (Comet-Strike) | `0x2be1c0` state 10 / 11 | 0x10000 | | 1 | `comet` |
| **Walloper lunge** | `0x2370b8` case 0x20 | 0x30000 | 0 / 3, class 180 | 3 | `walloper::deliver_hits` (§15) |
| Bomb Glove bomb, mine 74, Devastator 153 / R.Y.N.O. 457 missiles, the Suck Cannon's burst, the gold chicken | `0x26f8f8` / `coll_sphere_mobys` | 0x830000 | 2 / 1 or 3 / 1..3 | 2 or 3 | `bomb`, `mine`, `devastator_missile`, `ryno_missile`, `react::burst`, `chicken` |
| Blaster shot 305 | `0x2e2170` | 0x10001 | 1 / 1 | 0.25 | `blaster_shot` |
| Pyrocitor | `0x2cd458` spheres | 0x10000 | **5** / 0..1 (burn) | 1 (+gold) | `pyrocitor` |
| Tesla Claw | `0x2ce448` | 0x210000 | **5** / 1..3 (burn) | 2 (+gold) | `tesla` |
| Suck Cannon, a fired creature | `react::flight` (0x305260) | 0x430000 | 1 / 1, class 0x351 | 2 | `react` |
| Taunter's crate hit, mine seek, drones 479 | `0x2cc830`, `0x2bfe40`, `0x2e92b8` | 0x10000 | | 1 | `taunter`, `mine`, `drone` |
| creatures' beam explosions with damage | `SpawnBeamExplosion` 0x273310 | 0x810001 | 2 / 1 | param | `creature::fx::beam_explosion` |
| camera line into a crate | 0x312ef8 | 0x800000 | 3 / 3 | 20 | `tick.rs` |
| Glove of Doom bots 186 | `0x2d5c10` | — | | | **not ported** (G-WPN-002) |
| Visibomb missile 172 | `0x2cbda8` → `0x26f8f8` | — | | | ported (§17: the path's record, damage 6, flags 0x830000; the blast sphere 4, `attack::area_hit`) |

### 14.2 The hit records: readers on the ported levels (01 Novalis, 05 Rilgar, 11 Pokitaru, 13 Gemlik)

`R` = through the resolver `0x26f378` (damage, knockback, burn), `T` = a Suck Cannon reaction table, `L` = reads the
lure (damage record +0x18), `M` = morphable (class type 5 with a damage record; the Morph-o-Ray's search).

| class (levels) | game update | mask | R / T / L / M | port reacts |
|---|---|---|---|---|
| 577 critter (01) | `0x2efc60` | 0x330000 | R T L M | **yes** (`critter`, `react::CRITTER`) |
| 572 / 866 amoeboids (01, 05, 11) | `0x2edca0` → `0x2eec68` | 0x330000 | R T L M | **yes** (`amoeboid`, `react::AMOEBOID_572 / _866`) |
| 865 amoeboid (01, 05, 11) | same | 0x330000 | R L M (default table: never sucked) | **yes** |
| 459 robot trooper (01) | `0x2e6bf0` | 0x210000 | R L M | **yes** (`path_enemy`) |
| 660 Blarg flyer (01) | `0x2f4428` (+ `0x2f5168`'s 0x210000 block, levels 3 / 9 only) | 0x800000 | — | **yes**: killed; the blast **fixed here** (§14.4); skill point / banner / wreck 1510 not (G-SAV-007, G-CLS-015) |
| 688 gunship (01) | `0x2f7728` | 0x800000 | — | **yes** (the blast; skill point / wreck not) |
| 666 dropship (01) | the driver only (0x210000, levels 3 / 9) | — | — | n/a on 01 (reads no hit there) |
| 1818 Sonic Summoner (01..) | `0x30df40` | — (no reader) | | n/a (immune in the game too) |
| 270 chicken (every level) | `0x2df448` | every flag | T | **yes** (`chicken`) |
| crates 500 / 501 / 502 / 505 / 511 | `0x2ea178` | 0x1830000 | R | **yes** (`crate_`) |
| 704 rock, 754 pots, 778 pipe, 1042 shootables, 1813 | own updates | 0x10000 | | **yes** (`breakables`, `props`) |
| 709–711 shell walls | `0x2f9d80` | 0x830000 | | **yes** |
| 729 big wall | `0x2fa800` | 0x800000 | | **yes** |
| 74 mine, 203 / 1900 decoys, 479 drones | own | 0x830000 / 0x30001 / 1 | | **yes** |
| 186 Doom bots, 166 (not placed on 01) | `0x2d5c10`, `0x2c90c0` | 0x800001, 0x10000 | | no (G-WPN-002; 166 unplaced) |
| Rilgar 79, 133, 623 (R), 625 (R), 846, 1511 | own | | | **no** (G-ENM-001) |
| Pokitaru 1231 (R), 1242, 1246 (R T), 1264, 1319 | own | | | **no** (G-ENM-001); 1859 breakable: yes |
| Gemlik 63 (R T), 69, 101 / 111 / 170 / 231 / 388 / 1261 / 1262 / 1805 / 1885 (R), 212 | own | | | **no** (G-ENM-001) |

All 19 levels: 171 class-levels / **2,874 created instances** read hit records without a port (85 through the resolver,
17 with a reaction table); the ported readers are 130 class-levels / 6,073 instances.

**Which enemy reacts to which weapon** (mask ∩ flags ≠ 0; the class's own code then decides): 577 and the amoeboids
(0x330000) react to **every** weapon (wrench, Walloper, Blaster, Pyrocitor and Tesla Claw with the burn, the explosions,
the fired creatures); 459 (0x210000) to every weapon too (all carry 0x10000 or 0x200000); the flyers and the gunship
(0x800000) **only to explosions** (bomb, mine, Devastator, R.Y.N.O., the cannon's burst); the chicken to everything; the
dropship and the Sonic Summoner to nothing, as in the game. Beyond hits: the Suck Cannon takes 577, 572, 866 and the
chicken (reaction tables); the Taunter lures 577, the amoeboids and 459 (and triples a mine's reach); the decoys draw
577, the amoeboids and 459; the Morph-o-Ray morphs any class-type-5 creature with a damage record (577, 572 / 865 / 866,
459 on Novalis; 866 on Rilgar).

### 14.3 The per-class reaction tables (the Suck Cannon), the lure, the morph meter

| row | game | port |
|---|---|---|
| table slot +0x00 suck start (`0x304168`) | 577 `0x2f1c78`, 572 / 866 `0x2efa88`, 270 `0x2e0a28` | `react::slot_start` / `approach` |
| slot +0x04 swallow (`0x304390`) | wrappers +0x50 | `react::slot_swallow` / `swallow` |
| slot +0x08 fire out (`0x3044a0`; 577 / 270 bounce sound 2 / 1) | wrappers | `react::slot_fire` / `fire_out` |
| slot +0x0c let go (`0x304690`) | wrappers | `react::slot_let_go` / `let_go` |
| slot +0x10 the suck record (577 +0x60, amoeboids +0xc0, chicken global) | wrappers | `react::record` |
| slot +0x14 delete (default `DeleteMoby` wrapper) | 0x20c3ac | `react::slot_delete` |
| the carried update `0x305260`, landing `0x3051a8`, burst `0x304798` | class states 7 / 0xe / 5 | `react::{carried, land, burst}` |
| the other levels' tables: ~~749~~ (ported 2026-09-29: `react::VELDIN_749`, found by `tables_from_overlays`; creatures.md §9), 580, 340, 827, 252, 193, 1246, 238, 63, 1445, 1382, 568, 1906 | | not ported (G-ENM-001) |
| the lure: the Taunter writes record +0x18 (`0x2cc830`); 577 (240-tick alert), 572 family (alert), 459 (600 ticks), the mine 74 (reach ×3) read and clear it | | ported (`taunter`, the classes, `mine`) |
| the morph meter: record health (the meter), s16 +0x04 (the scale), +0x0e keep byte; excluded 0x58e / 0x452 | `0x2d2450`, `0x2defb0` | ported (`morph_ray`, `react::morph_target`) |

### 14.4 The shared creature helpers the reactions use (and the misses fixed here)

| helper | game | status |
|---|---|---|
| hit resolver (damage, cooldown, reaction byte, burn marker) | `0x26f378` | ported (`creature::damage`) |
| knockback flight | `0x271558` | ported (`creature::knock`) |
| **the burn's fire puffs** (kind 4 marks the flight: the Pyrocitor's / Tesla Claw's hits) | `0x271258` → `0x26fba0` + `PartType04Spawn` | **fixed**: `knock::burn_sparks` (was counted as unreachable: both weapons are ported now) |
| **the knockback's water-entry splash** 775 | `0x2ff768(3, p)` | **fixed**: `splash::spawn` at the water height (the splash class was already ported) |
| **the beam explosion's debris** (`param_16`: 3 draws, one fireball 122 toward the camera within 14, `debris − 1` thrown) | `0x273310` → `0x2c4c20` | **fixed**: `bomb::fireball` (the draws were skipped: every R.Y.N.O. / mine / decoy / drone / trooper / gunship / gold-chicken blast was off the game's RNG stream) |
| **the flyer 660's kill blast** | `SpawnBeamExplosion` at 0x2f45d4 | **fixed**: `flyer::KILL_BEAM` (shared with the gunship 688) |
| hit flash | `0x2723f8` | ported (`creature::flash`) |
| death explosion / beam explosion / colour shift / explosion light | `0x273310`, `0x270fa8`, 639 | ported (`creature::fx`) |
| area hit with push | `0x26f8f8` | ported (`creature::attack::area_hit`) |
| target acquire with decoys | `0x274b78` | ported (`creature::target`) |
| move-collide | `0x26d270`, `0x26d610` | ported (`creature::walker`) |
| the missiles' record +0x1e \|= 0x80 | Devastator / R.Y.N.O. | written; no ported class reads it (the readers are unported classes: G-ENM-001) |

**Tests**: `hero_weapons4.rs` `novalis_flyer_kill_blast` (a 0x800000 hit: the flyer gone, 20+ type-15 streaks),
`novalis_pyrocitor_burn_puffs` (type-4 puffs off a burning amoeboid), `novalis_ryno_state_3` (the missiles' blasts
throw debris fireballs 122); `creature::knock::tests::knocked_into_water_splashes_once`. The guns, gloves, Pyrocitor,
creature, reactive-lib, morph and targeting tests stay green (the RNG stream after a blast is now the game's).

## 15. The Walloper and the gadget lunge 0x20 (2026-09-28)

**Ids**: item **18** / class **180** (the item table 0x179f40; `lvl.vtbl` 0x2d11c0 on 01, the same code on every level);
def +0x18 = 0 (no holding layer), no ammo. Module `hero/walloper.rs` (its doc has the whole state machine), engine
`walloper_render.rs`. **H** (decompiler output checked against the disassembly of 0x239d84..0x23a1b4, 0x2417a0..0x2417c8,
0x23fa20..0x23fa80, 0x2d11c0..0x2d1734).

| address | what it does | port |
|---|---|---|
| `0x240ed8` case 0x12 | group 0 / 1 (4 once landed), not state 1; ○ within `min(ticks(10), 0x140400)` | `walloper::fire` |
| same, after SetState | `PlayClassSound(0, 0, item)` (the swing) | `fire` → `fx.item_sounds` |
| `0x23cf98` group-6 part | group 6, 0x13fdbc = 1, 0x1415d4 = 0, aim / target / hit cleared | `melee_entry` (shared) |
| `0x23cf98` case 0x20 | `SetAnim(−1, 0x67, 4)`, `MobyAnimBlend(item, 3, 4, −1)` (with the item, when playing) | `walloper::entry` |
| same | `0x2351d0(11, 50°, −1)`: stick aim + the target search `0x22e238` / `0x22dff0` (score `d + yaw·d`, +7 crates) | `Hero::aim_assist`, `melee::aim_search` (new; the wrench states still aim without it: G-WPN-008) |
| `0x2370b8` case 0x20 | aimed: TurnTo(0.05, 0.2, 15.0098/s); else re-aim for T < 4 | `walloper::physics` |
| same | target speed `18·dt` for 10 < T < 18; SpeedStep(190·dt², 90·dt²) after T = 10; SetPlanarVel(aim or facing) | same |
| same | after-images: `0x277400` + ghosts 0x30 @ 2, 0x17 @ 4, 0x0c @ 6 at T = 8; `0x277508` every tick, fade 5 after T = 18 | `packs::thruster_trail_start`, `packs::hero_trail_update` (shared with the Thruster jumps) → `afterimage`, drawn by `afterimage_render` |
| same | 10 < T < 22 with the item: `FUN_0026e808(3, tmpl, Ratchet, 0x30000, (2cos, 2sin, 1.3))`, +0x0c = 5627.92, +0x18 / +0x19 = 0 / 3, +0x1a = the item's class; three `coll_sphere_mobys(0.8, pos + 0.8·(cos, sin) + 0.55 up, 0, ignore)` at yaw / yaw − 50° / yaw + 50°, ignoring the item / **Ratchet** / the item | queued, `walloper::deliver_hits` (the items' update, before the hand item's) |
| `0x236cb8` | first listing of the lunge: item sound 1 (0x13fdb4); per listed moby of class type 5 or 9: `0x248d80(0.5)` (0.5 up) + `0x2bdb18(p, 4)` | `deliver_hits`: `fx.item_sounds`, `fx::sparkle_burst` (= 0x2bdb18, the L00 copy 0x2a7e20) |
| same | HeroEdgeBrake(3.7) after T = 10, HeroWallCheck(1), gravity 24·dt² from 0x13f460 in the air + HeroSteepWallStop, else 54·dt² | `physics` |
| `0x242930` case 0x20 | with the item, not blending, frame > 20 → `SetState(0, 0)` (no item: never ends) | `walloper::transitions` |
| `0x2d11c0` +0x20 = 0 | → 2, the ten arc timers 0 (and the unread s16 arrays 0x1dc370 = 2 / 0x1dc388 = 1: n/a), fade timers `ticks(5)` / `ticks(20)` | `walloper::update` |
| same, state 2, lunging | item key time ≥ 6: arcs active; glow 0x161704: `1 − A/5` while A runs, then 1 until key 12, then `B/20` while B runs, then 0 | same |
| same, state 2, not lunging | timers reset; `MobyAnimBlend(item, 1, 0, ticks(12))` every tick | same (`items::blend_item`) |
| same | 0x17b33c = 0.1 unless Ratchet's sequence B is 0x82 | n/a: no reader in the level code (the Tesla Claw writes it too) |
| same, the arcs | timer −1 (≥ 0); at 0 while active: timer 2, alpha 0x40, `0x26cae0` (2 angles, 0.2), four segments turned ±10°..45° about the view ray; else the jitter `randf_sym(0, 0.1)` per axis, alpha 0x20 | same |
| same | `RegisterDrawCallback2(0x2d1768, item)` | `Walloper::drawn` + `draw_callbacks::Callback::Walloper` (on Ratchet's moby) |
| `0x2d1768` → `0x2d1830` | per live arc: core strip FX 14 (0xffffff, 0.05) and glow FX 16 (0x7f2020, 0.5, +0.2 each end), alpha × glow | `walloper::draw_quads` → `tesla::strip_colored` (the Tesla's `0x2d0748`, the same algorithm) |
| `0x2d1e08` | fist glow FX 8, 0.75, 0.2 toward the eye from `pos − 0.5·row 1`, `randi(4)` ? 0x307f4040 : 0x7f7f4040, alpha × glow | `draw_quads`; the `randi(4)` in `draw_callbacks::run_frame` (the game's draw time) |
| `0x2d1830` tail | `VU1_addGSregister(0x47, 0x5360b)` (TEST back) | n/a (GS state: the port's material) |

**A quirk kept** [M]: the spheres at yaw and yaw + 50° ignore the hand item, not Ratchet, so once his body reaches them
his own moby is listed: the hit sound plays in the open too, and his hit slot takes the lunge's 0x30000 record (his
intake reads mask 1 and drops it; a weaker enemy hit arriving then is refused by the delivery's damage rule). To be
checked on PCSX2 (G-TOOL-007).

**Native**: `f32` for the new code (arcs, draw, spheres, search); the state's physics calls the existing PS2-float hero
helpers as the other states. **[L]**: the fist glow's quad corners `fun_001f9d20` read as a rotation plus the centre;
the gravity step of the sparkle point in gravity mode 0 only (Magneboots: along the normal in the game).

**Tests** (`tests/weapons/hero_weapons4.rs`, twice identical; `walloper::tests`): Novalis — the lunge into a big amoeboid 572
(swing sound, after-images made at T = 8 and faded, 3 spheres a tick in the window, one hit sound, sparkles, the
amoeboid hit, the lunge back to 0, the arcs and draw ending), a lunge in the open (only Ratchet listed, no sparkles, it
carries him forward, two lunges), a crate broken; Rilgar — a small amoeboid 866 hit. Unit: the aim search's scores and
cones, the arcs' 0.2 segments and the draw's quads / alphas, `FastDecTimer`. `novalis_hero_digest` (NO_IDLE)
unchanged: md5 596306d7….

**Frames** (scratch `wvr/shots/`, two runs byte-identical): Novalis `RC_GIVE_ITEMS=18 RC_HERO_AT=54.0176,141.6441,40.08,
3.14159 RC_PLAY_SCRIPT='40-41:press CIRCLE' RC_DUMP_FRAMES=44..72` (`grid_w1.png`: frames 50 / 54 / 58 / 62, the fist glow,
the after-images, the amoeboid split); Rilgar `RC_LEVEL=5 RC_HERO_AT=161.4752,329.0638,26.5,1.5708`, same script
(`grid_w2.png`).

## 16. R.Y.N.O. update state 3 (2026-09-28)

| address | what it does | port |
|---|---|---|
| `0x2e4e60` prologue (states 2 / 3) | lock timer, search, marker | ported before (`ryno::update`) |
| `0x2e5264` | `FUN_0020fb60(1.0, 0, 90, item, 0xff0f0fff, none, 0x23, −1, 4)`: the red crosshair at the screen centre | `ryno::update` state 3 (`Markers`, `at: None`) |
| same | 0x13cae0 & 5 (L1 / L2) held: ○ pressed, not 0x1413fc, `0x249450` one ammo → the salvo (stats 0x141738.., swap lock 2, state 4, counters, `0x22ee08`, past targets cleared) | `ryno::start_salvo` (shared with state 2: the game has the code twice) |
| same | released → `LAB_002e5704`: state 2 | same |
| writers of state 3 | none in the level code (0x2e5104 / 0x2e5124 / 0x2e5220 / 0x2e5390 / 0x2e556c / 0x2e5704 store 1, 2, 4, 4, 5, 2) | unreachable in play; tested by setting it |

Not kept: the shot statistics (G-SAV-009). Tests: `hero_weapons4.rs` `novalis_ryno_state_3` (crosshair, L1 + ○ → salvo,
one ammo, missiles, their debris fireballs), `novalis_ryno_state_3_released`.

## 17. The Visibomb: the gun, the missile, the type-6 camera, state 0x1d (2026-09-28)

Ported. Level01 addresses; **H** from the decompiler output, the missile's update `0x2cbda8` (no Ghidra function, 674
instructions) and `0x302438` / `0x2cb808` / `0x2cb968` from the disassembly. Code: `rc-game/src/hero/visibomb.rs` (the
gun), `moby_update/classes/visibomb.rs` (the missile, its launch, end, look, glow and explosion),
`follow_camera/type6.rs` (the camera mode), `hero/scripted.rs` (0x1d), `classes/rc_range.rs` (the static, the cut-off),
`rc-engine/src/visibomb_view.rs` + `assets/shaders/missile_view.wgsl` (the look in the renderer). Tests:
`rc-game/tests/weapons/hero_visibomb.rs` (Novalis, Rilgar), the unit tests of those modules.

* **Ids** (the item table 0x179f40, checked by `visibomb_is_item_13_with_its_classes`): item **13**, gun class **163**
  (update `0x2c8cc0`, def +0x18 = 2: both glove-holding layers; no arm sequences, def +0x30 = 0), the missile **172**
  (`0x2cbda8`), its glow **179** (the Pyrocitor's pilot flame, `0x2d1068`, created by `0x2d0fc8`), the range limiter
  **832** (`0x302648`).
* **The camera is not shared.** Type 6 of the camera table 0x20c480 (entry 4; activation `0x317f50` returns 0,
  release `0x318028` is empty): a scan of the 19 overlays for calls to the switch `0x317d88`, the tracking `0x317aa0`,
  the placement `0x317778`, the hand-back `0x317e70` and `0x20cdf8(6)` finds only the Visibomb (the gun, the launch,
  the missile's update, the end) and the functions' own calls. It is ported as a camera mode of `follow_camera`
  (`type6::Type6`, entered through `cinematic::CinematicCall::Type6`), generic in its input (a centre, an Euler, a
  timer, the orbit's steps), with the script camera's springs (`script::spring` / `angle_spring` / `sph_point` =
  `Cam_InterpValues`, `0x20cf28`, `0x20f180`) and the blend of `CamBlend`. Its one consumer is the Visibomb.
* **Frame order.** The gun's update runs in the hero's slot loop: the launch (a `CreateMoby` in the moby world through
  the hit sink's `world`) switches the camera in the same tick (the tick applies the hand items' camera calls right
  after the item updates, before `CameraUpdate`), and `SetState(0x1d, 1)` is made right after the slot loop
  (`weapons::after_items`). The missile's update runs in the moby loop from the next tick; its camera calls are applied
  right after the moby loop; its `SetState(0, 1)` goes through the hero calls (`cinematic::hero_state`).

**Coverage** (`address | what it does | port`).

| address | what it does | port |
|---|---|---|
| **`0x2c8cc0`** gun update | `FastDecTimer(+0)` | `hero::visibomb::update` |
| | fire only with ○ pressed (0x13cae4 & slot mask), timer 0, no missile (+4), not 0x1413fc | `update` (the missile slot: the moby gone from the table = the end's clear of +4) |
| | groups 7 / 2 / 3 / 4 / 5 / 6 / 0x10 / 0xe / 9 / 0xa / 0xb / ≥ 0xd refused; states 0x72 / 0x65 refused | `group_fires`, `update` |
| | active camera of type 5 (script) refused | n/a [L]: every script camera on the ported levels holds Ratchet in 0x72 / 100 (refused by the states) |
| | not grounded (0x13f650 = 0) refused; ground moby 0x13f64c of class 0x13e / 0x46f refused | `update`; the ground moby: n/a (0x13f64c never set in the port) |
| | `0x249450(−1, 1)` one ammo; none → `PlayClassSound(0, 0, gun)` (the click) | `update` (`use_ammo`, `EMPTY_SOUND`) |
| | first person (state 1, Ratchet hidden by the first-person camera): yaw / pitch of the view, point eye + 0.3·fwd − 0.35·up | `aim` (test `novalis_holding_layers_and_first_person_launch`) |
| | 0x141618 = 1: pitch from the gun's rows | n/a [L]: never set in the port (as the other guns) |
| | else yaw 0x13f9d8 (= 0x13f3e8 outside group 0xf), pitch −0, point `polar(0.6, yaw, −0)` + gun | `aim` |
| | the gun's sequence 3 (`MobyAnimBlend(gun, 3, 0, 0)` when not on it) | `fire` |
| | `0x225a28`: the help box suspended | `fire` (`Help::suspend`) |
| | `0x2cb540(yaw, pitch, gun, point)` → +4 | `fire` → `classes::visibomb::launch` |
| | the shot statistics 0x1416e8 / 0x1416ea / 0x1416ec | NOT ported (G-SAV-009) |
| | fire timer `ticks(60)` | `fire` |
| | tail: sequence 1 when the gun's sequence wrapped (+0x70 & 2); +0x34 \|= 4 | `update`; +0x34: n/a (the hand item's matrix is the attach's) |
| **`0x2cb540`** launch | `CreateMoby(0xac)`; 0x17e988 = 1; 0x141330 = missile | `launch` (`Globals::hud_off_at`, `missile`) |
| | +0x30 / +0x32 = 0xff, state 1, visible, rotation (0, pitch, yaw), at the point, +8 / +0xc = 0 | `launch` |
| | +4 = a moving ground moby's speed (0x275290, `|0x13f450|`) else 0 | `launch`: 0 (0x13f64c never set) |
| | +0x10 `ticks(3000)`, +0x14/+0x18/+0x1c = 0, +0x44 = 0.4 | `launch` |
| | `0x2cb338` the look | `view_on` |
| | `MobyBuildMatrix` | `launch` (`World::build_matrix`) |
| | `0x317d88` → +0x14 | `launch` (`CinematicCall::Type6(Switch)`) |
| | `SetState(0x1d, 1)` | `hero::visibomb::fire` → `weapons::after_items` |
| | `AttachManipulator(m, 0 / 1, 0x1412b0 / 0x1412f0)` (the fins) | NOT ported (G-WPN-004: a created class's joint-list targets) |
| | `0x2d0fc8(m)` the glow 179 → +0x20 | `launch` + `pyro_glow::init` (shared with the Pyrocitor) |
| | +0x48 = 24·dt; `0x272078` Ratchet's light word / ambient | `launch` |
| | `PlayClassSound(0, 4, m)` → +0x2c | `launch` (test `novalis_launch`) |
| | `CollLine_Fix((gun.xy, point.z), point, 0, Ratchet)`: a hit → +0xbc = 1, at the hit | `launch` |
| | 0x15f608 = 1; `force_help_message(6, 0)` | `launch` (`all_visible_at`, `Interact::force_prompt`) |
| **`0x2cbda8`** update | 0x15f608 = 1; 0x15f458 = 1; `0x225a28`; 0x17e988 = 1; `force_help_message(6, 0)` | `frame_flags`; 0x15f458: n/a (no reader in the overlay) |
| | +0x10 − 1; ≤ −`ticks(100)` → `0x2cb788` | `update` |
| | Ratchet in 0x16 / 0x3d / 0x72 / 0x65, groups 2 / 3 / 4 / 5 / 7 / 0xa / 0x10 / ≥ 0xd, not grounded → `0x2cb788` | `update` |
| | ground moby of class 0x13e / 0x46f → `0x2cb788` | n/a (0x13f64c never set) |
| | timer < 0: `0x317aa0`, `0x2cb458(m, 0)`, `0x2cb808` | `update` (`track`, `view_off`, `glow_frame`) |
| | target speed → 24·dt (34·dt with ✕ 0x13cae0 & 0x40) by 10 %; speed by 1 %, ≤ 34·dt | `update` (test `novalis_range_limit_ends_the_flight`: ✕) |
| | `SoundSetPitchBend(+0x2c, trunc((target − 24dt/(34dt − 24dt))·100))` | `update` → `World::set_pitch_bend` → `SoundSink::set_pitch_bend` (test: −199 every tick) |
| | timer > 0 and past 5 ticks: the stick (0x141070 / 74) eased 10 %, roll 6 %, pitch −= y·0.7° (±89.94°), yaw −= x·0.7° | `update` (test `novalis_steering_and_circle`) |
| | `0x26eec8(m, 0.2)` blob shadow | NOT ported (G-REN-025; counted) |
| | step `0x277b50(speed, yaw, −pitch)`; the first 60 ticks: `GroundHeight(0.5)`, below ground + 0.4 over the world or a creature class (+0x46 = 5): `Approach(z, 6·dt)`, pitch −= `min(atan(speed, gap), 90°·dt)` | `update` |
| | out of 4.5..1018.5 → `0x2cb788` | `update` |
| | `0x2cb808` | `glow_frame` |
| | the fins `0x221e38(+0x24/+0x28, 1, −(y/2 + |x/3| ± 1.1·(x − roll))·0.7)` | NOT ported (G-WPN-004, with the manipulators) |
| | `CollLine_Fix(old, new, 0, Ratchet (first 40 raw ticks) / itself, tmpl: push, 1, 5627.92; 0x830000; 3 / 2; class; 6)` | `update` (`line_hit_in`: the hit record to crates and creatures) |
| | a moby → +0xbc = 2 (Ratchet: no blast); a face (+0x1c > 0) → 1; at the hit; the loop voice released (owner / state tested) | `update`, `release_voice` (test: `Release` on the crate) |
| | the face edge, the reflected step (2·dt), the normal | n/a: `SpawnBeamExplosion`'s param_9 and a stack vector, never read |
| | nothing and timer 0 → `0x2cb788` (a 3000-tick flight ends quietly) | `update` |
| | after 60 ticks ○ (0x13cae4 & 0x20) → +0xbc = 1, voice released | `update` (test: ○ at 50 ignored, at 80 explodes) |
| | +0xbc: `SpawnBeamExplosion(0, 0, 4, 2, 0, 1, 20, m, _, pos, 10, 3, 16, 1, 1, 1, −1, 0)` unless on Ratchet | `explode` → `fx::beam_explosion` (streaks 15, sparks 11, puffs 8, fireball 122, flashes, class sound 1, shake, light 20) |
| | `coll_sphere_mobys(pos, 0x10, m, 0, 4)` + `0x26f8f8(m, pos, list, n, 6, 1, 1, 0, 0x830000, 3, 2)` | `explode` → `attack::area_hit` (tests: the crate breaks, a critter reacts) |
| | +0x34 \|= 1; +4 = 0.02; +0x18 ·= 0.01; timer −`ticks(100)` within 30 ticks else −1 | `explode` |
| | five times: pos = blast + `0x2747a0(1.5)`, `0x2cb968(m, 1)`; pos = blast; +0xbc = 0 | `explode` (`fx::jitter`, `explosion`) |
| | `0x317aa0` | `track` |
| **`0x2cb788`** end | gun +4 = 0; 0x141330 = 0 | `end_flight` (the hero side sees the missile gone) |
| | `0x317e70` | `end_flight` (`Type6(Release)`) |
| | `SetState(0, 1)` | `end_flight` (`cinematic::hero_state`) |
| | `0x2cb458(m, 1)`; `0x225a88`; `DeleteMoby` (the loop voice stops with its owner); 0x167494 = 0 | `end_flight` (`view_off(full, underwater_off)`, `Help::resume`); the glow's owner state written 0xfd (it deletes itself next tick, as reading the deleted owner) |
| **`0x2cb338`** look | save 0x15f444..0x15f454 into +0x30..+0x42; fog 0x40 / 0x60 / 0x40, near 0, far 0x48000000, F 255 → 0 | `view_on` → `FogWrite::Swap` → `visibomb_view::apply_fog` (on `fog_state`'s level globals: the zones still overwrite them, as in the game) |
| | 0x15f30c = 1 (the scanline overlay, the range static) | `View::overlay` → `MissileViewOverlay` (`missile_view.wgsl`) |
| | 0x16a478 = 0 (no sky, the frame cleared); `SetBackgroundColor(0x40, 0x60, 0x40)` | `View::no_sky` / `background` → `visibomb_view::sky` |
| | 0x160f80 / 0x160fe0 / 0x1604a4 / 0x15fff0 = 144 units | NOT ported (G-REN-029) |
| | 0x16017c = 80 units (particle far) | n/a: `UpdateFog` writes 500 at the end of the same frame |
| **`0x2cb458`** restore | 0x15f30c = 0; the globals back; 0x16a478 = 1; `full`: the far distances 500 / 720; `SetBackgroundColor(level)` | `view_off` → `FogWrite::Restore` (test: the orbit's writes) |
| **`0x2cb808`** glow | +0xc eased by 0.05 to 0.45; yaw −π/2; rows = Euler × missile rows; at 0.11 along the combined y | `glow_frame` (test: 0.05 steps, 0.11 away) |
| | `MobyAnimSphereLerp` | n/a (the renderer's bounding sphere) |
| **`0x2cb968`** explosion | 6 type-13 puffs (0.3, 1.01, 1.075 / 1.07, 0.05, 50000, z + 0.3, 1, 0x40808080) | `explosion` (test: ≥ 30 type 13) |
| | camera distance d: `trunc(d) + 2` (d < 8) else 10 type-11 rings, 8..10·dt − (7 − d)·dt, colours 0x20aab8 / 0x20aad0, `ticks(15..20)`, `ticks(25..30)` | `explosion` |
| | frame load < 0.95: flashes 3 / 15 / 7f7f7f / 20 and 2 / 24 / 7f2000 / 20 | `explosion` (the load is 0 in the port: always) |
| | flashes 3 / 20 / 7f4000 / 30, 2 / 27 / 601000 / 40, 1 / 29 / 200000 / 20 | `explosion` (test: ≥ 17 flash mobys) |
| **camera** `0x317d88` | `0x20cdf8(6)`; 0x167370 = 0; `0x20d110` (a cut [M]); `0x317aa0`; centre = target; springs = 0x208820; `0x317808`; D+0x9c = 1; k 0x208838, d 0x208850, max 0 | `Camera::type6_switch` (test: the camera at the missile on the launch tick) |
| | the type's init `0x317f58` (from Ratchet) | n/a: overwritten by the switch |
| `0x317808` / `0x3176d0` | velocities, roll, centre spring 0.3 / 0.3, the placement | `Type6::init` / `place` |
| `0x317aa0` | flight: roll /2, pitch offset 5 %, centre, yaw base; timer −1: `0x317778` (6 away, elevation = pitch offset); after: roll += +0x18 (×0.97), distance += push (the 0.77 / 0.7 rule on Ratchet or the hand item, xy `0x221398`) | `Type6::track` + the missile's `track` (tests: `orbit_after_the_flight`, the crate run's orbit) |
| `0x3178d8` | the five springs, the centre, position, Euler, rows | `Type6::step` |
| `0x317e70` | cut (+0x7e = 4) when Ratchet ≥ 32° off the view or ≥ 8 away (3-D `0x221360`), else a blend 0.018 | `release_blends`, `type6_frame` (unit test both; the crate run cuts) |
| `Camera_handleCollWithHero` | the active camera's collision with Ratchet | n/a: not modelled for the script camera either |
| **state 0x1d** | `SetState`: group 9, 0x1413fc = 1, 0x1415d4 = 0, idle `0x226f10(0)` over 10 ticks (no put-away); physics: the idle case; transitions: none | `scripted::entry` / `physics` / `transitions` (test `visibomb_flight_holds_ratchet_with_the_gun_out`) |
| **832** `0x302648` | the counter; the missile in 0x1d; the records (path + floor / ceiling, cuboid) | `rc_range::update` (unchanged) |
| | counter ≥ 1 and 0x15f30c: `RegisterDrawCallback2(0x302438, m)` | `rc_range::update` (`Callback::RangeStatic`) |
| | counter > `ticks(90)`: `0x2cb788` | `rc_range::update` → `visibomb::end_flight` (test: the end exactly 90 ticks into the last stretch outside) |
| `0x302438` | 17 × 14 quads of FX 0x1e from (−16, −16), `(randi(32), randi(32))` each, colour 0x7f7fff, alpha `trunc(n·127.5/ticks(90))` | `rc_range::draw_callback` (at the next tick's `run_frame`) → `visibomb_view::statics` (unit test `static_grid`) |
| **HUD / render** | `HudDraw`: nothing while 0x17e988 (cleared by it) | `visibomb_view::hud_off` → `hud_render` |
| | `DrawWorld` end: `0x21b9f8` with 0x16cc00 / 0x16cc30 (ALPHA 0x42: subtract 0x50 from red and blue; bands of 31 rows every 34 minus 46 / 30) | `MissileViewOverlay` |
| | `UpdateOcclusion` with 0x15f608 = 1: everything visible | `visibomb_view::all_visible` → `occlusion` |
| | `UnderwaterTest` 0x20e9f0: the flag 0 while the active camera is of type 6 (`+0x86 == 6`) | `fog_state` (`Camera::type6_active`) |

**Native, not emulated [L].** The hand item is not a table moby: the gun's pvars live in `Weapons::visibomb`, its +4
is "the missile still in the table". The fog globals are the engine's (`fog_state`), so the save of `0x2cb338` is kept
there (`FogSwap`) instead of in the missile's pvars; the missile's writes reach it as an ordered log. The camera
calls are queued (`CinematicCall::Type6`) and applied at the points of the frame where the game's direct calls
land. The orbit's decaying values (+4, +0x18) stay in the missile's pvars and the camera gets the steps. The
range static's quads are computed at the next tick's `run_frame` (the port's draw-callback rule) and shown one frame
later. The glow scale's 0.05 steps oscillate across 0.45 (the f32 sum misses it), as the game's.

**Misses found by the coverage walk** (beyond the previous agent's list): the flight also ends when Ratchet is not on
the ground or in groups 0xa / 0x10; the path test ignores Ratchet only for the first 40 *raw* ticks (then the missile
can hit him: no blast, his hit record); ○ is ignored for the first 60 ticks; the terrain following refuses to rise
over non-creature mobys; the post-blast orbit re-places the camera 6 away with the elevation of the flight's pitch
offset and is pushed out by the missile's residual speed; the orbit restores the fog every tick while the far
distances stay at 144 until the end; an explosion within 30 ticks of the launch skips the orbit (−`ticks(100)`);
the scanline overlay (0x15f30c's second use in `DrawWorld`), its lighter record on levels 2, 5–8 and 10; the static's
colour 0x7f7fff (a stack argument the decompiler drops); 0x15f458 has no reader; the pitch bend's expression is the
game's (−199 at cruise, not a 0..100 ramp).

**The overlay pass.** Bevy 0.19's `FullscreenMaterialPlugin` keeps one pipeline component per view for every
material type, so with several types registered only one draws (the underwater tint never drew in gameplay). The
missile view overlay and the underwater tint now use the port's copy with a per-type pipeline
(`rc-engine/src/gs_post.rs`); the scene fade stays on Bevy's (its only user).

**Frames** (`RC_GIVE_ITEMS=13`, `RC_PLAY_SCRIPT`, `RC_DUMP_FRAMES`, `RC_SETTINGS_FILE`; every frame of two runs
identical). Novalis from the spawn, ○ at tick 60 then the stick pulled back with ✕ (`60:press circle,66-400:stick 0
1,66-400:press cross`): frame 100 the missile view (green, the scanline bands, no HUD), frame 240 the static near the
range edge over the climbing missile, frame 275 back to Ratchet (HUD back, the flight cut off after 90 ticks out).
Novalis at crate 376 (`RC_HERO_AT=147.42233,133.81314,57.0,-1.5707964`, `60:press circle,66-80:stick 0 -1`): frame
128 the explosion in the missile view (inside fog zone 0, which overwrites the green fog as in the game: darker),
frame 140 the orbit round the blast with the look restored, frame 236 back to Ratchet (a cut: he is behind the
orbiting camera). Rilgar (`RC_LEVEL=5`, `60:press circle`): frame 90 the missile view with the lighter bands of
record 0x16cc30.

## 18. The Glove of Doom: the canister 230 and its bots 186 (2026-09-28)

**Ids** (the item table 0x179f40 against `lvl.vtbl`, the same code on all 19 levels): item **20** / hand class **229**
(0xe5) → `0x2dd6c0` (the shared glove update of §13: `hero::gloves`, its fourth `GloveRow`); the canister **230** (0xe6)
`0x2de650` (`classes::doom_canister`); the bots **186** (0xba) `0x2d5c10` (`classes::doom_bot`). **H** (decompiler
output checked against the disassembly of 0x2d49f8 and 0x2d5c10..0x2d7378, where the decompiler lost the search's
arguments, the bot's saved state byte and the turns' velocity pointers). The helpers the gap row listed as
`0x2d42b8` / `0x2d44f8` are the Morph-o-Ray's draw (called from `0x2d38b8`), not the bots'.

**What the game does.**
* **The glove** (`0x2dd6c0` = the Bomb Glove's `0x2d8330`): no warm-up; the hand point (0.01, −0.14, −0.05) in the
  glove's frame; the shared trigger (voice 0x1a, one ammo, state 3); the aim `0x2dd1b0` = the Decoy Glove's
  `0x2ed0c0` (2.5 ahead of the launch point 0.39082 up, 3.5 / the camera's line with L1 / L2) solved by `0x2dcf88` =
  `weapons::launch_velocity` (gravity 9) into the canister's velocity; an empty throw clicks (class sound 0); a new
  canister whenever there is ammo; state 6 leaves the canister alone. No holding layer (def +0x18 = 0), no loop
  sound, no hand-item sound but the click: the glove's side is §13's, unchanged.
* **The canister** (`0x2dde80` create, `0x2ddf58` release, `0x2de650` update): held it grows to a quarter of its
  class scale and glows (particle type 32, `0x283d88`, every 4 ticks, not in first person); thrown it grows to half,
  glows, falls (9·dt²), bounces (a 0.3 sphere and its bottom line; the velocity reflected ×0.5) and, once slower than
  0.01 a tick, opens (at once on the world or a moby's mesh, after 240 slow ticks on a moby's primitive); on water /
  lava it pops (20 type-5 puffs) instead. Open, it lets out a bot every 8 ticks — four, at the quarter turns — while
  fewer than 8 bots are alive (else it waits 15 ticks), then fades (alpha −4, scale ×0.92 a tick).
* **The bots** (module doc of `classes::doom_bot` for the states): they drop out, then walk through the shared walker
  (`walker::walk_to` = `0x26de80`, now shared with the mouse 1818), heading where the hop solver `0x2d4cb8` points;
  one tick in four they search the target list 0x1abe80 (`0x2d49f8`: **creatures only**, class type 5, within 40;
  score = distance + squared yaw / pitch offsets, +15 when 3.25 above, +5 when blocked; blocked = a line test within
  30 (heights within 8), the "drawn" byte beyond) and, with none, follow Ratchet when more than 3.2 away; chasing they
  jump over what blocks them (`0x2d5658` / `0x2d5518` / `0x270340`). They **explode** on touching a creature (a
  targetable type-5 moby's primitive in their sphere), within the target's reach (1 + record +0x0a / 8) when their
  sphere (mobys only) touches something, landing on a creature or water / lava (`0x2d5b30`), when hit (mask
  0x800001), after 3600 ticks or past 4000 patience (+500 per unreachable target): `SpawnBeamExplosion` (flashes 2 / 1
  beyond 9, light 15, 5 streaks, 2 spark pairs, 4 puffs, the shake, 5 debris fireballs), every moby of their sphere
  (radius 1, 0.5 up, flags 0x15) hit (damage 3, pushed 1 / 1 up, flags 0x10000, type 2 / 3: creatures, crates, props,
  decoys, mines, other bots' hit slots) and their class sound 0.

**Which mobys react.** The bots **target** class type 5 only (577, 572 / 865 / 866, 459 on the ported levels; crates,
chickens and props are never targets). Their **blast** (0x10000) reaches every reader whose mask has 0x10000 (§14.2):
577, the amoeboids, 459, the crates, 704 / 754 / 778 / 1042 / 1813, the chickens, the decoys (0x30001), the mines
(0x830000); not the flyers / gunship (0x800000), the drones (1) or the bots (0x800001: a blast's 0x10000 record, being
stronger, even replaces an explosion record a bot holds — the game's delivery rule, seen in the crate test). The bots
are set off by explosions (0x800000) and enemy hits (1).

**Coverage** (status **P** ported (file::fn), **G** gap id, **n/a** reason). Level01 addresses.

*The glove* `0x2dd6c0` (the rows of §13's table hold; the Doom-specific ones):

| address / branch | what it does | status |
|---|---|---|
| hand offset (0x3c23d70a, 0xbe0f5c29, 0xbd4ccccd), no warm-up | | P `gloves::GLOVES[3]` |
| `FUN_002dd1b0` → `0x2dcf88(9·dt², 1e−5, 2.5/3.5, launch, target, pvars)` | the aim (0x13fda0 cleared without L1 / L2) | P `gloves::doom_aim` (+ `short_throw_aim`, `weapons::launch_velocity`) |
| case 3 without a canister: `fun_0022da68(0, 0, glove)` | the click | P (`Empty::Click`); test `novalis_glove_of_doom_clicks_without_a_canister` |
| case 3: `s16 0x141720++`, 0x141722 minutes, 0x141724 level bits | throw statistics | **G-SAV-009** (counted in `Weapons::throws`) |
| case 3: `FUN_002ddf58(glove+0x40, canister, pvars)` | the release | P `gloves::release_canister` |
| tail: `FUN_00249530(−1)` → `FUN_002dde80(glove, hand, hand)` | a new canister with ammo | P (`Refill::Ammo`); test (the next canister at the throw tick) |
| case 6 → return | the canister deletes itself | P (`OnDelete::Keep`) |
| HAND_ITEMS row (weapon check `weapons::fire`, 0x23 / the arm) | | P `gadgets::HAND_ITEMS` |

*The canister* `0x2de650` (+ `0x2dde80`, `0x2ddf58`, `0x2de0a8`, `0x2ddce0`, `0x283d88`):

| address / branch | what it does | status |
|---|---|---|
| `0x2dde80`: `CreateMoby(0xe6)`, +0x32 / +0x30 0xff, +0x31 1, state 0, the hand point, pvars[0] = the point, owner, +0x38 / +0x34 = 0, scale = class · 0.01, 0x1413f5 / 0x1413ff → mode \| 0x41 | | P `doom_canister::init_held` |
| `0x2ddf58`: velocity kept, (0x13f64c platform add), at the hand point, `CollLine_Fix((Ratchet.xy, point.z), point, 0, Ratchet)` → on the wall `(9 − 0.1)·dt²` off, velocity reflected ×0.6; state 2 | | P `doom_canister::release` + `gloves::wall_probe` (platform: n/a, 0x13f64c never set) |
| out of [2, 1021]³ → `DeleteMoby` | | P |
| 0x1413f5 and state ≤ 1 → mode \| 0x41, else clear | hidden in first person | P |
| owner class 0xe5, +0x3a == 0, \|vel\| > 0 → `0x2de0a8(…, 1)` (0 / 2 on platforms) | the landing preview | P `decoy::short_glove_preview` (shared with the decoy); never draws (the first update marks +0x3a: §13's quirk) |
| `0x2de0a8`: 0x72 / 0x15f5c4 → 0; held from the launch point for 300 ticks (grind displacement), flying from it for +0x38 ticks and +0x3a = 1 when the glove holds one; the arc (gravity 9, flags 0x10, Ratchet / the owner skipped in the first 10 ticks); `FUN_0021afe0(0x2dda60)` (= the bomb's reticle `0x2c23c0`); the point 0.95 from the camera | | P (the same function as `0x2d9760`: `tg::arc_landing`, `tg::BOMB_RETICLE`) |
| 0 → 1: +0x34 ticks(4), +0x3c / +0x38 = 0 | | P |
| 1: +0x38 = 0; scale += class·0.05 below class·0.25; `FastDecTimer(+0x34)` → ticks(4), `0x283d88` unless 0x1413f5 | grows, glows | P; test `opens_and_lets_out_four_bots` (glows), `novalis_bots_blow_up_a_critter` (glows while held) |
| 1: 0x1413f4 == 0, owner alive, hand class 0xe5 → stay; else `DeleteMoby` | | P (the owner = the glove in hand [L]) |
| 2: scale += class·0.05·gs below class·0.5·gs; the glow timer; vz −= 9·dt²; `FUN_00212960(0.3·gs, next + 0.2 up, 4, Ratchet)`, else `CollLine_Fix(bottom, next bottom, 0, Ratchet)`; none → moved | the flight | P `doom_canister::fly` |
| 2 hit: to the pushed centre (0.2 down) or the line point (bottom back up); `FUN_00221570` reflect ×0.5; slow (< 0.01): +0x38++, world / mesh (0x1742dc > 0) / > ticks(240) → 3; (platform ride); pos += vel; `0x2ddce0` | | P; test (opened) |
| `0x2ddce0`: surfaces 0, 1, 3, 8, 0xb, 0xc, 0xd → 20 × (3 `randf(±0.3)`, 3 `rand`, `randf(1e5, 8e5)`, `rand`) `PartType05Spawn`, state 4 | the pop | P `doom_canister::pop` (type 5 records: **G-PRT-001**); test `pops_on_a_pop_surface` |
| 3: `FUN_0026e618` + `FUN_002752c0` | ground probe for the platform ride | n/a (0x13f64c never set; no other effect) |
| 3: `FastDecTimer(+0x38)` → count class 0xba not dead in the moby list; < 8 → `FUN_002d5a70(owner, pos + v, v)`, v = (cos, sin)(k·π/2)·0.05 + 0.02 up, +0x38 ticks(8), +0x3c++, > 3 → 4; else +0x38 ticks(15) | the bots | P `doom_canister::open`; tests `opens_and_lets_out_four_bots` (8 apart), `waits_while_eight_bots_are_out` |
| 3: +0x3c < 2 → the glow timer | | P |
| 4: alpha −4, scale ×0.92, (ride), alpha 0 → `DeleteMoby` | the fade | P; tests (deleted) |
| default → 0 | | P |
| gold (0x13e534: size ×2, sphere 0.6) | | **G-WPN-009** |
| `0x283d88` / `0x283ea8` (type 32): the glow riding the canister (alpha pulse 0 → 0.6 → 0 by 0.016, white → blue, turning, size scale·3e6 + 5e4) | the glow | P `particles::type32` (new); test `type32::tests::pulses_follows_and_dies` |

*The bots* `0x2d5c10` (+ `0x2d5a70`, `0x2d4718`, `0x2d47f8`, `0x2d4830`, `0x2d4868`, `0x2d48b0`, `0x2d48e8`,
`0x2d49f8`, `0x2d4cb8`, `0x2d5478`, `0x2d5518`, `0x2d5658`, `0x2d5b30`, engine `0x26de80`, `0x26d8b0`, `0x270340`,
`0x26e3e8`):

| address / branch | what it does | status |
|---|---|---|
| `0x2d5a70`: `CreateMoby(0xba)`, distances 0xff, visible, state 0, +0xbc 0, pos, velocity, owner, matrix, Ratchet's lighting (`FUN_00272078`), scale = class·0.1 | | P `doom_bot::create` |
| game mode 2 → hidden (mode \| 0x41), return; else shown | | P |
| out of the world box → `DeleteMoby` | | P |
| `MobyGetHitMessage(0x800001)` → 0xb | set off by explosions and enemy hits | P; test `explodes_when_hit` (0x800000 and 1 set it off, 0x10000 not) |
| scale eases 5 % to class·g; +0x58 anim speed `(0.5·gold + 1)/g` in sequences 0 / 6 / 7 | | P (g = 1: gold not mirrored) |
| not 0xb: J0 = trunc(183.296·g) + 1, J12 = 0, J1 = 0.198·g + 0.02 | the walker globals | P (per bot, `pv::J`); test `drops_out_and_walks` |
| state 0: the jump record cleared; J2 / J3 / J7 / J8 / J9 / J10 / J15..17, R gravity 29.7·dt², R max 0.1·g, R keys 7.5 / 11.5, sequences 2 / 3 / 4; +0x40 / +0x44 / +0x5c; target 0; → 0xe; `0x2d47f8` / `0x2d4868`; sequence 5 (ticks(5)) | init | P `doom_bot::init` |
| 0xd: owner alive and in the box → return; else delete | | P (0xd is never set: dead code, ported as written) |
| age++ > ticks(3600) → 0xb | the lifetime | P; test `lives_sixty_seconds` (3601st update) |
| `FastDecTimer(+0x30)`; +0x20 = pos | | P |
| `0x2d4718(5)`: a targetable creature's primitive → 0xb; the world or a primitive → onto the pushed centre (the `fabs(z − z) > 0.6` hold never true), velocity = the normal clamped 0.05 (3-D, xy), 8, a target → failed + 500; sequence 5 | touching / pushed | P `doom_bot::knocked`; test (Rilgar: the first bot out touches the amoeboid and blows up) |
| z < 0.1 → delete; +0x30 > 4000 → 0xb | | P; test `runs_out_of_patience` |
| `FUN_0026e618` + `FUN_002752c0` | the platform ride | n/a (0x13f64c) |
| 0: `STUB_printf`, → 1; 1 / 9 / 10: wrap → 2 | | P (debug print n/a) |
| 2..5: target alive: reach = g' + record +0x0a / 8 (z + record +0x10); dist < reach and `FUN_00212960(g', pos, 1)` → 0xb | explode on touching the target | P `doom_bot::chase`; tests (the critter blown up) |
| 2..5: within reach/2 (xy, z) → `0x2d4868`, `0x2d5518`, `0x2d48b0`, point = target, top 0.066, sequence 0, state 6 (overwritten below: the game's) | | P |
| `randi(4) == 0` → `0x2d49f8(bot, pos + 0.3 up, rot, failed, 1)` | the search | P `doom_bot::search`; tests (the critter's moby as target; Rilgar) |
| top 0.1, state 4; no target: `randf(±π)`, `randi(4)` → `0x2d4cb8(h, 1.85, 4, 0.35g, 4, J0/1024, pos, 0)` else clear 5 | wander | P |
| a non-Ratchet target dead / classless / not type 5 / not targetable → 0, state 2, top 0.0273 | | P |
| no target → 5, sequence 6 (ticks(10)), top 0.0273 | | P |
| target: `randi(4)` → `0x2d5658(…, target, 1)` → point, top 0.066, sequence 0 (ticks(2)), 6 | the jump at the target | P |
| Ratchet → 5, on the wrap sequence 6 (ticks(0)), top 0.0273; d ≤ 20 → 4 (sequence 7), else 5 (sequence 0) | | P |
| `randi(4)` → `0x2d4cb8(yaw, 1.85, 4, 0.35, 4, J0/1024, pos, target)` else clear 5 | | P |
| state 4 after 5 → 0xc (sequence 1) | never (the game compares the state it just wrote) | P (dead, as written) |
| the walk: point = pos + 5·(cos, sin)(+0x64), 1 up; `0x2d47f8`; `SpringTurn2(+0x64, J15, J16, J17, &+0x44)`; `CollLine_Fix(pos + 0.15 up, + 5·vel (z 0), 6)`: clear / the target / a creature → `0x26de80` (height jump > 1 undone, `0x2d4718` push); else clear 0, backing off (0.1 from the point) | | P `doom_bot::walk` + `walker::walk_to` (shared with 1818) |
| `0x2d4830`; blocked step or clear < 4 → `0x2d5658(…, point, 0)`: jump (6, sequence 0) or failed + 500, backing off, 8, sequence 5 | | P |
| 6: `SpringTurn2(to the point, 2·J15, J16/2, 2·J17, &J5)`; within 5° → sequence R+0x2c (ticks(3)), 7 | | P |
| 7: `0x2d4868`, `0x270340` → 4 (sequence 7, top 0.1), `0x2d48b0` | the jump | P `doom_bot::jump_step` |
| `0x270340`: 0 wind-up past key 7.5 (wrap → the air sequence, 1); 1 turn, move, gravity, the descent test (33 ticks) → the landing sequence, 2; 2 fall to `GroundHeight` → 3; the wrap → 4 | | P |
| 8 / 0xe: gravity, `0x26d8b0` = `walker::move_collide(0.7g, J0/1024, 0)`, ground → vz 0, ×0.5, clamp 0.05 (xy / 3-D); the wrap / the ground → 2, else sequence 5; `SpringTurn2(atan vel, J7, J8, J9, &+0x40)`, `0x2d4830` | backing off / dropping | P; test `drops_out_and_walks` |
| 0xe: `FUN_00212960(0.5g, pos, 1)` on another bot → pushed 1.1 × the gap out (xy) | apart from the other bots | P `doom_bot::drop_out` |
| 0xb in the box: `SpawnBeamExplosion(0, 0, 2g', g', 9, 1, 15, bot, (0,0,1,1), pos, 5, 2, 4, −1, 1, 5, −1, gold)` | the blast effect | P `doom_bot::BLAST` (`fx::beam_explosion`); tests `lives_sixty_seconds` (streaks, sparks, light, shake), `novalis_bots_blow_up_a_critter` |
| 0xb: `FUN_00214468(g', pos + 0.5g up, 0x15, bot)` + `FUN_0026f8f8(3, 1, 1, bot, pos, list, n, 0, 0x10000, 2, 3)` | the blast's hits | P `attack::area_push` (split out of `area_hit`); tests (the critter 1 → −2; the crate broken and its bolts; the other bots' records replaced) |
| 0xb: `fun_0022da68(0 (gold 4), 0, bot)`; `DeleteMoby` | the class sound | P; tests (class sound 0) |
| 0xc: the wrap → 4 (sequence 7); 0xd: `STUB_printf`; default → 0 | | P |
| `0x2d5b30` after each `break`: line (z + 1 → z − 0.3, flags 6): a creature or surfaces 0, 1, 3, 8, 0xb, 0xc, 0xd → 0xb | landing on a creature / water | P `doom_bot::landing_check`; test `explodes_on_a_pop_surface` |
| bounds → `DeleteMoby` | | P |
| `0x2d49f8` (module doc of `search`) | the target set: the list 0x1abe80's class-type-5 mobys (no crates), 40 / 30 / 8 / 3.25, the fallback to Ratchet beyond 3.2 (xy); the failed target is passed but not read | P |
| `0x2d4cb8` (module doc of `hop`) | the heading | P; a probe meeting the (empty) target — without a target the world — returns at once [M] |
| `0x2d5658` (+ `0x2d48e8` ground, `0x26e3e8` apex) | the jump planner (halves to 0.5); +0xbc = 0xd | P `doom_bot::plan_jump` (+0xbc read by nothing) |
| `0x2d5518`: speed, heading, `0x26faf0` lob (≤ 7·dt) | | P `doom_bot::solve_jump` (0 / 0 with no distance across: [L]) |
| `0x2d5478` (the debug arc lines) | | n/a (draws nothing in retail) |
| blob shadow / draw callback | none registered by the bots or the canister | n/a |
| gold bots (0x13e534: size, reach, anim speed, sound 4) | | **G-WPN-009** |

**Globals kept per bot** [L]: the walker record 0x141210 and the jump record 0x141260 are shared in the game but every
bot rewrites their constants (each tick or in its state 0) and loads its own fields before every use, so each bot
carries a copy (`pv::J`, `pv::R`); the BSS words the bots never write (J11, J13, J14 flags, J19) are 0. **Quirks kept**
[M]: a bot without a target sees the world as its target in the hop solver, the walk's wall probe and the jump arc (the
game compares the hit moby with the empty target); the proximity jump is overwritten by the search's state 4; the
state-8 back-off waits for the fall sequence 5 to wrap (19 frames at 1/6: up to 114 ticks).

**A miss fixed on the way** (§14.4's target acquisition): `0x274b78` / `0x274df8` measure a decoy's distance with
`fun_001f9b80` = `VecDistance2` **0x221398 (xy)**; the port used the 3-D distance (`creature::target`). Fixed; the
gloves / creature tests are unchanged.

**Native, not emulated**: standard `f32` everywhere; the rand draws are the game's (`randi(4)` gates, `randf(±π)`, the
pop's and the glow's draws). **Inferred [L]**: the owner is "the Glove of Doom in hand"; the globals per bot (above);
moving platforms n/a (0x13f64c never set); the type-5 puffs are records only (G-PRT-001); 0 / 0 in `0x2d5518` keeps the
speed; the gold canister / bots not mirrored (G-WPN-009).

**Shared pieces and their users**: `hero::gloves::update` + `short_throw_aim` (Decoy, Doom); `weapons::launch_velocity`
(Bomb, Decoy, Doom); `decoy::short_glove_preview` (the decoy, the canister) and `decoy::reflect` (the decoy, the
canister); `walker::walk_to` = `0x26de80` (the mouse 1818, the bots); `walker::step` / `move_collide` (the bots, the
mouse, 577, 572, the chicken, the decoy, the drones); `attack::area_push` = `0x26f8f8` on a caller's list (the bots;
`area_hit` = the mine, the gold chicken, the tank); `fx::beam_explosion`; `targeting::record`; `knock::lob_up`
(the knockback, the bots); `ground::key_time` / `ground::ground`; `turn::spring_turn2_pvar`; particle type 32 (the
canister).

**Tests** (twice identical where integration): `tests/weapons/hero_doom.rs` — `novalis_bots_blow_up_a_critter` (the throw from
0x23 with voice 0x1a and one ammo, the next canister at once, the canister 0 → 1 → 2 → 3 → 4 → gone, glows held and
flying, four bots 8 ticks apart, a bot targeting critter 592, its blast: light, sparks, streaks, puffs, the shake,
class sound 0, the critter 1 → −2 and dead, another bot's blast elsewhere), `rilgar_bot_blows_up_an_amoeboid` (the
first bot out touches the amoeboid 866 and blows it up; Ratchet untouched), `novalis_bot_blast_breaks_a_crate` (an
explosion's hit sets off the first bot; its 0x10000 blast breaks crate 376, its bolts fly, and replaces the other
bots' records: one blast), `novalis_glove_of_doom_clicks_without_a_canister`; unit (`classes::doom_canister::tests`,
`classes::doom_bot::tests` on the moby bench, `particles::type32::tests`): opening and the four bots, the pop, the
8-bot limit, the drop and walk with the globals' constants, the 3601-tick life, the hit masks, water, patience, the
apex. The gloves, Bomb Glove, reactive, morph, weapons4, guns, weapons, targeting, creature (incl. enemies), gadgets
and level-ports tests: green; `novalis_hero_digest` (NO_IDLE) md5 596306d7…, unchanged.

**Frames** (scratch `doom/shots/`, 200 frames each, two runs byte-identical; `RC_SETTINGS_FILE=0 RC_SCENE=0
RC_AUDIO=0`, frames 1..200): Novalis `RC_GIVE_ITEMS=20 RC_HERO_AT=148.6956,177.82448,40.5,3.14159
RC_PLAY_SCRIPT='40-41:press CIRCLE'` (`grid_a1.png`: 50 the canister glowing in the glove, 90 thrown (2/10) with the
critters coming, 110, 125; `grid_a2.png`: 130 / 135 a bot's blast at a critter ahead of Ratchet, 150 a bot beside a
critter in the debris, 190 another blast, a critter dying and its bolts); Rilgar `RC_LEVEL=5
RC_HERO_AT=161.52519,325.2638,26.5,1.5708 RC_PLAY_SCRIPT='30-31:press CIRCLE'` (`grid_r1.png`: 60 the canister in
flight, 100 landed by the amoeboid 866, 110 the first bot's blast on it, 120 the amoeboid's burst).

## 19. Joint-modifier producers, Clank's eyelids and glow, the wrench rebound (W3 lane 1, 2026-09-30)

The class side (the shared manipulator records, the NPC look-at, the vendor's hologram, the Visibomb fins) is
moby_animation.md §9. Here Ratchet's and Clank's writers. Native `f32`; constants cited from the disassembly.

**Coverage** (every call and branch of the ported parts):

| address | what it does | port |
|---|---|---|
| 0x238634..0x2386f8 (`0x2370b8` 0x33..0x35) | springs of records 1 / 2 / 3 (`0x22b5e0(0.009, 0.22)`, `0x22b5f8(0.04, 0.2)`, `0x22b610(0.02, 0.2)`); record 1 targets (1.4·roll, 47·Δpitch within ±0.8727, 1.5·roll) with roll = the roll target, Δpitch = this tick's pitch change (0x237f18 `fast_subtract_rotations`); records 4 / 5 (the feet) (–, 40·Δpitch, 1.1·roll), springs (0.03, 0.22) | `hero::swim::Hero::swim_lean` |
| 0x23a9f8, 0x23ab60..0x23abbc (`0x2370b8` 0x31) | the turn `FastDiffRots` (≥ 0; the port had used the signed difference: fixed, the body now rolls one way as in the game), capped at 80°·dt, ×9; the springs of records 1 / 2 / 3 as above; record 1 targets x 3.5·Δ, z 2.1·Δ | `hero::surface::sinking_floor` |
| 0x23c458 (every 5th tick) | the edge probe: on the ground, a line (flags 2) hero-local (1.1, 0, 1) → (1.1, 0, −20) (end z ≥ 0.5); a floor within 3: none; else capsule-radius spheres at (0.7 + 0.1k, 0, 0) until one is free (none before 1.6: none); 0x13f5b0 / 0x13f5a8 (the drop, 20 without a floor) / 0x13f5ac (radius + offset, ≥ 0) | `Hero::edge_probe` |
| 0x22b928 edge branch | a drop > 1.4 and no wall within 1 (0x13f598): head (0.015, 0.3) y = min(1.5°·drop + 11°, 27°, 35°·(1 − room)); neck (0.008, 0.3) y = 0.7 of it | `Hero::head_look` |
| `0x2278c0` | nothing while Clank is hidden (0x141628; the control modes 1 / 2 put these on Ratchet's moby: G-HERO-005); the glow word, red on the hit flash 0x13f53e (fading in over 5 of 45 ticks, out over the last 20) | `Hero::clank_glow_blink` |
| `0x2278c0` blink | frames 1..21: the eyelid nodes 0x140240 + 0x40·k on Clank's lists 2..5 (gp−0x7508) attached once (node 3 first), mode 1, poses 0x17c660 / 0x17c6a0 / 0x17c6e0 (× Clank's scale), weight `CLANK_BLINK[frame]`; all detached at frame 0 | `Hero::clank_modifiers`; `rc-engine` `moby_attach` poses Clank with them |
| `0x2278c0` +0x90, +0x34 \|= 0x10 | Clank's glow packets in the pulse's colour | `moby_attach` (`ExtraMobys::set_glow`) |
| `HeroItemsAttach` 0x22fec0 (slot 3, Clank) | `CreateMoby(1204)` (glow 0x801432d7, draw distance 0x40, scale × 1.3, mode 0); every frame it shows: `MobyAttachToJoint(clank, 6)`, its rows and position, Ratchet's light, the ticks(120) pulse (red min(0xd7 + 90s, 0xff), green 0x32 + 50s, blue 0x14 + 10s), mode \|= 0x10 | `moby_attach::place_antenna`, `hero::idle::antenna_glow` |
| `0x229400` → `0x229440` | the red dot glow quad at the antenna point with the Heli- / Thruster-Pack ready (and the other control modes' quads) | NOT ported (G-REN-005) |
| MobyProc point lights | the point lights reach the hand, back and worn items (their centre: the item's position [L]) | `moby_attach::upload` |
| 0x2be5d0..0x2be710 (the wrench update) | the swing's line / sphere moby with a target record flag 2 (+0x1e, `FUN_002bdad8`) → `SetState(0x21, 1)`, 0x13fdb8 = the direction from that moby to Ratchet (behind him without one); else the hit sound once, unless the class type is 0x14 (line) / the sphere's is 0x12 | `hero::melee::wrench_update`, `rebounds` |
| SetState 0x21 | group 10, speed 9·dt, 0x1415d4 = 0; sequence 0x27 + the row's step (2 for the jump attack's row) from frame 5 over ticks(5), the wrench 7 + step | `Hero::rebound_entry` (the wrench's blend lands one tick later: `pending_blend` [L]) |
| `0x2370b8` / `0x242930` 0x21 | 0x7a's brake along 0x13fdb8 and idle once the anim wraps | `packs::rebound_physics`, the registry |

**Digest**: `novalis_hero_digest` NO_IDLE byte-identical (the posed joints sit in the idle block; no swimming, sinking,
edge or rebound in it). **Rng**: the Water Pump Worker's head look (moby_animation.md §9) draws `randf` three times per
glance on Novalis, which moves the pit's critters; three weapons tests were re-placed (the mine 7 → 9 units, the morph
5 → 4, the Visibomb 6 → 8) and say so.

**Frames** (scratch, two runs identical): Clank on Ratchet's back at the Novalis vendor with the antenna's glowing tip
(before: the plain dark dot).

**Not ported at the time** (G-HERO-009; the list below was ported 2026-10-01, next section): the Magneboots aim lean `0x2352e0` (needs a weapon in hand on the boots, which the port's
boots code keeps false), the pack record 18 `0x235e60` (Clank's record on the back through `0x227050`'s other record
kinds), `HeroScanTargets`' look target, the grind's body lean and look-ahead records (0x17a6e0..), the side probes'
model tilt (0x13f610..0x13f624), the slope foot IK `0x22c5c0`; the big-head cheats (G-SAV-006).

**The rest of the hero producers (2026-10-01).** `rc_game::hero::pose` (the feet `0x22c5c0`, `HeroScanTargets`, the
Magneboots lean `0x2352e0`, Clank's sway `0x235e60`; its module doc has the full coverage table), `hero::boots::grind_lean`
(level00's grind records), the side probes in the ground probe's tail. System or not: all of them write the one shared
record block 0x17ab00 (`idle::JointRec`, springs `0x2273d0`); each writer is per-state code, ported as such. The
record block grew: records 6 / 7 (lists 7 / 8, the legs), 18 (kind 1: Clank's list 0, linked into Clank's list through
`Hero::clank_modifiers`, only while Clank's moby exists), and the translation springs (+0x70 / +0x80 / +0x90, `0x270780`)
every record runs. Corrections: `0x2352e0` needs no weapon (only the magnetic floor 0x13f658); `0x248920` is not the pad
vibration (packs.rs said so) but the foot motes' parameters.

| address | what it does | port |
|---|---|---|
| 0x232dc0 tail | side probes (±90°, 0.2 out, ±0.45, flags 0x22): drop, pitch, roll per side (0x13f610..0x13f624) | `Hero::side_probes` |
| `0x22c5c0` | the slope tilt of records 4..7, the worn feet's scale 0.01, the foot motes (state 3, the jump landing's first 12 ticks; type 28 at lists 22 / 23 with their draws) | `Hero::feet_update` |
| `0x248920` callers in `0x242930` | the landings' mote parameters: fall 6 (hard, roll, walk, plain), the jump group's landing, the glide 8 (crouch, idle, run with rows), the hover 0x81 | `air::tr_fall`, `jump`, `packs::tr_glide` / `tr_hover` (`pose::motes`) |
| `HeroScanTargets` 0x22c080 | the look target (every 7th tick; record +0x38 range, +0x39 priority, +0x3a height), the line from the head, records 3 / 2 | `Hero::scan_targets` (the target list: `melee::MeleeTarget::look`, now collected every tick) |
| `0x2352e0` | the Magneboots lean: records 1, 3, 13..16 from world down in the hero's frame | `Hero::magnet_lean` |
| `0x235e60` | record 18 in the walk (state 2): sway z against the turn, the 0.07 rise in Clank's units | `Hero::clank_sway` |
| L00 0x21ce0c..0x21d1b0 | the grind: record 0 y (the rail's bend 30·\|eff\| ahead, after 90 ticks), x (the displacement's pitch), the gun lean (records 3 / 1 / 2 z by sequence 0x31 / 0x4a / 0x4b) | `boots::grind_lean` (the look-at moby 0x13f928 branch: n/a, no ported class sets it) |
| `0x22b700` | the arm lowered at a wall (record 9, 0x141618) | NOT ported (G-HERO-009; 0x141618 is never set) |

**Digest** (`novalis_hero_digest` NO_IDLE): state / timer / position columns identical on all 2,800 lines; the hashes differ
from the landing at lake 209 / moves 93 on: the landing's foot motes (their `Mote28` spawns in `fx.parts` and their
draws on the one stream). `HeroScanTargets` alone leaves it identical (no target list in the digest runs). Not
re-baselined. **Tests**: `tests/hero/hero_pose.rs` (7), `hero_boots_grind::kalebo_grind_leans`. **Frames** (scratch,
two runs identical, before = the producers switched off): Kalebo III rail 1 at tick 260 (the body bent into the bend),
Orxon's magnetic bowl with the Magneboots (the body pitched over the slope), the same bowl without them (the downhill
leg bent, the foot flat on the slope).
