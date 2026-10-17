# Player controller (hero) and gameplay follow camera

What Ratchet does per logic tick on foot, and how the type-0 follow camera follows him, precise enough to port
basic play on Novalis. Addresses are **level01.elf** (the in-game engine; hero code only exists in the overlays).
Hero and pad data are fixed **boot .bss** globals, so they have the same address in every overlay. gp = 0x166c00.
Sources: Ghidra C of level01 (`work/decomp/level01.elf/`) plus disassembly wherever the decompiler dropped float
arguments (it often does for this code). Lombyte only names the camera and pad functions (`UpdatePad__FR3PAD`,
`ProcessPadInput__FR3PADPUci`, `UpdateAllCameras__Fi`, `Cam_InterpValues__FffPffff`,
`Camera_handleCollWithHero__FiP9UpdateCam`). Every hero name below is ours.
Confidence: **H** = read from the instructions, **M** = decompiler C, partly checked, **L** = inference.

## 0. Units and time
- The world is Z-up, in game units (the collision grid is 4 units per cell). Velocities are **units per tick**. Speeds in
  code are written `c·dt` and accelerations `c·dt²`, where dt = `0x15ed6c` = 1/60 and dt² = `0x15ed70` = 1/3600 (NTSC). So
  the source constants read directly as u/s and u/s². (H)
- `fun_001f96f8(n)` = `(int)(n·0x15ed68 + 0.5)` converts tick counts. It is ×1.0 on NTSC. PAL (`fun_00214970(1)`) uses
  0.8333 ticks, dt 0.02, speed scale `0x15ed60` = 1.2 and `0x15ed64` = 1.44. The port targets NTSC, so every count below is
  a literal tick count. (H)
- **Main loop** (`entry` 0x259c40), mode 0: `UpdatePad` 0x27c478 → gameplay 0x2aba68 → render 0x21aa30. If RCNT1 >
  0x2580 (one 60 Hz field) after the frame, it runs one extra `UpdatePad` + gameplay tick (catch-up; at most 1). Pad,
  hero and camera are all per logic tick. (H, catch-up rule from the pad analysis)
- **Gameplay tick 0x2aba68** (after the pause/freeze checks; update mask `0x16c4e0` = 0xf normally, bit 0x10 = the
  debug single-step): mobys (mask 2: 0x263300, 0x279470, 0x2a1a18, 0x2b9a68) → **hero `0x228870`** (mask 1) →
  particles (mask 4) → **camera `0x20eca8`** (mask 8) → sound, point lights, help → `0x2ab960` (tick counter
  `0x15f5cc`++). Rendering then uses Camera `0x167240` (boot 0x187080). (H for the order, M for which moby calls are
  which)

## 1. Input (PAD at 0x13c940, libpad2) — H
| Off | Meaning |
|---|---|
| +0x100/+0x104 | right stick X/Y (f32, after dead zone) |
| +0x108/+0x10c | **left stick X/Y** (0x13ca48/4c) |
| +0x110..+0x13c | pressures /255 (Cross +0x128, Square +0x12c) |
| +0x1a0 | **held** = buttons plus stick-direction bits (lx<0 → 0x8000, lx>0 → 0x2000, ly<0 → 0x1000, ly>0 → 0x4000) |
| +0x1a4 / +0x1a8 | **pressed** = held & ~prev (+ 0x10000 "flick"), **released** = prev & ~held |
| +0x1b0 | raw buttons only (the hero's d-pad fallback) |
| +0x1e0 / +0x258 / +0x2d0 | 30-entry history: pressed masks, stick angle, stick length. Index +0x18e, count +0x190 |

- Buttons: `((b0<<8)|b1) ^ 0xffff`, the standard SCE layout: L2 1, R2 2, L1 4, R1 8, △ 0x10, ○ 0x20, **✕ 0x40
  (jump)**, □ 0x80 (wrench), Select 0x100, L3 0x200, R3 0x400, Start 0x800, Up 0x1000, Right 0x2000, Down
  0x4000, Left 0x8000. Crouch = `held & 0xa` (R1|R2). Strafe = `held & 5` (L1|L2).
- Axes, per axis (not radial): `d=|b−127|; v = d<48 ? 0 : min((d−48)/76, 1); if b<127: v=−v`. **Up and left are
  negative.** A diagonal can reach length √2. Mirror option `0x15edb4` (cheat byte 4) negates lx and rx and swaps L/R.
- Flick bit 0x10000: len > 0.9 now, and either len < 0.25 within the last 3 ticks, or the angle changed by more than 55° within 4 ticks.
- `0x27b940(mask, n, *ago)`: mask pressed within the last n ticks, this tick included (**input buffer**).
  `0x27b9e0(a, b, n, gap)`: both pressed within n ticks, at most gap ticks apart (combos). `0x27ba60(n)`: the stick length
  n−1 ticks ago.
- Hero stick (`0x231d18`): `s = (lx, ly)` from 0x13ca48/4c. If |s| < 0.25, the d-pad replaces it:
  `(R−L, D−U)` from +0x1b0. Stored at `0x141070`. The magnitude `0x1415ec` = min(|s|, 1).

## 2. Hero record (global block at 0x13f350; Ratchet's moby* at 0x1413d0) — M unless marked
| Addr | Meaning |
|---|---|
| 0x13f350 | rotation matrix rows from the Euler at 0x13f3e0 (`fun_001fa050`), copied to moby+0xc0 each tick |
| **0x13f3d0** | **position** (feet) |
| 0x13f3e0 | Euler rotation; **0x13f3e8 = yaw** (rad, wrapped to ±π) |
| **0x13f430** | **velocity** (u/tick; .z = 0x13f438) |
| 0x13f440 | platform delta for this tick (w carries the platform yaw) |
| 0x13f450 | total displacement this tick (pos − old); scaled down when len(eff) > 52·dt |
| 0x13f460 / 470 / 480 | "effective" velocity (the velocity projected on the real displacement), and its vertical / horizontal parts |
| 0x13f490 / 0x13f49c | platform displacement actually applied / platform yaw (the camera reads both) |
| 0x13f4a0 | carried momentum (decays in the ground states) |
| 0x13f4b0 / 4b4 / 4b8 / 4bc | len(eff), len(eff.xy), forward speed (eff·facing, ≥ 0), slope ratio dz / len(eff.xy) clamped ±0.5 |
| 0x13f4c0 | stick direction in the world (camera rows × (−ly, −lx)) |
| **0x13f4d0 / 4d4 / 4d8** | **target yaw**, yaw velocity, yaw residual |
| **0x13f4e0 / 4e4** | **target speed / current speed** (u/tick) |
| **0x13f4e8** | **state timer** (ticks in the state; ++ in `0x23c710` after the move, reset to 0 by the state setter) |
| 0x13f550 / 560 | last capsule contact normal (unnormalised) / contact point |
| 0x13f570 / 574 / 584 | capsule top / bottom offset / radius: **0.8 / 0.7 / 0.45** (they spring toward the targets 0x13f578/57c/580) |
| 0x13f58c / 0x13f5a7 | moby hit by the capsule / "capsule hit this tick" |
| 0x13f5c0 / 5e0 / 5f0 | ground normal / gravity dir (0,0,−1) / ground point |
| 0x13f628 / **62c** / 630 / 63c | ground z / **height above ground** (negative if the ground is above) / slope angle / slope yaw |
| 0x13f634 / 638 | ground pitch/roll under the hero (±45°), used by the velocity and the model tilt |
| 0x13f64c / 0x13f650 | ground moby / grounded tick count |
| **0x13f65e** | **air ticks** (0 = grounded: hit < 0.02 below and slope ≤ 50°) |
| 0x13f750.. 0x13f7ff | jump block (§4.4) |
| 0x13fc70 | external push vector (knockback), decays |
| 0x13fde0 / de4 / df8 | anim playback speed / blend rate / current frame (float) |
| 0x140e50 | 32 × vec4 position history (collision fallback) |
| **0x1413d4** | **state** (enum §3); 0x1413d8 substate; **0x1413dc** movement group; 0x1413e0/e4 previous state/group |
| 0x1413f4 | character (0 = Ratchet; 1/2/3 = other bodies, L) · 0x141403 gravity mode (0 = normal Z) · 0x1415f8 health |

## 3. State machine
Per tick, the hero update `0x228870` (`HeroUpdate`; Ghidra's level01 name `PatchShrubGifs` there is a wrong earlier rename, see tools/ghidra/names/doc_names.csv) runs:
1. `0x247d48` Ratchet's own anim advance.
2. **`0x231d18`**: stick → `0x2370b8` per-state **physics** → `0x23c458` **move+collide** → timer++. It also pushes the position/yaw
   history, rebuilds 0x13f350, and sizes the capsule (`0x231ae0`).
3. `0x22cd48` surface reaction (water, special floors).
4. `0x22d090`.
5. **`0x242930` transitions**, which run with the new timer.
6. Cosmetics and the moby sync `0x229f20` (moby+0x10 = pos, +0x40 = Euler, rows = 0x13f350, then `fun_0020def8`).

**`0x23cf98` SetState(id, playAnim)** saves the previous state/group/timer, sets the group, zeroes the timer, runs the entry
action and, when playAnim ≠ 0, the entry anim. It returns 0 when refused (locked groups). (H)

States (group 0x1413dc in brackets; **the full inventory of all 131 ids across the 19 levels, with corrected names, is
docs/plan/hero_states.md** — it supersedes the "noted only" names below where they differ):
- **On foot, detailed below:** 0 idle [0], 2 walk/run [1], 3 stop/skid [1], 4 crouch [0xc], 6 fall [2],
  7 jump [4], 9 running jump [4], 0xe double jump [4], 0xb crouch jump (long jump; M-L) [4].
- **Water (§14):** 0x37 tread water / 0x36 surface swim [0x12]; 0x33 underwater stroke / 0x34 drift / 0x35 Hydro-Pack
  [0x11]; 0x12 jump out of the water [4]; 0x73 wading [1]; 0x6a drowned [0x14].
- **Noted only:** 1/0x1e strafe stance (L1/L2) [0]; 0x81 strafe-move/look [1]; 0x2d/0x77 fall
  variants [2]; 8 heli glide [5]; 10, 0x10 pack jumps (Clank module `0x22ddd8(3)` = 2/3, not available at the start of Novalis);
  0xc, 0xd, 0xf, 0x11, 0x12, 0x1c, 0x29, 0x2a, 0x3c, 0x49, 0x4c, 0x5e, 0x69 other jumps [4]; 0x13–0x15, 0x20, 0x23, 0x51
  melee/wrench [6]; 0x22 air crouch attack [0xb]; 0x16, 0x56, 0x75, 0x76 [7]; 0x17, 0x27, 0x30 [8]; 0x18–0x1b [3]; 0x31 [0x10], 0x32
  [9]; 0x41–0x42 [0xf]; 0x1d, 0x1f, 0x32, 0x3b, 0x72, 0x78 [9, scripted]; 0x3d death (health < 1), 0x57, 0x6a, 0x7f,
  0x80, 0x82 [0x14]; 99/100 [0x18]; 0x53–0x59, 0x65–0x67, 0x70, 0x71, 0x79, 0x7a (surface, ship and vendor states).

### 3.1 Transitions (0x242930, then SetState(new, 1)) — M
- **0 idle:**
  - L1|L2 held → 1.
  - R1|R2 held and grounded → **4**.
  - ✕ within the last **7** ticks → `0x241968`: crouch held + ✕ with a direction or flick (`0x27b9e0(0x40,0x1f000,7+12,12)`) → **0xb**, else **7**.
  - Air ticks > 4 **and** height > 0.4 → **6** (coyote time: 4 ticks).
  - Stick > 0.22 → **2**. Exception: just after a 1/0x1e stance (< 24 ticks), only when the stick is within 70° of the camera heading.
- **2 walk/run:**
  - Air ticks ≥ 5 and height > 1.35 → **6**.
  - ✕ within **9** ticks and (air ticks ≤ 4 or 0x13f532) → **9** if substate = run, stick > 0.82, |target yaw − yaw| < 60°
    and |eff.xy| > 0.85·5.7·dt; otherwise → **7**. Crouch → 4. Strafe → 1.
  - Stick < 0.17: |eff| ≥ 4.4·dt and the stick 3 ticks ago ≥ 0.8 → **3** skid; else |eff| > 2.7·dt → 3 with anim 0x14 at
    frame 7 (short stop); else (slow) and timer > 25 → **0**.
- **3 stop:** ✕ pressed → 7; crouch → 4; air ticks > 8 and height > 0.3 → 6; stick > 0.5 → 2; speed 0 → 0.
- **4 crouch:** release after 11 ticks → 0 (anim blend −2); ✕ → `0x242420` (crouch jumps); □ within 15 ticks with weapon 8
  → 0x15; air ticks > 8 → 6.
- **Airborne jump group (7, 9, 0xe, 0xb, …):**
  - **Landing:** when the landed counter `0x13f768` = 0 and air ticks == 0 and descending (`0x13f76e`), set `0x13f768 = 1`
    and play the landing anim. It counts up after that. Once it is > 1, `0x242198` picks the next state: crouch held → 4
    (anim 0xd); stick > 0.3 → 2 (run substate, anim 4); forward speed > 3·dt and more than 3 ticks → 3 (anim 5, blend −1);
    after 12 ticks or when the anim ends → 0.
  - **Double jump → 0xe** needs all of: state 7 or 9 (flag `0x13f7ff`), not landed, timer > 14, ✕ pressed within the last
    `clamp(timer−4, 2, 30)` ticks, (height ≥ 0.4 or no special floor). That is enough before the apex; once descending it
    also needs height > 0.7 and |vz| < 5.8·dt.
  - Timer > `0x13f7ea` (80 ticks for 7/9) and not landed → **6**.
  - Pack moves (8/10/0x10) and the air crouch attack 0x22: note only.
- **6 fall:** ✕ within 6 ticks while 0x13f532 > 20 → jump. Landing (air ticks == 0) → 0/2/3 by stick and speed (anim 0xc). The
  landing clamps vz and the displacement z to ≥ −9·dt. An impact after > 90 ticks is handled separately.

## 4. Per-state physics (0x2370b8) — M, constants H (read from the instructions)
Shared helpers:
- **`Approach(t, step, &x)`** (0x270728): move x toward t by at most step.
- **`Spring(t, k, d, max, &x, &v)`** (0x270780): `v += k(t−x) − d·v`; clamp |v| ≤ max and |v| ≤ |t−x|; `x += v`; if
  |t−x| < 0.01·max, snap x = t and v = 0.
- **`TurnTo(k, d, max)`** (0x232490 → 0x270b58): the same spring on yaw 0x13f3e8 toward 0x13f4d0, with wrapped angle differences
  and yaw velocity 0x13f4d4.
- **`StickTarget(scale, mode)`** (0x231f70): s = stick, rescaled to length 1 if |s| > 1. s == 0 → target speed 0 and target
  yaw = current yaw. Otherwise target yaw = `atan2(−lx, −ly) + CameraYaw(0x167258)` (wrapped; **FastArcTan(a,b) =
  atan2(b,a)**) and target speed = |s|·scale.
- **`SpeedStep(acc, dec)`** (0x2326e8): `Approach(target, target > speed ? acc : dec, &speed)`.
- **`SetPlanarVel(yaw)`** (0x232738): `vel.xy = speed·(cos yaw, sin yaw)·cos(pitch 0x13f634)`. vel.z is untouched.

**4.1 Ground states 0, 1, 3, 4:**
1. `StickTarget(1, 0)`. Target speed := 0.
2. SpeedStep(0, dec): dec = **12.6** (idle), **6.48** (crouch), or 3: 12·min(1, slope+1).
3. `vel = 0; vel += momentum` (0x13f4a0). The momentum shrinks in length by dec each tick (0x236678).
4. Crouch (4) with stick > 0.2: TurnTo(0.002, 0.07, 400°/s·dt), anim 0xe/0xf (turn L/R) when |yaw velocity| > 20°/s·dt, else 0xd.
5. Edge brake `0x236a68(3.7, 0)`: sample `pos + vel·3.7`, cast a line from +0.3 to −0.2 around it (flags 4). No ground, or ground steeper
   than 50° → zero all velocities.
6. Wall check `0x232978(0)`.
7. **Gravity:** grounded → `vel.z −= 54·dt²`, which keeps it pressed to the floor. Airborne → `vel.z = eff.z − 25·dt²`.

**4.2 Walk/run 2:**
- `StickTarget(1, 0)`, then 0x232290 picks the speed from a binary table at `0x17c238` (0x10-byte rows {down, up, speed,
  stick}): **target = 0.9·dt if stick < 0.82, else 5.7·dt (walk 0.9 u/s, run 5.7 u/s)**.
- Turn: run (stick > 0.82) TurnTo(0.019·(s+1)/2, 0.1, 570°/s·dt·(s+1)/2); walk TurnTo(0.005(s+0.35), 0.1,
  270°/s·dt·(s+0.35)); run substate without a sharp turn (0.008, 0.15, 570°/s).
- **SpeedStep(7.5·dt²·f, 8.5·dt²)**. f = 0 while the stick < 0.8 and the residual turn > 30°, else 1. A sharp-turn flag
  (`0x13f70e`, set by a flick) uses decel 17 and scales the target by clamp(150° − residual, 0.2, 1).
- SetPlanarVel(yaw).
- Wall check `0x232978(1)`, then gravity as in 4.1. `0x232820` stops climbing steep walls just after leaving the ground.
- On entry, speed = min(|eff.xy|, 7·dt). It is set to 0 if the turn is > 90° right after a landing.
- **Anim** (`0x241a08`, hysteresis at most every 4 ticks): walk seq **3** → run seq **4** when speed > 2.35·dt, back to
  walk when speed < 1.9·dt; blend 8 ticks, frame phase-matched. Playback speed: walk 157·speed ∈ [0.6, 4], run 14·speed ∈ [0.6, 2.2].
- Procedural lean: `0x235638` writes joint-modifier angles (0x17ab60..) from the yaw residual. Cosmetic.

**4.3 Fall 6:**
- Entry: vel = eff. The group gravity is 0x13fc90 = **24·dt²**.
- Per tick: `StickTarget(1)` + table speed; TurnTo(0.04, 0.2, 860°/s); AirAccel(20·dt², or 2·dt² with a small stick);
  `vel.z −= 24·dt²`, clamped ≥ **−50·dt** (terminal 50 u/s).
- Anim: seq 10 (blend 10), then seq **0xb** after 18 ticks or when height > 1.75.
- `AirAccel(a)` (0x234358): desired D = (cos, sin)(target yaw)·target speed; f = clamp(1.5·(1 − cos∠(v, D)), 1, 3); `v.xy +=
  clampLen(D − v.xy, a·f)`.

**4.4 Jumps (7, 9, 0xe, …; entry in 0x23cf98, per tick 0x2370b8 → 0x234b40 / 0x2345f0).** The jump block:

| Field | 7 jump | 9 running | 0xe double | meaning |
|---|---|---|---|---|
| 0x13f770 | 5 | 5 | −1 | takeoff tick (windup before leaving the ground) |
| 0x13f7d8 / 7dc | 1.47 / 2.62 | 1.0 / 2.7 | h / h+0.05 | min / max jump height |
| 0x13f7e8 | 15 | 15 | 14 | ticks to ramp min→max |
| 0x13f7f0 | 29.7·dt² | 29.7·dt² | 29.7·dt² | gravity up; **×1.17 once descending** (7 only; `0x13f748`) |
| 0x13f774 | 5.7·dt | 5.7·dt | 3.5·dt | air stick speed scale |
| 0x13f7d0 / 7d4 | 20 / 11 ·dt² | same | same | windup accel/decel |
| 0x13f7ea | 80 | 80 | 80 | ticks before forcing fall 6 |
| anim | seq 7, blend 5 | seq 8, blend 5 | seq 0x16 fr 3, blend 7, speed 0.8 | |

For the double jump, `h = 0.7`, or `0.7 + (2.5 − height)·0.5` when the height above ground is < 2.

**Vertical, per tick (0x2345f0) for 7/9:**
1. If ✕ is held (or nothing has been computed yet) and h < hmax and T ≤ 15: `h = min(h + (hmax−hmin)/ramp, hmax)`,
   `pending = sqrt(2·h·g) − applied` (variable jump height).
2. If T ≥ takeoff and pending > 0: `vz += pending; applied += pending`.
3. T < takeoff: `vz = −48·dt²`. Otherwise `vz = max(vz − g, eff.z − 0.1, −50·dt)`.
4. The descending flag is set (and g switched) when T > takeoff and vz < 0.001 at the start of the tick.

Double jump 0xe:
- First 8 ticks: vz rises toward V = 7·dt × (1 | 0.7 | 0.45 once the height above the first jump's takeoff point 0x13f750 exceeds 1.7 | 2.1) by at most 150·dt² per tick.
- Then gravity as above.

Horizontal:
- Windup: stick turn (0.015, 0.2, 860°/s), SetPlanarVel, SpeedStep(20, 11 ·dt²).
- In the air (0x234b40 default): TurnTo(0.04, 0.2, 860°/s), `StickTarget(0x13f774)`, AirAccel(20·dt², or 2·dt² with a small stick).
- In the first 10 ticks with stick > 0.2, a line 0.85 ahead at +0.5 (flags 4) sets the "blocked" flag 0x13f7f4. It caps air speed at 2.1·dt.

Model result (Python, f32, flat ground, no collision), T counted from the first tick in state 7: **tap ✕** apex **1.227**
at T = 21, lands at T = 38; **hold ✕** apex **2.054** at T = 27, lands at T = 48; state 9 tap 0.845 at T = 18, hold 2.056.
The takeoff keeps the windup's −48·dt² in vz, which is why the apex is below h. (M: model, not yet trace-verified)

## 5. Move and collision pipeline (0x23c458 → 0x233de0) — H for order and constants
1. The per-state code leaves `vel`. **Clamp |vel| ≤ r − 0.02** (0.43 u/tick) unless 0x13f51c is set.
2. `pos += vel; pos += push(0x13fc70); push = 0`. Clear the capsule-hit flag and moby.
3. **Capsule resolve** `0x233d08` → `0x233940`:
   - Base = pos + (0, 0, bottom 0.7), height = max(0.05, top − bottom) = 0.1, radius 0.45, so the body spans z + 0.25 … z + 1.25.
   - Up to **8 passes** of `coll_capsule(0x2135a0, flags 0x24)` (flags 0xd24 in state 0x7f) plus hero groups `0x214d70`.
   - Each pass: pos = CollOutput+0x30 (push-out), normal +0x40 → 0x13f550, contact +0x20 → 0x13f560, moby +0x18 → 0x13f58c.
   - Stop when neither query hits.
   - If the total push > 1.5·r, reject: restore pos from the 32-entry history (0x140e50, stepping back one entry per retry, up to 16 tries).
   - Group 0x11 (water) uses a 0.6 sphere (0x212960) instead.
4. **Ground probe** `0x232dc0`:
   - `CollLine_Fix` from pos + (0, 0, 2.2·r = 0.99) down to z = max(0, pos.z − 48), flags 2.
   - Surface 0 (water) records the level and re-casts with flags 0x24.
   - Stores the ground point, normal, height = |pos − hit| (negative if the hit is above the feet), slope angle, and the footstep class.
   - **Grounded** iff height < 0.02 and slope ≤ **50° (0.87266463)**. Otherwise air ticks++.
5. `0x13f450 = pos − old`. **Step-up / snap** `0x233588`, when grounded and ground.z > pos.z:
   - Depth ≥ |vz| + 0.01: undo this tick's downward dz, then Spring(ground.z, 0.057, 0.3, 2·dt) on pos.z — a smoothed step-up.
   - Otherwise snap pos.z = ground.z, unless the capsule touched within 0.5·r.
   - There is no explicit step-height constant. Steps are whatever the capsule base (0.25 above the feet) lets through, plus the probe start (0.99).
6. **Effective velocity:**
   - `eff = dir(vel)·max(0, dir(disp)·vel)`. The same is done separately for the horizontal and vertical parts.
   - Then |eff|, |eff.xy|, forward speed and the slope ratio.
   - **vel itself is not rewritten.** Each state rebuilds vel from speed/yaw and from eff.z. That is how walls "slide": the move is resolved
     by the capsule, and the next tick's vertical terms use eff.
7. **Platform:** if 0x13f440 ≠ 0 (standing moby moved), `pos += it`, then repeat steps 3–5. 0x13f490 = the applied part.
8. **Walls and slopes:**
   - `0x232978`: when a wall is hit (slope ≥ 50°, or surface 8/0xc), remove the velocity component into the horizontal normal.
   - `0x232820`: just airborne, rising, and the contact is steeper than 50° → remove the into-wall part and add vz −= 108·dt².
   - The walk speed is scaled by cos(ground pitch).

The capsule shape eases toward its target at 0.02 per tick: group 4 (air) after takeoff bottom 0.6; state 6 bottom 0.5;
crouch top 0.35; water 0.

## 6. Animation (Ratchet, `ratchet_seq` slot = seq id) — M
- `0x247a90 SetAnim(blend, seq, frame)` always snapshots the current pose (like `fun_00212f90`). It then sets t = 0 and speed
  (0x13fde0) = 1.
  - blend > 0: t += 1/blend per tick.
  - blend = −n: eased curve n (t table at `0x17c270 + 100·(n−1)`, length from gp table −0x7520).
- Ratchet is advanced by `0x247d48` (not by the generic MobyAnimAdvance) at playback speed 0x13fde0 × 0x13fde4.
- Seq ids: 0 idle (0x54 at health 1); 3 walk, 4 run, 5 land-skid, 0x14 stop; 0xd crouch, 0xe/0xf crouch turn; 7 jump,
  8 running jump, 0x16 double jump; 10 fall start, 0xb fall, 0xc hard landing; 0x1c/0x1d crouch jump L/R.
- The hero-animation mirror byte 0x15edb5 (cheat byte 5, next to the mirror option 0x15edb4; a separate flag read by
  `SetAnim` 0x247a90, `SetState` 0x23cf98, `HeroSyncMoby`, `HeroItemsAttach`) swaps 0x31↔0x32 (`SetAnim`) and 0x1c↔0x1d (`SetState`).

## 7. Follow camera (type 0) — H for layout, order and constants; M for the pseudo-code
- `0x20eca8` CameraUpdate:
  1. pre-steps: `0x20e670` computes up2 = −gravity(0x13f5e0) and the smoothed up_s (Interp 0.015/0.2), plus the hero move direction 0x167310 and
     speed 0x167334.
  2. `UpdateAllCameras` 0x20d620: highest-priority UpdateCam of 48 at 0x1675d0 (stride 0xa0), `Camera_handleCollWithHero`, then the
     type's update from `lvl.camvtbl` 0x20c480 (type 0 = init 0x311f38, update **0x314e00**).
  3. It copies cam+0x30 → **0x167240** (pos) and the rows → **0x167450/60/70** (forward, left, up).
  4. It derives Euler 0x167250 (yaw 0x167258, read by the hero) and adds shake.
- Type 0 never writes Euler angles or FOV: **tan(hfov/2) = 0.63 at 0x16cf70** (level01; boot 0x18cdb0), set by `InitViewContext`.
- `Cam_InterpValues(cur, tgt, k, d, max, *vel)` (0x20ce40) is the same spring as §4: `vel += k·e − d·vel`, clamp ±max and ±|e|.
- Data block D (0x169590; defaults copied to 0x169a90): **distance 4.64** (+0x15c), **pivot height 2.0** (+0x160), **look
  height 1.5** (+0xf0), pull-back max 6.0, yaw rate [1.0°, **1.3°**, 1.6°]/tick (option 0x15ede4, default index 1), pitch ±40°.
- Per tick (0x314e00):
  1. Platform carry.
  2. **Target**: the hero position, split into horizontal and vertical parts, each springed (0.015, 0.2). Look-at = target + up·1.5,
     pivot = hero + up·2.0. Ceiling line (flags 0xb4) up to 20 lowers the pivot to ceiling − 0.95 (midpoint if the gap < 1.9).
  3. **Leash:** `off += ((camPrev − pivot) − off)·0.75`, then clamp |off| to the distance. The camera trails the hero
     through this leash. There is no automatic "rotate behind the hero's facing" (an auto-yaw exists only for scripted focus objects or after 400
     ticks idle near one).
  4. **Right stick:** yaw += rx·1.3°/tick about up_s (default options; 0x15ede0 = 0 inverts it). Pitch target = the ry
     remap ((|ry|−0.3)/0.7) × 40°, reached at ≤ 1.75° per tick; stick-up raises the camera (option 0x15eddc).
  5. **Avoidance:**
     - 6 spheres (r 0.35–0.85, flags 0x94) along pivot→offset re-aim the offset around obstacles. With the stick also pushed, the distance reduces by 0.075 per tick.
     - End sphere r 0.95 (flags 0x14/0x34) pushes vertically only and shortens the distance (min 0.2).
     - The reduction recovers over a cosine curve (timer 2000).
     - 30 ticks of a blocked pivot→camera line → hard reset behind the hero (`0x20ee80`).
  6. **Placement spring:** distance (0.02, 0.2; 0.04/0.3 when the hero runs toward the camera), elevation and yaw (0.015, 0.2 each).
     `pos = smoothedTarget + up·2.0 + off`. The elevation from the up axis is kept ≥ 15°.
  7. **Rows:** forward = horizontal(look − pos), pitched down by atan2(offset height + pivot height − look height, horizontal length). Plus a
     springed bias of ≤ 15° when the camera is low. The pitch is clamped to ±70°. Then left = up_s × fwd, up = fwd × left.
     90-tick blend from the previous camera's forward on camera switches.
  8. The parameters (distance/pivot/look) spring back to their defaults (k 0.003 / 0.004).
- Collision ignores the camera moby (class 0x3ef, spawned at the camera by `Camera_handleCollWithHero`).
- State-dependent tweaks: see 0x3111d8 — states 0xb/0xc rigid, 0x81 stiffer, water group: pivot 0.5–2.5.

## 8. Port plan (`crates/rc-game`, PS2 float model where the kernels already use it)
- **`input.rs`:** `Pad` (fields of §1), `process(raw: [u8; 18], prev) -> Pad` (dead zone, bit synthesis, flick, 30-deep
  history), `pressed_within(mask, n)`, `combo(a, b, n, gap)`. The engine maps a gamepad/keyboard to the 18 libpad2 bytes, so
  the game logic sees exactly the PS2 data.
- **`hero/`:** `state.rs` (the enum with raw ids, SetState with entry actions, the §3.1 transitions); `physics.rs` (Approach,
  Spring, TurnTo, StickTarget, SpeedStep, AirAccel, the §4 per-state updates); `motion.rs` (the §5 pipeline on
  `collision_query::{coll_capsule, coll_line}`); `anim.rs` (SetAnim snapshot/blend requests, Ratchet's advance). Keep the
  globals as one `Hero` struct mirroring §2, so traces can diff it field by field.
- **`camera.rs`:** the type-0 data block D, `Cam_InterpValues`, the §7 steps. Output is pos + rows into the existing
  `game_camera.rs` (which builds the matrices exactly like `fun_001f2260`, with the rows as forward, left, up).
- **What the engine must expose:**
  - A fixed 60 Hz tick with one catch-up step, run as: pad → mobys → hero → camera → render.
  - Collision: capsule, line, sphere (done); hero groups and moby collision (TODO: platforms, 0x13f440).
  - Animation: blend-to (seq, frame, ticks | curve n) with a snapshot, per-moby playback speed, a frame index readout, a "sequence
    finished" flag (+0x70 bit 1), and the joint-modifier list for the lean.
  - Camera-yaw readback (0x167258) for the stick mapping.
- **Unit tests (exact numbers, NTSC):**
  - Pad byte 127 or 80 → 0.0. Byte 0 → −1.0. Byte 200 → (73−48)/76 = 0.328947. Byte 251 → 1.0.
  - From rest, full stick in state 2: speed = n·0.00208333 until **0.095 at tick 46**. Distance over 60 ticks = **3.5813**.
    Release → 0 in 41 ticks.
  - Idle momentum 0.095 decays to 0 in 28 ticks (12.6·dt²).
  - Jump 7, tap: apex 1.2268 at T = 21, lands at T = 38. Hold: apex 2.0545 at T = 27, lands at T = 48 (§4.4 model).
  - Fall from rest: vz = −n·24/3600, reaching −50/60 at tick 125.
  - Capsule on flat ground: pos.z stays 0. A 0.2 step is climbed by the spring. A wall stops the move with 8 passes and slides along the normal.
  - Camera init snap (`0x311890`) behind a hero at the origin with yaw 0: pos = (−4.64, 0, 2.0), look = (0, 0, 1.5). Full
    right stick for 10 ticks turns the desired offset D.130 by 13°; the placed offset D.140 lags through the (0.015, 0.2) yaw spring.

## 9. Unknowns / next checks
- Everything here is unverified against a live run. The first PCSX2 trace should record 0x13f3d0/0x13f430/0x1413d4/0x13f4e8 and 0x167240
  per tick for a jump, a run and a stop.
- 0xb vs "long/high jump" naming (`0x23ceb0` stick classification not decoded). States 0xc/0xd/0xf/0x11/0x12/0x1c not decoded.
- The moby collision pass, hero groups and moving platforms (0x249618, 0x13f440) are not reversed. The edge-brake details in states other than idle are not reversed.
- The exact landing-anim frames (0x13f798), the curve tables for negative blends, and the camera target modes 2–11 (hero states 0xb–0x14).
- Whether 0x2ab960's `0x15f5cc` counter and the catch-up tick interact with the animation advance.

## 10. In the port (`crates/rc-game`)

**Restructured 2026-09-26** (docs/plan/hero_states.md: every state, its owner module, the packages): `hero/registry.rs` (state
table + the entry / physics / transitions dispatch), `hero/states.rs` (SetState driver, transitions prologue), `hero/common.rs`,
one file per group (`ground.rs` 0/3/4, `walk.rs` 2/0x73, `air.rs` 6, `jump.rs` the jump group, `melee.rs`, `swim.rs`) and the
package stubs. Files before that split: `pad.rs` (§1), `moby_runtime.rs`, `hero.rs` + `hero/physics.rs` (§4, §5), `hero/states.rs` (§3), `hero/anim.rs`
(§6), `follow_camera.rs` (§7), `tick.rs` (§0). Tests: unit tests in each module (flat hand-built meshes) and
`tests/hero/hero_novalis.rs` (disc data, skipped without `extracted/`). Engine wiring: §11.

**Arithmetic.** All float math runs on the PS2 FPU/VU model (`ps2v::Pf`: round toward zero, no denormals, the adder's
guard-bit rule; operators in the game's op order), with the game's own trig: the VU0 28259 sine polynomial
(`fast_sin/cos`), `FastArcTan` (0x2217c0, VU0 polynomial + octant table), the FPU `asin` polynomial (0x221728), the
quaternion rotation (0x274ac8), Euler → rows via 28259. So the ported steps are **bit-exact by construction** as far as that
model goes (the multiplier's rare last-bit deviations and a possible FPU-vs-VU adder difference are not modelled; nothing
is trace-verified yet). There is no f32-in-order approximation left in the ported code paths.

**Ported, op by op (H from the disassembly):**
- Pad: `UpdatePad`/`ProcessPadInput` (axes, pressures, mirror, stick bits, pressed/released, lock modes, the 30-deep
  histories, the flick bit) and `PressedWithin` 0x27b940 / `Combo` 0x27b9e0 / `StickLenAgo` 0x27ba60.
- Hero driver 0x231d18, per-state physics 0x2370b8 for 0, 3, 4 (ground), 2, 6, 7, 9, 0xb, 0xe (with 0x231f70, 0x232290,
  0x232490/0x270b58, 0x232598, 0x2326e8, 0x232738/0x277b50, 0x234358, 0x234b40, 0x2345f0, 0x236678, 0x236a68, 0x232978,
  0x232820, 0x233850, 0x22a620), capsule sizing 0x231ae0, the move 0x233de0 (clamp, integrate, capsule resolve
  0x233d08/0x233940 with the history rollback, ground probe 0x232dc0 incl. pitch/roll 0x235fe0, snap 0x233588, eff/eff_h/
  eff_v, the platform/edge-nudge step, slope ratio, 52·dt cap), post-move 0x23c710 (timers, stuck counter 0x13f532, edge
  nudge 0x23cc20, body/shadow points, death plane), surface reaction 0x22cd48 (flags part), SetState 0x23cf98 for
  0/2/3/4/6/7/9/0xb/0xe, the transitions 0x242930 for those states (+ 0x241968, 0x242420, 0x242198, 0x2426d0, 0x246690,
  0x241a08, 0x246310, 0x248268, 0x26e520, 0x2720f8), write-back 0x229f20 (pos, Euler, rows), hero init 0x226b70 (snap).
- Animation: `AnimCtl` is the SetAnim 0x247a90 / advance 0x247d48 interface; `RatchetAnim` implements Ratchet's own path
  on `rc_formats::moby_anim` data (always-snapshot SetAnim, curves −1/−2/−3, snap windows, loop range, frame-A rate,
  `0x13fde8` flags, `0x13fdf8` readout 0x263920). `RecordingAnim` is a data-free stand-in for tests.
- Camera: 0x20eca8 pre-steps 0x20eb40/0x20e670, type-0 init 0x311f38 + target reset 0x30f608 + snap 0x311890, update
  0x314e00 (platform carry 0x30f890, target modes 0x30fb08 / vertical target 0x3101c0, targets/pivot/ceiling 0x310a08,
  leash 0x314118, stick 0x313b88/0x313d90, avoidance 0x312ef8 with SphereChain 0x312c08, Reaim 0x3127f0, EndSphere
  0x3124f0/0x312310, the 30-tick reset, placement 0x3148f8 + DistRate 0x3146f0, rows 0x3141e8, spring-back 0x311010),
  the copy to `Camera` and yaw = `FastArcTan(fwd.x, fwd.y)`.
- `MobyTable`: `CreateMoby` 0x263390 (linear scan of the dynamic slots, reuse two ticks after `DeleteMoby` 0x2636c0),
  0x263300, `InitMobyInstance` 0x263488 field defaults.

**Not ported (the hero freezes with `HeroTick::Unimplemented(state)`, logged once):** every state outside the nine above
(strafe 1/0x1e, ledge grabs 0x18.., (swimming and wading are ported since, §14), weapons (the wrench combo 0x13 and jump attack 0x14 are ported since, §12), packs 8/10/0x10/0xd/0xf, hurt/death 0x3d/0x77,
0x79/0x7f/0x81 …); transitions into them are still taken exactly. Moby collision is ported (collision_queries.md §7): the
hero's line / sphere / capsule queries read `Env::mobys` (the table as the tick's moby loop left it; `Env::hero_moby` =
Ratchet, their `a2`), the write-back is followed by `MobyBuildMatrix(Ratchet)` (bounding sphere, grid re-registration; the
write-back copies only the three rotation rows, 0x30 bytes, keeping +0xf0), and the camera's queries read `CamInput::mobys`
(follow_camera.rs module doc: the game's `a2` per query, the avoidance's switch-off of mesh-less mobys, the ±1° nudges, the
crate hit of a blocked camera line). Not ported: Ratchet's per-tick collision cylinders (`FUN_00263cb8`), moving platforms (0x249618; 0x13f440 only from the edge nudge), the ledge-grab probes (0x22c9a0,
0x22d090), the periodic wall/ledge probes of 0x23c458, idle fidgets (RNG-driven 0x241e00; SetState(0)'s
`rand_range(50,100)` **is** drawn), cosmetics (lean 0x235638, side probes, squash, sounds/particles), the Euler x/y
straightening 0x236520 (no-op on foot), the camera moby 0x3ef and the 15-unit focus query of 0x3111d8, pass-through volumes, camera switches/blends and the
shake, the random ±0.5 of the snap when the camera would sit on the hero (the port takes +), and PAL.

**Model numbers the port reproduces (tests):** tap jump apex 1.2267 at T = 21, touchdown T = 38; held jump apex 2.0543 at
T = 27, touchdown T = 48; running jump 9 tap apex 0.8445 at T = 18; run from rest reaches 5.7·dt in 46 ticks (walk 0.9 →
run 5.7 through the table); a release after a full run goes to skid 3 and stops in ~30 ticks (1.33 u); idle momentum
0.095 decays in 28 ticks; coyote fall at the 5th airborne tick; double jump on a tap at T > 14 and from a buffered tap
(window clamp(T−4, 2, 30)); crouch + back stick + ✕ → flip 0xb with seq 0x1f. Novalis (`hero_novalis.rs`): Ratchet
ground-snapped at (162.53, 136.39, 60.0) stays grounded in state 0 for 120 ticks; 120 ticks of full stick move him 9.09 u
along the camera yaw (1.987 rad) on the terrain with anims 0 → 3 → 4; a tap jump peaks at 1.2268 and returns 7 → 0 with
seqs 7 → 0; the camera settles 4.64 u behind at pivot height with a clear pivot → camera line (flags 0xb4).

### 10.1 Corrections found while porting (supersede the sections above)
- §1: `0x1415ec` (stick magnitude) is last tick's: 0x231ed8 runs before the stick copy. `PressedWithin` covers
  `min(n, count)` ticks with this tick as age 0; `Combo` needs `|agoA − agoB| < gap` (strict); `StickLenAgo(n)` reads
  `min(n, count) − 1` ticks back. `FastArcTan(1, 0)` is 2⁻²²·0.75, not 0.
- §2: `0x141403` (gravity mode) is recomputed every tick by 0x248ad8 from `0x13f658`. The capsule 0.8/0.7/0.45 are targets;
  top/bottom approach them at 0.02/tick, the radius springs (0.02, 0.3, 4·dt), easing freezes while the last contact is
  within 0.5·r; group 4 after takeoff uses bottom = `0x13f784` (0.6). `0x13f62c` = ±3-D distance to the probe hit, 32.0 on a
  miss; `0x13f630` = atan2(|n.xy|, n.z) of the raw normal; `0x13f634/638` are 0 unless height < 0.25 and the group is not an
  air group. `0x13fc70` push is rebuilt each tick by 0x233850 from the knockback parameters 0x13f680/684/688 and zeroed after
  the move (not decayed). `0x13f514` is the landing lockout (blocks jump/crouch/walk and the fall air control).
- §3.1: **0xb is the side/back flip** (crouch held + stick left/right/back of the facing in 45° sectors + a stick-direction or
  flick press within 12 ticks of ✕; h 3.2–3.25; seqs 0x1c/0x1d/0x1f); crouch + ✕ without a Clank pack is a plain 7 after 6
  ticks of crouch. Idle: coyote exception after state 8; the lockout blocks jump/crouch/walk. Walk: the fall also needs the
  step-down probe 0x2426d0 to fail; nothing after the jump test runs while airborne; the 9 test is stick length > 0.82, turn
  < 60°, |eff.xy| > (5.7·dt)·0.85, and walking substate always gives 7. Stop: ✕ only on the press tick; anim end → idle;
  "speed 0 → idle" only while the stick > 0.2. Landing: the picker runs from landed == 2; skid uses forward speed
  `0x13f4b8` > 3·dt; bunny hop ✕ within 8 ticks of touchdown. Jumps: also fall over when descending, timer ≥ 70 (50 for 0xb)
  and height > 2.5 (4.5); R1/R2 in the air ends that tick's transitions; a slow 9 turns into 7 mid-air by a raw write; only
  7 and 9 can double jump; 0xe does not land while its anim frame < 35. Fall landings: ≥ 90 ticks → 0xc fr 4 + lockout 22;
  after the 0xb anim switch → 0xc fr 9 curve −1 + lockout 7; short fall → 2 / 3 (seq 6) / 0.
- §4: SetPlanarVel writes **vel.z = sin(pitch)·speed** (it does not keep vel.z). The walk accel cut (stick < 0.8 and
  residual > 30°) compares the **signed** residual, so only left turns cut it. Entering 2 keeps the jump's speed and zeroes
  it after a > 90° turn only when coming from stop 3 / hurt group 7. In the fall group with a small stick there is no
  AirAccel: |vel.xy| decays by `0x13fc94` = 7·dt² per tick (0x235150; the decompiler drops it). Double jump: gravity is also
  applied during the 8 boost ticks. The ground states call 0x233850 and the edge brake before StickTarget; the edge brake is
  skipped with stick > 0.5 and zeroes vel, disp, eff, momentum and speed. 9 has frames 18/29/11 and no ×1.17 descent
  gravity; 0xe's anim is 0x16 fr 3 blend 7 at speed 0.8; 9's is seq 8 or 9 by run phase (table 0x17c258). On a
  slippery floor (the superset build, L00 0x217970) the jump windup branches on 0x13f7f8 (keep_speed, from 0x140632 at
  the entry): it runs the slide move 0x2167d0 instead of SetPlanarVel (and copies the velocity into 0x13f480), so the
  slide's momentum carries into the jump.
- §5: 0x23c458 is a wrapper around the move 0x233de0 (+ periodic probes). The clamp is `r_current − 0.02` on the **3-D**
  velocity and rewrites vel — so the effective terminal fall speed is **0.43 u/tick (25.8 u/s)**; the fall state's −50·dt
  cap never binds. The capsule resolve is not bit-neutral (`pos = pushed − off`). The probe starts 2.9·r up in the fall
  group; grounded also in gravity mode 1 / group 0x16. Snap rule: shallow (depth < |vz| + 0.01) → snap unless the capsule
  touched within 0.5·r, else spring (0.057, 0.3, 2·dt) after undoing a downward disp. eff comes from the pre-platform
  displacement; the 52·dt cap scales only 0x13f450/0x13f4b0. 0x23c710 also has the death plane pos.z < 1.0.
- §6: SetAnim always snapshots (no t > 0.025 test), does not clamp the frame and swaps only 0x31↔0x32 with the mirror.
  Curves: −1 = 11 ticks, −2 = 16, −3 = 3. Ratchet's advance keeps the rate in 0x13fde4 (frame A's rate word), snaps t in
  (0.99, 1.01) → 1 and |t| < 0.01 → 0, steps keys while t ≥ 1, has a loop range (crouch 3..18, re-entry at 1/3 rate) and
  flags in 0x13fde8 (bit 1 = wrapped). Seqs 5 and 6 are both skids, 10 the fall start, 8/9 running jumps.
- §7: the look point is the **raw** target + up·1.5; only the final position uses the springed target
  (`pos = S + up·pivotH + placed`). The leash pulls toward last tick's pivot + offset and clamps to [dist − reduction,
  dist]. The ceiling lines go up and down 20 from hero + 0.5; h = 0.95; midpoint if the gap < 1.9. The yaw input is
  1.3°·rx through a k = 1, d = 1 filter; the pitch target moves ≤ 0.02/tick (0.01 back toward centre) and the elevation
  follows ≤ 1.75°/tick. The 0.075 distance reduction needs |rx| > 0.05 and a mostly horizontal hit, and sets the recovery
  timer to 2180; recovery multiplies the reduction by the cosine curve every tick; **the minimum distance is 1.5**. The
  placement (0.02, 0.2) speeds up and pulls back to ≤ 6.0 when the hero runs toward the camera (dot ≤ −0.3). The rows
  smooth the heights at (0.004, 0.2); the low-camera bias springs at (0.005, 0.2); pitch clamp ±70°. The parameter
  spring-back is 0.003 (distance, pivot height) and 0.005 (look height). 0x3111d8 is code and does nothing for these
  states; per-state behaviour lives in the target modes (jumps hold the take-off height, 0xb raises 1.0 over 35 ticks,
  falls use mode 1) and the 0xb/0xc rigid follow. Option defaults 0x15eddc = 0x15ede0 = 0x15ede4 = 1. `FastVecCross(a, b)`
  returns b × a.

## 11. Engine wiring (`crates/rc-engine`: `gameplay.rs`, `input_map.rs`, `play_camera.rs`)

Novalis is playable with keyboard or gamepad (`cargo dev`; `RC_PLAY=0` keeps the old fly-camera-only viewer, pixel-identical
to before this wiring at frame 120).

- **Input** (`input_map.rs`): every frame the devices become the 18 libpad2 bytes (`PadInput`), so `PadState` decodes exactly
  what a DualShock would send. Left stick: WASD / arrows (full deflection, bytes 0/255; Shift = half = walk) or the gamepad
  stick scaled per axis by 1.33 and clamped (`STICK_SCALE`, PCSX2's DualShock 2 default: a full diagonal reads full on both
  axes as on a DualShock 2; unscaled, a round-gate pad's full diagonal decoded to length 0.80, under the walk / run 0.82),
  then `round(127.5 + 127.5·v)`, y flipped; the game's own dead zone 48/76 applies. ✕ Space /
  South, □ J / West, ○ K / East, △ L / North, L1 Q / left bumper, R1 E or Ctrl / right bumper, L2 Z, R2 C (triggers),
  Start Enter, Select Backspace, R3 V or middle mouse / right stick click, d-pad T/F/G/H. Right stick: mouse motion while the
  right button is held (12 px per frame = full), `,` / `.`, or the gamepad right stick. Buttons are digital (pressure 0xff).
  Gamepads come from Bevy's default `bevy_gilrs` feature (on in Bevy 0.19's defaults; nothing enabled here).
- **Scripted pad** `RC_PLAY_SCRIPT="a-b:action,..."`: per gameplay tick t (0-based; in frame-exact mode tick t runs in update t+1,
  so `RC_SCREENSHOT_FRAME=N` shows the state after ticks 0..N−1); actions `stick x y` (x right, y down: `stick 0 -1` =
  forward), `rstick x y`, `press X+R1…`. Devices are ignored while a script is set. `RC_PLAY_TRACE=1` prints one line per tick.
- **Load**: `LoadedLevel` now carries the collision mesh (`parse_collision`) and `death_z` (level settings +0x28; 0.0 on
  Novalis). The `MobyTable` is the gameplay file's instances (static part, `init_instance` with a `ClassInfo` built from each
  class header: +0x06/+0x0c/+0x0e/+0x0f, collision, scale, glow, mode bits, sequence 0; update function unknown → `None`)
  plus 256 dynamic slots, the loader's ship created in the first. Hero = the class-0 instance; `Game::new` snaps him to
  (162.530, 136.393, 60.000), yaw 1.9873. The game state of a direct boot (`rc_game::game_state`: new game → Veldin start +
  Clank init → transition → Novalis start) gives the options (`Options::game_options`), the mirrored-animation option and
  HP 4; it is published as the `Persistent` / `Session` resources.
- **Tick** (`FixedUpdate`, 60 Hz): `Game::tick` = pad → mobys hook (no-op until the moby scheduler) → hero → particles hook
  (the existing `particle_render` tick: emitters + `UpdateParts`; its own system is then skipped) → camera → counter. Ticks per
  rendered frame: `ticks_for_frame(RCNT1 of the last frame)` = at most 2 (frame-exact mode: exactly 1); more fixed steps than
  that are dropped, i.e. the game slows down below 30 fps like the PS2.
- **Ratchet's instance**: drawn as an extra instance of his class (`moby_render::ExtraMobys`, own record + palette) from his moby
  +0x10 / +0xc0 rows after the write-back, re-lit with his light word / ambient for the new rows; his gameplay-instance entities
  are hidden (the static record cannot move, and the static occlusion word of his spawn cell would cull him elsewhere). His
  `RatchetAnim` state/snapshot is copied into his `MobyAnim` instance (`skip_advance`, so the generic advance never runs on him)
  and his rows/position into `MobyAttach::set_host`: the wrench, pack and Clank follow the pose and the position.
- **Camera** (`play_camera.rs`): `CameraView` → Bevy transform (`game_to_bevy(pos)`, looking along row 0 with row 2 up) in
  `RunFixedMainLoop` after the fixed loop, so every reader sees this frame's camera; same `GameProjection` (tan 0.63; the hook
  rewrites it only if the game asks for another tangent), letterbox and fog. `CameraSource` (game_camera.rs): Tab switches
  game ↔ fly camera (the fly camera resumes from the current view; the pad is neutral while flying); `P` prints the view.
- **Death**: `OutOfBounds`, or the hero entering the unported death states 0x77 (below death_z) / 0x3d → respawn at the uid-0
  moby with the hero block, pad and camera rebuilt as at load and `SessionState::hero_init` (HP = max); tick counter and RNG
  continue. The game's fade / checkpoint sequence is not reversed. `R` respawns on demand (e.g. from a frozen unported state).
- **Checks** (frame-exact, bit-identical on repeat): `0-179:stick 0 -1` runs 9.09 u in 120 ticks along the camera yaw (as
  `hero_novalis.rs`), seqs 0 → 3 → 4 (run), leaves the ground at tick ~131 off a ledge (state 6 at tick 136, fall seq 11) and is still falling at
  tick 179 (z 51.7), the camera held on the ledge above; `30-60:press X` from idle: state 7, seq 7, apex +2.0544 at tick 58.
- **Not yet**: the moby update hook (scheduler), moby collision / platforms, the pack/Clank idle rule gated on hero state 0
  (`moby_attach` applies it in every state), Clank hidden from `SessionState::clank_hidden`, the death fade and checkpoints.


## 12. Melee and item swap (wrench combo, jump attack, hand slot) — 2026-09-26

Level01 addresses; H = read from the disassembly, M = decompiler C checked against it, L = inferred. Port:
`crates/rc-game/src/hero/melee.rs`, `hero/items.rs`; engine: `gameplay.rs`, `moby_attach.rs`, `hud_render.rs`.

### 12.1 Trigger: `HeroPdaGadget` 0x240ed8 (top of the transitions, after the HP check) — H
Returns "state changed" (`timer < timer at entry`); the transitions then stop. Needs `0x1413f4 == 0`, a hand moby
(`0x1403e0`), slot state `0x140404 == 2`; with `0x1413fc` only for the wrench in gravity mode 1. Item 8 (wrench, fire mask
`0x1403f0 = □ 0x80`):
- **Ground** (group < 2, or group 4 after landing): □ within 7 ticks, the hand moby is class 0x47 and in hand (+0x20 = 0),
  not on a special floor (0x14063a) → **0x13** (0x70 in gravity mode 1); crouch held → **0x15** (comet strike); in the strafe
  stance (1/0x1e) after 20 ticks → `0x236da0` (not ported).
- **Air**: states 7/9/8/6/0x2d/0x11, 0xe after frame 32, 0xb after frame 18 (20 for the back flip), timer > 10: □ within 10
  ticks (18 for 0xb/0xe) → **0x14** when the height above ground > 0.1 (0.57 once descending), else 0x13.
- **Combo** (group 6, not blending, frame ≥ row.chain-from): □ within `clamp((int(frame) − row.ref)·2, 1, 15)` ticks → 0x13
  (the next swing is chosen by SetState), crouch → 0x15.

### 12.2 The melee table 0x17c0a8 (stride 0x2c, row = 0x13fdb0) — H (data)
| row | use | step | chain-before | ref | chain-from | jump-after | idle-after | hit | 
|---|---|---|---|---|---|---|---|---|
| 0 | swing 1, seq 0x17 | 0 | 33 | 19 | 26 | 24 | 31 | 17–23 |
| 1 | swing 2, seq 0x18 | 1 | 25 | 7 | 12 | 13 | 23 | 8–13 |
| 2 | swing 3, seq 0x19 | 2 | 0 | 16 | 23 | 19 | 21 | 12–16 |
| 3 | comet 0x15, seq 0x1a | – | 99 | 83 | 88 | 87 | 91 | – |
| 4 | jump attack 0x14, seq 0x2b | – | 99 | 32 | 28 | 28 | 30 | 1–28 |

Frames are Ratchet's readout 0x13fdf8. Swing 3's idle-after (21) precedes its chain-from (23): the combo always ends in idle
after swing 3, and □ there starts swing 1.

### 12.3 SetState 0x23cf98, group 6 (0x13, 0x14, 0x15, 0x20, 0x23, 0x51) — H
Common: group 6, `0x13fdbc = 1`, `0x1415d4 = 0`, aim `0x13fda8`/target `0x13fda4`/hit flag `0x13fdb4` cleared.
- **0x13**: `0x141440 = 0`; next step `(row.step + 1) % 3` when the previous group was 6, the row is a combo row and frame <
  row.chain-before, else 0; aim `0x2351d0(5, π/4, π/4)`; SetAnim(blend 5 (step 0) / 7, seq 0x17 + step, frame 0/1/2) with
  `trunc(1.5·|aim − yaw|)` more blend ticks when aimed and step < 2; the wrench blends to its seq 3 + step (same frame and
  ticks). (Stats records 0x1416c0.. not ported.)
- **0x14**: row 4, `0x1415d4 = 0x50`, aim `(8, 60°, −1)`, `0x13fdc0 = 0`, swap lockout `0x1403f8 = 10`; SetAnim(11, or 17
  right after an early back flip, seq 0x2b, frame 4), wrench seq 10 frame 4 over blend + 2.
- **0x15**: row 3, aim `(14, 30°, −1)`, `0x13fdcc` toggled, SetAnim(10 / 3 / 17, 0x1a, 0), loop 6..21 (entry ported; the
  state is not).
- Aim `0x2351d0`: unless already aimed, the aim yaw 0x13fdac is the facing, or (stick > 0.5) the stick direction with the aim
  flag set; then the target search `0x22e238` over the targetable list (mode 0x1000 with a mode-0x20 record ≠ 0): **none in
  the port** (no ported class has the record).

### 12.4 Physics 0x2370b8 — H
- **0x13** (0x239780): `0x1403f8 = 2` while blending or frame < hit-to; target speed = (dt·4.4)·k (step 0, frame < 18), (dt·{5.7, 4.5
  after frame 7, 3.0 after 8})·k (step 1, frame < 9 or blending), (dt·3.7)·k (step 2, frame < 12 or blending), else 0;
  playback speed → 1 by 0.2/tick; aimed: turn toward the aim (0.05, 0.2, 15.01 rad/s·dt), step 1 turning right slows the
  anim to 1/(Δ + 1); not aimed: aim again (except step 1 after frame 7); SpeedStep(37·dt², 28·dt²); SetPlanarVel(aim or
  facing); gravity 54·dt² on the ground / eff.z − 25·dt² in the air; edge brake (3.7, 0).
- **0x14** (0x2390b8): `0x1403f8 = 2` while blending or frame < 30; SpeedStep(15·dt², 15·dt²), |vel.xy| = speed; aim until
  frame 25; vertical: timer < 9 → vz approaches dt·7.7 (×0.5 above 2.25, ×0.7 above 1.8) by 150·dt²; 9..14 unchanged; then
  vz −= 130·dt², ≥ −37·dt, `0x13fdc0 = 1` once falling. Past frame 23.5 while falling: the **shockwave** `coll_sphere_mobys(0.4,
  feet − 0.5, 0x10, Ratchet, {facing·2, Ratchet, 0x10000, damage 2, class 0x47})`.
- Transitions (0x243e44): 0x14 playback speed 0.33 while rising, 2/1 (frame < 26) on the ground, else the landing-ETA fit
  `0x22a620(26, eta(60, 130·dt², −1), 0.5, −1)` (≥ 0.75 below 0.7); then after frame jump-after and not blending ✕ within
  `(int(frame) − ref)·2` → jump 7 (crouch: `0x242420`); not blending and frame > idle-after → SetState(0, no anim).

### 12.5 The hit: the wrench moby's update 0x2be1c0 (in hand) — H
Called from the slot loop every tick (the moby's +0x74). `0x1403fc = 0`; slot state 3 → return. Wrench sequence upkeep: a
wrapped sequence ≠ 1 → seq 1 over 5 ticks; leaving 0x13/0x14/0x21/0x2b/0x70 on seqs 3..5 → seq 1 over 4; the wrench plays at
Ratchet's speed 0x13fde0. In group 6 (or 0x2b):
- **hit window** = not blending and frame in [row.hit-from, row.hit-to];
- last tick's points kept (0x13fd60/70); **head** = wrench joint list 1, **hand** = Ratchet joint list 0 (`0x2645a8`: partial
  pose, `P.r3 · scale/1024`, rows, + position); aim = facing, in 0x13 the direction to the head ± 90° (−90° on step 1);
- head := hand + setlen(head − hand, |…| + 0.17), shortened by `0x2be110` (reach × (0.7 + 0.3·(105° − min(Δ, 105°))/105°));
  a head more than 90° off the facing closes the window (not in 0x14);
- template `0x26e808`: dir = (cos aim, sin aim)·(1 | 1.55 in 0x14), z = 1, w = 5627.9; attacker the wrench moby; flags
  **0x10000**; damage **1** (2 in 0x14); bytes 0/1, class 0x47; +0x20 = 1;
- `0x26ebe8`: 5 `CollLine_Fix(…, flags 0, Ratchet, tmpl)` between last tick's and this tick's hand → head (first hit
  ends it); then `coll_sphere_mobys(0.35 (0.7 group 0xf, 0.47 in 0x14), head − 0.085 along the segment, 0, Ratchet, tmpl)`;
- the first contact of a swing sets 0x13fdb4 and plays the wrench's sound (`0x2bda88` picks 0/1 from the target record) with
  sparks `0x2bdd20`; a target record with flag 2 (`0x2bdad8`) sends Ratchet to the rebound 0x21 (unreachable in the port).
Crates take the hit through their mask 0x1830000 (0x10000 ⊂ it); 502 needs 0x20000, so the wrench does not break it.

### 12.6 The hand slot (0x1403e0 + 0x50·slot) and the swap — H
Record: +0 moby, +0x10 fire mask, +0x14 creation gate (frame), +0x18 lockout timer (s16), +0x1a detached, +0x1b put-away
counter, +0x1c swap state, +0x1d reload, +0x20 ticks ready, +0x24 state (2 ready, 3 put away), +0x28 item id (0x140408).
- **Create** 0x22f3c0: empty slot, gate < `0x15f3f8` (frame counter), request 0x141408 ∈ {0, 0x24}: item = target 0x141424,
  else 8 while 0x15ed90, else the saved hand item 0x141660, else 8; `CreateMoby(def.o_class)`; state 2; wrench: mask □, seq 1
  over 1 tick; others mask ○.
- **Attach** 0x22fec0: position = W.r3 of list `def.attach` (wrench list 0, bomb glove list 6); not a glove: advance, rows = W,
  normalised; glove (10/17/20/25): rows = W, no advance, pose = `HeroItemPoseFromRatchet` 0x22a9c8 (joint 0 identity, joints
  1..16 = Ratchet's local pose of joints 55..70 from table 0x17aa40, inherited scales, translations differing from the glove's
  rest) shown as frame A = B, t = 0.
- **Slot loop** 0x231088: empty → `0x2307e0(slot)`; state 2 → ready ticks++, `0x2307e0`, a wrapped seq 0 → seq 1 over 2;
  state 3 → `0x2305e8(slot, frame + 2)` at once for the hand (delete, state 0, id 0, gate); then the moby's update.
- **Swap** `0x2307e0(0)`: □ pressed with another item → target 8 (the item saved in 0x141440 when 0x1413f6); ○ pressed with the
  wrench → target = saved item; request 0x141408 ≠ 0 → target = request, saved = request (previous 0x15ed8c = saved unless the
  wrench is held); request == target → request cleared. A change needs swap state ≠ 2 and the lockout timer (0x1403f8,
  `FastDecTimer`) out: ready ticks = 3, `0x22b8e8`, fidget timer `rand_range(50, 90)` (**one game RNG draw**), per-item
  timers 0x13f52a/0x13f52c (items 9, 0xb, …), 0x15ed90 = (target == 8), request cleared, the item blends to seq 2 over 2
  ticks, slot state 3.
- Timeline (1 tick per frame, the gate compared with the tick counter in the port): tick N swap starts, N+1 deleted (gate N+3),
  N+4 the new item. The ring repeats its confirm during its 8-tick fade-out (menus.md §2), and the create waits for the
  request to be cleared (request == target), so after a ring swap the new item appears ~8 ticks later (seen: request at 101,
  wrench away 102, glove 110).

### 12.7 In the port
- `hero::melee`: 0x240ed8 (item 8), the group-6 entries, physics 0x13/0x14, transitions 0x13/0x14, the aim (no targets), the
  wrench update's hand part (sequence upkeep + hit test), the shockwave. `state::implemented` now includes 0x13 and 0x14.
- `hero::items`: slot 0, create / attach (incl. the glove pose) / slot loop / swap / delete; the item moby lives in the hero
  block (`HeroItems::slot.item`: class, anim state + snapshot, rows, position), **not** in the moby table (L: no effect on the
  moby slots the scheduler uses, but the game's dynamic-slot indices differ). A `MobyAnimBlend` made by SetState is applied at
  the start of the item update (nothing reads the item's animation in between).
- `tick::Game::tick_with_hits(…, &mut dyn HitSink)`; `Game::item_data` (definitions from the overlay's item table, the 21
  gadget classes and their joint lists, Ratchet's lists), `Game::item_globals` (request / saved / previous / wrench flag, synced
  by `gameplay.rs` with `Session::temp_hand` and `Persistent` `equipped[0]`, `last_hand_item`, `wrench_held`).
- `moby_update::services::ServiceHits` (hit plumbing): `coll_sphere_mobys` 0x214468 and `CollLine_Fix`'s moby pass are the
  game's kernels (grid walk, bounding sphere, class blob triangles and primitives, pose cache; collision_queries.md §7), with the
  hit records for mode-0x4000 mobys and `deliver_hit_in` (0x26e968).
- `AnimView::frame_step` (0x13fdfc), `AnimCtl::eval_chains` / `pose_frame` (Ratchet's partial pose and local-pose keyframe).
- Engine: `moby_attach` draws the game's hand item (wrench 71 or bomb glove 192, shown / hidden by the slot), the back items as
  before; `HeldWeapon` + `Persistent` / `Session` feed the HUD (bolts 0x15ed98, HP 0x1415f8 / max 0x15eda0, the weapon slot of
  the held item when its record has ammo: the glove shows 10/40, the wrench nothing). `RC_PLAY_TRACE` adds hand id:state,
  (seq, frame, position), combo row, hit flag, frame readout and the wrench head.
- Tests (`hero::melee` tests): □ starts swing 1 (seq 0x17, blend 5, wrench seq 3); chaining in the window (buffered press);
  a late press does not chain and the combo ends in idle after frame 31; swing 3 ends in idle and □ restarts swing 1; hit frames
  per row; □ in a jump → 0x14 (lockout 10, seq 0x2b fr 4), lands and returns to idle; the swap timeline wrench → glove
  (N put away, N+1 deleted, N+4 glove, fire mask ○) and □ back to the wrench.
- Checks (frame-exact, `RC_SCENE=0`): Novalis script `0-5:rstick 1 0,20-226:stick 0 -1,250-310:rstick 1 0,330-331:press SQUARE`
  — Ratchet drops to the crate pair at (151.1, 154.5, 40/41), swing 1 hits at tick 341, the top crate breaks (debris + bolts,
  22 dynamic mobys), the bolts fly in from tick ~400 (2 → 10 at tick 430) and the HUD counter shows; ring script
  `60-100:press TRIANGLE,70-100:stick 0 -1` swaps to the bomb glove (hand 8 → none at 103 → 10 at 110), HUD slot 10/40; two runs
  of a combined 600-tick script give identical `RC_PLAY_TRACE` output and identical PNGs.
- Not ported: comet strike 0x15 (state and the wrench's thrown states 10/11), rebound 0x21, 0x20/0x23/0x51/0x70/0x2b, the strafe
  wrench move `0x236da0`, target search and the target-distance speed rule, the wall-spark line and the 0x14 ground sparks, wrench
  sounds (counted in `HeroItems::hit_sounds`), the trail counters, stats records, the bomb glove's own update (its ○ throw), the
  other item slots, the hand joint-modifier records (0x140c40, `FUN_00227050`).

## 13. Idle behaviour (fidgets, head look, secondaries, blinks, Clank on the back) — 2026-09-27

Port: `crates/rc-game/src/hero/idle.rs`. Sources: level01 disassembly of every routine below; checked against the
two Novalis savestates (trace_results_novalis.md "Second savestate"). Confidence **H** for control flow, constants
and draw order (read from the instructions), **H** for the fidget timing (see 13.6).

### 13.1 Where it runs (0x228870, mode 0)
advance 0x247d48 (→ **0x247800**) → physics → surface → transitions 0x242930 (state 0 → **fidget / return /
Clank fidget**) → 0x236860, 0x22c5c0, 0x22b700 → **0x22b928 head look** → HeroScanTargets → **0x22bdd0 secondaries**
→ **0x227590** (record 12, **Ratchet's blink 0x2274e8**) → **0x2278c0** (Clank's glow + **blink**) → 0x2352e0 →
**0x2273d0 joint springs** → HeroSyncMoby → … → HeroItemsUpdate (**back items created / advanced**). Timers:
`HeroTickStateTimer` 0x23c710 decrements the fidget cooldown 0x13f538 (s16) and the five record cooldowns +0x60.
`FastDecTimer` (0x220e78 / 0x220ea8) returns non-zero while the timer is 0 (1) or when it reaches 0 (2).

### 13.2 Fidgets (0x242930 state 0 + 0x241e00)
* Item 0x1b in hand: back to the idle sequence (curve −2), no fidget. Back slot ready (0x1404f4 = 2): a wrapped
  pack / Clank not on sequence 1 blends to 1 (7 ticks); head record angles 0x17ad58 > 0.611 or 0x17ad54 > 0.436 →
  both to 1 (12 ticks).
* Ratchet on the idle sequence: `0x241e00`. Cooldown 0x13f538 ≠ 0 → nothing. Candidates = records 0x179d10 + k·0x70
  with +0x60 = 0, +0x58 ≠ 0, item +0x50 ∈ {−1, held}, state records only off mobys; k = 3 only on level 0xc (item ≠
  0x17, line body → body + 8z clear); k = 4 only within 50 ticks of group 1 at health ≤ 1. Sum of `1/(int)(s·60)`, one
  `randf(0, 1)`; below the sum, the record is the first whose running remainder drops below the draw. Health 1: record
  4 with `randf < 0.33` on a wrap. Records: (seq 1, 10 s, cd 5.5 s), (seq 2, 10 s, 5.5 s), (state 0x40, 0 s),
  (seq 0x53, 8 s, 5 s), (seq 0x5a, 4 s, 2.5 s). On Novalis records 0 and 1: 1/600 each per tick.
* Start: 0x1415c0 = k, 0x13f538 = 120, +0x60 = (int)(cd·60) = 330, `SetAnim(12, seq, 0)` (or SetState(0x40)).
* Otherwise, on a wrap (0x13fde8 & 2): `SetAnim(−2, idle, 0)` (0x54 at health 1: 18 ticks).
* Clank's fidget: timer > 10, Clank on sequence 1, `FastDecTimer(0x141614)`, head angles below 0.559 / 0.349, slot
  ready → 0x141614 = `rand_range(110, 270)`, `0x2473e0`: pack on 1 → a random row of Ratchet sequence 0 in the back
  table (one draw), pack / Clank blend to it over 7 ticks.

### 13.3 The back table (0x2476d0) and the SetAnim / advance hooks
Table by back item (`0x22ddd8(3)`): 2 → 0x17c070 {(0: 4, 11), (0: 3, 10), (0: 7, 14), (21: 5, 12), (18: 8, 15)}
(Ratchet seq: pack, Clank). With n rows for a sequence, `rand_range(0, n − 1)` — **one draw even for n = 1**.
* `0x247800` (end of every advance, mode 0): rows for Ratchet's sequence B → one draw, pack / Clank speed =
  0x13fde0. **One draw per idle tick on Novalis.**
* `0x247550` (every `SetAnim` in mode 0, blend in ticks = the blend or the curve length 11 / 16): rows and seq ≠ 0 →
  blend both to the row (unless the pack plays it); else back to sequence 1 (7 ticks; 19 after state 8 with pack 2),
  unless group 0 and the pack plays a sequence-0 row. **The blend back to sequence 0 draws once.**
* The back mobys are created by `HeroItemsCreate` on the first hero update (pack of item 2 = class 607, Clank 601)
  and advanced by `HeroItemsAttach` (generic `MobyAnimAdvance`).

### 13.4 Head look 0x22b928, secondaries 0x22bdd0
* Record 17 scale = 0x15ee14, which approaches 0.92 by ≤ 0.05. State 0, sequence 0..2 or 0x3a, no look target
  (0x1415c4): in sequences 1/2 **0x140360 = rand_range(40, 70) every tick** (so it never expires); `FastDecTimer
  (0x140360)` → pitch 0x140354 = −randf(−0.157, 0.436), yaw 0x140358 = randf_sym(0.349, 0.995), k = randf(0, 1),
  spring k / d = lerp(0.015, 0.007, k) / lerp(0.29, 0.3, k), timer = rand_range(90, 200) + (int)(80k) (**4
  draws**). Sequences 1/2 clamp yaw to ±0.349, pitch to 0..0.262. Record 3 (joint 4) gets pitch / yaw, record 1
  (joint 10) ×0.55 / ×0.52. 0x140360 is this timer (not a fidget countdown): HeroInit `rand_range(180, 300)`,
  SetState(0) `rand_range(50, 100)` then `0x22b8e8` clears the look and the secondaries.
* 0x22bdd0 (state 0, sequence 0): timers 0x1403b0..bc; expiry → record 13 z randf_sym(0.175, 0.524) + (70, 150);
  14 z same + (40, 90); 15 z randf_sym(0.262, 0.873) + (40, 90); 16 y randf(−0.087, 0.524), z randf_sym(0.349,
  0.96) with the sign forced to alternate, + (35, 70). 2 / 2 / 2 / 3 draws.

### 13.5 Blinks, glow, joint springs
* Ratchet (0x2274e8): period 0x140348 = 0x68 after SetState of 0, 2, 4, 6 (0 for 3, the jumps, melee). Period 0:
  next = counter + 30. Else when next < counter (0x15f5cc): frame 0x140340 = 1, next = counter + 20 + randi(p) +
  randi(p) (**2 draws**). The frame then runs 2..11 (7 eyelid manipulators, values 0x17c610).
* Clank (0x2278c0, on Clank's moby in mode 0): colour +0x90 = 0x80 | b 0x4c+⌊24s⌋ | g 0xa0+⌊48s⌋ | r 0x38(0x88 at
  health 1)+12+⌊24s⌋, s = fast_sin(2π·(counter mod 110)/110 − π). Blink: `FastDecTimer(0x14034c)` with Clank on
  sequence 1 → 0x14034c = rand_range(50, 200) (**1 draw**), frame 0x14034e 1..21 (values 0x17c720).
* 0x2273d0: records 0x17ab00 + 0xb0·r; in mode 0 kinds 0 / 1 (Ratchet / Clank). Anything set (targets, scale ≠
  1, |angle| ≥ 0.005) → `0x270b58(target, k, d, 0, &angle, &vel, 2)` per axis, attach; else detach. Targets and
  scale reset. Record 3's angles (0x17ad54 / 58) gate Clank's fidget and return (13.2).

### 13.6 Checks
* Sequence 1 and 2 both wrap **188 ticks** after `SetAnim(12, seq, 0)` on the disc data: slot 1 (record 0 cooldown
  128 = 330 − 202, sequence timer 14, t = 0.97 = curve −2 step 14) and slot 2 (sequence 2 after 100 ticks on
  frames 32 → 33, t = 0; cooldown 20, record 1 cooldown 230) agree (`tests/hero/hero_novalis.rs`).
* Hero-only replay of 3600 idle ticks from the load pass: 1.87 draws / tick, 1887 in ticks 2439..3511
  (the report's estimate of ≈ 1200 missed the back table and the SetAnim draw).

### 13.7 In the port / not ported
`Hero::idle` (these fields), `Hero::back` (created only once `Hero::set_back_classes(607, 601)` is called),
`Hero::idle.counter` (0x15f5cc mirror; the tick driver should set it), `Hero::idle.level` (0x15ed84; also
selects the ground probe's water footstep class 3 on levels 1 and 0x12). `Hero::set_anim` takes the RNG.
Not ported: the edge look-down of 0x22b928 (0x13f5b0), HeroScanTargets (look targets), ~~HeroLean (walk)~~ (ported: `Hero::lean`, hero_gameplay.md §7),
the manipulators' pose application (the angles are computed, not rendered), the sound triggers of the advances
(PlayClassSound pitch draws), options 0x15edb1/3/5, Clank hidden 0x141628, the hit flash 0x13f53e.

## 14. Swimming (surface, underwater, Hydro-Pack, wading, drowning) — 2026-09-26

Level01 addresses; H = read from the disassembly, M = decompiler C checked against it. Port: `crates/rc-game/src/hero/swim.rs`
(+ the jump / walk entries in `hero/states.rs`, the curve and probe in `hero/physics.rs`), camera tweak in `follow_camera.rs`,
engine wiring in `rc-engine/src/gameplay.rs`. The brief's assumption "no dive without the pack" is wrong: R&C1 lets Ratchet
dive anywhere (0x33, slow, 15 s of air); the Hydro-Pack adds the thrust state 0x35; the O2 mask removes the air limit.

### 14.1 States (SetState 0x23cf98 entries) — H
| id | group | entry | anim |
|---|---|---|---|
| **0x37** tread water | 0x12 | 0x1413f7 = 1 (wrench kept); from under water: splash (3 rings, 10 drops) + `RippleDisturb(x, y, 0.4, 0.3)`, gasp voices (air < 2500 → 8 after 30; < 7500 → 7 after 27 and 40); from above with disp.z < −0.5·dt: splash (3, min(300·|dz|, 40), big) + ripple (0.5, −0.4), splash timer 0x13fc5a = 75. Bob: vel 0x13f9e4 = max(0.37·disp.z, −7·dt), level 0x13f9ec = W, off 0x13f9e0 = z − W + 0.12; momentum = (eff.xy, 0) clamped to 5.5·dt; stroke timer 0x13f9e8 = 0 | 0x3a, 22 |
| **0x36** surface swim | 0x12 | stroke timer = 45, 0x13fc4c = 0 | 0x65, 17 |
| **0x33** stroke | 0x11 | 0x15d4 = 0xb, 0x13fc4c = 0; from the surface: sound 3 + substate 1 (dive-in); else 0x13fc44 = 45; 0x13fc30 = 0 | 0x3b, 12 |
| **0x34** drift | 0x11 | momentum = eff (3-D) | 0x3c, 35 (curve −2 after 0x35) |
| **0x35** Hydro-Pack | 0x11 | 0x13fc44 = 60; from the surface: sound 3 + substate 1 | 0x3d, 11 (15 after 0x33) |
| **0x12** water jump | 4 | jump block (§4.4): h 0.1..0.2 over 10, takeoff 2, frames 18/28/14, bottom 0.6; depth 0x1415f4 < 1.25: takeoff 15, h 1.95..1.98 (a normal jump); else vz = 0 and the **scripted curve** 0x13f76c: window [2, 37), table 0x17c3e0 rows (accel·dt² at the row start, increment, ticks) = (−27, 9) (40, 1) (55, 1) (75, 1) (90.5, 9) (75, 4) (40, 3) (0, 100) added to vz each tick on top of g = 29.7·dt²; 0x13f728 = 0, 0x13f72c = 18, 0x13f730 = 40 | 0x72, curve −1, frame 1 |
| **0x73** wade | 1 | the walk entry (state 2's case); anim even without playAnim | 0x60, 8 |
| **0x6a** drowned | 0x14 | health 0 (locks SetState), momentum = eff, stats (not ported) | 0x5f, 10 |

SetState's prologue clears 0x1413f7 / 0x1413fc (mode 0) and its tail sets the restore request 0x14145c when 0x1413f7 went 1 → 0
(the gadget comes back after the water). Now in the port's `set_state`.

### 14.2 Entry / exit checks (top of the transitions, after the weapon check) — H
- **`0x2408e8` enter** (not in 0x6a; needs the in-water flag 0x140634 from the surface reaction):
  - group 0x11: substate ≠ 1, disp.z > 0 and z > W − 0.4 → **0x37** (surfacing);
  - groups other than 0x12 / 3: `|W − (z + 0.45)| < max(0.2, |vz| + 0.07)`, depth > 0.8, disp.z < 0 (a fall reaching the level), or
    z < W − 0.8 (feet 0.8 under); group 4 only once descending → **0x37** + sound 3;
  - 0x12 descending and z < W − 0.7 → 0x37 (falls back in);
  - outside groups 0x11/0x12/7/0x14 and state 0x12, every 16 ticks (tick counter & 0xf): a line from z + 4 (z + 16 every 64 ticks)
    down to z + 1.3 (flags 2) hits water, and the same line started 0.01 under that water hits nothing → **0x34** (standing
    submerged, e.g. walking down into a pool).
- **`0x2406b0`**: group 0x11: air 0x1415f0 −= 10000 / (60·15) = 11 per tick unless the **O2 mask** (0x13d4c6); at 0 → **0x6a**
  unless z > W − 2, pitch 0x13f3e4 < −50° and disp.z > dt; z > W + 0.4 → **6** (and the underwater flag 0x167494 = 0).
  Elsewhere (not 0x6a / 0x76) the air refills to 10000. Group 0x12: a line from W + 0.01 to W + 0.31 hitting anything
  (a low ceiling) → dive (0x35 with the pack and R1|R2 held, else 0x33).
- Pack / mask ownership: the game state's item table 0x13d4c0 + id (**4 = Hydro-Pack, 6 = O2 mask**).

### 14.3 Physics 0x2370b8 — H/M
- **0x36 / 0x37** (0x2378xx): anim speed 0.6; StickTarget(3·dt); a held direction floors the target at 1.5·dt; TurnTo(0.007, 0.08,
  300°/s); 0x36: no direction during the 45-tick stroke window → 3·dt; residual > 45° → 0; SpeedStep(4, 5)·dt²; vel = speed ×
  `0x22a718(0x17c440, 30)` × 5.5 along the yaw, vz = 0; 0x37: speed = vel = 0. Then momentum decay 4.2·dt², wall check (0), and
  **the bob `0x240c78`**: when timer > 10, off < 0, or coming from group 0x11: 0x36 springs off → 0 (0.03, 0.3, 1.5·dt, vel
  0x13fc4c) and zeroes 0x13f9e4; 0x37 is a damped oscillator `v −= 0.005·off + 0.045·v` (v = 0 once |off| < 0.001, v < 1e−4);
  off += v; 0x36 never floats above 0 (off approaches 0 by 0.7·dt); the level springs toward W (0.027, 0.3, no cap; group
  0x16 uses 0x13fbf0); **z = level − 0.12 + off** (written before the move).
- **Stroke curve** `0x22a718(T, n)`: i = trunc(frame 0x13fdf8), f = frac; prev = (i + n − 1) mod n (n − 1 after a wrap);
  `f·T[i] + (0x13fdfc − f)·T[prev]` (0x13fdfc = how far the readout moved this tick). T (0x17c440) = 0.4, 0.4, 0.45, 0.52, 0.6,
  0.68, 0.78, 0.89, 1 ×4, 0.95 … 0.4 (30 entries).
- **0x33 / 0x34 / 0x35**: splash timer; dive-in substate 1 (tick 1: splash + ripple (0.4, −0.3), timer 70; ends after 40 ticks).
  **Pitch** (Euler y 0x13f3e4, spring 0.015 / 0.2, max dt·(40° + 30°·max(✕, □ pressure))): □ without ✕ (or the dive-in) → +78° ×
  clamp(1.5·pressure, 0.2, 1) (≥ 0.35 in the dive-in) = **down**; ✕ → −78° × clamp(1.5·pressure, 0.2, 1) (full when air < 1500) =
  **up**; 0x34 → 0. (Pad mode ≠ 0x79 ramps by hold time instead of pressure; the port assumes the DualShock 2 mode 0x79.)
  StickTarget(1) + TurnTo(0.007, 0.08, 300°/s) steer the yaw. **Target speed**: dive-in 6·dt; 0x33 3·dt; 0x35 7·dt with R1|R2,
  2.8·dt without; × k (k = stick, 1 with ✕/□ held or the dive-in, ≥ 0.35 (0.4 in 0x35) once stick > 0.15) except 0x35 thrusting.
  SpeedStep (13, 8)·dt² in 0x35, else (7, 4)·dt². **Velocity** `0x277b50(s, yaw, −pitch)`: 0x35 s = speed; 0x33 s = speed ×
  stroke × 5 (≥ 4.5·dt in the 15 ticks after 0x35); 0x34: |momentum| → 0 by 8·dt² (after 0x35) / 4·dt², vel = momentum. No
  gravity or buoyancy: vz comes only from the pitch. Wall check (0); **roll** (Euler x) springs to −7·yawvel·cos(pitch)
  (0.007, 0.17, 70°/s). Group 0x11 collides as a 0.6 sphere at the feet (§5), the probe finds the floor (height, grounded).
- **0x6a**: |momentum| → 0 by 4·dt², vel = momentum, vz = 0x13fc4c (≤ 0) → −1.5·dt by 1.5·dt² (sinks).
- **0x12**: the jump physics (§4.4) with the ramp always on (not only while ✕ is held), the curve above in `0x2345f0`, and in
  the air |vel.xy| ≤ 3.7·dt. **0x73**: the walk (state 2) with the table target `max(0.8·t, 2.5·dt)` (`0x232290`).
- **Straightening `0x236520`** (post-move, not while frozen, gravity mode 0): outside group 0x11/0x15/0x16 and states
  0x25/0x26/0x77/0x31/0x2d/0x2c the Euler x and y spring to 0 (0.015, 0.3, 300°/s; velocities 0x13f3f0/f4): the pitch and
  roll of a dive unwind on the surface.

### 14.4 Transitions 0x242930 — H
- **0x37**: ✕ within 8 (0x13f528 = 0: only water currents set it) and timer > 10 → **0x12**; □ within 7 (or R1|R2 within 7 with
  the pack; 0x13f52e = 0) → dive; wading flag 0x1413f9 and depth < 0.4 → SetState(0, no anim) + idle seq blend 18; stick > 0.2
  and timer > 10 → **0x36**.
- **0x36**: ✕ within 8 → 0x12; □ within 7 → dive; wading flag and depth < 0.7 → SetState(0x73, no anim), seq 0x60 blend 17,
  disp.z = vz = 0; stick < 0.1, speed 0 and the stroke window over → 0x37.
- **0x33 / 0x35**: 0x33 plays at 15 × speed in [0.25, 0.75]; nothing held, frame in (9, 19), not diving in, stick < 0.1 and speed
  < 2·dt → 0x34; with the pack and R1|R2: 0x33 → 0x35 after 20 ticks; 0x35 without R1|R2: stick > 0.5 → 0x33, else (no ✕/□,
  stick < 0.7) → 0x34.
- **0x34**: a wrapped sequence ≠ 0x3c → 0x3c (11); stick > 0.2 or ✕/□ (or R1|R2 with the pack) → dive state.
- **0x73**: shares state 2's case: 2 ↔ 0x73 by the wading flag; no direction and |vel.xy| < 3.2·dt → idle (blend 17); anim
  speed 35·|eff| in [0.6, 1.05].
- **0x12**: the jump group's case; while the curve runs the playback aims at 0x13f728 by the window start, then at the apex frame
  over 0x13f730 − start ticks (`0x22a620`), and the landing / fall-over logic waits for the window's end.
- **0x6a** (death group): sequence wrap → `0x2319b0` (deaths++, fade to black, 0x141401 = 1).

### 14.5 Camera, sound, effects
- Camera `0x3111d8`, group 0x11: look height (D+0xf0) = 0.25 each tick, pivot height → 0.5 at rate 0.003 (`0x313690`). The
  flags of `0x20eb40` (0x34 in groups 0x11/0x12/0x73) were already ported. The underwater flag / fog / tint are the camera's
  own test (world_animation.md §6, engine `fog_state.rs`).
- `SwimEvent`s: `Ripple` (applied by the engine to the 751 patches with `RippleSim::disturb`), `Splash` (a record),
  `Sound` / `Played` (3 splash in, 0x11 landing in water; played at the call point by the hero update), `Voice` (7/8
  gasps), `Drowned`. The particles, splash mobys, breath bubbles, the pack's loop (slot 5) and jets: hero_states.md "Swim
  effects" (since 2026-09-28). Not ported: the joint-modifier lean, the ✕-tap stats, the oxygen HUD meter (`queue_animation_update(4, …)`), hurt-in-water 0x76,
  the water currents 613/679 (moby_update), the Hydro-Pack back-item model (Ratchet still wears the heli-pack model).

### 14.6 Generality
The water is only level data: surface-0 collision faces (the probe's hit; the re-cast with 0x24 finds the floor under them),
refined by the level's water tables through `hero::swim::WaterQuery` (`0x26ed38`: `RippleSim::patch_height` for class 751;
a flat-plane table can implement the same trait). `Env::water` carries it; `MobySystem::water()` (default None) hands it to
the tick; the engine's `HeroWorld` wraps the moby services and the `WaterState` ripple sim. Every rule reads the probe's water
level 0x13f640, the depth 0x1415f4 and the flags of the surface reaction; no level, pool or class is named.

### 14.7 Checks
- Unit tests (`hero/swim/tests.rs`, hand-built pools): a fall into 4-deep water → 0 → 6 → 0x37, floating at W − 0.12, splash + sound
  3; full stick → 0x36, speed exactly 3·dt, 0x37 again after release; without the pack R1 does nothing and □ is 0x33 (dive-in,
  pitched down, air −11/tick); with the pack R1 → 0x35, R1+□ reaches 7·dt and dives > 1.5, R1+✕ surfaces (0x37) and the pitch /
  roll straighten; air runs out after 910 ticks → 0x6a (sinking, locked), the O2 mask stops the drain; ✕ in deep water →
  0x12 on the curve, apex > W + 0.5, back to 0x37; swimming up a ramp → 0x36 → 0x73 → 2, out of the water; a `WaterQuery`
  height replaces the face's.
- `tests/hero/hero_novalis.rs` `novalis_walk_into_the_lake_swim_and_dive`: from the spawn, `LAKE_SCRIPT` runs off the ledge to the
  z-40 walkway and walks into the lake (water faces z ≈ 39.0–39.3, floor ≈ 33.5) at tick ~528 → 0x37 → 0x36; `DIVE_SCRIPT` (R1 at
  760, R1+□ + stick 761–820, R1 to 900): with the pack 0x35 down to z ≈ 34.1; without it R1 does nothing and □ gives 0x33.
- Engine (`RC_SCENE=0`, frame-exact): `RC_PLAY_SCRIPT="0-214:stick 0 -1,215-299:stick 0.5 -0.85,300-399:stick 0.2 -0.98,400-700:stick 0 -1"`,
  `RC_SCREENSHOT_FRAME=640`: Ratchet swims on the lake (state 0x36, seq 0x65, z 39.09). Adding
  `,760-760:press R1,761-820:press R1+SQUARE,761-840:stick 0 -1,821-900:press R1` with `RC_GIVE_HYDROPACK=1` (new debug
  switch: the game state owns item 4), frame 830: state 0x35 pitched down near the lake floor (z ≈ 35), camera under water
  (`underwater: on`). Two runs: identical `RC_PLAY_TRACE` and identical PNGs.

## 15. The level camera system (camera records, the choice, class 17) — 2026-09-30 (W3 lane 3, G-HERO-027)

Code: `rc-formats/src/cameras.rs` (records, pvar views), `rc-formats/src/level_overlay.rs` (`camvtbl`),
`rc-game/src/follow_camera/level.rs` (slots, the choice, class 17), `rc-game/src/follow_camera.rs` (the setters, the
avoidance's level branches), `rc-engine/src/gameplay.rs` (`camera_ports`, the load; `RC_LEVEL_CAMERAS=0` skips it).

**System or not (evidence first).** A system: one `UpdateAllCameras` 0x20d620 / `Camera_ActivationCheckPriority`
0x20d410 / switch `FUN_0020d110` / slot init `FUN_0020ef58` in every overlay (overlay-diff over every function the
camera code cites: identical on all 19 levels up to relocations and alignment padding), driven by per-level data (the
records) and a per-level class table `lvl.camvtbl` (`{class, activate, init, update, pre}` × n, `-1` ending it). The
classes are per class: each has its own four functions. Class 17 (the follow-camera tweak regions) is one code on all 14
levels that have it (clusters 1477..1479). The follow camera's setters (0x313560..0x313b48) are shared: callers are
class 17, the hero-state tweaks `0x3111d8`, the first-person camera's entry turn, and 29 census units (mobys).
The camera-collision grid is **not** a working system in RAC1: section 0x84 is 0x4010 bytes with every cell 0 on all
19 levels (`cameras::tests::every_level_disc`), so `0x20fdb0` never returns a primitive and its users (the avoidance's
pass-through test `0x30f468`, the push-out `0x30f358`, the end sphere, the first-person "no first person" cells) always
take the "none" path — what the port already did. Nothing to port.

**Records** (gameplay 0x08, loader 0x255958 → table 0x15ef50, reordered to `{pos, class @+0x0c, rot, pvar @+0x1c}`):
| level | 00 | 01 | 02 | 03 | 04 | 05 | 06 | 07 | 08 | 09 | 10 | 11 | 12 | 13 | 14 | 15 | 16 | 17 | 18 |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| records | 7 | 7 | 7 | 34 | 8 | 8 | 8 | 7 | 13 | 10 | 15 | 6 | 11 | 7 | 15 | 13 | 12 | 9 | 7 |

Classes over all levels: 0 ×19 (follow, priority 5, kind 0), 4 ×19 (first person), 5 ×19 (script), 6 ×19 (type 6),
7 ×19 (kind 7: the Swingshot's camera mode 7), **17 ×58** (14 levels), 3 ×13 (kind 7, camera mode 3: rails; with a rail
in pvar +0x24 on 08 / 16), 23 ×12, 18 ×8, 19 ×6, 14 ×4 (kind 4, Kerwan), 1 ×2, 8 ×2, 22 ×2, 20 ×1, 21 ×1. Pvar header
(every class): +0x08 sphere, +0x0c cuboid, +0x10 cylinder, +0x14 path, +0x18 (1.5; not read [L]), +0x1c priority,
+0x1d blend kind, +0x1e, +0x1f activation kind. Class 17's block (0x60) is `RegionTweak` (fields in `cameras.rs`).

**Coverage: the system** (level01 addresses)
| address | what it does | port |
|---|---|---|
| 0x20ef58 slot init | per record: UpdateCam +0x84 index, +0x86 class, +0x8c mode (`0x20ee30`: the class's `camvtbl` row), +0x7c priority, +0x74 kind, +0x78 = −1, +0x7d/+0x7e/+0x8e/+0x8a/+0x89 = 0, +0x80/+0x82 = −1, active flag, pvar +0x04 = slot; camera globals cleared; `CameraResetBehindHero` | `LevelCameras::new` (the fields the choice reads); the follow camera's reset `Camera::new`; the UpdateCam words of classes the port does not run: n/a |
| 0x20ee30 | class → `camvtbl` row (0 → 0; unknown → the end row) | `CameraPorts::from_overlays` (per class, by code identity) |
| 0x20d620 `UpdateAllCameras` | the current camera's pre hook (+0x10), every active slot but the current in slot order through 0x20d410 against the best so far, the switch on a change, `Camera_handleCollWithHero`, the update (+0xc), +0x64 = position, `ExecuteCamPostUpdFuncs` | `Camera::activation_loop` (every slot but the current camera's own: two class-3 records hand over; the winning slot in `LevelCameras::won_slot`) + `Camera::update` (pre hooks: the first-person release, the script / type-6 releases as before, the level-class cameras' in `class_frame`); `Camera_handleCollWithHero`: `follow_camera/camera_moby.rs` (2026-10-01); post callbacks 0x15ef4c: the Swingshot hint's only |
| 0x20d410 check | priority 0 → no; the class hook (−1 no / 1 yes / 0 on); kind 0 always, 1 / 2 once entered (+0x7d), 4 the cuboid pvar +0x0c at the feet 0x13f3d0 (priority first), 7 hero camera mode 0x1415d4 = class (class 3: its rail 0x13f8b0 and not off the end 0x13f8c0), else no; beats the best only with a higher priority unless the best releases (+0x7e) | `level::activation_check` (test `activation_check_kinds_and_priorities`) |
| 0x20d110 switch | blend by the new camera's pvar +0x1d / the old's +0x7e (1 / 5 blend at +0x78 or 0.018, 3 / 6 copy the pose, 4 cut; Novalis rate 0.01), D copied to 0x169810, the new camera's init, `BackupCurrentCam`, 0x167240 | the switches the port has (first person, script, type 6: follow_camera.rs, script.rs, type6.rs; the Swingshot camera in and out: `swing_switch_in` / `swing_frame`, the pose copy with the follow init's `0x311dd0` path `Camera::init_from`); **the general blend `Camera::switch_blend` and the level-class cameras in and out (`class_switch_in` / `follow_switch_in`, the shared D block's words read before written: `d_word_00` / `d_word_64`; coverage table: module doc of `follow_camera/class_cam.rs`)**; a switch to a class the port does not run (8, 19..22): recorded in `LevelCameras::wanted`; over a released camera the follow camera comes back instead |
| 0x167373 / 0x167372 blend kind | 0 the rates blend, 2 the orbit about Ratchet (`fun_001ec8a0` capture → `fun_001ec710` / `fun_001ec7f0` / `fun_001ec868`, `fun_001ed2b0` → `fun_001eccd8` step), 1 (no writer among the ported cameras) | `CamBlend::kind` / `running`, `OrbitBlend` (the first-person init / release, `CameraScript2(2 / 4)`, the type-6 hand-back reset it to 0 as the game does) |
| 0x20e670 `CameraPreUpdate` | up vectors, hero motion | `Camera::pre_motion` (§7; unchanged) |

**Coverage: the classes** (activate / init / update / pre; level01 unless noted)
| class | functions | what | port |
|---|---|---|---|
| 0 | 0x314df8 (returns 0) / 0x311f38 / 0x314e00 / 0x314e90 (empty) | the follow camera | §7, `Camera::update_type0`; init now also clears D+0x220 / D+0x230 |
| 4 | 0x316880 / 0x316b98 / 0x316330 / 0x316c08 | first person | `first_person_*` (its hook is the loop's class-4 slot) |
| 5 | 0x317668 (0) / 0x3171b8 / 0x317670 / 0x3176c8 (empty) | script camera | `script.rs` (switched by `CameraScript`, not by +0x7d) |
| 6 | 0x317f50 (0) / 0x317f58 / 0x318008 / 0x318028 (empty) | the Visibomb's view | `type6.rs` |
| 17 | 0x319688 / 0x319788, 0x319790, 0x319798 (all empty) | follow-camera tweak regions (never current) | `level.rs` (below) |
| 7 | 0x318b10 (0) / 0x318030 / 0x318b18 / 0x318c08 | Swingshot camera (camera mode 7; uses the collision push `0x20f2a8`) | `swing.rs` (below; all 19 levels) |
| 3 | 0x315dd8 (0) / 0x315de0 / 0x315358 / 0x316030 | rail / slide camera (mode 3; Novalis, 05, 08, 16, 17) | `rail.rs` (below; 2026-10-01) |
| 1, 14 | L03 0x2e8870 / 0x2e8940 / 0x2e8960 / 0x2e8980; 0x2ebd70 / 0x2eb978 / 0x2ebdb0 / 0x2ebde8 | kind 4: the fixed view (03, 04) and the side view (03) | `cuboid.rs` (below; 2026-10-01) |
| 23 | L00 0x2ed8b0 (0) / three empty hooks | the placed view (arrival spots; never current) | `level.rs` `placed_*` (below; 11 levels) |
| 18 | L02 0x2fc298 (0) / three empty hooks | the moby focus (never current) | `focus.rs` (below; 7 levels) |
| 22 | L15 0x2f8ba8 (−1) → 0x2f88e8 / three empty | class 18's modes 1 / 2 for **giant Clank only** (body 0x1413f4 = 2; its target a point of the table 0x1600ec + i·0x80 + 0x30; D+0x230 = 2) | NOT (G-HERO-005: body 2 is not reachable; no consumer) |
| 19 | L10 0x2f65f8 / 0x2f5b58 / 0x2f5f18 / 0x2f66c0 (+ arm 0x2f5a50) | the armed fly-by (10, 13, 14, 15; below) | NOT: no ported class arms it (G-HERO-027) |
| 8 | L05 0x32a6d8 (0) / 0x32a6e0 / 0x32a758 / 0x32a788 (empty) | the hoverboard camera (kind 7, camera mode 8: hero states 0x6b / 0x6c; 05, 16) | `board.rs` (2026-10-01; the orbit `0x3298d8` is level 16's: G-HERO-008) |
| 20, 21 | L14 0x315920 / 0x314f00 / 0x315290 / 0x3159d0; 0x316748 / 0x3159d8 / 0x315d48 / 0x3167e0 | the level-14 grind race: its intro fly-by (20) and race camera (21) | NOT: G-LVL-007 (the race) |

**Coverage: class 17** (module doc of `follow_camera/level.rs` has the full rules)
| address | call / branch | port (`follow_camera/level.rs`) / test |
|---|---|---|
| 0x319688 | +0x48 = 1 and +0x4a ≠ 0 → nothing | `region_hook` / `leave_and_once` |
| | mode 11 only with body 2 (0x1413f4) | `region_hook` (body 2 unreachable: G-HERO-005) |
| | modes 1 / 10, counter 0, airborne 0x13f65c ≠ 0 → not unless level 14 with camera mode 0xd | `region_hook` (untested: level-14 ledge) |
| | mode 2 outside group 0x1a → counter 0 (lock kept) | `region_hook` / `mode2_cable_group` |
| 0x318de0 | mode 8 needs D+0x230 ≠ 0, mode 6 needs 0 | `region_update` (D+0x230 = 1 from the focus scan not ported: mode 8 never runs, G-HERO-026) |
| | region test false → leave (+0x4a, counter 0, `0x313560`) | `region_leave` / `leave_and_once` |
| | lock `0x313598` refused → counter 0 (+0x4a), no release | `claim_owner` / `owner_lock_priorities` |
| | +0x36 and `FastDiffRots(rot.z, 0x167258)` ≥ 80° → leave | / `facing_check`, `novalis_region_tilts_the_camera` |
| | +0x48 and mode 5 and 0x13cae0 & 5 (L1 / L2) → leave | `region_update` (untested) |
| | counter 0 → capture D+0x11c / +0x120 into +0x40 / +0x44 | / `region_writes_the_follow_camera` |
| | mode 5, counter 0 → the snap: turn +0x00 := 0; D+0x130 set to the distance, +0x15c, +0x160, +0xf0, +0x140, position = pivot + offset, +0xd0 = +0x80, +0x40 = +0xa0 = the feet, +0x00 = +0x1f0 = position, +0x34 / +0x38 / +0x2c / +0x30 = 0, +0x24 = +0xf0, +0x28 = +0x160 | `region_snap` (untested: 5 regions on 02 / 05 / 11 / 12 / 14) |
| | counter += 1; mode 10 → `0x313858`, D+0x230 = 0x14d; mode 11 → `0x313858`, D+0x230 = 2 | / D+0x230 = 0x14d skips the ledge turn in `update_type0` (untested) |
| | modes 2 / 4: +0x1b8 |= 3, counter ≤ 200, `0x3137f8` | / `mode2_cable_group` |
| | modes 1 / 7 / 9 / 10: stick → 200; ≥ 200 stopped (held while `0x167334` > 0); ≥ 400 → 1 | / `turn_counters` |
| | mode 3: counter ≤ 1 (rate / 1); others ≤ 200 | / `turn_counters` |
| | the turn `0x313af0(turn·π/180·counter/200, tol·π/180, (cos, sin, 0) of rot.z)` unless stopped in 1 / 7 / 9 | `turn_toward` / `turn_counters` |
| | distance +0x24: `0x313628(d, 0.003)` + `0x3137f8`; modes 6 / 8: 0.002 + `0x313668(d + 1.36)` | / `region_writes_the_follow_camera` (mode 6/8 rates untested) |
| | pivot +0x28 `0x313690` (0.003 / 0.002); look +0x30 `0x3136c8` (0.005 / 0.002); pitch +0x50 `0x313718`; +0x54 / +0x56 → +0x1b8 |= 2 / 1; +0x34 = 0 → `0x313740(0)` | / `region_writes_the_follow_camera` |
| | spring +0x38 / +0x3c: eased over 120 ticks from the captured one (`0x313768`, zero args skipped) | / `region_writes_the_follow_camera` |
| | modes 1 / 7 / 10: `0x3137f8`, `0x313820`, `0x3137b0(0.01, 0.2)` | / `turn_counters` |
| 0x318c40 | not class 0 current → false; forced inside while running in mode 2 + group 0x1a or mode 4 + camera mode 3; cuboid +0x0c, cuboid +0x4c, cylinder +0x10, sphere +0x08, path +0x14 (level 04's path test is a superset copy: no class-17 path there) | `region_test` / `mode2_cable_group`, `kerwan_cables_take_the_cable_regions` |

**Class 7, the Swingshot camera** (`follow_camera/swing.rs`, 2026-10-01). System or not: one camera class (a row of
`lvl.camvtbl`), one code on all 19 levels (masked overlay diff of 0x318030 / 0x318b18 / 0x3182c8 / 0x318900 /
0x20f2a8 and the blend's 0x20d730 / 0x20d910 / 0x20d9f0 / 0x20ded8: `=` everywhere, the `c` / `s` cells on 02..18 are the
matcher's function ends); its level branches (14, 7, 9) are data in the one copy. Every level has one record (priority
6, kind 7, blend 3). `CameraPorts::swing` (test `ported_camera_classes_every_level`: true on 00..18).
| address | call / branch | port (`swing.rs`) / test |
|---|---|---|
| 0x318b10 | activation hook: 0 (the kind-7 test decides) | `Candidate::hook` 0 / `unported_class_is_wanted` |
| 0x20d110 → 0x318030 | blend kind 3 (pose copied, +0x7d = 2; kinds 1 / 5 would set rates, others +0x8e = 1: all overridden by the init) | `swing_switch_in` / `switch_in_and_release` |
| 0x318030 | D+0x80 = 0; D+0x8c = 1.5 (2.0 on level 14 in 0x2c / 0x2d); D+0xa0 = 4.64, D+0xb0 = 2.0, D+0x30 = 0, D+0xbc = 0 | `swing_init` / `swing_placement` |
| | D+0x88 (12°), +0x98, +0x9c, +0x84 (Ratchet), +0xa4, +0xb4, +0x00, +0x10..+0x1e, +0x34..+0x3c | n/a: no class-7 code reads them; the follow camera's init rewrites what it reads |
| | pose = the previous camera 0x167284 (rows, position), +0x40 = its forward, +0x7e = 0 | `swing_init` / `switch_in_and_release` |
| | swinging (0x2c / 0x2d): D+0x70 = (cos, sin) of 0x13fcfc, D+0xb8 = it, D+0xc0 = 0x13fce4; else D+0x70 = flat unit (0x13fcb4 − camera), D+0xb8 its yaw, D+0xc0 = 0x13fcb4 | / `switch_in_and_release`, `pull_init_and_ramp` |
| | 0x167370 = 1, 0x167373 = 2, 0x1673f4 = 60 | / `switch_in_and_release`, `orbit_blend_lands` |
| 0x318b18 | state 0x2c: D+0xc0 ≠ 0x13fce4, or 0x24..0x26: ≠ 0x13fcb4 → own pose into 0x167284's camera, the init, 0x1673f4 = 45 | `swing_update` / `target_change_reinits` |
| | 0x318ad0 (0x3182c8, 0x318900), the push `0x20f2a8(0.5, pos)`, D+0x80 = 0 | `swing_update` |
| 0x3182c8 | distance / height targets 4.64 / 2.0; 0x2c with a target: f = clamp(target z − Ratchet z, 0, 0x13fcf0) / 0x13fcf0 → 3.65 + f / 2f | `swing_place` / `swing_placement` |
| | `Approach` distance 8·dt, height 4·dt, look height (h − 0.5; level 14 in 0x2c: h + 0.25) 8·dt | / `swing_placement` |
| | 0x2c on levels 14 / 7 / 9 with the target's group ≠ −1: the group list 0x1abcc0[+0x21], classes 0x323 / 0x2f6, not the target, cos > 0 with 0x13f3e8, score `d − 10·cos` < best (1000) → yaw target = toward it | `next_target` (the tick feeds `SwingCamera::group`, `MobySystem::group`) / `group_look` |
| | else 0x2c: 0x13f3e8, k 0.00125; 0x25: 0x13f3e8, k 0.0035 (gp−0x49b0); d 0.175 (`0x20cf28`); other states: no spring | / `group_look`, `pull_init_and_ramp` |
| | D+0x80 = 1 → the moby D+0x84's yaw +0x48 | n/a: D+0x80 is 0 after the init and every update, no other writer |
| | D+0x70 from the yaw; target = Ratchet − dir·distance + (0, 0, height) | / `swing_placement` |
| | `CollLine_Fix(camera, target, 0x12)` hit: 0.5 short (when > 0.5 away) and up to 5 × `coll_sphere(0.75, flags 0)`; yaw = toward Ratchet, D+0xbc = 0; ≤ 0.5 away: the target point kept | / `wall_stops_short` |
| 0x318900 | look = Ratchet + (0, 0, D+0x8c); angle = 90° − asin(look dir · forward); 0x25: D+0x30 += 0.017 (≤ 1) × angle; else `0x20cf28(0, angle, 0.02, 0.175)` | `swing_rows` / `pull_init_and_ramp`, `swing_placement` |
| | forward = rot(saved, turn, saved × dir); saved = it normalised; left = up_s × forward; up = forward × left | / `swing_placement` |
| 0x318c08 | pre hook: 0x1415d4 ≠ 7 and state ≠ 6 → +0x7d = 0, +0x7e = 3 | `swing_pre` / `switch_in_and_release` |
| 0x20d110 back | +0x7e = 3 with the follow record's blend 0: pose copied, +0x7d = 2, no blend; init `0x311dd0` (T = S = Ratchet, vertical target, velocities 0, pivot / look along −gravity, offsets from the copied position, D+0x220 = 0), D+0x20 = 0 after class 7 on the ground (0x13f65c = 0) else 90 | `swing_frame`, `init_from`, `place_under_pose` / `switch_in_and_release` |
| 0x20f2a8 | +0x8a ≠ 0 → no push, +0x8a = 0 | n/a: set by `CameraScript2(0)` on the script camera only |
| | n = trunc(\|pos − +0x64\| / 0.9r) + 1 steps, each: up to 6 × `coll_sphere(r, flags 0x15ef5c, Ratchet)`, +0x89 = 1 when pushed; 0x167240 = the result (unless 0x16c4ec) | `collision_push` / `collision_push_open`, `collision_push_wall` |
| 0x20d620 | +0x64 = +0x30 after the update | `swing_frame` |
| `Camera_handleCollWithHero` | the camera moby 0x3ef deleted while class 7 is current | `camera_moby.rs` (any camera but the follow camera deletes it) |
| orbit blend 0x20ded8 | n < 1 → done (the camera's view, 0x167372 = 0, 0x167370 = 0); f = 1 / CosInterp(1, N, n / N); target (yaw, pitch, distance) of the camera about Ratchet in the captured frame; each eased by f (angles wrapped); position = Ratchet + rot(rot(setlen(fwd, d), yaw, up), pitch, up × v); rotation: the flat angle to the camera's forward (the long way when its side disagrees with the yaw step beyond 90°) × f about the frame's up, then the pitch × f; rows with left = −(gravity × forward); n −= 1 | `CamBlend::orbit_step` / `orbit_blend_lands` |

**Class 23, the placed view** (`follow_camera/level.rs` `placed_*`, `rc_formats::cameras::PlacedView`, 2026-10-01).
System or not: one class, level 00's code (hook 0x2ed8b0, region test 0x2ed348, update 0x2ed498) on the 11 levels that
list it (00, 04, 06, 07, 08, 09, 10, 13, 15, 16, 18; masked overlay diff of the two helpers `=` on each, the hook by
`Relocation::same_code` in `CameraPorts::placed`); like class 17 it never becomes current and retunes the follow camera
through the same setters and the same lock (0x313598 / 0x313560). Not class 17's code (no shared function). 12 records,
priority 4, activation kind 3, blend 3. The arrival spots: Ratchet starts inside the cuboid at the ship, the camera
stands at the record's position; once he is outside with the follow camera up the region is done for good (+0x26).
| address | call / branch | port / test |
|---|---|---|
| 0x2ed8b0 | +0x26 ≠ 0 → nothing; game mode 0x15f5c4 = 0 or not inside → update; else (a cutscene inside) +0x20 = 0, +0x26 = 0; returns 0 | `placed_hook` (the port's camera runs in game mode 0 only: the cutscene branch n/a) / `placed_view_places_the_camera` |
| 0x2ed348 | follow camera not current → 0; +0x46 and body ≠ 1 → 0; +0x44 and body 1 → 0; +0x34 and \|D+0x164\| or \|D+0x1b0\| > 0.01 → 0; cuboid +0x0c, cylinder +0x10, sphere +0x08 (Ratchet's feet) → 1; else +0x26 = 1, 0 | `placed_test` / `placed_view_conditions`, `placed_view_places_the_camera` |
| 0x2ed498 | not inside → leave (+0x26 = 1 if running, +0x20 = 0, `0x313560`) | `placed_leave` / `placed_view_places_the_camera` |
| | `0x313598` refused → leave without the release | / (`owner_lock_priorities` covers the lock) |
| | L1 / L2 (0x13cae0 & 5) → leave | / `placed_view_conditions` |
| | +0x20 < 2: distance +0x38 = flat \|record − Ratchet\| (about gravity 0x13f5e0), height +0x3c = −(d · g), look +0x40 = height − distance·tan(+0x2c) (0 at ±90°); the camera at the record: +0x30, D+0x15c, D+0x160, D+0xf0, pivot / look along −g, D+0x130 = D+0x140 = offset, D+0x40 = D+0xa0 = Ratchet, D+0x00 = D+0x1f0 = the position, D+0x200 = 0, D+0x34 / +0x38 / +0x2c / +0x30 = 0, D+0x24 / +0x28 = the heights, D+0x22 = 0 | `placed_update`, `placed_snap` / `placed_view_places_the_camera` |
| | +0x20 += 1 (≤ 200); +0x38 → `0x313628(d, 0.003)` + `0x3137f8`; +0x3c → `0x313690(h, 0.003)`; +0x40 → `0x3136c8(l, 0.005)`; +0x28 → `0x313718(deg)`; +0x24 = 0 → `0x313740(0)` | / `placed_view_places_the_camera`, `placed_view_conditions` |
| pvar +0x30 | (0.32 / 0.737 / …) | n/a: not read by the class [L] |

**Class 18, the moby focus** (`follow_camera/focus.rs`, `rc_formats::cameras::MobyFocus`, 2026-10-01). System or
not: one class; level 02's code (hook 0x2fc298, update 0x2fb9c8, region test 0x2fb648, view test 0x2fb788) on 02,
03, 06, 07, 08, 12, 18 (decompiles identical up to addresses; `CameraPorts::focus` compares the hook and its update
copy word for word, the tests by `Relocation`); never current; the follow camera's setters and region lock (with 17 and
23). Class 22 (L15) is a separate giant-Clank variant of its modes 1 / 2 (not ported, G-HERO-005). 8 records.
**Data fix**: the camera blocks get the loader's moby-link fixups (gameplay 0x50; +0x28 of every class-18 record):
`rc_formats::cameras::remap_moby_links` through the load's instance → moby map (engine load); without it the index
named a free slot on levels with uncreated instances (L08: the regions were dead).
| address | call / branch | port (`focus.rs`) / test |
|---|---|---|
| 0x2fc298 | follow camera current: +0x20 < 0 → nothing, else the update; +0x50 = 0; answers 0 | `focus_hook` / `moby_gone_group_and_suppress` |
| 0x2fb9c8 | +0x50 ≠ 0 → +0x20 = 0, release | `focus_update` / `moby_gone_group_and_suppress` (no writer of +0x50 found [L]) |
| | the moby: group +0x44 ≥ 0 → its list's first member with state < 0x80, none → gone; else the moby +0x28, state ≥ 0x80 → gone; gone → +0x20 = −1, release | `focus_moby` (the tick feeds `CamWorld::mobys` / `groups`) / same |
| | +0x48 = the moby; mode 6 and 0x13f64c ≠ it → +0x20 = 0, release | `focus_ground` / `mode6_and_near_kind` |
| | `0x313598` refused → +0x20 = 0; region test false → +0x20 = 0, release | / `mode3_turns_toward_the_moby` |
| | mode 5: +0x20 < 2 → +0x40 = record rot.z, else \|rx\| or \|ry\| ≥ 0.3 → the camera's yaw (the "+π when > 90° off" result is discarded); mode 4: rot.z, or rot.z + π when that is more than 90° off the camera's yaw | / `modes_4_5_facing` |
| | +0x20 += 1; mode 7: rx ≠ 0 → 300; ≥ 300: no turn, +0x3e += 1, beyond 330 a tenth; ≥ 400 → 1 (+0x3e = 0); 200 < c < 300 → 200 (+0x3e = 300) | / `mode7_tenth` |
| | modes 1 / 2: end-sphere flags 0xb0 (`0x2f6570`), sphere chain 1 / 12 / 0.11 (`0x2f6598`); stick → 300; ≥ 300: +0x3e += 1, below 400 (mode 1) / 350 no turn, else the view test (30°) → 560 / 400; ≥ that → 1; 200..300 → 200 | `set_end_flags`, `set_sphere_chain` / `modes_1_2_stick_and_restart` |
| | others: ≤ 200 | / `mode3_turns_toward_the_moby` |
| | turn = +0x00·scale·counter/200 (°); the view test (+0x30°, +0x2c°) passed: modes 4 / 5 `0x313af0` along +0x40, mode 3 eased over 90 ticks then `0x313b48` toward the moby, else `0x313b48` | `turn_toward`, `Camera::turn_toward_point` / `mode3_turns_toward_the_moby`, `modes_4_5_facing` |
| | +0x34 → `0x313628(d, 0.003)` + `0x3137f8`; +0x38 → `0x313690`; +0x4c → `0x3136c8(l, 0.005)` | / `mode3_turns_toward_the_moby` |
| | mode 3: leash 0, `0x3137f8`, `0x313820`, v spring 0.01 / 0.2, h spring 0.01 → 0.03 over 120 ticks / 0.2 | / `mode3_turns_toward_the_moby` |
| | modes 4 / 5: leash 0, `0x3137f8`, `0x313820`, v spring 0.02 / 0.2, distance 5.84; mode 4 with the moby > 110° off +0x40 (after a turn) → distance 8 (or +0x34 + 3.36); end-sphere flags 0xb0 | / `modes_4_5_facing` |
| 0x2fb648 | (0x16735c = the moby → inside: no ported writer); mode 6 → on it; +0x22 = 1 → within +0x24 of it; else the first of cuboid +0x0c, cylinder +0x10, sphere +0x08, path +0x14 | `focus_test` / `mode6_and_near_kind` |
| 0x2fb788 | both 0 → yes; the moby's elevation from the camera (about 0x1672c0) within +0x2c° of the camera's; the flat angle within +0x30° (flat → no) | `focus_view` / `view_limits` |

**The focus scan** (`0x3111d8`'s tail; G-HERO-026): +0x230 = 0 (0x16735c = 0), then `coll_sphere_mobys(15, Ratchet's
feet, flags 1, Ratchet)`: the first listed moby with a target record (`FUN_002711f8`: mode 0x20) whose byte +0x0d is
set → +0x230 = 1. Port: `update_type0` (the tick feeds `CamWorld::focus`, the mobys with that byte). Untested beyond
the query (`collision_query` tests `coll_sphere_mobys`); class 17's modes 6 / 8 read it.

**The Swingshot targets' look-up hint** (G-HERO-026's "look-at hint"; `swing.rs` `LookHint`, level03 0x2eb3d0 /
0x2eb4c0 / 0x2eb408 / 0x2eb468, level01's record 0x167480..0x167490; the code sits after the Swingshot camera's on every
level that has the targets [L: read on 03, placed by position on the others]). It is not the auto-yaw: the callback
raises the follow camera's look height.
| address | call / branch | port / test |
|---|---|---|
| 0x2eb3d0 | reset: camera 0, callback 0x2eb408, distance 10000, target 0, weight 0 (a target going state 0 → 1) | `HintCall::Reset`, `swing_target.rs` / `look_hint` |
| 0x2eb4c0 | follow camera current, \|0x13ca44\| ≤ 0.05, 0 < d < 22, d < distance, target above Ratchet, ≤ 64° off the camera forward from Ratchet, ≤ 36° from the camera → weight += 0.03 (≤ 1), camera, target, distance = from the camera | `Camera::look_hint` (the tick applies the moby loop's calls before the hero update: `CinematicCall::LookHint`) / `look_hint` |
| 0x2eb408 | from `0x3111d8`: weight ≠ 0 → `0x3136c8(0.35·weight, 0.005, add)`, `AddCamPostUpdFunc(0x2eb468)` | `look_hint_callback` / `look_hint` |
| 0x2eb468 | distance 10000; a target → target 0, else weight −= 0.03 (≥ 0); camera 0 | `look_hint_post` / `look_hint` |
| 0x20ef58 | clears the camera, the weight and the callback | `Camera::set_level` |

**Class 3, the rail / slide camera** (`follow_camera/rail.rs`, `rc_formats::cameras::RailCamera`, 2026-10-01; full
coverage table in the module doc). System or not: one class, level 01's four functions and `0x314e98` on 01, 05, 08,
16, 17 (`CameraPorts::rail`, `Relocation::same_code`); the spline walks are the shared `0x2726c8` / `0x272e28`
(`crate::spline`). Activation kind 7 (camera mode 3: grinding, the cable slide 0x74, the sinking state 0x31) with the
record's rail (pvar +0x24) checked by 0x20d410 and the pre hook. 13 records:
| level | records (mode, path, rail, blend) | consumer |
|---|---|---|
| 01 | #0 (3, 42, −1, 5) | the sinking state 0x31 (camera mode 3): the camera beside Ratchet along path 42, 3 behind (12 ahead when it started ahead: +0x36), looking 2 ahead, 0.5 up |
| 05 | #1 (1, 18, −1, 3) | state 0x31 |
| 08 | #0 (0, 118, 5), #1 (0, 54, 1), #2 (2, 119, 6; maps 44 / 43), #3 (2, 120, 0; maps 46 / 45), #4 (0, 51, 2), blend 3 | the grind rails 5, 1, 6, 0, 2 |
| 16 | #0 (2, 154, 1; maps 129 / 130), #1 (0, 155, 3), #2 (0, 156, 2), #3 (2, 157, 7; maps 132 / 131), #4 (1, 158, 8), blend 3 | the grind rails |
| 17 | #8 (0, −1, −1) | no path: with camera mode 3 (state 0x31) the port's camera stays put and looks at the feet [L: the game reads the word before the path table] |
Switches: in with blend 3 (the pose copied, a cut) or 5 (Novalis: the orbit blend, 40 ticks); out with +0x7e = 3 (a
cut back to the follow camera, its row blend D+0x20 = 90 ticks) or 5 (mode 3: the rates blend at 0.01 on Novalis).
Rail to rail (08 / 16): the old record lets go (+0x7e = 3), the new rail's record wins over the follow camera's
5 and is switched in from the old pose. Kerwan's cables have no class-3 record (class 17's mode-2 regions).

**Classes 1 and 14** (`follow_camera/cuboid.rs`, 2026-10-01; coverage table in the module doc): kind 4, Ratchet's
feet in the record's cuboid, level 03's code. Class 1 (03 #22 cuboid 45 blend 1, 04 #0 cuboid 39 blend 3): the fixed
view at the record turning toward Ratchet (+ the look height pvar +0x18); its hook asks the rates blend (0.018) when the
camera is outside the cuboid. Class 14 (03 #7, #8, #13, #14: cuboids 19, 24, 38, 39, blend 5, distance 8, height 4):
the side view along the record's facing, Ratchet's height held through a jump, the orbit blend in over 120 ticks;
never in a jump out of the water (camera mode 0x50) or in group 6.

**The hero-state tweaks** `CamType0HeroStateTweaks` 0x3111d8 (follow_camera.rs `update_type0`, whole since
2026-10-01):
| branch | what | port |
|---|---|---|
| camera mode 0xd, D+0x230 ≠ 0x14d | the ledge turn `0x313af0(12°, 0, dir(ledge yaw + π))`, `0x313820` | `Hero::ledge_camera_yaw` |
| group 0x11 / 5 / 0x10 | look height 0.25; pivot 0.5 (water), 1.5 / 2.5 (gliding, Clank / Ratchet) at 0.003 | `update_type0` |
| body 1 (Clank) | no region holds the lock: distance 3 (base too) at 0.003, not gliding: look and pivot 1; D+0x20c = 0.6 | `update_type0` |
| body 2 (giant Clank) | D+0x230 < 2: look 9 (0.005), distance 12 (base, 0.003), pivot 15 (0.003); `0x3137f8`; D+0x20c = 2.0, D+0x214 = 0.14 | `update_type0` (body 2 unreachable: G-HERO-005) |
| 0x1413fa (weapon up), group ≠ 0xf | leash off, `0x313820`, h spring 0.04 / 0.2 | `update_type0` |
| 0x167490 | the Swingshot hint's callback | `look_hint_callback` |
| 0x16735c ≠ 0, D+0x230 < 2 | the moby's state ≥ 0x80 → cleared; else the right stick (|x| or |y| ≥ 0.3) zeroes 0x167360; +1, ≤ 400; `0x313b48((n / 400)·12°, 0, moby +0x10)` | `update_type0`, `Camera::set_focus_moby` (writers 1422 / 1470 / 1051 not ported: `cinematic::focus_moby` for their ports), the tick feeds the moby (`CamWorld::mobys`) |
| | D+0x230 = 0, then the focus scan | `update_type0` |
| state 0x81, 0x13cae0 & 3 (L2 / R2) | leash off, `0x313820`, h spring 0.04 / 0.2, D+0x12 / +0x14 / +0x16 = 0 | `update_type0` (D+0x12..0x16: n/a, read only by the next branch) |
| gp 0x16220c ≠ 0 | the yaw stabiliser (D+0x12..0x16, 0x15ef40 from the yaw history 0x167340) | n/a: 0 on disc and no writer |
Class 18's region test reads 0x16735c too (the region's moby being it → inside): `focus.rs` `focus_test`.

**Target modes** (D+0x104; `0x30fb08` = `target_mode`, `0x3101c0` = `vertical_target`, 2026-10-01: all modes; before
only 0 / 1 / 2 / 5 (state 0xe) / 6 / 7 / 8): which hero states use which.
| mode | entered | vertical target | left |
|---|---|---|---|
| 1 | group 2 (from 0, 10 ticks); 2 in state 0x11 or a jump looking down (view < −0.01 or > 90°); 7 / 4 in group 2; 9 / 10 in group 2 or 55 ticks airborne (row blend 90, saved forward) | plain (the look from Ratchet) | out of groups 2 / 4 → 0 (no blend, D+0x105 kept) |
| 2 | group 4 (30) | held unless landing / below / on a moving platform | as 1 |
| 3 | state 0xf (the raise 3.0 over 85 ticks) | the raise | state 0xb → 6; out of group 4 → 0 (20) |
| 4 | the end of a raise (3 / 5 / 7, 20) | plain | out of group 4 → 1 (group 2) or 0 (20) |
| 5 | states 0xd / 0xe (the raise 3.5 over 45) | the raise | state 0xc → 8 (30); out of group 4 → 0; state 0x10 → 0 (20) |
| 6 / 7 | state 0xb (the raise 1.0 over 35); 7: the raise's end → a second raise (35); 7 not descending → 6 | the raise | 6 out of state 0xb → 0 (20); 7 → 4 at its end |
| 8 | state 0xc (30) | held | out of state 0xc → 0 (20) |
| 9 | state 0x14 (the raise 1.5 over 30; no blend) unless in 1 / 9 | halfway between plain and the raise's start | out of state 0x14 → 10 (30) |
| 10 | from 9 | as 9 | the raise timer out → 0 (20) |
| 11 | state 0x77 (row blend 90, saved forward) | the smoothed target's height, the target flattened | never (the next init) |
Fixes on the way: the look blend's inverse (D+0x108) now set on the 2 → 1 and raise → 4 changes; 1 / 2 → 0 keeps D+0x105.

**The camera moby** (class 1007, `follow_camera/camera_moby.rs`, 2026-10-01): `Camera_handleCollWithHero` creates it
while the follow camera is current (hidden, always updated, with collision) and deletes it under any other camera; its
update parks it at (15, 15, 15) unless Ratchet runs toward the camera at its height, when it stands at the camera's
spot on his plane (he bumps into it). Registered as `ClassUpdate::CameraMoby` (level01 0x2ba7c8 and its copies).

**The collapsing platform 701's look** (`classes/props.rs` `collapse_look`, `FUN_002f9000`, states 2 / 3): the follow
camera turned along the trigger cuboid's x row at 8° a tick, distance 7, pivot 2.5, look 3.5 (0.02), through
`cinematic::follow_turn_toward` / `follow_distance` / `follow_pivot_height` / `follow_look_height`.

**The classes the port does not run** (a choice of one is logged in `LevelCameras::wanted`; specs for their ports):
* **Class 19, the armed fly-by**: ported 2026-10-04 (`follow_camera/flyby.rs`; the spec below kept). (L10 hook 0x2f65f8, init 0x2f5b58, update 0x2f5f18, pre 0x2f66c0 (empty), arm
  0x2f5a50; copies on 13, 14, 15). Records: 10 #4 / #9, 13 #0, 14 #3 / #7, 15 #3 (priority 6, blend 0, kind 3; no
  shapes). The arm (`0x2f5a50(slot)`: +0x38 = 1, +0x39 = 0) is called by units 343 / 344 (L10 1421 / 1424), 415
  (L13 388, through 0x2e9a48 → 0x2e97e0), 451 / 458 (L14 921 / 1352), 490 (L15 1430) — none ported. Hook: +0x39 (done)
  → −1; armed and the best not class 19 → 1; else the best releasing or of lower priority, the region test (shapes,
  else armed and not done), group +0x48 / state +0x44 (−1: any) → 1. Init: +0x38 = 1; D+0x40..0x54 = 0, D+0x56 =
  ticks(+0x3a); D+0x58[8] = −1, D+0x68[8] = 0; D+0x88 = 2·atan(0x16cf70) (the FOV); path +0x20's w = chords, the
  points with w > 0 noted as FOV keys (D+0x58 index, D+0x68 degrees, > 180 → the current FOV), D+0x48 its length;
  +0x3a > 0 → +0x3c = length / ticks · 60; path +0x24's w = chords, D+0x4c its length; D+0x00/0x04 = 0.03 / 0.2, D+0x20
  / 0x24 = 0.03 / 0.2; the pose copied; +0x4c = 0 → `SetState(0x72, 1)`. Update: phase 0: the fade 0x15f3fc += 0.05
  until 1, then the letterbox 0x15f404 = 1, phase 1, the camera at path +0x20's first point looking at path +0x24's
  first (or the moby +0x28 + the rotated offset +0x2c..0x34); phase 1: the fade −0.05, the camera along path +0x20 at
  +0x3c·dt (`0x250318`), D+0x56 ticks, the FOV keyed along the path (`0x24a8d8` lerp, `UpdateViewContext`), the look
  along path +0x24 by the arc travelled (or the moby); at the end (path end, the timer, or +0x39) phase 2: the fade
  +0.05 to 1, then phase 3: the FOV back, +0x39 = 1, 0x167358 = 0.05, +0x7e = 4 (a cut), the letterbox off, +0x4c = 0
  → `SetState(0, 1)`; +0x40 ≠ 0 keeps it running. Needs: a gameplay fade and FOV (the camera view's tangent), the
  arming classes. G-HERO-027.
* **Classes 20 / 21, the level-14 grind race**: ported 2026-10-04 (`follow_camera/race.rs`, its coverage table in the
  module doc; the race start's bot half and the attack calls in `units::oltanis_rail_bot`). 20 is class 19's fly-by
  without the fades (its FOV eased, `CosInterp`), holding Ratchet in cuboid +0x54 (0x72); its end starts the race
  (`0x2d6570`: Ratchet on his rail at the bot's mark, the bots 9 behind in state 6) and cuts to 21, which orbits him by
  a 28-row table keyed on the share of his rail behind him (yaw / pitch / distance / tilts sprung, a facing override by
  rail), turns his head back in stage 1 (head record targets 20° / −120°) and calls the bots' attacks (commands 1 / 5).
  Its stores reach the moby world before the next moby loop (`RaceOut`). G-LVL-007.
* **Class 8, the hoverboard camera**: ported 2026-10-01 (`follow_camera/board.rs`, its coverage table in the module doc).
* **Class 22** (L15 0x2f8ba8): class 18's modes 1 / 2 for Giant Clank (body 2), turning toward a cuboid's centre:
  ported 2026-10-04 (`follow_camera/giant.rs`, its coverage table in the module doc).

**The setters** (`Camera::…` in follow_camera.rs; each a no-op unless the follow camera is current): `0x313628`
`set_distance`, `0x313668` `set_pull_max`, `0x313690` `set_pivot_height`, `0x3136c8` `set_look_height`, `0x313718`
`set_script_pitch`, `0x313740` `set_leash`, `0x313768` `set_h_spring`, `0x3137b0` `set_v_spring`, `0x3137f8`
`lock_toward`, `0x313820` `look_from_smoothed`, `0x313858` `stick_off`, `0x313af0` → `0x313888` `turn_toward` /
`yaw_input_toward` (with the tolerance; the hero's ledge turn calls it with tolerance 0, `Hero::ledge_camera_yaw`),
`0x313598` / `0x313560` `claim_owner` / `release_owner`, `0x313b48` `turn_toward_point` (class 18; the focus moby's
auto-yaw of 0x3111d8, `set_focus_moby`), level 02's `0x2f6570` `set_end_flags` and `0x2f6598`
`set_sphere_chain` (class 18; no level-01 copy). Consumers now:
classes 17, 18 and 23, the Swingshot targets' hint, the underwater / glide pivot height of `update_type0`, the
first-person entry turn.

**Per-level differences** (overlay-diff over the camera code, `rc-trace overlay-diff --cite crates/rc-game/src/follow_camera*`):
no function differs beyond relocations and padding; the per-level behaviour is level-number tests inside the shared
code, now ported: the avoidance `0x312ef8` lists meshed mobys too on level 15 with body 2 (untested: body 2 is G-HERO-005)
and skips the sphere chain and the line on level 13 in state 0x7b (test `avoidance_level13_sinking_skips_the_line`);
class 17's ledge exception on level 14; the switch's Novalis blend rate (already in script.rs). `0x316e88` exists on
01 / 04 / 08 only (the bolt cranks' levels).

**Proof.** `tests/world/camera_levels.rs`: Kerwan's three cables take records 10 / 11 / 12 for the whole slide (counter
held at 200), the follow camera's distance ≈ 6.45 against 4.64 without the records, and the camera stays ≥ 1.45 off the
cable (without: 0.42 on cable 1); Novalis's record 1 (cuboid 82) tilts the camera to −22.1° against −6.2°, and never
takes it facing away. Engine, frame-exact, same spot before (`RC_LEVEL_CAMERAS=0`) and after: Kerwan
`RC_LEVEL=3 RC_HERO_AT=197.137,148.537,76.03,2.912 RC_PLAY_SCRIPT="30:press X" RC_DUMP_FRAMES=120..120` (after: the camera
higher and further back, the cable across the view; before: behind and under the cable); Novalis
`RC_PLAY_SCRIPT="1560-1774:stick 0 -1" RC_DUMP_FRAMES=1670..1670` (after the intro scene; after: tilted down over the
pad to the river; before: level). Two runs identical PNGs.

**The Novalis digest.** The hero digest's runs cross record 1's cuboid (lake / lake+pack ticks 58..135, moves 180..292
and 612..): with the records the camera tilts there, and camera-relative input would move Ratchet differently. The
digest harness (`tests/hero/hero_novalis.rs`) loads no records, so `novalis_hero_digest` (NO_IDLE) is unchanged; the
engine now differs from it there. Loading the records into the digest is a human decision (a digest re-baseline).

**Dependencies.** Reads: the hero's feet, group 0x1413dc, camera mode 0x1415d4, body 0x1413f4, air ticks 0x13f65c, rail
0x13f8b0 / 0x13f8c0, the pad's right stick and held buttons, the level number, the trigger shapes
(`moby_update::triggers`). Written: the follow camera's data (one tick; the spring-back 0x311010 undoes it). Readers
of the lock words: `0x3111d8` (D+0x230, D+0x220 for Clank's distance, not ported: G-HERO-005).
