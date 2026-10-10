# In-level menus (RAC1 NTSC `SCUS_971.99`, Novalis = level 01)

Addresses are **level01.elf** (the overlay carries the engine). gp = 0x166c00. Pad: pressed mask 0x13cae4 (PAD+0x1a4),
held 0x13cae0, 0x13cb00/0x13cb04 = held/pressed with the mirror undone and before the one-shot pad lock (PAD+0x1c0/+0x1c4;
the menus, vendor and quick-select read these), 0x13caf4 = pressed edges of buttons only (PAD+0x1b4). Bits: L2 1, R2 2, L1 4, R1 8, △ 0x10, ○ 0x20, ✕ 0x40, □ 0x80, Select 0x100, L3 0x200, R3 0x400,
Start 0x800, Up 0x1000, Right 0x2000, Down 0x4000, Left 0x8000. `ScaleTicks` = `fun_001f96f8` (×1.0 on NTSC). Strings are
`msg_string(id)` (English text quoted). Confidence: **[H]** read from code and data, **[M]** code with lost arguments or
one inference, **[L]** unverified. Some Lombyte names are wrong. `PauseAllSounds` 0x28bf50 is really *EnterMenuMode*,
and `DrawDebugProfiler` is the main world render.
Other Lombyte misnomers here: `DrawMapScreen` 0x28f868 (= MapWidgetUpdate), `DrawCheatsMenu` 0x294a38 (= ToggleListDraw),
`DrawGBsShipMenu__maybe` 0x292980 (= GoldBoltPanelDraw), `draw_transition_overlay` 0x293670 (= StreamedImageDraw).
Earlier doc-coined names, also wrong: `MenuSetPostAction` 0x28e600 (= `ListWidgetUpdate`), `MenuInput` 0x298f80
(= `CheatEntryUpdate`). `0x15f5c4` is the game mode, `0x15f5cc` the tick counter.

## 1. Mode system [H]

**Variable** `0x15f5c4` (gp−0x763c). There is no single set-mode function: each mode's entry function writes it.
`0x15f5c8` counts frames spent in the current mode (reset on change). `0x15f5d8 = 1` asks the frame to skip rendering once
(set on every transition). The main loop is `entry` 0x259c40. Per frame: `VU1_sendChain` → switch → memcard → sync.
The switch is below. Ticking = calls `FUN_002ab960`, which does `0x15f5cc++` (60 Hz logic tick), `0x15eea4++` (play
time) and updates the idle counter.

| Mode | Update / render | Meaning | Entered by | Ticks? |
|---|---|---|---|---|
| 0 | `0x27c478` pad, `0x2aba68` / `0x21aa30` | gameplay | many (`0x2ad2b8`, `0x2ac608`, menus on exit) | yes |
| 1 | `0x2ad498` / — | **PSS movie** (FMV). Saves VRAM, plays, restores → mode 0 (`0x2ad2b8`) | `0x2ad0c0(lsn,size,planet)` via `0x2acf50`/`0x2ad050`/`0x2acfe8` | no |
| 2 | `0x2aca80` / `prepare_debug_profiler_render` | **in-engine cutscene** (scene stream + camera keys) | `0x2ac330(scene)` | yes |
| 3 | `0x28c990` / `0x21ab50`→`0x28d080(0)` | **page menu**: pause, map, ship planet select, goodies | `0x28bf50(kind)` | no |
| 4 | `UpdateModeFreeze` 0x2249b0 / `0x21aac8` | **freeze dialog** (Quit?/pad removed/save notice) | `mode_freezeInit(kind,arg)` 0x223640 | no |
| 5 | `0x2b03b8` / `0x2b4020` | **Gadgetron vendor** (+ weapon demo scene) | `0x2ae1a0(vendor)`, `0x2aea70` | substates 0, 2, 3 only |
| 6 | `0x2a4080` / `dispatch_game_state_update` | **ship / space**: take-off, landing, fly-away, loading flight | `0x2a24b8(sub)`, `0x2a2848(dest)`, `0x2a29c0`, `DoSpaceTransition` | yes (sub 0/3/8, 4 via 0x2a33b0) |
| 7 | `0x2ad738` / `0x21ab78` | streamed full-screen image slideshow (ending/credits), Start skips | `0x2ad558` (menu post-action 7) | no |
| −1 | `0x216198` / `0x2174e0` | debug free camera/menu (exit: pressed & 0x500) | debug only | no |

Mode 6 substates live in `0x13e050`, with the tick counter `0x13e054`. Values: 0 take-off, 8 landing (level start), 3 fly
away to another planet, 4 in-space loading loop (`FUN_002a33b0`, entered from `FUN_002a5868`).

**What draws behind menus.**
* Modes 3 and 5 grab the frame once on entry: `FUN_002b4c88(buf, fbp=0x15ee84, size=0x13e504<<11)` downloads the frame
  buffer to EE RAM (`0x1ba180` for the menu, `0x1ca950` for the vendor). Every frame `FUN_002b4d38` uploads it again
  with IMAGE GIFtags (0x4000-byte chunks). The world is a **frozen snapshot**, not re-rendered and not blurred.
* The menu then draws a black rect at alpha **0x30** (`emit_rgba_draw_packet(0,0,0,0x30)`), then its 3D mobys and its 2D
  layer.
* Mode 4 re-renders the (static) world with the normal render, or the menu if kind 3. It then draws black alpha
  **0x40** and `DrawDialogText`.
* Mode 7 draws black at `0x15f3fc·128` over the image. `0x15f3fc` is the global fade-to-black factor 0..1 used by
  modes 2, 5, 6 and 7 (ramps of −0.125, −0.34, −0.25 or +0.0625 per tick; see each section).

**Gameplay → menu triggers** (in `0x2aba68`, only in mode 0, after ≥ 8 frames in the mode, hero not in state
0x72/0x32/0x1d, HP ≠ 0, not a cinematic):
* Start (0x800) or pad lost (`0x13cadc == 0`) → `0x28bf50(0)` → pause menu.
* Select or R3 (0x500) → `0x28bf50(10)` → map.
* Return page `0x1ba260` set → `0x28bf50(0)`, which reopens that page (used after a movie or scene launched from the menu).
* Race level 0xf, vehicles (hero riding class 0x45/0x563/0x4da) and some states → `mode_freezeInit(0|1|4)`
  instead: the "Quit?" dialog. Pad-removed flag `0x15eeb4 & 1` → `mode_freezeInit(3)`.
* Ship △ → `0x15f630 = 1` → next tick `FUN_002a27c8` (§5).

**Freeze kinds** (`0x1734c0`, previous mode in `0x1734d4`). Two-line text from `DrawDialogText`.
* 0: 20334 "Quit Race?" / 21064 "✕ Quit" / 21065 "○ Continue". Box slides 8+8 ticks; ✕ quits, ○ or Start continue.
* 1 and 4: 21033 "Quit?" / 20192 "△ Exit" / 21066 "✕ Continue".
* 2: 21066 only.
* 3: pad or memory-card message, 30-tick guard.
* 5: 20011 (save-icon notice).
* 6: 30 ticks.

Confidence [H] for ids and flow, [M] for the kind-3 sub-cases.

## 2. Quick-select ring (△, HUD slot 3) [H unless noted]

Callbacks: init `QuickSelectInit` 0x24d180, update `QuickSelectUpdate` 0x24d238, draw `QuickSelectDraw` 0x24d938 (the
same code in every overlay; level02 copies 0x23a860/0x23af60). Static values from the level-01 overlay. The game stays
in **mode 0**: no pause, no time scale, no darkening, no mobys; the ring is a HUD slot drawn in the HUD pass.

**Open** (hero update `0x242930`, before its state switch on `0x1413d4`): △ pressed this tick, `(0x13cae4 & 0x10) ||
(0x13cb04 & 0x10)` (PAD+0x1a4 masked pressed, or PAD+0x1c4 pressed before the lock). Then:
* `0x141622 = ScaleTicks(15)`, or 27 while the double-tap timer `0x141626` runs;
* `HudShow(3, icon 0, 0x24d180, 0x24d238, 0x24d938, 0, 0)`; identical arguments never re-init, so △ during the fade-out
  fades back in keeping the selection;
* health (`0x24a498`) and bolts (slot 2) visible timers pushed to ≥ 180 via `0x24b4b0` (only on a slot not pending, [M]).
* **Blocked** (code [H], meanings [L]) when an earlier state change fires: `FUN_00231580(1)`, HP `0x1415f8 < 1`,
  `0x240ed8`/`0x2408e8`/`0x2406b0`/`0x2405f8` ≠ 0; `0x1403fc == 2` (hand-0 swap running); `*0x15f594 ≠ 0`; `0x1413fc` or
  `0x1413f7` set unless `0x1413f4 == 1` or `0x13f502 ≠ 0`; `0x1413f4` not 0 or 3. HUD skipped while `0x17e988`/`0x15f404`.

**Input lock**: each update tick `if FastDecTimer(0x141622) != 0: PAD+0x1cc (0x13cb0c) = 2` (`FastDecTimer` is non-zero
when the timer is 0 or reaches 0), so from 15 (27) ticks after the open on, every tick while the ring stays open (a tap
never locks), `ProcessPadInput` 0x27bd90 masks held/pressed/released/raw to Start|Select (0x900) and zeroes the left stick
+0x108/+0x10c (one-shot, re-armed each tick). The ring reads the unmasked copies: stick +0x148/+0x14c, held +0x1c0, pressed +0x1c4,
stick-active +0x1d8. The right stick is not zeroed [M: camera may still turn].

**Update 0x24d238** per tick: visible timer +0x7c = ScaleTicks(180), +0x6c = 24; lock as above; stick (below);
* **Arming**: init sets sel +0x74 = −2, +0x78 = ScaleTicks(30), +0x70 = 0, page `0x15f72c` = 0 and copies
  `quickselect[8]` (`0x141ea0`) into the 8 entries at `0x17df40` (item +0x18, icon +0 = u16 at `0x179f78 + 0x4c·item`).
  While +0x78's top byte is 0: a d-pad/stick press edge (`0x13cb04 & 0xf000`), or stick length < 0.5, or `--+0x78 == −1`
  (31 ticks) arms it: sel = −1, +0x78 = 0x10000ff. A stick already held at open is ignored for up to 31 ticks.
* **Selection** only while armed and △ held (`0x13cb00 & 0x10`):
  * stick copy (0x13ca88/8c; dead zone |b−127| > 47, `(d−48)/76` ≤ 1, y down); `len = sqrt(x²+y²)`, angle =
    `FastArcTan(x/len, y/len)` (octant table 0x1c28c0).
  * **stick** (len > 0.9): hysteresis: if sel ≠ −1 and `FastDiffRots(2π·sel/8 − π, angle − π/2) ≤ 0.5890486` (3π/16) keep
    sel. Else `a = angle + π + π + π/2 + π/8`, `v = a·8/2π`, `k = (int)v % 8`; k even (cardinal): `frac(v) > 1−0.2` → k+1,
    `< 0.2` (0x15f788) → k−1, then `(k+8) % 8`. Cardinal sectors 27°, diagonal 63°. `0x15f784 = Normalize(a − (k·π/4 +
    π/8))`, dir = +1 if > 0 else −1.
  * **d-pad** (len ≤ 0.9, PAD+0x1d8 == 0, press edge in 0xf000): Up, Down, Left, Right applied in that order, each
    `sel = sel == −1 ? default : entry[sel].dir`. Entry 0x1c bytes `{icon, ?, left, right, up, down, item}`, page 0 table
    `0x17df40` (L,R,U,D): s0 7,1,0,4 · s1 0,2,0,2 · s2 6,2,1,3 · s3 4,2,2,4 · s4 5,3,0,4 · s5 6,4,6,4 · s6 6,2,7,5 ·
    s7 6,0,0,6. Defaults from −1 (`0x17e098..a4`): L 6, R 2, U 0, D 4. Up+Right from −1 → 0 → 1.
  * If `entry[sel].icon == 0` and dir ≠ 0 (stick only): try `(sel + dir) % 8` (C remainder: 0 + (−1) = −1 reads the word
    at 0x17df24 = 314159 ≠ 0, so sel becomes −1) and take it if its icon ≠ 0.
* **Text fade** +0x71: +1/tick up to 8 (`0x15f780`) while sel is unchanged this tick, else 0.
* **△ held**: +0x70 += 1 up to 8. **Released**: first the **confirm** (below) when sel ≥ 0 and the slot's item ≠ 0, then
  if +0x70 == 0 close at once, else +0x70 −= 1 and close when it reaches 0 (so the confirm repeats on each of the 8
  fade-out ticks, [M] the count may rise more than once). A tap (released the tick after the open, fade 1) closes with sel −1 (or −2), no
  equip. **Close**: `0x15fa98 = +0x78`, `0x15fa9c = sel`, `0x15fa94 = *0x15f3f8`, `fun_001ff480(+0x64)` (empty element, +0x7c = 0), +0x6c = −6.
* **Confirm**: stats record `0x1418e8` (misc record 20): count +1 when item ≠ held `0x140408` and count ≠ 0xffff;
  `+2 = max(+2, ScaleTicks(0x15eea4)/600)`; `+4 |= 1 << 0x15ed84 | 0x80000000`. Then hand-0 request `0x141408 = item`
  (item 0x18 Drone Device: `0x141345 = 1`, request unchanged), ammo clamped to max if the item has ammo. The request
  is consumed by `FUN_002307e0` (target 0x141424, last 0x141660, previous 0x15ed8c, `LoadHandGadget` 0x297d70 [M]). No sound in the ring code or in 0x22b8e8.
* **Double-tap △** = previous weapon: every △ press sets `0x141624 = 20`; on raw release (`0x13caf8 & 0x10`) while
  0x141624 or 0x141626 runs: 0x141626 == 0 → 0x141626 = 20; else request `0x141408 = *0x15ed8c` (or `*0x141660` when
  the held item 0x140408 is 8, the wrench), stats 0x141940/42/44, both timers cleared.

**Layout** (NTSC 512×416): init size +0x58/+0x5c = 210×200 (0x15f718/1c), offset +0x48 = 0; slot-3 anchor (20,208), bits
4 (left, v-centre) via 0x24b108 → box X0,Y0 = (20,108); centre offset (105,100) (0x15f720/24) → (125,208). No slide, only
alpha. `s = +0x70/8`, `f = +0x71/8` clamped to [0,1]; `A = trunc(128·s)`.
1. Ring: icon 59700 frame 0 (128×128) × 4 via `HudSprite` 0x1ffc30: (X0,Y0,105,100), (X0+210,Y0+200,−105,−100),
   (X0+210,Y0,−105,100), (X0,Y0+200,105,−100), alpha A (negative size mirrors, [M]).
2. Per slot i (N = 8 = 0x15fa90): `θ = FastAddRot(2πi/N − π, π/2)`; `px = X0+105 + (int)(74·cos θ·1.05)`, `py = Y0+100 +
   (int)(74·sin θ)` (74 = 0x15f78c, 1.05 = 0x15f7c4): ≈ (125,134) (179,156) (202,208) (179,260) (125,282) (71,260) (48,208)
   (71,156). Cursor first, if i == sel and icon ≠ 0: 59804 frame 0 at (px−19, py−19) 38×38, alpha
   `trunc(128s·(0.875 + 0.125·sin(2π·(t%30)/30 − π)))` with t = `0x15f5cc`, period 0x15f7b0 = 30 (literals 6.28318/3.14159).
   Then the icon if ≠ 0: `HudFrame(slot, GetIconFrame(icon, gold ? 4 : 0), px, py, 1 = centred, α)`, gold = byte
   `0x13e520[item]`, α = A when selected else `trunc(A·0.5)` (0x15f7c0).
3. Text when sel ≥ 0 and occupied, colours tweened by `s·f`, regular font, centred (`font_print_center` 0x21d5a8),
   shadow first at (+1,+1) tween 0x00000000→0x80000000:
   * ammo if `u16 0x1c4538 + 0x18·item ≠ 0`: `"%d/%d"` (0x15f7c8) of `0x13d428[item]` / max `u16 0x1c453e + 0x18·item` at
     (125, 208+12); colour 0x0040e020→0x8040e020 (green), or 0x004040ff→0x804040ff (red) when ammo is 0;
   * name `msg_string(u16 at 0x179f86 + 0x4c·item)`, gold: `+0x48`; colour 0x00ff8080→0x80ff8080. Split: first '-' at
     index j ≥ 1 → line 1 = bytes 0..j (keeps '-'), line 2 from j+1; else first ' ' at j ≥ 1 → line 1 = j bytes, line 2
     from j+1. Line 2 is 16 px lower. y = 208 + (−25 if the item has ammo (0x15f7b4), −15 if split (0x15f7b8), else 0).

**Slots**: `u32 quickselect[8]` at `0x141ea0` (save chunk 13). `GiveItem` 0x275760 puts a new item of type 0 (table +8)
into the first empty slot (nothing when full); save load 0x29a710 zeroes slots of unowned items; debug give-all 0x251da0:
slot 0 = 0xf, slot 7 = 0x13. Page 1 (`0x17e020`, 4 entries, icon 30035) is unused. Pause-menu assignment [M, PAL
func_0021D7A0]: R1/L1 step the slot mod 8, ✕ puts the highlighted owned item there (removing it elsewhere).
## 3. Pause menu / page system / Options / Planet select (mode 3) [H unless noted]

**Triggers** (0x2aba68, disasm 0x2abd98..0x2abe4c; mode 0, `0x15f5c8 ≥ 8`, hero state `0x1413d4` ∉ {0x72, 0x32, 0x1d},
`0x1403fc ≠ 2`, `0x141401 == 0`, HP ≠ 0): Start (`0x13cae4 & 0x800`) or pad lost (`0x13cadc == 0`) → `0x28bf50(0)`;
Select|R3 (0x500) → `0x28bf50(10)` (strings 1007/2013: map on "SELECT or R3"); return page `0x1ba260` set →
`0x28bf50(0)`; race level 0xf, vehicles (class 0x45/0x563/0x4da) → `mode_freezeInit(0|1|4)`; pad-removed `0x15eeb4 & 1`
→ `mode_freezeInit(3)`. Pad masks (libpad2 bytes big-endian, inverted, boot 0x217328): 0x13cb00 (+0x1c0) held incl.
stick directions, 0x13cb04 (+0x1c4) pressed edges incl. stick, 0x13caf4 (+0x1b4) pressed edges of buttons only. The
main loop runs `UpdatePad` 0x27c478 before every mode's update (not mode 1); no catch-up tick in mode 3.

**Entry** `0x28bf50(kind)` (Lombyte `PauseAllSounds`; really *EnterMenuMode*): pause SFX group 0x1d and music; hand item
0x24 → 0; `0x1ba2a4` = level 0xd or `0x14161b`; `0x1ba2a8` = level 0 or 0xe; Goodies `0x1ba248 = (0x15eea0 ||
0x15ee20 || 0x1ba268)` [M]; `0x1ba24c = kind == 0x23`; **Goodies wiring**: `0x1b2b70` (Weapons.above) and `0x1b2d04`
(Options.below) = Goodies 0x1b2d18 when unlocked, else Options 0x1b2cc8 / Weapons 0x1b2b38 (wrap over 6); mode 3,
`0x1ba170 = kind`, `0x1ba280 = 0` (menu ticks), `0x1ba17c = 0` (post-action), `0x184894` = level if < 0x13 else 0.

**First tick** (`FUN_0028c128`, when kind ∈ {0, 10, 0xe, 0x11, 0x21, 0x2d, 0x23}): kind → page: 10 → 0xb `0x1b3998`
(map), 0xe → 0xf `0x1b6508` (planet select), 0x21 → 0x22 `0x1b7670` (`0x1ba294 = 1`: no close keys), 0x23 → `0x1b6fb8`,
0x2d → 2 `0x1b8b48`, else 2 `0x1b2a08`. A set return page `0x1ba260` wins (kind `0x1ba264`; 0xb/0xf restore
`0x184894 = 0x1ba254`). Current page `0x1ba174` = target `0x1ba178`. Saves camera 0x167240 (0x1ba1b0) and FOV
0x16cf70, FOV = 0.63 (0x3f2147ae); menu lighting 0x1806c0..; `DownloadFrameBuffer(0x1ba180, FBP 0x15ee84, 0x13e504<<11)`
(512×416×4, once); map kinds 0xb/0xf use the big layout (VRAM 0x40000 + 4×0x4000 at 0x1848b0..c4, `FUN_0025a8d0`,
`HudMoveBank(1)`), others `HudMoveBank(0)`; saves equipped 0x141660[4]; spawns **14 class-0x472 mobys**
(`SpawnHandGadgetMoby`, `0x1ba310[14]`, update `MenuMobyUpdate` 0x309898, at the camera position, rotation 0) each
`hard_cut(page.seq[i], nframes−1)`. The kept target then triggers the open transition on the same tick.

**Structs.** Page: `+0x00 s32 seq[14]`, `+0x38 parent`, `+0x3c kind`, `+0x40 focus`, `+0x44 widget[14]`, `+0x80 pending
focus`. Widget: `+0 update` (returns ≠ 0 = close), **`+4 draw`**, `+8 enter`, `+0xc leave`, `+0x10 draw flags` (1 draw
direct, 2 3D pass, 4 hidden), `+0x14 moby` (= moby slot of the widget index, written on each transition), `+0x18 x, +0x1c
y, +0x20 w, +0x24 h`, `+0x30..` type data. List: `+0x30 flags, +0x34 items, +0x38 above, +0x3c below, +0x40 cursor`.
Filler widget 0x1b2878 has flags 4. Item (12 bytes): `{s16 label, s16 action, u32 arg, s16 sublabel@+8, s16
hl_timer@+10}`, ends at label 0.

**Start root** `0x1b2a08`: kind 2, seqs [14..20, 7..13], no parent, focus W0; 7 one-entry lists, update 0x28e600, draw
0x28ebd0, flags 4 (large font); W0..W2 also enter `0x2917d8` (action = 0 = disabled when byte `0x1413f4 == 1`, else 3).
W0 20194 Weapons → 0x1b3238 (kind 3) · W1 20195 Gadgets → 0x1b2d68 (4) · W2 20196 Quick Select → 0x1b3738 (5) · W3 20197
Items → 0x1b6138 (6) · W4 20198 Help → 0x1b3e18 (7, title 20318) · W5 20199 Options → 0x1b4dd0 (8, title 20253) · W6 20418
Goodies → 0x1b6ca0 (9, title 20419). Moby slot 6 is skipped everywhere while `0x1ba248 == 0`.

**Other pages** [H data, M roles]: Weapons 0x1b3238 (3D model 0x297d70/0x291800, icon grid 3×5 0x28f260/0x291350 with
10-byte cells at 0x1b36a0, name label from 0x179f40 + 0x4c·id, hints 20204 "✕ Equip" / 20192 "△ Exit"); Gadgets 0x1b2d68
(4 grids, labels 20200..20203 Hand/Back Packs/Head/Foot); Quick Select 0x1b3738 (0x2901a8/0x294528 + grids); Items
0x1b6138 (grid, `DrawItemsMenu` 0x292528, image widget 0x2937d0/0x293d50); Help 0x1b3e18 (list 0x1b4018 wrap: 20437,
20209, 20210, 20194, 20195); Goodies 0x1b6ca0 (list 0x1b6e78 flags 0x1080: Skill Points → 0x1b71c0, 20421 Credits action
11, Cheats → 0x1b7fb0, Cinematics → 0x1b8250, In-Level Movies → 0x1b8560, four locked entries action −1). Grid icons
5990x (+1 equipped, +2 unusable; `0x1ba2a4/a8`); selected cell rect colour `(((t&0x3f)−0x20) tri-wave + 0x40)·0x10202`,
alpha 0x80. **Options** 0x1b4dd0 (kind 8, seqs [67..72, 20, 7..13]): W0 title 20253 (flags 0x200f, enter 0x2904e8), W1
label, W2 image 0x2937d0, W3 list 0x1b4f78 (flags 0x1000): 20254 HelpDesk → 0x1b5040 · 20255 Save (action 4 →
`mode_freezeInit(3, 0x1b5b48)`, `0x15eeb4 |= 2`) · 20256 Load (5 → 0x1b5bd0, `|= 4`) · 20258 Sound → 0x1b5a18 · 20259
Camera → 0x1b5ee0 · 20260 Subtitles → 0x1b5788 · 20262 Quit Game → 0x1b6060 (Save/Load go straight to the page when
`0x15eeb0` ∈ {1, 0x10}) [M]; W4 hints 0x1b4090 (flags 0x4002, items 0x1b4068 "✕ Select / △ Exit").

**Generic keys** (focused widget's update): Start/Select/R3 (`0x13cb04 & 0xd00`) → return ≠ 0 → close (`0x1ba17c = 1`,
or 2 in kinds 0xf/0x10) unless `0x1ba294`; △ (0x10) → target = parent, or close without one; ✕ (0x40) → item action.
List flag 0x20: those keys first set `0x184894` = current level. Label update (`0x28d788`) does the same keys when focused.

**List widget** update `0x28e600` (earlier doc name `MenuSetPostAction`): hl_timer per item: focused widget and cursor → +1 (no
clamp); else `min(t, S)` then −1 floored at 0, `S = ScaleTicks(*0x160274 = 10)`. Actions on ✕: 0 disabled (nothing), 2
"????" (20308) + sound 2, 3 go to page arg, 4/5 freeze kind 3 (above), 6 → post 5 scene, 7 → 3, 8 → 4, 10 → 6, 11 → 7
(each: `0x1ba254 = arg`, return page `0x1ba260` = current, sound 0; 6/7/8/10 set `0x1ba264`), 9 language `0x15ed88 = arg`.
Cursor: keys from `0x13cb04`, or `0x13caf4` with flag 1 (buttons only); Up 0x1000 (or L1 4 with flag 0x100): cursor 0 →
wrap to last with flag 0x1000, else page pending focus = above; Down 0x4000 (R1 8 with 0x100): next item's label and
action both ≠ 0 → +1, else wrap to 0 (0x1000) or pending = below. Moves (or a pending focus) → sound 1, and with flag 0x20
`0x184894 = (*0x1602a0 = 0x13d510)[cursor]`. **No cursor sprite**: colour `FUN_0028f0e0(t)` = `FastTweenColor(t ≥ S ?
1 : 1 − (S−t)/S, 0x80ffa888 → 0x8020ffff)` (light blue R88 GA8 BFF → yellow RFF GFF B20); action 0: 0x80303030, or
0x80006060 when selected; flag 2: fixed 0x80ffa888.
**List draw** `0x28ebd0`: size 12 regular (FX 1, glyphs 0x1c35d0), flag 4 → 14 large (FX 3, 0x1c3d10), flag 8 → 10 small
(FX 2, 0x1c3970); ALPHA 0x44, TEST 0x2004b. Step = `h/(n+1)`, or size+3 with flag 0x10; first y = step − size/2 − 1.
x = w/2 − width/2, 4 with flag 0x40, `w/2 − maxw/2` with flag 0x4000 (maxw = widest label, ≤ w). Flag 0x20000: when
maxw + 6 > w and not flag 8 → set flag 8, return 1 (no blit this frame). Per item: shadow black 0x80000000 at (+2,+2)
(0x160278/7c) with colour codes off (`0x15f45c = 0`), then the label (flag 0x80: codes off too); a sub-label (+8) one
step lower the same way; next item one step further. Returns 2 (1:1 blit).
**Checkbox** `draw_menu_selection_marker` 0x2932f0: 11×11 rect 0x80ffa888, inner rect 0x80100808, checked: icon 59806
frame 1 30×30 at (x−13, y−18); used by list flags 0x200 (skill points 0x13d408) and 0x800 (language).
**Label** (update 0x28d788, draw 0x28dd30, enter 0x28dd20 sets +0x44 = −1): content by flags — 4 fixed id +0x34; 0x20
level−1; 0x40 `0x184894−1`; 0x80 focused list cursor; 0x100/0x1000/default grid cell (id = `*(table +0x34 + (idx·stride
+0x38 & ~3) + variant·4)`). Window x 0..w−4, y 4..h−4 (`*0x160318` = 4), anchor 4 (flag 1: w/2), y 4 (flag 2: h/2), line
flags 8|1|2; fonts regular / 8 large / 0x10 small. Colour `FUN_0028f0e0(+0x44, mix(0.5, 0x80100808, 0x80ffa888),
0x80ffa888)`; +0x44 += 3/tick while the content is unchanged; on change it falls 3/tick (clamped to S first), then
switches: a cross-fade. Shadow (+2,+2) black. Flag 0x200 prefixes "Planet %s" (20172 via 0x160340) except ids 0x4ed2,
0x4ed9, 0x4edd. Overflow (not flag 0x2000): flag 0x400 auto-scroll +3/16 px/tick (+10/16 with L2) and a second copy.

**Transitions** `0x28c990` (when target `0x1ba178` ≠ 0): 1) sound **3** if target == current (open) else **4**, via
`fun_0022da68(n, 0x11, moby0)` (class-0x472 sound table: 0 confirm/toggle, 1 cursor, 2 denied, 3 open, 4 page change);
2) every widget's leave; 3) mobys: `back = target == current.parent`, inverted on a self-transition (the open) unless
current ∈ {0x1b7670, 0x1b8250, 0x1b8560, 0x1b6878, 0x1b7d70, 0x1b6ca0} or kind 0x23; back: `hard_cut(current.seq[i],
last)` at speed **−1.0** (moby+0x58); forward: `hard_cut(target.seq[i], 0)` at +1.0. So **opening a page plays its seqs
reversed from the last frame**; each widget +0x14 = moby i. 4) kind 1 for **12 raw ticks** (`0x1ba184 = 0xc`, not
scaled), current = 0: only mobys and panel rects draw. 5) then current = target, kind = page kind, every enter(w, 0).
**Every tick**: `0x1ba280++`; `FUN_00298f80` (`CheatEntryUpdate`, earlier doc name `MenuInput` [M]: while held & 0xf == 6 (R2+L1) it records 20 presses
Up 0 Down 1 Left 2 Right 3 □ 4 ○ 5 against 0x1ba020); widget updates (all 14; a non-zero return closes); pending focus:
leave(old, 1), focus = pending, enter(new, 1); moby update `FUN_002793d8`; `sound_update`.
**Close** (`0x1ba17c ≠ 0`, `PageMenuClose` 0x28c6c8) only once `0x1ba280 ≥ 10`: widgets' leave, hand/back/head/feet
re-requested if changed, HUD bank restored, mobys freed, health + bolts re-shown (180), kind **0x14** for **2 ticks**
(snapshot + darkening only), then camera/FOV restored, `UpdateFog`, post-action: 1 resume; 2 `ShipTravelTo(0x184894)`
(music stays paused); 3/4/6 fade 16 + movie `0x2acf50`/`0x2ad050`/`0x2acfe8(0x1ba254)`; 5 fade 16 + scene
`0x2ac330(0x1ba254)`; 7 fade 16 + slideshow (mode 7).
**Game tick** [H]: `0x15f5cc` and play time `0x15eea4` advance only in `FUN_002ab960` (called by modes 0, 2, 6, 5 and
0x2a33b0); mode 3 never calls it. The VSync counter `0x15f3f8` keeps running (blinks, scrolling).

**Render** `PageMenuDraw` 0x28d080(0): 1) snapshot upload (`0x2b4d38`, world not re-rendered); 2) full-screen black
alpha **0x30** (packet 0x13cc90; mode 4 over the menu: 0x40); in kind 0x14 nothing else. 3) `TEST_1 = 0x5360b`, the 14
mobys `DrawMobyList(m, 1)` (enable array 0x1b2840 all 1). 4) per moby: pvar corners [+0x00] and [+0x30] = joints 0 and 3
(`MenuMobyUpdate` → `fun_0023a318` → `MobyGetBoneMatrix(m, 4, joints {0,1,2,3} at 0x161fe0, pvar)`) projected by
`MobyScreenRect` 0x2ada30 (`(p − cam 0x167240)·1024`, matrix 0x167200, scales 0x16d050/54, `·0.25 + centre
0x13e508/0c`) → x, y (+1), w, h into pvar+0x50 and widget +0x18..+0x24; 5) navy rect **0x80100808** (`*0x160270`) from
(x+1, y+1) to (x+w−1, y+h−1). 6) widgets in two passes (pass 0: draw flags & 2, 3D; pass 1 the rest; flag 4 hidden;
flag 1 draw direct): render to a `2^u × 2^v` target (≥ 128 each, u+v ≤ 17, v reduced) cleared to 0x80100808
(`0x251838`), draw in panel-local pixels, then `DrawTexturedQuad` onto the rect with ALPHA 0x8000000064 (opaque copy):
draw returns 2 = 1:1 w×h, 4 aspect crop, 8 centre crop, 0x10 whole texture, 1 none. The rect loop writes the widget only
while a page is current (`0x1ba174 ≠ 0`); during a transition only the mobys and the navy rects draw.
7) Last (kind ≠ 0x14, with or without a page), for every slot i < 14 with 0x1b2840[i] ≠ 0 (slot 6 only with Goodies)
**`fun_00223e28(0x1ba310[i])`** (boot 0x223e28, level01 copy 0x297830; read from the disassembly) [H]: on the pvar
rect (+0x50..+0x5c; skipped unless x < 0x200, x + w ≥ 0, y < 0x1a1 (PAL 0x1c1), y + h ≥ 0): CLAMP_1 = 0 (REPEAT),
ALPHA_1 0x44; `fun_00200080` icon 0xe99e frame 7 (a 128×128 black vignette) over the rect, alpha 0x80; the noise
burst in pvar +0x48 (running) / +0x4c (counter): idle → `rand() % 2000 == 0` starts it (counter 0); running →
counter += 2, u = `rand() % 200`, v = `rand() % 200`, ALPHA_1 0x68 with FIX = `min(2·(0x80 − |c − 0x80|), 0x80)`
(`subtract_integer_with_clamp` 0x221110 is abs: 32 frames in, 64 held, 32 out), FX 0x1a `DrawTexturedQuad(x, y, w,
h, u, v, w, h, 0x808080)`, stopped once c ≥ 0x100; then ALPHA_1 0x44, the scan lines FX 0x1c `(x, y, w, h, 0, 0, w,
3h >> 1, 0x50606060)` every frame; the glass FX 0x19 `(x + 1, y + 1, w + dw, h + dh, 1, 1, 0x3e, 0x3e, 0x80808080)`,
dw / dh −2 below 0x4c pixels, −1 below 0x97, else 0. Per panel, on the game's `rand` stream. Port:
`rc_game::menus::screen_static` (shared with the vendor's monitors, interaction.md §9.3) and the HUD's static layer
(`hud_render::Hud2dHook::statics`, over the Weapons / Gadgets pages' 3D views). The boot's own copy of this draw,
0x2196b8 (called from `transition_default_draw` 0x1eb410), also calls it [L: the out-of-level menus; not ported]. Navy rect vs 3D frame
layering: the mobys are queued first (`DrawMobyList` into the VU1 chain, before the rect loop) [M: that `fun_00200e08` goes
into the same chain is not traced]; seqs 7..13 (one frame, rate 0: the advance never steps) park the unused slots off screen.

**Frame mobys (class 0x472) in detail** [H]. Class 0x472 = o_class 1138, an ordinary class of every level's moby table
(`moby_class/1138`): 5 joints (root + 4 corners), 1 high / 1 low LOD packet, no metal, scale 0.30554, 216 seqs (0..13 one
frame, the rest 7). `SpawnHandGadgetMoby` 0x298e98: `CreateMoby` if `0x198040[0x472] ≠ −1`, +0x32 = 0xff, +0x30 = 0xff,
+0x20 = 0, +0x31 = 1, `MobyBuildMatrix`, `pack_render_command_fields(m, 0x202020, 0xe, 0xe, 0)` 0x2650b0 → moby+0x38 =
light sets 14/14, fade 0, ambient 0x20 0x20 0x20. **Menu light**: `FUN_0028c128` rewrites set 14 (0x180340 + 14·0x40 =
0x1806c0): colour A = quadword 0x160290 (0.9, 0.9, 0.6, back factor −0.25), direction A = `VecScale(1.0, 0x160280)` =
(0.7155, 0.5367, −0.4472), colour B = direction B = 0. **Menu camera**: `FUN_0028c128` saves 0x167240..0x16725c and FOV,
sets FOV 0x16cf70 = 0.63 and the view-context fog 0x16d0d8 = 0, 0x16d0dc = 524288, 0x16d0e8 = 255, 0x16d0ec = 0, calls
`UpdateViewContext`, `fun_00218d10` (0x28bee8: position 0x167240 = (256, 256, 64), rows 0x167450 = identity) and
`BuildRotationViewProj` 0x218a48 (0x16c4ec = 0 → the rows, not the Euler angles). The camera is therefore fixed, not
derived from the gameplay camera; each moby is placed at that same point with rotation 0 and draws in front of it (bsphere
x ≈ 5 units). 0x167200 = `sceVu0MulMatrix(P, V)` with V the identity camera's view rows, i.e. rows (P2, −P0, −P1, P3) where
P = 0x16d000 with rows 0/1 × 4 (0x16d080/84): x = −(4·0x16d000)·y, y = −(4·0x16d014)·z, w = (1/32)·x.
**Per tick** (`MobyUpdateLoop` 0x2793d8: `MobyAnimAdvance` unless mode & 0x40, update callback, `MobyBuildMatrix` unless mode
& 4): `MenuMobyUpdate` 0x309898: if +0x70 & 2 (wrapped) then t = (speed > 0 ? 0 : 1.0), speed = 0 — a forward play stops on
the last frame, a backward one on frame 0 (key B at t = 1); then `fun_0023a318` 0x3098f0: `MobyGetBoneMatrix(m, 4, ids
{0, 1, 2, 3} at 0x161fe0, pvar)` 0x264630 = `fun_002106f8` 0x268c28 (same marking / partial evaluation as `fun_00210850`
0x268d80, `fun_002109b8`, but it copies only row 3 (SPR +0x30) of each list's last joint = the joint translation), then per
point `vmulx.xyz` by `moby+0x2c · (1/1024)`, the rows (`vmulax..vmaddw` 0x2215e0) and `vadd.xyz` position; lists 0..3 end on
joints 2, 1, 4, 3; pvar +0x40/+0x44 = |c1 − c0|, |c2 − c0| (unused by the menu). **`MobyScreenRect`** 0x2ada30 (a = corner 0,
b = corner 3): `vsub.xyz` 0x167240, `vmulx.xyz` 1024, w = 1, `fun_001f9d20` (0x221608, 4-row VU0 chain) with 0x167200,
`div.s` 1/w, x·(1/w)·0x16d050 (1024), y·(1/w)·0x16d054 (832), `cvt.w.s` (truncating) of `X_a·0.25 + (float)0x13e508` (256),
`Y_a·0.25 + (float)0x13e50c` (208), `(X_b − X_a)·0.25`, `(Y_b − Y_a)·0.25`. Root page settled (Novalis): x 155, w 203, h 35,
y 44/93/144/193/242/292/342 (stored +1).

**Options sub-pages** (their widgets are W3; all use the generic keys):
* HelpDesk 0x1b5040 (kind 0x13) and Subtitles 0x1b5788 (0x1b): toggle list update 0x294830, draw 0x294a38 (Lombyte
  `DrawCheatsMenu`: generic). Entries 0x14 bytes `{label, u8* flag, on_id, off_id, flags}`; HelpDesk 20383 Voice →
  0x15ee1c, 20384 Text → 0x15ee1d; Subtitles 20260 → 0x15ee40. Cursor +0x38, Up/Down no wrap, sound 1; ✕: sound 0, byte
  flips (entry flag 1 = cheat path, unused here). Draw: rows `h/(n+1)`, first y `h/(n+1) − 8`; label regular at (12, y)
  0x8020ffff selected else 0x80ffa888; value right-aligned at (w−12, y) 0x80ffa888; no shadow.
* Camera 0x1b5ee0 (0x1d): update 0x294cc0, draw 0x294e68; entries 0x18 `{label, u8*, str[4]}`, count = non-zero strs;
  ✕: `v = (v+1) % count`, sound 0. 20386 Left/Right movement → 0x15ede0 (0 Reversed, 1 Normal), 20387 Up/Down → 0x15eddc,
  21019 rotation speed → 0x15ede4 (Slow/Medium/Fast). Layout as the toggle list.
* Sound 0x1b5a18 (0x20): `SoundOptionsMenu` 0x290538 / `DrawSoundMenu` 0x290808; cursor +0x40, Up/Down wrap mod 3;
  held Right/Left (`0x13cb00`, no edge) ±3/tick clamped 0..0x400 on 21010 SFX `0x15edf0` or 21011 music `0x15edec`
  (**342 ticks** for the full range); ✕: row 2 toggles 21012 stereo `0x15ede8` (21013 MONO / 21014 STEREO), any row calls
  0x12e240 + sound 0; mixer 0x13e598.. rescaled [M]; leave 0x290508: `0x13e5a0 = sfx·8/10`. Rows y = h/4, h/2, 3h/4;
  label right-aligned at (w/2−8, y−8); frame (w/2+7, y−8)–(w−63, y+8) 0x80696969, inner (w/2+9, y−6)–(w−65, y+6)
  0x80383838; fill 59806 frame 8 (SFX) / 9 (music) from (w/2+9, y−6) to (w/2+8 + (w/2−74)·vol/1024, y+5); value text regular at (w/2+8, 3h/4−8).
* Quit 0x1b6060 (0xd): update 0x2921d0, `DrawQuitGameMenu` 0x292298: 20333 in a regular window {y 0..h, x 4..w−4, anchor
  w/2, y 6, line 16, flags 1}; 20287 "Quit Game?" centred at (w/2, h−64); 21067 "△ No" at (x, h−40), 21071 "○ Yes" at
  (x, h−20), x = (w − max width)/2; all 0x80ffa888. ○ quits: `0x13d384 = 0, 0x15f5c0 = −1, 0x15f5d8 = 1, 0x15f570 = 1`.
* Language is not in the in-level Options (only list 0x1b8fe8, action 9, in front-end page 0x1b8b48); no vibration option.

**Map** 0x1b3998 (Select; kind 0xb, seqs [21..25, 5..13], no parent, focus W2): W0 label flags 0x4b (large, centred,
`0x184894−1` → location 0x1c22c0 + 12·lvl), W1 0x24b ("Planet %s", planet 0x1c22c4 + 12·lvl; not for Nebula G34, Oltanis
Orbit, Veldin Orbit), W2 map widget **update 0x28f868** (Lombyte `DrawMapScreen` wrong) / draw 0x292d38, W3 globe
0x294258, W4 missions 0x292d70. W2 keys (not focus-gated, flag +0x34 & 0x40 = passive): generic close/△; ✕ → page
0x1b3bf8 (kind 0xc missions); ○ (dest ≠ 0) → Infobot movie (post 3, return 0x1b3998 kind 0xb); R1/L1 → next/previous
entry of the unlocked list 0x13d510 (sound 1); then it streams/composes the destination map (tables 0x1383a0/0x138438, fog mask 0x141ec0 + 0x800·lvl) [M].
**Planet select** 0x1b6508 (kind 0xe → 0xf, seqs [99..103, 5..13], focus W1): W0 title flags 0xf 20208 "Galactic Map";
W1 list (update 0x28e600, draw 0x290eb0, enter 0x295208, items 0x1ba838 BSS, flags 0x139 = buttons only, small, step 13,
cursor → 0x184894, L1/R1). Enter: one item per id of `0x13d510` (≤ 20, acquisition order) `{label u16 0x1c22c0 + 12·lvl
(location, 20154 "Tobruk Crater"), action 1, 0, sublabel u16 0x1c22c4 + 12·lvl (planet, 20173 "Novalis")}`, cursor = the
index of `0x184894` (else 0), flag |= 0x8000. The list (appended by `FUN_002756d0`, Infobot) sets `0x13dd40[lvl]` =
known; `0x13dd58[lvl]` = visited. Draw 0x290eb0: window x ..w−2, y ..h−4, line 12, y = step − size/2 − scroll; label at
x 4, sub-label x 20 [M], 8 px gap; colours 0x8020ffff selected / 0x80ffa888 (disabled 0x80006060 / 0x80303030), selected
with codes off; keeps the selection visible: scroll −4 when its y ≤ 3, +4 per tick when it passes y_max (jump with
0x8000, which also returns 1 once). W2 galaxy map update 0x295310 (✕, not focus-gated → page 0x1b6878), draw 0x295338 →
`FUN_002252d0` panel-local: black fill; 59802 frame 14 stretched w×h; CLAMP repeat, frame 15 with u offset `0x15f3f8 &
0xfff`; levels 1..19 with x ≠ 0 at `0x1c23b8 + 16·(lvl−1)` `{x, y, dx, dy}` (Novalis (80,95), Kerwan (90,65), Veldin
(35,80); PAL y·448/416): visited → 10×10 dot 59802 frame 12 at (x−5, y−5); known only → dot on while `t % (22+8) < 22`
(ScaleTicks, t = 0x15f3f8); selected → leader line from 8 px off the dot to (x+dx, y+dy), then horizontal under the
name (width signed by dx), name regular at y+dy−`*0x15f650` (17 in the overlay; the report read 11), all first in black
at (+1,+1) then 0x80f0f0f0; ring 59802 frame 13 20×20 at (x−10, y−10). W3 hints 0x1b4068. W4 streamed planet picture
0x293398/0x293670 (index 0x184894, sectors 0x138308) and W5 0x1b6810 (map update only, flags 0xc0) [M].
**Confirm** 0x1b6878 (kind 0x10, parent 0x1b6508): W0/W1 labels as the map; W2 `DrawMissionsMenu2` 0x293090 (update
0x28fec8: L1/R1 cycle known planets); W3 globe 0x294258 (3 layers 128×128, angle t + dest·0x2ab, stations 6/0xd/0x11
blink, radii 0x1c24e8); W4 gold bolts 0x292980 (Lombyte `DrawGBsShipMenu` wrong: spinning moby + "Found: N of M", N =
non-zero bytes 0x14bec0[4·dest..], M = 0x1c4e08[dest]); W5 hints 0x1b2988 + keys 0x295370: ✕ sound 0 + close (kind 0x10
→ post-action 2), ○ Infobot movie `FUN_002acf50(dest)` back to this page in kind 0xf, △ back to the list, Start: dest =
level, close. In the list, Start/Select/R3 or △ also set dest = level (flag 0x20). **Result**: close from kind 0xf/0x10 →
`0x1ba17c = 2` → `FUN_002a2848(0x184894)`: dest == `0x15ed84` → landing (`0x2a24b8(8)`), else `memcard_Save(0, dest)`, `0x15f5c0 = dest`, fly-away (§5).
## 4. Gadgetron vendor (mode 5) [H unless noted]

**Entry.** Vendor moby class 11:
* Hero within range, facing it (< 90°), not in state 0x1d/0x32 → help 21475 "△ Activate Gadgetron Vendor".
* △ → `FUN_002ae1a0(vendor)`.
* The PDA gadget (hero code 0x240ed8) calls `FUN_002ae1a0(0)`. That spawns a class-11 vendor in front of the camera and
  sets **remote mode** `0x1ca980 = 1`: ammo only, at the higher PDA price, no camera fly, no hologram.

**Item list** `FUN_002adef0` → `0x1caa10[n]`, 5 ints each `{item, is_ammo, locked, 0, holo_class}`, count `0x1cab50`.
* Source 1: the global stock `0x15edd0[12]` (save data, 0xff = empty). Low 6 bits = item, bit 0x40 = owned (sell ammo
  only if the item has an ammo price).
* Source 2: an ammo entry for every owned item (`0x13d4c0[i]`, i < 37) with an ammo price that is not already listed.
* Remote mode lists only ammo.
* At level load `FUN_00251da0` drops unknown items and adds the planet's new item `0x1c4318[level]`, e.g. level 1 → 16
  Pyrocitor, 3 → 15 Blaster, 4 → 20 Glove of Doom.

**Item tables.**
* Weapon record at `0x1c4530 + 0x18·id`: `+0` price, `+4` discounted price (used when flag byte `0x13d4e3` ≠ 0;
  identity of the flag [L]), `+8` u16 ammo unit price (vendor), `+0xa` u16 ammo unit price (PDA), `+0xe` u16 max ammo.
* Current ammo is `0x13d428[id]` and bolts are `0x15ed98`.
* Examples (price / ammo price vendor, PDA / max ammo):

| id | item | price | ammo price (vendor, PDA) | max ammo |
|---|---|---|---|---|
| 10 | Bomb Glove | 0 | 5, 25 | 40 |
| 15 | Blaster | 2500 | 1, 10 | 200 |
| 16 | Pyrocitor | 2500 | 1, 5 | 240 |
| 17 | Mine Glove | 7500 | 5, 15 | 50 |
| 19 | Tesla Claw | 40000 | 1, 10 | 240 |
| 23 | R.Y.N.O. | 150000 | 20, 50 | 50 |

* Item info at `0x179f40 + 0x4c·id`: `+0` name id (20039 "Blaster"…), `+0x10` moby class, `+0x38` icon (60000+id).
* `0x1c2588 + 0x14·id`: hologram class and scale floats, and `+0x10` description id (21083..21095, e.g. "BLASTER!  POINT
  IT…").

**Substates** `0x1ca940`, timer `0x1ca944`:
* **0 fly-in.** Fade `0x15f3fc −= 0.34`/tick. The camera tweens to the vendor's front (`FUN_00316ef8` → 0x1ca9a0).
  Vendor seq 2, speed 0.5. The world and tick run. After 40 ticks → 1 (vendor seq 3 blend 8, sound 4). The next render
  hides the vendor, draws the world once more, snapshots it (`FUN_0021b5c0`) and builds the screen mobys.
* **1 menu.** No world update and no tick.
  * ≤ 7 items: Left/Right (0x13cb04, edge-detected against `0xffffb6b0`) move without wrapping. The icon strip is at
    x = 12 + 56·i.
  * ≥ 8 items: a wrapping carousel. The scroll offset `0x1ca98c` moves ±56 px and eases back 4 px/tick.
  * Each move plays sound 1, puts the description in the ticker and re-shows the ammo HUD slot (`HudShow(0x30,…)`).
  * ✕ on an unlocked entry → buy flow. △ (and no voice line streaming) → exit after 8 ticks.
* **Buy flow** `FUN_002af7e8` (`0x1ca99c` 1..4). A quantity popup moby, class 0x471, opens (seq 0, sound 2).
  * Ammo: quantity `0x1622b4` (overlay .bss) from 1. Left/Right step it with auto-repeat (held > 16 ticks: every 8th tick; > 48 ticks:
    every tick). Capped at `min(max − cur, bolts / unit_price)`.
  * ✕ confirms, △ cancels; the popup closes over 8 ticks, played backward.
  * Buying a weapon: `FUN_00275760(id,1)` gives it and `bolts −= price`. Id 24 (Drone Device) is a special case.
  * Buying ammo: `FUN_002494d8(id,qty)` returns the overflow; bolts −= unit·(qty−overflow); stats `0x13dd70[id]` += bought.
  * Either purchase → ticker 20319 "THANK YOU", sound 7, list rebuilt. Can't pay → 20320 "You can't afford that",
    sound 0.
* **2 leave.** Vendor seq 4 at −0.5, 40 ticks, then seq 1 and detach the manipulators.
  * If a weapon was just bought and `0x1ca4a0[item] ≥ 0`: `FUN_002ae7f8` → **substate 3**, the weapon demo scene.
    It streams a camera/moby scene; fade out ends it → `FUN_002ae660`.
  * Otherwise `FUN_002ae660(0)`: camera back to the hero, sound 6, mode 0, music unpaused.

**Drawing** (`FUN_002b4020` → `FUN_002b3130`) [M: 0x2b3130 does not decompile; read from its call list].
* Several **512×128 off-screen targets** are drawn and then mapped as textured quads onto the vendor model's monitor
  bones (`FUN_00264630(vendor, 3, …)`, `fun_001f55d8` quads RGBA 0x80808080):
  * Ticker `0x2b1a48`: scrolls 2 px/tick. When empty it picks a random line from `0x1ca538[24]` (20321.. e.g. "HEY
    YOU!  YEAH, YOU WITH THE ROBOT…").
  * Salesman video `0x2b1b58`: class 0xc moby. Every 600 ticks it plays a random voice stream
    10000 + 6·(3·r+1) + planet.
  * Icon strip `0x2b1c10`: icons 48×48, y 6. The selected cell has a pulsing frame of alpha `((t&15)·4−32 clamp)+0x40`.
    Ammo entries use icon frame 2, weapons frame 0.
  * Item panel `0x2b1f08`: the item model (class at `0x179f50 + 0x4c·id` = item record 0x179f40 + 0x10) plus a background class 0xd. The name uses the small
    font at (6,8) in 0x80f0f0f0.
    * Ammo entries: 20317 "Ammo" at (24,24). The unit price is small-font right-aligned at (118,101); in PDA mode the
      vendor price is struck out with seven 1-px lines, colour 0x..959544, alpha 0x20→0x80.
    * Weapons: the price at (118,101) (grey 0x80808080 if discounted, and the discounted price at (118,85)).
  * Prompt `0x2b2688`: small centred at (40,20): 20192 "△ Exit", 21067 "△ No" or 21070 "✕ Yes" [M].
  * Button window `0x2b2430`: 21044 "✕ Buy" / 21043 "△ Back" [M].
* **3D in the world.** The **hologram** is item class at `0x1ca960+0x300` (+0x200 extra part from `0x1c94a0`) placed at
  the vendor offset. It spins `+0.05`/tick (angle `0x1622a4`, overlay .bss) and bobs `sin(t)/20`. Two class-0xd frame mobys turn at ±0.05.
  The hologram cone is `0x2b3cc8`: 4 quads, FX tex 0x18, V scroll +0.01/tick, ALPHA 0x4000000044. Glass quads `0x2b3700`:
  5 quads on the vendor bones, FX tex 0x19, TEST 0x32003.
* The HUD pass (`0x24fb50`) runs on top: bolt counter and ammo slot.

## 5. Ship menu (△ at ship → mode 6 → mode 3 page 0xf) [H for flow, planet page not yet extracted]

* **Ship update** `FUN_002a1c40` (class 531/532/533 = `0x160548[0x13e056]`).
  * Hero within 4 of the hatch point `0x1bdd00[ship]` → help 21476 "△ Enter ship". △ → `0x15f630 = 1`.
  * Special cases: story scenes 4/5/6 or movie 0xb run once through `0x2ac330`/`0x2acf50` before the ship becomes usable,
    guarded by flags `0x13d3d0..d3`; level 10 with `0x1413f4 == 1` runs scene 9.
* **Take-off** `FUN_002a27c8` → `FUN_002a24b8(0)`: mode 6, substate 0, fade 1.0, hero hidden, space-scene stream
  40000 + (ship+6) (or special ids).
  * Each tick: camera and ship keys from 32-byte frames (`parse_space_scene_chunk` every 0x60 ticks); fade −0.125/tick;
    the engine glow pulses.
  * ✕ or △ after 30 ticks skips to the end.
  * At the end the scene mobys are freed and it calls `0x28bf50(0)` with `0x1ba170 = 0xe` → planet-select page
    **0x1b6508** in mode 3.
* **Selection**: `0x184894 = dest`, `0x1ba17c = 2` → menu close → `FUN_002a2848(dest)`.
  * dest == current planet → landing (substate 8).
  * Otherwise `memcard_Save(0,dest)`, `0x15f5c0 = dest`, substate 3 (**fly-away**):
    1. Fade 12 ticks, then stream 0x9c4f + ship.
    2. Ship eases along path `0x1b0930[0x13e060]` for 120–150 ticks (cosine ease), then accelerates +0.8/tick up to 100.
    3. The trail is 32 samples.
    4. Fade +0.0625/tick near the end, then `0x15f570 = 1`, which leaves the main loop.
* **`DoSpaceTransition`** 0x2a68f8 (argument = `0x15f5c0`, the destination level):
  1. Stops sound and picks the ship model by story progress.
  2. Plays first-visit story transitions `fun_00231bd8(planet, a, b, 240 or 180 ticks)` (fade 6 or 12).
  3. `0x15ed84 = dest`, `FUN_002a5868` (substate 4 = random space-flight loop while the disc loads), level load.
  4. The new level starts in mode 6, substate 8 (landing) via `0x2a29c0`.

## 6. Infobot (class 750) [M]

Update `0x2fbf80`. It flies a path to the hero; within 2 units → state 8:
* Hides itself, unlocks the planet `FUN_002756d0(pvar[1])` and sets the mission byte `FUN_00265080`.
* Then chains, each waiting while mode == 2: in-engine scene `0x2ac330(pvar byte 0xc)` → **PSS movie**
  `0x2acf50(pvar byte 0xd)` (mode 1, movie lsn/size table 0x139388 NTSC / 0x139424 PAL, disc `mpegs/`) → a second scene
  `0x2ac330(pvar byte 0xe)`. A byte of −1 skips that step; the argument registers are lost in the decompile [M].
* Finally `memcard_Save(0,−1)`, `FUN_00277c38(pvar[1])` (planet banner 1009+ [L]) and the moby is deleted.
* Return to gameplay: mode 1 → `0x2ad2b8` → mode 0; mode 2 → mode 0 when the scene ends.

Help/tutorial boxes: hud_text.md §3.4 (`Help_Update` 0x225bd0). They run inside mode 0 and do not freeze the game.

## 7. In the port (2026-09-27)

* `crates/rc-game/src/menus/`: `mode.rs` (`Mode` 0..7/−1, `advances_tick()` = modes 0/2, `ModeState` with `0x15f5c8`); `quick_select.rs` (§2 exactly
  on the FPU model: hero part, update, draw → `MenuDraw`; constants, neighbour table (via `*0x15f738`), defaults and item fields read from the overlay
  / `ItemTables`; confirm writes `SessionState::temp_hand` (0x141408) and `Global::move_help[20]`); `pause.rs` (+ `options.rs`, `planet_select.rs`):
  §3's page machinery with records parsed from the overlay (`PageMenu::load`: 26 pages, 109 widgets on Novalis), widgets recognised by their level-01
  callback addresses, option pointers mapped to `Global` fields. Tests: slot positions, 27°/63° sectors and hysteresis, d-pad chains, tap/hold/double
  tap, overlay goldens, 12-tick transitions and seq direction, highlight timer, close timing, Goodies wiring, 342-tick sliders, planet list, blink 22/8.
* `crates/rc-engine/src/menu_render.rs`: per main-loop frame after the gameplay tick (which is skipped when `!advances_tick()`); `RC_PLAY_SCRIPT`
  indexed by main-loop frame. Background: a GPU copy of the entering frame (world + HUD, like `DownloadFrameBuffer`'s synchronous VRAM download at
  0x2b4c88), taken at the end of that frame's render graph from the main target's output attachment (the capture image, a screenshot's texture,
  or a window frame rendered offscreen on that frame and copied back), with the black 0x30 applied on the way by the GS formula on display bytes
  (`menu_snapshot.wgsl`, exact through a UNORM view); no CPU read-back, so it is on screen from the first menu frame on and frame-exact runs are
  run-to-run identical (frames 62/65/73/90 of `RC_PLAY_SCRIPT="60:press Start"`, 3 runs each). Shown by a UI node under the 2D pass; menu draws go through `hud_render::Hud2dHook` (HUD off / frozen in mode 3).
* **Frame mobys** (`rc-game/src/menus/pause/frame.rs`): spawn / `start` (transition) / `update` (advance, `MenuMobyUpdate`,
  corners via `rc_formats::moby_anim::{joint_translations, bone_points}`) / `free`, and `MenuProjection` (the camera constants
  derived with the PS2 float ops from the view-context defaults, NTSC) → `PageMenu::draw` writes pvar / widget rects and the navy
  rects from them exactly; `port_layout` is gone. Without class 0x472 no panels are placed (as the game). Test
  `root_page_panels_from_the_disc` pins the settled root layout. Render (`menu_render.rs`, "Menu layer"): an offscreen layer
  image composited between the world and the HUD composite: a `Camera2d` clears it to black α 0x30 and draws the darkened
  snapshot, a mode-3-only `Camera3d` draws the 14 `ExtraMobys` of class 1138 (light set 14 = the menu light, palettes from each
  slot's `MobyAnimEval`); the menu view is folded into the model matrices (`C_main · C_menu⁻¹ · model`) because other systems
  take "the" Camera3d's transform. Deviation: the level fog, not the menu's view-context fog, reaches these materials (≤ 1 step of F).
* **Port choices / stubs**: widgets drawn in place, scissored, instead of render target + 1:1 blit; lines as 1-px quads;
  unported pages / streamed images / globe / missions / cheat entry counted in `stub_calls`; post-actions other than resume logged; sounds only returned; the ring draws after the HUD (game: slot 3).

* **Gadgetron vendor (mode 5)**: `crates/rc-game/src/menus/vendor.rs` (list, substates, buy flow, screens as 2D panels) driven by
  `crates/rc-engine/src/interact_render.rs` from `menu_frame` (`Mode::Vendor`; the HUD ticks and draws over it); opened by the vendor
  class 11 through the "use" system (docs/plan/interaction.md: the context prompt 0x278f58 / 0x278eb8, which also blocks the ring
  through 0x15f594). Lombyte misnomers found there: `RaceTimerShow` 0x278eb8 = `PromptTick`, `HudRaceTimerDraw` 0x24c898 = the
  prompt's slot-12 draw, `OpenShipMenu` 0x279070 = `PromptRelease`, `DrawSpriteHelper_C` 0x2af7e8 = the vendor's buy flow.

## 8. Unknowns

* Navy rect vs 3D frame order (whether `fun_00200e08` shares the moby chain); widget-texture clear (0x251838) arguments; the other
  mobys `MobyUpdateLoop` ticks in mode 3 (the list is not filtered to the frame mobys).
* The Gadgets page (and the Weapons page's grids / name label / 3D Ratchet) is ported: docs/plan/gadgets.md §3. The
  rest of Weapons, Items/Help/Goodies internals only catalogued; meaning of `0x1413f4 == 1`; Goodies flags 0x15eea0 / 0x15ee20; the lost x anchor of
  the planet list sub-label; scroll u unit in 0x2252d0; whether 59802 frame 14 is HUD texture 235; `0x15f650` = 17 (overlay) vs 11 (report).
* Quick select: HUD/hero update order per tick; PS2 truncation of `(int)(74·fast_cos·1.05)`; meanings of 0x1413f4/f7/fc, 0x13f502, 0x15f594;
  readers of 0x15fa94/98/9c; the NTSC pause-menu writer of 0x141ea0; entry field +4; equip sound; the wrench (item 8, icon 0) is an empty slot.
* Kinds 0x21/0x23/0x2d entry points; vendor: discount flag `0x13d4e3`, `0x1ca4a0` demo-scene table, text ids in `0x2b2430`/`0x2b2688` (lost
  registers), monitor quads in `0x2b3130`; Infobot pvar bytes on Novalis; runtime values of 0x13d510/0x13dd40/0x13dd58.

## 9. Port-only settings ("Port Options")

Not in the game. Settings the PS2 never had (anti-aliasing first) live on one extra page, **Port Options**, reached from
pause → Options → "Port Options". It is built only from the game's own machinery, so it looks and behaves like the Options
sub-pages (`crates/rc-game/src/menus/pause/port.rs`, `PageMenu::install_port_page`):

* **Entry**: one item `{label "Port Options", action 3 → page}` inserted into the Options list 0x1b4f78 **right before Quit
  Game** (index 6 of 8), so Quit Game stays the last entry (and, with the list's wrap flag 0x1000, the one Up-from-top reaches)
  as on the disc; the list draws 8 rows with the game's own `h/(n+1)` spacing. The description label W1 (0x1b4fe8, flags 0x90,
  id table 0x1b4fc8 indexed by the list cursor) gets a patched copy of its table (`Label::table`) with the port description at
  the same index, so the table read never runs past the disc's 7 ids.
* **Page**: a page record (kind 0x7f, parent Options, focus the list) whose 14 frame-moby seqs and widget slots are those of
  Subtitles 0x1b5788 (≤ 2 rows) or Goodies / Cheats 0x1b7fb0 (3+: one wide panel, seqs 153..157, title / list / hints in
  slots 0 / 1 / 2). W0 is a clone of the model's title label (flags 0xf: large, centred) with the port title; the port list
  takes the slot of the model's own list (its focus) and the sub-pages' hint list (Subtitles' W4 0x1b2920, "✕ Toggle /
  △ Exit") the model's last slot. The port list is
  updated and drawn exactly like the camera list 0x294cc0 / 0x294e68 (rows `h/(n+1)`, first y `h/(n+1) − 8`, label regular at
  (12, y) 0x8020ffff selected / 0x80ffa888, value right-aligned at (w − 12, y) 0x80ffa888; Up/Down without wrap + sound 1; ✕
  cycles the value + sound 0; generic Start/Select/R3 close and △ → Options). Transitions (12 ticks, seqs forward / reversed),
  panels, fonts and colours are `PageMenu`'s.
* **Strings**: the port's own text, negative ids (−0x100..) resolved by `MenuAssets::msg` before the level's table (the
  disc's ids are 0..21500); "off" is the game's 20315. Records use addresses 0x7f00_xxxx, which no EE pointer can hold.
* **Rows** (`port::ENTRIES`, one entry per option): the graphics rows (Preset, Anti-aliasing: Original / off / 2x / 4x / 8x,
  Detail distance, Texture filtering, HUD; docs/plan/graphics_options.md) → `graphics::GraphicsSettings` (engine
  `menu_render.rs` syncs the rows with the resource around each menu tick; a change is saved; the anti-aliasing samples
  reach the cameras through `render_settings::RenderSettings::msaa`). Shadows: on / off. Aspect ratio: 4:3 / 16:10 / 16:9, Resolution: Window / 416p / 720p /
  1080p / 1440p / 2160p and Fullscreen: off / on → `display::DisplaySettings` (the game frame every camera renders into, presented
  on black at the window's size; keys `aspect` / `resolution` / `fullscreen` in the settings file). 16:10 / 16:9 are Hor+: the
  world projection's x tangent is the game's × 6/5 / × 4/3 and culling reads the live projection; the 512×416 screen maps to
  the frame's centred 4:3 box, the HUD layer is 51 / 85 game px wider on each side (its left/right-anchored slots move out,
  primitives spanning the screen stretch) and the menu pages stay clipped to the box (`Prim::boxed`). The world is not re-rendered under the menu (the snapshot is shown), so the new
  setting is visible from the first gameplay frame after the close.
* **Device support**: the row offers only the sample counts the GPU can use for every multisampled world attachment
  (`render_settings::SupportedMsaa`, detected at start from `RenderAdapter::get_texture_format_features` for
  `Rgba8UnormSrgb` and `Depth32Float` — the WebGPU guarantee {1, 4} when the device lacks
  `TEXTURE_ADAPTER_SPECIFIC_FORMAT_FEATURES`); ✕ skips the others (`PageMenu::set_port_choices`; Apple M-series: off /
  2x / 4x, no 8x). A stored or `RC_MSAA` / `RC_MSAA_SWITCH` value outside the set becomes the highest supported count
  below it with one warning line (8 → 4 on M-series); the file is not rewritten at start, only by a change on the page.
* **Persistence** (`render_settings.rs`): plain `key = value` text, `~/Library/Application Support/rerac/settings.toml` on
  macOS (`$XDG_CONFIG_HOME`/`~/.config` elsewhere, `%APPDATA%` on Windows), read at start, rewritten on change (unknown
  keys / comments kept). `RC_MSAA` / `RC_GFX_*` override the file at start; `RC_SETTINGS_FILE=<path>` picks another file, `=0` disables
  it; frame-exact runs (`RC_SCREENSHOT_FRAME` / `RC_DETERMINISTIC=1`) ignore the default file.
* `RC_SETTINGS_PAGE=0`: no entry, no page: every record is as read from the overlay (Options frames byte-identical to the
  game's layout). Without the variable, frames of every page but Options (8 rows instead of 7) are unchanged.

## 10. Help messages, pause pages, the in-game map: what is a system (2026-09-28)

Evidence gathered before the port (level01 addresses; "boot-hash-match" = the same object code as the boot ELF, i.e.
the engine's, compiled once for all levels).

| area | shared mechanism (one port) | per-page / per-level / per-object code (ported as what it is) |
|---|---|---|
| **Help messages** | **Yes, a system.** `Help_Request` 0x225818, `Help_Update` 0x225bd0 (boot 0x1fde90), size 0x225a98, draw 0x2266c0, the kill 0x2258b0, suspend / resume 0x225a28 / 0x225a88, the help log `FUN_00226a70` + its id table 0x1798d0 (`fun_001fecc8`), the records 0x141968 (help, chunk 16) and the bump rule. One state block 0x179890..0x1798cc. | The **callers**: every one is its owner's own code. Level 01 has 6 caller functions: the Novalis director 1341 `0x30acb8` (13 calls), the hero's `0x228498` (Hydro hint 0x4e2e, Quick-Select hint 0x4e26), the Pyrocitor `0x2cde98` (0x4e24), the Tesla Claw `0x2ce448` (0x4e30), class `0x30a6d0` (9000) and `mode_freezeInit(5)` (help log only, 0x4e2b). On the other 17 levels the directors (1413, 1324, 1342, …, level_scripting.md §3) are unique code. No shared "hint director". |
| help voice | The dialogue player 0x151720 (music_Update 0x27a688: request 0x1516ec, `PlayDialogue` 0x279cd8 → 30000.. = `help_audio[lang·150 + n]`, TOC field 0x1ab8 = 0x139638) — the audio system's; the help box drives it. | — |
| **Pause pages** | **The page machinery** (§3, ported) and a few **widget types used by several pages** (dispatch by callback): grid 0x28f260 / 0x291350 (Weapons, Quick Select ×2, Items, Gadgets), the **streamed image** 0x2937d0 / 0x293d50 (Items, Help, Moves, Help/Weapons, Help/Gadgets, Goodies, Skill Points, Options: 8 pages, TOC lump in +0x30), label / list (§3), the 3D item models 0x291c38 / 0x2919a0 (Weapons, Quick Select, Help/Weapons, Help/Gadgets), the icon list 0x28d818 / 0x28d9a8 (Help/Weapons, Help/Gadgets), the class-0x472 **menu sounds** `fun_0022da68(n, 0x11, moby)` (every page). | Page-only code: the Quick Select slot ring 0x2901a8 / 0x294528 (+ enter 0x2903d0 / leave 0x290468), the Items 3D view `DrawItemsMenu` 0x292528, the Help Log list (enter 0x290b70 / leave 0x290bf8 / text 0x290d40), the Controls image 0x294050 / 0x294198, the skill-point draw 0x295af8, the Weapons stats panel 0x28f7a0 / 0x292b60. |
| **In-game map** | **Yes, an engine system** (every function boot-hash-matched): the fog writer `FUN_0025c4f8` (hero update 0x228870 / 0x228000, every tick), world → map `fun_00208408`, the zone tiles `FUN_0025dd98`, the mask pack / unpack `fun_00207b08` / `fun_00207bb0` (chunk 3002 = 0x141ec0 + 0x800·lvl), compose `0x2053d8`, the map widget 0x28f868 (streams TOC field 0x820: 19 maps + 19 Map-o-Matic maps, chosen by owned[33] 0x13d4e1), pan / zoom `fun_00205440`, the draw `UNK_NoMapAvailable` 0x25b1c0 with its markers. | **Per-level data**: the zone table 0x183020 (19 levels × 16 zones × {z min, z max, flags, arg}) and a per-level predicate table 0x184410 (19 × 8 function pointers) whose entries are small hand-written map-pixel tests (half-planes `compute_cross_product_sign`, distances, hero state), compiled into the engine for every level. Ported as the game has them: one table of per-level predicates. |

**Reuse plan (Step 0).**
* Help: `hud::Help` (box states, sizing, draw) and `HudState::help_request` exist; `GameState` types chunks 16 / 17 / 18 /
  1010 / 1011; `hero::melee::bump_record` is the bump rule; `Interact::force_prompt` is `force_help_message`;
  `cinematic::{start_scene, start_movie}` are the kill's callers; `audio::class_sounds::level_sound::HELP_OPEN` the opening
  sound; `scene_command(Speech)` plays a VAG. New: `rc_game::help` (the one help state machine, gate, log, records mirror,
  dialogue-player view), wired into `Services` and read by the HUD draw; the box state moves there from `HudState`.
* Pages: `PageMenu` dispatch by callback, `gadgets.rs` (grid, preview, model), `options.rs`, `planet_select.rs`,
  `screen_static`, `screen_canvas`; new widgets are added to the same dispatch, no second page system.
* Map: `planet_select::map_update` (the keys) exists; the map image, fog, markers are new (`rc_game::map`), loaded through
  `rc_data` / `disc_source` like the other global lumps.

## 12. The pause pages and the menu audio (ported 2026-09-28)

Code: `rc-game/src/menus/pause/pages.rs` (the widgets below, dispatched by callback from `PageMenu` like every other
widget: no second page system), `rc-game/src/audio/scene.rs` (`AudioSystem::menu_open` / `menu_close`,
`pause_groups` / `continue_groups`), `rc-game/src/audio.rs` (`sound_update_with`), `rc-game/src/menus/mod.rs`
(`MenuSound::event`), `rc-engine/src/menu_render.rs` (the calls, `menu_sound_frame`, `MenuDraw::Image` → the HUD atlas
slots of `rc-engine/src/hud_images.rs`), `rc-formats/src/pif.rs` (the pictures). Tests:
`rc-game/tests/ui/pause_pages_novalis.rs` (9 tests, disc data), `audio::scene::tests::menu_open_holds_music_and_world_sounds_and_plays_menu_sounds`,
the page-machine tests in `menus/pause/tests.rs`, `tests/ui/gadgets_novalis.rs`.

**What the game does to the audio** (the play-test finding "the music keeps playing"). `EnterMenuMode` 0x28bf50 calls
`snd_PauseAllSoundsInGroup(0x1d)` (boot 0x12e3e8: 989snd groups 0, 2, 3, 4 hold where they are), `music_Pause(0)` and
`snd_FlushSoundCommands`. Every menu frame (`SceneController` 0x28c990) runs `MobyUpdateLoop` then **`sound_update`
alone** (the level's sound instances, 0x2a19a8, run only from the level update 0x256810, so no ambient sound starts
in the menu). The close (kind 0x14 at the end of its 2 ticks) calls `snd_ContinueAllSoundsInGroup(0x1d)` (0x12e418),
**`music_Unpause` unless the post-action is 2 (ship travel: the music stays held into the flight)**, the flush and
`sound_update`. Neither a duck nor a separate menu track: the music is paused and resumed. The vendor's
`music_Pause` / `music_Unpause` (mode 5) are the same helpers; its frame now also runs `sound_update` alone.

| address | what it does | ported / NOT ported / n/a |
|---|---|---|
| 0x28bf50 | `snd_PauseAllSoundsInGroup(0x1d)`: the handlers of groups 0, 2, 3, 4 stop (grains, LFOs), their voices hold (pitch, volume 0) | `AudioSystem::pause_groups` via `menu_open` (grain_vm `set_groups_paused`) |
| 0x28bf50 | `music_Pause(0)` | `AudioSystem::music_pause` via `menu_open` |
| 0x28bf50 | `snd_FlushSoundCommands` 0x12dc80 | n/a (the port's commands apply at once) |
| 0x28c990 | per frame: `sound_update` 0x2a0638 (slots, reverb command, `music_Update`), no sound instances | `menu_render::menu_sound_frame` → `AudioSystem::sound_update_with` |
| 0x28c990 | per frame: `PlayClassSound(3 / 4, 0x11, frame moby 0)` on a transition start (3 = the same page: the open; 4 = another page, △ back included) | `PageMenu::tick`, played by `menu_render::play_menu_sounds` |
| 0x28c990 close | waits for 0x1516d8 == 0 (no dialogue stream read pending) | n/a (the port's reads never pend) |
| 0x28c990 close | `snd_ContinueAllSoundsInGroup(0x1d)`: the held voices take their pitch and group volume back | `AudioSystem::continue_groups` via `menu_close` |
| 0x28c990 close | `music_Unpause` unless post-action 2 | `menu_close(unpause_music)`, false for `PostAction::ShipTravel` |
| 0x28c990 close | flush, `sound_update` | the close frame's `menu_sound_frame` |
| 0x28c990 close | camera restore, fog, resource tables, the post-action fades / movies | pre-existing (§3); the movie / scene post-actions G-UI-004 |

**Every menu sound.** `PlayClassSound(n, 0x11, moby)` (0x2a1618, boot-hash name `fun_0022da68`) with class 0x472's
table: 0 confirm, 1 cursor, 2 denied, 3 open, 4 page change. There is **no close sound and no separate back sound**:
△ back and Start / Select close go through the transition (4) or the close (none). All 31 call sites of level 01's page
code, by function:

| function | sounds | port | test |
|---|---|---|---|
| 0x28c990 SceneController | 3 open, 4 page change (forward and △ back) | `PageMenu::tick` | `open_transition_is_12_raw_ticks_with_the_seqs_reversed` |
| 0x28d818 icon list | 1 (Up / Down moved) | `pages::icons_update` | `help_weapons_icon_list` (none at the end) |
| 0x28e600 list | 2 (action 2), 0 (actions 4, 5, 6, 7, 8, 10, 11), 1 (cursor or pending focus moved) | `PageMenu` list update | `highlight_timer_and_focus_moves`, `help_page_streamed_image_states`, `goodies_skill_points_and_movies` (locked entry: none) |
| 0x28f260 grid | 1 (moved), 0 (✕ select), 2 (✕ denied) | `gadgets::grid_update` | `gadgets_page_equips_the_packs` |
| 0x28f868 map widget | 1, 0 | `planet_select::map_update` (Part 3 extends it) | planet-select tests |
| 0x28fec8 missions | 1 (×2) | `planet_select::missions_update` | planet-select tests |
| 0x2901a8 Quick Select ring | 1 (R1 / L1) | `pages::slots_update` | `quick_select_assigns_slots_and_writes_them_back` |
| 0x290538 Sound options | 1, 0 | `options.rs` | options tests |
| 0x294830 toggles | 0, 1 | `options.rs` | options tests |
| 0x294cc0 camera | 1, 0 | `options.rs` | options tests |
| 0x295370 confirm page | 0 (×2) | `planet_select::confirm_update` | planet-select tests |
| 0x295770 Sketchbook pager (30 pages) | 1 | NOT ported (G-UI-002: locked Goodies pages) | — |
| 0x295858 Epilogue pager (12 pages) | 1 (×2) | NOT ported (G-UI-002) | — |
| 0x296990 / 0x296ce0 / 0x296fc0 memory-card pages | 1, 0 | NOT ported (G-SAV-002) | — |

The sounds reach the audio system as `MenuSound::event` (class 0x472, flags 0x11, one owner for all 14 frame mobys:
the owner only places and privileges the slot, and the sounds are 2-D [L]). Test: all five sounds get a slot and a
sounding voice while the menu holds the world's voices (`menu_open_holds_music_and_world_sounds_and_plays_menu_sounds`).

**The pages' widgets** (addresses level01; every function boot-hash-matched).

| address | what it does | ported / NOT / n/a | test |
|---|---|---|---|
| 0x2936e8 image enter | +0x44 = 0, +0x50 / +0x54 = −1, +0x5c = 0; the Controls widget shares it | `pages::image_enter` (Image and Controls) | `help_page_streamed_image_states`, `help_controls_page_two_pictures` |
| 0x293780 image leave | +0x44 = −1, indices −1 | `pages::image_leave` | `help_page_streamed_image_states` |
| 0x2937d0 image update | index by flags 4 (grid cursor), 8 (cell +8), 0x100 (memcard preview), else list cursor (0x4000: action 2 → 9), 0x2000 (skill point not earned → 0x1e); two-buffer states 0..6 | `pages::image_update`; flags 1 / 2 / 0x400 / 0x1000 are the front end's (n/a in-level); 0x100 NOT ported (G-SAV-002) | `help_page_streamed_image_states` (states 0→1→2, 3→4, 4→2, 4→5→6→2) |
| 0x293d50 image draw | state < 2: nothing (0x100: "Checking memory card" text, G-SAV-002); flag 4: only when the cell's item is owned / flag set; buffer A in states 2, 3, B in ≥ 4, over the panel, return 0x10 | `pages::image_draw` → `MenuDraw::Image` | same |
| 0x294050 Controls update | states 0→1 (`help_controls[lang]`), 2→3 (`[6 + lang]`), each after the stream is idle | `pages::controls_update` | `help_controls_page_two_pictures` |
| 0x294198 Controls draw | the two 256×256 pictures side by side, return 8 | `pages::controls_draw` | same |
| 0x2903d0 QS enter | copy 0x141ea0, cursor = first empty slot | `pages::slots_enter` | `quick_select_…` |
| 0x2901a8 QS update | R1 / L1 ±1 mod 8 (sound 1); ✕ on an owned focused item: move record 21 bump, the item out of its old slot, into the cursor's, cursor + 1 | `pages::slots_update` | `quick_select_…` |
| 0x290468 QS leave | **writes 0x141ea0** (the NTSC writer of the quick-select slots) | `pages::slots_leave` | `quick_select_…` |
| 0x294528 QS draw | ring of 8 at r = min(w, h)/2 − 40, the cursor's pulsing frame, empty squares 0x40404040, icons (gold variant 4), shoulder tabs | `pages::slots_draw` | `quick_select_…` (7 empties); frame `qs_a.png` |
| 0x292b60 ammo text | "ammo/max" (`%d,%03d` over 999) or "(no ammo)" 0x4f52 | `pages::ammo_draw` | `weapons_page_ammo_text_and_model` |
| 0x2919a0 ammo model | the focused item's ammo pickup class (item +0x3a) spawned at camera + (6, 0, −0.3), angle π, replaced in place on a change, freed for none; its update turns 0.01 rad / tick | `pages::ammo_model_update` (moby: `menu_models` role Ammo) | same |
| 0x291b18 ammo model draw | not owned: nothing; no class: "Uses no ammo" 0x4f4d | `pages::ammo_model_draw` | same |
| 0x292450 / 0x2924f8 gold bolt enter / leave | class 0x46e at camera + (8, 0.5, −0.1), pitch −1.9, spun 0.02 rad / tick; freed | `pages::gold_enter` / `gold_leave` / `gold_tick` | `items_page_gold_bolts_and_picture` |
| 0x292528 Items draw | "Gold Bolts", Found / Used / Remain (0x14bec0 count ≤ 40, 4 × gold weapons ≤ 10, the rest) | `pages::gold_draw`, `gold_counts` | same |
| 0x290b70 / 0x290bf8 Help Log list | items = log newest first, titles from 0x1798d0 | `pages::log_list_enter` | `help_log_page_lists_the_log_and_shows_the_message` |
| 0x290c00 / 0x290d40 / 0x290cd0 text | `all_text` streamed and swapped in (hidden until loaded), swapped back on leave | `pages::text_enter` / `text_update` / `text_leave` | same |
| label source 0x1000 | the focused log entry's help message | `PageMenu` label draw | same |
| 0x28d818 / 0x28d9a8 icon list | Up / Down without wrap, scroll, arrows | `pages::icons_update` / `icons_draw` | `help_weapons_icon_list` |
| 0x295000 / 0x2950c8 Help Weapons / Gadgets enter | owned items of the Weapons grid / Gadgets grids; text tables (+0x40 / +0x42 gold / +0x44) | `pages::help_weapons_enter` / `help_gadgets_enter` | `help_weapons_icon_list` |
| 0x2904a0 Moves label enter | Heli-Pack owned → 0x1b4c50, else 0x1b4c88 | `pages::moves_label_enter` | `help_moves_table_follows_the_heli_pack` |
| 0x2904e8 | stream buffer layout | n/a (memory only) | — |
| 0x295af8 Skill Points draw | the entry's location / planet names, or "All Levels" | `pages::skill_draw` | `goodies_skill_points_and_movies` |
| 0x295730 In-Level Movies enter | the list = 0x1b8aa8[level % 19] | `pages::movies_enter` | same |
| Goodies entries | Credits (11), Cinematics, Movies post-actions; Sketchbook / Epilogue / Making Of / Commercials locked (−1) | lists ported; post-actions G-UI-004; locked pages NOT ported (G-UI-002) | same (locked: no action, no sound) |
| Cheats page 0x1b7fb0 | the cheat list | NOT ported (G-SAV-006) | — |
| Save / Load pages | memory-card slots, previews | NOT ported (G-SAV-002) | — |

**Pictures.** The widgets name a global TOC field (`+0x30` − 0x137b80) and an index; the engine reads
`global/<field>/NNN.bin` through `disc_source`, WAD-decompresses and decodes the PIF (`rc_formats::pif`: "2FIP", PSMT8,
CT32 palette at +0x20, pixels at +0x420), and copies it into one of 4 slots of 512×512 below the HUD atlas (least
recently drawn replaced). No extractor change: the lumps were already extracted.

Frames (two runs each, identical): `scratchpad/helpmap/frames/{weapons,qs,items,help,help_log,controls,moves,help_weapons,goodies,skill}_a.png`.

## 13. The in-game map (ported 2026-09-28)

Code: `rc-game/src/map.rs` (files, masks, transforms, zones, the fog writer), `rc-game/src/map/predicates.rs` (the
per-level reveal predicates), `rc-game/src/menus/pause/map_page.rs` (the page: compose, pan / zoom, draw, globe,
legend, missions' status, markers), the engine's `gameplay.rs` (`map_setup`, `map_file`, `map_tick`),
`menu_render.rs` (the open / close hand-over, the Select / R3 rule, the map-used record, the palettes and hooks),
`scene_render.rs` (the save's pack). Tests: `rc-game/tests/ui/map_levels.rs` (6, disc data, all 19 levels),
`map::predicates::tests` (53 predicates against the game's own code), `menus::pause::map_page::tests` (3).
Frames: `scratchpad/helpmap/frames/{map01,map05,map13,mapfog,mapomatic}_{a,b}.png`.

**What is a system.** One engine system for every level (§10): the same functions and the same tables in all 19
overlays (`tables_on_every_level` compares the zone table, the default pans, the predicate slots, the mission and
marker lists, the marker sizes and the prices). The per-level parts are data (zones 0x183020, transforms 0x182c90,
lists 0x1870f0 / 0x187140) and small per-level functions (the reveal predicates 0x184410, the mission callbacks
0x262b40..0x262d78), ported as tables keyed by level. No per-level code in the port.

**Data.** The map files are the global TOC field 0x820 (38 entries, WAD): 0..18 plain, 19..37 Map-o-Matic
(`crate::map::MapFile`: zone tiles, run table, fogged / revealed pictures, three globe PIFs). No extractor change
(the lump was already extracted as `global/unknown_0820`). The picture's palette is the HUD grid icon's (0xe999, frame
= the level): the draw's TEX0 takes its CBP from that frame; the CLUT the compose uploads to 0x3ff0 (the picture's
first KB) is never sampled.

| address | what it does | ported / NOT ported / n/a |
|---|---|---|
| 0x25a4c0 level entry | default pans 0x184370, zoom 0.65 ×19; the world → map transforms from 0x182c90; the level's map file (Map-o-Matic set by owned[33], 0x1848a0); the zone tiles copied; the mask from chunk 3002 (first byte ≠ 0: `fun_00207bb0`) else from the tiles (`fun_00206710`); the tile cache (8 × 0x200); the brush 0x184814 | `MapState::enter` (the tiles decoded once instead of the 8-tile cache: n/a, memory only), `map::brush_open` |
| 0x25e800 `fun_00208408` | world → map (0..1) per level; level + 100 on level 6 with gp−0x6e08: the quarter-turned map | `map::world_to_map` |
| 0x25c4f8 fog writer | hero pixel + 1; zone rules per zone (z range, flags 1 / 2 riding, 4 / 8 group 0x10, 0x10 / 0x20 group 0xf, 0x40 magnetic floor, 0x80 zone flag 0x184928[arg], 0x100 << k predicate k); every fogged pixel of the 32 × 32 square in the brush and in an open zone cleared | `MapState::zones_open`, `MapState::reveal` (tests `fog_writer_reveals_the_brush`, `zone_rules`) |
| 0x228a1c / 0x228088 callers | the hero update (not in movement group 22 or state 50) and the other player modes' update, every tick | `gameplay::map_tick` |
| 0x184410 predicates | 53 functions on 14 levels (`side` 0x25ec10, riding, flags, zone flags, z ranges, a 2D distance 0x221398) | `map::predicates::call` (test: 20,000 inputs each against the level01 code) |
| 0x25c3e0 `fun_00206860` | a tile's run-length nibbles | `MapFile::tile` (test: every tile of the 38 files decodes to 1024 pixels) |
| 0x25c290 `fun_00206710` | the mask from the tiles (zone ≠ 0 → fogged) | `Mask::initial` |
| 0x25af58 compose | fogged picture where the mask is set, revealed elsewhere | `map::compose` |
| 0x25deb8 `fun_00207b08` pack | run coding over the run table (`FUN_00222328`), else the 4 × 4 downsample (`fun_00208030`); the length into 0x13d560[level] (a statistic) | `Mask::pack`, `MapState::pack` (the statistic kept as `packed_len`) (test `pack_and_unpack`) |
| 0x25df60 `fun_00207bb0` unpack | `fun_00207c28` (runs) / `fun_00207e58` (coarse) | `Mask::unpack` |
| 0x261448 / 0x29a5c8 savers | pack the current level's mask into chunk 3002 before writing | the engine's `EngineRequest::Save` (`scene_render`); the card write itself is G-SAV-002 |
| 0x2aba68 Select / R3 | ≥ 8 frames in mode 0, not state 0x72 / 0x32 / 0x1d, not group 22, HP > 0 → `EnterMenuMode(10)` | `menu_render` (group 22 added) |
| 0x2abfb0 | while 0x184694 (the map page found a map) is set, move record 9 bumped every mode-0 frame | `menu_render` (`help::bump`) |
| 0x28bf50 → 0x262760 | the destination's (the level's) mission status at every menu open | `map_page::enter` |
| 0x28c128 → 0x25a8d0 (kinds 0xb / 0xf) | map available = the level has one; a Map-o-Matic picked up since the entry switches the current level's picture set; shown = −2; `FUN_00262da0(dest, 1)` | `map_page::setup` |
| 0x2904d0 globe enter | shown = −1 | `map_page::globe_enter` |
| 0x28f868 widget update | pan / zoom `fun_00205440`; the keys (Start / Select / R3 close, △ back, ✕ missions page, ○ Infobot movie, L1 / R1 previous / next unlocked planet: sound 1 and `FUN_00262760`); on a destination change the compose (the live mask on the current level, the saved mask of a visited level, the tiles otherwise), the globe layers, `FUN_00262da0(dest, 0)` | `map_page::pan_zoom`, `planet_select::map_update`, `map_page::compose` (test `map_page_on_novalis`) |
| 0x28f868 streaming | the stream-buffer cache of map files (slots 0x1848e8, 0x184910), prefetching | n/a (the engine reads the file when composed; memory management) |
| 0x28f7a8 widget leave | stream break | n/a (no streams) |
| 0x25afc0 `fun_00205440` | not after Select / R3; zoom × (1 − 0.02·ry) in 0.65..4; pan += stick·3·10⁶ / zoom; clamped | `map_page::pan_zoom` |
| 0x292d38 → 0x25b1c0 draw | the grid tile 0xe999 (repeated), the picture, the markers, the hero arrow 0xe99a frame 5 on the current level (yaw, +π/2 in group 0xf or level 6's alternative, mirror cheat) | `map_page::draw` |
| 0x25b1c0 "No Map Available" | debug-font text when the level has no map | n/a (all 19 levels have one) |
| 0x25b1c0 tile-cache rects | debug (0x18469c) | n/a (debug flag, never set) |
| 0x25e950 `fun_00208508` | only with 0x18468c, never written | n/a |
| 0x294258 globe | stations 6 / 13 / 17: the three layers, two blinking; planets: the margin 0x1c24e8, the surface's u scroll | `map_page::globe_draw` (aspect crop [L]) |
| 0x292d70 legend | View Missions / Play Infobot (not level 0) / Previous / Next Map / Track/Zoom / Exit | `map_page::legend_draw` |
| 0x262760 mission status | per record: flag 4 waits for the visit; first condition → 1, both → 2; the callbacks' values; "none open" | `map_page::mission_status`, `condition` (`fun_0020baf0` 0x262900), `callback` (tests `conditions`, `callbacks`, `mission_status_rules`) |
| 0x262da0 markers | the destination's list when visited; centre the view on the hero (with 1); the points (fixed −1..−9, hook mobys 0x179638 on the current level, landmarks 0x13d5b0 elsewhere); shown by the mission's status; flag 0x1000 needs the landmark's flag 1; the label box sizing | `map_page::setup_markers` (the box sizing at the draw) |
| 0x25b1c0 markers | rectangles (icon size × 1.5 / zoom / (2z + 5) / 13), pairwise push-apart, backing (0x40), icon or turned sprite (0x100; sizes gp−0x6de8.., landmark flag 2 → next frame), label box (UI frame + small font) | `map_page::draw_markers` |
| 0x25e630 label text | `%b` → the price 0x1c4530, else "error" | `map_page::label_text` |
| missions page 0x1b3bf8 | the mission list 0x293090 (`fun_0020bc00`), the mission picture 0x293398 / 0x293670 (`mission_ss`), the selection marks | ported 2026-10-01 (`map_page` coverage table) |
| level 6's gp−0x6e08 | set by level 6's class code (0x302578) | the flag is `MapState::alt`; its writer NOT ported (G-UI-001) |
| 0x184928 zone flags | written by level classes (L07 / L13 movers …) | `MapState::zone_flags`; the writers NOT ported (G-UI-001) |

**[L] notes.** The globe's copy "aspect crop" (return 4) keeps the target's centre at the panel's aspect. The map's
coordinates are converted from 1/16 pixels to whole pixels when drawn. The file is read when composed (the game
streams and keeps a cache; the picture appears the same update here).
