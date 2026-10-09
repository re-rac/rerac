//! The class-port units of the census (docs/plan/class_census.md "Cheap wins", gaps.md G-CLS-027): classes whose
//! update calls only shared functions the port already has, each ported once per **unit** (the classes and the level
//! copies that run the same code, `LevelPorts` / `Relocation`) and registered here as `ClassUpdate::Unit(i)`, the
//! index into [`PORTS`]. A row names the unit's reference update (a function of the level's overlay the census lists
//! as its first copy), the classes that level's table runs it for, and the Rust update. Every other level whose class
//! table names the same code runs the same row.
//!
//! | unit | classes (levels) | reference update | port |
//! |---|---|---|---|
//! | U408 | 212, 1412 (13) | level13 0x2e1638 | [`asteroid`] |
//! | U303 | 1181 (09) | level09 0x304360 | [`chain_link`] |
//! | U553 | 885, 888, 891, 892, 894, 900, 901, 936 (18) | level18 0x2e9768 | [`barricade`] |
//! | U294 | 1182–1189 (09) | level09 0x2c26c8 | [`tethered_platform`] |
//! | U417 | 1261 (13), and the fireball 1634 it makes | level13 0x307b10, 0x30c3b8 | [`explosive_tank`] |
//! | U533 | 1359–1364, 1367, 1369, 1372, 1373 (17) | level17 0x2e87d8 | [`fleet_door`] |
//! | U477 | 937 (15) | level15 0x2e46b0 | [`linked_cog`] |
//! | U479 | 1250 (15) | level15 0x2e73c0 | [`quartu_belt`] |
//! | U500 | 650 (16) | level16 0x2d5cb8 | [`rising_float`] |
//! | U499 | 647 (16) | level16 0x2d59e0 | [`extending_piece`] |
//! | U563 | 1584 (18) | level18 0x2fa728 | [`veldin_carrier`] |
//! | U559 | 1432 (18) | level18 0x2f7ab0 | [`hidden_prop`] |
//! | U484 | 1425 (15) | level15 0x2eb928 | [`bubble_vent`] |
//! | U456 | 1397 (14) | level14 0x3061d8 | [`oltanis_switchboard`] |
//! | U493 | 482 (16) | level16 0x2cb600 (`DeleteMoby(self)`) | [`marker::update`] |
//! | U99 | 87, 283, 346, 419, 690, 765, 789, 1104, 1140, 1278, 1280, 1672 (02, 05–07, 10, 12, 14–16, 18) | level02 0x2dd4d0 (`jr ra`) | [`empty`] |
//! | U27 | 1060 lamps (00, 02, 05, 18) | level00 0x2df4f8 | [`lamp`] |
//! | U268 | 344, 547–551, 588–598, 782–785 loose pieces (08) | level08 0x2dba40 | [`loose_piece`] |
//! | U95 | 296, 652, 653 conveyor belts (02, 12) | level02 0x2dc6b0 | [`conveyor`] |
//! | U241 | 886 timed switches (07, 12) | level07 0x30bf90 | [`timed_switch`] |
//! | U247 | 129, 130, 182, 183, 360, 1063, 1078, 1079, 1131 movers on a linked moby's state (07, 13) | level07 0x310df0 | [`linked_mover`] |
//! | U229 | 1512 steam / spark vents (06) | level06 0x308c68 | [`vent`] |
//! | U280 | 621 rail mines (08) | level08 0x2f44c0 | [`grind_mine`] |
//! | U185 | 852, 853 rising blocks (05) | level05 0x314eb0 | [`rising_block`] |
//! | U221 | 1091–1098, 1103 bobbing blocks (06) | level06 0x300df0 | [`bob_block`] |
//! | U139 | 915, 916, 917 markers deleted at once (03, 10) | level03 0x2dc310 | [`marker`] |
//! | U281 | 648 smoke emitters (08, 10) | level08 0x2f5830 | [`smoke_emitter`] |
//! | U170 | 341 Hydrodisplacer pads (05, 07, 11, 12, 18) | level05 0x2f8080 | [`hydro_pad`] |
//! | U96 | 615 Trespasser locks (02, 04, 06, 08, 11, 13, 18) | level02 0x2d8ad0 | [`trespasser_lock`] |
//! | U107 / U108 | 743 / 744 the sliding doors of the Trespasser lock (02) | level02 0x2dffc8 / 0x2e0138 | [`lock_doors`] |
//! | U365 | 1159 the split door of the Trespasser lock / floor switch (11) | level11 0x30e978 | [`lock_doors`] |
//! | U204 | 367 sliders (06) | level06 0x2d9d10 | [`slider`] |
//! | U82 | 1341 the Novalis help-hint director (01) | level01 0x30acb8 | [`help_director`] |
//! | U36 | 1564 path gliders (00, 07, 10, 18) | level00 0x2e3a88 | [`path_glider`] |
//! | U495 | 257 Kalebo rail cars (16) | level16 0x2c3d38 | [`rail_car`] |
//! | U523 | 1667–1671 Kalebo air traffic (16) | level16 0x2e76b8 | [`kalebo_traffic`] |
//! | U25 | 749 Veldin horny toads (00, 18) | level00 0x2d4610 | [`horny_toad`] |
//! | U287 | 1023 hopping gunners (08, 09), and their shot 1292 | level08 0x301158, 0x307298 | [`hop_gunner`] |
//! | U553 | 568 rolling mines (18; the pool of the boss 1422) | level18 0x2d5918 | [`rolling_mine`] |
//! | U301 | 193 pack biters (09, 15) | level09 0x2e27d8 | [`pack_biter`] |
//! | U268 | 252 hover zappers (08, 14), and their draw callbacks (the glow, the arc) | level08 0x2d2af0, 0x2d4108, 0x2d3878 | [`hover_zapper`] |
//! | U407 | 63 flying biters (13) | level13 0x2b50d8 | [`flying_biter`] |
//! | U300 | 52 buzz bombs (09, 16) | level09 0x2c5990 | [`buzz_bomb`] |
//! | U521 | 1445 area stalkers (16) | level16 0x2e5e08 | [`area_stalker`] |
//! | U426 | 1271 the wave gate (13) | level13 0x30af50 | [`wave_gate`] |
//! | U183 | 838 Rilgar laser fences (05; one looping voice per group, G-AUD-010) | level05 0x30dc68 | [`laser_fence`] |
//! | U32 | 1413 the Veldin help director (00) | level00 0x2e0988 | [`help_veldin`] |
//! | U119 | 1324 the Aridia help director (02) | level02 0x2ee890 | [`help_aridia`] |
//! | U145 | 1342 the Kerwan help director (03) | level03 0x2df520 | [`help_kerwan`] |
//! | U165 | 1343 the Eudora help director (04) | level04 0x2e4418 | [`help_eudora`] |
//! | U204 | 1347 the Rilgar help director (05) | level05 0x31bf40 | [`help_rilgar`] |
//! | U232 | 1348 the Blarg help director (06) | level06 0x3083b0 | [`help_blarg`] |
//! | U292 | 1349 the Batalia help director (08) | level08 0x307540 | [`help_batalia`] |
//! | U305 | 1000 the Gaspar help director (09) | level09 0x300888 | [`help_gaspar`] |
//! | U341 | 1344 the Orxon help director (10) | level10 0x2e85b8 | [`help_orxon`] |
//! | U391 | 422 the Hoven help director (12) | level12 0x2ed280 | [`help_hoven`] |
//! | U419 | 558 the Gemlik help director (13) | level13 0x2f3778 | [`help_gemlik`] |
//! | U470 | 77 Quartu alarm drones (15, 17) | level15 0x2a2488 | [`quartu_drone`] |
//! | U480 | 408 Quartu alarms (15, 17), and the drones they release | level15 0x2cb4c8 | [`quartu_alarm`] |
//! | U216 | 1039 kill cuboids (06, 08, 13, 18; off while flying a ship: hero state 0x32, G-HERO-002) | level06 0x2f7930 | [`kill_volume`] |
//! | U274 | 438 Batalia's circling fighters (08; shot down from the turret in hero state 0x32) | level08 0x2de848 | [`batalia_fighter`] |
//! | U474 | 123 swinging lasers (15, 17) | level15 0x2a6e68 | [`swing_laser`] |
//! | U411 | 127, 128, 159, 169 rotators on a linked moby's state (13) | level13 0x2c7f38 | [`linked_rotator`] |
//! | U335 | 1196 Orxon's path scouts (10; wake the brawlers' groups) | level10 0x2df270 | [`orxon_flyers`] |
//! | U336 | 1199 Orxon's swoop flyers (10) | level10 0x2e01a8 | [`orxon_flyers`] |
//! | U337 | 1202 Orxon's brawlers (10; Clank's-part branches: G-HERO-005) | level10 0x2e1d38 | [`orxon_brawler`] |
//! | U323 | 22 Orxon's Clank section (10: Clank in charge without the O2 Mask, `crate::hero::bodies`) | level10 0x298b68 | [`clank_section`] |
//! | U500 | 1451 / 1899 Giant Clank's pads (15, 18: in and out of Giant Clank, `crate::hero::bodies`) | level15 0x2ed068 | [`giant_pad`] |
//! | U220 | 1061 Blarg's Clank station (06: Ratchet and Clank trade places, `crate::hero::bodies`) | level06 0x2fc640 | [`blarg_clank_lift`] |
//! | — | 0x593 Giant Clank's landing shockwave (15, 18; made by the hero code, `crate::hero::bodies::giant`) | level15 0x29ead0 | [`giant_shockwave`] |
//! | — | 0x100 Giant Clank's missiles (15, 18; made by the hero code: a copy of the Devastator missile with its own search) | level15 0x29d6c0 | [`giant_missile`] |
//! | — | 0x5f3 Giant Clank's head beam (15, 18; held by the hero code, then flies) | level15 0x29edb0 | [`giant_beam`] |
//! | U499 | 1446 Quartu's Giant Clank mission NPC (15: the talk, the two groups, out of Giant Clank, planet 16) | level15 0x2ec760 | [`quartu_giant_mission`] |
//! | U307, U316, U255 | 664, 1293 / 1320 Gaspar's sinking floats (09), 1069 Umbris' rocking floats (07): they carry Ratchet by writing his platform delta (`HeroFields::ride`) | level09 0x2f86a0, 0x3091b0, level07 0x3112c8 | [`riding_floats`] |
//! | U407 | 29 Gemlik's gun turrets (13), their rider 36 and shot 1238 (created by code) | level13 0x2b41b8, 0x2b4c80, 0x306300 | [`gemlik_turret`] |
//! | U215 | 1038 orb holders (06, 10, 17), and the orb 1040 each makes | level06 0x2f7288, 0x2f7ab8 | [`orb_holder`] |
//! | U503, U502, U514 | 552 barrier posts, 546 switches, 1387 walls (16) | level16 0x2cf4a8, 0x2cf198, 0x2e36e8 | [`kalebo_barrier`] |
//! | U185 (2026-09-29 run) | 843 sliding blocks placed by a cuboid (05) | level05 0x30e508 | [`cuboid_slider`] |
//! | U473 | 93 swing doors (15) | level15 0x2a3ba8 | [`swing_door`] |
//! | U307 | 1172 chain anchors (09) | level09 0x303d10 | [`chain_anchor`] |
//! | U477 | 196, 197, 1958 sliding doors (15, 17) | level15 0x2bddb0 | [`slide_door`] |
//! | U102, U126, U179 | 707 / 734 turntables (02), 1210 joint-carried platform (03), 812 pinned platforms (05) | level02 0x2ddc00, level03 0x2953f8, level05 0x30bf98 | [`carriers`] |
//! | U565 | 1381 falling platforms carrying the Veldin carriers (18) | level18 0x2f16f0 | [`falling_platform`] |
//! | U207, U472 | 1511 breakable light fixtures (05, 14) | level05 0x31c8e0, level14 0x307ad8 | [`light_fixture`] |
//! | U514 | 1143 Gadgetron logos (16 placed; every level's vendor hologram; a manipulator on its own list 1, `crate::moby_update::manip`) | level16 0x2e1088 | [`hologram_logo`] |
//! | U180 | 823 sweeping searchlights (05, 07; a manipulator on its head, the beam callback 0x30c220) | level05 0x30c0a8 | [`sweep_light`] |
//! | U155 | 481 bobbing floats with three spinning parts (04; look-at records `manip::look`, a platform) | level04 0x2cdda0 | [`spinner_float`] |
//! | U203 | 1139 hoverboard-course sparkles (05, 16) | level05 0x31abe0 | [`board_sparkle`] |
//! | U177 | 717 Rilgar's hoverboard racers (05; their boards are created 439s) | level05 0x307910 | [`rilgar_racer`] |
//! | U171 | 133 the hoverboard course's roaming boost pickups (05) | level05 0x2dac80 | [`board_boost`] |
//! | U503 | 556 Kalebo III's hover racers (16; Rilgar's racer code through `rilgar_racer::Layout`, shot down and back) | level16 0x2d04a0 | [`kalebo_racer`] |
//! | U174 | 439 the Hoverboard (05, 16; mounts Ratchet, its thrusters, and the hero code's stores: `crate::hero::hoverboard`) | level05 0x2f87a8 | [`hoverboard`] |
//! | U440 | 30 Oltanis pop-up turrets (14), and their shot 681 | level14 0x2b3bf0, 0x2ece00 | [`popup_turret`] |
//! | U212 | 1021 Blarg petal doors (06) | level06 0x2f4f00 | [`petal_door`] |
//! | U390 | 339 Hoven's animated idlers (12) | level12 0x2ec1d0 | [`anim_idler`] |
//! | U248 | 1013, 1014, 1064, 1065 panels on a linked moby's state (07) | level07 0x30cf90 | [`linked_slider`] |
//! | U329 | 1015, 1282 Orxon trip blocks (10) | level10 0x2d90a8 | [`trip_block`] |
//! | U162 | 1101, 1102, 1531, 1532 Eudora switched movers (04) | level04 0x2e17d8 | [`switched_mover`] |
//! | U487 | 1209 Quartu pressure pads (15, 17) | level15 0x2e5958 | [`pressure_pad`] |
//! | U211 | 911 Blarg flame jets (06) | level06 0x2f3ad8 | [`flame_jet`] |
//! | U542 | 669 the fleet's underwater laser spinners (17; only while Ratchet is in the water) | level17 0x2d77f0 | [`water_laser`] |
//! | U349 | 1544 Orxon's particle vents: puffs, drips, columns (10) | level10 0x2ea1f0 | [`orxon_vent`] |
//! | U375 | 1246 Pokitaru's biters: beach, swimmer, boat boarders (11) | level11 0x314318 | [`pokitaru_biter`] |
//! | U95, U101 | 580 Aridia's sand sharks and their nests 668 (02) | level02 0x2d3e50, 0x2dcb38 | [`aridia_sandshark`] |
//! | U96 | 612 Aridia's flame-throwing sentries (02) | level02 0x2d7748 | [`aridia_flamer`] |
//! | U373 | 1231 Pokitaru's ball throwers (11), and the ball 1297 they make | level11 0x310180, 0x318b30 | [`pokitaru_thrower`] |
//! | U128 | 75, 115–120, 132, 795 Kerwan's air traffic (03), and the exhaust trail 235 of 75 / 119 | level03 0x29dba8, 0x2bae48 | [`air_traffic`] |
//! | U126 | 868, 905, 928 Kerwan's swinging path movers (03) | level03 0x294c08 | [`kerwan_mover`] |
//! | U506 | 471 Kalebo's reversing belts (16): the group's belt push, the shared voice, two scrolling quad layers | level16 0x2c9878, draw 0x2c9cd0 | [`kalebo_belt`] |
//! | U541 | 99 the fleet's sliding laser emitters (17): beam hit line, type-79 sparks, beam quad | level17 0x2a8da0, draw 0x2a95b8 | [`fleet_laser`] |
//! | U485 | 221 Quartu's shaking critters (15): head look-at, shake, dust, death pieces | level15 0x2c2938 | [`quartu_critter`] |
//! | U567 | 1355 Veldin's divers (18; released by the boss 1422): wander, dive, crash, trails and glows | level18 0x2efb88, draws 0x2f0390 / 0x2f06c0 | [`veldin_diver`] |
//! | U162 | 617 Eudora's path riders (04) | level04 0x2d6f68 | [`eudora_path_rider`] |
//! | U167 | 466 / 480 / 485 / 486 / 488 / 490 / 493 / 494 / 495 / 498 / 555 Eudora's flying machines (04): path flight, rotors, riders, landing pads, blown apart and back | level04 0x2ca420 | [`eudora_flyers`] |
//! | U163 | 340 Eudora's brawler bots (04): wander or patrol, circle in on Ratchet, swing; pushed back, blown up | level04 0x2c2270 | [`eudora_brawler`] |
//! | U164 | 427 Eudora's gunners (04): keep their distance inside an area, fire bursts of gun shots, strike up close, dissolve when killed | level04 0x2c4850 | [`eudora_gunner`] |
//! | U162 | 217 Eudora's loggers (04): carry a prop, walk a path when roused, swing; flinch, stagger, knocked down, blown up | level04 0x2ba520 | [`eudora_logger`] |
//! | U170 | 563 Eudora's leg walkers (04) on the shared leg walker (`creature::legs`), and their wood chips 1516 | level04 0x2d16b8, 0x2e5178 | [`eudora_walker`] |
//! | U160 | 86 Eudora's flock spawners (04): five class-85 members each, steered by the engine's boids | level04 0x29ecf8 | [`eudora_flock`] |
//! | U172 | 584 Eudora's grabbable blocks (04): a carrier with a ledge record | level04 0x2d3580 | [`eudora_ledge_block`] |
//! | U174 | 642 Eudora's drifting speck (04): sinks on the level wind, grows in, fades, starts again | level04 0x2d7e90 | [`eudora_drifter`] |
//! | U179 | 1549 Eudora's cutscene FX driver (04): the infobot's thrusters in scenes 0 / 1 | level04 0x2e53a0 | [`eudora_scene_fx`] |
//! | U375 | 361 Pokitaru's spline wall (11): names the ring the hero's capsule pass keeps Ratchet inside (`Hero::wall_spline`) | level11 0x2f3350 | [`pokitaru_wall`] |
//! | U401 | 1350 Pokitaru's skill-point watcher (11): skill point 0x13d41b once its group is gone | level11 0x31aa80 | [`pokitaru_skill`] |
//! | U389 | 1179 the Thruster-Pack floor buttons (11, 15): stomped down for good, saved on Pokitaru, with their help hints | level11 0x30ee00 | [`pokitaru_button`] |
//! | U390 | 1180 the button-turned piece (11): turns 60° when its button is pressed | level11 0x30f2a8 | [`pokitaru_turner`] |
//! | U388 | 1178 Pokitaru's tilting platforms (11): tip under Ratchet's weight and carry him | level11 0x30eb90 | [`pokitaru_tilt`] |
//! | U384 | 1156 Pokitaru's unfolding machine (11): 18 pieces that fold out in a cutaway when its button is pressed | level11 0x30c788, 0x30d1b0 | [`pokitaru_unfold`] |
//! | U559, U560 | 1439, 1441 Kalebo's pass-through gates (16): pulse, click and set their command byte when Ratchet passes | level16 0x2e5010, 0x2e5258 | [`kalebo_gates`] |
//! | U561 | 1442 Kalebo's rail switches (16): turned on by a lean while grinding past, or a hit | level16 0x2e54a0 | [`kalebo_rail_switch`] |
//! | U569 | 1891 Kalebo's spinning sign (16) | level16 0x2e88d0 | [`kalebo_spinner`] |
//! | U565 | 1561 Kalebo's grind skill point (16): skill point 0x13d421 | level16 0x2e7208 | [`kalebo_grind_skill`] |
//! | U570 | 1923 Kalebo's chicken pad (16): sends hidden chickens through teleporter pad #1192 | level16 0x2e8970 | [`kalebo_chicken_pad`] |
//! | U568 | 1826 Kalebo's lifts (16): call or ride to the other end; the glow column `0x2e84e8` | level16 0x2e7f80, 0x2e84e8 | [`kalebo_lift`] |
//! | U567 | 1812 the jets in Kalebo's first scene (16) | level16 0x2e7e00 | [`kalebo_scene_jet`] |
//! | U552 | 654 Kalebo's arena triggers (16): barriers, the script camera, the enemies, the spline wall | level16 0x2d5ef8 | [`kalebo_arena`] |
//! | U558, U215 | 1410 Kalebo's cars (16), 998 Rilgar's cars (05): ride between stations on △ (one code per level) | level16 0x2e43e0, level05 0x318c78 | [`hover_car`] |
//! | U545 | 541 Kalebo's arena troopers (16): grenadiers, sweepers and flamers on their paths, woken by 654; their glow `0x2cef60` | level16 0x2cddb8, 0x2cef60 | [`kalebo_trooper`] |
//! | — | 281 the troopers' grenades (16; created by code: `0x2c6268`) | level16 0x2c5eb0 | [`kalebo_grenade`] |
//! | U216, U214, U220 | 1099 the scene-hidden prop, 984 the bobbing marker, 1550 the scene FX driver (05) | level05 0x319c28, 0x318b98, 0x31d5c8 | [`rilgar_small`] |
//! | U198 | 841 Rilgar's sewer fog switch (05): the level fog, the underwater tint, the water level | level05 0x30e1f8 | [`rilgar_fog_switch`] |
//! | U210 | 920 / 447 the watching bystanders (05, 07, 13): the head look-at, the scene big head | level05 0x317aa0 | [`rilgar_watcher`] |
//! | U201 | 846 Rilgar's breakable posts (05): a burst of loose pieces and sparkles | level05 0x30f5c8 | [`rilgar_breakable`] |
//! | U204 | 877 Rilgar's lift pads (05): called, or a cutaway ride with the spline wall | level05 0x3156d0 | [`rilgar_lift_pad`] |
//! | U184 | 79 Rilgar's trail riders (05): banked spline loops, ribbon trails `0x2d7920`, joint glows `0x2d6f48` | level05 0x2d7140 | [`rilgar_trail_rider`] |
//! | U183 | 35 Rilgar's sea beasts (05): the swimmers and the lurker that takes Ratchet | level05 0x2d1688 | [`rilgar_sea_beast`] |
//! | U200 | 844 Rilgar's rising rocks (05): the rumble, the rise, the float, the fall, bubbles and dust | level05 0x30e7f8 | [`rilgar_rising_rock`] |
//! | U190 | 625 Rilgar's flame tanks (05): path driving, the turret's flames and their hits, the treads | level05 0x304320 | [`rilgar_flame_tank`] |
//! | U189 | 623 Rilgar's biters (05): the arena graph, the weaving chase, the wind-up sparks and bite, knockback and death | level05 0x301f48 | [`rilgar_biter`] |
//! | U180 | 810 Rilgar's rocking floats (05) | level05 0x30bdd8 | [`rilgar_rocker`] |
//! | U195 | 895 Rilgar's flaps (05) | level05 0x3166a0 | [`rilgar_flap`] |
//! | U283 | 467 / 472 Batalia's linked lifts (08) | level08 0x2ea398 | [`batalia_lift`] |
//! | U436 | 1577 Gemlik's watchers (13) | level13 0x30be60 | [`gemlik_watch`] |
//! | U158 | 484 (04): `DeleteMoby(self)` | level04 0x2ce060 | [`marker::update`] |
//! | U313 | 1206 Gaspar's hazard columns (09) | level09 0x305a28 | [`gaspar_hazard`] |
//! | U60 | 695 Novalis' floating pushables (01) | level01 0x2f8268 | [`floating_pushable`] |
//! | U256 | 1080 Umbris' sinking floats (07) | level07 0x311bc8 | [`umbris_sinker`] |
//! | U505 | 470 Kalebo's glowing beacons (16; the gold bolts' item glow) | level16 0x2c9480 | [`kalebo_glow`] |
//! | U341 | 1240 Orxon's spark fountains (10) | level10 0x2e5be0 | [`orxon_sparks`] |
//! | U263 | 104, 106, 1129 linked platforms (07, 13): `linked_mover`'s state machine at +0xa0 with a carry | level07 0x31aee0 | [`linked_mover::platform_update`] |
//! | U276 | 1128 the held linked mover (07): `linked_mover`'s machine, held shut or open by its mission's loaded byte | level07 0x31a9e8 | [`linked_mover::held_update`] |
//! | U222 | 1054 Blarg's split doors (06) and their half 1055 | level06 0x2fbfb0 | [`blarg_doors`] |
//! | U88 | 1504 wandering point lights (01, 06) | level01 0x30b618 | [`wandering_light`] |
//! | U376 | 1248 Pokitaru's rising gates (11) and their halves 1247 | level11 0x316320 | [`pokitaru_gate`] |
//! | U194 | 893 Rilgar's hinged hatches (05) | level05 0x316258 | [`rilgar_hatch`] |
//! | U191 | 855 Rilgar's water spouts (05): type-50 droplets, type-46 rings, the group voice | level05 0x3150f0 | [`rilgar_spout`] |
//! | U438 | 1805 Gemlik's breakable props (13) | level13 0x30cdb8 | [`gemlik_breakable`] |
//! | U570 | 1750 the save before the last boss (18): `MakeWholeSave` into the ending buffer once | level18 0x2fad08 | [`ending_save`] |
//! | U252 | 1041 Umbris' lobbing turrets (07) and their shot 882 | level07 0x30d4d8, 0x30bbc8 | [`umbris_lobber`] |
//! | U118 | 1212 the path ships (02, 09) and the big ones 1213 / 1973 (02, 09, 10): path, exhaust ([`engine_trail`], shared with [`batalia_fighter`]), glow quads | level02 0x2ec4b8, draw 0x2ec308 | [`path_ship`] |
//! | U155 | 434 Eudora's crank lifts (04): posed by a bolt crank, sink back unwinding it | level04 0x2c6bb8 | [`eudora_crank_lift`] |
//! | U154 | 432 / 1052 Eudora's crank followers (04): posed by a bolt crank, swap their own class (`class_swap`) | level04 0x2c6858 | [`eudora_crank_follower`] |
//! | U225 | 1066 Blarg's creature wakers (06): the cuboid, then one dormant member of the group thrown out every 20 ticks | level06 0x2fda30 (the wake 0x2e9d10) | [`blarg_waker`] |
//! | U140 | 899 Kerwan's path-mover lines (03) and the movers 898 they create: platforms 2.9 apart gliding along the path, carrying riders | level03 0x2db280, 0x2db198, 0x2db020 | [`kerwan_path_spawner`] |
//! | L03 825 | 825 Kerwan's turntable (03): turns at 0.48 rad/s, carrying its riders | level03 0x2d3e58 | [`kerwan_turntable`] |
//! | L03 997 | 997 Kerwan's riser (03): rises 4 units once its save bits are set | level03 0x2dca30 | [`kerwan_riser`] |
//! | L03 1548 | 1548 Kerwan's cutscene FX driver (03): the infobot's thrusters in scenes 4 / 10, smoke from actor 3 in scene 3 | level03 0x2e01d0 | [`kerwan_scene_fx`] |
//! | L03 914 | 914 Kerwan's talking bystander (03): talks before the Swingshot (a checkpoint after node 2), blown up by an explosion for skill point 5, then smokes | level03 0x2dbd90 | [`kerwan_bystander`] |
//! | L03 816, 1012 | 816 Kerwan's called platforms and 1012 its two-way shuttles (03): flown along splines, ridden with △ | level03 0x2d3198, 0x2dccc0 (the follower 0x2d3918) | [`kerwan_transport`] |
//! | L03 578, 627 | 578 Kerwan's blob layers (03) and the blobs 627 they drop: patrol a path, drop five blobs every 300 ticks; the blobs burst when stepped on or touched | level03 0x2c9eb8, 0x2cbea8 | [`kerwan_layer`] |
//! | U319 | 1885 the pod launchers (09, 13) and their pods 1886: lobbed pods bounce, rest and hatch a revived member of the launcher's group | level09 0x30ab80, 0x30b3b0, 0x30a778 | [`pod_launcher`] |
//! | U99, U101, U104 | 656, 675, 732 Aridia's animated props (02): clips and random rests; 732 hums | level02 0x2dca10, 0x2dd370, 0x2df1a8 | [`aridia_idlers`] |
//! | U105 | 733 Aridia's background cannon and its shell 1208 (02): turns, fires a smoking shell, its boom travels to the camera | level02 0x2df3d8, 0x2ebf20 | [`aridia_cannon`] |
//! | U106 | 735 / 736 Aridia's gate halves (02): swing to their target pose once triggered | level02 0x2df948 | [`aridia_gates`] |
//! | U110 | 762 Aridia's boulder (02): a blast bursts it, remembered by a global flag | level02 0x2e0720 | [`aridia_boulder`] |
//! | U113 | 792 Aridia's arm platform (02): rides an animated arm's joint, swings between its ends | level02 0x2e2228 | [`aridia_arm_platform`] |
//! | U120 | 1479 Aridia's fire vents (02): flame streams and spark bursts along their facing | level02 0x2ef020 | [`aridia_fire`] |
//! | U103 | 713 Aridia's launch tube (02): rises at the end Ratchet nears, carries him to the other end under a flying camera | level02 0x2ddc88 | [`aridia_tube`] |
//! | U97 | 651 Aridia's anti-grav lifts (02): ride their cylinder between its ends, come for Ratchet, glow while moving | level02 0x2dbd38, draw 0x2dc2c8 | [`aridia_lift`] |
//! | U134 | 455 the pod spawners (03, 08, 14) and their pods 545: once Ratchet is near, lobbed pods bounce, rest and hatch one of the spawner's placed creatures | level03 0x2bef68, 0x2c4718, 0x2bec60 | [`pod_spawner`] |
//! | U139 | 631 Kerwan's Blarg hover ship (a 574 riding it) and its flame stream 848: arrives along a path, patrols facing Ratchet, sprays flames from its turret, exits and switches paths | level03 0x2cca00, 0x2ce238, 0x2d47c0 | [`hover_ship`] |
//! | — | 1475 Kalebo III's board missile (16; created by the board weapon, item 0x24; the Devastator missile's trail) | level16 0x2a3f38 (0x2a3e30 the spawn) | [`board_missile`] |
//! | U509 | 933 Kalebo's floating mines (16): bob, spin, pulse; blown up by a hit or a touch, the placed ones back out of view | level16 0x2ddde0 | [`kalebo_mine`] |
//! | U521 | 1401 Kalebo's mine drones (16): wait in their cuboid, fly a path carrying a new race mine 933, drop it, fly back | level16 0x2e37a0 (0x2de298 the mine) | [`kalebo_mine_drone`] |
//! | U509 | 923 Oltanis's mine drones (14): level14's copy of 1401 | level14 0x2fe2a0 | [`kalebo_mine_drone`] |
//! | U363 | 1075 Pokitaru's boats: the path, the propellers and wake, the boarders' moving area, the lift (11) | level11 0x309ac0 | [`pokitaru_boat`] |
//! | U130 | 574 Kerwan's gun troopers (03), and the rocket 833 they fire | level03 0x2c6fd0, 0x2d43c8 | [`kerwan_trooper`] |
//! | U280 | 452 Batalia's runners (08) | level08 0x2e2df0 | [`batalia_runner`] |
//! | U359 | 318 Pokitaru teleporter pads (11; the sibling of the pads 1135, `classes::teleporter`), and their beam | level11 0x2f2518, 0x2f2d58 | [`pokitaru_teleporter`] |
//! | — | 787 the cave drips the ripple manager 751 spawns (01) | level01 0x2ffdc0 (spawner 0x2ffcd0) | [`drip`] |
//! | U50 | 613 Novalis's water currents (01) | level01 0x2f3120 | [`water_current`] |
//! | U129 | 573 Kerwan's charging creatures (03): wait, charge, bite, run a path, wait by a trooper | level03 0x2c5bb8 | [`kerwan_hound`] |
//! | U353, U350 | 114 Pokitaru's commando (11): talks, follows Ratchet through four phases, starts and rides the boats 1075, gives the O2 Mask; and the gate 65 he opens | level11 0x2d0fa8, 0x2cb810 | [`pokitaru_commando`] |
//! | U363 | 1157 Pokitaru's cutaway machine (11): its 24 slats 1207 open in a cutaway (fades, Ratchet held at the switch 830, the script camera's glide) | level11 0x30d800, 0x30dd18 | [`pokitaru_cutaway`] |
//! | U564 | 1422 the boss of Veldin's last arena (18): every state, its phases, damage, the arena camera record, the boss meter (HUD slot 6), the scenes, the death and the finale | level18 0x2f2bf0 (draw 0x2f7880) | [`veldin_boss`] |
//! | U557 | 644 the arena's cutaway camera (18), started by the boss | level18 0x2df608, 0x2dfaa0 | [`veldin_cutaway`] |
//! | U556 | 587 the arena's floating platforms (18): bob, carry, sink by the difficulty word 0x1623a8 | level18 0x2d82c0 | [`veldin_floater`] |
//! | U572 | 1906 the boss's hoppers (18), woken by the boss | level18 0x2fbb98 | [`veldin_hopper`] |
//! | U554, U555 | 583 the boss's pads and 586 the button / countdown (18) | level18 0x2d6600, 0x2d7b20 (countdown draw 0x2d8098) | [`veldin_pads`] |
//! | — | 564 / 624 / 628 / 983 / 1898 the boss's shots (18, created only by 1422 / 1906): shell, ring, aura, beam, flash | level18 0x2d5050, 0x2db460, 0x2dc260, 0x2e9e70, 0x2fb548 | [`veldin_shots`] |
//! | U123, U124 | 822 Kerwan's train (03): the locomotive pulling the cars 1210 behind its lead 845 (the flyer driver), the ride, the arrival, the infobot's release | level03 0x292e98, 0x293c78 | [`kerwan_train`] |
//! | U114, U115 | 786 Aridia's surfer (the Sonic Summoner) and 788 his agent (the Hoverboard; the shark-count race) (02) | level02 0x2e0dc0, 0x2e1950 | [`aridia_story`] |
//! | U142, U145 | 890 Helga (the Swingshot) and 909 the Heli-Pack giver (03) | level03 0x2da870, 0x2db558 | [`kerwan_story`] |
//! | U169, U170 | 1120 the Suck Cannon pickup and 1190 the Blarg informant (planet 6) (04) | level04 0x2e1a10, 0x2e3078 | [`eudora_story`] |
//! | U200, U201, U203 | 918 the race girl (flag 0), 919 the bouncer (planet 7), 925 the R.Y.N.O. salesman (05) | level05 0x316ab8, 0x317470, 0x3180a0 | [`rilgar_story`] |
//! | U233 | 1105 Blarg's scientist (the Grindboots) (06) | level06 0x301070 | [`blarg_story`] |
//! | U297, U298, U299 | 1130 the commando (planet 10), 1144 the deserter (planet 9), 1283 the turret host (the Metal Detector) (08) | level08 0x302ce8, 0x305270, 0x3065d8 | [`batalia_story`] |
//! | U321 | 1290 the Pilot's Helmet pickup (09; the code is in every overlay) | level01 0x30a6d0 | [`gaspar_story`] |
//! | U329, U350 | 18 the Magneboots pickup and 1326 the Nanotech seller (G-SAV-005) (10) | level10 0x298668, 0x2e8358 | [`orxon_story`] |
//! | U360, U363, U365 | 23 the O2 Mask prop, 90 the Thruster-Pack giver, 298 the Persuader giver (11) | level11 0x2cb668, 0x2d0710, 0x2f0e40 | [`pokitaru_story`] |
//! | U394, U398, U411 | 282 (flag 1, flag 0x61), 328 the Hydro-Pack giver, 1404 the scene triggers (12) | level12 0x2e6c48, 0x2eb570 | [`hoven_story`] |
//! | U440 | 1353 Gemlik's story director (the arrival, Qwark's ambush, planet 14) (13) | level13 0x30b628 | [`gemlik_story`] |
//! | U464, U469, U474 | 851 Qwark (the PDA), 924 the scrap merchant (planet 15), 1354 the Morph-o-Ray (14) | level14 0x2fba20, 0x2fefe0, 0x305758 | [`oltanis_story`] |
//! | U503, U506, U511 | 1388 the Bolt Grabber, 1419 the broadcast director (planet 17), 1469 the help director (15) | level15 0x2ea748, 0x2eb4c0 | [`quartu_story`] |
//! | U529 | 1377 the Map-O-Matic giver (flags 0x70 / 0x71) (16) | level16 0x2e3190 | [`kalebo_story`] |
//! | U561 | 1428 the fleet's item scene (flags 2 / 0x78) (17) | level17 0x2f1790 | [`fleet_story`] |
//! | U31 | 834 Veldin's Clank (flag 8, the trip to Novalis) (00) | level00 0x2d9dc8 | [`veldin_story`] |
//! | U24 | 530 Ratchet's ship on Veldin (00; hidden behind the scenes' own ship, its canopy glass) | level00 0x2d1e80 | [`veldin_ship`] |
//! | U36, U37 | 1440 Veldin's beam drones (00; fly in, fire a crackling beam, two hits) and 1471 their beam manager (the three beam slots, the strands, sparks and draw) | level00 0x2e0b88, 0x2e1df0 | [`veldin_beamer`] |
//! | U38 | 1545 Veldin's cutscene FX driver (00; the infobot's thrusters, scene 4's dust) | level00 0x2e3800 | [`veldin_scene_fx`] |
//! | U597 | 1434 / 1435 the pieces that turn over at the boss's checkpoint (18) | level18 0x2f7ad8 | [`veldin_turnover`] |
//! | U602 | 1799 the boss's jet in the scenes of Veldin's last level (18) | level18 0x2fad28 | [`veldin_scene_jet`] |
//! | U593 | 1392 the fields beside the Trespasser locks of Veldin's last level (18; a hum and pulsing bands until the lock is solved) | level18 0x2f1e68, draw 0x2f1fd8 | [`veldin_lock_field`] |
//! | U594 | 1402 the Hydrodisplacer pools of Veldin's last level (18; two heights, their groups carried, the surface meshes) | level18 0x2f2310, draws 0x2f2970 / 0x2f2620 / 0x2f27c8 | [`veldin_pool`] |
//! | U519 | 1443 (16) / 1890 (18) the energy fans (blades and rings until their switch group is thrown) | level18 0x2fae48, draw 0x2fb018 (level16 0x2e5708) | [`energy_fan`] |
//! | U599 | 1563 the cutscene effects of Veldin's last level (18; the seat glows, the jets and blasts of scene 4, the piece's glow and beam, the flash, the puffs) | level18 0x2f88e8, draws 0x2f9780 / 0x2f9a98 / 0x2f9c20 / 0x2f9eb8, the Morph-o-Ray beam 0x2c04b8 | [`veldin_finale_fx`] |
//! | U598 | 1454 the tanks of Veldin's last level (18; a path, a turret, shells 41 that may home, treads 331) | level18 0x2f7c40, 0x2a7220, 0x2ce7d0 | [`veldin_tank`] |
//! | U511 | 1356 the dropships (16, 18; the approach with the troopers, the drop, the exit, the homing shots 50) | level18 0x2f0920, 0x2a7b90 | [`dropship`] |
//! | U504 | 638 the hover troopers (16, 18; patrols, bursts of shots 49, the dropship's passengers) | level18 0x2dc918, 0x2a76e0 | [`drop_trooper`] |
//! | — | 1510 the burning wreck of a flyer 660 / gunship 688 (01; created only) | level01 0x30ba18, draw 0x30bfa8 | [`super::burning_wreck`] |
//! | U581 | 582 the rail chooser of Veldin's last level (18; the twice-laid rails, one way live at a time; two nanotech clusters) | level18 0x2d62e8 | [`veldin_rails`] |
//! | U248 | 436 Umbris' story director (the lair, planet 8, the trip to Batalia) (07) | level07 0x2f5ba0 | [`umbris_story`] |
//! | U118 | 1005 / 1016 the item scenes: the Trespasser (02), the Hydrodisplacer (06) | level02 0x2ea210 | [`aridia_story`] |
//! | U237 | 1109 Blarg's shuttle (the station's routes, the last ride's blast, the infobot hand-off, planet 5) (06) | level06 0x302578 | [`blarg_shuttle`] |
//! | U504 | 712 Oltanis's ferries (14): carry Ratchet between docks on △ | level14 0x2f0538 | [`oltanis_ferry`] |
//! | U502 | 685 Oltanis's ride cart (14): runs its paths under Ratchet with a moving wall | level14 0x2ee8d8 | [`oltanis_cart`] |
//! | U497 | 557 Oltanis's pull-target gliders (14): flyers carrying a Swingshot target that slow under Ratchet | level14 0x2e7bd8 | [`oltanis_glider`] |
//! | U501 | 684 Oltanis's arrival scene and lightning strikes (14) | level14 0x2ed280, 0x2edc18 | [`oltanis_lightning`] |
//! | U498 | 610 Oltanis's wind tunnels (14): push Ratchet along their yaw, blow the weather and the motes | level14 0x2eaf88, 0x2eb810 | [`oltanis_wind`] |
//! | U492 | 31 Oltanis's grenade drones (14) with their pieces 81 and grenades 1193 | level14 0x2b46e8, 0x2bac78, 0x300e00 | [`oltanis_drone`] |
//! | U520 | 1417 Oltanis's flying cars (14): fly a path carrying a Swingshot target, sink under Ratchet's weight | level14 0x306ee0 | [`oltanis_car`] |
//! | U506 | 908 Oltanis's fighters (14): 921's flight round a closed path | level14 0x2fc0f0 | [`oltanis_carrier`] |
//! | U512 | 1224 Oltanis's lightning cuboids (14): branching bolts along the cuboid that hurt Ratchet | level14 0x3015d0, 0x301fa8 | [`oltanis_bolt`] |
//! | U493 | 211 Oltanis's rail bots (14): ride a grind rail beside Ratchet and throw arcs ahead of him | level14 0x2d5f40 | [`oltanis_rail_bot`] |
//! | U490 | 28 the zapper bots (14, 15) with their arcs, and the pieces 325 / 403 they leave | level14 0x2b17d8, 0x2de670, 0x2dfd10 | [`oltanis_zapper`] |
//! | U514 | 1331 the arc slots' keeper (14, 15): three shared arcs the zappers 28 and turrets 211 throw | level14 0x3039e0, 0x3046f8 | [`oltanis_arcs`] |
//! | U507 | 921 Oltanis's missile carriers and 922 their missiles (14) | level14 0x2fcbb0, 0x2fdbb8 | [`oltanis_carrier`] |
//! | U496 | 386 Oltanis's rail arcs (14) | level14 0x2dee28 | [`oltanis_arc`] |
//! | U519 | 1416 Oltanis's pressure pads, 1559 its skill point and thrusters, 1395 the stair builder (14) | level14 0x306b78, 0x3087c0, 0x305d28 | [`oltanis_small`] |
//! | U499 | 643 Oltanis's floating mines, 1352 its risers (14) | level14 0x2ec810, 0x305408 | [`oltanis_small`] |
//! | U494 | 250 Oltanis's hatches, 309 its leaning floats (14) | level14 0x2d96e0, 0x2de1f8 | [`oltanis_small`] |
//! | U488 | 8 Oltanis's searchlight sentries (14): sweep a beam along a path, raise the pop-up turrets on finding Ratchet | level14 0x2ac618, 0x2aee28, 0x2ae260 | [`oltanis_sentry`] |
//! | U551, U560, U536, U556, U530 | 1394 Quartu's bomb droppers and their bombs 1257, 1560 its scene thrusters, 92 its gates, 1430 its dispenser and its piece 1428, 67 its sliding doors (15) | level15 0x2eabd8, 0x2e7f68, 0x2edb20, 0x2a36d0, 0x2ec030, 0x2ebd78, 0x29aff0 | [`quartu_small`] |
//! | U552 | 1408, 1409, 1565, 1567 Quartu's and 1405, 1406, 1407 the Fleet's rippling water meshes (15, 17) | level15 0x2eb0a0.., level17 0x2f08e0.. | [`wave_mesh`] |
//! | U535 | 78 the energy barriers (15, 17): a shimmering wall with drifting motes, shut down by a link or the mission | level15 0x2a2bf0, 0x2a3138 | [`barrier_field`] |
//! | U546 | 655 the electrified water (15, 17): shocks Ratchet in it, sparks along its lines, switched off for a countdown | level15 0x2d6810, 0x2d8710, 0x2d77c0 | [`water_shock`] |
//! | U528 | 44 Quartu's guards (15): watch, patrol, raise the alarm, shoot; fooled by the Hologuise; their shot 934 | level15 0x2979d8, 0x2e4380 | [`quartu_guard`] |
//! | U543 | 233 Quartu's flame drones (15): hover round Ratchet with a rider, spray flame streams; Clank's come back | level15 0x2c5630 | [`quartu_hover`] |
//! | U545 | 491 Quartu's jet robots (15, Giant Clank's): fly in, shoot volleys, lunge and swipe, jump between paths | level15 0x2cf3a8 | [`quartu_jet_bot`] |
//! | U539, U540 | 148, 255 Quartu's buildings and 154 its walls (15): knocked down by Giant Clank into burning bits and smoke | level15 0x2a7880, 0x2aa280 | [`quartu_building`] |
//! | U612 | 1382 the Fleet's crew (17): alert, salute the Hologuise, chase and punch inside their area | level17 0x2eeb68, 0x2f0448 | [`fleet_crew`] |
//! | U611 | 1380 the Fleet's lift pads (17): carry Ratchet between two heights, turning over for the Magneboots | level17 0x2ee2b0 | [`fleet_lift`] |
//! | U606 | 835 the Fleet's floating mines (17): bob on the water, blow up when hit or touched, with a splash | level17 0x2dc3f8 | [`fleet_small`] |
//! | U615 | 1470 the Fleet's help director (17): the barrier, Hydro-Pack and look hints; the dive and party skill points | level17 0x2f26d0 | [`help_fleet`] |
//! | U616 | 1562 the Fleet's scene thrusters (17): the infobot thrusters on actor 2 in scenes 0 and 2 | level17 0x2f2e78 | [`fleet_small`] |
//! | U614 | 1448 the Fleet's shuttle (17): Ratchet's ship flies him between its two landing spots on △ | level17 0x2f1940 | [`fleet_shuttle`] |
//! | U617 | 1772 the Fleet's space backdrop (17): the death height on and off the fleet, the glowing lanes and nebulae | level17 0x2f3848, 0x2f32f0 | [`fleet_sky`] |
//! | U480 | 1403 Gemlik's tracker, 1558 its scene thrusters (13) | level13 0x30bb70, 0x30bdf0 | [`gemlik_small`] |
//! | U477 | 1270 Gemlik's tipping lift (13) | level13 0x30a6d8 | [`gemlik_lift`] |
//! | U475 | 1262 Gemlik's stomper robots (13): the shockwave ring, the walk and swing, the knockback and death | level13 0x307e98, 0x3098d0 | [`gemlik_robot`] |
//! | U470 | 667 Gemlik's state relay, 674 / 677 / 680 its watched fields (13) | level13 0x2f8880, 0x2f8a78 | [`gemlik_relay`] |
//! | U467 | 404, 405 Gemlik's tower fields (13): the flat fields that hurt what touches them, fading once their tower is destroyed | level13 0x2ed4a8 | [`gemlik_field`] |
//! | U462 | 231 Gemlik's target switches (13): shot green in a group, the timed countdown, the blinking done state | level13 0x2e4448 | [`gemlik_switch`] |
//! | U459 | 170 Gemlik's base towers (13): hit from the ship above, the staged blow-up and debris; their hit proxy 1632 (created by code) | level13 0x2cefc0, 0x30c060 | [`gemlik_tower`] |
//! | U457 | 111 Gemlik's space fighters (13): the squadron's path flight, the attack runs on Ratchet's ship, the pickups they drop | level13 0x2c4428 | [`gemlik_fighter`] |
//! | U456 | 101 Gemlik's missile drones (13) and their missiles 1233 (created by code) | level13 0x2c25b8, 0x305a58 | [`gemlik_launcher`] |
//! | U449 | 21, 244 Gemlik's lifts (13): the ride with the script camera, the call to the other end | level13 0x2b39e0 | [`gemlik_lift`] |
//! | U448 | 6 Gemlik's state triggers (13): the cuboid that sets mobys' and groups' states on enter and leave | level13 0x2b0e80 | [`gemlik_trigger`] |
//! | U445 | 1557 Hoven's scene thrusters (12) | level12 0x3094a0 | [`hoven_story`] |
//! | U441 | 1345 Hoven's power beams (12): the crackling curtain held until its moby is destroyed, the mission | level12 0x3081b0, 0x308350 | [`hoven_beam`] |
//! | U440 | 1281 Hoven's mine dispensers (12): opening on the target and releasing their seeker mines | level12 0x307840 | [`hoven_dispenser`] |
//! | U438 | 1269 Hoven's seeker mines (12): the pop-up, the swerving roll at the target, the bursts, the blob shadow | level12 0x304e00 | [`hoven_mine`] |
//! | U436 | 1259 Hoven's arc posts (12): sliding pairs with four crackling bolts between them that hurt | level12 0x302f30, 0x3028c8 | [`hoven_arc`] |
//! | U432 | 384 Hoven's falls (12): the pulsing curtain raised and lowered by commands, its drips | level12 0x2ec720, 0x2ecac0 | [`hoven_fall`] |
//! | U430 | 336 Hoven's helicopters (12): circling their paths, blown up and back out of view, hidden during the carrier battle, the skill point | level12 0x2ebea8 | [`hoven_copter`] |
//! | U427 | 294 Hoven's gunners (12): the ambush, the path, the sidestepping approach, the aimed gun shot, the knockback and death, the crate smash | level12 0x2e72c0 | [`hoven_gunner`] |
//! | U424 | 240 Hoven's hover platforms (12): the path, bob and hover kinds, the tilt under Ratchet, the jets | level12 0x2e3ed8 | [`hoven_platform`] |
//! | U423 | 238 Hoven's burrowers (12): out of the ground, the sidestepping chase and bite, the wander, the jump-out, the Suck Cannon's hold | level12 0x2e1be0 | [`hoven_burrower`] |
//! | U369 | 1378 Orxon's air curtains: no air on their front (the O2 Mask's cue), their six-layer shimmer | level10 0x2e9028, 0x2e91b8 | [`orxon_airlock`] |
//! | U363 | 1229 Orxon's gun drones (10): the rise, the patrol and the shots at Ratchet or Clank's bots, the muzzle flash, the flight away; their shots 819 | level10 0x2e4a88, 0x2cc2e8 | [`orxon_drone`] |
//! | U349 | 702 Orxon's flame vents (10): the on / off bursts of flame lines (particle type 40) and the roar | level10 0x2c7a20 | [`orxon_flame`] |
//! | U347 | 351, 1301 Clank's teleport pads on Orxon (10): the jump between partners, the sparkles and rings, the camera's settle | level10 0x2be858 | [`orxon_pads`] |
//! | U353 | 947 Orxon's live wires (10): the charge along the path with its light and buzz, the sparks 1090 that hurt | level10 0x2d8d30, 0x2dd3d8 | [`orxon_wire`] |
//! | U357 | 1067, 1073, 794 Orxon's energy barriers (10), their strand field and their generators: the wiggling ribbons, the hum, switched off | level10 0x2da2c8, 0x2da690, 0x2dcc58 | [`orxon_curtain`] |
//! | U359, U343, U355, U348, U344, U370, U371, U373 | 1100, 1033, 1031, 353, 1117, 1421, 1424, 1555 Orxon's cracked walls, lift, pressure plates, sliding gate, sinking platforms, bridge, sliding block and scene thrusters (10) | level10 0x2dd650, 0x295a38, 0x2d92b8, 0x2befe8 | [`orxon_small`] |
//! | U352 | 939 Orxon's lava spouts (10) and their glowing rocks 938: thrown, bouncing, cooling, batted back, bursting | level10 0x2d8c00, 0x2d85c8 | [`orxon_lava`] |
//! | U313 | 1766 Gaspar's lava raft (09): waiting and rocking, ridden along its path with its paddle wheel and wake, turned back from the far cuboids | level09 0x309c08 | [`gaspar_raft`] |
//! | U308 | 1201 Gaspar's cannons (09): the mount, the stick aim, the two barrels, the seat camera and light, the crosshair; the bases 324 and the shells 1258 | level09 0x304c80, 0x2efe40, 0x307d68 | [`gaspar_cannon`] |
//! | U305 | 1150, 1151 Gaspar's path platforms (09): the path platform 726 with the riding state and sounds | level09 0x3033a0 | [`crate::moby_update::classes::path_platform`] |
//! | U302, U310 | 276, 1298, 1299 Gaspar's staged breakable rocks (09): each hit the next stage, the spray, chunks 320 / 321, the empty last stage 1300; the ring rocks 1285–1288 and their story flag | level09 0x2edc30, 0x2ef9b0, 0x309828 | [`gaspar_breakable`] |
//! | U301 | 263–267 Gaspar's meteor shower (09): the controller, the drifting rocks, the falling meteors and their bursts, debris; the dust puffs 417 | level09 0x2eb970, 0x2f0040 | [`gaspar_meteors`] |
//! | U286 | 671 Batalia's wreck flames (08): the swaying fire sheet, embers, sparks, the wandering light | level08 0x2f70a0, 0x2f5e58 | [`batalia_flame`] |
//! | U276, U278, U279 | 444, 462, 463 Batalia's gunships (08): intro loops, the attack runs at the turret, missiles, pods knocked off; their parts 441–451, 464, 465 (falling, smoke, splash) | level08 0x2e24e8, 0x2e8cd0, 0x2e9b70, 0x2e16c0 | [`batalia_gunship`] |
//! | U273 | 435 Batalia's bombers (crash through the walls) and the gunships' missiles (climb, then dive at the turret) | level08 0x2dd3c0 | [`batalia_bomber`] |
//! | U275 | 440 Batalia's anti-aircraft turret (08): the seat, the stick, the shells, the waves of gunships and their missiles, the radar HUD, the end with the host | level08 0x2e0328 | [`batalia_turret`] |
//! | U295 | 424 Batalia's ferries (08): the barge between three docks, the prompt, the ride with Ratchet held on its deck, the bob and the propeller | level08 0x2dbdb0 | [`batalia_ferry`] |
//! | U292 | 253 Batalia's tanks, their treads 331 and shells 425 (08): the wall break, the crank gate, the patrol, the turret, the bouncing shells, the wreck | level08 0x2d42d8, 0x2da3c0, 0x2dc9a8 | [`batalia_tank`] |
//! | U294 | 333 Batalia's grenadiers and their grenades 248 (08): the spots they hide in, the pop-up, the lob, the bounce, the knock-back, the smoke burst | level08 0x2dabf0, 0x2d23f0 | [`batalia_grenadier`] |
//! | U305 | 468 / 469, 1553, 1629, 1641 Batalia's small classes (08): the crank-turned bridges, the scene's thrusters, the falling streaks, the steam puffs | level08 0x2ea4c0, 0x3084d8, 0x3085f0, 0x308c90 | [`batalia_small`] |
//! | U317 | 1400 the weather emitter (08, 12, 14): the rain or snow around the camera, the splashes, the wind | level08 0x307cf0, level12 0x308be0, level14 0x306390 | [`weather`] |
//! | U254 | 1046, 1049 the Snagglebeast's shockwave rings and spit globs (07), with its tongue, beam, shimmer and fire-line draws | level07 0x30e1f8, 0x30e458 | [`umbris_beast_fx`] |
//! | U254 | 1106 Umbris' Snagglebeast (07): the arena walk between its platforms, the tongue grab, the stomp's rings, the spit, the beam and the fire sweep, the shimmer, the falls and the death | level07 0x314150 | [`umbris_beast`] |
//! | U267 | 1059 Umbris' swamp beasts (07): cruising the swamp, swallowing Ratchet in the water, wading and biting, knocks | level07 0x30f5f0 | [`umbris_swamp`] |
//! | U258 | 1126 Umbris' pop-up turrets and their shots 880 (07): the sweep, the bursts, down into the ground when hit | level07 0x31a250, 0x30b6a8 | [`umbris_turret`] |
//! | U272 | 1110, 1112, 871 Umbris' floating mines (07): lone mines, the chain mines on their leader's path, the leader that revives them | level07 0x319040, 0x3196e0, 0x30b000 | [`umbris_mines`] |
//! | U258 | 529, 1789, 1552, 1133, 1142, 1113–1116 Umbris' small classes (07): the parked ship, scene jets and dust, the swinging part, the ammo drop, the beast's walls | level07 0x2fbdb8, 0x31f900, 0x31eba8, 0x31b3f0, 0x31d2e0, 0x319f48 | [`umbris_small`] |
//! | U254 | 38, 1474 Umbris' path lifts (07): the ride from end to end, Ratchet held, the pause over him, the vanish and return | level07 0x2cd2b0, 0x2cdb28 | [`umbris_lift`] |
//! | U224 | 857 Clank's gadgetbots (06, 10), 302 their bubbles, 303 their markers (06, 10): follow, wait, attack and pad commands, the share-out of targets, the glow, the shattering bubble | level06 0x2f0040, 0x2f37d0, 0x2d85a0, 0x2d8738, 0x2d8c20 | [`blarg_gadgetbot`] |
//! | U233 | 1051 Blarg's mini-boss (06): the drop-in cutaway, the chase and slam, the crawler and trooper phases, the boss meter | level06 0x2f9a28 | [`blarg_boss`] |
//! | U238 | 1068 Blarg's fire-wave bots (06, 10): the rolling fire wall and its strip, the swing, the walk-out, knockback and death | level06 0x2fdbd0, 0x2ff680 | [`blarg_wave_bot`] |
//! | U232 | 1048 Blarg's troopers (06): group wakes, the surround, the jab, the leap, the guards, knockback and death | level06 0x2f7d78 | [`blarg_trooper`] |
//! | U236 | 1062 Blarg's windows, 1083 its breakable window, 1085..1088 the shards (06): overlay glass meshes, the break cutaway | level06 0x2fd088, 0x2ffcb0, 0x300b90 | [`blarg_glass`] |
//! | U227 | 1028 Blarg's bridge (06): Clank's cutaway, the deck sliding out, the rails flipping up | level06 0x2f55a0 | [`blarg_bridge`] |
//! | U242 | 1108 Blarg's escape (06): the held groups, the tube's ride, the camera run, the countdown, the blasts, the white-out | level06 0x301b90, 0x3021d8 | [`blarg_escape`] |
//! | U244 | 1118 Blarg's launch tube (06): the doors, the prompt, the ride up with the script camera and sparks | level06 0x304098 | [`blarg_launch_tube`] |
//! | U246 | 1302 Clank's gadgetbot pads (06, 10): the hologram arrow, the count in digits, the links | level06 0x307d30, 0x307578 | [`blarg_bot_pad`] |
//! | U228 | 1035 Blarg's laser gates (06): nine shared crackling beams, the hurt lines, the touch hint, the generators | level06 0x2f6470, 0x2f6dd0 | [`blarg_laser_gate`] |
//! | U245 | 1123 Blarg's energy barriers (06): four crackling beams, the hurt lines, the switch, the spark burst | level06 0x3049f8, 0x305170 | [`blarg_barrier`] |
//! | U221 | 55 Blarg's landing bay doors, 1551 its scene thrusters (06) | level06 0x2b4770, 0x309348 | [`blarg_small`] |
//! | U223 | 827 Blarg's crawlers (06, 10): surface crawling, pods, the entry leap, the bite, goo, lives, the Suck Cannon | level06 0x2e8678 | [`blarg_crawler`] |
//! | U405 | 1267 Hoven's turret mini-game (Gemlik's unlock, planet 13), its HUD 0x304218 and red screen 0x304d98 (12) | level12 0x303540 | [`hoven_turret`] |
//! | U424 | 69 Gemlik's ship (the base battle: the flight, the guns, the missiles, the vehicle record), its HUD 0x2b97f8 (the lock, the targets left, the gauge) (13) | level13 0x2bb068 | [`gemlik_ship`], [`gemlik_ship_hud`] |
//! | U434 | 388 Qwark's ship (the Gemlik base battle's boss: paths, phases, tractor beam, shield, taunts), its parts 389..401, shield 352, missile 82 and mine 83 (13) | level13 0x2eb098 | [`qwark_ship`], [`qwark_ship_parts`] |
//! | — | 458 the ridden turrets' shell (08, 12; created by code: the spawner 0x2ef2e8) | level12 0x2ef648 | [`turret_shell`] |
//! | U407 | 1274 Hoven's carrier (the turret game's target: 17 parts, five guns, the fall, scene 7) (12) | level12 0x3069d0 | [`hoven_carrier`] |
//! | — | 184 the gun shot (04, 12; created by code: the spawner 0x2d4150) | level12 0x2d4378 | [`gun_shot`] |
//! | — | 1009 the ships' laser shot (11, 13, 17; created by code: the spawner 0x3025f8, the search 0x302420) | level13 0x302780 | [`ship_laser`] |
//! | — | 295 the ships' homing missile (13, 17; created by code: the spawner 0x2e6a58) | level13 0x2e6c08 | [`ship_missile`] |
//! | — | 1034 its earlier copy (11, 13, 17; created by code: the spawner level11 0x309378) | level11 0x3094f8 | [`ship_missile`] |
//! | U384 | 1242 Pokitaru's jet (the convoy mission: the flight, the guns, the missiles, the edge bend, the ambush calls), its HUD 0x311d50 (the convoys left, the lock, the gauge) (11) | level11 0x313290 | [`pokitaru_jet`], [`pokitaru_jet_hud`] |
//! | U387 | 1264 Pokitaru's convoys (the jet mission's targets), their cars 1265 and the sludge 1524 they drop (11) | level11 0x3172c0, 0x3181f0, 0x31ab08 | [`pokitaru_convoy`] |
//! | U389 | 1319 Pokitaru's fighters (the convoys' escorts and the jet's ambushers: paths, attack runs, shots, pickups), their contrails 0x3192c8 (11) | level11 0x319838 | [`ship_fighter`] |
//! | U576 | 1843 the fleet's fighters (the same code, the floating pickups), their contrails 0x2f3b68 (17) | level17 0x2f40d8 | [`ship_fighter`] |
//! | U433 | 224 / 228 the floating ship pickups (13: 8 + 8; 17: dropped by 1843) | level13 0x2e1cc8 | [`ship_pickup_float`] |
//! | U568 | 1379 the fleet's ship (Gemlik's flight with the edge bend; the turrets' mission), its HUD 0x2eb9a8 (17) | level17 0x2ed018 | [`fleet_ship`], [`fleet_ship_hud`] |
//! | U563 | 347 the fleet's turrets (the ship mission's targets) and their bolt 1368 (17) | level17 0x2cb310, 0x2e8e08 | [`fleet_turret`] |
//! | — | 1218 / 1220 the ship pickups (missiles / health) and their parachute 1219 (11; created by code: the spawner 0x30f5a8) | level11 0x30f728, 0x310028 | [`ship_pickup`] |
//! | — | 1017 the fighters' laser shot (11, 13, 17; created by code: the spawner level11 0x308f48) | level11 0x309098 | [`fighter_shot`] |
//! | U397 | 326 Hoven's gun drones (ordinary and the turret game's attackers) and their shot 409 (12) | level12 0x2e9f68, 0x2ece00 | [`hoven_drone`] |
//! | — | 1371 the drones' rider (12, 15; created by code: 0x308670, knocked off by 0x308708) | level12 0x308918 | [`drone_rider`] |
//! | U538 | 1455 Kalebo III's hoverboard-race host (the Hologuise, skill point 0x13d422; the race itself: G-LVL-007) (16) | level16 0x2e6808 | [`kalebo_race`] |

use crate::moby_runtime::MobyId;
use crate::moby_update::services::World;

pub mod asteroid;
pub mod chain_link;
pub mod barricade;
pub mod tethered_platform;
pub mod explosive_tank;
pub mod fleet_door;
pub mod linked_cog;
pub mod quartu_belt;
pub mod rising_float;
pub mod extending_piece;
pub mod veldin_carrier;
pub mod hidden_prop;
pub mod bubble_vent;
pub mod oltanis_switchboard;
pub mod empty;
pub mod lamp;
pub mod loose_piece;
pub mod conveyor;
pub mod timed_switch;
pub mod linked_mover;
pub mod vent;
pub mod grind_mine;
pub mod rising_block;
pub mod bob_block;
pub mod blarg_shuttle;
pub mod oltanis_ferry;
pub mod oltanis_cart;
pub mod oltanis_glider;
pub mod oltanis_lightning;
pub mod oltanis_wind;
pub mod oltanis_drone;
pub mod oltanis_car;
pub mod oltanis_bolt;
pub mod oltanis_rail_bot;
pub mod oltanis_zapper;
pub mod oltanis_arcs;
pub mod oltanis_carrier;
pub mod oltanis_arc;
pub mod oltanis_small;
pub mod oltanis_sentry;
pub mod quartu_small;
pub mod wave_mesh;
pub mod barrier_field;
pub mod tesla_bolt;
pub mod water_shock;
pub mod quartu_guard;
pub mod quartu_hover;
pub mod quartu_jet_bot;
pub mod quartu_building;
pub mod fleet_crew;
pub mod fleet_sky;
pub mod fleet_shuttle;
pub mod fleet_small;
pub mod help_fleet;
pub mod fleet_lift;
pub mod gemlik_small;
pub mod gemlik_robot;
pub mod gemlik_relay;
pub mod gemlik_field;
pub mod gemlik_switch;
pub mod gemlik_tower;
pub mod gemlik_fighter;
pub mod gemlik_launcher;
pub mod gemlik_lift;
pub mod gemlik_trigger;
pub mod hoven_beam;
pub mod hoven_dispenser;
pub mod hoven_mine;
pub mod hoven_arc;
pub mod hoven_fall;
pub mod hoven_copter;
pub mod hoven_gunner;
pub mod hoven_platform;
pub mod hoven_burrower;
pub mod orxon_airlock;
pub mod orxon_drone;
pub mod orxon_flame;
pub mod orxon_pads;
pub mod orxon_wire;
pub mod orxon_curtain;
pub mod orxon_small;
pub mod orxon_lava;
pub mod gaspar_raft;
pub mod gaspar_cannon;
pub mod gaspar_breakable;
pub mod gaspar_meteors;
pub mod batalia_flame;
pub mod batalia_gunship;
pub mod batalia_bomber;
pub mod batalia_turret;
pub mod batalia_ferry;
pub mod batalia_tank;
pub mod batalia_grenadier;
pub mod batalia_small;
pub mod weather;
pub mod umbris_beast_fx;
pub mod umbris_beast;
pub mod umbris_swamp;
pub mod umbris_turret;
pub mod umbris_mines;
pub mod umbris_small;
pub mod umbris_lift;
pub mod blarg_gadgetbot;
pub mod blarg_boss;
pub mod blarg_wave_bot;
pub mod blarg_trooper;
pub mod blarg_glass;
pub mod blarg_bridge;
pub mod blarg_escape;
pub mod blarg_launch_tube;
pub mod blarg_bot_pad;
pub mod blarg_laser_gate;
pub mod blarg_barrier;
pub mod blarg_small;
pub mod blarg_crawler;
pub mod marker;
pub mod smoke_emitter;
pub mod hydro_pad;
pub mod trespasser_lock;
pub mod lock_doors;
pub mod slider;
pub mod help_director;
pub mod path_glider;
pub mod rail_car;
pub mod kalebo_traffic;
pub mod horny_toad;
pub mod hop_gunner;
pub mod laser_fence;
pub mod help_veldin;
pub mod help_aridia;
pub mod help_kerwan;
pub mod help_eudora;
pub mod help_rilgar;
pub mod help_blarg;
pub mod help_batalia;
pub mod help_gaspar;
pub mod help_orxon;
pub mod help_hoven;
pub mod help_gemlik;
pub mod hints;
pub mod rolling_mine;
pub mod pack_biter;
pub mod hover_zapper;
pub mod flying_biter;
pub mod buzz_bomb;
pub mod area_stalker;
pub mod wave_gate;
pub mod quartu_drone;
pub mod quartu_alarm;
pub mod kill_volume;
pub mod batalia_fighter;
pub mod swing_laser;
pub mod linked_rotator;
pub mod orxon_flyers;
pub mod orxon_brawler;
pub mod gemlik_turret;
pub mod orb_holder;
pub mod kalebo_barrier;
pub mod cuboid_slider;
pub mod swing_door;
pub mod chain_anchor;
pub mod slide_door;
pub mod carriers;
pub mod falling_platform;
pub mod light_fixture;
pub mod hologram_logo;
pub mod sweep_light;
pub mod spinner_float;
pub mod board_sparkle;
pub mod hoverboard;
pub mod board_boost;
pub mod kalebo_racer;
pub mod rilgar_racer;
pub mod popup_turret;
pub mod petal_door;
pub mod anim_idler;
pub mod linked_slider;
pub mod trip_block;
pub mod switched_mover;
pub mod pressure_pad;
pub mod flame_jet;
pub mod water_laser;
pub mod orxon_vent;
pub mod pokitaru_biter;
pub mod aridia_sandshark;
pub mod aridia_flamer;
pub mod pokitaru_thrower;
pub mod air_traffic;
pub mod kerwan_mover;
pub mod pokitaru_boat;
pub mod kerwan_trooper;
pub mod batalia_runner;
pub mod kalebo_belt;
pub mod fleet_laser;
pub mod quartu_critter;
pub mod veldin_diver;
pub mod eudora_path_rider;
pub mod eudora_flyers;
pub mod eudora_drifter;
pub mod eudora_ledge_block;
pub mod eudora_brawler;
pub mod eudora_gunner;
pub mod eudora_logger;
pub mod eudora_walker;
pub mod eudora_flock;
pub mod eudora_scene_fx;
pub mod pokitaru_wall;
pub mod pokitaru_skill;
pub mod pokitaru_button;
pub mod pokitaru_turner;
pub mod pokitaru_tilt;
pub mod pokitaru_unfold;
pub mod kalebo_gates;
pub mod kalebo_rail_switch;
pub mod kalebo_spinner;
pub mod kalebo_grind_skill;
pub mod kalebo_chicken_pad;
pub mod kalebo_lift;
pub mod kalebo_scene_jet;
pub mod kalebo_arena;
pub mod hover_car;
pub mod kalebo_trooper;
pub mod kalebo_grenade;
pub mod rilgar_small;
pub mod rilgar_fog_switch;
pub mod rilgar_watcher;
pub mod rilgar_breakable;
pub mod rilgar_lift_pad;
pub mod rilgar_trail_rider;
pub mod rilgar_sea_beast;
pub mod rilgar_rising_rock;
pub mod rilgar_flame_tank;
pub mod rilgar_biter;
pub mod rilgar_rocker;
pub mod rilgar_flap;
pub mod batalia_lift;
pub mod gemlik_watch;
pub mod gaspar_hazard;
pub mod floating_pushable;
pub mod umbris_sinker;
pub mod kalebo_glow;
pub mod orxon_sparks;
pub mod blarg_doors;
pub mod wandering_light;
pub mod pokitaru_gate;
pub mod rilgar_hatch;
pub mod rilgar_spout;
pub mod gemlik_breakable;
pub mod ending_save;
pub mod umbris_lobber;
pub mod engine_trail;
pub mod path_ship;
pub mod clank_section;
pub mod giant_pad;
pub mod blarg_clank_lift;
pub mod giant_shockwave;
pub mod giant_missile;
pub mod giant_beam;
pub mod quartu_giant_mission;
pub mod riding_floats;
pub mod eudora_crank_lift;
pub mod eudora_crank_follower;
pub mod blarg_waker;
pub mod kerwan_path_spawner;
pub mod kerwan_turntable;
pub mod kerwan_riser;
pub mod kerwan_scene_fx;
pub mod kerwan_bystander;
pub mod kerwan_transport;
pub mod kerwan_layer;
pub mod pod_launcher;
pub mod pod_spawner;
pub mod aridia_lift;
pub mod aridia_idlers;
pub mod aridia_cannon;
pub mod aridia_gates;
pub mod aridia_boulder;
pub mod aridia_arm_platform;
pub mod aridia_fire;
pub mod aridia_tube;
pub mod hover_ship;
pub mod kalebo_mine_drone;
pub mod kalebo_mine;
pub mod board_missile;
pub mod veldin_finale_fx;
pub mod veldin_lock_field;
pub mod veldin_pool;
pub mod veldin_rails;
pub mod drop_trooper;
pub mod dropship;
pub mod energy_fan;
pub mod veldin_scene_jet;
pub mod veldin_ship;
pub mod veldin_tank;
pub mod veldin_turnover;
pub mod veldin_scene_fx;
pub mod veldin_beamer;
pub mod pokitaru_teleporter;
pub mod drip;
pub mod water_current;
pub mod kerwan_hound;
pub mod pokitaru_commando;
pub mod pokitaru_cutaway;
pub mod veldin_boss;
pub mod veldin_cutaway;
pub mod veldin_floater;
pub mod veldin_hopper;
pub mod veldin_pads;
pub mod veldin_shots;
pub mod kerwan_train;
pub mod story_npc;
pub mod aridia_story;
pub mod kerwan_story;
pub mod eudora_story;
pub mod rilgar_story;
pub mod blarg_story;
pub mod batalia_story;
pub mod gaspar_story;
pub mod orxon_story;
pub mod pokitaru_story;
pub mod hoven_story;
pub mod gemlik_story;
pub mod oltanis_story;
pub mod quartu_story;
pub mod kalebo_story;
pub mod fleet_story;
pub mod veldin_story;
pub mod umbris_story;
pub mod hoven_turret;
pub mod gemlik_ship;
pub mod gemlik_ship_hud;
pub mod qwark_ship;
pub mod qwark_ship_parts;
pub mod turret_shell;
pub mod hoven_carrier;
pub mod gun_shot;
pub mod ship_laser;
pub mod ship_missile;
pub mod fighter_shot;
pub mod ship_pickup;
pub mod ship_pickup_float;
pub mod fleet_turret;
pub mod fleet_ship;
pub mod fleet_ship_hud;
pub mod ship_fighter;
pub mod pokitaru_convoy;
pub mod pokitaru_jet;
pub mod pokitaru_jet_hud;
pub mod hoven_drone;
pub mod drone_rider;
pub mod kalebo_race;

/// One unit's port.
#[derive(Clone, Copy, Debug)]
pub struct UnitPort {
    /// The census unit id (see docs/plan/class_census.md).
    pub unit: &'static str,
    /// The level whose overlay holds [`UnitPort::func`].
    pub level: u32,
    pub func: u32,
    /// The classes the reference level's table runs it for.
    pub classes: &'static [i16],
    pub update: fn(&mut World, MobyId),
    /// The classes whose joint points the port reads (`FUN_002645a8`): the loader fills their joint lists
    /// (`LevelPorts::needs_joint_lists`).
    pub joints: &'static [i16],
}

pub const PORTS: &[UnitPort] = &[
    UnitPort { unit: "U408", level: asteroid::REFERENCE_LEVEL, func: asteroid::UPDATE_FN, classes: &asteroid::CLASSES, update: asteroid::update, joints: &[] },
    UnitPort { unit: "U303", level: chain_link::REFERENCE_LEVEL, func: chain_link::UPDATE_FN, classes: &chain_link::CLASSES, update: chain_link::update, joints: &chain_link::JOINTS },
    UnitPort { unit: "U553", level: barricade::REFERENCE_LEVEL, func: barricade::UPDATE_FN, classes: &barricade::CLASSES, update: barricade::update, joints: &[] },
    UnitPort { unit: "U294", level: tethered_platform::REFERENCE_LEVEL, func: tethered_platform::UPDATE_FN, classes: &tethered_platform::CLASSES, update: tethered_platform::update, joints: &tethered_platform::JOINTS },
    UnitPort { unit: "U417", level: explosive_tank::REFERENCE_LEVEL, func: explosive_tank::UPDATE_FN, classes: &explosive_tank::CLASSES, update: explosive_tank::update, joints: &[] },
    UnitPort { unit: "U417 fireball", level: explosive_tank::REFERENCE_LEVEL, func: explosive_tank::FIREBALL_FN, classes: &explosive_tank::FIREBALL_CLASSES, update: explosive_tank::fireball_update, joints: &[] },
    UnitPort { unit: "U533", level: fleet_door::REFERENCE_LEVEL, func: fleet_door::UPDATE_FN, classes: &fleet_door::CLASSES, update: fleet_door::update, joints: &[] },
    UnitPort { unit: "U477", level: linked_cog::REFERENCE_LEVEL, func: linked_cog::UPDATE_FN, classes: &linked_cog::CLASSES, update: linked_cog::update, joints: &[] },
    UnitPort { unit: "U479", level: quartu_belt::REFERENCE_LEVEL, func: quartu_belt::UPDATE_FN, classes: &quartu_belt::CLASSES, update: quartu_belt::update, joints: &[] },
    UnitPort { unit: "U500", level: rising_float::REFERENCE_LEVEL, func: rising_float::UPDATE_FN, classes: &rising_float::CLASSES, update: rising_float::update, joints: &[] },
    UnitPort { unit: "U499", level: extending_piece::REFERENCE_LEVEL, func: extending_piece::UPDATE_FN, classes: &extending_piece::CLASSES, update: extending_piece::update, joints: &[] },
    UnitPort { unit: "U563", level: veldin_carrier::REFERENCE_LEVEL, func: veldin_carrier::UPDATE_FN, classes: &veldin_carrier::CLASSES, update: veldin_carrier::update, joints: &[] },
    UnitPort { unit: "U559", level: hidden_prop::REFERENCE_LEVEL, func: hidden_prop::UPDATE_FN, classes: &hidden_prop::CLASSES, update: hidden_prop::update, joints: &[] },
    UnitPort { unit: "U484", level: bubble_vent::REFERENCE_LEVEL, func: bubble_vent::UPDATE_FN, classes: &bubble_vent::CLASSES, update: bubble_vent::update, joints: &[] },
    UnitPort { unit: "U456", level: oltanis_switchboard::REFERENCE_LEVEL, func: oltanis_switchboard::UPDATE_FN, classes: &oltanis_switchboard::CLASSES, update: oltanis_switchboard::update, joints: &[] },
    // Level16 0x2cb600 is `DeleteMoby(self)`: the same effect as the markers' update (`marker`, U139), separate code.
    UnitPort { unit: "U493", level: 16, func: 0x2c_b600, classes: &[482], update: marker::update, joints: &[] },
    UnitPort { unit: "U99", level: empty::REFERENCE_LEVEL, func: empty::UPDATE_FN, classes: &empty::CLASSES, update: empty::update, joints: &[] },
    UnitPort { unit: "U27", level: lamp::REFERENCE_LEVEL, func: lamp::UPDATE_FN, classes: &lamp::CLASSES, update: lamp::update, joints: &[] },
    UnitPort { unit: "U268", level: loose_piece::REFERENCE_LEVEL, func: loose_piece::UPDATE_FN, classes: &loose_piece::CLASSES, update: loose_piece::update, joints: &loose_piece::CLASSES },
    UnitPort { unit: "U95", level: conveyor::REFERENCE_LEVEL, func: conveyor::UPDATE_FN, classes: &conveyor::CLASSES, update: conveyor::update, joints: &[] },
    UnitPort { unit: "U241", level: timed_switch::REFERENCE_LEVEL, func: timed_switch::UPDATE_FN, classes: &timed_switch::CLASSES, update: timed_switch::update, joints: &[] },
    UnitPort { unit: "U247", level: linked_mover::REFERENCE_LEVEL, func: linked_mover::UPDATE_FN, classes: &linked_mover::CLASSES, update: linked_mover::update, joints: &[] },
    UnitPort { unit: "U229", level: vent::REFERENCE_LEVEL, func: vent::UPDATE_FN, classes: &vent::CLASSES, update: vent::update, joints: &[] },
    UnitPort { unit: "U280", level: grind_mine::REFERENCE_LEVEL, func: grind_mine::UPDATE_FN, classes: &grind_mine::CLASSES, update: grind_mine::update, joints: &[] },
    UnitPort { unit: "U185", level: rising_block::REFERENCE_LEVEL, func: rising_block::UPDATE_FN, classes: &rising_block::CLASSES, update: rising_block::update, joints: &[] },
    UnitPort { unit: "U221", level: bob_block::REFERENCE_LEVEL, func: bob_block::UPDATE_FN, classes: &bob_block::CLASSES, update: bob_block::update, joints: &[] },
    UnitPort { unit: "U139", level: marker::REFERENCE_LEVEL, func: marker::UPDATE_FN, classes: &marker::CLASSES, update: marker::update, joints: &[] },
    UnitPort { unit: "U281", level: smoke_emitter::REFERENCE_LEVEL, func: smoke_emitter::UPDATE_FN, classes: &smoke_emitter::CLASSES, update: smoke_emitter::update, joints: &[] },
    UnitPort { unit: "U170", level: hydro_pad::REFERENCE_LEVEL, func: hydro_pad::UPDATE_FN, classes: &hydro_pad::CLASSES, update: hydro_pad::update, joints: &[] },
    UnitPort { unit: "U96", level: trespasser_lock::REFERENCE_LEVEL, func: trespasser_lock::UPDATE_FN, classes: &trespasser_lock::CLASSES, update: trespasser_lock::update, joints: &trespasser_lock::CLASSES },
    UnitPort { unit: "U107", level: lock_doors::REFERENCE_LEVEL, func: lock_doors::UPDATE_743, classes: &lock_doors::CLASSES_743, update: lock_doors::update_743, joints: &[] },
    UnitPort { unit: "U108", level: lock_doors::REFERENCE_LEVEL, func: lock_doors::UPDATE_744, classes: &lock_doors::CLASSES_744, update: lock_doors::update_744, joints: &[] },
    UnitPort { unit: "U365", level: lock_doors::LEVEL_1159, func: lock_doors::UPDATE_1159, classes: &lock_doors::CLASSES_1159, update: lock_doors::update_1159, joints: &[] },
    UnitPort { unit: "U204", level: slider::REFERENCE_LEVEL, func: slider::UPDATE_FN, classes: &slider::CLASSES, update: slider::update, joints: &[] },
    UnitPort { unit: "U82", level: help_director::REFERENCE_LEVEL, func: help_director::UPDATE_FN, classes: &help_director::CLASSES, update: help_director::update, joints: &[] },
    UnitPort { unit: "U36", level: path_glider::REFERENCE_LEVEL, func: path_glider::UPDATE_FN, classes: &path_glider::CLASSES, update: path_glider::update, joints: &[] },
    UnitPort { unit: "U495", level: rail_car::REFERENCE_LEVEL, func: rail_car::UPDATE_FN, classes: &rail_car::CLASSES, update: rail_car::update, joints: &[] },
    UnitPort { unit: "U523", level: kalebo_traffic::REFERENCE_LEVEL, func: kalebo_traffic::UPDATE_FN, classes: &kalebo_traffic::CLASSES, update: kalebo_traffic::update, joints: &[] },
    UnitPort { unit: "U25", level: horny_toad::REFERENCE_LEVEL, func: horny_toad::UPDATE_FN, classes: &horny_toad::CLASSES, update: horny_toad::update, joints: &horny_toad::JOINTS },
    UnitPort { unit: "U287", level: hop_gunner::REFERENCE_LEVEL, func: hop_gunner::UPDATE_FN, classes: &hop_gunner::CLASSES, update: hop_gunner::update, joints: &hop_gunner::JOINTS },
    UnitPort { unit: "U287 shot", level: hop_gunner::REFERENCE_LEVEL, func: hop_gunner::SHOT_FN, classes: &hop_gunner::SHOT_CLASSES, update: hop_gunner::shot_update, joints: &[] },
    UnitPort { unit: "U553 mine", level: rolling_mine::REFERENCE_LEVEL, func: rolling_mine::UPDATE_FN, classes: &rolling_mine::CLASSES, update: rolling_mine::update, joints: &[] },
    UnitPort { unit: "U301", level: pack_biter::REFERENCE_LEVEL, func: pack_biter::UPDATE_FN, classes: &pack_biter::CLASSES, update: pack_biter::update, joints: &[] },
    UnitPort { unit: "U268 252", level: hover_zapper::REFERENCE_LEVEL, func: hover_zapper::UPDATE_FN, classes: &hover_zapper::CLASSES, update: hover_zapper::update, joints: &hover_zapper::CLASSES },
    // Draw callbacks only (no class runs them: `Callback::UnitGlow` / `Callback::UnitQuads` payloads).
    UnitPort { unit: "U268 252 glow", level: hover_zapper::REFERENCE_LEVEL, func: hover_zapper::GLOW_FN, classes: &[], update: empty::update, joints: &[] },
    UnitPort { unit: "U268 252 arc", level: hover_zapper::REFERENCE_LEVEL, func: hover_zapper::ARC_FN, classes: &[], update: empty::update, joints: &[] },
    UnitPort { unit: "U407 63", level: flying_biter::REFERENCE_LEVEL, func: flying_biter::UPDATE_FN, classes: &flying_biter::CLASSES, update: flying_biter::update, joints: &[] },
    UnitPort { unit: "U300 52", level: buzz_bomb::REFERENCE_LEVEL, func: buzz_bomb::UPDATE_FN, classes: &buzz_bomb::CLASSES, update: buzz_bomb::update, joints: &[] },
    UnitPort { unit: "U521 1445", level: area_stalker::REFERENCE_LEVEL, func: area_stalker::UPDATE_FN, classes: &area_stalker::CLASSES, update: area_stalker::update, joints: &area_stalker::CLASSES },
    UnitPort { unit: "U426 1271", level: wave_gate::REFERENCE_LEVEL, func: wave_gate::UPDATE_FN, classes: &wave_gate::CLASSES, update: wave_gate::update, joints: &[] },
    UnitPort { unit: "U183", level: laser_fence::REFERENCE_LEVEL, func: laser_fence::UPDATE_FN, classes: &laser_fence::CLASSES, update: laser_fence::update, joints: &[] },
    UnitPort { unit: "U32 help", level: help_veldin::REFERENCE_LEVEL, func: help_veldin::UPDATE_FN, classes: &help_veldin::CLASSES, update: help_veldin::update, joints: &[] },
    UnitPort { unit: "U119 help", level: help_aridia::REFERENCE_LEVEL, func: help_aridia::UPDATE_FN, classes: &help_aridia::CLASSES, update: help_aridia::update, joints: &[] },
    UnitPort { unit: "U145 help", level: help_kerwan::REFERENCE_LEVEL, func: help_kerwan::UPDATE_FN, classes: &help_kerwan::CLASSES, update: help_kerwan::update, joints: &[] },
    UnitPort { unit: "U165 help", level: help_eudora::REFERENCE_LEVEL, func: help_eudora::UPDATE_FN, classes: &help_eudora::CLASSES, update: help_eudora::update, joints: &[] },
    UnitPort { unit: "U204 help", level: help_rilgar::REFERENCE_LEVEL, func: help_rilgar::UPDATE_FN, classes: &help_rilgar::CLASSES, update: help_rilgar::update, joints: &[] },
    UnitPort { unit: "U232 help", level: help_blarg::REFERENCE_LEVEL, func: help_blarg::UPDATE_FN, classes: &help_blarg::CLASSES, update: help_blarg::update, joints: &[] },
    UnitPort { unit: "U292 help", level: help_batalia::REFERENCE_LEVEL, func: help_batalia::UPDATE_FN, classes: &help_batalia::CLASSES, update: help_batalia::update, joints: &[] },
    UnitPort { unit: "U305 help", level: help_gaspar::REFERENCE_LEVEL, func: help_gaspar::UPDATE_FN, classes: &help_gaspar::CLASSES, update: help_gaspar::update, joints: &[] },
    UnitPort { unit: "U341 help", level: help_orxon::REFERENCE_LEVEL, func: help_orxon::UPDATE_FN, classes: &help_orxon::CLASSES, update: help_orxon::update, joints: &[] },
    UnitPort { unit: "U391 help", level: help_hoven::REFERENCE_LEVEL, func: help_hoven::UPDATE_FN, classes: &help_hoven::CLASSES, update: help_hoven::update, joints: &[] },
    UnitPort { unit: "U419 help", level: help_gemlik::REFERENCE_LEVEL, func: help_gemlik::UPDATE_FN, classes: &help_gemlik::CLASSES, update: help_gemlik::update, joints: &[] },
    UnitPort { unit: "U470 77", level: quartu_drone::REFERENCE_LEVEL, func: quartu_drone::UPDATE_FN, classes: &quartu_drone::CLASSES, update: quartu_drone::update, joints: &[] },
    UnitPort { unit: "U480 408", level: quartu_alarm::REFERENCE_LEVEL, func: quartu_alarm::UPDATE_FN, classes: &quartu_alarm::CLASSES, update: quartu_alarm::update, joints: &[] },
    UnitPort { unit: "U216 1039", level: kill_volume::REFERENCE_LEVEL, func: kill_volume::UPDATE_FN, classes: &kill_volume::CLASSES, update: kill_volume::update, joints: &[] },
    UnitPort { unit: "U274 438", level: batalia_fighter::REFERENCE_LEVEL, func: batalia_fighter::UPDATE_FN, classes: &batalia_fighter::CLASSES, update: batalia_fighter::update, joints: &[] },
    UnitPort { unit: "U474 123", level: swing_laser::REFERENCE_LEVEL, func: swing_laser::UPDATE_FN, classes: &swing_laser::CLASSES, update: swing_laser::update, joints: &[] },
    UnitPort { unit: "U411 127", level: linked_rotator::REFERENCE_LEVEL, func: linked_rotator::UPDATE_FN, classes: &linked_rotator::CLASSES, update: linked_rotator::update, joints: &[] },
    UnitPort { unit: "U335 1196", level: orxon_flyers::REFERENCE_LEVEL, func: orxon_flyers::SCOUT_FN, classes: &orxon_flyers::SCOUT_CLASSES, update: orxon_flyers::update_1196, joints: &[] },
    UnitPort { unit: "U336 1199", level: orxon_flyers::REFERENCE_LEVEL, func: orxon_flyers::SWOOP_FN, classes: &orxon_flyers::SWOOP_CLASSES, update: orxon_flyers::update_1199, joints: &orxon_flyers::SWOOP_CLASSES },
    UnitPort { unit: "U337 1202", level: orxon_brawler::REFERENCE_LEVEL, func: orxon_brawler::UPDATE_FN, classes: &orxon_brawler::CLASSES, update: orxon_brawler::update, joints: &orxon_brawler::CLASSES },
    UnitPort { unit: "U407 29", level: gemlik_turret::REFERENCE_LEVEL, func: gemlik_turret::UPDATE_FN, classes: &gemlik_turret::CLASSES, update: gemlik_turret::update, joints: &gemlik_turret::CLASSES },
    UnitPort { unit: "U407 36", level: gemlik_turret::REFERENCE_LEVEL, func: gemlik_turret::RIDER_FN, classes: &gemlik_turret::RIDER_CLASSES, update: gemlik_turret::rider_update, joints: &gemlik_turret::RIDER_CLASSES },
    UnitPort { unit: "U407 1238", level: gemlik_turret::REFERENCE_LEVEL, func: gemlik_turret::SHOT_FN, classes: &gemlik_turret::SHOT_CLASSES, update: gemlik_turret::shot_update, joints: &[] },
    UnitPort { unit: "U215 1038", level: orb_holder::REFERENCE_LEVEL, func: orb_holder::UPDATE_FN, classes: &orb_holder::CLASSES, update: orb_holder::update, joints: &[] },
    UnitPort { unit: "U215 orb", level: orb_holder::REFERENCE_LEVEL, func: orb_holder::ORB_FN, classes: &orb_holder::ORB_CLASSES, update: orb_holder::orb_update, joints: &[] },
    UnitPort { unit: "U503 552", level: kalebo_barrier::REFERENCE_LEVEL, func: kalebo_barrier::UPDATE_FN, classes: &kalebo_barrier::CLASSES, update: kalebo_barrier::post_update, joints: &[] },
    UnitPort { unit: "U502 546", level: kalebo_barrier::REFERENCE_LEVEL, func: kalebo_barrier::SWITCH_FN, classes: &kalebo_barrier::SWITCH_CLASSES, update: kalebo_barrier::switch_update, joints: &[] },
    UnitPort { unit: "U514 1387", level: kalebo_barrier::REFERENCE_LEVEL, func: kalebo_barrier::WALL_FN, classes: &kalebo_barrier::WALL_CLASSES, update: kalebo_barrier::wall_update, joints: &[] },
    UnitPort { unit: "U185 843", level: cuboid_slider::REFERENCE_LEVEL, func: cuboid_slider::UPDATE_FN, classes: &cuboid_slider::CLASSES, update: cuboid_slider::update, joints: &[] },
    UnitPort { unit: "U473 93", level: swing_door::REFERENCE_LEVEL, func: swing_door::UPDATE_FN, classes: &swing_door::CLASSES, update: swing_door::update, joints: &[] },
    UnitPort { unit: "U307 1172", level: chain_anchor::REFERENCE_LEVEL, func: chain_anchor::UPDATE_FN, classes: &chain_anchor::CLASSES, update: chain_anchor::update, joints: &[] },
    UnitPort { unit: "U477 196", level: slide_door::REFERENCE_LEVEL, func: slide_door::UPDATE_FN, classes: &slide_door::CLASSES, update: slide_door::update, joints: &[] },
    UnitPort { unit: "U102 707", level: carriers::REFERENCE_LEVEL_TURNTABLE, func: carriers::TURNTABLE_FN, classes: &carriers::TURNTABLE_CLASSES, update: carriers::turntable, joints: &[] },
    UnitPort { unit: "U102 734", level: carriers::REFERENCE_LEVEL_TURNTABLE, func: carriers::TURNTABLE_FN_734, classes: &carriers::TURNTABLE_CLASSES_734, update: carriers::turntable, joints: &[] },
    UnitPort { unit: "U126 1210", level: carriers::REFERENCE_LEVEL_JOINT, func: carriers::JOINT_FN, classes: &carriers::JOINT_CLASSES, update: carriers::joint_platform, joints: &carriers::JOINT_CLASSES },
    UnitPort { unit: "U179 812", level: carriers::REFERENCE_LEVEL_PINNED, func: carriers::PINNED_FN, classes: &carriers::PINNED_CLASSES, update: carriers::pinned_platform, joints: &[] },
    UnitPort { unit: "U565 1381", level: falling_platform::REFERENCE_LEVEL, func: falling_platform::UPDATE_FN, classes: &falling_platform::CLASSES, update: falling_platform::update, joints: &[] },
    UnitPort { unit: "U207 1511", level: light_fixture::REFERENCE_LEVEL, func: light_fixture::UPDATE_FN, classes: &light_fixture::CLASSES, update: light_fixture::update, joints: &[] },
    UnitPort { unit: "U472 1511", level: light_fixture::REFERENCE_LEVEL_14, func: light_fixture::UPDATE_FN_14, classes: &light_fixture::CLASSES, update: light_fixture::update, joints: &[] },
    UnitPort { unit: "U514 1143", level: hologram_logo::REFERENCE_LEVEL, func: hologram_logo::UPDATE_FN, classes: &hologram_logo::CLASSES, update: hologram_logo::update, joints: &hologram_logo::CLASSES },
    UnitPort { unit: "U180 823", level: sweep_light::REFERENCE_LEVEL, func: sweep_light::UPDATE_FN, classes: &sweep_light::CLASSES, update: sweep_light::update, joints: &sweep_light::CLASSES },
    UnitPort { unit: "U155 481", level: spinner_float::REFERENCE_LEVEL, func: spinner_float::UPDATE_FN, classes: &spinner_float::CLASSES, update: spinner_float::update, joints: &spinner_float::CLASSES },
    UnitPort { unit: "U203 1139", level: board_sparkle::REFERENCE_LEVEL, func: board_sparkle::UPDATE_FN, classes: &board_sparkle::CLASSES, update: board_sparkle::update, joints: &[] },
    UnitPort { unit: "U177 717", level: rilgar_racer::REFERENCE_LEVEL, func: rilgar_racer::UPDATE_FN, classes: &rilgar_racer::CLASSES, update: rilgar_racer::update, joints: &rilgar_racer::JOINTS },
    UnitPort { unit: "U171 133", level: board_boost::REFERENCE_LEVEL, func: board_boost::UPDATE_FN, classes: &board_boost::CLASSES, update: board_boost::update, joints: &[] },
    UnitPort { unit: "U503 556", level: kalebo_racer::REFERENCE_LEVEL, func: kalebo_racer::UPDATE_FN, classes: &kalebo_racer::CLASSES, update: kalebo_racer::update, joints: &kalebo_racer::CLASSES },
    UnitPort { unit: "U174 439", level: hoverboard::REFERENCE_LEVEL, func: hoverboard::UPDATE_FN, classes: &hoverboard::CLASSES, update: hoverboard::update, joints: &hoverboard::JOINTS },
    UnitPort { unit: "U440 30", level: popup_turret::REFERENCE_LEVEL, func: popup_turret::UPDATE_FN, classes: &popup_turret::CLASSES, update: popup_turret::update, joints: &[] },
    UnitPort { unit: "U440 681", level: popup_turret::REFERENCE_LEVEL, func: popup_turret::SHOT_FN, classes: &popup_turret::SHOT_CLASSES, update: popup_turret::shot_update, joints: &[] },
    UnitPort { unit: "U212 1021", level: petal_door::REFERENCE_LEVEL, func: petal_door::UPDATE_FN, classes: &petal_door::CLASSES, update: petal_door::update, joints: &[] },
    UnitPort { unit: "U390 339", level: anim_idler::REFERENCE_LEVEL, func: anim_idler::UPDATE_FN, classes: &anim_idler::CLASSES, update: anim_idler::update, joints: &[] },
    UnitPort { unit: "U248 1013", level: linked_slider::REFERENCE_LEVEL, func: linked_slider::UPDATE_FN, classes: &linked_slider::CLASSES, update: linked_slider::update, joints: &[] },
    UnitPort { unit: "U329 1015", level: trip_block::REFERENCE_LEVEL, func: trip_block::UPDATE_FN, classes: &trip_block::CLASSES, update: trip_block::update, joints: &[] },
    UnitPort { unit: "U162 1101", level: switched_mover::REFERENCE_LEVEL, func: switched_mover::UPDATE_FN, classes: &switched_mover::CLASSES, update: switched_mover::update, joints: &[] },
    UnitPort { unit: "U487 1209", level: pressure_pad::REFERENCE_LEVEL, func: pressure_pad::UPDATE_FN, classes: &pressure_pad::CLASSES, update: pressure_pad::update, joints: &pressure_pad::CLASSES },
    UnitPort { unit: "U211 911", level: flame_jet::REFERENCE_LEVEL, func: flame_jet::UPDATE_FN, classes: &flame_jet::CLASSES, update: flame_jet::update, joints: &[] },
    UnitPort { unit: "U542 669", level: water_laser::REFERENCE_LEVEL, func: water_laser::UPDATE_FN, classes: &water_laser::CLASSES, update: water_laser::update, joints: &[] },
    UnitPort { unit: "U349 1544", level: orxon_vent::REFERENCE_LEVEL, func: orxon_vent::UPDATE_FN, classes: &orxon_vent::CLASSES, update: orxon_vent::update, joints: &[] },
    UnitPort { unit: "U375 1246", level: pokitaru_biter::REFERENCE_LEVEL, func: pokitaru_biter::UPDATE_FN, classes: &pokitaru_biter::CLASSES, update: pokitaru_biter::update, joints: &[] },
    UnitPort { unit: "U95 580", level: aridia_sandshark::REFERENCE_LEVEL, func: aridia_sandshark::UPDATE_FN, classes: &aridia_sandshark::CLASSES, update: aridia_sandshark::update, joints: &[] },
    UnitPort { unit: "U101 668", level: aridia_sandshark::REFERENCE_LEVEL, func: aridia_sandshark::NEST_FN, classes: &aridia_sandshark::NEST_CLASSES, update: aridia_sandshark::nest_update, joints: &[] },
    UnitPort { unit: "U96 612", level: aridia_flamer::REFERENCE_LEVEL, func: aridia_flamer::UPDATE_FN, classes: &aridia_flamer::CLASSES, update: aridia_flamer::update, joints: &aridia_flamer::JOINTS },
    UnitPort { unit: "U373 1231", level: pokitaru_thrower::REFERENCE_LEVEL, func: pokitaru_thrower::UPDATE_FN, classes: &pokitaru_thrower::CLASSES, update: pokitaru_thrower::update, joints: &[] },
    UnitPort { unit: "U373 1297", level: pokitaru_thrower::REFERENCE_LEVEL, func: pokitaru_thrower::BALL_FN, classes: &pokitaru_thrower::BALL_CLASSES, update: pokitaru_thrower::ball_update, joints: &[] },
    UnitPort { unit: "U128", level: air_traffic::REFERENCE_LEVEL, func: air_traffic::UPDATE_FN, classes: &air_traffic::CLASSES, update: air_traffic::update, joints: &air_traffic::JOINTS },
    UnitPort { unit: "U128 235", level: air_traffic::REFERENCE_LEVEL, func: air_traffic::TRAIL_FN, classes: &air_traffic::TRAIL_CLASSES, update: air_traffic::trail_update, joints: &[] },
    UnitPort { unit: "U126", level: kerwan_mover::REFERENCE_LEVEL, func: kerwan_mover::UPDATE_FN, classes: &kerwan_mover::CLASSES, update: kerwan_mover::update, joints: &[] },
    UnitPort { unit: "U363 1075", level: pokitaru_boat::REFERENCE_LEVEL, func: pokitaru_boat::UPDATE_FN, classes: &pokitaru_boat::CLASSES, update: pokitaru_boat::update, joints: &pokitaru_boat::JOINTS },
    UnitPort { unit: "U130 574", level: kerwan_trooper::REFERENCE_LEVEL, func: kerwan_trooper::UPDATE_FN, classes: &kerwan_trooper::CLASSES, update: kerwan_trooper::update, joints: &kerwan_trooper::JOINTS },
    UnitPort { unit: "U130 833", level: kerwan_trooper::REFERENCE_LEVEL, func: kerwan_trooper::ROCKET_FN, classes: &kerwan_trooper::ROCKET_CLASSES, update: kerwan_trooper::rocket_update, joints: &[] },
    UnitPort { unit: "U280 452", level: batalia_runner::REFERENCE_LEVEL, func: batalia_runner::UPDATE_FN, classes: &batalia_runner::CLASSES, update: batalia_runner::update, joints: &batalia_runner::JOINTS },
    UnitPort { unit: "U506 471", level: kalebo_belt::REFERENCE_LEVEL, func: kalebo_belt::UPDATE_FN, classes: &kalebo_belt::CLASSES, update: kalebo_belt::update, joints: &[] },
    // Draw callback only (the belt's second quad layer: `Callback::UnitQuads` payload).
    UnitPort { unit: "U506 471 layer", level: kalebo_belt::REFERENCE_LEVEL, func: kalebo_belt::DRAW_FN, classes: &[], update: empty::update, joints: &[] },
    UnitPort { unit: "U541 99", level: fleet_laser::REFERENCE_LEVEL, func: fleet_laser::UPDATE_FN, classes: &fleet_laser::CLASSES, update: fleet_laser::update, joints: &[] },
    UnitPort { unit: "U485 221", level: quartu_critter::REFERENCE_LEVEL, func: quartu_critter::UPDATE_FN, classes: &quartu_critter::CLASSES, update: quartu_critter::update, joints: &quartu_critter::CLASSES },
    UnitPort { unit: "U567 1355", level: veldin_diver::REFERENCE_LEVEL, func: veldin_diver::UPDATE_FN, classes: &veldin_diver::CLASSES, update: veldin_diver::update, joints: &[] },
    // Draw callback only (the group's trails: `Callback::UnitQuads` payload; the glows are `UnitGlow` of the row above).
    UnitPort { unit: "U567 1355 trail", level: veldin_diver::REFERENCE_LEVEL, func: veldin_diver::TRAIL_FN, classes: &[], update: empty::update, joints: &[] },    // Draw callback only (the teleporter pads 1135's beam `0x3094f0`: `Callback::UnitQuads` payload; the pads are the
    UnitPort { unit: "U162 617", level: eudora_path_rider::REFERENCE_LEVEL, func: eudora_path_rider::UPDATE_FN, classes: &eudora_path_rider::CLASSES, update: eudora_path_rider::update, joints: &[] },
    UnitPort { unit: "U167 466", level: eudora_flyers::REFERENCE_LEVEL, func: eudora_flyers::UPDATE_FN, classes: &eudora_flyers::CLASSES, update: eudora_flyers::update, joints: &eudora_flyers::JOINTS },
    UnitPort { unit: "U163 340", level: eudora_brawler::REFERENCE_LEVEL, func: eudora_brawler::UPDATE_FN, classes: &eudora_brawler::CLASSES, update: eudora_brawler::update, joints: &eudora_brawler::CLASSES },
    UnitPort { unit: "U164 427", level: eudora_gunner::REFERENCE_LEVEL, func: eudora_gunner::UPDATE_FN, classes: &eudora_gunner::CLASSES, update: eudora_gunner::update, joints: &eudora_gunner::CLASSES },
    UnitPort { unit: "U162 217", level: eudora_logger::REFERENCE_LEVEL, func: eudora_logger::UPDATE_FN, classes: &eudora_logger::CLASSES, update: eudora_logger::update, joints: &eudora_logger::CLASSES },
    UnitPort { unit: "U170 563", level: eudora_walker::REFERENCE_LEVEL, func: eudora_walker::UPDATE_FN, classes: &eudora_walker::CLASSES, update: eudora_walker::update, joints: &eudora_walker::CLASSES },
    UnitPort { unit: "U170 1516 chip", level: eudora_walker::REFERENCE_LEVEL, func: eudora_walker::CHIP_FN, classes: &eudora_walker::CHIP_CLASSES, update: eudora_walker::chip_update, joints: &[] },
    UnitPort { unit: "U160 86", level: eudora_flock::REFERENCE_LEVEL, func: eudora_flock::UPDATE_FN, classes: &eudora_flock::CLASSES, update: eudora_flock::update, joints: &[] },
    UnitPort { unit: "U172 584", level: eudora_ledge_block::REFERENCE_LEVEL, func: eudora_ledge_block::UPDATE_FN, classes: &eudora_ledge_block::CLASSES, update: eudora_ledge_block::update, joints: &[] },
    UnitPort { unit: "U174 642", level: eudora_drifter::REFERENCE_LEVEL, func: eudora_drifter::UPDATE_FN, classes: &eudora_drifter::CLASSES, update: eudora_drifter::update, joints: &[] },
    UnitPort { unit: "U179 1549", level: eudora_scene_fx::REFERENCE_LEVEL, func: eudora_scene_fx::UPDATE_FN, classes: &eudora_scene_fx::CLASSES, update: eudora_scene_fx::update, joints: &[] },
    UnitPort { unit: "U375 361", level: pokitaru_wall::REFERENCE_LEVEL, func: pokitaru_wall::UPDATE_FN, classes: &pokitaru_wall::CLASSES, update: pokitaru_wall::update, joints: &[] },
    UnitPort { unit: "U401 1350", level: pokitaru_skill::REFERENCE_LEVEL, func: pokitaru_skill::UPDATE_FN, classes: &pokitaru_skill::CLASSES, update: pokitaru_skill::update, joints: &[] },
    UnitPort { unit: "U389 1179", level: pokitaru_button::REFERENCE_LEVEL, func: pokitaru_button::UPDATE_FN, classes: &pokitaru_button::CLASSES, update: pokitaru_button::update, joints: &[] },
    UnitPort { unit: "U390 1180", level: pokitaru_turner::REFERENCE_LEVEL, func: pokitaru_turner::UPDATE_FN, classes: &pokitaru_turner::CLASSES, update: pokitaru_turner::update, joints: &[] },
    UnitPort { unit: "U388 1178", level: pokitaru_tilt::REFERENCE_LEVEL, func: pokitaru_tilt::UPDATE_FN, classes: &pokitaru_tilt::CLASSES, update: pokitaru_tilt::update, joints: &[] },
    UnitPort { unit: "U384 1156", level: pokitaru_unfold::REFERENCE_LEVEL, func: pokitaru_unfold::UPDATE_FN, classes: &pokitaru_unfold::CLASSES, update: pokitaru_unfold::update, joints: &[] },
    UnitPort { unit: "U559 1439", level: kalebo_gates::REFERENCE_LEVEL, func: kalebo_gates::UPDATE_FN, classes: &kalebo_gates::CLASSES, update: kalebo_gates::update, joints: &[] },
    UnitPort { unit: "U561 1442", level: kalebo_rail_switch::REFERENCE_LEVEL, func: kalebo_rail_switch::UPDATE_FN, classes: &kalebo_rail_switch::CLASSES, update: kalebo_rail_switch::update, joints: &[] },
    UnitPort { unit: "U569 1891", level: kalebo_spinner::REFERENCE_LEVEL, func: kalebo_spinner::UPDATE_FN, classes: &kalebo_spinner::CLASSES, update: kalebo_spinner::update, joints: &[] },
    UnitPort { unit: "U565 1561", level: kalebo_grind_skill::REFERENCE_LEVEL, func: kalebo_grind_skill::UPDATE_FN, classes: &kalebo_grind_skill::CLASSES, update: kalebo_grind_skill::update, joints: &[] },
    UnitPort { unit: "U570 1923", level: kalebo_chicken_pad::REFERENCE_LEVEL, func: kalebo_chicken_pad::UPDATE_FN, classes: &kalebo_chicken_pad::CLASSES, update: kalebo_chicken_pad::update, joints: &[] },
    UnitPort { unit: "U568 1826", level: kalebo_lift::REFERENCE_LEVEL, func: kalebo_lift::UPDATE_FN, classes: &kalebo_lift::CLASSES, update: kalebo_lift::update, joints: &[] },
    UnitPort { unit: "U567 1812", level: kalebo_scene_jet::REFERENCE_LEVEL, func: kalebo_scene_jet::UPDATE_FN, classes: &kalebo_scene_jet::CLASSES, update: kalebo_scene_jet::update, joints: &[] },
    UnitPort { unit: "U552 654", level: kalebo_arena::REFERENCE_LEVEL, func: kalebo_arena::UPDATE_FN, classes: &kalebo_arena::CLASSES, update: kalebo_arena::update, joints: &[] },
    UnitPort { unit: "U558 1410", level: hover_car::KALEBO.level, func: hover_car::KALEBO.update, classes: &hover_car::KALEBO_CLASSES, update: hover_car::update_kalebo, joints: &[] },
    UnitPort { unit: "U215 998", level: hover_car::RILGAR.level, func: hover_car::RILGAR.update, classes: &hover_car::RILGAR_CLASSES, update: hover_car::update_rilgar, joints: &[] },
    UnitPort { unit: "U560 1441", level: kalebo_gates::REFERENCE_LEVEL, func: kalebo_gates::UPDATE_FN_1441, classes: &kalebo_gates::CLASSES_1441, update: kalebo_gates::update_1441, joints: &[] },
    UnitPort { unit: "U568 1826 glow", level: kalebo_lift::REFERENCE_LEVEL, func: kalebo_lift::GLOW_FN, classes: &[], update: empty::update, joints: &[] },
    UnitPort { unit: "U545 541", level: kalebo_trooper::REFERENCE_LEVEL, func: kalebo_trooper::UPDATE_FN, classes: &kalebo_trooper::CLASSES, update: kalebo_trooper::update, joints: &kalebo_trooper::JOINTS },
    UnitPort { unit: "U545 541 glow", level: kalebo_trooper::REFERENCE_LEVEL, func: kalebo_trooper::GLOW_FN, classes: &[], update: empty::update, joints: &[] },
    UnitPort { unit: "U545 281 grenade", level: kalebo_grenade::REFERENCE_LEVEL, func: kalebo_grenade::UPDATE_FN, classes: &kalebo_grenade::CLASSES, update: kalebo_grenade::update, joints: &[] },
    UnitPort { unit: "U216 1099", level: rilgar_small::REFERENCE_LEVEL, func: rilgar_small::HIDDEN_FN, classes: &rilgar_small::HIDDEN_CLASSES, update: rilgar_small::hidden_update, joints: &[] },
    UnitPort { unit: "U214 984", level: rilgar_small::REFERENCE_LEVEL, func: rilgar_small::BOB_FN, classes: &rilgar_small::BOB_CLASSES, update: rilgar_small::bob_update, joints: &[] },
    UnitPort { unit: "U220 1550", level: rilgar_small::REFERENCE_LEVEL, func: rilgar_small::SCENE_FX_FN, classes: &rilgar_small::SCENE_FX_CLASSES, update: rilgar_small::scene_fx_update, joints: &[] },
    UnitPort { unit: "U198 841", level: rilgar_fog_switch::REFERENCE_LEVEL, func: rilgar_fog_switch::UPDATE_FN, classes: &rilgar_fog_switch::CLASSES, update: rilgar_fog_switch::update, joints: &[] },
    UnitPort { unit: "U210 920", level: rilgar_watcher::REFERENCE_LEVEL, func: rilgar_watcher::UPDATE_FN, classes: &rilgar_watcher::CLASSES, update: rilgar_watcher::update, joints: &rilgar_watcher::CLASSES },
    UnitPort { unit: "U201 846", level: rilgar_breakable::REFERENCE_LEVEL, func: rilgar_breakable::UPDATE_FN, classes: &rilgar_breakable::CLASSES, update: rilgar_breakable::update, joints: &[] },
    UnitPort { unit: "U204 877", level: rilgar_lift_pad::REFERENCE_LEVEL, func: rilgar_lift_pad::UPDATE_FN, classes: &rilgar_lift_pad::CLASSES, update: rilgar_lift_pad::update, joints: &[] },
    UnitPort { unit: "U184 79", level: rilgar_trail_rider::REFERENCE_LEVEL, func: rilgar_trail_rider::UPDATE_FN, classes: &rilgar_trail_rider::CLASSES, update: rilgar_trail_rider::update, joints: &rilgar_trail_rider::CLASSES },
    UnitPort { unit: "U184 79 trails", level: rilgar_trail_rider::REFERENCE_LEVEL, func: rilgar_trail_rider::TRAIL_FN, classes: &[], update: empty::update, joints: &[] },
    UnitPort { unit: "U184 79 glows", level: rilgar_trail_rider::REFERENCE_LEVEL, func: rilgar_trail_rider::GLOW_FN, classes: &[], update: empty::update, joints: &[] },
    UnitPort { unit: "U183 35", level: rilgar_sea_beast::REFERENCE_LEVEL, func: rilgar_sea_beast::UPDATE_FN, classes: &rilgar_sea_beast::CLASSES, update: rilgar_sea_beast::update, joints: &[] },
    UnitPort { unit: "U200 844", level: rilgar_rising_rock::REFERENCE_LEVEL, func: rilgar_rising_rock::UPDATE_FN, classes: &rilgar_rising_rock::CLASSES, update: rilgar_rising_rock::update, joints: &[] },
    UnitPort { unit: "U190 625", level: rilgar_flame_tank::REFERENCE_LEVEL, func: rilgar_flame_tank::UPDATE_FN, classes: &rilgar_flame_tank::CLASSES, update: rilgar_flame_tank::update, joints: &rilgar_flame_tank::CLASSES },
    UnitPort { unit: "U189 623", level: rilgar_biter::REFERENCE_LEVEL, func: rilgar_biter::UPDATE_FN, classes: &rilgar_biter::CLASSES, update: rilgar_biter::update, joints: &rilgar_biter::CLASSES },
    UnitPort { unit: "U180 810", level: rilgar_rocker::REFERENCE_LEVEL, func: rilgar_rocker::UPDATE_FN, classes: &rilgar_rocker::CLASSES, update: rilgar_rocker::update, joints: &[] },
    UnitPort { unit: "U195 895", level: rilgar_flap::REFERENCE_LEVEL, func: rilgar_flap::UPDATE_FN, classes: &rilgar_flap::CLASSES, update: rilgar_flap::update, joints: &[] },
    UnitPort { unit: "U283 467", level: batalia_lift::REFERENCE_LEVEL, func: batalia_lift::UPDATE_FN, classes: &batalia_lift::CLASSES, update: batalia_lift::update, joints: &[] },
    UnitPort { unit: "U436 1577", level: gemlik_watch::REFERENCE_LEVEL, func: gemlik_watch::UPDATE_FN, classes: &gemlik_watch::CLASSES, update: gemlik_watch::update, joints: &[] },
    // Level04 0x2ce060 is `DeleteMoby(self)`: the markers' effect (`marker`, U139), separate code.
    UnitPort { unit: "U158 484", level: 4, func: 0x2c_e060, classes: &[484], update: marker::update, joints: &[] },
    UnitPort { unit: "U313 1206", level: gaspar_hazard::REFERENCE_LEVEL, func: gaspar_hazard::UPDATE_FN, classes: &gaspar_hazard::CLASSES, update: gaspar_hazard::update, joints: &[] },
    UnitPort { unit: "U60 695", level: floating_pushable::REFERENCE_LEVEL, func: floating_pushable::UPDATE_FN, classes: &floating_pushable::CLASSES, update: floating_pushable::update, joints: &[] },
    UnitPort { unit: "U256 1080", level: umbris_sinker::REFERENCE_LEVEL, func: umbris_sinker::UPDATE_FN, classes: &umbris_sinker::CLASSES, update: umbris_sinker::update, joints: &[] },
    UnitPort { unit: "U505 470", level: kalebo_glow::REFERENCE_LEVEL, func: kalebo_glow::UPDATE_FN, classes: &kalebo_glow::CLASSES, update: kalebo_glow::update, joints: &[] },
    UnitPort { unit: "U341 1240", level: orxon_sparks::REFERENCE_LEVEL, func: orxon_sparks::UPDATE_FN, classes: &orxon_sparks::CLASSES, update: orxon_sparks::update, joints: &[] },
    UnitPort { unit: "U263 104", level: linked_mover::REFERENCE_LEVEL, func: linked_mover::PLATFORM_FN, classes: &linked_mover::PLATFORM_CLASSES, update: linked_mover::platform_update, joints: &[] },
    UnitPort { unit: "U276 1128", level: linked_mover::REFERENCE_LEVEL, func: linked_mover::HELD_FN, classes: &linked_mover::HELD_CLASSES, update: linked_mover::held_update, joints: &[] },
    UnitPort { unit: "U222 1054", level: blarg_doors::REFERENCE_LEVEL, func: blarg_doors::UPDATE_FN, classes: &blarg_doors::CLASSES, update: blarg_doors::update, joints: &[] },
    UnitPort { unit: "U88 1504", level: wandering_light::REFERENCE_LEVEL, func: wandering_light::UPDATE_FN, classes: &wandering_light::CLASSES, update: wandering_light::update, joints: &[] },
    UnitPort { unit: "U376 1248", level: pokitaru_gate::REFERENCE_LEVEL, func: pokitaru_gate::UPDATE_FN, classes: &pokitaru_gate::CLASSES, update: pokitaru_gate::update, joints: &[] },
    UnitPort { unit: "U194 893", level: rilgar_hatch::REFERENCE_LEVEL, func: rilgar_hatch::UPDATE_FN, classes: &rilgar_hatch::CLASSES, update: rilgar_hatch::update, joints: &rilgar_hatch::CLASSES },
    UnitPort { unit: "U191 855", level: rilgar_spout::REFERENCE_LEVEL, func: rilgar_spout::UPDATE_FN, classes: &rilgar_spout::CLASSES, update: rilgar_spout::update, joints: &[] },
    UnitPort { unit: "U438 1805", level: gemlik_breakable::REFERENCE_LEVEL, func: gemlik_breakable::UPDATE_FN, classes: &gemlik_breakable::CLASSES, update: gemlik_breakable::update, joints: &[] },
    UnitPort { unit: "U570 1750", level: ending_save::REFERENCE_LEVEL, func: ending_save::UPDATE_FN, classes: &ending_save::CLASSES, update: ending_save::update, joints: &[] },
    UnitPort { unit: "U252 1041", level: umbris_lobber::REFERENCE_LEVEL, func: umbris_lobber::UPDATE_FN, classes: &umbris_lobber::CLASSES, update: umbris_lobber::update, joints: &umbris_lobber::CLASSES },
    UnitPort { unit: "U252 882", level: umbris_lobber::REFERENCE_LEVEL, func: umbris_lobber::SHOT_FN, classes: &umbris_lobber::SHOT_CLASSES, update: umbris_lobber::shot_update, joints: &[] },
    UnitPort { unit: "U118 1212", level: path_ship::REFERENCE_LEVEL, func: path_ship::UPDATE_FN, classes: &path_ship::CLASSES, update: path_ship::update, joints: &path_ship::CLASSES },
    UnitPort { unit: "U118 1213", level: path_ship::REFERENCE_LEVEL, func: path_ship::BIG_FN, classes: &path_ship::BIG_CLASSES, update: path_ship::update, joints: &path_ship::BIG_CLASSES },
    UnitPort { unit: "U155 434", level: eudora_crank_lift::REFERENCE_LEVEL, func: eudora_crank_lift::UPDATE_FN, classes: &eudora_crank_lift::CLASSES, update: eudora_crank_lift::update, joints: &eudora_crank_lift::CLASSES },
    UnitPort { unit: "U154 432", level: eudora_crank_follower::REFERENCE_LEVEL, func: eudora_crank_follower::UPDATE_FN, classes: &eudora_crank_follower::CLASSES, update: eudora_crank_follower::update, joints: &[] },
    UnitPort { unit: "U225 1066", level: blarg_waker::REFERENCE_LEVEL, func: blarg_waker::UPDATE_FN, classes: &blarg_waker::CLASSES, update: blarg_waker::update, joints: &[] },
    UnitPort { unit: "U140 899", level: kerwan_path_spawner::REFERENCE_LEVEL, func: kerwan_path_spawner::UPDATE_FN, classes: &kerwan_path_spawner::CLASSES, update: kerwan_path_spawner::spawner_update, joints: &[] },
    // The movers 899 creates (created only: not in the census's placed units).
    UnitPort { unit: "U140 898", level: kerwan_path_spawner::REFERENCE_LEVEL, func: kerwan_path_spawner::MOVER_FN, classes: &kerwan_path_spawner::MOVER_CLASSES, update: kerwan_path_spawner::mover_update, joints: &[] },
    UnitPort { unit: "L03 825", level: kerwan_turntable::REFERENCE_LEVEL, func: kerwan_turntable::UPDATE_FN, classes: &kerwan_turntable::CLASSES, update: kerwan_turntable::update, joints: &[] },
    UnitPort { unit: "L03 997", level: kerwan_riser::REFERENCE_LEVEL, func: kerwan_riser::UPDATE_FN, classes: &kerwan_riser::CLASSES, update: kerwan_riser::update, joints: &[] },
    UnitPort { unit: "L03 1548", level: kerwan_scene_fx::REFERENCE_LEVEL, func: kerwan_scene_fx::UPDATE_FN, classes: &kerwan_scene_fx::CLASSES, update: kerwan_scene_fx::update, joints: &[] },
    UnitPort { unit: "L03 914", level: kerwan_bystander::REFERENCE_LEVEL, func: kerwan_bystander::UPDATE_FN, classes: &kerwan_bystander::CLASSES, update: kerwan_bystander::update, joints: &[] },
    UnitPort { unit: "L03 816", level: kerwan_transport::REFERENCE_LEVEL, func: kerwan_transport::PLATFORM_FN, classes: &kerwan_transport::PLATFORM_CLASSES, update: kerwan_transport::update_816, joints: &[] },
    UnitPort { unit: "L03 1012", level: kerwan_transport::REFERENCE_LEVEL, func: kerwan_transport::SHUTTLE_FN, classes: &kerwan_transport::SHUTTLE_CLASSES, update: kerwan_transport::update_1012, joints: &[] },
    UnitPort { unit: "L03 578", level: kerwan_layer::REFERENCE_LEVEL, func: kerwan_layer::LAYER_FN, classes: &kerwan_layer::LAYER_CLASSES, update: kerwan_layer::update_578, joints: &[] },
    // The blobs 578 drops (created only: not in the census's placed units).
    UnitPort { unit: "L03 627", level: kerwan_layer::REFERENCE_LEVEL, func: kerwan_layer::BLOB_FN, classes: &kerwan_layer::BLOB_CLASSES, update: kerwan_layer::update_627, joints: &[] },
    UnitPort { unit: "U319 1885", level: pod_launcher::REFERENCE_LEVEL, func: pod_launcher::UPDATE_FN, classes: &pod_launcher::CLASSES, update: pod_launcher::update, joints: &pod_launcher::CLASSES },
    // The pods 1885 lobs (created only: not in the census's placed units).
    UnitPort { unit: "U319 1886", level: pod_launcher::REFERENCE_LEVEL, func: pod_launcher::POD_FN, classes: &pod_launcher::POD_CLASSES, update: pod_launcher::pod_update, joints: &[] },
    UnitPort { unit: "U99 656", level: aridia_idlers::REFERENCE_LEVEL, func: aridia_idlers::FN_656, classes: &aridia_idlers::CLASSES_656, update: aridia_idlers::update_656, joints: &[] },
    UnitPort { unit: "U101 675", level: aridia_idlers::REFERENCE_LEVEL, func: aridia_idlers::FN_675, classes: &aridia_idlers::CLASSES_675, update: aridia_idlers::update_675, joints: &[] },
    UnitPort { unit: "U104 732", level: aridia_idlers::REFERENCE_LEVEL, func: aridia_idlers::FN_732, classes: &aridia_idlers::CLASSES_732, update: aridia_idlers::update_732, joints: &[] },
    UnitPort { unit: "U105 733", level: aridia_cannon::REFERENCE_LEVEL, func: aridia_cannon::UPDATE_FN, classes: &aridia_cannon::CLASSES, update: aridia_cannon::update, joints: &aridia_cannon::CLASSES },
    UnitPort { unit: "U105 1208", level: aridia_cannon::REFERENCE_LEVEL, func: aridia_cannon::SHELL_FN, classes: &aridia_cannon::SHELL_CLASSES, update: aridia_cannon::shell_update, joints: &[] },
    UnitPort { unit: "U106 735", level: aridia_gates::REFERENCE_LEVEL, func: aridia_gates::UPDATE_FN, classes: &aridia_gates::CLASSES, update: aridia_gates::update, joints: &[] },
    UnitPort { unit: "U110 762", level: aridia_boulder::REFERENCE_LEVEL, func: aridia_boulder::UPDATE_FN, classes: &aridia_boulder::CLASSES, update: aridia_boulder::update, joints: &[] },
    UnitPort { unit: "U113 792", level: aridia_arm_platform::REFERENCE_LEVEL, func: aridia_arm_platform::UPDATE_FN, classes: &aridia_arm_platform::CLASSES, update: aridia_arm_platform::update, joints: &[] },
    UnitPort { unit: "U120 1479", level: aridia_fire::REFERENCE_LEVEL, func: aridia_fire::UPDATE_FN, classes: &aridia_fire::CLASSES, update: aridia_fire::update, joints: &[] },
    UnitPort { unit: "U103 713", level: aridia_tube::REFERENCE_LEVEL, func: aridia_tube::UPDATE_FN, classes: &aridia_tube::CLASSES, update: aridia_tube::update, joints: &[] },
    UnitPort { unit: "U97 651", level: aridia_lift::REFERENCE_LEVEL, func: aridia_lift::UPDATE_FN, classes: &aridia_lift::CLASSES, update: aridia_lift::update, joints: &[] },
    UnitPort { unit: "U97 651 glow", level: aridia_lift::REFERENCE_LEVEL, func: aridia_lift::DRAW_FN, classes: &[], update: empty::update, joints: &[] },
    UnitPort { unit: "U134 455", level: pod_spawner::REFERENCE_LEVEL, func: pod_spawner::UPDATE_FN, classes: &pod_spawner::CLASSES, update: pod_spawner::update, joints: &pod_spawner::CLASSES },
    UnitPort { unit: "U134 545", level: pod_spawner::REFERENCE_LEVEL, func: pod_spawner::POD_FN, classes: &pod_spawner::POD_CLASSES, update: pod_spawner::pod_update, joints: &[] },
    UnitPort { unit: "U139 631", level: hover_ship::REFERENCE_LEVEL, func: hover_ship::UPDATE_FN, classes: &hover_ship::CLASSES, update: hover_ship::update, joints: &hover_ship::CLASSES },
    UnitPort { unit: "U139 848", level: hover_ship::REFERENCE_LEVEL, func: hover_ship::FLAME_FN, classes: &hover_ship::FLAME_CLASSES, update: hover_ship::flame_update, joints: &[] },
    UnitPort { unit: "U581 582", level: veldin_rails::REFERENCE_LEVEL, func: veldin_rails::UPDATE_FN, classes: &veldin_rails::CLASSES, update: veldin_rails::update, joints: &[] },
    UnitPort { unit: "U597 1434", level: veldin_turnover::REFERENCE_LEVEL, func: veldin_turnover::UPDATE_FN, classes: &veldin_turnover::CLASSES, update: veldin_turnover::update, joints: &[] },
    UnitPort { unit: "U602 1799", level: veldin_scene_jet::REFERENCE_LEVEL, func: veldin_scene_jet::UPDATE_FN, classes: &veldin_scene_jet::CLASSES, update: veldin_scene_jet::update, joints: &[] },
    UnitPort { unit: "U593 1392", level: veldin_lock_field::REFERENCE_LEVEL, func: veldin_lock_field::UPDATE_FN, classes: &veldin_lock_field::CLASSES, update: veldin_lock_field::update, joints: &[] },
    // Draw callback only (1392's bands `0x2f1fd8`: `Callback::UnitQuads`).
    UnitPort { unit: "U593 1392 field", level: veldin_lock_field::REFERENCE_LEVEL, func: veldin_lock_field::DRAW_FN, classes: &[], update: empty::update, joints: &[] },
    UnitPort { unit: "U594 1402", level: veldin_pool::REFERENCE_LEVEL, func: veldin_pool::UPDATE_FN, classes: &veldin_pool::CLASSES, update: veldin_pool::update, joints: &[] },
    // Draw callbacks only (1402's three meshes: `Callback::UnitQuads`).
    UnitPort { unit: "U594 1402 mesh 0", level: veldin_pool::REFERENCE_LEVEL, func: veldin_pool::DRAW_FNS[0], classes: &[], update: empty::update, joints: &[] },
    UnitPort { unit: "U594 1402 mesh 1", level: veldin_pool::REFERENCE_LEVEL, func: veldin_pool::DRAW_FNS[1], classes: &[], update: empty::update, joints: &[] },
    UnitPort { unit: "U594 1402 mesh 2", level: veldin_pool::REFERENCE_LEVEL, func: veldin_pool::DRAW_FNS[2], classes: &[], update: empty::update, joints: &[] },
    UnitPort { unit: "U519 1890", level: energy_fan::REFERENCE_LEVEL, func: energy_fan::UPDATE_FN, classes: &energy_fan::CLASSES, update: energy_fan::update, joints: &[] },
    // Draw callback only (the fan's blades and rings `0x2fb018`: `Callback::UnitQuads`, two groups).
    UnitPort { unit: "U519 1890 fan", level: energy_fan::REFERENCE_LEVEL, func: energy_fan::DRAW_FN, classes: &[], update: empty::update, joints: &[] },
    UnitPort { unit: "U599 1563", level: veldin_finale_fx::REFERENCE_LEVEL, func: veldin_finale_fx::UPDATE_FN, classes: &veldin_finale_fx::CLASSES, update: veldin_finale_fx::update, joints: &[] },
    // Draw callbacks only (1563's flash, glow and beam: `Callback::UnitQuads`; the seat glows: `UnitGlow`).
    UnitPort { unit: "U599 1563 flash", level: veldin_finale_fx::REFERENCE_LEVEL, func: veldin_finale_fx::FLASH_FN, classes: &[], update: empty::update, joints: &[] },
    UnitPort { unit: "U599 1563 seat", level: veldin_finale_fx::REFERENCE_LEVEL, func: veldin_finale_fx::SEAT_FN, classes: &[], update: empty::update, joints: &[] },
    UnitPort { unit: "U599 1563 glow", level: veldin_finale_fx::REFERENCE_LEVEL, func: veldin_finale_fx::GLOW_FN, classes: &[], update: empty::update, joints: &[] },
    UnitPort { unit: "U599 1563 beam", level: veldin_finale_fx::REFERENCE_LEVEL, func: veldin_finale_fx::BEAM_FN, classes: &[], update: empty::update, joints: &[] },
    UnitPort { unit: "U599 1563 morph beam", level: veldin_finale_fx::REFERENCE_LEVEL, func: veldin_finale_fx::MORPH_FN, classes: &[], update: empty::update, joints: &[] },
    UnitPort { unit: "U598 1454", level: veldin_tank::REFERENCE_LEVEL, func: veldin_tank::UPDATE_FN, classes: &veldin_tank::CLASSES, update: veldin_tank::update, joints: &veldin_tank::CLASSES },
    // The tank's shells and treads (created only: not in the census's placed units).
    UnitPort { unit: "U598 41", level: veldin_tank::REFERENCE_LEVEL, func: veldin_tank::SHELL_FN, classes: &veldin_tank::SHELL_CLASSES, update: veldin_tank::shell_update, joints: &[] },
    UnitPort { unit: "U598 331", level: veldin_tank::REFERENCE_LEVEL, func: veldin_tank::TREAD_FN, classes: &veldin_tank::TREAD_CLASSES, update: veldin_tank::tread_update, joints: &[] },
    UnitPort { unit: "U511 1356", level: dropship::REFERENCE_LEVEL, func: dropship::UPDATE_FN, classes: &dropship::CLASSES, update: dropship::update, joints: &dropship::CLASSES },
    // The dropship's shots (created only).
    UnitPort { unit: "U511 50", level: dropship::REFERENCE_LEVEL, func: dropship::SHOT_FN, classes: &dropship::SHOT_CLASSES, update: dropship::shot_update, joints: &[] },
    UnitPort { unit: "U504 638", level: drop_trooper::REFERENCE_LEVEL, func: drop_trooper::UPDATE_FN, classes: &drop_trooper::CLASSES, update: drop_trooper::update, joints: &drop_trooper::CLASSES },
    // The trooper's shots (created only).
    UnitPort { unit: "U504 49", level: drop_trooper::REFERENCE_LEVEL, func: drop_trooper::SHOT_FN, classes: &drop_trooper::SHOT_CLASSES, update: drop_trooper::shot_update, joints: &[] },
    // The flyers' and gunship's wreck (created only; the module sits with its level-01 ships).
    UnitPort { unit: "1510", level: super::burning_wreck::REFERENCE_LEVEL, func: super::burning_wreck::UPDATE_FN, classes: &super::burning_wreck::CLASSES, update: super::burning_wreck::update, joints: &[] },
    // The Summoner mouse 1818's glow sprites (draw only; its update is `classes::mouse`).
    UnitPort { unit: "1818 glow", level: super::mouse::REFERENCE_LEVEL, func: super::mouse::GLOW_FN, classes: &[], update: empty::update, joints: &[] },
    // The blob shadows' draw (`fun_001f4880`, `crate::shadows::blob`; draw only).
    UnitPort { unit: "blob shadow", level: 1, func: crate::shadows::BLOB_FN, classes: &[], update: empty::update, joints: &[] },
    UnitPort { unit: "1510 glow", level: super::burning_wreck::REFERENCE_LEVEL, func: super::burning_wreck::DRAW_FN, classes: &[], update: empty::update, joints: &[] },
    UnitPort { unit: "U24 530", level: veldin_ship::REFERENCE_LEVEL, func: veldin_ship::UPDATE_FN, classes: &veldin_ship::CLASSES, update: veldin_ship::update, joints: &veldin_ship::CLASSES },
    UnitPort { unit: "U36 1440", level: veldin_beamer::REFERENCE_LEVEL, func: veldin_beamer::UPDATE_FN, classes: &veldin_beamer::CLASSES, update: veldin_beamer::update, joints: &veldin_beamer::CLASSES },
    UnitPort { unit: "U37 1471", level: veldin_beamer::REFERENCE_LEVEL, func: veldin_beamer::MANAGER_FN, classes: &veldin_beamer::MANAGER_CLASSES, update: veldin_beamer::manager, joints: &[] },
    // Draw callbacks only (1440's eye glow `0x2e1c78`: `Callback::UnitGlow`; 1471's beams `0x2e2af0`: `UnitFrame` + `UnitQuads`).
    UnitPort { unit: "U36 1440 eye", level: veldin_beamer::REFERENCE_LEVEL, func: veldin_beamer::EYE_FN, classes: &[], update: empty::update, joints: &[] },
    UnitPort { unit: "U37 1471 beams", level: veldin_beamer::REFERENCE_LEVEL, func: veldin_beamer::BEAM_FN, classes: &[], update: empty::update, joints: &[] },
    UnitPort { unit: "U38 1545", level: veldin_scene_fx::REFERENCE_LEVEL, func: veldin_scene_fx::UPDATE_FN, classes: &veldin_scene_fx::CLASSES, update: veldin_scene_fx::update, joints: &[] },
    UnitPort { unit: "1475 board missile", level: board_missile::REFERENCE_LEVEL, func: board_missile::UPDATE_FN, classes: &board_missile::CLASSES, update: board_missile::update, joints: &[] },
    UnitPort { unit: "U509 933", level: kalebo_mine::REFERENCE_LEVEL, func: kalebo_mine::UPDATE_FN, classes: &kalebo_mine::CLASSES, update: kalebo_mine::update, joints: &[] },
    UnitPort { unit: "U521 1401", level: kalebo_mine_drone::REFERENCE_LEVEL, func: kalebo_mine_drone::UPDATE_FN, classes: &kalebo_mine_drone::CLASSES, update: kalebo_mine_drone::update, joints: &[] },
    UnitPort { unit: "U509 923", level: kalebo_mine_drone::L14_LEVEL, func: kalebo_mine_drone::L14_FN, classes: &kalebo_mine_drone::L14_CLASSES, update: kalebo_mine_drone::update_l14, joints: &[] },
    // class port `classes::teleporter`).
    UnitPort { unit: "U85 1135 beam", level: 1, func: crate::moby_update::classes::teleporter::BEAM_FN, classes: &[], update: empty::update, joints: &[] },
    UnitPort { unit: "U359 318", level: pokitaru_teleporter::REFERENCE_LEVEL, func: pokitaru_teleporter::UPDATE_FN, classes: &pokitaru_teleporter::CLASSES, update: pokitaru_teleporter::update, joints: &[] },
    // Draw callback only (the beam of the Pokitaru pads: `Callback::UnitQuads` payload).
    UnitPort { unit: "U359 318 beam", level: pokitaru_teleporter::REFERENCE_LEVEL, func: pokitaru_teleporter::BEAM_FN, classes: &[], update: empty::update, joints: &[] },
    // The cave drips 751 spawns (created only: not in the census's placed units).
    UnitPort { unit: "787 drip", level: drip::REFERENCE_LEVEL, func: drip::UPDATE_FN, classes: &drip::CLASSES, update: drip::update, joints: &[] },
    UnitPort { unit: "U50 613", level: water_current::REFERENCE_LEVEL, func: water_current::UPDATE_FN, classes: &water_current::CLASSES, update: water_current::update, joints: &[] },
    UnitPort { unit: "U129 573", level: kerwan_hound::REFERENCE_LEVEL, func: kerwan_hound::UPDATE_FN, classes: &kerwan_hound::CLASSES, update: kerwan_hound::update, joints: &[] },
    UnitPort { unit: "U353 114", level: pokitaru_commando::REFERENCE_LEVEL, func: pokitaru_commando::UPDATE_FN, classes: &pokitaru_commando::CLASSES, update: pokitaru_commando::update, joints: &pokitaru_commando::CLASSES },
    UnitPort { unit: "U350 65", level: pokitaru_commando::REFERENCE_LEVEL, func: pokitaru_commando::GATE_FN, classes: &pokitaru_commando::GATE_CLASSES, update: pokitaru_commando::gate_update, joints: &[] },
    UnitPort { unit: "U363 1157", level: pokitaru_cutaway::REFERENCE_LEVEL, func: pokitaru_cutaway::UPDATE_FN, classes: &pokitaru_cutaway::CLASSES, update: pokitaru_cutaway::update, joints: &[] },
    // The boss of Veldin's last arena and its partners (level 18; 2026-10-01).
    UnitPort { unit: "U564 1422", level: veldin_boss::REFERENCE_LEVEL, func: veldin_boss::UPDATE_FN, classes: &veldin_boss::CLASSES, update: veldin_boss::update, joints: &veldin_boss::JOINTS },
    // Draw callback only (the boss's glows `0x2f7880`: `Callback::UnitGlow` payload).
    UnitPort { unit: "U564 1422 glow", level: veldin_boss::REFERENCE_LEVEL, func: veldin_boss::DRAW_FN, classes: &[], update: empty::update, joints: &[] },
    UnitPort { unit: "U557 644", level: veldin_cutaway::REFERENCE_LEVEL, func: veldin_cutaway::UPDATE_FN, classes: &veldin_cutaway::CLASSES, update: veldin_cutaway::update, joints: &[] },
    UnitPort { unit: "U556 587", level: veldin_floater::REFERENCE_LEVEL, func: veldin_floater::UPDATE_FN, classes: &veldin_floater::CLASSES, update: veldin_floater::update, joints: &[] },
    UnitPort { unit: "U572 1906", level: veldin_hopper::REFERENCE_LEVEL, func: veldin_hopper::UPDATE_FN, classes: &veldin_hopper::CLASSES, update: veldin_hopper::update, joints: &veldin_hopper::CLASSES },
    UnitPort { unit: "U572 1906 glow", level: veldin_hopper::REFERENCE_LEVEL, func: veldin_hopper::GLOW_FN, classes: &[], update: empty::update, joints: &[] },
    UnitPort { unit: "U572 1906 beam", level: veldin_hopper::REFERENCE_LEVEL, func: veldin_hopper::BEAM_FN, classes: &[], update: empty::update, joints: &[] },
    UnitPort { unit: "U554 583", level: veldin_pads::REFERENCE_LEVEL, func: veldin_pads::PAD_FN, classes: &veldin_pads::PAD_CLASSES, update: veldin_pads::pad_update, joints: &[] },
    UnitPort { unit: "U555 586", level: veldin_pads::REFERENCE_LEVEL, func: veldin_pads::BUTTON_FN, classes: &veldin_pads::BUTTON_CLASSES, update: veldin_pads::button_update, joints: &[] },
    // Draw callback with a `rand` at draw time (586's countdown `0x2d8098`: `Callback::UnitFrame` payload).
    UnitPort { unit: "U555 586 countdown", level: veldin_pads::REFERENCE_LEVEL, func: veldin_pads::COUNTDOWN_FN, classes: &[], update: empty::update, joints: &[] },
    // The boss's created shots and effects (not placed: created only).
    UnitPort { unit: "564 lob", level: veldin_shots::REFERENCE_LEVEL, func: veldin_shots::LOB_FN, classes: &veldin_shots::LOB_CLASSES, update: veldin_shots::lob_update, joints: &[] },
    // Draw callback only (564's target marker `0x2d5768`: `Callback::UnitQuads` payload).
    UnitPort { unit: "564 lob marker", level: veldin_shots::REFERENCE_LEVEL, func: veldin_shots::LOB_MARK_FN, classes: &[], update: empty::update, joints: &[] },
    UnitPort { unit: "624 ring", level: veldin_shots::REFERENCE_LEVEL, func: veldin_shots::RING_FN, classes: &veldin_shots::RING_CLASSES, update: veldin_shots::ring_update, joints: &[] },
    UnitPort { unit: "628 aura", level: veldin_shots::REFERENCE_LEVEL, func: veldin_shots::AURA_FN, classes: &veldin_shots::AURA_CLASSES, update: veldin_shots::aura_update, joints: &[] },
    UnitPort { unit: "628 aura bands", level: veldin_shots::REFERENCE_LEVEL, func: veldin_shots::AURA_DRAW_FN, classes: &[], update: empty::update, joints: &[] },
    UnitPort { unit: "983 beam core", level: veldin_shots::REFERENCE_LEVEL, func: veldin_shots::CORE_DRAW_FN, classes: &[], update: empty::update, joints: &[] },
    UnitPort { unit: "983 beam strands", level: veldin_shots::REFERENCE_LEVEL, func: veldin_shots::STRAND_DRAW_FN, classes: &[], update: empty::update, joints: &[] },
    UnitPort { unit: "983 beam", level: veldin_shots::REFERENCE_LEVEL, func: veldin_shots::BEAM_FN, classes: &veldin_shots::BEAM_CLASSES, update: veldin_shots::beam_update, joints: &[] },
    UnitPort { unit: "1898 flash", level: veldin_shots::REFERENCE_LEVEL, func: veldin_shots::FLASH_FN, classes: &veldin_shots::FLASH_CLASSES, update: veldin_shots::flash_update, joints: &[] },
    UnitPort { unit: "U123 822", level: kerwan_train::REFERENCE_LEVEL, func: kerwan_train::UPDATE_FN, classes: &kerwan_train::CLASSES, update: kerwan_train::update, joints: &kerwan_train::JOINTS },
    UnitPort { unit: "U124 845", level: kerwan_train::REFERENCE_LEVEL, func: kerwan_train::LEAD_FN, classes: &kerwan_train::LEAD_CLASSES, update: kerwan_train::lead_update, joints: &[] },
    UnitPort { unit: "U323 22", level: clank_section::REFERENCE_LEVEL, func: clank_section::UPDATE_FN, classes: &clank_section::CLASSES, update: clank_section::update, joints: &[] },
    UnitPort { unit: "U500 1451", level: giant_pad::REFERENCE_LEVEL, func: giant_pad::UPDATE_FN, classes: &giant_pad::CLASSES, update: giant_pad::update, joints: &[] },
    UnitPort { unit: "U220 1061", level: blarg_clank_lift::REFERENCE_LEVEL, func: blarg_clank_lift::UPDATE_FN, classes: &blarg_clank_lift::CLASSES, update: blarg_clank_lift::update, joints: &[] },
    UnitPort { unit: "Giant Clank 0x593", level: giant_shockwave::REFERENCE_LEVEL, func: giant_shockwave::UPDATE_FN, classes: &giant_shockwave::CLASSES, update: giant_shockwave::update, joints: &[] },
    UnitPort { unit: "Giant Clank 0x100", level: giant_missile::REFERENCE_LEVEL, func: giant_missile::UPDATE_FN, classes: &giant_missile::CLASSES, update: giant_missile::update, joints: &[] },
    UnitPort { unit: "Giant Clank 0x5f3", level: giant_beam::REFERENCE_LEVEL, func: giant_beam::UPDATE_FN, classes: &giant_beam::CLASSES, update: giant_beam::update, joints: &[] },
    UnitPort { unit: "U499 1446", level: quartu_giant_mission::REFERENCE_LEVEL, func: quartu_giant_mission::UPDATE_FN, classes: &quartu_giant_mission::CLASSES, update: quartu_giant_mission::update, joints: &quartu_giant_mission::CLASSES },
    UnitPort { unit: "U307 664", level: riding_floats::REFERENCE_LEVEL_664, func: riding_floats::UPDATE_FN_664, classes: &riding_floats::CLASSES_664, update: riding_floats::float_664, joints: &[] },
    UnitPort { unit: "U316 1293", level: riding_floats::REFERENCE_LEVEL_1293, func: riding_floats::UPDATE_FN_1293, classes: &riding_floats::CLASSES_1293, update: riding_floats::float_1293, joints: &[] },
    UnitPort { unit: "U255 1069", level: riding_floats::REFERENCE_LEVEL_1069, func: riding_floats::UPDATE_FN_1069, classes: &riding_floats::CLASSES_1069, update: riding_floats::float_1069, joints: &[] },
    // Story drivers (batch 6, lane story: docs/plan/progression.md `## story`).
    UnitPort { unit: "U114 786", level: aridia_story::REFERENCE_LEVEL, func: aridia_story::SURFER_FN, classes: &aridia_story::SURFER_CLASSES, update: aridia_story::surfer_update, joints: &aridia_story::SURFER_CLASSES },
    UnitPort { unit: "U115 788", level: aridia_story::REFERENCE_LEVEL, func: aridia_story::AGENT_FN, classes: &aridia_story::AGENT_CLASSES, update: aridia_story::agent_update, joints: &aridia_story::AGENT_CLASSES },
    UnitPort { unit: "U142 890", level: kerwan_story::REFERENCE_LEVEL, func: kerwan_story::HELGA_FN, classes: &kerwan_story::HELGA_CLASSES, update: kerwan_story::helga_update, joints: &kerwan_story::HELGA_CLASSES },
    UnitPort { unit: "U145 909", level: kerwan_story::REFERENCE_LEVEL, func: kerwan_story::AL_FN, classes: &kerwan_story::AL_CLASSES, update: kerwan_story::al_update, joints: &kerwan_story::AL_CLASSES },
    UnitPort { unit: "U169 1120", level: eudora_story::REFERENCE_LEVEL, func: eudora_story::SUCK_FN, classes: &eudora_story::SUCK_CLASSES, update: eudora_story::suck_cannon_update, joints: &[] },
    UnitPort { unit: "U170 1190", level: eudora_story::REFERENCE_LEVEL, func: eudora_story::INFORMANT_FN, classes: &eudora_story::INFORMANT_CLASSES, update: eudora_story::informant_update, joints: &[] },
    UnitPort { unit: "U200 918", level: rilgar_story::REFERENCE_LEVEL, func: rilgar_story::GIRL_FN, classes: &rilgar_story::GIRL_CLASSES, update: rilgar_story::race_girl_update, joints: &rilgar_story::GIRL_CLASSES },
    UnitPort { unit: "U201 919", level: rilgar_story::REFERENCE_LEVEL, func: rilgar_story::BOUNCER_FN, classes: &rilgar_story::BOUNCER_CLASSES, update: rilgar_story::bouncer_update, joints: &rilgar_story::BOUNCER_CLASSES },
    UnitPort { unit: "U203 925", level: rilgar_story::REFERENCE_LEVEL, func: rilgar_story::SALESMAN_FN, classes: &rilgar_story::SALESMAN_CLASSES, update: rilgar_story::salesman_update, joints: &rilgar_story::SALESMAN_CLASSES },
    UnitPort { unit: "U233 1105", level: blarg_story::REFERENCE_LEVEL, func: blarg_story::SCIENTIST_FN, classes: &blarg_story::SCIENTIST_CLASSES, update: blarg_story::scientist_update, joints: &blarg_story::SCIENTIST_CLASSES },
    UnitPort { unit: "U297 1130", level: batalia_story::REFERENCE_LEVEL, func: batalia_story::COMMANDO_FN, classes: &batalia_story::COMMANDO_CLASSES, update: batalia_story::commando_update, joints: &batalia_story::COMMANDO_CLASSES },
    UnitPort { unit: "U298 1144", level: batalia_story::REFERENCE_LEVEL, func: batalia_story::DESERTER_FN, classes: &batalia_story::DESERTER_CLASSES, update: batalia_story::deserter_update, joints: &batalia_story::DESERTER_CLASSES },
    UnitPort { unit: "U299 1283", level: batalia_story::REFERENCE_LEVEL, func: batalia_story::WORKER_FN, classes: &batalia_story::WORKER_CLASSES, update: batalia_story::water_worker_update, joints: &batalia_story::WORKER_CLASSES },
    UnitPort { unit: "U321 1290", level: gaspar_story::REFERENCE_LEVEL, func: gaspar_story::UPDATE_FN, classes: &gaspar_story::CLASSES, update: gaspar_story::update, joints: &[] },
    UnitPort { unit: "U329 18", level: orxon_story::REFERENCE_LEVEL, func: orxon_story::MAGNEBOOTS_FN, classes: &orxon_story::MAGNEBOOTS_CLASSES, update: orxon_story::magneboots_update, joints: &[] },
    UnitPort { unit: "U350 1326", level: orxon_story::REFERENCE_LEVEL, func: orxon_story::NANOTECH_FN, classes: &orxon_story::NANOTECH_CLASSES, update: orxon_story::nanotech_update, joints: &[] },
    UnitPort { unit: "U360 23", level: pokitaru_story::REFERENCE_LEVEL, func: pokitaru_story::MASK_FN, classes: &pokitaru_story::MASK_CLASSES, update: pokitaru_story::mask_update, joints: &[] },
    UnitPort { unit: "U363 90", level: pokitaru_story::REFERENCE_LEVEL, func: pokitaru_story::THRUSTER_FN, classes: &pokitaru_story::THRUSTER_CLASSES, update: pokitaru_story::thruster_update, joints: &pokitaru_story::THRUSTER_CLASSES },
    UnitPort { unit: "U365 298", level: pokitaru_story::REFERENCE_LEVEL, func: pokitaru_story::PERSUADER_FN, classes: &pokitaru_story::PERSUADER_CLASSES, update: pokitaru_story::persuader_update, joints: &pokitaru_story::PERSUADER_CLASSES },
    UnitPort { unit: "U394 282", level: hoven_story::REFERENCE_LEVEL, func: hoven_story::MERCHANT_FN, classes: &hoven_story::MERCHANT_CLASSES, update: hoven_story::merchant_update, joints: &hoven_story::MERCHANT_CLASSES },
    UnitPort { unit: "U411 1404", level: hoven_story::REFERENCE_LEVEL, func: hoven_story::SCENE_FN, classes: &hoven_story::SCENE_CLASSES, update: hoven_story::scene_trigger_update, joints: &[] },
    UnitPort { unit: "U398 328", level: hoven_story::REFERENCE_LEVEL, func: hoven_story::HYDRO_FN, classes: &hoven_story::HYDRO_CLASSES, update: hoven_story::hydro_update, joints: &hoven_story::HYDRO_CLASSES },
    UnitPort { unit: "U440 1353", level: gemlik_story::REFERENCE_LEVEL, func: gemlik_story::UPDATE_FN, classes: &gemlik_story::CLASSES, update: gemlik_story::update, joints: &[] },
    UnitPort { unit: "U464 851", level: oltanis_story::REFERENCE_LEVEL, func: oltanis_story::QWARK_FN, classes: &oltanis_story::QWARK_CLASSES, update: oltanis_story::qwark_update, joints: &oltanis_story::QWARK_CLASSES },
    UnitPort { unit: "U469 924", level: oltanis_story::REFERENCE_LEVEL, func: oltanis_story::MERCHANT_FN, classes: &oltanis_story::MERCHANT_CLASSES, update: oltanis_story::merchant_update, joints: &oltanis_story::MERCHANT_CLASSES },
    UnitPort { unit: "U474 1354", level: oltanis_story::REFERENCE_LEVEL, func: oltanis_story::MORPH_FN, classes: &oltanis_story::MORPH_CLASSES, update: oltanis_story::morph_update, joints: &[] },
    UnitPort { unit: "U503 1388", level: quartu_story::REFERENCE_LEVEL, func: quartu_story::GRABBER_FN, classes: &quartu_story::GRABBER_CLASSES, update: quartu_story::grabber_update, joints: &[] },
    UnitPort { unit: "U511 1469", level: quartu_story::REFERENCE_LEVEL, func: quartu_story::HELP_FN, classes: &quartu_story::HELP_CLASSES, update: quartu_story::help_update, joints: &[] },
    UnitPort { unit: "U506 1419", level: quartu_story::REFERENCE_LEVEL, func: quartu_story::BROADCAST_FN, classes: &quartu_story::BROADCAST_CLASSES, update: quartu_story::broadcast_update, joints: &[] },
    UnitPort { unit: "U529 1377", level: kalebo_story::REFERENCE_LEVEL, func: kalebo_story::UPDATE_FN, classes: &kalebo_story::CLASSES, update: kalebo_story::update, joints: &kalebo_story::CLASSES },
    UnitPort { unit: "U561 1428", level: fleet_story::REFERENCE_LEVEL, func: fleet_story::UPDATE_FN, classes: &fleet_story::CLASSES, update: fleet_story::update, joints: &[] },
    UnitPort { unit: "U31 834", level: veldin_story::REFERENCE_LEVEL, func: veldin_story::UPDATE_FN, classes: &veldin_story::CLASSES, update: veldin_story::update, joints: &[] },
    UnitPort { unit: "U248 436", level: umbris_story::REFERENCE_LEVEL, func: umbris_story::UPDATE_FN, classes: &umbris_story::CLASSES, update: umbris_story::update, joints: &[] },
    UnitPort { unit: "U118 1005", level: aridia_story::REFERENCE_LEVEL, func: aridia_story::ITEM_SCENE_FN, classes: &aridia_story::ITEM_SCENE_CLASSES, update: aridia_story::item_scene_update, joints: &[] },
    UnitPort { unit: "U405 1267", level: hoven_turret::REFERENCE_LEVEL, func: hoven_turret::UPDATE_FN, classes: &hoven_turret::CLASSES, update: hoven_turret::update, joints: &hoven_turret::CLASSES },
    // Draw callbacks only (no class runs them): the HUD (`Callback::UnitFrame`) and the red screen (`Callback::UnitQuads`).
    UnitPort { unit: "U405 1267 hud", level: hoven_turret::REFERENCE_LEVEL, func: hoven_turret::HUD_FN, classes: &[], update: empty::update, joints: &[] },
    UnitPort { unit: "U405 1267 tint", level: hoven_turret::REFERENCE_LEVEL, func: hoven_turret::TINT_FN, classes: &[], update: empty::update, joints: &[] },
    UnitPort { unit: "U237 1109", level: blarg_shuttle::REFERENCE_LEVEL, func: blarg_shuttle::UPDATE_FN, classes: &blarg_shuttle::CLASSES, update: blarg_shuttle::update, joints: &[] },
    UnitPort { unit: "U504 712", level: oltanis_ferry::REFERENCE_LEVEL, func: oltanis_ferry::UPDATE_FN, classes: &oltanis_ferry::CLASSES, update: oltanis_ferry::update, joints: &[] },
    UnitPort { unit: "U502 685", level: oltanis_cart::REFERENCE_LEVEL, func: oltanis_cart::UPDATE_FN, classes: &oltanis_cart::CLASSES, update: oltanis_cart::update, joints: &[] },
    UnitPort { unit: "U497 557", level: oltanis_glider::REFERENCE_LEVEL, func: oltanis_glider::UPDATE_FN, classes: &oltanis_glider::CLASSES, update: oltanis_glider::update, joints: &oltanis_glider::CLASSES },
    UnitPort { unit: "U501 684", level: oltanis_lightning::REFERENCE_LEVEL, func: oltanis_lightning::UPDATE_FN, classes: &oltanis_lightning::CLASSES, update: oltanis_lightning::update, joints: &[] },
    UnitPort { unit: "U501 684 bolt", level: oltanis_lightning::REFERENCE_LEVEL, func: oltanis_lightning::DRAW_FN, classes: &[], update: empty::update, joints: &[] },
    UnitPort { unit: "U498 610", level: oltanis_wind::REFERENCE_LEVEL, func: oltanis_wind::UPDATE_FN, classes: &oltanis_wind::CLASSES, update: oltanis_wind::update, joints: &[] },
    UnitPort { unit: "U498 610 motes", level: oltanis_wind::REFERENCE_LEVEL, func: oltanis_wind::DRAW_FN, classes: &[], update: empty::update, joints: &[] },
    UnitPort { unit: "U492 31", level: oltanis_drone::REFERENCE_LEVEL, func: oltanis_drone::UPDATE_FN, classes: &oltanis_drone::CLASSES, update: oltanis_drone::update, joints: &oltanis_drone::CLASSES },
    UnitPort { unit: "U643 81", level: oltanis_drone::REFERENCE_LEVEL, func: oltanis_drone::PIECE_FN, classes: &oltanis_drone::PIECE_CLASSES, update: oltanis_drone::piece_update, joints: &[] },
    UnitPort { unit: "U644 1193", level: oltanis_drone::REFERENCE_LEVEL, func: oltanis_drone::GRENADE_FN, classes: &oltanis_drone::GRENADE_CLASSES, update: oltanis_drone::grenade_update, joints: &[] },
    UnitPort { unit: "U520 1417", level: oltanis_car::REFERENCE_LEVEL, func: oltanis_car::UPDATE_FN, classes: &oltanis_car::CLASSES, update: oltanis_car::update, joints: &oltanis_car::CLASSES },
    UnitPort { unit: "U506 908", level: oltanis_carrier::REFERENCE_LEVEL, func: oltanis_carrier::FIGHTER_FN, classes: &oltanis_carrier::FIGHTER_CLASSES, update: oltanis_carrier::fighter_update, joints: &[] },
    UnitPort { unit: "U512 1224", level: oltanis_bolt::REFERENCE_LEVEL, func: oltanis_bolt::UPDATE_FN, classes: &oltanis_bolt::CLASSES, update: oltanis_bolt::update, joints: &[] },
    UnitPort { unit: "U512 1224 bolt", level: oltanis_bolt::REFERENCE_LEVEL, func: oltanis_bolt::DRAW_FN, classes: &[], update: empty::update, joints: &[] },
    UnitPort { unit: "U493 211", level: oltanis_rail_bot::REFERENCE_LEVEL, func: oltanis_rail_bot::UPDATE_FN, classes: &oltanis_rail_bot::CLASSES, update: oltanis_rail_bot::update, joints: &oltanis_rail_bot::CLASSES },
    UnitPort { unit: "U493 211 glows", level: oltanis_rail_bot::REFERENCE_LEVEL, func: oltanis_rail_bot::DRAW_FN, classes: &[], update: empty::update, joints: &[] },
    UnitPort { unit: "U490 28", level: oltanis_zapper::REFERENCE_LEVEL, func: oltanis_zapper::UPDATE_FN, classes: &oltanis_zapper::CLASSES, update: oltanis_zapper::update, joints: &oltanis_zapper::CLASSES },
    UnitPort { unit: "U490 28 glows", level: oltanis_zapper::REFERENCE_LEVEL, func: oltanis_zapper::DRAW_FN, classes: &[], update: empty::update, joints: &[] },
    UnitPort { unit: "U641 325 glow", level: oltanis_zapper::REFERENCE_LEVEL, func: oltanis_zapper::PIECE_DRAW_FN, classes: &[], update: empty::update, joints: &[] },
    UnitPort { unit: "U641 325", level: oltanis_zapper::REFERENCE_LEVEL, func: oltanis_zapper::PIECE_FN, classes: &oltanis_zapper::PIECE_CLASSES, update: oltanis_zapper::piece_update, joints: &[] },
    UnitPort { unit: "U642 403", level: oltanis_zapper::REFERENCE_LEVEL, func: oltanis_zapper::SPARK_PIECE_FN, classes: &oltanis_zapper::SPARK_PIECE_CLASSES, update: oltanis_zapper::spark_piece_update, joints: &[] },
    UnitPort { unit: "U514 1331", level: oltanis_arcs::REFERENCE_LEVEL, func: oltanis_arcs::UPDATE_FN, classes: &oltanis_arcs::CLASSES, update: oltanis_arcs::update, joints: &[] },
    UnitPort { unit: "U514 1331 arcs", level: oltanis_arcs::REFERENCE_LEVEL, func: oltanis_arcs::DRAW_FN, classes: &[], update: empty::update, joints: &[] },
    UnitPort { unit: "U488 8", level: oltanis_sentry::REFERENCE_LEVEL, func: oltanis_sentry::UPDATE_FN, classes: &oltanis_sentry::CLASSES, update: oltanis_sentry::update, joints: &oltanis_sentry::CLASSES },
    UnitPort { unit: "U488 8 cones", level: oltanis_sentry::REFERENCE_LEVEL, func: oltanis_sentry::MASTER_DRAW_FN, classes: &[], update: empty::update, joints: &[] },
    UnitPort { unit: "U488 8 spot", level: oltanis_sentry::REFERENCE_LEVEL, func: oltanis_sentry::SPOT_FN, classes: &[], update: empty::update, joints: &[] },
    UnitPort { unit: "U551 1394", level: quartu_small::REFERENCE_LEVEL, func: quartu_small::DROPPER_FN, classes: &quartu_small::DROPPER_CLASSES, update: quartu_small::dropper_update, joints: &[] },
    UnitPort { unit: "U551 1257", level: quartu_small::REFERENCE_LEVEL, func: quartu_small::BOMB_FN, classes: &quartu_small::BOMB_CLASSES, update: quartu_small::bomb_update, joints: &[] },
    UnitPort { unit: "U560 1560", level: quartu_small::REFERENCE_LEVEL, func: quartu_small::THRUSTERS_FN, classes: &quartu_small::THRUSTERS_CLASSES, update: quartu_small::thrusters_update, joints: &[] },
    UnitPort { unit: "U536 92", level: quartu_small::REFERENCE_LEVEL, func: quartu_small::GATE_FN, classes: &quartu_small::GATE_CLASSES, update: quartu_small::gate_update, joints: &[] },
    UnitPort { unit: "U530 67", level: quartu_small::REFERENCE_LEVEL, func: quartu_small::DOOR_FN, classes: &quartu_small::DOOR_CLASSES, update: quartu_small::door_update, joints: &[] },
    UnitPort { unit: "U556 1430", level: quartu_small::REFERENCE_LEVEL, func: quartu_small::DISPENSER_FN, classes: &quartu_small::DISPENSER_CLASSES, update: quartu_small::dispenser_update, joints: &[] },
    UnitPort { unit: "U556 1428", level: quartu_small::REFERENCE_LEVEL, func: quartu_small::PIECE_FN, classes: &quartu_small::PIECE_CLASSES, update: quartu_small::piece_update, joints: &[] },
    UnitPort { unit: "U535 78", level: barrier_field::REFERENCE_LEVEL, func: barrier_field::UPDATE_FN, classes: &barrier_field::CLASSES, update: barrier_field::update, joints: &[] },
    UnitPort { unit: "U535 78 shimmer", level: barrier_field::REFERENCE_LEVEL, func: barrier_field::DRAW_FN, classes: &[], update: empty::update, joints: &[] },
    UnitPort { unit: "U546 655", level: water_shock::REFERENCE_LEVEL, func: water_shock::UPDATE_FN, classes: &water_shock::CLASSES, update: water_shock::update, joints: &[] },
    UnitPort { unit: "U546 655 bolt", level: water_shock::REFERENCE_LEVEL, func: water_shock::DRAW_FN, classes: &[], update: empty::update, joints: &[] },
    UnitPort { unit: "U546 655 countdown", level: water_shock::REFERENCE_LEVEL, func: water_shock::COUNTDOWN_FN, classes: &[], update: empty::update, joints: &[] },
    UnitPort { unit: "U528 44", level: quartu_guard::REFERENCE_LEVEL, func: quartu_guard::UPDATE_FN, classes: &quartu_guard::CLASSES, update: quartu_guard::update, joints: &quartu_guard::CLASSES },
    UnitPort { unit: "U528 44 eyes", level: quartu_guard::REFERENCE_LEVEL, func: quartu_guard::DRAW_FN, classes: &[], update: empty::update, joints: &[] },
    UnitPort { unit: "U528 934", level: quartu_guard::REFERENCE_LEVEL, func: quartu_guard::SHOT_FN, classes: &quartu_guard::SHOT_CLASSES, update: quartu_guard::shot_update, joints: &[] },
    UnitPort { unit: "U543 233", level: quartu_hover::REFERENCE_LEVEL, func: quartu_hover::UPDATE_FN, classes: &quartu_hover::CLASSES, update: quartu_hover::update, joints: &quartu_hover::CLASSES },
    UnitPort { unit: "U545 491", level: quartu_jet_bot::REFERENCE_LEVEL, func: quartu_jet_bot::UPDATE_FN, classes: &quartu_jet_bot::CLASSES, update: quartu_jet_bot::update, joints: &quartu_jet_bot::CLASSES },
    UnitPort { unit: "U539 148 255", level: quartu_building::REFERENCE_LEVEL, func: quartu_building::UPDATE_FN, classes: &quartu_building::CLASSES, update: quartu_building::update, joints: &[] },
    UnitPort { unit: "U540 154", level: quartu_building::REFERENCE_LEVEL, func: quartu_building::WALL_FN, classes: &quartu_building::WALL_CLASSES, update: quartu_building::wall_update, joints: &[] },
    UnitPort { unit: "U612 1382", level: fleet_crew::REFERENCE_LEVEL, func: fleet_crew::UPDATE_FN, classes: &fleet_crew::CLASSES, update: fleet_crew::update, joints: &fleet_crew::CLASSES },
    UnitPort { unit: "U612 1382 eyes", level: fleet_crew::REFERENCE_LEVEL, func: fleet_crew::DRAW_FN, classes: &[], update: empty::update, joints: &[] },
    UnitPort { unit: "U611 1380", level: fleet_lift::REFERENCE_LEVEL, func: fleet_lift::UPDATE_FN, classes: &fleet_lift::CLASSES, update: fleet_lift::update, joints: &[] },
    UnitPort { unit: "U606 835", level: fleet_small::REFERENCE_LEVEL, func: fleet_small::MINE_FN, classes: &fleet_small::MINE_CLASSES, update: fleet_small::mine_update, joints: &[] },
    UnitPort { unit: "U615 1470", level: help_fleet::REFERENCE_LEVEL, func: help_fleet::UPDATE_FN, classes: &help_fleet::CLASSES, update: help_fleet::update, joints: &[] },
    UnitPort { unit: "U616 1562", level: fleet_small::REFERENCE_LEVEL, func: fleet_small::THRUSTERS_FN, classes: &fleet_small::THRUSTERS_CLASSES, update: fleet_small::thrusters_update, joints: &[] },
    UnitPort { unit: "U614 1448", level: fleet_shuttle::REFERENCE_LEVEL, func: fleet_shuttle::UPDATE_FN, classes: &fleet_shuttle::CLASSES, update: fleet_shuttle::update, joints: &[] },
    UnitPort { unit: "U617 1772", level: fleet_sky::REFERENCE_LEVEL, func: fleet_sky::UPDATE_FN, classes: &fleet_sky::CLASSES, update: fleet_sky::update, joints: &[] },
    UnitPort { unit: "U617 1772 sky", level: fleet_sky::REFERENCE_LEVEL, func: fleet_sky::DRAW_FN, classes: &[], update: empty::update, joints: &[] },
    UnitPort { unit: "U552 1408", level: wave_mesh::DEFS[0].level, func: wave_mesh::DEFS[0].update, classes: &wave_mesh::CLASSES[0], update: wave_mesh::UPDATES[0], joints: &[] },
    UnitPort { unit: "U552 1408 draw", level: wave_mesh::DEFS[0].level, func: wave_mesh::DEFS[0].draw, classes: &[], update: empty::update, joints: &[] },
    UnitPort { unit: "U552 1409", level: wave_mesh::DEFS[1].level, func: wave_mesh::DEFS[1].update, classes: &wave_mesh::CLASSES[1], update: wave_mesh::UPDATES[1], joints: &[] },
    UnitPort { unit: "U552 1409 draw", level: wave_mesh::DEFS[1].level, func: wave_mesh::DEFS[1].draw, classes: &[], update: empty::update, joints: &[] },
    UnitPort { unit: "U552 1565", level: wave_mesh::DEFS[2].level, func: wave_mesh::DEFS[2].update, classes: &wave_mesh::CLASSES[2], update: wave_mesh::UPDATES[2], joints: &[] },
    UnitPort { unit: "U552 1565 draw", level: wave_mesh::DEFS[2].level, func: wave_mesh::DEFS[2].draw, classes: &[], update: empty::update, joints: &[] },
    UnitPort { unit: "U552 1567", level: wave_mesh::DEFS[3].level, func: wave_mesh::DEFS[3].update, classes: &wave_mesh::CLASSES[3], update: wave_mesh::UPDATES[3], joints: &[] },
    UnitPort { unit: "U552 1567 draw", level: wave_mesh::DEFS[3].level, func: wave_mesh::DEFS[3].draw, classes: &[], update: empty::update, joints: &[] },
    UnitPort { unit: "U552 1405", level: wave_mesh::DEFS[4].level, func: wave_mesh::DEFS[4].update, classes: &wave_mesh::CLASSES[4], update: wave_mesh::UPDATES[4], joints: &[] },
    UnitPort { unit: "U552 1405 draw", level: wave_mesh::DEFS[4].level, func: wave_mesh::DEFS[4].draw, classes: &[], update: empty::update, joints: &[] },
    UnitPort { unit: "U552 1406", level: wave_mesh::DEFS[5].level, func: wave_mesh::DEFS[5].update, classes: &wave_mesh::CLASSES[5], update: wave_mesh::UPDATES[5], joints: &[] },
    UnitPort { unit: "U552 1406 draw", level: wave_mesh::DEFS[5].level, func: wave_mesh::DEFS[5].draw, classes: &[], update: empty::update, joints: &[] },
    UnitPort { unit: "U552 1407", level: wave_mesh::DEFS[6].level, func: wave_mesh::DEFS[6].update, classes: &wave_mesh::CLASSES[6], update: wave_mesh::UPDATES[6], joints: &[] },
    UnitPort { unit: "U552 1407 draw", level: wave_mesh::DEFS[6].level, func: wave_mesh::DEFS[6].draw, classes: &[], update: empty::update, joints: &[] },
    UnitPort { unit: "U507 921", level: oltanis_carrier::REFERENCE_LEVEL, func: oltanis_carrier::CARRIER_FN, classes: &oltanis_carrier::CARRIER_CLASSES, update: oltanis_carrier::carrier_update, joints: &[] },
    UnitPort { unit: "U508 922", level: oltanis_carrier::REFERENCE_LEVEL, func: oltanis_carrier::MISSILE_FN, classes: &oltanis_carrier::MISSILE_CLASSES, update: oltanis_carrier::missile_update, joints: &[] },
    UnitPort { unit: "U496 386", level: oltanis_arc::REFERENCE_LEVEL, func: oltanis_arc::UPDATE_FN, classes: &oltanis_arc::CLASSES, update: oltanis_arc::update, joints: &[] },
    UnitPort { unit: "U496 386 ribbons", level: oltanis_arc::REFERENCE_LEVEL, func: oltanis_arc::DRAW_FN, classes: &[], update: empty::update, joints: &[] },
    UnitPort { unit: "U519 1416", level: oltanis_small::REFERENCE_LEVEL, func: oltanis_small::PAD_FN, classes: &oltanis_small::PAD_CLASSES, update: oltanis_small::pad_update, joints: &[] },
    UnitPort { unit: "U523 1559", level: oltanis_small::REFERENCE_LEVEL, func: oltanis_small::SKILL_FN, classes: &oltanis_small::SKILL_CLASSES, update: oltanis_small::skill_update, joints: &[] },
    UnitPort { unit: "U517 1395", level: oltanis_small::REFERENCE_LEVEL, func: oltanis_small::STAIRS_FN, classes: &oltanis_small::STAIRS_CLASSES, update: oltanis_small::stairs_update, joints: &[] },
    UnitPort { unit: "U499 643", level: oltanis_small::REFERENCE_LEVEL, func: oltanis_small::MINE_FN, classes: &oltanis_small::MINE_CLASSES, update: oltanis_small::mine_update, joints: &[] },
    UnitPort { unit: "U515 1352", level: oltanis_small::REFERENCE_LEVEL, func: oltanis_small::RISER_FN, classes: &oltanis_small::RISER_CLASSES, update: oltanis_small::riser_update, joints: &[] },
    UnitPort { unit: "U494 250", level: oltanis_small::REFERENCE_LEVEL, func: oltanis_small::HATCH_FN, classes: &oltanis_small::HATCH_CLASSES, update: oltanis_small::hatch_update, joints: &[] },
    UnitPort { unit: "U495 309", level: oltanis_small::REFERENCE_LEVEL, func: oltanis_small::FLOAT_FN, classes: &oltanis_small::FLOAT_CLASSES, update: oltanis_small::float_update, joints: &[] },
    UnitPort { unit: "U480 1403", level: gemlik_small::REFERENCE_LEVEL, func: gemlik_small::TRACKER_FN, classes: &gemlik_small::TRACKER_CLASSES, update: gemlik_small::tracker_update, joints: &[] },
    UnitPort { unit: "U481 1558", level: gemlik_small::REFERENCE_LEVEL, func: gemlik_small::THRUSTERS_FN, classes: &gemlik_small::THRUSTERS_CLASSES, update: gemlik_small::thrusters_update, joints: &[] },
    UnitPort { unit: "U477 1270", level: gemlik_lift::REFERENCE_LEVEL, func: gemlik_lift::TIP_FN, classes: &gemlik_lift::TIP_CLASSES, update: gemlik_lift::tip_update, joints: &[] },
    UnitPort { unit: "U475 1262", level: gemlik_robot::REFERENCE_LEVEL, func: gemlik_robot::UPDATE_FN, classes: &gemlik_robot::CLASSES, update: gemlik_robot::update, joints: &gemlik_robot::CLASSES },
    // Draw callback only (1262's shockwave ring: `Callback::UnitFrame` for its `rand`, `Callback::UnitQuads`).
    UnitPort { unit: "U475 1262 ring", level: gemlik_robot::REFERENCE_LEVEL, func: gemlik_robot::DRAW_FN, classes: &[], update: empty::update, joints: &[] },
    UnitPort { unit: "U470 667", level: gemlik_relay::REFERENCE_LEVEL, func: gemlik_relay::RELAY_FN, classes: &gemlik_relay::RELAY_CLASSES, update: gemlik_relay::relay_update, joints: &[] },
    UnitPort { unit: "U471 674", level: gemlik_relay::REFERENCE_LEVEL, func: gemlik_relay::FIELD_FN, classes: &gemlik_relay::FIELD_CLASSES, update: gemlik_relay::field_update, joints: &[] },
    UnitPort { unit: "U467 404", level: gemlik_field::REFERENCE_LEVEL, func: gemlik_field::UPDATE_FN, classes: &gemlik_field::CLASSES, update: gemlik_field::update, joints: &[] },
    UnitPort { unit: "U462 231", level: gemlik_switch::REFERENCE_LEVEL, func: gemlik_switch::UPDATE_FN, classes: &gemlik_switch::CLASSES, update: gemlik_switch::update, joints: &[] },
    UnitPort { unit: "U459 170", level: gemlik_tower::REFERENCE_LEVEL, func: gemlik_tower::UPDATE_FN, classes: &gemlik_tower::CLASSES, update: gemlik_tower::update, joints: &gemlik_tower::CLASSES },
    UnitPort { unit: "— 1632", level: gemlik_tower::REFERENCE_LEVEL, func: gemlik_tower::PROXY_FN, classes: &gemlik_tower::PROXY_CLASSES, update: gemlik_tower::proxy_update, joints: &[] },
    UnitPort { unit: "U457 111", level: gemlik_fighter::REFERENCE_LEVEL, func: gemlik_fighter::UPDATE_FN, classes: &gemlik_fighter::CLASSES, update: gemlik_fighter::update, joints: &[] },
    UnitPort { unit: "U456 101", level: gemlik_launcher::REFERENCE_LEVEL, func: gemlik_launcher::UPDATE_FN, classes: &gemlik_launcher::CLASSES, update: gemlik_launcher::update, joints: &gemlik_launcher::CLASSES },
    UnitPort { unit: "— 1233", level: gemlik_launcher::REFERENCE_LEVEL, func: gemlik_launcher::MISSILE_FN, classes: &gemlik_launcher::MISSILE_CLASSES, update: gemlik_launcher::missile_update, joints: &[] },
    UnitPort { unit: "U449 21", level: gemlik_lift::REFERENCE_LEVEL, func: gemlik_lift::UPDATE_FN, classes: &gemlik_lift::CLASSES, update: gemlik_lift::update, joints: &[] },
    UnitPort { unit: "U448 6", level: gemlik_trigger::REFERENCE_LEVEL, func: gemlik_trigger::UPDATE_FN, classes: &gemlik_trigger::CLASSES, update: gemlik_trigger::update, joints: &[] },
    UnitPort { unit: "U445 1557", level: hoven_story::REFERENCE_LEVEL, func: hoven_story::THRUSTERS_FN, classes: &hoven_story::THRUSTERS_CLASSES, update: hoven_story::thrusters_update, joints: &[] },
    UnitPort { unit: "U441 1345", level: hoven_beam::REFERENCE_LEVEL, func: hoven_beam::UPDATE_FN, classes: &hoven_beam::CLASSES, update: hoven_beam::update, joints: &[] },
    // Draw callback only (1345's curtain: `Callback::UnitFrame` for its `rand`, `Callback::UnitQuads`).
    UnitPort { unit: "U441 1345 curtain", level: hoven_beam::REFERENCE_LEVEL, func: hoven_beam::DRAW_FN, classes: &[], update: empty::update, joints: &[] },
    UnitPort { unit: "U440 1281", level: hoven_dispenser::REFERENCE_LEVEL, func: hoven_dispenser::UPDATE_FN, classes: &hoven_dispenser::CLASSES, update: hoven_dispenser::update, joints: &[] },
    UnitPort { unit: "U438 1269", level: hoven_mine::REFERENCE_LEVEL, func: hoven_mine::UPDATE_FN, classes: &hoven_mine::CLASSES, update: hoven_mine::update, joints: &[] },
    UnitPort { unit: "U436 1259", level: hoven_arc::REFERENCE_LEVEL, func: hoven_arc::UPDATE_FN, classes: &hoven_arc::CLASSES, update: hoven_arc::update, joints: &[] },
    // Draw callback only (1259's bolts: `Callback::UnitQuads`).
    UnitPort { unit: "U436 1259 bolts", level: hoven_arc::REFERENCE_LEVEL, func: hoven_arc::DRAW_FN, classes: &[], update: empty::update, joints: &[] },
    UnitPort { unit: "U432 384", level: hoven_fall::REFERENCE_LEVEL, func: hoven_fall::UPDATE_FN, classes: &hoven_fall::CLASSES, update: hoven_fall::update, joints: &[] },
    // Draw callback only (384's strips: `Callback::UnitQuads`).
    UnitPort { unit: "U432 384 strips", level: hoven_fall::REFERENCE_LEVEL, func: hoven_fall::DRAW_FN, classes: &[], update: empty::update, joints: &[] },
    UnitPort { unit: "U430 336", level: hoven_copter::REFERENCE_LEVEL, func: hoven_copter::UPDATE_FN, classes: &hoven_copter::CLASSES, update: hoven_copter::update, joints: &[] },
    UnitPort { unit: "U427 294", level: hoven_gunner::REFERENCE_LEVEL, func: hoven_gunner::UPDATE_FN, classes: &hoven_gunner::CLASSES, update: hoven_gunner::update, joints: &hoven_gunner::CLASSES },
    UnitPort { unit: "U424 240", level: hoven_platform::REFERENCE_LEVEL, func: hoven_platform::UPDATE_FN, classes: &hoven_platform::CLASSES, update: hoven_platform::update, joints: &[] },
    UnitPort { unit: "U423 238", level: hoven_burrower::REFERENCE_LEVEL, func: hoven_burrower::UPDATE_FN, classes: &hoven_burrower::CLASSES, update: hoven_burrower::update, joints: &hoven_burrower::CLASSES },
    UnitPort { unit: "U369 1378", level: orxon_airlock::REFERENCE_LEVEL, func: orxon_airlock::UPDATE_FN, classes: &orxon_airlock::CLASSES, update: orxon_airlock::update, joints: &[] },
    // Draw callback only (1378's shimmer: `Callback::UnitQuads`, six groups).
    UnitPort { unit: "U369 1378 shimmer", level: orxon_airlock::REFERENCE_LEVEL, func: orxon_airlock::DRAW_FN, classes: &[], update: empty::update, joints: &[] },
    UnitPort { unit: "U363 1229", level: orxon_drone::REFERENCE_LEVEL, func: orxon_drone::UPDATE_FN, classes: &orxon_drone::CLASSES, update: orxon_drone::update, joints: &orxon_drone::CLASSES },
    UnitPort { unit: "U363 819", level: orxon_drone::REFERENCE_LEVEL, func: orxon_drone::SHOT_FN, classes: &orxon_drone::SHOT_CLASSES, update: orxon_drone::shot_update, joints: &[] },
    UnitPort { unit: "U349 702", level: orxon_flame::REFERENCE_LEVEL, func: orxon_flame::UPDATE_FN, classes: &orxon_flame::CLASSES, update: orxon_flame::update, joints: &[] },
    UnitPort { unit: "U347 351", level: orxon_pads::REFERENCE_LEVEL, func: orxon_pads::UPDATE_FN, classes: &orxon_pads::CLASSES, update: orxon_pads::update, joints: &[] },
    UnitPort { unit: "U353 947", level: orxon_wire::REFERENCE_LEVEL, func: orxon_wire::UPDATE_FN, classes: &orxon_wire::CLASSES, update: orxon_wire::update, joints: &[] },
    UnitPort { unit: "U353 1090", level: orxon_wire::REFERENCE_LEVEL, func: orxon_wire::SPARK_FN, classes: &orxon_wire::SPARK_CLASSES, update: orxon_wire::spark_update, joints: &[] },
    UnitPort { unit: "U357 1067", level: orxon_curtain::REFERENCE_LEVEL, func: orxon_curtain::UPDATE_FN, classes: &orxon_curtain::CLASSES, update: orxon_curtain::update, joints: &[] },
    UnitPort { unit: "U357 1067 draw", level: orxon_curtain::REFERENCE_LEVEL, func: orxon_curtain::DRAW_FN, classes: &[], update: empty::update, joints: &[] },
    UnitPort { unit: "U350 794", level: orxon_curtain::REFERENCE_LEVEL, func: orxon_curtain::GENERATOR_FN, classes: &orxon_curtain::GENERATOR_CLASSES, update: orxon_curtain::generator_update, joints: &[] },
    UnitPort { unit: "U358 1073", level: orxon_curtain::REFERENCE_LEVEL, func: orxon_curtain::FIELD_FN, classes: &orxon_curtain::FIELD_CLASSES, update: orxon_curtain::field_update, joints: &[] },
    UnitPort { unit: "U359 1100", level: orxon_small::REFERENCE_LEVEL, func: orxon_small::WALL_FN, classes: &orxon_small::WALL_CLASSES, update: orxon_small::wall_update, joints: &[] },
    UnitPort { unit: "U343 1033", level: orxon_small::REFERENCE_LEVEL, func: orxon_small::LIFT_FN, classes: &orxon_small::LIFT_CLASSES, update: orxon_small::lift_update, joints: &[] },
    UnitPort { unit: "U355 1031", level: orxon_small::REFERENCE_LEVEL, func: orxon_small::PLATE_FN, classes: &orxon_small::PLATE_CLASSES, update: orxon_small::plate_update, joints: &[] },
    UnitPort { unit: "U348 353", level: orxon_small::REFERENCE_LEVEL, func: orxon_small::GATE_FN, classes: &orxon_small::GATE_CLASSES, update: orxon_small::gate_update, joints: &[] },
    UnitPort { unit: "U368 1346", level: orxon_small::REFERENCE_LEVEL, func: orxon_small::TURRET_FN, classes: &orxon_small::TURRET_CLASSES, update: orxon_small::turret_update, joints: &[] },
    UnitPort { unit: "U356 1047", level: orxon_small::REFERENCE_LEVEL, func: orxon_small::CORE_FN, classes: &orxon_small::CORE_CLASSES, update: orxon_small::core_update, joints: &[] },
    UnitPort { unit: "U356 1122", level: orxon_small::REFERENCE_LEVEL, func: 0x2d_d8d8, classes: &[1122], update: empty::update, joints: &[] },
    UnitPort { unit: "U356 1921", level: orxon_small::REFERENCE_LEVEL, func: 0x2e_b950, classes: &[1921], update: empty::update, joints: &[] },
    UnitPort { unit: "U308 1258 (10)", level: gaspar_cannon::ORXON_LEVEL, func: gaspar_cannon::ORXON_SHELL_FN, classes: &gaspar_cannon::SHELL_CLASSES, update: gaspar_cannon::shell_update, joints: &[] },
    UnitPort { unit: "U344 1117", level: orxon_small::REFERENCE_LEVEL, func: orxon_small::SINK_FN, classes: &orxon_small::SINK_CLASSES, update: orxon_small::sink_update, joints: &[] },
    UnitPort { unit: "U370 1421", level: orxon_small::REFERENCE_LEVEL, func: orxon_small::BRIDGE_FN, classes: &orxon_small::BRIDGE_CLASSES, update: orxon_small::bridge_update, joints: &[] },
    UnitPort { unit: "U371 1424", level: orxon_small::REFERENCE_LEVEL, func: orxon_small::BLOCK_FN, classes: &orxon_small::BLOCK_CLASSES, update: orxon_small::block_update, joints: &[] },
    UnitPort { unit: "U373 1555", level: orxon_small::REFERENCE_LEVEL, func: orxon_small::THRUSTERS_FN, classes: &orxon_small::THRUSTERS_CLASSES, update: orxon_small::thrusters_update, joints: &[] },
    UnitPort { unit: "U352 939", level: orxon_lava::REFERENCE_LEVEL, func: orxon_lava::UPDATE_FN, classes: &orxon_lava::CLASSES, update: orxon_lava::update, joints: &[] },
    UnitPort { unit: "U352 938", level: orxon_lava::REFERENCE_LEVEL, func: orxon_lava::ROCK_FN, classes: &orxon_lava::ROCK_CLASSES, update: orxon_lava::rock_update, joints: &[] },
    UnitPort { unit: "U313 1766", level: gaspar_raft::REFERENCE_LEVEL, func: gaspar_raft::UPDATE_FN, classes: &gaspar_raft::CLASSES, update: gaspar_raft::update, joints: &gaspar_raft::CLASSES },
    UnitPort { unit: "U308 1201", level: gaspar_cannon::REFERENCE_LEVEL, func: gaspar_cannon::UPDATE_FN, classes: &gaspar_cannon::CLASSES, update: gaspar_cannon::update, joints: &gaspar_cannon::CLASSES },
    UnitPort { unit: "U308 324", level: gaspar_cannon::REFERENCE_LEVEL, func: gaspar_cannon::BASE_FN, classes: &gaspar_cannon::BASE_CLASSES, update: empty::update, joints: &[] },
    UnitPort { unit: "U308 1258", level: gaspar_cannon::REFERENCE_LEVEL, func: gaspar_cannon::SHELL_FN, classes: &gaspar_cannon::SHELL_CLASSES, update: gaspar_cannon::shell_update, joints: &[] },
    UnitPort { unit: "U305 1150", level: crate::moby_update::classes::path_platform::GASPAR_LEVEL, func: crate::moby_update::classes::path_platform::GASPAR_FN, classes: &crate::moby_update::classes::path_platform::GASPAR_CLASSES, update: crate::moby_update::classes::path_platform::gaspar_update, joints: &[] },
    UnitPort { unit: "U487 903", level: crate::moby_update::classes::path_platform::OLTANIS_LEVEL, func: crate::moby_update::classes::path_platform::OLTANIS_FN, classes: &crate::moby_update::classes::path_platform::OLTANIS_CLASSES, update: crate::moby_update::classes::path_platform::oltanis_update, joints: &[] },
    UnitPort { unit: "U302 276", level: gaspar_breakable::REFERENCE_LEVEL, func: gaspar_breakable::UPDATE_FN, classes: &gaspar_breakable::CLASSES, update: gaspar_breakable::update, joints: &[] },
    UnitPort { unit: "U302 320", level: gaspar_breakable::REFERENCE_LEVEL, func: gaspar_breakable::CHUNK_FN, classes: &gaspar_breakable::CHUNK_CLASSES, update: gaspar_breakable::chunk_update, joints: &[] },
    UnitPort { unit: "U302 1300", level: gaspar_breakable::REFERENCE_LEVEL, func: gaspar_breakable::LAST_FN, classes: &gaspar_breakable::LAST_CLASSES, update: empty::update, joints: &[] },
    UnitPort { unit: "U310 1285", level: gaspar_breakable::REFERENCE_LEVEL, func: gaspar_breakable::RING_FN, classes: &gaspar_breakable::RING_CLASSES, update: gaspar_breakable::ring_update, joints: &[] },
    UnitPort { unit: "U301 263", level: gaspar_meteors::REFERENCE_LEVEL, func: gaspar_meteors::UPDATE_FN, classes: &gaspar_meteors::CLASSES, update: gaspar_meteors::update, joints: &[] },
    UnitPort { unit: "U301 417", level: gaspar_meteors::REFERENCE_LEVEL, func: gaspar_meteors::DUST_FN, classes: &gaspar_meteors::DUST_CLASSES, update: gaspar_meteors::dust_update, joints: &[] },
    UnitPort { unit: "U286 671", level: batalia_flame::REFERENCE_LEVEL, func: batalia_flame::UPDATE_FN, classes: &batalia_flame::CLASSES, update: batalia_flame::update, joints: &[] },
    UnitPort { unit: "U286 671 draw", level: batalia_flame::REFERENCE_LEVEL, func: batalia_flame::DRAW_FN, classes: &[], update: empty::update, joints: &[] },
    UnitPort { unit: "U276 444", level: batalia_gunship::REFERENCE_LEVEL, func: batalia_gunship::BIG_FN, classes: &batalia_gunship::BIG_CLASSES, update: batalia_gunship::update, joints: &batalia_gunship::JOINT_CLASSES },
    UnitPort { unit: "U278 462", level: batalia_gunship::REFERENCE_LEVEL, func: batalia_gunship::MID_FN, classes: &batalia_gunship::MID_CLASSES, update: batalia_gunship::update, joints: &batalia_gunship::JOINT_CLASSES },
    UnitPort { unit: "U279 463", level: batalia_gunship::REFERENCE_LEVEL, func: batalia_gunship::SMALL_FN, classes: &batalia_gunship::SMALL_CLASSES, update: batalia_gunship::update, joints: &batalia_gunship::JOINT_CLASSES },
    UnitPort { unit: "U276 parts", level: batalia_gunship::REFERENCE_LEVEL, func: batalia_gunship::PART_FN, classes: &batalia_gunship::PART_CLASSES, update: batalia_gunship::part_update, joints: &[] },
    UnitPort { unit: "U273 435", level: batalia_bomber::REFERENCE_LEVEL, func: batalia_bomber::UPDATE_FN, classes: &batalia_bomber::CLASSES, update: batalia_bomber::update, joints: &[] },
    UnitPort { unit: "U275 440", level: batalia_turret::REFERENCE_LEVEL, func: batalia_turret::UPDATE_FN, classes: &batalia_turret::CLASSES, update: batalia_turret::update, joints: &[] },
    UnitPort { unit: "U275 440 hud", level: batalia_turret::REFERENCE_LEVEL, func: batalia_turret::HUD_FN, classes: &[], update: empty::update, joints: &[] },
    UnitPort { unit: "U295 424", level: batalia_ferry::REFERENCE_LEVEL, func: batalia_ferry::UPDATE_FN, classes: &batalia_ferry::CLASSES, update: batalia_ferry::update, joints: &batalia_ferry::CLASSES },
    UnitPort { unit: "U292 253", level: batalia_tank::REFERENCE_LEVEL, func: batalia_tank::UPDATE_FN, classes: &batalia_tank::CLASSES, update: batalia_tank::update, joints: &batalia_tank::CLASSES },
    UnitPort { unit: "U292 331", level: batalia_tank::REFERENCE_LEVEL, func: batalia_tank::TREAD_FN, classes: &batalia_tank::TREAD_CLASSES, update: veldin_tank::tread_update, joints: &batalia_tank::TREAD_CLASSES },
    UnitPort { unit: "U292 425", level: batalia_tank::REFERENCE_LEVEL, func: batalia_tank::SHELL_FN, classes: &batalia_tank::SHELL_CLASSES, update: batalia_tank::shell_update, joints: &batalia_tank::SHELL_CLASSES },
    UnitPort { unit: "U294 333", level: batalia_grenadier::REFERENCE_LEVEL, func: batalia_grenadier::UPDATE_FN, classes: &batalia_grenadier::CLASSES, update: batalia_grenadier::update, joints: &batalia_grenadier::CLASSES },
    UnitPort { unit: "U294 248", level: batalia_grenadier::REFERENCE_LEVEL, func: batalia_grenadier::SHOT_FN, classes: &batalia_grenadier::SHOT_CLASSES, update: batalia_grenadier::shot_update, joints: &batalia_grenadier::SHOT_CLASSES },
    UnitPort { unit: "U305 468", level: batalia_small::REFERENCE_LEVEL, func: batalia_small::BRIDGE_FN, classes: &batalia_small::BRIDGE_CLASSES, update: batalia_small::bridge_update, joints: &[] },
    UnitPort { unit: "U318 1553", level: batalia_small::REFERENCE_LEVEL, func: batalia_small::SCENE_FX_FN, classes: &batalia_small::SCENE_FX_CLASSES, update: batalia_small::scene_fx_update, joints: &[] },
    UnitPort { unit: "U319 1629", level: batalia_small::REFERENCE_LEVEL, func: batalia_small::STREAKS_FN, classes: &batalia_small::STREAKS_CLASSES, update: batalia_small::streaks_update, joints: &[] },
    UnitPort { unit: "U319 1629", level: batalia_small::OLTANIS_LEVEL, func: batalia_small::OLTANIS_STREAKS_FN, classes: &batalia_small::STREAKS_CLASSES, update: batalia_small::streaks_update, joints: &[] },
    UnitPort { unit: "U320 1641", level: batalia_small::REFERENCE_LEVEL, func: batalia_small::STEAM_FN, classes: &batalia_small::STEAM_CLASSES, update: batalia_small::steam_update, joints: &[] },
    UnitPort { unit: "U317 1400", level: weather::REFERENCE_LEVEL, func: weather::UPDATE_FN, classes: &weather::CLASSES, update: weather::update, joints: &[] },
    UnitPort { unit: "U433 1400", level: weather::HOVEN_LEVEL, func: weather::HOVEN_FN, classes: &weather::CLASSES, update: weather::update, joints: &[] },
    UnitPort { unit: "U433 1400", level: weather::OLTANIS_LEVEL, func: weather::OLTANIS_FN, classes: &weather::CLASSES, update: weather::update, joints: &[] },
    UnitPort { unit: "U254 1046", level: umbris_beast_fx::REFERENCE_LEVEL, func: umbris_beast_fx::RING_FN, classes: &umbris_beast_fx::RING_CLASSES, update: umbris_beast_fx::ring_update, joints: &[] },
    UnitPort { unit: "U254 1049", level: umbris_beast_fx::REFERENCE_LEVEL, func: umbris_beast_fx::GLOB_FN, classes: &umbris_beast_fx::GLOB_CLASSES, update: umbris_beast_fx::glob_update, joints: &[] },
    UnitPort { unit: "U254 1106 tongue", level: umbris_beast_fx::REFERENCE_LEVEL, func: umbris_beast_fx::TONGUE_FN, classes: &[], update: empty::update, joints: &[] },
    UnitPort { unit: "U254 1106 beam", level: umbris_beast_fx::REFERENCE_LEVEL, func: umbris_beast_fx::BEAM_FN, classes: &[], update: empty::update, joints: &[] },
    UnitPort { unit: "U254 1106 shimmer", level: umbris_beast_fx::REFERENCE_LEVEL, func: umbris_beast_fx::SHIMMER_FN, classes: &[], update: empty::update, joints: &[] },
    UnitPort { unit: "U254 1106 fire line", level: umbris_beast_fx::REFERENCE_LEVEL, func: umbris_beast_fx::GROUND_FN, classes: &[], update: empty::update, joints: &[] },
    UnitPort { unit: "U254 1046 ring", level: umbris_beast_fx::REFERENCE_LEVEL, func: umbris_beast_fx::RING_DRAW_FN, classes: &[], update: empty::update, joints: &[] },
    UnitPort { unit: "U254 1106", level: umbris_beast::REFERENCE_LEVEL, func: umbris_beast::UPDATE_FN, classes: &umbris_beast::CLASSES, update: umbris_beast::update, joints: &umbris_beast::CLASSES },
    UnitPort { unit: "U267 1059", level: umbris_swamp::REFERENCE_LEVEL, func: umbris_swamp::UPDATE_FN, classes: &umbris_swamp::CLASSES, update: umbris_swamp::update, joints: &umbris_swamp::CLASSES },
    UnitPort { unit: "U258 1126", level: umbris_turret::REFERENCE_LEVEL, func: umbris_turret::UPDATE_FN, classes: &umbris_turret::CLASSES, update: umbris_turret::update, joints: &[] },
    UnitPort { unit: "U258 880", level: umbris_turret::REFERENCE_LEVEL, func: umbris_turret::SHOT_FN, classes: &umbris_turret::SHOT_CLASSES, update: umbris_turret::shot_update, joints: &[] },
    UnitPort { unit: "U272 1110", level: umbris_mines::REFERENCE_LEVEL, func: umbris_mines::LONE_FN, classes: &umbris_mines::LONE_CLASSES, update: umbris_mines::lone_update, joints: &[] },
    UnitPort { unit: "U273 1112", level: umbris_mines::REFERENCE_LEVEL, func: umbris_mines::CHAIN_FN, classes: &umbris_mines::CHAIN_CLASSES, update: umbris_mines::chain_update, joints: &[] },
    UnitPort { unit: "U260 871", level: umbris_mines::REFERENCE_LEVEL, func: umbris_mines::LEADER_FN, classes: &umbris_mines::LEADER_CLASSES, update: umbris_mines::leader_update, joints: &[] },
    UnitPort { unit: "U258 529", level: umbris_small::REFERENCE_LEVEL, func: umbris_small::SHIP_FN, classes: &umbris_small::SHIP_CLASSES, update: umbris_small::ship_update, joints: &[] },
    UnitPort { unit: "U284 1789", level: umbris_small::REFERENCE_LEVEL, func: umbris_small::JETS_FN, classes: &umbris_small::JETS_CLASSES, update: umbris_small::jets_update, joints: &[] },
    UnitPort { unit: "U280 1552", level: umbris_small::REFERENCE_LEVEL, func: umbris_small::SCENE_FX_FN, classes: &umbris_small::SCENE_FX_CLASSES, update: umbris_small::scene_fx_update, joints: &[] },
    UnitPort { unit: "U278 1133", level: umbris_small::REFERENCE_LEVEL, func: umbris_small::SWING_FN, classes: &umbris_small::SWING_CLASSES, update: umbris_small::swing_update, joints: &[] },
    UnitPort { unit: "U279 1142", level: umbris_small::REFERENCE_LEVEL, func: umbris_small::AMMO_FN, classes: &umbris_small::AMMO_CLASSES, update: umbris_small::ammo_drop_update, joints: &[] },
    UnitPort { unit: "U274 1113", level: umbris_small::REFERENCE_LEVEL, func: umbris_small::WALL_FN, classes: &umbris_small::WALL_CLASSES, update: umbris_small::wall_update, joints: &[] },
    UnitPort { unit: "U254 38", level: umbris_lift::REFERENCE_LEVEL, func: umbris_lift::UPDATE_FN, classes: &umbris_lift::CLASSES, update: umbris_lift::update, joints: &[] },
    UnitPort { unit: "U255 1474", level: umbris_lift::REFERENCE_LEVEL, func: umbris_lift::GATED_FN, classes: &umbris_lift::GATED_CLASSES, update: umbris_lift::gated_update, joints: &[] },
    UnitPort { unit: "U224 857", level: blarg_gadgetbot::REFERENCE_LEVEL, func: blarg_gadgetbot::UPDATE_FN, classes: &blarg_gadgetbot::CLASSES, update: blarg_gadgetbot::update, joints: &blarg_gadgetbot::CLASSES },
    UnitPort { unit: "U351 857", level: blarg_gadgetbot::ORXON_LEVEL, func: blarg_gadgetbot::ORXON_FN, classes: &blarg_gadgetbot::CLASSES, update: blarg_gadgetbot::update, joints: &blarg_gadgetbot::CLASSES },
    UnitPort { unit: "U224 857 glow", level: blarg_gadgetbot::REFERENCE_LEVEL, func: blarg_gadgetbot::DRAW_FN, classes: &[], update: empty::update, joints: &[] },
    UnitPort { unit: "U224 302", level: blarg_gadgetbot::REFERENCE_LEVEL, func: blarg_gadgetbot::BUBBLE_FN, classes: &blarg_gadgetbot::BUBBLE_CLASSES, update: blarg_gadgetbot::bubble_update, joints: &[] },
    UnitPort { unit: "U224 302 bubble", level: blarg_gadgetbot::REFERENCE_LEVEL, func: blarg_gadgetbot::BUBBLE_DRAW_FN, classes: &[], update: empty::update, joints: &[] },
    UnitPort { unit: "U224 303", level: blarg_gadgetbot::REFERENCE_LEVEL, func: blarg_gadgetbot::MARKER_FN, classes: &blarg_gadgetbot::MARKER_CLASSES, update: blarg_gadgetbot::marker_update, joints: &[] },
    UnitPort { unit: "U233 1051", level: blarg_boss::REFERENCE_LEVEL, func: blarg_boss::UPDATE_FN, classes: &blarg_boss::CLASSES, update: blarg_boss::update, joints: &blarg_boss::CLASSES },
    UnitPort { unit: "U238 1068", level: blarg_wave_bot::REFERENCE_LEVEL, func: blarg_wave_bot::UPDATE_FN, classes: &blarg_wave_bot::CLASSES, update: blarg_wave_bot::update, joints: &blarg_wave_bot::CLASSES },
    UnitPort { unit: "U238 1068 wave", level: blarg_wave_bot::REFERENCE_LEVEL, func: blarg_wave_bot::DRAW_FN, classes: &[], update: empty::update, joints: &[] },
    UnitPort { unit: "U232 1048", level: blarg_trooper::REFERENCE_LEVEL, func: blarg_trooper::UPDATE_FN, classes: &blarg_trooper::CLASSES, update: blarg_trooper::update, joints: &blarg_trooper::CLASSES },
    UnitPort { unit: "U236 1062", level: blarg_glass::REFERENCE_LEVEL, func: blarg_glass::WINDOWS_FN, classes: &blarg_glass::WINDOWS_CLASSES, update: blarg_glass::windows_update, joints: &[] },
    UnitPort { unit: "U239 1083", level: blarg_glass::REFERENCE_LEVEL, func: blarg_glass::WINDOW_FN, classes: &blarg_glass::WINDOW_CLASSES, update: blarg_glass::window_update, joints: &[] },
    UnitPort { unit: "U239 1085", level: blarg_glass::REFERENCE_LEVEL, func: blarg_glass::SHARD_FN, classes: &blarg_glass::SHARD_CLASSES, update: blarg_glass::shard_update, joints: &[] },
    UnitPort { unit: "U236 glass 0", level: blarg_glass::REFERENCE_LEVEL, func: blarg_glass::DRAW_FNS[0], classes: &[], update: empty::update, joints: &[] },
    UnitPort { unit: "U236 glass 1", level: blarg_glass::REFERENCE_LEVEL, func: blarg_glass::DRAW_FNS[1], classes: &[], update: empty::update, joints: &[] },
    UnitPort { unit: "U236 glass 2", level: blarg_glass::REFERENCE_LEVEL, func: blarg_glass::DRAW_FNS[2], classes: &[], update: empty::update, joints: &[] },
    UnitPort { unit: "U236 glass 3", level: blarg_glass::REFERENCE_LEVEL, func: blarg_glass::DRAW_FNS[3], classes: &[], update: empty::update, joints: &[] },
    UnitPort { unit: "U236 glass 4", level: blarg_glass::REFERENCE_LEVEL, func: blarg_glass::DRAW_FNS[4], classes: &[], update: empty::update, joints: &[] },
    UnitPort { unit: "U236 glass 5", level: blarg_glass::REFERENCE_LEVEL, func: blarg_glass::DRAW_FNS[5], classes: &[], update: empty::update, joints: &[] },
    UnitPort { unit: "U236 glass 6", level: blarg_glass::REFERENCE_LEVEL, func: blarg_glass::DRAW_FNS[6], classes: &[], update: empty::update, joints: &[] },
    UnitPort { unit: "U227 1028", level: blarg_bridge::REFERENCE_LEVEL, func: blarg_bridge::UPDATE_FN, classes: &blarg_bridge::CLASSES, update: blarg_bridge::update, joints: &[] },
    UnitPort { unit: "U242 1108", level: blarg_escape::REFERENCE_LEVEL, func: blarg_escape::UPDATE_FN, classes: &blarg_escape::CLASSES, update: blarg_escape::update, joints: &[] },
    UnitPort { unit: "U242 1108 countdown", level: blarg_escape::REFERENCE_LEVEL, func: blarg_escape::DRAW_FN, classes: &[], update: empty::update, joints: &[] },
    UnitPort { unit: "U244 1118", level: blarg_launch_tube::REFERENCE_LEVEL, func: blarg_launch_tube::UPDATE_FN, classes: &blarg_launch_tube::CLASSES, update: blarg_launch_tube::update, joints: &[] },
    UnitPort { unit: "U246 1302", level: blarg_bot_pad::REFERENCE_LEVEL, func: blarg_bot_pad::UPDATE_FN, classes: &blarg_bot_pad::CLASSES, update: blarg_bot_pad::update, joints: &[] },
    UnitPort { unit: "U246 1302 hologram", level: blarg_bot_pad::REFERENCE_LEVEL, func: blarg_bot_pad::DRAW_FN, classes: &[], update: empty::update, joints: &[] },
    UnitPort { unit: "U228 1035", level: blarg_laser_gate::REFERENCE_LEVEL, func: blarg_laser_gate::UPDATE_FN, classes: &blarg_laser_gate::CLASSES, update: blarg_laser_gate::update, joints: &[] },
    UnitPort { unit: "U228 1035 beams", level: blarg_laser_gate::REFERENCE_LEVEL, func: blarg_laser_gate::DRAW_FN, classes: &[], update: empty::update, joints: &[] },
    UnitPort { unit: "U245 1123", level: blarg_barrier::REFERENCE_LEVEL, func: blarg_barrier::UPDATE_FN, classes: &blarg_barrier::CLASSES, update: blarg_barrier::update, joints: &[] },
    UnitPort { unit: "U245 1123 beams", level: blarg_barrier::REFERENCE_LEVEL, func: blarg_barrier::DRAW_FN, classes: &[], update: empty::update, joints: &[] },
    UnitPort { unit: "U221 55", level: blarg_small::REFERENCE_LEVEL, func: blarg_small::DOOR_FN, classes: &blarg_small::DOOR_CLASSES, update: blarg_small::door_update, joints: &[] },
    UnitPort { unit: "U249 1551", level: blarg_small::REFERENCE_LEVEL, func: blarg_small::SCENE_FX_FN, classes: &blarg_small::SCENE_FX_CLASSES, update: blarg_small::scene_fx_update, joints: &[] },
    UnitPort { unit: "U223 827", level: blarg_crawler::REFERENCE_LEVEL, func: blarg_crawler::UPDATE_FN, classes: &blarg_crawler::CLASSES, update: blarg_crawler::update, joints: &blarg_crawler::CLASSES },
    UnitPort { unit: "U424 69", level: gemlik_ship::REFERENCE_LEVEL, func: gemlik_ship::UPDATE_FN, classes: &gemlik_ship::CLASSES, update: gemlik_ship::update, joints: &gemlik_ship::CLASSES },
    UnitPort { unit: "U424 69 hud", level: gemlik_ship::REFERENCE_LEVEL, func: gemlik_ship_hud::HUD_FN, classes: &[], update: empty::update, joints: &[] },
    UnitPort { unit: "U434 388", level: qwark_ship::REFERENCE_LEVEL, func: qwark_ship::UPDATE_FN, classes: &qwark_ship::CLASSES, update: qwark_ship::update, joints: &qwark_ship::CLASSES },
    UnitPort { unit: "U434 388 beam", level: qwark_ship::REFERENCE_LEVEL, func: qwark_ship::BEAM_FN, classes: &[], update: empty::update, joints: &[] },
    UnitPort { unit: "388 parts", level: qwark_ship_parts::REFERENCE_LEVEL, func: qwark_ship_parts::PART_FN, classes: &qwark_ship_parts::PART_CLASSES, update: qwark_ship_parts::part_update, joints: &[] },
    UnitPort { unit: "388 shield 352", level: qwark_ship_parts::REFERENCE_LEVEL, func: qwark_ship_parts::SHIELD_FN, classes: &qwark_ship_parts::SHIELD_CLASSES, update: qwark_ship_parts::shield_update, joints: &[] },
    UnitPort { unit: "388 missile 82", level: qwark_ship_parts::REFERENCE_LEVEL, func: qwark_ship_parts::MISSILE_FN, classes: &qwark_ship_parts::MISSILE_CLASSES, update: qwark_ship_parts::missile_update, joints: &qwark_ship_parts::MISSILE_CLASSES },
    UnitPort { unit: "388 mine 83", level: qwark_ship_parts::REFERENCE_LEVEL, func: qwark_ship_parts::MINE_FN, classes: &qwark_ship_parts::MINE_CLASSES, update: qwark_ship_parts::mine_update, joints: &[] },
    UnitPort { unit: "458 shell", level: turret_shell::REFERENCE_LEVEL, func: turret_shell::UPDATE_FN, classes: &turret_shell::CLASSES, update: turret_shell::update, joints: &[] },
    UnitPort { unit: "U407 1274", level: hoven_carrier::REFERENCE_LEVEL, func: hoven_carrier::UPDATE_FN, classes: &hoven_carrier::CLASSES, update: hoven_carrier::update, joints: &hoven_carrier::JOINTS },
    UnitPort { unit: "184 shot", level: gun_shot::REFERENCE_LEVEL, func: gun_shot::UPDATE_FN, classes: &gun_shot::CLASSES, update: gun_shot::update, joints: &[] },
    UnitPort { unit: "1009 laser", level: ship_laser::REFERENCE_LEVEL, func: ship_laser::UPDATE_FN, classes: &ship_laser::CLASSES, update: ship_laser::update, joints: &[] },
    UnitPort { unit: "295 missile", level: ship_missile::REFERENCE_LEVEL, func: ship_missile::UPDATE_FN, classes: &ship_missile::CLASSES, update: ship_missile::update, joints: &ship_missile::CLASSES },
    UnitPort { unit: "1034 missile", level: ship_missile::EARLY_LEVEL, func: ship_missile::EARLY_UPDATE_FN, classes: &ship_missile::EARLY_CLASSES, update: ship_missile::update, joints: &ship_missile::EARLY_CLASSES },
    UnitPort { unit: "U389 1319", level: ship_fighter::REFERENCE_LEVEL, func: ship_fighter::UPDATE_FN, classes: &ship_fighter::CLASSES, update: ship_fighter::update, joints: &[] },
    UnitPort { unit: "U389 1319 trails", level: ship_fighter::REFERENCE_LEVEL, func: ship_fighter::TRAIL_FN, classes: &[], update: empty::update, joints: &[] },
    UnitPort { unit: "U576 1843", level: ship_fighter::FLEET_LEVEL, func: ship_fighter::FLEET_UPDATE_FN, classes: &ship_fighter::FLEET_CLASSES, update: ship_fighter::update, joints: &[] },
    UnitPort { unit: "U576 1843 trails", level: ship_fighter::FLEET_LEVEL, func: ship_fighter::FLEET_TRAIL_FN, classes: &[], update: empty::update, joints: &[] },
    UnitPort { unit: "U433 224 228", level: ship_pickup_float::REFERENCE_LEVEL, func: ship_pickup_float::UPDATE_FN, classes: &ship_pickup_float::CLASSES, update: ship_pickup_float::update, joints: &ship_pickup_float::CLASSES },
    UnitPort { unit: "U568 1379", level: fleet_ship::REFERENCE_LEVEL, func: fleet_ship::UPDATE_FN, classes: &fleet_ship::CLASSES, update: fleet_ship::update, joints: &fleet_ship::CLASSES },
    UnitPort { unit: "U568 1379 hud", level: fleet_ship::REFERENCE_LEVEL, func: fleet_ship_hud::HUD_FN, classes: &[], update: empty::update, joints: &[] },
    UnitPort { unit: "U563 347", level: fleet_turret::REFERENCE_LEVEL, func: fleet_turret::UPDATE_FN, classes: &fleet_turret::CLASSES, update: fleet_turret::update, joints: &fleet_turret::CLASSES },
    UnitPort { unit: "U563 1368", level: fleet_turret::REFERENCE_LEVEL, func: fleet_turret::SHOT_FN, classes: &fleet_turret::SHOT_CLASSES, update: fleet_turret::shot_update, joints: &[] },
    UnitPort { unit: "U384 1242", level: pokitaru_jet::REFERENCE_LEVEL, func: pokitaru_jet::UPDATE_FN, classes: &pokitaru_jet::CLASSES, update: pokitaru_jet::update, joints: &pokitaru_jet::CLASSES },
    UnitPort { unit: "U384 1242 hud", level: pokitaru_jet::REFERENCE_LEVEL, func: pokitaru_jet_hud::HUD_FN, classes: &[], update: empty::update, joints: &[] },
    UnitPort { unit: "U387 1264", level: pokitaru_convoy::REFERENCE_LEVEL, func: pokitaru_convoy::UPDATE_FN, classes: &pokitaru_convoy::CLASSES, update: pokitaru_convoy::update, joints: &[] },
    UnitPort { unit: "U387 1265 car", level: pokitaru_convoy::REFERENCE_LEVEL, func: pokitaru_convoy::CAR_FN, classes: &pokitaru_convoy::CAR_CLASSES, update: pokitaru_convoy::car_update, joints: &[] },
    UnitPort { unit: "U387 1524 sludge", level: pokitaru_convoy::REFERENCE_LEVEL, func: pokitaru_convoy::SLUDGE_FN, classes: &pokitaru_convoy::SLUDGE_CLASSES, update: pokitaru_convoy::sludge_update, joints: &[] },
    UnitPort { unit: "1218 pickups", level: ship_pickup::REFERENCE_LEVEL, func: ship_pickup::UPDATE_FN, classes: &ship_pickup::CLASSES, update: ship_pickup::update, joints: &ship_pickup::CLASSES },
    UnitPort { unit: "1219 parachute", level: ship_pickup::REFERENCE_LEVEL, func: ship_pickup::CHUTE_FN, classes: &ship_pickup::CHUTE_CLASSES, update: ship_pickup::chute_update, joints: &ship_pickup::CHUTE_CLASSES },
    UnitPort { unit: "1017 shot", level: fighter_shot::REFERENCE_LEVEL, func: fighter_shot::UPDATE_FN, classes: &fighter_shot::CLASSES, update: fighter_shot::update, joints: &[] },
    UnitPort { unit: "U397 326", level: hoven_drone::REFERENCE_LEVEL, func: hoven_drone::UPDATE_FN, classes: &hoven_drone::CLASSES, update: hoven_drone::update, joints: &hoven_drone::JOINTS },
    UnitPort { unit: "U397 409", level: hoven_drone::REFERENCE_LEVEL, func: hoven_drone::SHOT_FN, classes: &hoven_drone::SHOT_CLASSES, update: hoven_drone::shot_update, joints: &[] },
    UnitPort { unit: "1371 rider", level: drone_rider::REFERENCE_LEVEL, func: drone_rider::UPDATE_FN, classes: &drone_rider::CLASSES, update: drone_rider::update, joints: &[] },
    UnitPort { unit: "U538 1455", level: kalebo_race::REFERENCE_LEVEL, func: kalebo_race::UPDATE_FN, classes: &kalebo_race::CLASSES, update: kalebo_race::update, joints: &kalebo_race::CLASSES },
];

/// Port indices as the `ClassUpdate::Unit` payload.
pub fn ids() -> impl Iterator<Item = u16> { 0..PORTS.len() as u16 }

/// `(*moby+0x74)(moby)` for unit port `i`.
pub fn update(w: &mut World, id: MobyId, i: u16) { (PORTS[i as usize].update)(w, id) }

/// The level data words the unit ports' class code reads and writes (a level's `$gp` / `.data` globals, e.g. the
/// rising floats' last-sound tick), by their address in the unit's reference overlay; reset with the level
/// (`Services::new`).
#[derive(Clone, Debug, Default)]
pub struct Globals {
    words: std::collections::HashMap<u32, u32>,
    /// Veldin's beam slots (level00 0x161bf8.., `veldin_beamer`).
    pub veldin_beams: veldin_beamer::Beams,
    /// Veldin's last level's cutscene effects (`veldin_finale_fx`: its level words and this tick's draws).
    pub veldin_finale: veldin_finale_fx::Fx,
    /// The pool meshes of level 18 (`veldin_pool`; read from the overlay by the engine at the level load).
    pub veldin_pools: Option<std::sync::Arc<veldin_pool::Meshes>>,
    /// Blarg's laser gates' shared beams (level06 0x1db0f0 / 0x1db870 / 0x161e40: `blarg_laser_gate`).
    pub blarg_gates: blarg_barrier::Beams,
    /// The gadgetbots' listener count (level06 0x17ec84: +1 for each bot in earshot of Clank's command menu each tick;
    /// the menu arms only when it is not 0; no reset found in the level code [L]): `blarg_gadgetbot`, `menus::quick_select`.
    pub bot_listeners: i32,
    /// Level 6's glass meshes (`blarg_glass`; read from the overlay by the engine at the level load).
    pub blarg_glass: Option<std::sync::Arc<blarg_glass::Meshes>>,
    /// The gadgetbot bubble mesh of levels 6 and 10 (`blarg_gadgetbot`; read from the overlay by the engine at the level load).
    pub blarg_bubble: Option<std::sync::Arc<blarg_gadgetbot::Bubble>>,
    /// The Snagglebeast's tongue and its draws' inputs (`umbris_beast_fx`).
    pub umbris_beast: umbris_beast_fx::Fx,
    /// Oltanis's and Quartu's three arc slots (`oltanis_arcs`, level14 0x162148.. / 0x1edd40..).
    pub oltanis_arcs: oltanis_arcs::Arcs,
    /// Oltanis's lightning strike (`oltanis_lightning`, level14 0x161cb4.. / 0x1dfd80..).
    pub oltanis_lightning: oltanis_lightning::Lightning,
    /// Oltanis's searchlight cones' colours and the draws' camera (`oltanis_sentry`, level14 0x1d80c8..).
    pub oltanis_sentry: oltanis_sentry::Cone,
    /// Quartu's and the Fleet's rippling water strips (`wave_mesh`; read from the overlay by the engine at the level load).
    pub wave_meshes: Option<std::sync::Arc<wave_mesh::Meshes>>,
    /// The electrified water's bolt (`water_shock`, level15 0x1d3750.. / 0x161cc0..).
    pub water_shock: water_shock::Bolt,
    /// The Veldin boss beam's strands (`veldin_shots`, level18 0x1d9d00.. / 0x161fe8..).
    pub veldin_strands: tesla_bolt::Bolt,
    /// The Fleet's sky's animation statics (`fleet_sky`, level17 0x1de040..).
    pub fleet_sky: fleet_sky::Sky,
    /// Paths a class emptied by zeroing their point count (the game's header word; Blarg's boss 1051 parks its arena
    /// during its cutaways): the points, to put back.
    pub parked_paths: std::collections::HashMap<usize, Vec<[u32; 4]>>,
}

impl Globals {
    /// The word at `addr` (0 until written: the words these ports use start at 0 in the data).
    pub fn word(&self, addr: u32) -> u32 { self.words.get(&addr).copied().unwrap_or(0) }
    pub fn set_word(&mut self, addr: u32, v: u32) { self.words.insert(addr, v); }
    /// The word at `addr`, `init` until written (a word the overlay data starts at another value: Qwark's −1).
    pub fn word_or(&self, addr: u32, init: u32) -> u32 { self.words.get(&addr).copied().unwrap_or(init) }
}

// ---------------------------------------------------------------------------------------------------
// Small shared reads the unit ports make (each the game's own global or class-header field).

/// Ratchet's position `0x13f3d0` as `f32`.
pub fn hero_pos(w: &World) -> [f32; 4] { w.hero.pos.map(|x| f32::from_bits(x.0)) }

/// The class scale (class header +0x24) of `o_class` as `f32`.
pub fn class_scale(w: &World, o_class: i16) -> f32 { crate::moby_update::services::fl(w.class_scale(o_class)) }

/// Whether the class header of `o_class` has a collision blob (class +0x10: what the game stores into moby +0x94 to
/// turn the moby's collision back on).
pub fn class_collision(w: &World, o_class: i16) -> bool { w.classes.info(o_class).is_some_and(|i| i.has_collision) }

/// `0x277a00(amp, rate, moby, &phase, &prev)`: a vertical bob: phase += rate (`fast_add_rotations`), z −= prev,
/// prev = amp·sin(phase), z += prev. `phase` / `prev` are pvar offsets.
pub fn bob(w: &mut World, id: MobyId, amp: f32, rate: f32, phase: usize, prev: usize) {
    use crate::moby_update::creature as c;
    let a = c::add_rot(c::pf(w, id, phase), rate);
    c::set_pf(w, id, phase, a);
    let old = c::pf(w, id, prev);
    let s = a.sin() * amp;
    c::set_pf(w, id, prev, s);
    let m = w.mm(id);
    m.position[2] = (m.position[2] - old) + s;
}

/// `0x277a80(amp, rate_a, rate_b, moby, &a, &b)`: a tilt wobble: rot.x = amp·sin a·sin b, rot.y = amp·sin a·cos b,
/// then a += rate_a, b += rate_b (`fast_add_rotations`). `a` / `b` are pvar offsets. (The mines' copy of it is inline
/// in `classes::mine`.)
pub fn wobble(w: &mut World, id: MobyId, amp: f32, rate_a: f32, rate_b: f32, pa: usize, pb: usize) {
    use crate::moby_update::creature as c;
    let (a, b) = (c::pf(w, id, pa), c::pf(w, id, pb));
    let m = w.mm(id);
    m.rotation[0] = amp * a.sin() * b.sin();
    m.rotation[1] = amp * a.sin() * b.cos();
    c::set_pf(w, id, pa, c::add_rot(a, rate_a));
    c::set_pf(w, id, pb, c::add_rot(b, rate_b));
}

/// `FUN_00272078(moby)`: Ratchet's moby's light word and ambient (+0x38..+0x3f) copied onto `id`.
pub fn take_hero_light(w: &mut World, id: MobyId) {
    if let Some(h) = w.hero_moby {
        let (l, a) = (w.m(h).light, w.m(h).ambient);
        let m = w.mm(id);
        m.light = l;
        m.ambient = a;
    }
}

// ---------------------------------------------------------------------------------------------------
// Draw callbacks and the registry lookups of the unit ports.

/// The row of the unit whose reference update is `func` of level `level` (as the `Callback::UnitGlow` payload).
pub fn row(level: u32, func: u32) -> Option<u16> { PORTS.iter().position(|u| (u.level, u.func) == (level, func)).map(|i| i as u16) }

/// One call of the glow quad `0x2781d0(size, pull, point, rgba)` (`rc-engine` `fx_draw::glow_quad`).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct GlowQuad {
    pub size: f32,
    /// Toward the camera.
    pub pull: f32,
    pub point: [f32; 3],
    /// GS RGBA (R low).
    pub rgba: u32,
}

/// One `FastDrawQuadReal` quad of a unit's draw callback, in world space: corners in GS strip order, ST, GS RGBA.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct FxQuad {
    pub corners: [[f32; 3]; 4],
    pub st: [[f32; 2]; 4],
    pub rgba: [u32; 4],
}

/// A unit draw callback's quads with their FX texture and blend (ALPHA 0x48 additive, else 0x44).
#[derive(Clone, Debug, PartialEq)]
pub struct FxQuads {
    pub fx: usize,
    pub additive: bool,
    /// The colour taken off the frame (`(Cd − Cs)·FIX`, FIX 0x80: the vertex alpha 0x80); over `additive`.
    pub subtract: bool,
    pub quads: Vec<FxQuad>,
}

/// The quads of the draw callback unit row `i` registered for moby `id` (`Callback::UnitQuads(i)`; draw only).
pub fn fx_quads(table: &crate::moby_runtime::MobyTable, svc: &crate::moby_update::Services, i: u16, id: MobyId) -> Option<FxQuads> {
    match PORTS.get(i as usize).map(|u| (u.level, u.func)) {
        Some((1, crate::moby_update::classes::teleporter::BEAM_FN)) => crate::moby_update::classes::teleporter::beam_quads(table, svc, id),
        Some((pokitaru_teleporter::REFERENCE_LEVEL, pokitaru_teleporter::BEAM_FN)) => pokitaru_teleporter::beam_quads(table, svc, id),
        Some((laser_fence::REFERENCE_LEVEL, laser_fence::UPDATE_FN)) => laser_fence::fx_quads(table, svc, id),
        Some((hoven_fall::REFERENCE_LEVEL, hoven_fall::DRAW_FN)) => hoven_fall::fx_quads(table, svc, id),
        Some((hoven_arc::REFERENCE_LEVEL, hoven_arc::DRAW_FN)) => hoven_arc::fx_quads(table, svc, id),
        Some((hoven_beam::REFERENCE_LEVEL, hoven_beam::DRAW_FN)) => hoven_beam::fx_quads(table, svc, id),
        Some((oltanis_zapper::REFERENCE_LEVEL, oltanis_zapper::PIECE_DRAW_FN)) => oltanis_zapper::piece_quads(table, svc, id),
        Some((oltanis_sentry::REFERENCE_LEVEL, oltanis_sentry::SPOT_FN)) => oltanis_sentry::spot_quads(table, svc, id),
        Some((l, f)) if wave_mesh::DEFS.iter().any(|d| d.level == l && d.draw == f) => wave_mesh::fx_quads(table, svc, l, f, id),
        Some((oltanis_bolt::REFERENCE_LEVEL, oltanis_bolt::DRAW_FN)) => oltanis_bolt::fx_quads(table, svc, id),
        Some((oltanis_wind::REFERENCE_LEVEL, oltanis_wind::DRAW_FN)) => oltanis_wind::fx_quads(table, svc, id),
        Some((gemlik_robot::REFERENCE_LEVEL, gemlik_robot::DRAW_FN)) => gemlik_robot::fx_quads(table, svc, id),
        Some((hover_zapper::REFERENCE_LEVEL, hover_zapper::ARC_FN)) => hover_zapper::fx_quads(table, svc, id),
        Some((kalebo_lift::REFERENCE_LEVEL, kalebo_lift::GLOW_FN)) => kalebo_lift::fx_quads(table, svc, id),
        Some((rilgar_trail_rider::REFERENCE_LEVEL, rilgar_trail_rider::TRAIL_FN)) => rilgar_trail_rider::fx_quads(table, svc, id),
        Some((quartu_drone::REFERENCE_LEVEL, quartu_drone::UPDATE_FN)) => quartu_drone::fx_quads(table, svc, id),
        Some((swing_laser::REFERENCE_LEVEL, swing_laser::UPDATE_FN)) => swing_laser::fx_quads(table, svc, id),
        Some((kalebo_barrier::REFERENCE_LEVEL, kalebo_barrier::UPDATE_FN)) => kalebo_barrier::fx_quads(table, svc, id),
        Some((sweep_light::REFERENCE_LEVEL, sweep_light::UPDATE_FN)) => sweep_light::fx_quads(table, svc, id),
        Some((light_fixture::REFERENCE_LEVEL, light_fixture::UPDATE_FN)) => light_fixture::fx_quads(table, svc, id),
        Some((water_laser::REFERENCE_LEVEL, water_laser::UPDATE_FN)) => water_laser::fx_quads(table, svc, id),
        Some((kalebo_belt::REFERENCE_LEVEL, kalebo_belt::UPDATE_FN)) => kalebo_belt::fx_quads(table, svc, id, false),
        Some((kalebo_belt::REFERENCE_LEVEL, kalebo_belt::DRAW_FN)) => kalebo_belt::fx_quads(table, svc, id, true),
        Some((fleet_laser::REFERENCE_LEVEL, fleet_laser::UPDATE_FN)) => fleet_laser::fx_quads(table, svc, id),
        Some((veldin_diver::REFERENCE_LEVEL, veldin_diver::TRAIL_FN)) => veldin_diver::fx_quads(table, svc, id),
        Some((veldin_lock_field::REFERENCE_LEVEL, veldin_lock_field::DRAW_FN)) => veldin_lock_field::fx_quads(table, svc, id),
        Some((veldin_pool::REFERENCE_LEVEL, f)) if veldin_pool::DRAW_FNS.contains(&f) => veldin_pool::fx_quads(table, svc, id),
        Some((veldin_shots::REFERENCE_LEVEL, veldin_shots::LOB_MARK_FN)) => veldin_shots::lob_quads(table, svc, id),
        Some((veldin_shots::REFERENCE_LEVEL, veldin_shots::RING_FN)) => veldin_shots::ring_quads(table, svc, id),
        Some((veldin_shots::REFERENCE_LEVEL, veldin_shots::AURA_DRAW_FN)) => veldin_shots::aura_quads(table, svc, id),
        Some((veldin_hopper::REFERENCE_LEVEL, veldin_hopper::BEAM_FN)) => veldin_hopper::beam_quads(table, svc, id),
        Some((aridia_lift::REFERENCE_LEVEL, aridia_lift::DRAW_FN)) => aridia_lift::fx_quads(table, svc, id),
        Some((qwark_ship::REFERENCE_LEVEL, qwark_ship::BEAM_FN)) => qwark_ship::beam_quads(table, svc, id),
        Some((blarg_barrier::REFERENCE_LEVEL, blarg_barrier::DRAW_FN)) => blarg_barrier::fx_quads(table, svc, id),
        Some((blarg_laser_gate::REFERENCE_LEVEL, blarg_laser_gate::DRAW_FN)) => blarg_laser_gate::fx_quads(table, svc, id),
        Some((blarg_gadgetbot::REFERENCE_LEVEL, blarg_gadgetbot::BUBBLE_DRAW_FN)) => blarg_gadgetbot::bubble_quads(table, svc, id),
        Some((blarg_wave_bot::REFERENCE_LEVEL, blarg_wave_bot::DRAW_FN)) => blarg_wave_bot::fx_quads(table, svc, id),
        Some((orxon_curtain::REFERENCE_LEVEL, orxon_curtain::DRAW_FN)) => orxon_curtain::fx_quads(table, svc, id),
        Some((batalia_flame::REFERENCE_LEVEL, batalia_flame::DRAW_FN)) => batalia_flame::fx_quads(table, svc, id),
        Some((ship_fighter::REFERENCE_LEVEL, ship_fighter::TRAIL_FN)) | Some((ship_fighter::FLEET_LEVEL, ship_fighter::FLEET_TRAIL_FN)) => ship_fighter::trail_quads(table, svc, id),
        _ => None,
    }
}

/// The quad groups of the draw callback unit row `i` registered for moby `id` (`Callback::UnitQuads(i)`): the rows
/// whose callback draws more than one texture or blend (1471's beams), else [`fx_quads`]'s one group.
pub fn fx_quad_groups(table: &crate::moby_runtime::MobyTable, svc: &crate::moby_update::Services, i: u16, id: MobyId) -> Vec<FxQuads> {
    match PORTS.get(i as usize).map(|u| (u.level, u.func)) {
        Some((veldin_beamer::REFERENCE_LEVEL, veldin_beamer::BEAM_FN)) => veldin_beamer::fx_quad_groups(svc),
        Some((umbris_beast_fx::REFERENCE_LEVEL, f)) if umbris_beast_fx::DRAW_FNS.contains(&f) => umbris_beast_fx::quad_groups(table, svc, f, id),
        Some((energy_fan::REFERENCE_LEVEL, energy_fan::DRAW_FN)) => energy_fan::fx_quad_groups(table, svc, id),
        Some((blarg_glass::REFERENCE_LEVEL, f)) if blarg_glass::DRAW_FNS.contains(&f) => blarg_glass::fx_quad_groups(table, svc, f),
        Some((blarg_bot_pad::REFERENCE_LEVEL, blarg_bot_pad::DRAW_FN)) => blarg_bot_pad::fx_quad_groups(table, svc, id),
        Some((orxon_airlock::REFERENCE_LEVEL, orxon_airlock::DRAW_FN)) => orxon_airlock::fx_quad_groups(table, svc, id),
        Some((oltanis_arc::REFERENCE_LEVEL, oltanis_arc::DRAW_FN)) => oltanis_arc::fx_quad_groups(table, svc, id),
        Some((oltanis_arcs::REFERENCE_LEVEL, oltanis_arcs::DRAW_FN)) => oltanis_arcs::fx_quad_groups(table, svc, id),
        Some((oltanis_lightning::REFERENCE_LEVEL, oltanis_lightning::DRAW_FN)) => oltanis_lightning::fx_quad_groups(table, svc, id),
        Some((oltanis_zapper::REFERENCE_LEVEL, oltanis_zapper::DRAW_FN)) => oltanis_zapper::fx_quad_groups(table, svc, id),
        Some((oltanis_sentry::REFERENCE_LEVEL, oltanis_sentry::MASTER_DRAW_FN)) => oltanis_sentry::fx_quad_groups(table, svc, id),
        Some((barrier_field::REFERENCE_LEVEL, barrier_field::DRAW_FN)) => barrier_field::fx_quad_groups(table, svc, id),
        Some((water_shock::REFERENCE_LEVEL, water_shock::DRAW_FN)) => water_shock::fx_quad_groups(table, svc, id),
        Some((oltanis_rail_bot::REFERENCE_LEVEL, oltanis_rail_bot::DRAW_FN)) => oltanis_rail_bot::fx_quad_groups(table, svc, id),
        Some((fleet_sky::REFERENCE_LEVEL, fleet_sky::DRAW_FN)) => fleet_sky::fx_quad_groups(table, svc, id),
        Some((1, crate::shadows::BLOB_FN)) => {
            let quads = svc.blobs.1.iter().filter(|b| b.0 == id).map(|(_, b)| {
                let (corners, st) = crate::shadows::blob_quad(b);
                FxQuad { corners, st, rgba: [0x4080_8080; 4] }
            }).collect();
            vec![FxQuads { fx: 0, additive: false, subtract: false, quads }]
        }
        Some((super::burning_wreck::REFERENCE_LEVEL, super::burning_wreck::DRAW_FN)) => super::burning_wreck::fx_quads(table, svc, id).into_iter().collect(),
        Some((veldin_shots::REFERENCE_LEVEL, veldin_shots::CORE_DRAW_FN)) => veldin_shots::core_quads(table, svc, id),
        Some((veldin_shots::REFERENCE_LEVEL, veldin_shots::STRAND_DRAW_FN)) => veldin_shots::strand_quads(table, svc, id),
        Some((veldin_finale_fx::REFERENCE_LEVEL, f)) if [veldin_finale_fx::FLASH_FN, veldin_finale_fx::GLOW_FN, veldin_finale_fx::BEAM_FN, veldin_finale_fx::MORPH_FN].contains(&f) => veldin_finale_fx::fx_quad_groups(svc, f),
        _ => fx_quads(table, svc, i, id).into_iter().collect(),
    }
}

/// The glow quads of the draw callback unit row `i` registered for moby `id` (`Callback::UnitGlow(i)`; draw only).
pub fn glow_quads(table: &crate::moby_runtime::MobyTable, svc: &crate::moby_update::Services, i: u16, id: MobyId) -> Vec<GlowQuad> {
    match PORTS.get(i as usize).map(|u| (u.level, u.func)) {
        Some((lamp::REFERENCE_LEVEL, lamp::UPDATE_FN)) => lamp::glow_quads(table, svc, id),
        Some((blarg_gadgetbot::REFERENCE_LEVEL, blarg_gadgetbot::DRAW_FN)) => blarg_gadgetbot::glow_quads(table, svc, id),
        Some((hover_zapper::REFERENCE_LEVEL, hover_zapper::GLOW_FN)) => hover_zapper::glow_quads(table, id),
        Some((veldin_diver::REFERENCE_LEVEL, veldin_diver::UPDATE_FN)) => veldin_diver::glow_quads(table, svc, id),
        Some((veldin_boss::REFERENCE_LEVEL, veldin_boss::DRAW_FN)) => veldin_boss::glow_quads(table, svc, id),
        Some((veldin_hopper::REFERENCE_LEVEL, veldin_hopper::GLOW_FN)) => veldin_hopper::glow_quads(table, svc, id),
        Some((path_ship::REFERENCE_LEVEL, path_ship::UPDATE_FN | path_ship::BIG_FN)) => path_ship::glow_quads(table, svc, id),
        Some((veldin_beamer::REFERENCE_LEVEL, veldin_beamer::EYE_FN)) => veldin_beamer::glow_quads(table, svc, id),
        Some((veldin_finale_fx::REFERENCE_LEVEL, veldin_finale_fx::SEAT_FN)) => veldin_finale_fx::glow_quads(table, svc),
        Some((super::mouse::REFERENCE_LEVEL, super::mouse::GLOW_FN)) => super::mouse::glow_quads(table, id),
        Some((kalebo_trooper::REFERENCE_LEVEL, kalebo_trooper::GLOW_FN)) => kalebo_trooper::glow_quads(table, svc, id),
        Some((rilgar_trail_rider::REFERENCE_LEVEL, rilgar_trail_rider::GLOW_FN)) => rilgar_trail_rider::glow_quads(table, svc, id),
        Some((quartu_guard::REFERENCE_LEVEL, quartu_guard::DRAW_FN)) => quartu_guard::glow_quads(table, svc, id),
        Some((fleet_crew::REFERENCE_LEVEL, fleet_crew::DRAW_FN)) => fleet_crew::glow_quads(table, svc, id),
        _ => Vec::new(),
    }
}

/// The state part of the unit draw callback row `i` run for moby `id` by the frame's callbacks
/// (`Callback::UnitFrame(i)`, `draw_callbacks::run_frame`): its `rand` draws and what it leaves for the renderer.
pub fn frame_callback(w: &mut World, i: u16, id: MobyId) {
    match PORTS.get(i as usize).map(|u| (u.level, u.func)) {
        Some((veldin_pads::REFERENCE_LEVEL, veldin_pads::COUNTDOWN_FN)) => veldin_pads::countdown_draw(w, id),
        Some((blarg_escape::REFERENCE_LEVEL, blarg_escape::DRAW_FN)) => blarg_escape::frame(w, id),
        Some((blarg_wave_bot::REFERENCE_LEVEL, blarg_wave_bot::DRAW_FN)) => blarg_wave_bot::frame(w, id),
        Some((hoven_turret::REFERENCE_LEVEL, hoven_turret::HUD_FN)) => hoven_turret::hud_frame(w, id),
        Some((gemlik_ship::REFERENCE_LEVEL, gemlik_ship_hud::HUD_FN)) => gemlik_ship_hud::hud_frame(w, id),
        Some((pokitaru_jet::REFERENCE_LEVEL, pokitaru_jet_hud::HUD_FN)) => pokitaru_jet_hud::hud_frame(w, id),
        Some((fleet_ship::REFERENCE_LEVEL, fleet_ship_hud::HUD_FN)) => fleet_ship_hud::hud_frame(w, id),
        Some((veldin_beamer::REFERENCE_LEVEL, veldin_beamer::BEAM_FN)) => veldin_beamer::frame(w, id),
        Some((batalia_turret::REFERENCE_LEVEL, batalia_turret::HUD_FN)) => batalia_turret::hud_frame(w, id),
        Some((batalia_flame::REFERENCE_LEVEL, batalia_flame::DRAW_FN)) => batalia_flame::frame(w, id),
        Some((hoven_beam::REFERENCE_LEVEL, hoven_beam::DRAW_FN)) => hoven_beam::frame(w, id),
        Some((gemlik_robot::REFERENCE_LEVEL, gemlik_robot::DRAW_FN)) => gemlik_robot::frame(w, id),
        Some((oltanis_arc::REFERENCE_LEVEL, oltanis_arc::DRAW_FN)) => oltanis_arc::frame(w, id),
        Some((oltanis_arcs::REFERENCE_LEVEL, oltanis_arcs::DRAW_FN)) => oltanis_arcs::frame(w, id),
        Some((oltanis_lightning::REFERENCE_LEVEL, oltanis_lightning::DRAW_FN)) => oltanis_lightning::frame(w, id),
        Some((oltanis_zapper::REFERENCE_LEVEL, oltanis_zapper::DRAW_FN)) => oltanis_zapper::frame(w, id),
        Some((oltanis_sentry::REFERENCE_LEVEL, oltanis_sentry::MASTER_DRAW_FN | oltanis_sentry::SPOT_FN)) => oltanis_sentry::frame(w, id),
        Some((fleet_sky::REFERENCE_LEVEL, fleet_sky::DRAW_FN)) => fleet_sky::frame(w, id),
        Some((water_shock::REFERENCE_LEVEL, water_shock::DRAW_FN | water_shock::COUNTDOWN_FN)) => water_shock::frame(w, id),
        Some((veldin_shots::REFERENCE_LEVEL, veldin_shots::STRAND_DRAW_FN)) => veldin_shots::strands_frame(w, id),
        Some((oltanis_rail_bot::REFERENCE_LEVEL, oltanis_rail_bot::DRAW_FN)) => oltanis_rail_bot::frame(w, id),
        Some((oltanis_bolt::REFERENCE_LEVEL, oltanis_bolt::DRAW_FN)) => oltanis_bolt::frame(w, id),
        Some((oltanis_wind::REFERENCE_LEVEL, oltanis_wind::DRAW_FN)) => oltanis_wind::frame(w, id),
        Some((oltanis_zapper::REFERENCE_LEVEL, oltanis_zapper::PIECE_DRAW_FN)) => oltanis_zapper::piece_frame(w, id),
        Some((umbris_beast_fx::REFERENCE_LEVEL, f @ (umbris_beast_fx::BEAM_FN | umbris_beast_fx::GROUND_FN))) => umbris_beast_fx::frame(w, f, id),
        _ => {}
    }
}
