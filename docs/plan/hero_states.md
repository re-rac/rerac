# Hero states: inventory, port packages, code structure

Every state of Ratchet's (and the other hero bodies') state machine, what reaches it, what it does, which module
of the port owns it and whether it is ported; the shared hero subsystems the missing states need; the six port
packages for the "finish Ratchet" push; and the restructured hero code that lets them be ported in parallel.
Companion of `docs/plan/player_controller.md` (the detailed spec of the ported states) — where the names in its §3
"noted only" list differ from this page, this page is the corrected one.

Confidence: **H** read from the instructions / decompiler C of the switch case itself, **M** from the case plus its
callers, **L** inferred (identity guessed from behaviour, flagged). Addresses are level01 unless marked `L00`
(level00, the superset build; decompiler C in `work/decomp/level00.elf/`).

## 0. How the state machine is built (and one surprise)

- Three switches on the state `0x1413d4`: **SetState** `0x23cf98` (entry: group byte `0x1413dc`, jump block, anim),
  **per-state physics** `0x2370b8` (called by the driver `0x231d18` before the move), **transitions** `0x242930`
  (after the move and the surface reaction; ends in `SetState(new, 1)`). Several ids share one case in each switch
  (the whole jump group shares one entry, one physics case and one transition case). **H**
- **Each level overlay compiles its own copy of the three switches** (clusters.tsv: `SetState` is unique per
  level, 13832 bytes on level01), with only the states that level can reach. Level01 (Novalis) has 83 ids; the
  union over the 19 levels has 126 SetState case labels; level00 (Veldin 1) has all but 0x3e (level 16 only) and
  0x6b..0x6f (levels 5 and 16: the Hoverboard). The surface reaction `0x22cd48` is per-level too (level01 lacks
  surfaces 1, 3 and 7; level00's is `0x20b960`). Per-level SetState: L00 0x2223f8, 02 0x22b728, 03 0x216648,
  04 0x2159d8, 05 0x24cee8, 06 0x2356a0, 07 0x24a260, 08 0x230b38, 09 0x2420d8, 10 0x215a60, 11 0x24db50,
  12 0x2400d0, 13 0x22dee0, 14 0x22ff58, 15 0x216c38, 16 0x21e398, 17 0x21e530, 18 0x227dd0; level00 physics
  0x217970, transitions 0x229b70. **H**
- **Port consequence:** the port implements the superset once (the "general" rule), with level00 as the reference
  for code that level01 does not contain. A state a level does not compile is unreachable there anyway (nothing in
  its data leads to it). Surface rules are the one place where the superset could change a level's behaviour
  (a face with surface 7 on a level whose reaction ignores 7): package P1 checks the surface ids each level's faces
  use against that level's own reaction.
- **Checked per level (2026-09-29, level_generalisation.md H2, `rc-trace overlay-diff`).** Every level's copy of the
  hero functions the port has is level 01's or level 00's code (the three switches differ in their case sets only),
  except: the surface Ratchet's capsule and `LandEta` pass through (0 / 3 on level 2 / 0xd on 6 and 0xe:
  `surface::PASS_SURFACE`, ported), Kerwan's own path search for the cables (`boots::FIXED_REACH_LEVELS`, ported),
  the Hoverboard and 0x3e on 5 / 16 (G-HERO-008) and the body branches on 15 / 18 (G-HERO-005). Where a level takes
  level 00's code the port matches only for the parts of level 00 it has (G-HERO-031).
- Scripts used for this inventory: `work/decomp/*/` (all levels' unique functions), the per-level case union and
  level01-name mapping through `tools/ghidra/names/clusters.tsv` (the scripts were throwaway; rerun by grepping each
  level's export for `param_1 != 100` + `iRam001413dc == 0x14`).

### 0.1 Items (the game state's owned table `0x13d4c0 + id`; `GetClankModule(slot)` 0x22ddd8 = the ready item of item slot `slot`, slot 0 the hand, slot 3 the back)

| id | item | owned flag | used by |
|---|---|---|---|
| 2 | Heli-Pack | 0x13d4c2 | glide 8, 10, 0xf; back module 2 |
| 3 | Thruster-Pack | 0x13d4c3 | 0x10, 0xd, 0x22, 0x81; back module 3 (the Thruster tests read only the module) |
| 4 | Hydro-Pack | 0x13d4c4 | 0x35 (ported); back module 4 blocks the glide |
| 6 | O2 mask | 0x13d4c6 | no air drain (ported) |
| 7 | Pilot's helmet | 0x13d4c7 | (ship) |
| 8 | wrench | — | melee (ported) |
| 12 | Swingshot | 0x13d4cc | hand item → 0x24 / 0x2c |
| 28 | Magneboots | 0x13d4dc | gravity mode 1 on surface 2; 0x3f / 0x70 / 0x71 |
| 29 | Grind Boots | 0x13d4dd | grind 0x28.. (the rail contact L00 0x20cf58 tests the flag first) |
| 30 | Hoverboard | 0x13d4de | 0x6b..0x6f |
| 31 | Hologuise | — | hand item → body mode 3 |

Names from Wrench's `types_rac.h` gadget enum (item ids), matched to the flags the hero code tests. **H** for the
flags read, **M** for the names.

### 0.2 Bodies (`0x1413f4`, "character mode"; `SwitchCharacter(mode, state, moby)` L00 0x210a08)

0 Ratchet · 1 Clank (own health 0x1415fc) · 2 Giant Clank (energy 0x140980 = 200) · 3 the Hologuise disguise (moby
class 0x27a, entered after the Hologuise's 18-tick timer 0x14162e, state 0x53). The states 0x43..0x52, 0x5a..0x62,
0x53..0x59 belong to bodies 1, 2, 3. **M** (1 and 2 from their levels and moves: **L**)

### 0.3 Groups (`0x1413dc`)

0 standing · 1 walking · 2 falling · 3 ledge · 4 jumping · 5 gliding · 6 melee · 7 hurt · 8 weapon stance ·
9 scripted · 0xa rebound · 0xb stomp · 0xc crouch · 0xd Swingshot pull · 0xe Swingshot swing · 0xf grind ·
0x10 sinking floor · 0x11 underwater · 0x12 water surface · 0x13 gadget pose · 0x14 dead (SetState refuses
everything but 100) · 0x15 (level 16) · 0x16 Hoverboard · 0x18 cutscene · 0x19 sinking liquid · 0x1a cable.

## 1. State inventory (0..=0x82)

Columns: **grp** = group byte; **reached by** = where SetState(id) comes from; **leaves to** = the transitions;
**module** = the port file / package; **st** = port status (**P** ported, **E** entry only, **–** not). The same
table is `crates/rc-game/src/hero/registry.rs::STATES` (name, group, module, ported).

### 1.1 Ratchet on foot (ported)

| id | name | grp | reached by | leaves to | anims | module | st |
|---|---|---|---|---|---|---|---|
| 0 | idle | 0 | landing, stop, walk release | 1 L1/L2, 0x81 hover latch, 7/0xb jump, 0x71 (magnet ✕), 4 R1/R2, 6 coyote, 2 stick, 0x79 pit, 0x40 fidget record 2 | 0 / 0x54 (health 1) | ground.rs | P |
| 2 | walk / run | 1 | idle, landing, stop | 6, 7/9/0xb, 1, 4, 0x73 / 0x2f / 0x3f by surface, 0x28 rail, 3, 0 | 3 walk, 4 run | walk.rs | P |
| 3 | stop / skid | 1 | walk, landings | 1, 7, 4, 6, 0, 2 | 5 / 6, 0x14 | ground.rs | P |
| 4 | crouch | 0xc | R1/R2 | 1, 0 release, 0xb / pack crouch jumps / 7, 0x71, 0x15 (□), 6 | 0xd, 0xe/0xf turn | ground.rs | P |
| 6 | fall | 2 | coyote, jumps' fall-over | landing 0/2/3 (0xc hard), 0x3d, 8 glide, jump (stuck), 0x10, 0x18 ledge, 0x28 rail, 0x74 cable | 10, 0xb, 0xc | air.rs | P |
| 7 | jump | 4 | ✕ | 0xb flip chain, 10/0x10 R1 tap, 0x22, 0x28 rail / 0x74 cable (rail / cable overlays), 6, landing picker, 9/7 bunny hop, 0x11 wall, 0xe, 8 glide, 0x18 | 7 | jump.rs | P |
| 9 | running jump | 4 | ✕ running | as 7 (raw write to 7 when slow) | 8 / 9 by run frame (0x17c258) | jump.rs | P |
| 0xb | side / back flip | 4 | crouch + side/back stick + ✕ | as 7 (no double jump) | 0x1c/0x1d/0x1f | jump.rs | P |
| 0xe | double jump | 4 | ✕ in 7/9 after 14 ticks | landing, 6, 8 glide, 0x18 | 0x16 fr 3 | jump.rs | P |
| 0x12 | water jump | 4 | ✕ treading water | as 7, 0x37 | 0x72 | jump.rs | P |
| 0x13 | wrench combo | 6 | □ (item 8) | 0x13 chain, 0x15, 7, 0 | 0x17..0x19 | melee.rs | P |
| 0x14 | wrench jump attack | 6 | □ in the air | 7, 0 | 0x2b | melee.rs | P |
| 0x33 / 0x34 / 0x35 / 0x36 / 0x37 / 0x6a | swim (see player_controller §14) | 0x11 / 0x12 / 0x14 | water | — | 0x3b..0x3d, 0x65, 0x3a, 0x5f | swim.rs | P |
| 0x73 | wade | 1 | walk in 0.25..0.85 of water | 2, 0 | 0x60 | walk.rs | P |

### 1.2 Ratchet, not ported

Physics column: the case of `0x2370b8` (L00 `0x217970` where level01 lacks it) and the helpers it calls.

| id | name | grp | reached by | leaves to | physics / helpers | anims | module (package) | st |
|---|---|---|---|---|---|---|---|---|
| 1 | look stance | 0 | L1/L2 in 0/2/3/4 | 0 release (anim `0x226f10(1)`), 4 | ground case + camera-facing turn (0x1413f5) | `0x226f10(1)`, 0xd | stance.rs (P2) | P |
| 8 | pack glide / hover | 5 | ✕ held in 6 (> 0.6) / jumps (> 0.6, 1.0, 1.5), Heli-Pack owned, back ≠ Hydro | 6 after 30 ticks without ✕, 0x18, 0x11, landing 3/4/0/2 (0x13f524 = 12 − T) | target 3·dt (5·dt with 0x1404f8 = 3), TurnTo(0.025, 0.3, 720°/s), SpeedStep(15, 7)·dt², vz 0x248f68/0x248b68, sound slots, `HeroWallLedgeCheckA` | 0x13 curve −2 | packs.rs (P4) | P |
| 0xa | Heli-Pack long jump | 4 | crouch + ✕ moving (stick > 0.7, fwd > 0.9·x) with the Heli-Pack; R1 tap in 7/9 | 3 on landing (anim 5, momentum ×0.8), 0x7a wall | jump case: h 1.9, g 11·dt², air 120 | 0x12 | packs.rs (P4) | P |
| 0xc | jump variant (dead) | 4 | **no caller on any level** (2026-09-29, "Scripted control"); only the shared jump-group case labels name it | jump case | jump case | — | jump.rs (none) | – |
| 0xd | Thruster-Pack high jump | 4 | crouch + ✕ standing, Thruster | jump case | curve jump: h 0.1..0.2, takeoff 9, table 0x17c3a0, window 9..44 | 0x11 | packs.rs (P4) | P |
| 0xf | Heli-Pack high jump | 4 | crouch + ✕ standing, Heli-Pack | jump case | curve jump: h 1.9, takeoff 9, table gp−0x7518, fall-over 4.5, air 130 | 0x15 | packs.rs (P4) | P |
| 0x10 | Thruster-Pack long jump | 4 | crouch + ✕ moving / ✕+R1 combo in 6 / R1 tap in 7/9, Thruster | 6 after 60 ticks > 1.5, 0x7a, 0x22 | jump case: h 0.4..0.9, ramp 1, g 8.5·dt², acc 44 / dec 50·dt², 0x13f744 = 11.5·dt | 0x26 | packs.rs (P4) | P |
| 0x11 | wall jump | 4 | ✕ within 7 while the wall window 0x13f504 is open (7, 9, 0x11 after 25) | jump case | jump case: h 3.3, takeoff 9, g 29·dt², 0x13f7c0 = the wall normal | 0x23 | ledge.rs (P3) | P |
| 0x15 | comet strike | 6 | crouch + □ (crouch transitions, with the wrench), □ late in 0x13 / 0x15 while crouched | 0 (row 3's idle frame; another hand item), 7 / crouch jumps (✕ after the row's jump frame); with the wrench after 100 ticks the loop exit `0x247d18(2)` + `0x13fe04 = 0x1a` | 0x15 case: speed 0, aim (or the turn to it, 15 rad/s), **the throw `0x236da0` at key time 33**, SpeedStep(37, 28), the combo's gravity; the wrench's flight `0x2be1c0` states 10 / 11 | 0x1a, loop 6..0x15 | comet.rs | P |
| 0x16 | hurt (knockback) | 7 | hit intake on foot | 6 (airborne after frame 12.5 above 1), 0x3d, 0 / 0x81 after 27 | |vel| − 4·dt² (10·dt² grounded after 10), edge brake on 0x140637, vz −= 0.004 | 0x10 curve −3 fr 3 | damage.rs (P2) | P |
| 0x17 | weapon fire stance | 8 | only 0x2e's transitions (no SetState caller in any level) | 0 when the fire button is released and 0x13f520 = 0 | ground case (`0x2324f8` turn) | idle blend 10 | weapons.rs (unreachable) | – |
| 0x18 | ledge grab | 3 | 6, 8, jumps with 0x13f838 | 0x19 after frame 13.5 | turn to ledge yaw + π (0.04, 0.2, 360°/s), pull to 0x13f820, ClampLen 4·dt | 0x20 | ledge.rs (P3) | P |
| 0x19 | ledge hang | 3 | 0x18, shimmy end | 0x1c (✕), 6 (back + ✕ / R1, 0x13f500 = 10 / 40), 0x1a / 0x1b (probe `HeroWallLedgeCheckC` 0x22d838 (L00 0x20c758) at ±90° 0.3) | as 0x18 | 0x21 | ledge.rs (P3) | P |
| 0x1a / 0x1b | shimmy left / right | 3 | 0x19 + stick | 0x19 (wrap / no probe), 0x1c, 6 | two probes `HeroWallLedgeCheckB`, speed from table 0x1c4130 by frame | 0x24 / 0x25 | ledge.rs (P3) | P |
| 0x1c | ledge climb / jump up | 4 | ✕ hanging | jump case | jump case: h 2.0, takeoff 10, g 25·dt²; carry follows 0x13f848 | 0x22 | ledge.rs (P3) | P |
| 0x1d / 0x1f / 0x32 / 0x72 / 0x78 | class holds: 0x1d steering the Visibomb, 0x1f held with control (the mouse), 0x32 mounted (turrets, vehicles; frozen), 0x72 the cinematics' hold, 0x78 held by the Umbris boss (vel = 0) — "Scripted control" (§3) | 9 | the classes that set them | only the setter's `SetState(0, 1)` | idle case (0x1d / 0x1f / 0x72) / none (0x32 / 0x78) | idle / 0xb | scripted.rs | P (all, 2026-09-29) |
| 0x3b | bolt crank (was "item use") | 9 | `SetState(0x3b, 1)` by the bolt crank class 280 (`0x2e0c68`, levels 1 / 4 / 8): □ at the bolt with the wrench | 0 by the crank (□ after 60 ticks, or done); the hit intake | none (frozen 0x1413fd: the crank stores his position, yaw, target yaw and clears 0x13f430..0x13f4bf every tick) | 0x44 latch, 0x3f hold, 0x40 / 0x41 turn; wrench 0xb | crank.rs | P |
| 0x1e | look stance (mobys) | 0 | four moby classes | 0 | ground case | `0x226f10(1)` | stance.rs (P2) | P |
| 0x20 | gadget lunge (the Walloper) | 6 | ○ with the Walloper (group 0 / 1, or 4 once landed; not state 1) | 0 after frame 20 | lunge 18·dt ticks 10..18, 3 hit spheres (0.8 ahead, ±50°) | — | walloper.rs (hero_gameplay.md §15) | P |
| 0x21 | wrench rebound | 0xa | wrench hit on a flag-2 target | 0 on wrap | speed → 0 by 24·dt², away from 0x13fdb8 | 0x27 + row | melee.rs (later) | – |
| 0x22 | Thruster stomp | 0xb | R1/R2 in the air (7, 9, 0xd, 0xf, 0xe) with the Thruster | 0x10 (✕ early), 0 on wrap | up to 5.7 u/s (70·dt²) for 12 ticks, held until tick 33, then 100·dt² down; while coming down and airborne `coll_sphere_mobys` (0.8, feet − 0.5, flags 0x10, template 0x30000); camera shake 0x167260 on the ground (`hero::fx::shake`) | 0x2a | packs.rs (P4) | P |
| 0x23 | glove throw | 6 | the throw gloves' ○ (items 10, 0x11, 0x14, 0x18, 0x19) standing / crouched / walking slowly; the holster check `0x2405f8` | 7 / crouch jumps (✕ within 11 after key time 19), 0 from 21 | aim (`0x2351d0(11, 50°, −1)`) or turn to it, SpeedStep(30, 35), gravity 54 (25 in the air + the steep-wall stop); the glove's update throws at tick 16 | 0x2c from 7 | weapons.rs | P |
| 0x24 | Swingshot fire | 0xd | ○ at a usable pull target 0x13fcb4 (weapon check case 0xc) | 0x25 / 0x26 (the hand item's update once the hook holds), 0 / 6 (blocked, released), 0 (hand item not the Swingshot) | turn to the target (0.02 / 0.15 / 360°/s), the ground case's braking at 12.6·dt², edge brake | 0x2f loop 10..14 | swingshot.rs (P6) | P |
| 0x25 | Swingshot pull | 0xd | the hook holds (record +0x08 = 0) | 6 (○ released / braked < 4 u/s; gravity 29, 42·dt²), 0 below 0.3 | speed → 27 u/s at 80·dt², brakes at 42·dt² to stop 1.9 short (`0x26e520` roots), left hand aimed, pitch to the target, sink in the last 30 ticks | 0x30, 10 at the end | swingshot.rs (P6) | P |
| 0x26 | Swingshot arrive | 0xd | the hook holds (record +0x08 ≠ 0: never on the disc) | 6 (○ released above 0.4), 0 | pendulum about the target, length 0x13fccc → record +0x04 at 10 u/s, damping 0.9, 15·dt² | 0x30 | swingshot.rs (P6) | P |
| 0x27 / 0x30 | weapon stances | 8 | no SetState caller in any level | 0 (0x30: anim 0x36 wrap) | ground-like (0x30: drag, edge brake, gravity) | idle | weapons.rs (unreachable) | – |
| 0x28 | grind | 0xf | rail contact 0x13f8bc from 0 / 2 / 3 / 6 / jumps | 0x29 (✕), 0x2b (□), 0x42 / 0x16 (hit or blocked), 6 (5 ticks off the rail end) | `crate::spline` nearest / advance on the grind paths (section 0x74), speed 12 u/s ± slope, rail pull spring, sparks type 25 (recorded) | 0x31 / 0x32 by stance, leans 0x4a..0x4d | boots.rs (P5) | P |
| 0x29 / 0x2a | grind jump / rail switch | 0xf | ✕ / ✕ + stick sideways in 0x29's first 10 ticks (a rail 2..4.5 aside) | 0x28 (landing, raw write 2 ticks later), 6, 0 | grind case + the jump system (h 2.4 (+4.7 booster 0x140638, +0.7 near one Kalebo booster) / 2.5; g 27 / 24·dt²); 0x2a blends old → new rail over 48 ticks | 0x50 / 0x1e, 0x1f | boots.rs (P5) | P |
| 0x2b | grind wrench | 0xf | □ grinding | 0x29 (✕ after the jump frame), 0x2b, 0x28 (idle frame), 6 | grind case; hit sphere 1.0 at the hip (delivered with the pack hits) | 0x4e / 0x4f, wrench 0xc / 0xd | boots.rs (P5) | P |
| 0x2c | Swingshot swing | 0xe | ○ at a usable swing target 0x13fce0 (better score than the pull target) | 0x2d (○ released after 10 ticks; gravity 27, 7·dt²) | rope 0x13fcf4 springs to record +0x04 (k / d / max +0x24..+0x2c or 0.025 / 0.33 / 11·dt), hooked (0x13fcec): gravity 40·dt², drag, stick pump, speed cap +0x0c, first-swing kick; body lean; anim re-timed to π·√(L / g) | 0x34 / 0x35 | swingshot.rs (P6) | P |
| 0x2d | fall after a swing | 2 | 0x2c | the fall's transitions (0x1415d4 = 7) | roll / pitch back upright about the point 0.6 up (`0x22a8d8`), then the fall case | 0xb | swingshot.rs (P6) | P |
| 0x2e | weapon draw walk | 1 | no SetState caller in any level | 0x17 / 0 after frame 28 | — | 0x2e | weapons.rs (unreachable) | – |
| 0x2f | slippery-floor walk | 1 | walk on surface 7 (0x140632; L00) | 2 off the surface, 0 | own case (L00 0x217970) | 0x37 / 0x6d | surface.rs (P1) | P |
| 0x31 | sinking floor | 0x10 | surface 4 (0x140633), grounded, not in groups 7/0x10/0x14 | 0 when off it and 0x13f530 = 0 (momentum = the carried part) | vz = min(disp.z − 50·dt², −40·dt²), turns with 0x13f440, particles 47, sound slot 0x141570, body roll springs | 100 | surface.rs (P1) | P |
| 0x38..0x3a | the Hydrodisplacer's use poses | 0x13 | the Hydrodisplacer item's update `0x309cd0` (item 22) on a pad 341 | 0 (0x38 / 0x39 unless holding item 0x16; 0x3a on the wrap) | velocity to the pad's centre clamped to 2·dt, yaw toward the pad's (≤ 9.42·dt a tick) | 0x46 / 0x47 / 0x48 (8 ticks) | hydrodisplacer.rs (2026-10-01) | P |
| 0x3c | burn bounce | 4 | surface 1 (0x140635; L00), damage 1 | jump case | jump case: h 5.5, takeoff 5, g 15·dt², 150 ticks; → 0x7c on level 10 / twice | 0x43 | damage.rs (P2) | P |
| 0x3d | death | 0x14 | health < 1 in 0/2, landing without health | fade 0x2319b0 on wrap | ground case | 0x45 | damage.rs (P2) | P |
| 0x3f | Magneboots walk | 1 | walk / SetState(2) with 0x13f658 = 1 | 4, 0x71, 6, 2 (off the floor), 0 (no stick after 30) | own case: 2..3.5 u/s inside ±70°, TurnTo / SetPlanarVel / gravity in mode 1 | 0x5b | boots.rs (P5) | P |
| 0x40 / 0x41 | fidget state / end | — | fidget record 2 | 1, 0x41 / 6, jumps, 4, 2, 0 | none | idle | stance.rs (P2) | P |
| 0x42 | grind hurt | 0xf | hit (0x13f90e) or blocked on the rail, health ≥ 2, the rail ahead clear | 0x28 (wrap, or falling within 0.3) | grind case at 2.5 u/s, hop 10 u/s, g 23·dt²; 1 damage, 77 ticks invulnerable | 0x10 curve −3 | boots.rs (P5) | P |
| 0x65 / 0x66 / 0x67 | walk to point (0x140990) | 1 / 1 / 0 | scripts (vendor, ship) | 0x67 / 0x65 / 0 | own cases | walk anims | stance.rs (P2) | P |
| 0x68 / 0x69 / 0x7b | sinking liquid (surface 3, to the level 0x13f644) / jump out / no health | 0x19 | surface 3 (0x140636; L00), level 0xd body contact type 0xb | 0x69 (✕) / jump case / fade | none (the liquid holds him) | 0x71 / 7 | surface.rs (P1) | P |
| 0x70 | Magneboots wrench swing | 6 | □ with 0x13f658 = 1 (weapon check) | 0 after the row's idle frame | ground case | 0x5c / 0x5d (melee rows 5 / 6) | boots.rs (P5) | P |
| 0x71 | Magneboots hop | 1 | ✕ in 0 / 4 / 0x3f with 0x13f658 = 1 | 0 after frame 25 | ground case, gravity mode 1 | 0x5e | boots.rs (P5) | P |
| 0x74 | cable slide (zipline: the wrench on a cable) | 0x1a | a **rising** jump-group state (7, 9, 10, 0xb..0x12, 0x1c, 0x3c, 0x69) or the fall 6 / 0x2d with the hands (0.3, 0, 1.34) −0.7 (−1.2 on the tick □ is pressed) .. 0.4 at a grind path (0x13f94c, `0x20d330`); code on 0, 3, 4, 7, 9, 10, 13, 17, cables only on Kerwan 3 (3) | 6 (5 ticks off the end; 8-tick lockout 0x13f534); no jump-off | spline follower on the hand point (speed → 14 u/s at 9·dt², half the entry speed along it to start, spring pull), sparks | 0x73 / 0x66 | boots.rs (P5; entries 2026-09-28) | P |
| 0x75 / 0x76 | hurt on the surface / under water | 7 | hit intake in groups 0x12 / 0x11 | 0x37 / 0x34 after 50, 0x6a / 0x82 without health | small cases | 0x70 / 0x6f | damage.rs (P2) | P |
| 0x77 | death fall | 2 | z below the level's death height and > 2 above ground | fade after 120 (level-specific z shortcuts) | fall-like, random spin (2 draws at entry) | 10 / 0xb | damage.rs (P2) | P |
| 0x79 | pit fall | 2 | surfaces 8 / 0xc (0x14063a) | fade after 300; 6 / 0 when off the pit surface | fall case | 0xb | damage.rs (P2) | P |
| 0x7a | pack jump rebound | 0xa | 10 / 0x10 into a wall | 0 on wrap | rebound case (with 0x21) | 0x29 | packs.rs (P4) | P |
| 0x7c | burn death | 0x14 | surface 1 without health (level 6: 0x3d) | fade | none | 0x71 | damage.rs (P2) | P |
| 0x7f | sinking death | 0x14 | surface 0xd (0x14063c) sinking | fade after 220 | own case (bubbles, CreateMoby) | 0x74 | damage.rs (P2) | P |
| 0x80 | hazard death | 0x14 | hit by class 0x4eb / 0x558 | fade | ground case | 0x7c | damage.rs (P2) | P |
| 0x81 | Thruster-Pack hover | 1 | R1 double tap on the ground (Thruster), idle with latch 0x14161a | 0 / land (✕ or R1 after 20 ticks; lockout 25) | own case (Spring, 0x2338d0) | 0x13 curve −2 | packs.rs (P4) | P |
| 0x82 | eaten in the water | 0x14 | hit by class 0x28f in water groups | fade | swim-death case | 0x80 | damage.rs (P2) | P |

### 1.3 Other bodies, Hoverboard, unused (not in this push)

| ids | what | grp | module |
|---|---|---|---|
| 0x43 idle, 0x44 walk (+0x50), 0x45 / 0x52 fall, 0x46 hurt, 0x47 death, 0x49 / 0x4c jumps (anims 7 / 9), 0x4a / 0x4b / 0x4d / 0x4e ledge (anims 10..0xc), 0x4f glide (0xe), 0x51 kick (group-6 entry in melee.rs), 0x7d burn, 0x48 (physics no-op only) | Clank (body 1; levels 0, 4, 6, 7, 9, 10, 13, 17) | 0..0x14 | Bodies (later) |
| 0x5a idle, 0x5b walk, 0x5c fall, 0x5d hurt, 0x5e jump (h 4.2), 0x5f..0x61 attacks, 0x62 death | Giant Clank (body 2, **L**; levels 0, 4, 7, 9, 10, 13, 15, 18) | | Bodies (later) |
| 0x53 idle, 0x54 walk, 0x55 fall, 0x56 hurt, 0x57 death, 0x58 pit fall, 0x59 ○ action | Hologuise disguise (body 3; every level) | | bodies/disguise.rs, hologuise.rs (2026-10-01) |
| 0x6b ride, 0x6c ramp jump, 0x6d crash, 0x6e into water, 0x6f into a wall; 0x3e (group 0x15, level 16) | Hoverboard (levels 5, 16) | 0x16 | hoverboard.rs: 0x6b..0x6f ported 2026-10-01 (with the race tracker, tricks, scoring, pickups; level05's source, which carries level 16's branches; level 16's copies diffed: the same code) and level 16's board weapon (item 0x24); 0x3e not ported, likely unreachable (no `SetState(0x3e)` on level 16: G-HERO-008) |
| 0x63 / 0x64 | cutscene control / the scene body (mode-2 scenes, the vendor); 100 is the only state group 0x14 accepts (respawn) | 0x18 | scripted.rs: ported 2026-09-28 (hero_gameplay.md §7) |
| 0x05, 0x7e | no SetState case (0x7e: a walk-case label only) | — | Unused |

### 1.4 Counts

131 ids: 2 unused, 32 other bodies, 6 Hoverboard, 91 Ratchet. Ported now **70** (after the push: the 19 below plus
P1 5, P2 18, P3 6, P4 8, P5 9, P6 5; the registry's `ported` rows). Before the push: ported 19 (ground 3, walk 2, air
1, jump 5, melee 2, swim 6). Not ported then, by package: P1 surface 5, P2 damage 11 + stance 7, P3 ledge 6, P4 packs 8, P5 boots 9, P6
Swingshot 5 = **51 in this push**; later: weapons 7 + melee 5 (0x15, 0x20, 0x21, 0x23, 0x51), scripted 8, jump
0xc 1. By game group (all ids, ported / not): 0: 1/7, 1: 3/10, 2: 1/8, 3: 0/8, 4: 5/11, 5: 0/2, 6: 2/8, 7: 0/6,
8: 0/3, 9: 0/6, 0xa: 0/2, 0xb: 0/1, 0xc: 1/0, 0xd: 0/3, 0xe: 0/1, 0xf: 0/5, 0x10: 0/1, 0x11: 3/0, 0x12: 2/0,
0x13: 0/3, 0x14: 1/9, 0x15: 0/1, 0x16: 0/2, 0x18: 0/2, 0x19: 0/3, 0x1a: 0/1, none: 0/9.

## 2. Shared hero subsystems

| subsystem | game | port today | needed by |
|---|---|---|---|
| jump system (jump block 0x13f720.., vertical 0x2345f0 incl. the scripted curves 0x13f76c, air control 0x234b40, landing picker, descent anim) | shared case | jump.rs for 7/9/0xb/0xe/0x12; curve table 0x17c3e0 only | P3 (0x11, 0x1c), P4 (10, 0xd, 0xf, 0x10), P5 (0x29, 0x2a entries), P1 (0x69), P2 (0x3c) |
| ledge / wall probes | 0x22c9a0 A, 0x22d090 B, 0x22d838 C (L00 0x20c758); 0x13f504, 0x13f838, 0x13f820/834/848 | ported (P3, `ledge.rs`; `Hero::ledge_blk`) | P3, P4 (glide), P1 (carry of 0x13f848) |
| surface classification | ground probe → 0x140630; reaction 0x22cd48 / L00 0x20b960 → flags 0x140632..0x14063e | ported (P1): every flag, the per-level rules `surface::LEVEL_RULES`, the probe's 3 / 0xb / 1 branches | P1 (rules), P2 (hazard deaths), P5 (surface 2) |
| slopes / sliding | 50° walkable limit, capsule slide, edge brake 0x236a68, steep-wall stop 0x232820, slope ratio 0x13f4bc, pitch/roll 0x13f634/638 | ported; the slide 0x2f and its slippery variants of the ground code (P1) | P1 (slippery 0x2f), P5 (gravity mode 1 frame) |
| platform carry | `HeroPlatformUpdate` 0x249618 + `triggers::platform_delta` / `carry_point`, 0x13f440 / 0x13f44c, 0x13f6b0/6b4 | ported (P1, `platform::platform_update`; carriers via `Env::world` = `platform::Carriers`) | P1; the lift 726 / elevators 703 / 715; P3 (hang on a moving ledge) |
| moby → hero writes | the class updates' stores into the hero block (0x13f440, 0x13f4a0, 0x13f530, 0x13fd20.., 0x13f542 / 544, …) | ported: `services::HeroFields`, applied by the tick after the moby loop (follow-ups) | 679, 726; 613, Swingshot targets, NPCs when ported |
| damage / knockback / invulnerability | hit intake 0x231580 (L00 0x210ce8), hit records 0x178580 (L00 0x178110), knockback `0x231518` (L00 `0x210c80`) **added to 0x13f430 and 0x13f450** (not 0x13f680: that is not a knockback), `HeroTakeDamage` 0x226fa8 (min(n, 1)), 0x13f510, 0x13f53e | ported (P2, `damage.rs`; the tick hands Ratchet's hit message over, `MobySystem::hit_message`) | every hazard state |
| death / respawn | 0x2319b0 (deaths++, fade, 0x141401); main loop: death reload `LoadLevelCoreData(0, 1)`, hero at the checkpoint record 0x1bb6b0 (`0x29adc8`) | ported (P2): `damage::death_fade` → `Hero::fell_out`; the engine respawns on the flag only, at the checkpoint (class 805 `0x29ac10`) | — |
| item ownership | `0x13d4c0 + id` | done (P4): `Hero::owned` (`hero::Owned`, `has(id)` / `set(id, b)`, the whole 37-byte table synced from `GameState::global.owned` by gameplay.rs every tick; tests: `Hero::grant_items(&[ids])`; debug `RC_GIVE_ITEMS=<ids>`); swim reads it | P5, P6 read `h.owned.has(id)` |
| back slot (pack module) | item slot 3, `GetClankModule(3)`, back anim table 0x2476d0 (rows 2 / 3 / 4) | done (P4): `Hero::back_slot` (`idle::BackSlot` = `idle::ItemSlot` + 0x15ed94 / 0x141628, synced with `equipped[3]` / `temp_back` / `thruster_last` / `clank_hidden`), `Hero::back_module()`; creation (target / saved / 2 or 3), `UpdateWrenchSelected(3)`'s back rules (Hydro-Pack in the water groups, restore after, Heli-Pack request in 8) + the common request / restore swap `ItemSlot::swap_requests`, put-away and re-creation; pack classes by item definition (607 / 608 / 609, `set_back_packs`), `Back::pack_o_class` drawn by moby_attach; 607's update `ClankPackUpdate` (rotor seq 6 in 8). Without classes only the bookkeeping (no swaps) | P5 (feet slot: reuse `ItemSlot`) |
| spline follower | L00 0x25d7a0 step, 0x25d808 advance (doc name `SplineProject`), 0x25df68 nearest point (doc name `SplineSample`), 0x25da70 / 0x25dcd8 / 0x25db00; grind paths section 0x74 (`rc_formats::volumes::GrindPath`, flag = closed), paths 0x70 | ported (P5): `crate::spline` (`step_index`, `advance`, `nearest`, `band_distance`, `closest_on_segment`, `heading`, `with_lengths`); used by `hero::boots` and the flow class 679 | P5 (grind, cable), moby classes |
| targets | Swingshot targets 0x13fcb4 / 0x13fce0 (+0x78 pvar record), target search in the weapon check | ported (P6): classes 758 / 803 (`classes::swing_target`, level03 0x2d0bd8), `swingshot::Targets` of the run list (`MobySystem::run_list`, the cuboids by `MobySystem::volumes`) through `HeroWorld::swing_targets`; the searches 0x222158 / 0x221ee0 | — |
| gravity frame | gravity mode 0x141403 (0x248ad8), gravity dir 0x13f5e0, `SetPlanarVel` / TurnTo in mode 1 | ported (P5): the game's rule (`boots::gravity_mode`); mode 1: gravity along −normal, the probe along 0x13f5e0 = −normal, TurnTo in the hero's frame (0x2323d8), SetPlanarVel along the tilted facing, the step snap 0x233588, the frame alignment 0x236358 (also with 0x13f548); mode 2 (level 16's 0x3e) not ported | P5 |
| timers | FastDecTimer list of 0x23c710 | all counters now in `Hero` and counted down (§5) | all |
| hero sounds | class sounds of the anim triggers inside 0x247d48; voices `0x236738` / `0x236810`; sound slots 0x141568.. (`0x236798`, released by 0x2283a8 on a group change) | ported (P2): `audio::class_sounds::HeroClassSounds` inside the hero update (`Game::tick_with_hero_sounds`); the surface sounds (follow-ups, `surface::flush`); the queued voices 0x236810 / 0x236860, the grind / cable loops, the item sounds (hero polish, `hero::fx`) | — |

## 3. Packages (the "finish Ratchet" push)

Rules for every package: own only your files; the shared files (`hero.rs`, `registry.rs`, `states.rs`,
`common.rs`, `physics.rs`, `ground.rs`, `walk.rs`, `air.rs`, `jump.rs`, `melee.rs`, `swim.rs`, `idle.rs`,
`items.rs`, `anim.rs`) are frozen except (a) setting `ported: true` on your rows in `registry.rs` (one-line edits),
(b) the owner of `jump.rs` named below, (c) a new seam agreed with the orchestrator (one call into your module at
the game's branch point, listed in your report). Behaviour of the ported states must not change (run the
`novalis_hero_digest` guard, §5). Native floats for new code (`Pf::f` / `to_f32` at the boundary, as swim.rs does).

| # | package | files owned | states | depends on | wave |
|---|---|---|---|---|---|
| P1 | platform carry + surfaces / slopes / sliding | `platform.rs`, `surface.rs` | 0x2f, 0x31, 0x68, 0x69, 0x7b | the moby table in `Env` (below) | 1 |
| P2 | damage, knockback, death + stances + hero sounds | `damage.rs`, `stance.rs` | 0x16, 0x3c, 0x3d, 0x75, 0x76, 0x77, 0x79, 0x7c, 0x7f, 0x80, 0x82; 1, 0x1e, 0x40, 0x41, 0x65..0x67 | hit records from moby_update (`HitSink` side), engine respawn contract | 1 |
| P3 | ledges + wall jump | `ledge.rs` (+ `jump.rs` during wave 1) | 0x11, 0x18..0x1c | — (P1's carry for moving ledges, later) | 1 |
| P4 | Heli-Pack + Thruster-Pack | `packs.rs` (+ `jump.rs` during wave 2) | 8, 10, 0xd, 0xf, 0x10, 0x22, 0x7a, 0x81 | item ownership mirror, back slot module id + pack models | 2 |
| P5 | Magneboots + Grind Boots + cable | `boots.rs` | 0x28..0x2b, 0x3f, 0x42, 0x70, 0x71, 0x74 | grind paths + spline follower, the gravity-mode rule (physics.rs: P5 may edit the gravity-mode paths), level data in `Env` | 2 |
| P6 | Swingshot (+ the other hand items' weapon check) | `swingshot.rs`, `gadgets.rs` | 0x24..0x26, 0x2c, 0x2d | target mobys (moby_update classes), level data in `Env` | 2 |

Order: wave 1 = P1, P2, P3 in parallel (P1 unblocks the lift and every carrier; P2 unblocks every hazard and the
engine's death handling; P3 is self-contained and Novalis has ledges). Wave 2 = P4, P5, P6 in parallel.

**`Env` extension (one shared edit, first thing in wave 1, by P1):** the hero needs level / moby data it cannot
reach today (carriers' platform blocks, later grind paths and swing targets). Add one field to `hero::physics::Env`
(e.g. `world: Option<&'a dyn HeroWorld>`, a trait in `platform.rs` with default methods P5 / P6 extend) and set it
in `tick.rs` (`Game::tick_with_sound`) and the hero test constructors. `MobyScene` only exposes collision.

### P1 — platform carry, surfaces, slopes and sliding
**Done (2026-09-26).** `platform.rs` (the whole of 0x249618, general: any moby with a platform block, snapshot
`platform::Carriers` in `Env::world`), `surface.rs` (the superset reaction switched per level by `LEVEL_RULES`,
the states 0x2f / 0x31 / 0x68 / 0x69 / 0x7b, the seams in the ground / walk / idle / melee / jump / probe / capsule code),
tests `platform::tests`, `surface::tests`, `tests/hero/hero_platform_novalis.rs` (the lift carries Ratchet down, the
steep bank at its foot, determinism), `tests/hero/hero_surfaces.rs` (surface ids per level vs rules, Aridia quicksand,
ice on levels 12 / 14, Novalis' flow chutes). Open: ~~the flow class 679~~ (done, "Hero follow-ups" below), ~~the
landing picker's leading slippery test (jump.rs)~~ (ported: `hero/jump.rs::landing_picker`), the Thruster hover's slippery brake (packs.rs), the sinking
~~floor's sand particles (`0x22b140(4, 2)`, the 6-draw `0x286cb0` spawn: still not ported, so the `rand` stream
diverges from the PS2 while sinking)~~ (done: `hero/fx.rs` PartType47Spawn, surface.rs), ~~the surface sounds~~ (done below).

### P2 — damage, knockback, death, stances, hero sounds
**Done (2026-09-26).** `damage.rs` (the hit intake 0x231580, the knockback 0x231518 added to 0x13f430 and 0x13f450,
`HeroTakeDamage` 0x226fa8, the hurts 0x16 / 0x75 / 0x76, the deaths 0x3d / 0x77 / 0x79 / 0x7c / 0x7f / 0x80 / 0x82, the
burn bounce 0x3c, the death sequence 0x2319b0 → `Hero::fell_out`; the engine respawns on that flag at the checkpoint),
`stance.rs` (1, 0x1e, 0x40 / 0x41, 0x65..0x67), the hero sounds inside the hero update (`HeroSounds`,
`audio::class_sounds::HeroClassSounds`, `Game::tick_with_hero_sounds`). Tests `tests/hero/hero_damage.rs`. Open (see the
module docs): the hurts' and deaths' particles and 0x7f's bubble moby, the queued voices 0x236810, 0x1413f5.
The original plan:
- The hit intake 0x231580 (seam `damage::hit_intake`, first call of the transitions): hit record, invulnerability
  0x13f510, knockback 0x210c80 → `Hero::knock`, `HeroTakeDamage`, the hurt state per group; the hit flash 0x13f53e.
- States in the table (0x16, 0x75, 0x76, the deaths, 0x77 incl. its two RNG draws at entry, 0x79, 0x3c via the jump
  system) and the death fade 0x2319b0 → `fell_out` / 0x141401; agree with the engine owner to respawn on
  0x141401 instead of on entering 0x77 / 0x3d.
- Stances 1 / 0x1e (look), 0x40 / 0x41 (fidget state), 0x65..0x67 (walk to point).
- **Move the animation-trigger sound draws into the hero update:** `hero::hero_update_with_sounds` calls
  `HeroSounds::anim_advanced(moby, before, after, rng)` right after Ratchet's advance 0x247d48 (the game's point,
  before the back items' 0x247800 draw). Implement `HeroSounds` for the audio layer (`audio::class_sounds::
  ratchet_trigger` + `AudioSystem::play_class_sound`), call `hero_update_with_sounds` from `Game::tick_with_sound`
  (tick.rs) and drop the deferred replay of `TriggerAnim` in `sound_step`. The draw count is unchanged; only their
  position in the tick moves (the RNG stream after the particles changes: re-baseline traces). Coordinate with the
  audio / tick owner.
- Verify: Novalis enemies' contact hits (needs their moby updates' hit records), a savestate taking one hit
  (0x1413d4, 0x13f680..688, 0x1415f8, 0x13f510 per tick), drowning (swim), the death fade.

### P3 — ledges and the wall jump
**Done (2026-09-26).** `ledge.rs` (probes A 0x22c9a0, B 0x22d090, C 0x22d838; 0x18..0x1c, the wall jump 0x11;
`Hero::ledge_blk`, `Hero::ledge_camera_yaw` for the camera), the 0x11 / 0x1c jump parameters in `jump.rs`. Tests
`ledge::tests`, `tests/hero/hero_ledge_novalis.rs` (grab and climb on Novalis, the digest scripts enter no ledge state).
Moby ledges (the pvar flag) and the hang on a moving ledge came with the follow-ups. The original plan:
- Probes A / B / C and the states; 0x11 / 0x1c through the jump system (`jump_block_defaults`-style entry +
  `phys_jump` / `tr_jump`); seams exist: `wall_ledge_probe_a` (jump physics), `wall_ledge_probe_b`
  (`hero_update`), `wall_jump` (jump transitions), `Hero::f838` → 0x18 (fall and jumps).
- P3 owns `jump.rs` in wave 1 (for 0x11 / 0x1c branches). Entries: `Hero::jump_block_defaults` (the shared part
  of the jump entry) + the id's parameters and anim.
- Verify: Novalis ledges (the drop off the spawn plateau): script jumping at a ledge edge; savestate hanging;
  unit tests on a hand-built step (testkit::cell).

### P4 — Heli-Pack + Thruster-Pack
**Done (2026-09-26).** `packs.rs` (8, 10, 0xd, 0xf, 0x10, 0x22, 0x7a, 0x81 and the pack tests of crouch / fall / jumps /
the weapon check's end), `jump.rs` (the four pack jumps' parameters and anims, the curve tables 0x17c3a0 / gp−0x7518,
the Heli long jump's windup push, the double jump's pack boost, the long-jump glide hold-off, the level00 landing
picker's slippery test), the owned-items mirror and the back slot (§2), the wall-ahead probe 0x13f598 / 0x13f5a0 /
0x13f5a4 / 0x13f5a5 of `0x23c458` (physics.rs), the hero's looping-sound slots 0x141568 (`packs::flush_sounds`,
`HeroSounds::release`), pack hits after the hero update (`packs::deliver_hits`, `HitSink::deliver`). Tests
`packs::tests` (12), `tests/hero/hero_packs_novalis.rs` (glide, both long jumps, stomp, the Hydro-Pack on the back in the
lake, determinism). Open: ~~the stomp's camera shake~~ (done, Hero polish), ~~the Thruster jumps'
after-images `0x277428`~~ and ~~the Thruster flame mobys (class 0xa7)~~ (done 2026-09-28: "Thruster flames",
"After-images" below), the pad vibration `0x248920`, ~~the ledge climb's Thruster voice (0x1c)~~ (done), the
Hydro-Pack's bubble jets; in the port the back swap's `rand_range(50, 90)` draws after
the hero update and before the hand's slot loop (the game: slot 0 first).

### P5 — Magneboots, Grind Boots, cable
**Done (2026-09-26).** `boots.rs` (0x28..0x2b, 0x42, 0x3f, 0x70, 0x71, 0x74; L00 SetState 0x2223f8, physics
0x217970, transitions 0x229b70), `crate::spline` (the general follower, §2), the rail / cable contacts
`0x20cf58` / `0x20d330` and the magnetic floor 0x13f658 (`boots::contacts`, after the wall probe in the hero update,
as L00's 0x2070d0), the gravity-mode rule and the mode-1 frame (§2; physics.rs), the grind's 0.45 sphere instead of
the capsule (L00 0x2133a8). Findings: the Grind Boots **are** tested (0x13d4dd) by the rail contact; the rail
contact only exists on levels 0, 4, 6, 7, 8, 9, 10, 13, 14, 16, 17, 18 and on 6 / 0xe / 0x10 only near fixed mount
points (the rail starts); the cable contact on 0, 3, 4, 7, 9, 10, 13, 17 (Kerwan's three 2-point grind paths are
cables); the rails' own collision is a pit surface (0xc) just under the spline; 0x13f658 needs the feet item
moby (0x140430, item slot **1**) to be the Magneboots (class 0xad; since 2026-09-28 the port tests the feet slot,
`Hero::magneboots_on`, which the automatic swap fills: docs/plan/gadgets.md); SetState(2) with
0x13f658 = 1 becomes 0x3f (walk.rs); idle and stop take 0x28 on the rail contact (ground.rs). The grind hit
0x13f90e is `damage::Damage::grind_hit`. Tests: `boots::tests` (11), `spline::tests` (5),
`tests/hero/hero_boots_grind.rs` (every level's 37 grind paths; Oltanis 14: grind, grind jump, rail switch 2 → 3;
Kalebo III 16: 20 s on the long rail 1 with the obstacles' grind hurts, deterministic), `tests/hero/hero_boots_magnet.rs`
(Orxon 10: idle in mode 1, 0x3f, no boots = mode 0). Engine: `RC_GIVE_ITEMS=29 RC_HERO_AT=x,y,z,yaw` (new debug
placement in gameplay.rs). Open: the grind / cable loop sounds (slot 0x141568, now general: `HeroSounds::release`),
the type-25 sparks as particles (`Boots::sparks` records them with their draws), the grind wrench's hit sphere
(`Boots::hits`, queued: needs a sink like the jump attack's shockwave), the moby-armed targeted grind jump
(0x13f908 / 0x13f90c / 0x13f8a0 / 0x13f918; through the moby → hero channel when its class is ported), level 16's
class-0x101 bump, the camera look-ahead and body-lean joint records, AirAccel / StickTarget / LandEta in mode 1,
the feet item slot (0x140430) itself, melee row 6 in `melee::COMBO` (the wrench update's hit window of the second
0x70 swing uses row 5's).

### Cable slide 0x74 (the zipline, 2026-09-28)
Ratchet hangs from a cable by the wrench and slides down it. **Identity** (no longer **L**): Kerwan's level-script
moby update `0x2df520` (level03) counts "state 0x74" as a stat (0x1418c8) and shows help message 0xbbf in a volume to
a hero swinging the wrench (group 6, 0x140408 = 8) who has not used a cable yet. **The line**: a grind path
(gameplay section 0x74), the same data and follower as the rails (`crate::spline`, `rc_formats::volumes::GrindPath`);
no moby class. The code (the contact `0x20d330`, hand point `0x20d2f8`, SetState / physics / transitions cases)
exists in the overlays of levels 0, 3, 4, 7, 9, 10, 13, 17 (identity: `0x233660(0.3, 0, 1.34)` + the −0.7 / −1.2 /
0.4 test: L00 0x20d330, L03 0x205a68, L04 0x200910, L07 0x235198, L09 0x22d010, L10 0x200998, L13 0x218e18, L17
0x20a288); of those only **Kerwan (3)** has grind paths: **3 cables** (2-point, 58.9 / 100.1 / 56.6 long, drops
26.7 / 21.6 / 19.0; a post at each start, the platform 2.2..2.9 below). Every level's instances go through the same
code: no per-level code.
- **Attach** (`0x20d330`, after the wall probe): 0x13f534 = 0 (the lockout), group 4 with 0x13f76e = 0 (**rising**)
  or group 2 (fall); the first path whose bounding sphere holds the hand point and whose nearest point
  (`0x25df68(12, 10, 0)`) is < 0.9 horizontally (0.3 in groups 0 / 1) and < 1.5 vertically (10 and +0.5 with
  0x13f51a); hands −0.7 .. 0.4 above it, −1.2 when □ is **pressed** this tick (0x13cae4 & 0x80). Then the
  transitions: the jump group's line (level00 0x229b70 case 7.., after the Thruster stomp's R1 test, before the forced
  fall: `0x28` on the rail contact, else `0x74`; Kerwan 0x21c668 has only the cable half) and the fall's (after the
  ledge 0x18 and the rail 0x28). The weapon check (level00 0x227fa0, Kerwan 0x21aac8) skips the jump attack 0x14 while
  the cable contact is set: □ at the cable is the grab. **The port had only the fall's line**, so a jump never caught
  a cable (a jump stays in 7 until it lands) and □ started the jump attack: fixed by `boots::jump_contacts` (called
  from `jump.rs`) and the weapon-check line (`melee.rs`). The same jump line also takes a descending jump onto a
  grind rail (0x28) on the rail levels.
- **Entry** (SetState 0x2223f8 case 0x74): group 0x1a, camera mode 0x1415d4 = 3, 0x1413f7 = 1 (the wrench is forced
  into the hand: `items::update_hand_selected` swaps any other item for it), blink period 0x68, 0x13f950 / 0x13f968 = 0;
  speed 0x13f954 = max(0, ½ · displacement · (direction to the point 1 ahead on the cable)); anim 0x73 (the grab)
  blend 8.
- **Physics** (0x217970 case 0x74): the hand point projected on the cable (`0x25df68(999, 8, 2.5)`), advanced by the
  speed; yaw / pitch of the cable ahead; TurnTo(0.018, 0.2, 6.458·dt); speed → 14 u/s at 9·dt²; vel = the step; the
  hands pulled onto the cable by a spring (0x25b8c0(0, 0.05, 0.3, 10·dt), its length only shrinking). At the last
  point the ticks-off counter 0x13f950 starts: he flies on with his momentum (vel.z − 24·dt² a tick). Anim 0x66
  (the slide): the loop class sound 0 (0x216e48) and, every other tick, a type-25 spark at the hand point (the lq/sq
  copy at 0x21dc38 Ghidra drops: the spark starts at the hands) with 0.4·displacement + a random 0..4·dt push within
  ±30° of the cable yaw, up 3..5·dt, gravity 35·dt².
- **Transitions** (0x229b70 case 0x74): anim 0x73 passing frame 11 → voice 0xd + sparkle burst `0x2a7e20(hands, 4)`;
  0x73's end → 0x66 (blend 0x1e); more than 5 ticks off the end → fall 6 with 0x13f534 = 8. **No jump-off**: the
  case reads no pad; the prologue's checks (hit intake, the weapon check: group 0x1a is neither on foot nor in the
  air list) do not leave it either.
- **Camera**: no hero-state tweak (0x3111d8 has none for 0x1a / 3). Kerwan's camera records 10..12 (class 17, mode
  +0x2c = 2 "while in group 0x1a", priority +0x1c = 4; +0x2e is the region's counter) retune the follow camera for the
  whole slide: distance 6.5, pivot 2.0, look −0.5, the stick off, a 1° / 4° turn (ported 2026-09-30,
  player_controller.md §15; `tests/world/camera_levels.rs`).
- **Not ported / left**: the help message 0xbbf and the 0x1418c8 stat (Kerwan's script moby,
  unported); the hit intake while hanging uses the ported generic path.
- **Tests**: `boots::tests` `a_rising_jump_catches_the_cable`, `square_is_the_wrench_grab_not_the_jump_attack`,
  `ride_to_the_end_no_jump_off`, `cable_slide` (distilled Kerwan numbers); `tests/hero/hero_cable_kerwan.rs` (Kerwan's 3
  cables on the level mesh: ✕ from the platform → 7 → 0x74 → 6 → 0, anims 0x73 → 0x66, 14 u/s, off ≤ 1.1 past the
  end, deterministic). Engine: `RC_LEVEL=3 RC_HERO_AT=197.137,148.537,76.03,2.912 RC_PLAY_SCRIPT="30:press X"`.

### P6 — Swingshot and the other hand items
**Done (2026-09-26).** `gadgets.rs` (the hand-item mechanism, one table: `HAND_ITEMS` rows = the weapon check's case
per item (`HandItemKind::fire`) and the item moby's update (`ItemUpdate::Slot` in the slot loop, `ItemUpdate::Hero`
right after it with the hero's context: `gadgets::after_items`, called by the tick); the request 0x141408 →
`UpdateWrenchSelected` → held 0x140408 path is items.rs's; the epilogue follows level00 0x227fa0: every switch case
ends in it, the early returns before the switch skip it), `swingshot.rs` (0x24..0x26, 0x2c, 0x2d; the searches; the
hand item's update level01 0x2dbdc0 with its hook and rope), `moby_update/classes/swing_target.rs` (758 / 803). Levels:
pull targets on 2, 3, 4, 5, 7, 10..18; swing targets on 2..7, 9..11, 13..16, 18 (none on 0, 1, 8). Engine: the
Swingshot (0xd0) and its hook (0xd1) drawn by moby_attach, the rope `0x2dba30` as a camera-facing textured strip
(effect texture 0xf); `RC_GIVE_ITEMS=12` also requests it into the hand. Tests: `swingshot::tests` (10),
`classes::swing_target::tests` (2), `tests/hero/hero_swingshot_levels.rs` (Aridia: a pull and a swing; Kerwan: a swing;
deterministic). Open: ~~the target glints (particle type 60: its draw is made, the particle not created)~~ (done: `classes/swing_target.rs`), the targets'
camera look-at hint `0x2eb4c0`, the look-stance aiming beams `0x20fb60`, the Swingshot's class sounds (queued in
`SwingItem::sounds`, not played: their pitch-bend draws are missing from the stream, as the wrench's hit sounds), the
targeted swing from a grind rail (0x13f904), the stats / help counters. The other hand items' rows (weapons, the
Hologuise, the PDA) and the holster check 0x2405f8 are not ported. The original plan:
- `gadgets::pda_item` (the non-wrench cases of 0x240ed8, must call `packs::pda_epilogue` when the state did not
  change), the target search, 0x24..0x26 (pull), 0x2c / 0x2d (swing), the straightening exclusions.
- Needs the target mobys' updates (moby_update classes) and their pvar records via `Env::world`.
- Verify: a Kerwan / Eudora savestate at a target; unit test pulling to a fixed target and swinging on a fixed
  pivot (rope length kept, release into 0x2d).

### Hero follow-ups (the gaps P1 / P2 / P3 left)
**Done (2026-09-26).**
- **Moby → hero write channel** (`moby_update::services::HeroFields`): the hero-block fields the game's class
  updates store into, read and written by a class through `World::hero_fields` / `World::hero_fields_mut` (the
  block as this tick's earlier classes left it) and applied by the tick right after the moby loop, before the hero
  update (`tick.rs`, `MobySystem::take_hero_writes`; nothing reads the block between the two in the game). Fields:
  0x13f440..0x13f44c push, 0x13f4a0 momentum, 0x13f530 the sinking floor's hold, 0x13fd20..0x13fd30 the flow, 0x13f542
  / 0x13f544 the lift's lockouts (726 now writes them through it). The other class stores found in level01 (613's
  0x13f4e4 / 0x13f528 / 0x141608, the Swingshot targets 0x13f904 / 0x13fcd8 / 0x13fcec, 0x300de0's 0x13f510, the NPCs'
  0x13f3d0, …) are listed on the struct for their classes to add.
- **Flow class 679** `0x2f6328` (`moby_update/classes/flow.rs`; the same function on levels 5 0x3067b8, 8 0x2f74c0,
  15 0x2d95e8): pushes the sinking hero along its splines (the shared follower `crate::spline`) with the pull back to
  the spline, holds 0x13f530 = 30, level 1's speed by the length left; off the end the momentum ×0.99. Novalis:
  instance 691 (splines 43, 44) carries Ratchet down the chute to the landing plateau; 692 has no spline.
- **Camera while hanging**: `follow_camera.rs` takes `Hero::ledge_camera_yaw` in the type-0 tweaks (0x3111d8, right
  after the platform carry): yaw rate 12°/tick, the scripted yaw input, look flag; also the glide (group 5) and
  sinking-floor (group 0x10) look / pivot heights of the same function.
- **Moby ledges**: `HeroWorld::moby_ledge_flag` (the pvar record's +0x1e bit 0, `triggers::record_ledge_flag`) in
  probe B's moby test. The bit is set (record +0x1e = 1, 5 or 0x41) on carriers of levels 3 (classes 868, 905, 928),
7 (1069, 1104, 1129), 12 (240) and 13 (104, 106); none on levels 1, 5, 8, 15.
- **Surface sounds**: `surface::flush` plays `Surf::events` through `HeroSounds::voice` / `release` right after the
  physics, the reaction and the transitions (the loop's voice becomes 0x141570; a group change releases it, the slot-2
  part of 0x2283a8).
- **`HeroOnMoby`** 0x277fb8's ledge branch (group 3 / state 0x1c: the ledge moby 0x13f848).
- Tests: `tests/hero/hero_followups.rs`, `classes::flow::tests`, `surface::tests::sinking_floor_state`.

### Hero polish (the audiovisual feedback the packages left out)
**Done (2026-09-26).**
- **Camera shake, one mechanism** (`follow_camera::Shake`, `ShakeRequest`, `Camera::request_shake`): the records
  0x167260 (along the camera's up row) and 0x167270 (along forward) that `CameraUpdate` 0x20eca8 applies after the
  Euler through `0x20e560`: `max = max(max, t)`, `FastDecTimer(t)`, `offset = amp·cos(NormalizeAngle(2t))·(t/max)²`,
  `0x167240 += setlen(row, offset)`. Requests: hero code `hero::fx::shake` (queued in `Hero::fx.shakes`, applied by
  the tick after the hero update), classes `World::shake_camera` (`Services::camera_shakes`, applied after the moby
  loop). The Thruster stomp uses it (0.2 up for 40 ticks, 0x2390a0); the collapsing platform 701 (0x2f93a0 /
  0x2f949c: 0.4 / 30, 0.1 / 20) and the explosion code (0x273310, 0x273f50, 0x2bfe40, 0x2c3300, 0x2c5b70, 0x304798)
  will call the same when ported.
- **Hero effects channel** `hero/fx.rs`: particle spawns with their draws at the game's point, created by the
  particle hook before `UpdateParts` (docs/plan/particles.md "the hero's particles"); the delayed-voice queue 0x141528
  (`0x236810` / `0x236860`, after the transitions) and the swim's recorded voices; the hand item's class sounds
  (`gadgets::flush_item_sounds`, `HeroSounds::item_sound`).
- **Sounds**: grind / cable loop (slot 0x141568, class sound 0; released by 0x29 / 0x2a / 0x42 and a group change), the
  ledge climb's voice (0x1c, `0x236738(4, 0)` at frame 10 with the Thruster-Pack as the back item 0x1404f8 = 3 and
  Clank shown: the game's only climb voice), the cable grab's voice 0xd + sparkle burst `0x2a7e20` (frame 11 of 0x73),
  the Swingshot's fire / hit / pull and the wrench's hit (gadget class defs: docs/plan/audio.md), the surfacing gasps.
- **Particles**: grind / cable sparks (type 25), swing target glints (type 60), the sinking floor's sand puff (type 47),
  the hurt-under-water bubbles of 0x76 (type 34, `0x22b140(min(10 − t/3, 8), 0)`).
- **Hits**: the grind wrench's sphere (0x259888(1.0, 0x10000) + `coll_sphere_mobys(1.0, hip, 0x10)`) goes with the
  pack hits through the hit sink after the hero update; `melee::COMBO` has row 6 (the second 0x70 swing, level01
  0x17c0a8 + 6·0x2c = `[0, 1, 25, 7, 17, 18, 23, 8, 13, 6, 12]`), shared with `hero::boots`.
- **Not done**: ~~the Thruster flames (class 0xa7 mobys `0x2c9da0`)~~ and ~~the Thruster jumps' after-images
  `0x277428`~~ (done 2026-09-28: "Thruster flames", "After-images" below), the pad vibration `0x248920`, the burn fire `0x209ec8` (type 4
  unported), the crouch slide's
  voice 0xc and 0x77's voice 0x17, the wrench hit sound's index 1 (`FUN_002bda88` reads a pointer record from the hit
  moby's pvars).
- Tests: `follow_camera::tests::shake_envelope`, `hero::fx::tests` (3), `particles::type25::tests` (2), `type34`,
  `type47`, `type60`, `audio::tests::gadget_class_sounds_on_every_level`; `packs::tests` (stomp request), `boots::tests`
  (sparks queued, the grind wrench's hit). The `novalis_hero_digest` guard is byte-identical.

### Swim effects (the hero's water effects)
**Done (2026-09-28).** The user's report: no splash jumping in, getting out or walking in ankle-deep water. Root cause:
the particle types and the splash moby existed, but no hero code called them (the swim code recorded
`SwimEvent::Splash`, which nothing consumed; the wake, rings, spray, joint bubbles and breath had no port). Now every
call site of the water helpers in the hero code (level01, disassembly; helpers and their draws: particles.md "The hero's
water effects") runs at its point, in its order, with its draws:

| where (level01) | condition | call | port |
|---|---|---|---|
| SetState 0x37, 0x23d654 | from under water (group 0x11) | `0x22b3a8(3, 10, 0)`, ripple (0.4, 0.3) | `swim_entry` |
| SetState 0x37, 0x23d6f0 | from above, disp.z < −0.5·dt | `0x22b3a8(3, min(trunc(300·|dz|), 40), 1)`, ripple (0.5, −0.4), countdown 0x13fc5a = 75 | `swim_entry` |
| water entry 0x2408e8 | after SetState(0x37) | voice 3 | `water_entry_check` (at the call point) |
| SetState 0x33 / 0x35 | from the surface | voice 3 (before the SetAnim) | `swim_entry` |
| jump entry 0x23e890 | state ≠ 0x12, in water 0x140634, z < W + 0.5 | `0x22b3a8(3, 16, 0)` | `jump_block_defaults` |
| jump transitions 0x24621c | past the take-off / curve window, 0x13f648 == 1 (first tick under the level) | voice 0x11, `0x22b3a8(3, 24, 1)` | `tr_jump` |
| physics 0x12, 0x23b5bc / 0x23b644 | tick 20 / frame in [4, 7] | `0x22b3a8(3, 16, 1)` + ripple (0.4, 0.35) / `0x22b140(7, 0)` | `effects::water_jump` |
| physics 0x33..0x35, 0x23790c.. | 0x35 / else | loop 0x236798(5, 0x13) / release slot 5 | `phys_underwater` |
| | countdown 0x13fc5a | FastDecTimer, `0x22b140(min(t/4, 8), 0)` | |
| | 0x33, not blending, frame in (0, 10) | `0x22b140(4, 1)` | |
| | 0x34 after 0x35, timer < 15 | `0x22b140((20 − t)/3 + 1, 1)` | |
| | dive-in tick 1 | `0x22b3a8(3, 16, 0)`, ripple (0.4, −0.3), countdown 70 | |
| | after the target speed | breath 0x13fc40 (`randi(100) < 40` → 4..11 else 40..90 ticks) | `effects::breath_underwater` |
| | 0x35 | jets 0x13fc48 | `effects::jets` |
| physics 0x36 / 0x37, 0x238a24.. | 0x37 / |eff.xy| > 1.5·dt / 0x36 and > dt | `0x22ac40(15, 30)` / `0x22af48(4, 12)` / `0x22ad38(0, 1)` | `phys_surface` |
| | countdown | t − 1, `0x22b140(min(t/8, 6), 0)` | |
| | 0x36, not blending, frame in (0, 10), after the velocity | `0x22b140(4, 1)` | |
| physics 0x6a / 0x82, 0x238934 | 0x13fc40 runs out | breath; next in 2..5 + t/5 (t < 40) or 10..20 (t > 70) | `effects::breath_drowning` |
| physics 0x75, 0x2387bc | always | `0x22ac40(15, 30)` | `damage::physics` |
| physics 0x76 | always | `0x22b140(min(10 − t/3, 8), 0)` | (ported before) |
| physics ground cases, 0x23714c | wading flag 0x1413f9 | `0x22ac40(15, 30)` | `effects::physics_prologue` |
| physics 2 / 0x73, 0x23a2e4 | wading, or in water under 0.25 deep | |eff.xy| > dt: `0x22ad38(0, 2)`; > 0.5·dt: `0x22af48(2, 4)`; `0x22b140(5, 2)` | `effects::physics_prologue` |
| physics 0x31 (sinking floor) | level ≠ 15 | `0x22b140(4, 2)` before the sand | `surface::sinking_floor` |

**Sounds.** Voices from SetState and the transitions play at their call points (`states::Ctx::voice`, which the hero
update fills with `HeroSounds::voice`), so a voice's own draws come before the splash draws after it
(`SwimEvent::Played` records them). Water footsteps: the ground probe's footstep class 3 on levels 1 / 0x12 (ported
before) plays through the walk keys (level def 16 when wading on Novalis: engine trace); the wade sequence 0x60 has no
keyed footsteps in `FUN_00227e90` (as in the game).

**General pieces.** `hero::fx::joint_point` (any of Ratchet's joint lists, his pose now and his moby as the last
write-back left it; `joint_world_point` is `World::joint_point`'s formula), `hero::fx::pack_point` (the back pack's
lists, placed by `fx::end` = `HeroItemsAttach`'s list-5 attach), `hero::fx::reserve` (a queued spawner's draws made at
the call), `MobySpawn` + `fx::create_mobys` (a moby the hero code creates, made by the tick after the hero update),
`type45::HERO_WATER_LEVEL` (the level pointer of rings 45 / 46). The engine hands Ratchet's and the packs' joint lists
over (`Hero::set_joint_chains`, `set_pack_joint_lists`).

**Guard.** `novalis_hero_digest`: the `moves` run is byte-identical; the two lake runs are identical up to tick 519 and
differ from tick 520 on, the first tick Ratchet walks into the lake's ankle-deep edge (depth 0.12: the walk case's
bubbles draw from the stream). The new fields are filtered while 0 / empty.

### Thruster flames (class 0xa7, 2026-09-28)
The user's report (Gaspar, the Thruster in use): no exhaust. Read from the decompiler C and the disassembly (level01;
the class is in every level's class list without a model, its table entry a copy of the same code; the callback's
tables are found on all 19 levels through the relocation):
* **Creation**: `HeroItemsCreate` 0x22f3c0 creates two flames (`FUN_002c9da0(0)`, `(1)`: pvars cleared with the side,
  +0x30 0xff, draw distance 0x7e) whenever it creates the back pack for back item 3 with Clank shown. Port:
  `packs::flames_on_create` → `fx::MobySpawn::ThrusterFlame` → the tick's `fx::create_mobys`.
* **Update** `0x2c9e00` (moby loop; `rc_game::moby_update::classes::thruster_flame`): "on" in 0xb..0xe or the ledge
  climb 0x1c at key times 6..16 (both only while not descending, 0x13f76e), 0x10 before tick 44, 0x22 before tick 15 or
  airborne after tick 33, 8 and 0x81; states 0 (hide, z + 0.5) → 1 (wait; deleted when `GetClankModule(3)` ≠ 3) → 2
  (sizes grow over `ticks(2)`) → 3 (burning) → 4 (shrink over `ticks(8)`) → 1; states 2..4 register the draw callback
  0x2c9290 on list 2.
* **Exhaust** `0x2c98b8` (state 3, "on"): type-21 sparks (orange → white, 10 ticks, splitting) at the nozzle (every
  third tick in the hover) and halfway back to the last smoke point; type-23 smoke puffs 0.3 behind the nozzle,
  drifting 0.075 u/tick outward and 0.025 up (0.025 − 5·dt in the glide, − 3·dt in the hover), growth 1.01..1.075
  (1.0165 glide, 1.037 hover), size 30000, grey 0x808080 (0xa0a0a0 glide / hover), ALPHA 0x44; 4 puffs a tick along
  the path since the last one (2 in the glide, 1 in the hover): the plume.
* **Draw callback** `0x2c9290`: the flame's frame from the Thruster-Pack's joint lists (2, 3) / (0, 1) (`0x264630`),
  two crossed quads (FX 0xc) with `randf(0, 0.1)` flicker on their far corners, a camera-facing glow (FX 0xb, RGBA
  0x7f2020ff) 0.13 behind the nozzle, and a 196-vertex flame cone (FX 0xa, scrolling ST, `0x21fda8` strips), all
  additive (ALPHA 0x8000000048). The state part (frame, jitter draws) runs in `draw_callbacks::run_frame`; the drawing
  is `rc-engine` `thruster_render` through `fx_draw`'s callback machinery.
* **Sounds / lights**: none in this code (the Thruster's loop, class sound 0x12, is the hero's `packs::loop_sound`,
  ported before); no point light.
* Tests: `thruster_flame::tests` (the states, the exhaust per tick in 0x10 / 0x81 / 8, the idle wait, the frame),
  `thruster_render::tests` (the geometry, the ST scroll, the tables on every level).

### After-images (the "speed blur", 2026-09-28)
The user's report: the original shows a speed / motion blur while thrusting. **The game has no screen-space blur**:
`DrawWorld` 0x21a1b8's full-screen passes are the frame clear (`append_gif_transfer_packet` 0x222ee8), the AA blit
`PutAABlitPacket_A` 0x223200 (a bilinear copy of the draw buffer, packet 0x151900: PRIM 0x16, no alpha blend), the
fogged sprite, the underwater tint and the fades; Lombyte's "aa blur" stage is that AA blit. What reads as a blur is
the **after-images**: `0x277400` / `0x277428` / `0x277508` / `0x277740` on a 0x140-byte record (a ring of the owner's
last 8 positions and rotations, up to 4 ghost mobys of the owner's class drawn blended with their alpha +0x23, in the
owner's current anim keys at the placement `back − 1` ticks old, faded by the caller), `rc_game::afterimage`. Users
(every call site in level01):

| record | user | ghosts (alpha @ ticks back) | fade | port |
|---|---|---|---|---|
| 0x1409c0 Ratchet | Thruster long jump 0x10 (SetState) | 0x28 @ 2, 0x14 @ 4, 0x0a @ 6 | 2 a tick after tick 12 | `jump.rs` entry, `packs::thruster_trail` |
| 0x1409c0 | Thruster high jump 0xd (SetState) | 0x28 @ 3, 0x14 @ 5 | 2 a tick after tick 12 | same |
| 0x1409c0 | gadget lunge 0x20 (physics, tick 8) | 0x30 @ 2, 0x17 @ 4, 0x0c @ 6 | 5 a tick after tick 18 | `walloper::physics` (hero_gameplay.md §15) |
| 0x1409c0 | ended by SetState (on foot), `HeroTeleport` 0x2368e0, the body switch 0x231348 | | | SetState: `states.rs`; the teleport's own kill and the body switch's are not wired (a teleport with a state ends it through SetState) |
| 0x140b00 wrench | the Comet-Strike's throw `0x236da0` | 0x30 @ 3, 0x17 @ 5 | none; ended at the catch | `comet.rs` |

`rc-engine` `afterimage_render` draws the ghosts as extra instances of the owner's class (Ratchet's, the gadget
table's wrench) with the ghost's alpha and mode 0x80a (the blended moby group). One mechanism, both records; the game's
ghosts are table mobys (they can fail to be created with a full table), the port's are not. Not modelled: the ghosts'
low LOD (+0x72 = 0 for class 0), MobyProc's culls on them. The PCSX2 reference frame also shows the one-frame ghosting
of an interlaced (frame-mode, 60 fields) picture deinterlaced by blending [L]: that is the emulator's display, not
the game.

### Weapons + first person
**Done (2026-09-26).**
- **The Comet-Strike** (`comet.rs`): 0x15's physics and transitions tail, the crouch's □ (ground.rs), the throw
  `0x236da0` and the thrown wrench's update (`0x2be1c0` states 10 / 11: out at 23 u/s, the deceleration growing by
  170·dt³ a tick, back to the hand point 0x1403c0 accelerating by 0.9·dt², 0.55 above the ground, the spin, the world
  bounce, the 0.4 hit sphere through the hero's hit path, the whoosh loop in slot 0x14156c, the clank / hit sounds,
  the catch voice and Ratchet's loop exit). The detached item keeps its own position and rotation
  (`HeroItemsAttach`, items.rs). The anim loop exit `0x247d18` / `0x13fe08` is in `anim.rs` (`AnimCtl::exit_loop`;
  its rate 0x7f800000 reproduced as its PS2 result: docs/plan/hardware_fidelity_layers.md).
- **First person** (`follow_camera.rs`: camera type 4 `0x316330` and the switch blend 0x167370): the look stances'
  camera. It takes over 7 ticks into the stance (the follow camera turning behind Ratchet meanwhile), blends in over
  20 ticks, then sets 0x1413f5 every tick: Ratchet turns to the view at once (stance.rs `first_person_turn`) and is
  hidden with his items (`HeroSyncMoby` `0x2486c0`: `Hero::write_back`, the engine's Ratchet and moby_attach), the
  thrown wrench excepted. Eye 1.6 above the feet, the sticks / d-pad turn it (1.5° a tick, eased at the 84° pitch
  limit, the eye pushed 0.5 forward when looking down). Out of the stance: the follow camera snaps behind Ratchet and
  blends in over ~56 ticks (a cut when the views are 80° apart during the blend-in). **No view model**: the game hides
  Ratchet and his hand item.
- **Throwing in first person is the game's own**: `HeroPdaGadget` item 8 in states 1 / 0x1e after 20 ticks calls the
  throw directly (camera-aimed, 7° up, or at the camera line's hit; voice 0x1b); the Bomb Glove's update throws on ○
  in 0x1e (its update turns the look stance 1 into 0x1e) aimed along the camera. No Port Options entry was needed.
- **The thrown wrench's rotation** (verified in the disassembly, 2026-09-27; was inferred). The throw `0x236da0`
  builds identity rows, multiplies in the source rows (`0x1fa328(M, src, M)`; src = the camera's Euler 0x167250 /
  0x167254 / 0x167258 through `0x1fa030` in the look stance, else Ratchet's moby rows +0xc0), row 3 = (0,0,0,1)
  (`0x1fa298`) and stores `0x2721f0(rows)` in the wrench's +0x40: in first person the model x axis points along the
  view (pitch 0x167254, positive looking down) and its z axis along the view's up. The flight's spin (`0x2be1c0`
  line `0x277380(0, 0, dt·24.43, wrench)`) is **matrix-based in the moby's own frame**: `W = 0x221980(+0x40)`,
  `S = 0x221980(0,0,θ)`, `W ← 0x221ce8(W, W, S)` (row i of S through W: R = R·Rz(θ)), `+0x40 = 0x2721f0(W)`; the
  render rows are `0x221980(+0x40)` = Rz·Ry·Rx (moby_render_notes.md §2). So the wrench always spins about the
  view's up, flat in the plane of the view and the view's left, at any pitch. `0x277380` has one caller per level
  (the wrench update). The port had added θ to the Euler z (a spin about the world z axis: the same only while
  level, a wobbling cone that grows with the pitch); now `comet::turn_local` / `launch_euler`. `0x2721f0` stores
  its last `FastArcTan` as is (x = atan2(r1.z, r1.y); the `neg.s f1, f0` at 0x2722d8 feeds a dead stack word):
  `services::rows_euler` returned −x and is fixed (its other users: the bolts' settle quaternion, the creature
  debris' Euler spin step, the Magneboots frame, all only when their rows carry an x rotation). The ballistic helper
  `0x26faf0` (`weapons::launch_velocity`, `knock::lob_up`) is the bomb's / knockback's arc, not the wrench's; both
  ports match its decompilation (`len2(to − from) / t`, `−((from.z − to.z) + g·n²/2) / n`).
- **Weapons** (`weapons.rs`): the throw gloves' fire case, 0x23, the weapon arm 0x1413f8 (its upper-body animation
  layer: ported 2026-09-28, hero_gameplay.md §2.4), the holster check, ammo (`0x249530` / `0x249450`, the game state's table mirrored by the
  engine; the HUD's count follows), the Bomb Glove's update `0x2d8330` as its `HAND_ITEMS` row. The bomb (class 121,
  `moby_update/classes/bomb.rs`: flight, contact explosion, water, the growing 2.5 sphere hitting through the moby
  hit path, fireballs 122, flashes 1192, type-11 rings, sound, camera shake) and its creation from the hero through
  `HitSink::create_moby`. **Remaining**: 0x20 (the Walloper's lunge, item 0x12), 0x21 (the rebound: needs the targets'
  records), the other throw gloves' item updates (Mine Glove 0x11, Glove of Doom 0x14, Drone 0x18, Decoy 0x19: their
  fire case is generic, their rows and projectiles are not ported), the Blaster 0xf / Pyrocitor / other weapons'
  cases, the auto-aim target list 0x1abe80. 0x17 / 0x27 / 0x2e / 0x30 have no SetState caller in any level; 0x38..0x3a
  are set by one moby callback (not weapons).
- Tests: `comet::tests` (5: with the first-person orientation and spin at 0° / 30° / 60° / 84° against
  Rz(yaw)·Ry(−p)·Rz(n·dt·24.43), and the old world-z step differing when pitched),
  `services::tests::rows_euler_*` (2), `weapons::tests` (4), `anim::tests::loop_exit_jumps_to_the_key`,
  `follow_camera::tests::first_person_*` (2), `classes::bomb::tests`, `tests/weapons/hero_weapons_novalis.rs` (the Comet-Strike
  breaks a crate and the wrench returns, a bomb breaks a crate, first person in / aim / out, the first-person throw;
  deterministic). The `novalis_hero_digest` guard is byte-identical up to the look stance at tick 980 of the moves
  script, where the first-person camera now comes up (camera state only; the digest ignores the new idle fields).

### Bolt crank (0x3b)
**Done (2026-09-27).** Levels 1, 4, 8 (class 280 `BoltCrankUpdate` level01 0x2e0c68, level04 0x2bfd30, level08
0x2d8270, identical; jump table 0x20ae00, tail 0x2e0bd8). **The crank class drives the state**, as in the game: it
tests Ratchet in the moby loop and calls `SetState(0x3b, 1)`, then every tick stores his position / yaw / target
yaw, clears 0x13f430..0x13f4bf and calls `SetAnim`, and lets go with `SetState(0, 1)`. 0x3b itself (`hero/crank.rs`):
entry = group 9, frozen 0x1413fd (no move pipeline), 0x1413f7 = 1, `SetAnim(ticks(8), 0x44, 2)`, the wrench (class
0x47) to its sequence 0xb frame 2 over 13 ticks; no physics case, no transition case (the prologue's hit intake
still ends it). [H: disassembly of 0x2e0c68 and SetState case 0x3b]
- **Latch** (`crank::latch` + the class): feet within 1.75 (XY) of the bolt, body 0, facing it within 90°, feet
  0.1..0.5 above its origin, item 8 in hand with its moby, □ within 7 ticks; the crank: > 30 ticks since the last
  release, the bolt at its top, not done, no targetable moby within 3.25 (XY).
- **Turning** (`crank::turn`): last tick's stick 0x141070 turned by the camera yaw; its part along his facing is
  the speed's target (step 5.5·dt², cap 3.7 u/s, ≥ 0, snap to 0 below 5.5·dt²/4); the step along the facing, then
  `Spring(0, 0.015, 0.3, 7·dt)` (velocity gp−0x5360) back onto the 1.05 ring, height kept; target yaw = the
  tangent (angle + 90°; − 90° with the mirrored-animation cheat 0x15edb5, which the moby loop cannot see in the port:
  taken as off), yaw toward it at 360°/s; drop 9.8·dt² per tick onto `GroundHeight(0.5)`. Anims: 0x44 → 0x3f (wrap,
  blend 7); with no blend running 0x3f → 0x40 moving, 0x40 → 0x41 above 0.6 of the top speed, 0x41 → 0x40 below
  0.4 (16), 0x40 → 0x3f stopped (14). The bolt follows his angle + 15° at 180°/s and steps ±60° (hexagon).
  Progress = unsigned angle turned / (turns · 2π).
- **Release**: □ again after 60 ticks in the state, or done. Let go early, the crank unwinds (1/60 a tick, spinning
  back) and what it drives follows back; done: death bits, class sound 0, sink 0.5, done on later loads; with a
  checkpoint cuboid (+0x34) and its mission not done: `SetMissionDone` + the checkpoint record.
- **Driven objects** read the progress through a pvar moby link (docs/plan/triggers.md §5a): Novalis 665 sliders
  (door pairs #683 / #684 ← crank #296, #685 / #686 ← #297) and 641 (rotator #672 ← #297); none is a carrier.
- **Channels** (general, not crank-specific): `services::HeroFields` gained `pose`, `clear_motion` and `calls`
  (`HeroCall::SetState` / `SetAnim`, run by the tick before the hero update with the hero's context; an entry's RNG
  draw therefore lands after the moby loop's later draws); `services::LoopGlobals` (`Hero::loop_in`: this tick's pad,
  the last camera Euler, Ratchet's anim view) is what the moby loop reads of the globals outside it; `SoundSink`
  gained `alive` / `release` (the slider's loop sound stops). Camera: `crate::cinematic` (`CameraScript(…, 3,
  ticks(180))` to the camera cuboid +0x20, `CameraScript2(4)`); its mode 3 (a timed swing, parameters `0x316e88(+0x28,
  +0x2c)`) is still treated as a snap there.
- **Not ported** (counted): `0x316e88`, the visit-state save `0x29b0a0`, the tail's Novalis global flags 0x13d394 /
  0x13d395; Eudora's and Batalia's consumers (other classes, e.g. a lift z − 15 · progress on Eudora).
- Tests: `crank::tests` (7), `classes::bolt_crank::tests` (4), `tests/classes/bolt_crank_novalis.rs` (crank #296: latched at
  tick 40, done at tick 180 with the doors at their targets, let go and idle the next tick, sunk 0.5; let go early at
  0.71 it unwinds and the doors close; deterministic; the class on levels 1 / 4 / 8). The `novalis_hero_digest` guard
  is byte-identical (its runs have no moby loop; `loop_in` is not part of the hashed block).

### Scripted control (G-HERO-002, 2026-09-29)

**Question.** Are the states the classes and scripts set — 0x1d, 0x1f, 0x32, 0x72, 0x78, the gadget poses 0x38..0x3a,
the jump variant 0xc — one "scripted control" mechanism with per-state data, or separate states? **Finding: one
shape, several owners, not one mechanism.** Each is its own `SetState` case (level 01 `0x23cf98`, level 00 `0x2223f8`;
`rc-trace overlay-diff` / H2: the level copies are case subsets of level 00's, the shared bodies unchanged) that writes
group 9, `0x1415d4 = 0` and an animation, has no case in the transitions `0x242930`, and is ended only by the class
that set it. What differs per state is exactly the data a table would hold (control flag, frozen flag, hand-item flag,
velocity, sequence, physics case), so the port keeps them as match arms of one module (`hero/scripted.rs`), not a
new abstraction. The gadget poses, 0xc and the per-body idle are **not** part of it (below).

**Who sets what** (every level's decomp grepped for a literal call of its own `SetState` copy and of `HeroTeleport`;
created instances from the census `classes.tsv`):

| state | setters (class: level, created) | ended by | the state is |
|---|---|---|---|
| 0x1d | Visibomb launch `0x2cb540` (hand item) | the flight's end `0x2cb788` | steering the missile (ported 2026-09-28) |
| 0x1f | the mouse 1818 (01–06, 08, 11, 12, 14: 10), `SetState(0x1f, 0)` + `SetAnim(6, 0, 0)`; the path trooper 459's arrival camera (01: 25, none with a camera cuboid on Novalis) | the mouse's state 4 `SetState(0, 1)`; the trooper after 360 ticks | held while a class shows something, **control kept** |
| 0x32 | Batalia's turret 440 (08: 1; walking into it once its mission byte is done, or `0x2e1698` from the Water Worker 1283's talk), Gaspar's 1201 (09: 3), Hoven's 1267 (12: 1), the ships 1242 (11), 69 (13, `HeroTeleport(…, 0x32, 1)`), 1379 (17, likewise) | the same classes (`SetState(0, 1)`) | **mounted**: a turret or vehicle holds and places him |
| 0x72 | camera trigger `0x2fb5b0`, gunship `0x2f7728` | `SetState(0, 1)` | the cinematics' hold (ported 2026-09-28) |
| 0x78 | the Umbris boss 1106 (07: 1, `0x314150`), which then writes his position and yaw every tick | the boss (`SetState(0, 1)`) | held by the boss |
| 0x38..0x3a | the Hydrodisplacer item's own update `0x309cd0` (item 22; the code is on every level) on a Hydrodisplacer pad 341 | its transitions (0x38 / 0x39 → 0 unless the hand item is 0x16; 0x3a → 0 on the wrap) | the Hydrodisplacer's use poses (G-WPN-006) |
| 0xc | **none**: no literal or teleport caller on any level; the only `0xc` values near SetState are animation ids | — | dead: only the shared jump-group case labels name it |

**Entry / physics / transitions** (level 01 = level 00 for these three; `0x2370b8` has no case for 0x32 / 0x78 on
level 01 — the jump table's tail — and an explicit empty one on level 00):

| state | `0x1413fc` | `0x1413f7` | `0x1413fd` frozen | velocity | animation | physics | transitions |
|---|---|---|---|---|---|---|---|
| 0x1f | 0 (the prologue's clear) | 0 | 0 | kept | idle `0x226f10(0)` over 10 ticks, whatever `play` | the idle case (drag, edge brake, `StickTarget`, gravity) | none |
| 0x32 | 1 | 1 | 1 | kept (the move is skipped) | with `play`: idle on the eased curve 1 (`SetAnim(−1, …)`) | none | none |
| 0x78 | 1 | 1 | 0 | zeroed (`0x221170(0x13f430)`) | with `play`: the fall sequence 0xb on curve 1 | none (no gravity) | none |

**What else reads them.** 0x32: the hit intake refuses hits (`0x231580`), the HUD health / bolts / bolt-alert draws
draw nothing (`0x24e418`, `0x24ea00`, `0x24eed8`), the hero update skips the fog-map writer (`0x228a1c`), the pause /
map gates and the prompt rules exclude it (`0x2aba68`, the vendor rule), the Drone Device hides its drones (479). 0x1f and
0x78 are read only by their own setters (the boss re-tests 0x78 before `SetState(0, 1)`).

**Coverage** (`crates/rc-game/src/hero/scripted.rs`; tests `hero::scripted::tests`, `hero_scripted_novalis`, `hud::tests`):

| address | what | port | test |
|---|---|---|---|
| `0x23cf98` case 0x1f | group 9, `0x1415d4` = 0, idle over 10 ticks; `0x1413fc` left clear; hand item kept | `scripted::entry` (HELD) | `class_hold_entries_match_setstate`, `novalis_mouse_summon_holds_ratchet_in_0x1f` |
| case 0x32 | `0x1413fc` = 1, group 9, `0x1415d4` = 0, `0x1413f7` = 1, `0x1413fd` = 1, `play` → curve-1 idle | `scripted::entry` (MOUNTED) | `class_hold_entries_match_setstate` |
| case 0x78 | `0x1413fc` = 1, group 9, `0x1415d4` = 0, `0x1413f7` = 1, velocity zeroed, `play` → curve-1 sequence 0xb | `scripted::entry` (GRABBED) | `class_hold_entries_match_setstate` |
| SetState prologue / tail | leaving 0x32 / 0x78 clears `0x1413f7` → the hand-item restore request `0x14145c`; `0x1413fd` cleared | `states::set_state` (existing) | `mounted_hangs_frozen_and_takes_no_hits` |
| `0x2370b8` 0x1f | the idle case | `scripted::physics` → `phys_ground` | `held_ignores_the_pad_until_let_go` |
| `0x2370b8` 0x32 / 0x78 | nothing; the frozen flag skips the move (0x32) | `scripted::physics` (empty arm); `input_physics_move` | `mounted_hangs_frozen_and_takes_no_hits`, `grabbed_hangs_where_the_boss_puts_him` |
| `0x242930` | no case; the prologue (hits, death height) runs | `scripted::transitions` (empty) | `hits_reach_the_held_and_the_grabbed` |
| `0x231580` | 0x32 refuses hits; 0x1f / 0x78 take them | `damage::hit_intake` (existing) | `mounted_hangs_frozen_and_takes_no_hits`, `hits_reach_the_held_and_the_grabbed` |
| `0x24e418` / `0x24ea00` | health / bolts draw nothing in 0x32 | `hud::draw_health` / `draw_bolts` (`Inputs::hero_state`) | `mounted_hides_health_and_bolts` |
| `0x24eed8` | the bolt alert returns early in 0x32 | NOT ported: the alert's draw itself is G-UI-011 | — |
| `0x228a1c` | the fog writer skipped in 0x32 | `rc-engine gameplay::map_tick` (existing) | — |
| `0x2aba68` | pause / map gates exclude 0x72 / 0x32 / 0x1d | `rc-engine menu_render` (existing); levels 08 / 12 in 0x32: Start → `mode_freezeInit(0)` and the vehicle pause (ship classes 0x45 / 0x563 / 0x4da) NOT ported (G-LVL-002 H1 / G-LVL-009); `RaceTimerShow` and the pad-0x200 HUD show skipped in 0x1d / 0x32: the show is not in the port (G-UI-011) | — |
| sounds, particles, lights, stats, bolts, save flags | none in the three cases | n/a | — |

**Consumers ported with it** (`classes::units`; tests `scripted_consumers`): the kill cuboids 1039 (U216: 06, 08, 13,
18; `kill_volume`: the 0x32 gate, `SetState(0x77)`, `FadeToBlack` + the death sequence through the new
`HeroCall::Death`) and Batalia's circling fighters 438 (U274: 08, 25; `batalia_fighter`: the path flight, the trail,
the shot-down fall and blasts; its 0x32 branch is the skill point, G-SAV-007 deferred). The mouse 1818 (already
ported) no longer freezes the hero (`HeroTick::Unimplemented`) in 0x1f.

**Ported 2026-10-01** (`hero/hydrodisplacer.rs`): 0x38..0x3a with the Hydrodisplacer (entry group 0x13, anim 0x46 / 0x47 /
0x48 over 8 ticks; physics (L00 `0x217970`): the velocity toward pad 341's centre clamped to 2·dt (`0x25f730`) and the yaw turned toward the pad's at up to 9.42·dt a tick; the
transitions above); 0xc (no caller: nothing to port); the per-body idle `0x227638` / L06 `0x239528` (body 0 is
`SetState(0, 1)`; the others are G-HERO-005's states); the vehicle word `0x140940` / `0x140944` that several 0x32
readers also test (written only by the ships: G-LVL-009).

### Later (not in this push)
Weapons (0x20, 0x21, the other gloves and guns: see "Weapons + first person"), the gadget poses 0x38..0x3a (with
the Hydrodisplacer, G-WPN-006; the scripted states are ported: "Scripted control" above), other bodies (Clank, Giant Clank,
Hologuise), the Hoverboard (0x6b..0x6f ported 2026-10-01: `hero/hoverboard.rs`). Each gets a module when started; the registry already names its rows.

## 4. Findings to fix inside the packages (not fixed now: the restructure is behaviour-neutral)
- **Gravity mode** (`Hero::input_physics_move`): 0x248ad8 returns 1 in state 0 on surface 2 when the Magneboots are
  owned and the air ticks are **below** 4; the port forces 1 with ≥ 4 air ticks and no ownership test. (P5)
  **Fixed (P5):** `boots::gravity_mode` is the game's rule.
- **Surface 2 is the magnetic floor** (Magneboots), not "slippery" as an old comment said; the slippery floor is
  surface 7 (0x140632, level00's reaction). (P1 / P5) **Fixed** (P1's reaction, P5's magnetic floor).
- `HeroItems::f52a` / `f52c` (0x13f52a / 0x13f52c) are set on a swap but never counted down (0x23c710 counts them).
  Nothing reads them yet. (weapons, later)
- `HeroPdaGadget` 0x240ed8: whether its early returns (no hand moby, slot not ready) skip the Thruster epilogue is
  not checked; `gadgets::pda_item` / `packs::pda_epilogue` must follow the disassembly. (P4 / P6) **Checked (P6):**
  the early returns skip it; every case of the switch (and an item without a case) ends in it.
- The jump buffers read 0x13f524 (1 tick instead of 7 / 9 / 6 while it runs): now in the port; P4's glide landing
  sets it.
- player_controller.md §3 "noted only" names corrected here: 1 / 0x1e are the look stance (not strafe), 8 the
  pack glide, 10 / 0x10 the pack long jumps, 0xd / 0xf the pack high jumps, 0x22 the Thruster stomp, 0x81 the
  Thruster hover (not strafe-move), 0x16 / 0x75 / 0x76 hurts, 0x17 / 0x27 / 0x30 weapon stances, 0x18..0x1b
  ledge, 0x31 the sinking floor, 0x41 / 0x42 are the fidget end and the grind hurt (not one group).

## 5. The restructure (2026-09-26)

(The stubs named below have since been filled by P1..P5 and the follow-ups: §3.)

`crates/rc-game/src/hero/`:
- `registry.rs` — `STATES` (131 rows: name, game group, module, ported), `module_of`, `implemented` (replaces the
  hard-coded list; `hero::state::implemented` delegates), and the three dispatchers `dispatch_entry` (SetState's
  per-state part; `None` = continue with the epilogue, `Some(r)` = return `r`), `dispatch_physics`,
  `dispatch_transitions`. Tests: the implemented set is unchanged; no ported state sits in a stub module.
- `states.rs` — the driver: `set_state` (refusals, bookkeeping, the 0x1413f7 restore), `set_state_body` (blink
  reset → dispatch → epilogue), `transitions` / the prologue (hit intake seam, death check, weapon check, water
  checks, holster seam, death plane) → dispatch. `Ctx`, `clamp_len_2745f0` (re-exported) stay here.
- `common.rs` — `yaw_ago`, `idle_seq`, `gravity_from`, the jump choosers (`stick_sector`, `flip_window`,
  `try_jump`, `crouch_jump` with the pack seam, `jump_or_running_jump`, `step_down_ahead`), `land_eta`,
  `quadratic`, `clip_to_world`, `blend`, `HALF_PI` / `QUARTER_PI`.
- Group files (code moved verbatim, only visibility and seams changed): `ground.rs` (0, 3, 4: entries,
  `phys_ground`, `tr_idle` / `tr_stop` / `tr_crouch`), `walk.rs` (2, 0x73: entry, `phys_walk`, `tr_walk`,
  `walk_run_anim`), `air.rs` (6: entry, `phys_fall`, `tr_fall`), `jump.rs` (the jump group: `jump_group_entry`,
  `jump_block_defaults` (the shared part of the entry, split out for the package jump ids), `jump_entry`, `phys_jump`, `jump_vertical`, `tr_jump`, `landing_picker`, `descent_anim`); `melee.rs`, `swim.rs`
  unchanged apart from the weapon-check seams.
- `physics.rs` keeps the primitives, `Env`, air control, the move pipeline; `state_physics` is the dispatcher
  wrapper; the move starts with `platform::platform_update`; `post_move` counts down every FastDecTimer of
  0x23c710 (new `Hero` fields f500, f502, f508, f510, f518, f51a, f520, f524, f530, f534, f536, f53e, f546).
- `hero.rs` — module list; `surface_reaction` moved to `surface.rs` (now takes the SetState context);
  `hero_update` = `hero_update_with_sounds(.., &mut NoHeroSounds)`; the `HeroSounds` hook after the advance;
  `ledge::wall_ledge_probe_b` after the surface reaction.
- Stubs, routed by the registry, all behaviour-neutral: `platform.rs`, `surface.rs` (P1), `damage.rs`,
  `stance.rs` (P2), `ledge.rs` (P3), `packs.rs` (P4), `boots.rs` (P5), `swingshot.rs`, `gadgets.rs` (P6). Each
  module doc lists its states, the game functions and the seams wired for it.
- Seams in shared code (each a call into a stub that returns "nothing happened" or plain code on a field that is
  always 0 today): transitions prologue (`damage::hit_intake`, `gadgets::holster_check`), weapon check
  (`gadgets::pda_item`, `packs::pda_epilogue`), idle (`packs::hover_latched`, 0x13f524 buffer), walk
  (`surface::walk_surface`, 2 → 0x3f on `f658`, `boots::rail_contact`, `surface::walk_tail`, 0x13f524), fall
  (`packs::fall_to_glide`, 0x13f524, `packs::fall_to_thruster`, `f838` → 0x18, `boots::rail_contact`,
  `boots::cable_contact`), jumps (`packs::jump_pack_moves`, `packs::jump_crouch_press`, `packs::pack_jump_wall_hit`,
  `packs::heli_long_jump_landed`, `packs::thruster_fallover`, `ledge::wall_jump`, `packs::jump_to_glide`, `f838`
  → 0x18, `ledge::wall_ledge_probe_a`), crouch jump (`packs::crouch_jump`), ground physics
  (`boots::ground_magnet`), move (`platform::platform_update`), hero update (`ledge::wall_ledge_probe_b`,
  `HeroSounds::anim_advanced`).

**Proof of identical behaviour.**
- `crates/rc-game/tests/hero/hero_novalis.rs::novalis_hero_digest` (new guard): with `RC_HERO_DIGEST=<file>` it runs
  the lake swim / dive with and without the Hydro-Pack and a move script (jumps, double jump, running jump,
  crouch, flip, fall off the plateau, L1 into the unported look stance), 2800 ticks, and writes per tick the state,
  timer, position and a hash of the whole hero block, Ratchet's anim state, camera, pad and RNG. Before vs after
  the restructure: byte-identical (the new zero timer fields are dropped from the hashed text,
  `DIGEST_NEW_FIELDS`). States covered: 0, 1, 2, 3, 4, 6, 7, 9, 0xb, 0xe, 0x33, 0x35, 0x36, 0x37.
- Engine, `RC_SCENE=0`, frame-exact: `RC_PLAY_SCRIPT="0-5:rstick 1 0,20-226:stick 0 -1,250-310:rstick 1
  0,330-331:press SQUARE,420-421:press X,440-441:press X,500-560:stick 1 0,520-521:press X,600-601:press SQUARE"`
  to frame 650 (crate combo, jumps, double jump; states 0, 2, 6, 0x13, 7, 0xe, 3) and the swim script of
  player_controller §14.7 with `RC_GIVE_HYDROPACK=1` plus `940-941:press X,990-991:press X` to frame 1050 (0x37,
  0x36, 0x35, 0x34, 0x33): `RC_PLAY_TRACE` identical tick for tick before and after (the engine's trace gained a
  `| back pack … | snd …` suffix from a concurrent gameplay.rs change between the runs; compared without it) and
  identical PNGs.
- All rc-game tests pass (200 unit + the integration tests), `cargo clippy -p rc-game --tests` clean for the hero files.

**Vendor (2026-09-27, docs/plan/interaction.md).** `OpenVendorMenu` puts Ratchet in state 100 (`SetState(100, 1)`, the
cutscene-control state, not ported) with 0x1413f5 = 1 (hidden); the port does not change the hero for mode 5: the gameplay tick
does not run while the vendor is open and crate::interact_render hides his entities. `VendorExit`'s `SetState(0, 1)` is therefore
a no-op here. The vendor's prompt rule reads the movement group 0x1413dc (0 / 1, or state 3), the state (not 0x1d / 0x32) and
0x1413f4 (`interact::vendor_rule`). `novalis_hero_digest` unchanged.
