# Forum triage (bugs-and-qa)

## 2026-11-06: R&C1 pass against Lombyte / OpenRAC (open posts, runtime 0.3.7 / 0.3.8)

Fixed on main (retest after the next release): the Oltanis crash after dying (255705f: stale draw callbacks across
the death reload); the gold Bomb Glove's first-person range (a2a7721); the FOV after crashing the Pokitaru jet while
boosting (7d63c30: the on-foot hero frame resets it); Batalia's turret, re-enterable after the battle and △ leaves
it (f5e133a: three mission tests were inverted against UpdateMoby_440); the mirror cheat's black flashes (5876aa5);
Gaspar's turret sounds every tick (cfda52d: the death reload now frees the point lights). PR #50 merged.

Looked at, the code matches Lombyte but the PS2 differs or needs data: the Hologuise collision
on ice (not reproduced headless: entered standing and sliding, left with □, walked back through the spot); the
Aridia counters after death (#62: the race watch is Lombyte's line for line); the Hoven crate z-fight (placements 1
apart, stacking at run time); the wrench reticle's last bit of opacity (the GS blends in gamma space, the port in
linear light). No usable data: the Veldin Orbit crashes on Linux (the logs end without a panic), #63 (every buffer
invalid at once: a lost device), the Novalis slab panic. Settled by a PINE trace on Hoven (work/trace/glove_ice*.tsv): the glove throw on ice is the original's. A direction
held (stick or d-pad: magnitude 1) throws with the arm and the slide goes on; nothing held → state 0x23, which brakes
from the stored speed 0x13f4e4 at 35·dt² a tick (0 when the slide began from standing: an instant stop). The port
does the same. Not changed: the original's death reload also calls
srand(1234) (the port's RNG continues; invisible).

## 2026-10-23: every open post (92, read in full)

Players: launcher 0.3.0 / 0.3.1, R&C1 runtime 0.3.6, R&C2 runtime 0.1.1. "main" = fixed after 0.3.6, ships with 0.3.7.
Closed today: Kalebo III hovertrack water (idunnomate confirmed with a screenshot), the reverb after the Pokitaru
teleport (HMDSOLO: the PS2 does it too).

**Second pass, same day (all on main, retest after 0.3.7):**
- Board races: the turbo meter shows, no ammo counter during the race, the counter follows switches after it (ff27e07,
  7927cd4: HMDSOLO 1561231247613763677, Pamo13 on Kalebo).
- The Epilogue's pictures on every planet (0ac02a1: idunnomate 1562597074946621551).
- Level textures at the PS2's scale: negative tfrag UVs count half (c4a5008: HMDSOLO's Novalis cave 1562272075203354674;
  likely the shared cause of the other broken-texture posts: Metropolis 1562299233376272434, Rilgar / Blarg
  1560490367449829477, Batalia 1561607428439547965, Pokitaru sewers 1561292714690224178, Orxon 1561261776539226182).
- In-Level Movies hide the level's NPC on every planet (76889fc: idunnomate 1562199509759565886).
- The Visibomb view's scanlines (b4e0373: HMDSOLO 1562010718641987654; the "missile visible a moment" part needs a
  PS2 clip: the launch and the camera cut are immediate in the decomp).
- The teleporters' arrival sound (3d28008: HMDSOLO 1562001348088561664).

**Third pass, same day (all on main, retest after 0.3.7):**
- Novalis' water flowers move aside for Ratchet (c0d308c: HMDSOLO 1562265530981679114; the float's step limit is from
  the tick's start, the port's was from its placed point).
- Class texture scrolls drawn on every moby (0b0ad1f: Gemlik's lasers / fields crawl, HMDSOLO 1562356617012580372; also
  Qwark's shield; the Pyrocitor's flame now through the same system).
- Oltanis' lightning shows (df1947a: HMDSOLO 1562745815846363156 and idunnomate's intro 1562165341138845859: the alpha
  cut ignored the strike timer). HMDSOLO asks for the lightning to return after leaving the planet as a ReRAC extra
  (the original stops it for good): the user's call.
- Oltanis' arrival strike on Clank (idunnomate's intro 1562165341138845859, PS2 frames from the user): level 14's scene
  hook `0x2460f8` draws a three-segment bolt (`0x2459c0`) off the actor's joint and sparks (`0x245e98`) for six ticks
  from scene tick 0x1c; ported as `cutscene_fx` + `Callback::SceneStrike`.
- Gemlik's towers and the Blaster (1562123434522189886; the helpdesk says Devastator or Visibomb): on main the Blaster
  no longer takes them from the base (82 hits headless, none above the tower's +8.7 test); retest after 0.3.7.

**Fourth pass, same day (all on main, retest after 0.3.7):**
- Oltanis' course lift by the ship runs back to the course's start once the PDA is bought (2195e4d: Dion 07
  1562847989691842620; the cart's way on to Steve needs its death bits clear, the port wanted them set).
- Bombs sink in moving water too (e9dfd0e: idunnomate 1562125255873532014; the game's water test is the hit's surface
  type alone, and Umbris's tanks, Pokitaru, Rilgar, Hoven and Veldin 2 have moby water).
- The Bomb Glove's auto-aim cone starts from the glove hand's yaw, 20.7° right of the facing (5f640ac: HMDSOLO
  1562298315612364810, crates picked on the left).
- Bombs thrown on a moving platform take Ratchet's motion (9d37410: HMDSOLO 1562302440961744917, Metropolis's train).
- The PDA shop shows the player's own view behind it with the vendor stand below (1de17e8: HMDSOLO
  1562747362307211324; the port drew the world along +x everywhere).

**Fifth pass, same day:**
- Decoys and mines ride the platforms they stand on (07209dc: HMDSOLO 1561271052456296448, Pokitaru's boats; the
  decoy follows the moby its ground line hits, the mine records its carrier +0x4c). Not checked in game.
- The map's ship star on other planets (1560410178598211727): the port captures the landmarks as the game does
  (checked headless: leaving Kerwan records its ship slot at the pad) and every level's ship maps onto its picture;
  saves from 0.3.4 and older never recorded the ship slot, so each planet shows it right once it has been left
  on 0.3.5 or later. Answer the post with that.
- The Decoy held in the glove (1561272266459709460, 0.3.4 screenshots): the shared glove hand offset be6e72f shipped
  in 0.3.6; ask for a retest.

**Sixth pass, same day:**
- Quartu: Clank's "mother" types her words on the monitor (0bf67a0: §1.35 and the 0.3.6 report; the broadcast
  director's draw callback `0x2eb6b8`; G-UI-025 closed).
- Launcher (re-rac/launcher main 50abc36, unreleased): the Linux AppImage hands the file manager, the game and the
  extractor the system's environment instead of its bundled library paths (Gengear, Open Data Folder on SteamOS).
  The SteamOS volume crash with the controller (1561896146421686282) may share the cause (the game loaded the
  AppImage's libraries); ask for a retest with the next launcher.
- Already fixed, the doc was stale: Giant Clank red during his △ attack (§1.20, a54637f on 10-20: the tint only
  reddens the world; checked headless with a forced tint).
- Not changed: the launcher run elevated under another admin account (Tactical 1562570286564376576): WebView2 cannot
  write its data folder in that setup; running it normally works (as the post says).

**Needs more (fifth pass):** the map's dark bar at the top (1560493051481165875): reproduced headless; it is the
picture's top 16 rows (a dark, partly transparent edge) once the pan reaches the clamp, which matches `fun_00205440`;
whether the PS2 shows it depends on the colour table the picture is drawn with: needs a PS2 shot panned to the top.
Sounds into the vendor (1561815960825102416): the group pause 0x1d is ported; the clip's loud stretch may be the
screens' static: needs a listen. Gemlik's tower as a Heli-Pack source (1561918614456504340): traced headless, the
hum's slot is freed when out of range and reused, as in the game; the port does not move sounds by the tower's offset
yet, so the reported symptom needs the exact steps. Hoven's low clouds moving with the camera (1562008926613151774):
the sky shells follow the camera in the game too; needs a PS2 clip.

**Looked at in the fourth pass:** the "oof" at a wall (1562191825899360416): the rebound states 0x21 / 0x7a play no
voice in SetState, the transitions or the physics, and start their animation at key 5, past its triggers: the port
does the same; needs a PS2 clip of when the oof plays. The barriers' "bit" (1562755433498673152): each barrier row has
a class-0 sphere emitter (defs 29..31, 0/0 retrigger), the port's matches `0x3197a0`; the clip's audio is too busy to
find the click: needs the exact moment or a clean recording. The Swingshot at Outpost X11 (1562477069412081776): the
searches and the score match `0x221ee0` / `0x22dff0`; the clip shows the hook holding for 4 s with no swing, so the
swing state's start; needs a repro at the spot.

**Open from the third pass:** the Hologuise's menu animation (1562218129155690506: reproduced: the worn props stay on
the sentry; `fun_002250f0` has no visible test, needs a PS2 clip); crouching under Hoven's arcs (1562749075747512370:
Ratchet's collision is two fixed cylinders the game never changes; the PS2 clip also loses health: needs a closer
comparison); the camera not following Qwark's falling ship (1562362129145790494: Gemlik's scene 2, the scene camera's
tracking).

**Open from the second pass:** the Walloper tilts out of the hand (1562347931296858242, 1562969762458243133: the
attach matches the decomp; needs `work/scratch/hand_item_probe.py` on a PCSX2 savestate with the Walloper, and one with
the Glove of Doom for its glow); the waterfalls flicker where they bend over the edge (Batalia 1560473204643074058,
Pokitaru 1561285436104642710: two surfaces at one depth; needs the spot from the user, F9); the Devastator on ice
swings out and up while firing (1562752763774107738: the weapon arm layer against the ice slide's arm pitch; needs a
traced repro on Hoven); the map's black bar at the top and its shape while zooming (1560493051481165875); the Suck
Cannon post (1562970122572791888) has a title only (ask what breaks); the Visibomb's missile seen for a moment
before the green view (1562010718641987654: the decomp launches and cuts on the press; needs a PS2 clip).

**Fixed on main (retest after 0.3.7):** the hollow / outline-only crosshairs (de9ae8b): everfeel 1562187260252921947,
ZFPInTheAir 1562540111143501914. Launcher "Could not move the build into … os error 5" (Windows): retried rename, then
copy (launcher main, unreleased): Glitch master 1561546349793648701, Kristian 1561920636593643580, Michan
1558748783205027870.

**Fixed on main today:** the Pyrocitor's flame and glow hide during a gold bolt's pickup (f52d5a7: HMDSOLO
1562365950169452595); the Hologuise's lights turn red with the guards' alert (6b0b3ea: idunnomate 1562222540590416092,
Spoober's list). **Same as the PS2 (the user checked):** the wrench coming out on every ledge climbed (HMDSOLO
1562756703521480744; the ledge states set 0x1413f7 in the game too). **Looked at, needs the PS2's numbers:** the Glove
of Doom glow (the port's formula is 0x2297b0's: the glove's +0xc0 rows and +0x10; the PCSX2 probe next); Clank's
music on Orxon (the second of Ratchet's track is the transition's timing, which the PS2 hides behind the disc
stream); Clank's ghost footsteps on Blarg (the lift hides Clank and stops his updates as the game does; needs a sound
trace). Attachments of every open post: `work/forum/files/` (2026-10-23).

**Partly fixed (our fix did not cover everything):**

| post | what is left |
|---|---|
| 1561982401914339379 Glove of Doom eggs | egg fixed; its glow sprite slips out of the glove while running (HMDSOLO, PS2 clip) |
| 1561235689503203388 Clank's music on Orxon | Clank's track plays, but Ratchet's plays for ~1 s first (HMDSOLO: not normal) |
| 1560528077749227520 wrench reticle | see-through, but lacks the PS2's glass look (de9ae8b changes the alpha > 1 blend: retest) |
| 1561090803533619351 FOV after the Pokitaru dogfight | crashing the jet while boosting keeps the FOV at respawn |
| 1560410178598211727 map star | wrong / missing on planets other than the current one; Umbris does not fill yellow |
| 1560493051481165875 map | the map changes shape when zooming, a black bar at its top |
| 1562199509759565886 In-Level Movies | the level's NPC still shows next to the scene's when it is present in the level |
| 1561231247613763677 Zoomerator turbo bar | entering a race with a weapon: no turbo bar, the weapon HUD stays (also Kalebo) |
| 1561815960825102416 vendor | sounds keep playing in the vendor (the crosshair and the hydrant are as on the PS2) |
| 1562010718641987654 Visibomb | PS2 clip given: the gun "spits out" the missile (visible a moment) before the green view |
| 1562001348088561664 teleporters | PS2 clips given: the teleport sound is incomplete |

**New since 2026-10-20 (R&C1 0.3.6):** Suck Cannon rapid fire / sounds; Ratchet's gloves poke through the Walloper
(2 posts); Clank's mother's monitor text missing (Quartu); Oltanis Magneboots lift goes to the end of the track on a
revisit; Ratchet takes out the wrench on every step he climbs; crackle from Oltanis's electric barriers and Hoven's
helicopters; the Devastator's firing animation on ice; the PDA's background; crouching under the red electric robots;
Oltanis's lightning (2 posts); the epilogue images; the Swingshot's reach on Aridia; vendor static on Veldin 2's
platforms; the Pyrocitor's flame floats when picking up a gold bolt; Gemlik's camera does not follow Qwark's ship; Gemlik's
lasers' motion; helmets (size, animation glitches); the Walloper / Thruster-Pack effects; the weapon sound at landing;
space too close in the flight; Clank's footsteps keep playing; the Bomb Glove on Metropolis's train and its auto-aim
on crates; Novalis's water flowers; the aerial turret HUD blur; the Hologuise alert lights and menu; the "oof" when
bumping a wall; the many broken textures (Metropolis, Novalis, Rilgar / Blarg, Orxon, Batalia, Pokitaru: one shared
cause); the waterfalls (Batalia, Pokitaru); the skies (Orxon, Gaspar).

**R&C2 (rerac2 repo):** Conker's crash log 1563200701394321478; beasty23jk 1563186473824423976 (weapon wheel, only the
Lancer, no ship on Oozla, vendor); Osvaldo 1563145419322818641 (video + list: extractor (launcher 0.3.1 fixed), scene
bars, wrench throw, backflip, enemies, floating crates, pause menu, lighting, aim, thief room, weapon in scenes, crash
at the level end).

**Launcher / platform:** MS Edge (WebView2) data folder as another admin (Tactical); Open Data Folder on SteamOS
(Gengear); Stuck on Running (no reply); controllers (DS4 Windows, PS5 over Bluetooth on macOS, Steam); Reshade; a
black screen fixed by WGPU_BACKEND=dx12 (Marco); the SteamOS volume crash.

Status: **2026-10-20, evening.** Section 0 is the current state of every open post (56), read in full on 2026-10-20 after
the 0.3.5 retest round. Sections 1–4 are the earlier rounds (history: their numbering is what older commits cite).
Players are on runtime 0.3.5 (launcher 0.2.1); "main" means fixed after 0.3.5, not released yet.

## 0. Every open post (2026-10-20)

Closed today (reporter confirmed in the post): footprints in Hoven's snow, plants / trees moving, the O2 mask at the
ship, Orxon's bombs, the Mirror World cheat, grind-rail weapons, the Mine Glove smoke, the indestructible mines,
Ratchet's head in the pause menu, Can't Start New Game (keymapping), the Xbox pad (keyboard joystick add-on). The
"Older posts" section of the forum (closed earlier, e.g. the reverb after the Pokitaru teleport) is not listed.

**New posts (not triaged before)**

| post | reporter | what | status / next |
|---|---|---|---|
| 1562010718641987654 | HMDSOLO | Visibomb: the green visor looks wrong; the firing animation is missing | new; 3 attachments, not downloaded |
| 1562008926613151774 | HMDSOLO | Hoven: the clouds below move with the camera (480p video) | new; sky / cloud layer parallax |
| 1562001348088561664 | HMDSOLO | Teleporters miss part of their sound | new; 6 attachments |
| 1561992729980837898 | HMDSOLO | Hoven water (Hydrodisplacer area) broken, "like the HD Collection"; PS2 and PS3 references posted | new; compare with the PS2 shots |
| 1561982401914339379 | HMDSOLO | The Glove of Doom does not hold the eggs inside | new; the glove hand offset (be6e72f, the Mine Glove's fix, shared by every glove) may already cover it: check |
| 1561920636593643580 | Kristian | Launcher (Windows 11): "Could not move build into … os error 5" on every install, with or without admin, any folder | new; same family as 1561546349793648701 (launcher) |
| 1561918614456504340 | idunnomate | Gemlik base: the first force-field tower becomes a Heli-Pack sound source (sometimes mutes the player's), by gliding off the start platform | new; a voice owner / position bug (the slot hand-over?) |
| 1561896146421686282 | Gengear | SteamOS: changing the volume with the controller (Steam + analog up/down) crashes the game ("GAME STOPPED"); keyboard is fine | new; crash in the options slider with a pad |
| 1561815960825102416 | everfeel | Sounds keep playing into the vendor; the crosshair shows in the vendor (roxas: the PS2 shows it too, Veldin 2 with the R.Y.N.O.); the yellow hydrant clips into the vendor menu on several planets | new; sounds and hydrant to check |

**Older posts with replies since the retest request (2026-10-19)**

| post | reporter | what was said | status / next |
|---|---|---|---|
| 1561295602464325733 | HMDSOLO | Wrench auto-aim: □ combos still miss enemies and crates; Hyper-Strike better but not right; the throw much better | the combo lunge 3b798b6 on main; ask again after the release |
| 1561116145958920253 | idunnomate, HMDSOLO | Ice: momentum good (idunnomate); weapons point at the ground; the wrench throw / catch do not push Ratchet | weapons 5ad9698, throw / catch 785eb3f, the jump lock 56f541c, all on main |
| 1560410178598211727 | HMDSOLO | Map star right on the current planet, in the corner or missing on others (PS2 shows it at each pad); Umbris does not fill yellow when completed | star: as designed since c3ee600 (each planet needs one visit on 0.3.5); Umbris fill: new, to look at |
| 1560406898736631849 | HMDSOLO | Mine Glove: the mine is in place now, but passes through the glove ("apart from that, it's fixed") | be6e72f on main |
| 1560484892951715910 | HMDSOLO | Gaspar turrets: no longer inverted | the reticle and the tilt still open |
| 1561090803533619351 | idunnomate | FOV: fixed after the dogfight; crashing the jet while boosting keeps it at the respawn | as the original (no FOV writer in the death path) |
| 1561170710410563624 | idunnomate, zshrocks | Persuader etc.: fine in gameplay and menus, missing in scenes / ship travel; zshrocks PR #43 (scenes, ship travel, glows) | bdc0fa4 on main covers the scenes; reconcile with PR #43 |
| 1561197235453169734 | idunnomate | Glove of Doom glow: an image, no text | fixed fc51251 (0.3.5); ask for a word before closing |
| 1560477029915168810 | HMDSOLO | Self-destruct music (0.3.4 videos); also the camera slides a little when it focuses on Ratchet | switches on main (test f6d8b57, heard by ear); asked to retest; the camera slide not looked at |
| 1560402964324155393 | roxas, HMDSOLO | Got more bug: O2 mask, Clank's antenna, hoverboard water and HUD, Magician, timers, cheats fixed; still audio crackle and speed-ups, Giant Clank pose (HMDSOLO posted a PS2 frame), skill point checks, mine hit box and Taunter, commando shots | skill checks a870581 and speed-ups 9336a71 on main; Giant Clank pose unresolved (§4 fourth pass); asked roxas which missiles pass through Giant Clank |
| 1560523101438222357 | Spoober, roxas | Big list: Codebot fixed, Swingshot fixed, Clank's light and Heli-Pack sound fixed; pause panels when spamming Start (more on Kerwan); Goodies movies show the NPC twice; camera under the quicksand; roxas: ice jump lock, speed-ups, not facing straight on the rail / Glove of Doom | pause 6cb39c3, movies d11b98b, ice 56f541c, speed-ups 9336a71, quicksand camera 066cb53 (faithful port, unverified), all on main; asked roxas for rail / Glove of Doom footage |
| 1561237746012524634, 1561391266959204382, 1561445647494938655 | HMDSOLO, idunnomate | Clank's Heli-Pack sound, Kalebo III water, the Codebot softlock: no reply in the posts | confirmed in the big threads (Spoober, roxas); close once the reporters confirm or on your word |

**Older posts with no reply since (state unchanged)**

| post | reporter | what | status |
|---|---|---|---|
| 1561035410677702737 | funee monkee | No ship to leave Oltanis Orbit | needs the save (§2.3) |
| 1560482718779703296 | HMDSOLO | Space too close during travel | to capture 4:3 vs 16:9 (§3) |
| 1560528077749227520 | HMDSOLO | Wrench reticle opaque | fixed 99ab08a on main |
| 1561617528063655988 | HMDSOLO | Blarg soldiers do not notice Ratchet | as the original (§3) |
| 1561621141335056394 | HMDSOLO | Metal Detector rings | no description (§2.8) |
| 1560400854425542717 | HMDSOLO | Helmets bigger, animation glitches | needs the PS2 side (§2.10) |
| 1561609406565851196 | HMDSOLO | Aerial turret HUD blurry | HUD High resolution option (§3) |
| 1561607428439547965 | HMDSOLO | Batalia textures | needs PS2 captures (§2.12) |
| 1561546349793648701 | Glitch master | Launcher: "Could not move the build into" | launcher log (§2.15); see 1561920636593643580 |
| 1560533067838390333 | HMDSOLO | Hydrodisplacer: camera dips under the floor (Rilgar, Pokitaru sewers) | open (§1.17); 066cb53 may touch it |
| 1561292714690224178 | HMDSOLO | Pokitaru sewers textures | §2.12 |
| 1561285436104642710 | HMDSOLO | Pokitaru waterfalls | §2.12 |
| 1561283743984132266 | HMDSOLO | Thruster-Pack turbo (dark trails) | the smoke's texture is dark in the data (§4 third pass); needs a PS2 clip |
| 1561282673308213278 | HMDSOLO | Jet fighter: no reticle, no missile HUD | keepcount: fixed in his upcoming batch |
| 1561274678180315166 | HMDSOLO | Decoy Glove: no reticle on the ground | fixed: the create left the velocity at the hand point, so the preview read it as thrown and the held reticle never drew |
| 1561272266459709460 | HMDSOLO | Decoy Glove: the decoy not held inside the glove | not triaged before (indexed as "Pokitaru deck"); likely the glove hand offset be6e72f |
| 1561271052456296448 | HMDSOLO | Decoys do not stay on Pokitaru's boats | not triaged before (indexed as "mines riding the boat") |
| 1561266123788255302 | HMDSOLO | Clank's second antenna in mission scenes | fixed ac0f9a3 on main |
| 1561261776539226182 | HMDSOLO | Orxon Clank-area textures | missing objects / layer (§3) |
| 1561236934947512401 | HMDSOLO | Orxon sky seam | §2.12 |
| 1561235689503203388 | HMDSOLO | Clank on Orxon plays Ratchet's track | fixed eca1c6b on main |
| 1561231247613763677 | HMDSOLO | Platinum Zoomerator: no turbo bar | keepcount: upcoming batch |
| 1561207360028934234 | HMDSOLO | Floating mine in the pause menu's Mine Glove animation | likely the glove fixes (PR #24, be6e72f); check |
| 1561203514447765584 | HMDSOLO | Weapon sound during the ship landing | needs a PS2 capture (§3) |
| 1561185074936811600 | idunnomate | Blarg helicopter audio crackle (Hoven turret) | needs an audio trace (§3) |
| 1561178470468427836 | MayaMachina | PS4 pad not detected (DS4Windows?) | not triaged before; controllers (§2.17) |
| 1560403211960057896 | HMDSOLO | Gaspar lava white structures | needs a PS2 capture (§2.9) |
| 1561085489359102032 | idunnomate | Purple water on Pokitaru | needs a PS2 reference (§2.11) |
| 1561126327271161906 | idunnomate | Heli / Thruster sound after the ice-cube death | probably the original (§2.18) |
| 1561109192507654224 | pederado | PS5 pad over Bluetooth on macOS | controllers (§2.17) |
| 1560907753696788541 | Marco | Black screen on Windows (Vulkan; DX12 works) | needs GPU and driver (§2.16) |
| 1560471128638623857 | HMDSOLO | Walloper effects | the glow is depth-tested as the original (§4 fourth pass); needs a PS2 clip |
| 1560847982658719765 | idunnomate | Wrench throw trail hides geometry | needs a PS2 capture (§3) |

## 1. Doable from the posts and Ghidra alone

Each has a description (and usually a video or screenshot) precise enough to find the original's code and compare.

**Blocks progress or changes gameplay**
1. **Quartu: softlock at the gold-bolt door after the Codebot.** After the door-unlock scene no input works but the
   camera (not even pause); the Codebot's model is missing during the scene and its animation plays afterwards.
   Blocks 100%. (idunnomate, Spoober)
2. **Magician skill point never triggers.** (roxas)
3. **Mirror World switches on by itself.** In New Game+, "cheat enabled" on every double jump and a left side-flip
   toggles Mirror World; the cheats menu is empty. The cheats' input-sequence test. (idunnomate; roxas saw it on Umbris)
4. **Weapons on the grind rail only fire straight to the right**; leaning while firing does not sweep the aim, so
   mines and gate switches cannot be hit while steering. (idunnomate, with old footage of the original)
5. **No wrench auto-aim on crates and enemies** (PS2 video posted). One player asks for it as an option.
6. **Blarg soldiers do not notice Ratchet** on Orxon and on Blarg Station (they do not run at him).
7. **Orxon's bombs (Clank area) cannot be knocked away** by Ratchet's wrench or Clank's punch (PS2 video posted).
8. **The Mine Glove's mines are hard to destroy** once placed.
9. **Damage that does not land:** the crashing ship's lasers do not hurt Ratchet; enemy missiles pass through Giant
   Clank without hurting him. (roxas)
10. **Enemy attacks do not break the green explosive canisters** (Gemlik; best seen with the Space Commandos).
11. **Kalebo III: dying on the grind rail** plays the ground death instead of falling off.
12. **The escape / underwater timers draw over the pause menu** (Blarg warship, Quartu, Drek's fleet, Deplanetizer).
13. **The camera does not snap behind Ratchet after a warp pad** (Oltanis Orbit; checked on PS2 by the reporter).
14. **The autosave icon never appears.**

**Cutscenes and camera**
15. **Clank shows two antennas in his mission scenes**: the extra one stays where the player stood before the scene
    (Blarg: Hydrodisplacer; Orxon: Magneboots, Infobot).
16. **Clank's footsteps keep playing during and after the Hydrodisplacer scene** (Blarg).
17. **The camera dips under the floor when the Hydrodisplacer is used** (Rilgar and Pokitaru sewers, Hoven; not on PS2
    per the reporter; videos posted).
18. **The field of view stays zoomed after Pokitaru's dogfight** when the booster was in use at the cutaway.
19. **Space looks too close to the camera during travel**, snapping to its place on arrival (screenshots posted).
20. **Giant Clank turns all red during his triangle attack** (PS2 footage posted).

**Sound**
21. **Clank on Orxon plays Ratchet's track** instead of "Clank's Second Foray" (needs the `ratchet1` Ghidra project:
    the track choice is not in the exported decompile).
22. **Blarg warship's self-destruct music never starts** (videos posted).
23. **No music during the ending credits.** (roxas)
24. **The equipped weapon's sound plays during the ship landing** (every planet; Blaster video).
25. **Static when a Blarg helicopter dies** under the Hoven turret (not in the original per HMDSOLO).
26. **The reverb stays after teleporting out of Pokitaru's sewers** (walking out of a reverb area clears it).

**Visuals with a reference or a clear description**
27. **The wrench reticle is opaque**; on PS2 it is glassy (PS2 screenshot posted).
28. **The thrown wrench's trail hides geometry behind it** (also Blarg Station's windows).
29. **The Walloper's effect glitches** where it crosses Ratchet's body, for half the punch directions (yaw dependent,
    zshrocks).
30. **The aerial turret's HUD is blurry / pixelated** (screenshot posted).
31. **Thruster-Pack and Hoverboard turbo effects are broken** (videos posted).
32. **Orxon's Clank area textures are broken** (PS2 screenshots posted for each spot).
33. **The Glove of Doom canister is slightly out of step with Ratchet's hand** (Spoober; may apply to every glove).

**Larger pieces (known, longer)**
34. Ratchet's worn items in scenes (Sonic Summoner and O2 mask clipping; G-CUT-009) — the ship part is PR #29.
35. Quartu: the monitor text in Clank's reunion scene is missing.
36. The Hologuise: Clank's transition, the antenna staying green when the alarm goes, its place in Ratchet's hand, the
    unstable transform (roxas).

## 2. Needs more input, screenshots or data

**Needs a reproduction or details from the reporter**
1. **Music and speech too fast** (roxas, on stream) / **music in slow motion after opening the vendor right after a
   music zone change** (Kerwan, Novalis). Track rates, the stream pitch through a vendor open and per-tick audio
   frames are ruled out. Needs: where it happened, the display's refresh rate, the audio device, and a log.
2. **Planet transitions hitch ("not responding")** before the planet loads. Needs a log and the PC's specs.
3. **Oltanis Orbit's ship disappeared** from both ends once (back after a restart). Needs the save and what was done
   before.
4. **Hoven "looks like it's being rebuilt"** after the planet is done. Needs a screenshot and where.
5. **Oltanis ice: an invisible wall / momentum loss** near the PDA, and the remaining ice items (melee drift, the
   wrench throw not pushing Ratchet, weapons pointing down while moving). Retest after PR #21.
6. **Hoven: a TNT crate and a crate inside each other** near the path to the Hydro-Pack. The placement data has no
   overlap (it happens at run time). Needs a screenshot of the spot.
7. **The Blarg Elite Commandos' shots are too dark** (Kalebo III). Needs a screenshot to tell which enemy.
8. **Metal Detector rings** — the post has no description. Needs what is wrong.
9. **Gaspar's lava "white structures"** — also on PS2, but more frequent here. Needs a PS2 capture of the same spot
   to compare the rate.

**Needs PS2 / PCSX2 references**
10. **Helmets (all) a little bigger, and their animation glitches** when Ratchet looks at enemies, draws a weapon,
    fires, or at low health (videos posted; the original's side is missing).
11. **Pokitaru's water turns purple** as the camera gets close (players like it; may be the original).
12. **Texture errors on Batalia** ("a UV error on almost every level"), **Pokitaru's sewers and waterfalls**
    (like Batalia's), **Rilgar and Blarg**, **Gaspar's and Orxon's skies**. Needs PS2 captures of the same views.
13. **The gold sheen on the gold weapons**, and the normal sheen on all weapons (Spoober compared with an emulator).
14. Earlier asks still open: the Mine Glove in hand, the helmets' size, pausing underwater, the Glove of Doom's
    canister timing.

**Launcher, setup and hardware**
15. **"Could not move the build into …"** when setting a runtime on Windows. Needs the launcher log and the install
    path.
16. **Black screen with sound on Windows** (Vulkan); `WGPU_BACKEND=dx12` works around it. Needs the GPU and driver
    to make DX12 the fallback where it is needed.
17. **Controllers:** a PS5 pad works on macOS by cable only (not Bluetooth); DS4Windows pads are not detected; an
    analog keyboard registering as a gamepad hid an Xbox pad (resolved by the player). Needs the hardware to test;
    players ask for a controller picker.

**Probably the original's behaviour**
18. The Heli/Thruster-Pack sound continuing after touching Hoven's water until a respawn (two players say the original
    does it).

## 3. Already done (not in the lists above)

**Fixed 2026-10-19 (from §1):** the Quartu Codebot door softlock (4968a3f: the slot waits for its fly-by like
`0x2f78e0`); the Magician skill point (cf7ebda: Kalebo's race host reads the Hoverboard's score); the cheats switching
on by themselves and the empty Cheats page (c80b9b3: the cheat tables were not relocated off level 01); weapons on the
grind rail (accd9d3: Ratchet's aim frame 0x13f990, the rail's yaw plus the aim lean steered by the stick); the wrench
auto-aim (23a24cf: the wrench states run the target search `0x22e238`); the escape / air timers over the pause menu
(14c8b55).

**Fixed 2026-10-20:** Clank on Orxon plays Ratchet's track (§1.21): the Clank section (class 22) requests
`MusicRequestTrack(2, 4)` from the first tick's moby loop, before the port's sound step made the level start's
`music_start_track(0)` (`entry` 0x259c40 makes it before its main loop): the idle player took track 2, then the start
overwrote it with 0. The start now runs before a class's first request (`AudioSystem::level_music_start`).
No music during the ending credits (§1.23): the last scene's end arms the music resume 30 ticks later
(`FUN_0027a460`), and `music_UpdateStream` counts it down inside `sound_update`; the boss enters the slideshow (mode 7)
at once, whose frames run only `sound_update`, while the port counted the delay in the gameplay frame only, so the music
stayed held through the credits. The countdown is now in `sound_update` (`music_resume_countdown`).
Clank's second antenna in his mission scenes (§1.15): the red light left where Clank stood is his antenna glow moby
0x140974; `FUN_002486c0` (the scene start's hide) hides it when Clank is the hero (0x1413f4 = 1) and `FUN_002487a8`
shows it again. The port's scene start now hides it with the other scene-hidden mobys.
The opaque wrench reticle (§1.27): the look stance's marker is `0xff917267` (alpha 0xff ≈ 2.0 on the GS), so the GS
blend `(Cs − Cd)·As + Cd` gives `2·Cs − Cd`: bright and tinted by the frame under it. The HUD layer kept the alpha but its
composite clamped it to 1 and drew the marker opaque. A second composite node takes the layer's pixels above 1 with a
subtract blend (`HudCompositeOver`).

**Looked at 2026-10-20, needs a PS2 reference:** the weapon sound during the ship landing (§1.24). The clip's sounds
are the hand item's take-out (the Blaster's class sounds 2 / 3, its sequence 0's triggers): `HeroItemsCreate`
0x22f3c0 creates the saved hand item at level start, and the landing's frame (0x2a4080 subs 0 / 8: `MobyUpdateLoop`,
which runs Ratchet's update as his moby's) runs the item pass as the gameplay frame does, so the original creates it
during the landing too. The port matches that code; whether the PS2 takes the sound out somewhere else needs a PCSX2
capture of a landing with the Blaster in hand. Clank's footsteps through the Hydrodisplacer scene (§1.16) need a run of
that scene as Clank (text-only report). Giant Clank red during his △ attack (§1.20): the emulator clip shows the beam's screen record
(0x15f330, `draw_fogged_fullscreen_sprite` after the shrubs in `DrawWorld`) reddening the terrain, ties and shrubs only;
the mobys and particles are drawn after it. The port's `screen_tint` pass runs after all the 3-D (its [L] note), so
Clank and his shot are tinted too: needs the tint inside the world pass, before the mobys (a renderer change). The blurry aerial turret HUD (§1.30): the gauges are 2-D HUD
art at the game's 512×416, scaled to the window and smoothed by the HUD option's Original setting (the TV's look);
the Sharp setting shows the pixels. The fix the post wants is the planned HUD High resolution option, not a bug. The
Kalebo III grind-rail death (§1.11): level 16's grind transitions (0x2270c8's case 0x28: the grind hit at health < 2
knocks Ratchet off backwards into the hurt 0x16) match the port; a headless run on rail 1 with a hit at 1 health goes
0x16 → the fall 6 → below the death height → the death fall 0x77, as the report says the PS2 does. Not reproduced:
needs the spot (a knock-off that lands back on a rail would end in the ground death 0x3d in both). The thrown
wrench's trail hiding the water (§1.28): the trail is the wrench's after-images (mode 0x80a: bit 8 fading, TEST_1
0x5308b = As ≥ 8 writes colour and Z; bit 0x800 deferred), drawn in `DrawMobys` before `RunDrawCallbacks_1`, the list
the ocean 1111 and the strip water draw from — the port's order too, so their Z hides the water behind them in both
unless the game masks Z for faded mobys (ZBUF ZMSK, not read). Needs a PS2 capture of a throw over water. The canisters
and enemy attacks (§1.10): the tanks 1261 take any record through `MobyGetHitMessage(0x330001)` and the resolver gives
out5 = 2 for every attack type, and enemy shots with a template (the turret's 1238, flags 0x10001) line-test into
mobys with mode 0x4000 as the hero's do; the Space Commandos' class is not identified (needs the spot). Hoven's helicopter static (§1.25): the clip
clips only a handful of samples; the noise is a broadband 4–7 kHz block before the first explosion and two clicks on
later ones; the copter 1255's own code plays no death sound (its burst's sound is −1), so the source is another
voice (its rotor loop or the explosions' shared sounds): needs an `RC_AUDIO_TRACE` run of the turret section. Orxon's
Clank-area textures (§1.32): the PS2 shots show lit square panels on the walls and the door, and vent grilles on the
raised blocks, that the port's shots lack — missing objects or a layer rather than wrong textures; needs a run there
to name the classes at those spots. Space too close during travel (§1.19): in the clips (16:9) the flight's
stars are larger and stretched sideways, the arrival's (the level's sky) round and fine. The flight sky is the
transition lump's six shells (`flight_render::update_sky`, one scalar scale each) with the sky camera's tangents raised to
at least `view_tans(sky_tan)` (Hor+ included), so the stretch is not in those two places; next: capture a flight at 4:3
and 16:9 (take off from Novalis's pad, choose a planet) and compare the sky camera's projection and the shells'
texture against the level's.

**Fixed later 2026-10-19:** the warp pads' camera (4f34672: a class teleport turns Ratchet's moby rows before the
camera reset snaps behind them); the FOV after Pokitaru's dogfight (142909a: a scene's end restores the gameplay
view, FUN_002ac608); the autosave icon (8447e57: the main loop's card icon 0x755d while the card is in use).

**Verified 2026-10-19 as the original's behaviour (code, data and a run of the classes):**
- §1.6 soldiers. Blarg Station's troopers 1048 wake and close in from 30 inside their arena (19 of 20 in a run; the
  leapers wait for their trigger cuboid, group 23 is parked in 0x12 until the boss / escape unparks it); they never
  leave the arena polygon, so the one behind the force field in the clip stays there. Orxon's brawlers 1202: the ten
  lone ones (idle +0x1b4 = 1) chase from 30 when Ratchet is within 3 in height; the 47 in packs sleep (state 1, sight 0)
  until a path scout calls them, a member is hit or touched (the group wakes), the Taunter or the Morph-o-Ray — the
  original's `0x2e1d38` / `0x2e3ef0` / `0x2e47d8` line for line; the seven scouts fly and wake their brawlers (one,
  #626, is zapped by the barrier 794 it flies through after its calls, as the original's zap lists any moby with
  collision). The clips show a soldier on a ledge above Ratchet (more than 3 in height) and packs not yet called.
- §1.26 the reverb after the sewer teleport: Pokitaru has no reverb boxes; its two reverb env points sit at the sewer
  pads, and the pad up top has no point within 8, while `HeroTeleport` is the only caller of `EnvNearestSamplePoint` on
  level 11 — the original keeps the sewer reverb too.
- §1.7 / §1.8 Orxon's bombs are the lava rocks 938 (the PS2 clip: hit, red, flown, burst). They ignored the wrench
  because the port's wrench hits named no attacker (the game's name the wrench moby, class 0x47, `FUN_0026e808`); the
  Mine Glove's mines test the same `attacker->class != own`. Fixed: the hit records the hero's swings write carry a
  wrench marker (`HitRecord::wrench`, `attacker_class` / `attacker_pos`), read by every class that tests the attacker
  (lava rocks, mines, Kerwan's layers and bystanders, the Qwark ship parts, the gunners and biters, the lights, the
  tanks and turrets, the drones, the amoeboids).

**Fixed on main, not yet released:** the Morph-o-Ray while moving (849f873); the Glove of Doom glow blue or yellow by
planet (fc51251); Ratchet on the ground during Giant Clank (6126b01); Kalebo III's race water (f8b3016); Clank's antenna
light at his feet in scenes (a9f976c); the ship star in the map's corner (c3ee600); the ammo counter after death
(8b2c9a9); the underwater tint over the pause and map pages (49651f6); Comet-Strike through doors (in 9d114f1); the
bomb in hand at a gold-bolt pickup (3974efd); Port Options in the main menu and the scroll arrows (9fb4c40).

**Open PRs:** trees sway (#39); Veldin Orbit's music after the fleet mission (#38); the O2 mask / helmet and Clank's
glow on the ship's take-off and landing (#29); always-on items such as the Persuader drawn on Ratchet (#28); the Mine
Glove's mine in hand and its smoke (#24, likely also the pause-menu floating mine); Clank's hover and Heli-Pack sounds
(#23); momentum on slippery surfaces (#21); Port Options in the main menu (#30, duplicates 9fb4c40). keepcount reports
the jet fighter's reticle and missile HUD and the Platinum Zoomerator's turbo bar fixed in an upcoming batch.

## 4. Retest replies on 0.3.5 (read 2026-10-20; new attachments in `work/forum`, `urls.tsv`)

**Confirmed fixed:** footprints in Hoven's snow, trees sway, the O2 mask at the ship's landings, Orxon's bombs, the
Mirror World / cheats (and the Cheats page shows them), grind-rail firing (idunnomate's clip), the Mine Glove smoke, the
mines set off by the wrench, Ratchet's head in the pause menu, Gaspar's turret pitch, ice jump momentum (idunnomate),
the Swingshot claws, the Morph-o-Ray while moving, Clank's antenna light, Clank's Heli-Pack sound, the Kalebo III race
water and HUD, the Magician skill point, the timers over the pause menu, the Glove of Doom glow (one colour on two
planets, image only).

**Partly fixed / still open (new detail):**
- Wrench auto-aim: the swing combos (□) on enemies and crates still miss; the Hyper-Strike is better but not right; the
  throw is much better (HMDSOLO).
- Always-on items (Persuader, Map-o-Matic, Bolt Grabber): fine in gameplay and the menus, missing on Ratchet in the
  scenes, the ship's included (PCSX2 comparison posted).
- FOV after the dogfight: fixed for the scene; crashing the jet while boosting keeps the FOV at the respawn (clip).
- The map's ship star: right on the current planet; on other planets' maps it sits in the corner or is missing (PCSX2
  shows it at each planet's pad). Also: Umbris's map does not fill yellow on completion.
- Mine Glove in hand: in place now, but the mine pokes through the glove (PCSX2 comparison).
- Ice: weapons still point at the ground while moving; the wrench throw and catch do not push Ratchet across the ice
  (VP44, VP45); roxas still gets stuck in place on some ice jumps (Oltanis, Hoven).
- Grind rail / Glove of Doom: roxas says Ratchet still does not face straight when firing (images).
- Giant Clank: Ratchet rides on Clank's back now but not in the holding pose.
- Skill points page: no check marks on earned skill points.
- Mines: odd hit box for the wrench; the Taunter does not set them off.
- The Blarg Elite Commandos' shots too transparent (image).
- Audio crackling and music / speech speed-ups (roxas).
- The pause menu still shows stray panels when Start is pressed repeatedly, more often on Kerwan (Spoober suspects the
  frame rate).

**New:** in-level movies replayed from the Goodies show the level's own NPC next to the scene's actor (two Big Als, a
second Qwark); the camera dips under Aridia's quicksand (image).

**No reply yet:** the Codebot softlock, Spoober's other items (he is replaying; has reached Blarg Station).

**Worked on after the retest (2026-10-20):**
- Fixed: the Skill Points page's check marks (a870581: 0x290eb0 flag 0x200); the always-on items on Ratchet in scenes and
  the ship scenes (bdc0fa4: `FUN_0024a0e8` on the actor); the held mine inside the Mine Glove (be6e72f: the hand offset
  turns with joint 0, `fun_001f9d20`); the Comet-Strike's pushes on ice (785eb3f: L00 case 0x15); the combo's lunge at the
  aimed target (3b798b6: 0x239780's target branch; a crate 34° off and 3.6 away breaks on the first swing — no miss
  reproduced for the swings themselves).
- As the original (no change): the FOV after crashing the jet while boosting — no writer of the view tangent runs between
  the crash and the respawn (`LoadLevelCoreData`'s death path, the loader pass, `entry`'s death branch on levels 01 / 11);
  the map's ship star on other planets — the landmark is captured at every save since c3ee600, so a 0.3.4 save keeps its
  (0, 0) until each planet is left once in 0.3.5.
- Second pass (2026-10-20, decomp-driven): guns held level in the ice slide (5ad9698: `0x22e660` pitches joint records
  8 / 9 to the special poses' angle, −55° in 0x37); the ice jumps keep the slide's momentum (56f541c: L00 `0x214658`'s
  slippery air rates; the stick had stopped Ratchet in mid-air: the "locked in place"); the pause menu's stray panels
  (6cb39c3: a frame that runs several ticks opened the menu and ran its first tick, so the snapshot held the panels;
  reproduced with `RC_DUMP_TICKS=2,1,3,1` on Kerwan — the script is indexed by loop frames and drives the menu); the
  game running fast (9336a71: on a ~60 Hz display whose vsync does not pace the loop the tick rule ran up to a tick per
  frame — movement, music and speech up to a quarter fast; the 1/60 s limiter now latches on).
- Checked and as the original (no change): the grind-rail aim (the Blaster's barrel follows the rail, records 1..3 twist
  the chest 84°); the Glove of Doom's throw (state 0x23 and sequence 0x2c as ported); the wrench's reach on mines (the
  wrench class's swing sequences stretch joint list 1 to 1.8–2.6: the smear, the game's data); the Taunter on mines (it
  only triples a landed mine's seek reach toward enemies); the hover troopers' 638 shots 49 (hidden mobys, the type-2
  trail puffs 0x8000c070 / 0x4000c0c0 with the 0x44 blend); Giant Clank's pilot (sequence 0x81 of the level's table at
  joint list 5, `0x2061f0`).
- Third pass (2026-10-20): the Goodies' In-Level Movies show the level's own NPC beside the scene's (d11b98b:
  `MenuSetPostAction` case 6 keeps the item's hook slot +6, `0x179638[slot]` in 0x1ba25c, hidden by `DialogStreamStart`);
  the camera line passes through the surface Ratchet was last in (066cb53: `0x20eb40`'s 0x167350, quicksand 0x3b4 —
  a faithful port; whether it is what Spoober's quicksand shot shows needs a PS2 capture of that spot). Checked and as
  the original: Blarg's self-destruct music (the escape's (6, 9) switches in a headless run and in a test with scene
  pauses at any point of an earlier transition: f6d8b57; the report is from 0.3.4; confirmed by ear on main); the
  Quartu jet robots' shots 819 against Giant Clank (level 15's copy `0x2dd730`: damage 1.0, flags 0x10003, body 2
  accepts 0x2; his class has collision; 1 of 200 energy a shot — which "missiles" roxas means needs the spot); the
  Thruster-Pack smoke (type-23 puffs, grey 0x808080, blend 0x44 as `0x2c98b8`; its texture `def[23]` is dark grey (≈80,
  alpha to 160), so the GS draws dark puffs: needs a PS2 clip to compare density and length). Giant Clank's red △ tint:
  needs every moby's draws late while the tint is up (a renderer task, see §1.20).
- Fourth pass (2026-10-20): Giant Clank's pilot — HMDSOLO posted a PS2 frame (`work/forum/files/1560402964324155393/
  1562025033889480784_Screenshot_YouTube.jpg`): Ratchet's hands above his head on the back's top edge. The port plays
  sequence 0x81 (level 15's `0x208b80` and level 00's `0x2061f0` both; the loader maps class sequence k to table entry k,
  Lombyte `l06/world/loaders/00252dd0.c`), whose hands sit at chest height (0.75 vs the head's 0.93); sequence 0x80 has
  the hands above the head (1.23 vs 1.08). The game re-blends 0x81 every tick (`moby_set_anim_snapshot` has no early
  out), converging on frame 0: still the chest pose. Unresolved. The Walloper's fist glow (`0x2d1e08` → `FastDrawQuadReal`)
  sends no TEST register: depth-tested as the port draws it, so its cut through Ratchet's body (reproduced punching
  toward the camera) would be the PS2's too; needs a PS2 capture. Asked roxas which missiles pass through Giant Clank,
  and HMDSOLO to retest the self-destruct music.
- Still open: the audio crackle (the ring's fill, drops at the cap and underruns are now in the 600-tick audio line; none
  seen in a 45 s Kerwan run here); what roxas means by "not facing straight" on the rail.
