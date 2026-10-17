# Gadgets: inventory, equipment, the Gadgets page, automatic items

The owned / equipped model of Ratchet's items, the starting state, the Gadgets page of the pause menu, the items
that equip themselves, and how Ratchet and Clank show what is worn. Addresses are **level01.elf** (gp = 0x166c00).
Confidence: **H** read from the decompiler C / disassembly of the function itself (or its data); **M** from the
function and its callers; **L** inferred. Companions: `hero_states.md` §0.1 (item ids), `menus.md` §3 (the page
machinery), `hero_gameplay.md` (weapons, `GiveItem`), `game_state.md` (the save chunks), `interaction.md` (the vendor).

## 1. Owned vs equipped [H]

**Item definitions** (0x4c bytes at 0x179f40 + 0x4c·id; `rc_game::inventory::ItemInfo`): `+0` name text, `+4` gold
name, `+8` **slot type**, `+0xc` attach word (Ratchet's attach matrix 0x13fe10 + 0x40·w), `+0x10` moby class,
`+0x14` second moby's class (the right boot), `+0x18` byte (0x1413fb), `+0x38` icon (60000 + id).

| slot | items (id: name, class) |
|---|---|
| 0 hand | 8 OmniWrench 71, 9 Suck Cannon 849, 10 Bomb Glove 192, 11 Devastator 157, 12 Swingshot 208, 13 Visibomb Gun 163, 14 Taunter 175, 15 Blaster 168, 16 Pyrocitor 176, 17 Mine Glove 190, 18 Walloper 180, 19 Tesla Claw 177, 20 Glove of Doom 229, 21 Morph-o-ray 185, 22 Hydrodisplacer 1251, 23 R.Y.N.O. 454, 24 Drone Device 483, 25 Decoy Glove 562, 26 Trespasser 188, 27 Metal Detector 585, 31 Hologuise 483, 32 Gadgetron PDA 619 |
| 1 feet | 28 Magneboots 173 (+ 173), 29 Grindboots 195 (+ 195), 30 Hoverboard 439 (not on the Gadgets page) |
| 2 head | 5 Sonic Summoner 433, 6 O2 Mask 1289, 7 Pilot's Helmet 1290 |
| 3 back | 2 Heli-Pack 607, 3 Thruster-Pack 608, 4 Hydro-Pack 609 (item 1 = Clank 601, always on the back) |
| 4 / 5 | 33 Map-o-matic 614, 34 Bolt Grabber 618 (slot 4), 35 Persuader 407 (slot 5): created by `HeroItemsCreate` whenever owned, never equipped |

(The table's names are the definitions' own text ids, so item 11 is the Devastator; 24 and 31 share class 483.)

**Fields, per slot s** (0 hand, 1 feet, 2 head, 3 back):

| field | address | kept in | port |
|---|---|---|---|
| owned | 0x13d4c0 + id | save chunk 10 | `GameState::global.owned` (mirrored into `Hero::owned` each tick) |
| ever acquired | 0x13d4e8 + id | chunk 11 | `global.acquired` |
| saved (equipped) item | 0x141660 + 4s | chunk 32 `equipped[7]` | `global.equipped[s]` |
| request | 0x141408 + 4s (0x26 = take off) | hero block (cleared at every level start) | `SessionState::temp_hand / temp_feet / temp_head / temp_back` |
| target, restore, restore request | 0x141424 / 0x141440 / 0x14145c + 4s | hero block | `ItemSlot::{target, restore, restore_pending}` |
| slot record: moby(s), +0x24 state (2 ready, 3 put away, 0 empty), +0x28 item | 0x1403e0 + 0x50s | hero block | `Hero::items.slot` (hand), `feet_slot`, `head_slot`, `back_slot.slot` |
| hand shows the wrench / last hand item | 0x15ed90 / 0x15ed8c | chunks 22 / 21 | `global.wrench_held / last_hand_item` |
| the Thruster was the last back item | 0x15ed94 | chunk 23 | `global.thruster_last` |
| Clank hidden | 0x141628 | session | `SessionState::clank_hidden` |

**`GetClankModule(s)`** 0x22ddd8: the slot's item while its state is 2, else −1 (the **ready item**; `Hero::held_item`,
`feet_module`, `head_module`, `back_module`).

**Creation** `HeroItemsCreate` 0x22f3c0 (every hero update, for an empty slot): hand = target, else 8 when 0x15ed90,
else the saved item, else 8; **feet** = the saved item 0x141664 when non-zero (the target is not read), two mobys;
**head** = the target 0x14142c, else the saved 0x141668, when either is set; **back** = the target 0x141430, else the
saved 0x14166c, else **2** (3 when 0x15ed94), and Clank once; slots 4 / 5 when items 33 / 34 / 35 are owned.

**Swaps** `UpdateWrenchSelected(s)` 0x2307e0 (the slot loop 0x231088, every tick, also for an empty slot): the
slot's own rules (§4), then the restore request (target = saved = the restore item, 0x26 → 0) and the request
(0x26: target = saved = 0; else target = request, saved = request unless it is the O2 Mask / Hydro-Pack put on by
the water rules); a change puts the item away: state 3, the moby blends to its sequence 2, `FUN_0022b8e8` (look
cleared), the fidget timer `rand_range(50, 90)`, a raised weapon put away (`0x22efd8`); slot 3 also sets 0x15ed94
from the item being put away. Deletion `0x2305e8`: the feet mobys at once, the others when their put-away wraps; the
next creation makes the target.

**The swap sequence and its timings** (re-read 2026-09-28 against the disassembly of 0x2307e0, the slot loop 0x231088,
the deletion 0x2305e8 and `HeroItemsCreate` 0x22f3c0) [H]. One path serves every change of a slot — the page close's
request, the water rules, the `SetState` restores (which reach the back only through the restore request 0x141468 set
by the back rules themselves: SetState writes the hand's 0x14145c and the feet's 0x141460, never the back's) and
`GiveItem`'s equip:
1. **Start** (the tick the request is taken; the slot's +0x18 timer 0x1403f8 + 0x50s must not be running — only the
   hand's is ever set, by the physics (2) and SetState (10)): +0x24 = 3, `FUN_0022b8e8`, the fidget timer
   `rand_range(50, 90)`, `0x230720` / `0x2306c0` (hand weapons only: the lock-ons 0x13f52a / 0x13f52c), a raised
   weapon put away, the slot's item blended to its **sequence 2 over 2 ticks** (not Clank on the back), 0x15ed94 for
   the back.
2. **Put away** (state 3, the slot loop): the hand (0) and the feet (1) are deleted on the next loop at once; the head
   (2) and the back (3) when the item's sequence 2 wraps (its anim flag 2). `0x2305e8` runs the item's own update one
   last time, empties the slot (+0x24 = 0) and deletes the moby (Clank stays).
3. **Empty for one tick**, then the next hero update's `HeroItemsCreate` makes the target's moby (state 2) on its
   **sequence 0** (the take-out: `InitMobyInstance`'s state); the hand waits for its +0x14 frame (`0x1403f4 <
   0x15f3f8`, two frames after the deletion), the others do not.
4. **Ready** (state 2): a wrapped sequence 0 blends to **sequence 1 over 2 ticks** (0x231088); the back table rows of
   Ratchet's anim calls take over from there.

The back packs' data make the Heli-Pack (607) and Thruster-Pack (608) changes quick: their sequences 0 and 2 are
**single frames** (the put-away is the 2-tick blend, the take-out one advance), while the Hydro-Pack (609) has a
7-key take-out (sequence 0, 2 ticks a key) and a 3-key put-away (sequence 2): into the lake the Heli-Pack is gone after
2 ticks, the slot is empty 1 tick, and the Hydro-Pack unfolds; out of it the Hydro-Pack folds for 6 ticks, 1 empty
tick, the Heli-Pack. The feet slot deletes at once (no put-away animation); the head slot uses the same put-away as the
back. The port runs exactly this (`idle::back_slot_states` / `back_swap`, `worn`), so no animation was missing: the
change *is* that short in the game (`tests/hero/hero_pack_swap_novalis.rs` pins the sequence and the tick counts).

**`GiveItem(id, equip)`** 0x275760 (`GameState::give_item`): acquired, owned, the item's ammo, the vendor stock, a
free quick-select slot for a hand item; with `equip` the request of hand (type 0), head (2) or back (3) — never feet.
Unchanged by this work (the vendor buys through it).

## 2. Starting state [H]

New game (the disc's save template, `global/save_game.bin`): nothing owned, `equipped` all 0, 0x15ed90 = 1 (the wrench
in hand), 0x15ed94 = 0, quick select empty. Veldin's level start (`FUN_00251da0`) gives the Bomb Glove (owned, equipped
hand 10, vendor slot 0, quick select [10]); Veldin's Clank sets 0x141628 for that session. **On arriving at Novalis**
(transition + level start, the hero init clearing the session): owned = {10 Bomb Glove}, `equipped` = [10, 0, 0, 0],
wrench held, no requests, Clank shown. `HeroItemsCreate` then puts the wrench in the hand, nothing on feet or head,
and on the back **item 2's model, class 607** (0x14166c = 0, 0x15ed94 = 0) with Clank 601: the game's own look of
Clank at this point — the Heli-Pack model on its folded idle sequence 1 (the rotors only come out in its sequence 6,
`ClankPackUpdate` 0x2f30c0 in the glide). No pack move is possible: the glide 8 and the Heli-Pack jumps test the
owned byte 0x13d4c2 (`0x242930` / `HeroCrouchJumps` 0x242420), the Thruster moves the back item 3.

## 3. The Gadgets page [H unless noted]

Page 0x1b2d68 (kind 4, parent the Start root, focus **the back packs grid**), seqs [0x1a, 0x1b, 0x22, 0x23, 0x1e,
0x1f, 0x14, 0x24, 0x21, 0x25..0x29]. Widgets: W0 title label 20195 "Gadgets" (flags 0xf); W1 the 3D Ratchet; W2..W3
the hand (2×3) and back (1×3) grids; W4 the item name label (flags 3, table 0x179f40 stride 0x4c); W5 the item preview;
W6 filler; W7 "Foot Items" 20203; W8 the hints list 0x1b3628 ("✕ Equip" 20204 / "△ Exit" 20192); W9 / W10 the head
(1×3) and feet (1×2) grids; W11..W13 "Head Items" 20202, "Back Packs" 20201, "Hand Items" 20200 (flags 0x12015).

* **Grids** (update 0x28f260, draw 0x291350, enter 0x28f1b0): `+0x30` flags (1 no ✕, 2 fixed row step, 4 packs, 8 head,
  0x20 no equipped icon, 0x8000 no wrap), `+0x34/+0x38` margins (0.1), `+0x3c` cursor, `+0x40` rows, `+0x44` columns,
  `+0x48` cells (10 bytes: icon, variant base, kind, item), `+0x4c/+0x50/+0x54/+0x58` neighbours up / down / left /
  right. Cells: hand {26 Trespasser, 22 Hydrodisplacer, 12 Swingshot, 32 PDA, 27 Metal Detector, 31 Hologuise}, back
  {2, 3, 4}, head {6 O2 Mask, 5 Sonic Summoner, 7 Pilot's Helmet}, feet {29 Grindboots, 28 Magneboots}. The chain
  wraps hand ↔ back ↔ head ↔ feet ↔ hand vertically; Left / Right wrap within a row.
* **Keys** (focused grid): generic Start / Select / R3 close, △ parent; Up / Down / Left / Right move the cursor, at
  an edge the focus goes to the neighbour grid (skipping unusable ones; the column carried, 3 ↔ 5 column grids shift
  by one), sound 1 on a move; ✕: an empty or unowned cell → sound 2; else sound 0 and the equip rule on the page's
  copy of the saved items `0x1ba1a0[4]` (copied at the menu's first tick): **hand and back items replace** (the
  equipped one selected again does nothing: there is no empty hand or back), **feet and head items toggle** (the
  equipped one selected again comes off); one item per slot; the Drone Device (0x18) has its own path (not ported).
* **Close** (`PageMenuClose` 0x28c6c8): for each slot whose copy differs from `0x141660[s]`, request `0x141408 + 4s` =
  the copy (0 → 0x26). The swaps then happen in play (§1), so the pack changes with its put-away animation.
* **Draw** (panel-local 1/16 px): the cell size 0.45 (gp−0x68b0/−0x68ac) and margins are in the frame moby's 3D units
  (its pvar +0x40/+0x44), scaled by max(w, h)·16 / max(size3d); owned items only, icon frame `GetIconFrame(icon,
  variant + v)` with v = 1 equipped (not for the hand grid), 4 gold, 2 unusable; the cursor: a 3-px frame pulsing
  `((|(t & 0x3f) − 0x20| + 0x40)·0x10202)` around a 1-px navy one. Unusable: head items on level 0xd (or 0x14161b),
  packs on levels 0 and 0xe (0x1ba2a4 / 0x1ba2a8); the grid enter moves the focus off an unusable grid.
* **Name label** (0x28dd30 default source): the focused grid's cursor item, variant = its gold byte → text `+0` / `+4`
  of its definition; empty when not owned.
* **Item preview** (0x291938 / 0x291c38 / 0x292010, draw flags 2): the cursor item's class (0x1df for the Drone),
  at camera + table 0x1c4988 (0x20 bytes: x (flag 1) / x, y, z, rot x, rot y, pivot a, b), turned about z from π by
  0.01 rad a tick (`FUN_002920a0` turns the pivot), cut to sequence 1 (6 for the Heli-Pack) and animated; Clank with
  a back item; drawn only when owned (return 8, centre crop).
* **3D Ratchet** (0x297ad0 / 0x297d70 `LoadHandGadget` / 0x291800 / 0x297cc0, draw flags 2): Ratchet at camera +
  (4, 0, −0.6), yaw π, with Clank; the pending hand item, head item (Ratchet's feet shrunk by manipulators on lists
  0x16 / 0x17 for the boots), boots, pack (607 on its rotor sequence 6), and when owned the Persuader 0x197, the
  Map-o-matic 0x266, the Bolt Grabber 0x26a, the drones 0x1df; per-item animations streamed (`fun_002265d8`, table
  0x1b9870) [M: the streaming is not traced]. Return 4 (aspect crop).
* The page's panels come from the 14 frame mobys (class 0x472) like every page (menus.md §3).

## 4. Automatic items [H]

| item | when it goes on | when it comes off | code |
|---|---|---|---|
| Grindboots 29 | grinding (group 0xf) with them owned, the saved feet item not them (or no feet moby): request 29, saved; the current feet item (or 0x26) kept to restore | the `SetState` tail out of group 0xf while they are on and a restore item exists: restore | 0x2307e0(1), 0x23cf98 |
| Magneboots 28 | on a magnetic floor (0x140637, the surface reaction's surface 2 near the ground) with them owned: likewise | off the magnetic gravity (0x13f658 = 0) while on: restore | same |
| any feet item | — | in the water (`FUN_0022dea8`) or state 0x12: kept to restore, request 0x26 (saved cleared); back on out of the water | same |
| O2 Mask 6 | under water (group 0x11, states 0x76 / 0x6a / 0x82, or 0x14161b) with it owned: request 6 **not saved**, the head item (or 0x26) kept to restore | out of those: the restore request 0x141464 | 0x2307e0(2) |
| Hydro-Pack 4 | in the water groups with it owned: request 4 not saved, the back item kept | out of the water (not 0x12): restore | 0x2307e0(3) |
| Heli-Pack 2 | in the glide 8 with the Hydro-Pack saved: request 2 | — | 0x2307e0(3) |

The magnetic gravity needs the feet moby to be the Magneboots' class 0xad (`HeroSurfaceReaction` 0x22cd48), so it
engages once the automatic swap has made them (a few ticks after stepping on); the rail contact tests the owned byte
0x13d4dd; the Magneboots' metal footsteps (`HeroFootstepSound` 0x227e48) test the feet moby. The air drain tests
the O2 Mask's owned byte (swim code). The Hologuise and Sonic Summoner are not automatic (hand / head items equipped
by the player; the Hologuise's hand swap 0x1f is the hand slot's).

## 5. What Ratchet and Clank show [H]

* **Back**: Clank 601 always (hidden with 0x141628) plus the pack moby of the back slot's item on the back list
  (attach word 5): 607 Heli (folded; rotors in sequence 6 in the glide), 608 Thruster, 609 Hydro; the back table
  0x2476d0 animates pack and Clank with Ratchet. The automatic Hydro swap in the water shows the Hydro-Pack.
* **Feet** (while slot 1 has its mobys): left boot on attach word 2, right on 3, posed from Ratchet's joints
  (`HeroItemPoseFromRatchet` 0x22a9c8: joint 0 identity, joints 1.. = Ratchet's local pose of joints 100..103 / 94..97);
  Ratchet's feet records 4 / 5 (lists 22 / 23) get scale 0.01 every tick (`0x22c5c0`), so the boots replace his feet.
* **Head** (slot 2): attach word 4, posed from Ratchet's joints 8, 10 (the Sonic Summoner 0x1b1: 8, 9, 10, 16, 17, 25,
  26, 29, 30, 33, 34, 35, 36, 41, 44 + 6 identity joints); its own put-away animation while state 3.
* Everything hides in first person (`0x2486c0`).

## 6. In the port (2026-09-28)

* `crates/rc-game/src/inventory.rs`: item ids, [`Slot`], `ItemInfo(s)` (definitions), the views of the saved state
  (`owns`, `equipped`, `request`, `set_request`, `owned_in`), the page rules `menu_select` / `menu_close_requests` /
  `apply_close_requests`, and `debug_grant`. `SessionState::temp_feet` (0x14140c) added.
* `crates/rc-game/src/hero/worn.rs`: slots 1 and 2 (`Hero::feet_slot`, `head_slot`, `worn`): creation, slot loop,
  rules, restores (`worn_on_set_state` from `SetState`), `feet_module` / `head_module` / `magneboots_on`, and the pose
  builder `pose_from_host` (+ the joint tables). `idle.rs`: the shared swap effects (`slot_swap_effects`, now also
  putting a raised weapon away for the back), `in_water_groups`, the feet records 4 / 5 and their 0.01 scale.
  `boots.rs`: the magnetic gravity 0x13f658 needs `magneboots_on` (was: owned); `fx.rs`: the footsteps' variant too.
* `crates/rc-game/src/menus/pause/gadgets.rs`: grids, name label source, item preview, 3D Ratchet view
  (`PageMenu::{equip, items, unusable_head, unusable_back, view}`, `MenuOut::equip`); the Weapons page gets the same
  grids / label / model (its other widgets remain stubs).
* Engine: `gameplay.rs` syncs the feet / head slots with `equipped[1..3]` / `temp_feet` / `temp_head` like the back;
  `moby_attach.rs` draws the boots and head items (`Slot::Feet` / `Slot::Head`, shown only while the slot holds their
  class — the same "absent stays hidden" rule as the hand and back); it also draws the **always-on extras** (items 33
  Map-o-Matic, 34 Bolt Grabber, 35 Persuader: `Slot::Extra`, created while their owned flag is set, at Ratchet's
  attach lists 29 / 30 — §1); `menu_render.rs` gives the page the item definitions and applies the close's requests to
  the session; `menu_models.rs` fills the two 3D widgets (which mobys,
  their poses, records and palettes) and draws each on a `crate::screen_canvas` canvas composed **over** the HUD
  (`Canvases::create_over_hud`; view = the panel's rectangle, the game projection's tangents given exactly with
  `Canvases::show_exact`, the view axis on the panel's centre, the navy clear; `hud_render::HudCompositeNode` made
  `pub(crate)` so the canvas node can sit under it as its child).
* **Debug grants** (port-only, env): `RC_GIVE_ITEMS=id,id,…` owns the items (with `GiveItem`'s ammo and quick-select
  slot) and, unless `RC_GIVE_ITEMS_EQUIP=0`, saves the **last back, feet and head item** among them as equipped and
  requests the last hand item — only when the variable is set; without it the state is the game's. E.g.
  `RC_GIVE_ITEMS=2,3` (both packs, Thruster on), `RC_GIVE_ITEMS=2,3 RC_GIVE_ITEMS_EQUIP=0` (both owned, the start's
  back item 2 kept: equip in the Gadgets page), `RC_GIVE_ITEMS=29,6` (Grindboots and O2 Mask worn).

**Why Clank looked like a Thruster** (the report of 2026-09-28): not the starting state (§2 is the game's), nor the
item → class mapping. The engine kept an entity set for every pack and gadget class and two passes (the vendor's
exit, a scene's end) re-showed every attached entity, so unworn packs appeared on Clank after the arrival scene;
fixed by the hand-item work (`moby_attach::keep_absent_hidden`), which also covers the new feet / head items.

**One render-to-texture mechanism**: `crate::screen_canvas` (docs/plan/interaction.md) serves the vendor's screens and
these widgets; each canvas says where it is composed relative to the HUD (the vendor's monitors under it, a page
widget over it: `PageMenuDraw` 0x28d080 copies a widget's render target after its panel's navy rect, and the port's
panels are HUD primitives). The first version of the widgets had its own cameras and images (`gadget_preview.rs`);
it was folded into `screen_canvas` on 2026-09-28 with pixel-identical results (menu, Clank and vendor captures).

**Digest**: `novalis_hero_digest` with `RC_HERO_DIGEST_NO_IDLE=1` is byte-identical; the full digest differs on every
line only because the idle block prints two more joint records (4 / 5, the feet; inactive without boots).

**Native, not emulated**: all of it is plain Rust state and `f32`; the pose builder builds an ordinary keyframe
(`MobyFrame`) evaluated by the existing evaluator; the 3D widgets are ordinary Bevy cameras and images, no GS
render-target emulation.

**Inferred / not ported**: the 3D Ratchet's per-item streamed animations (he idles on sequence 0) [L]; the drone mobys
on him and their callbacks; where exactly the widget's render target puts the
camera axis (assumed: the panel's centre, matching the reference screenshot) [L]; the Drone Device's ✕ path; the
foot IK of `0x22c5c0` (only its feet scale); 0x14161b (0); `FUN_00248ad8`'s gravity-mode test and the ground magnet
branch still read the owned byte [L]; the hint list's ✕ Equip / △ Exit are the disc's list, drawn as any list.

**Tests**: `inventory::tests` (the page rules per slot, the close requests), `hero::worn::tests` (Grindboots with the
grind group, Magneboots and water, the O2 Mask under water, menu requests), `tests/ui/gadgets_novalis.rs` (the first
arrival's state with no grants and no pack moves; the page on the disc's records: open, back grid focus, icons,
name label, preview view, equip Thruster / denied Hydro, close request, the swap in play, the stomp and the
Thruster glide, then the Heli-Pack back and its glide; feet toggle and head equip requests), the existing boots
tests (the magnetic floor now through the automatic Magneboots). `tests/hero/hero_pack_swap_novalis.rs` (2026-09-28): the back swap's
sequence and tick counts into and out of the Novalis lake (Heli-Pack → Hydro-Pack → Heli-Pack), and the Thruster long
jump's after-images.
