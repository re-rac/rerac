//! The ported class updates and the registry that maps level-table addresses to them.
//!
//! | classes | level01 fn | port |
//! |---|---|---|
//! | 13, 14, 15, 16 (bolts) | 0x2bb758 `BoltUpdate` | [`bolt`] |
//! | 500, 501, 502, 505, 511 (crates) | 0x2ea178 `CrateUpdate` | [`crate_`] |
//! | 724, 725 (grass) | 0x2fa720 `GrassUpdate` | [`grass`] |
//! | 660 (Blarg flyers) | 0x2f4428 `BlargFlyerUpdate` + 0x2f5168 `FlyerPathDriver` | [`flyer`] |
//! | 1135 (teleporter pads) | 0x308bd8 `TeleporterPadUpdate` | [`teleporter`] |
//! | 304, 1456–1465 (gold-weapon offers) | 0x2e1ac0 `ItemOfferUpdate` | [`item_offer`] |
//! | 149, 348, 349, 354–359, 363, 364 (debris pieces) | 0x2c5218 `DebrisUpdate` | [`debris`] |
//! | 112, 1192 (explosion flashes) | 0x2c22a8 `FlashUpdate` | [`debris`] |
//! | 726 (path platform / lift) | 0x2b9eb0 `PathPlatformUpdate` | [`path_platform`] |
//! | 805 (checkpoint triggers) | 0x300220 `CheckpointTriggerUpdate` | [`checkpoint`] |
//! | 679 (flow chutes; levels 1, 5, 8, 15) | 0x2f6328 `FlowUpdate` | [`flow`] |
//! | 758, 803 (Swingshot pull / swing targets; not on level01) | level03 0x2d0bd8 | [`swing_target`] |
//! | 577 (critters; level 1) | 0x2efc60 `GroundCritterUpdate` | [`critter`] |
//! | 572, 865, 866 (amoeboids; levels 1, 5, 11) | 0x2edca0 `AmoeboidUpdate` | [`amoeboid`] |
//! | 1736–1738, 1747–1749, 1761–1763, 1770, 1814, 1815, 1817 (body pieces; level 1) | 0x30cd18 `FxGroupUpdate` | [`crate::moby_update::creature::fx`] |
//! | 639 (explosion light; every level) | 0x2f3748 | [`crate::moby_update::creature::fx`] |
//! | 121 (Bomb Glove bomb), 122 (its fireballs) | 0x2c3300, 0x2c4d88 | [`bomb`] |
//! | 280 (bolt cranks; levels 1, 4, 8), 641 / 665 (the Novalis rotator / sliders they drive) | 0x2e0c68, 0x2f4348, 0x2f4710 | [`bolt_crank`] |
//! | 459 (robot troopers; level 1), 722 (their fire globs) | 0x2e6bf0 `PathEnemyUpdate`, 0x2fa4c0 | [`path_enemy`] |
//! | 666 (dropship; level 1) | 0x2f4960 `DropshipUpdate` | [`dropship`] |
//! | 688 (gunship), 686 (its shells), 700 (fires), 696–698 (embers; level 1) | 0x2f7728, 0x2f6a30, 0x2f8c58, 0x2f8718 | [`gunship`] |
//! | 815 (enemy spawner; level 1) | 0x3021b8 `EnemySpawnerUpdate` | [`enemy_spawner`] |
//! | 11 (Gadgetron vendor; every level) | 0x2bb128 | [`vendor`] |
//! | 774 (talking NPC; levels 1, 8) | 0x2ff118 `TalkingNpcUpdate` | [`talking_npc`] |
//! | 760 (fire / smoke fields on the bombed buildings), 809 (their smoke scroll; levels 0, 1, 14) | 0x2fdbc0 `ParticleFieldUpdate`, 0x2ba658 | [`fire_field`] (draw callbacks: [`draw_callbacks`]) |
//! | 705 (spinners), 703 / 715 (elevators), 768 / 769 (sliding doors), 1042 (shootables), 701 (collapsing platforms; level 1) | 0x2f9bf0, 0x2f95c0, 0x2fed68, 0x307c48, 0x2f9080 | [`props`] |
//! | 1007 (the camera moby; created by the camera system on every level) | 0x2ba7c8 | [`crate::follow_camera::camera_moby`] |
//! | 704 (rocks), 709–711 (shell walls), 729 (big wall), 778 (pipe) and its spray 779 (level 1) | 0x2f9810, 0x2f9d80, 0x2fa800, 0x2ff860, 0x2ffb28 | [`breakables::novalis`] |
//! | the break template's 21 copies (754, 1813 on level 1; 22 classes on levels 1–4, 6–12, 16, 17), 1816 (remains) | [`breakables::RECIPES`], 0x30d200 | [`breakables`] (`ClassUpdate::Breakable(i)`) |
//! | 1546 (cutscene FX driver; levels with scene effects) | 0x30c190 `CutsceneFxUpdate` | [`cutscene_fx`] |
//! | 204, 213, 214, 222, 223, 225, 226, 1006, 1438, 1447, 1449 (ammo pickups), 806 (nanotech cluster; every level) | 0x2db028, 0x300de0 | [`pickup`] |
//! | 775 (water splash; spawned by `0x2ff768`) | 0x2ff810 | [`splash`] |
//! | 737 (camera triggers; levels 1, 2, 3, 7, 10, 13), 730 / 790 (mission NPCs), 746 (hinged bridge; level 1) | 0x2fb5b0, 0x2fad68, 0x2fb8a8 | [`camera_trigger`], [`mission_npc`], [`hinged_bridge`] |
//! | 1134 (gold bolts; every level with one) | 0x307ca0 `GoldBoltUpdate` | [`gold_bolt`] |
//! | 750 (infobots; levels 0, 1, 3–8, 10, 12–15, 17) | 0x2fbf80 `InfobotUpdate` | [`infobot`] |
//! | 179 (the Pyrocitor's pilot flame; created by the hand item) | 0x2d1068 | [`pyro_glow`] |
//! | 305 (the Blaster's shot; created by the hand item) | 0x2e2170 | [`blaster_shot`] |
//! | 153 (the Devastator's missile; created by the hand item) | 0x2c5b70 | [`devastator_missile`] |
//! | 167 (the Thruster-Pack's flames; created with the pack; every level) | 0x2c9e00 | [`thruster_flame`] |
//! | 457 (the R.Y.N.O.'s missile; created by the hand item) | 0x2e5a48 | [`ryno_missile`] (the intercept shared with 153: [`missile`]) |
//! | 605 (buried bolt caches, the Metal Detector's targets; levels 1–18) | 0x2f2eb8 | [`buried_bolts`] |
//! | 832 (Visibomb range limiters; levels 1–15, 18) | 0x302648 | [`rc_range`] |
//! | 172 (the Visibomb's missile; launched by the hand item; every level) | 0x2cbda8 | [`visibomb`] |
//! | 270 (the Morph-o-Ray's chicken), 428 (its feathers); every level | 0x2df448, 0x2e2f68 | [`chicken`] |
//! | 203, 1900 (the Decoy Glove's decoys; created by the hand item; every level) | 0x2d9fe8 | [`decoy`] |
//! | 74 (the Mine Glove's mines; created by the hand item; every level) | 0x2bfe40 | [`mine`] |
//! | 479 (the Drone Device's drones; launched from the hand swap; every level) | 0x2e92b8 | [`drone`] |
//! | 230 (the Glove of Doom's canister; created by the hand item), 186 (its bots); every level | 0x2de650, 0x2d5c10 | [`doom_canister`], [`doom_bot`] |
//! | 604 / 1818 / 1633 (the Sonic Summoner's house, "mouse" and its shot; levels 1–6, 8, 11, 12, 14) | 0x2f2b68, 0x30df40, 0x30c9a8 | [`mouse`] |
//! | 258 (activation zones; levels 5, 7, 9, 10, 12–15, 17) | level05 0x2f5200 | [`activation_zone`] |
//! | 830 (floor switches; levels 5, 11, 15, 17, 18) | level05 0x30c5b0 | [`floor_switch`] |
//! | the census class-port units (G-CLS-027; one row per unit, per level) | per unit | [`units`] (`ClassUpdate::Unit(i)`) |
//! | 994, 879, 460, 1018, 327, 317, 1260, 1111, 1901 (the seas and liquid surfaces; levels 3, 5, 7–9, 11, 12, 14, 16) | per level | `crate::water::sea` (`ClassUpdate::Sea(i)`) |

pub mod activation_zone;
pub mod amoeboid;
pub mod bolt;
pub mod bolt_crank;
pub mod bomb;
pub mod burning_wreck;
pub mod blaster_shot;
pub mod bomb_water;
pub mod buried_bolts;
pub mod breakables;
pub mod camera_trigger;
pub mod checkpoint;
pub mod chicken;
pub mod crate_;
pub mod critter;
pub mod cutscene_fx;
pub mod debris;
pub mod decoy;
pub mod drone;
pub mod doom_bot;
pub mod doom_canister;
pub mod mine;
pub mod devastator_missile;
pub mod draw_callbacks;
pub mod dropship;
pub mod enemy_spawner;
pub mod flow;
pub mod flyer;
pub mod fire_field;
pub mod floor_switch;
pub mod gold_bolt;
pub mod grass;
pub mod hinged_bridge;
pub mod infobot;
pub mod gunship;
pub mod item_offer;
pub mod missile;
pub mod mission_npc;
pub mod mouse;
pub mod path_enemy;
pub mod path_platform;
pub mod pickup;
pub mod props;
pub mod pyro_glow;
pub mod rc_range;
pub mod ryno_missile;
pub mod splash;
pub mod swing_target;
pub mod talking_npc;
pub mod teleporter;
pub mod thruster_flame;
pub mod units;
pub mod vendor;
pub mod visibomb;

use crate::moby_runtime::MobyId;
use crate::moby_update::services::World;

/// A ported update function (the value of `moby+0x74`).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ClassUpdate {
    Bolt,
    Crate,
    Grass,
    Flyer,
    TeleporterPad,
    ItemOffer,
    Debris,
    Flash,
    PathPlatform,
    Checkpoint,
    Flow,
    SwingTarget,
    Critter,
    FxPiece,
    ExplosionLight,
    Amoeboid,
    Bomb,
    Fireball,
    BoltCrank,
    CrankRotator,
    CrankSlider,
    PathEnemy,
    PathEnemyShot,
    Dropship,
    Gunship,
    GunshipShell,
    GunshipFire,
    GunshipEmber,
    EnemySpawner,
    Vendor,
    TalkingNpc,
    FireField,
    TextureScroll,
    Spinner,
    Elevator,
    SlidingDoor,
    Shootable,
    Collapse,
    CameraMoby,
    Rock,
    ShellWall,
    Wall,
    Pipe,
    Spray,
    Remains,
    CameraTrigger,
    MissionNpc,
    HingedBridge,
    CutsceneFx,
    AmmoPickup,
    Nanotech,
    Splash,
    GoldBolt,
    Infobot,
    PyroGlow,
    BuriedBolts,
    RcRange,
    Visibomb,
    MouseHouse,
    Mouse,
    MouseShot,
    ActivationZone,
    FloorSwitch,
    BlasterShot,
    RynoMissile,
    DevastatorMissile,
    ThrusterFlame,
    Chicken,
    Feather,
    Decoy,
    Mine,
    Drone,
    DoomCanister,
    DoomBot,
    /// The player's ship 531 / 532 / 533 (`ShipUpdate` 0x2a1c40: the loader installs it; `crate::travel::ship`).
    Ship,
    /// A copy of the break template: `breakables::RECIPES[i]`.
    Breakable(u8),
    /// A water class: `crate::water::managers::PORTS[i]` (the ripple managers, the water plane).
    Water(u8),
    /// A sea / liquid surface class: `crate::water::sea::PORTS[i]`.
    Sea(u8),
    /// A census class-port unit (G-CLS-027): `units::PORTS[i]`.
    Unit(u16),
}

impl ClassUpdate {
    pub const ALL: [ClassUpdate; 75] = [
        ClassUpdate::Bolt,
        ClassUpdate::Crate,
        ClassUpdate::Grass,
        ClassUpdate::Flyer,
        ClassUpdate::TeleporterPad,
        ClassUpdate::ItemOffer,
        ClassUpdate::Debris,
        ClassUpdate::Flash,
        ClassUpdate::PathPlatform,
        ClassUpdate::Checkpoint,
        ClassUpdate::Flow,
        ClassUpdate::SwingTarget,
        ClassUpdate::Critter,
        ClassUpdate::FxPiece,
        ClassUpdate::ExplosionLight,
        ClassUpdate::Amoeboid,
        ClassUpdate::Bomb,
        ClassUpdate::Fireball,
        ClassUpdate::BoltCrank,
        ClassUpdate::CrankRotator,
        ClassUpdate::CrankSlider,
        ClassUpdate::PathEnemy,
        ClassUpdate::PathEnemyShot,
        ClassUpdate::Dropship,
        ClassUpdate::Gunship,
        ClassUpdate::GunshipShell,
        ClassUpdate::GunshipFire,
        ClassUpdate::GunshipEmber,
        ClassUpdate::EnemySpawner,
        ClassUpdate::Vendor,
        ClassUpdate::TalkingNpc,
        ClassUpdate::FireField,
        ClassUpdate::TextureScroll,
        ClassUpdate::Spinner,
        ClassUpdate::Elevator,
        ClassUpdate::SlidingDoor,
        ClassUpdate::Shootable,
        ClassUpdate::Collapse,
        ClassUpdate::CameraMoby,
        ClassUpdate::Rock,
        ClassUpdate::ShellWall,
        ClassUpdate::Wall,
        ClassUpdate::Pipe,
        ClassUpdate::Spray,
        ClassUpdate::Remains,
        ClassUpdate::CameraTrigger,
        ClassUpdate::MissionNpc,
        ClassUpdate::HingedBridge,
        ClassUpdate::CutsceneFx,
        ClassUpdate::AmmoPickup,
        ClassUpdate::Nanotech,
        ClassUpdate::Splash,
        ClassUpdate::GoldBolt,
        ClassUpdate::Infobot,
        ClassUpdate::PyroGlow,
        ClassUpdate::BuriedBolts,
        ClassUpdate::RcRange,
        ClassUpdate::Visibomb,
        ClassUpdate::MouseHouse,
        ClassUpdate::Mouse,
        ClassUpdate::MouseShot,
        ClassUpdate::ActivationZone,
        ClassUpdate::FloorSwitch,
        ClassUpdate::BlasterShot,
        ClassUpdate::RynoMissile,
        ClassUpdate::DevastatorMissile,
        ClassUpdate::ThrusterFlame,
        ClassUpdate::Chicken,
        ClassUpdate::Feather,
        ClassUpdate::Decoy,
        ClassUpdate::Mine,
        ClassUpdate::Drone,
        ClassUpdate::DoomCanister,
        ClassUpdate::DoomBot,
        ClassUpdate::Ship,
    ];

    /// The level01 class-table address of this update.
    pub const fn address(self) -> u32 {
        match self {
            ClassUpdate::Bomb => bomb::UPDATE_FN,
            ClassUpdate::Fireball => bomb::FIREBALL_UPDATE_FN,
            ClassUpdate::BoltCrank => bolt_crank::UPDATE_FN,
            ClassUpdate::CrankRotator => bolt_crank::ROTATOR_UPDATE_FN,
            ClassUpdate::CrankSlider => bolt_crank::SLIDER_UPDATE_FN,
            ClassUpdate::PathEnemy => path_enemy::UPDATE_FN,
            ClassUpdate::PathEnemyShot => path_enemy::SHOT_UPDATE_FN,
            ClassUpdate::Dropship => dropship::UPDATE_FN,
            ClassUpdate::Gunship => gunship::UPDATE_FN,
            ClassUpdate::GunshipShell => gunship::SHELL_FN,
            ClassUpdate::GunshipFire => gunship::FIRE_FN,
            ClassUpdate::GunshipEmber => gunship::EMBER_FN,
            ClassUpdate::EnemySpawner => enemy_spawner::UPDATE_FN,
            ClassUpdate::Vendor => vendor::UPDATE_FN,
            ClassUpdate::TalkingNpc => talking_npc::UPDATE_FN,
            ClassUpdate::Bolt => bolt::UPDATE_FN,
            ClassUpdate::Crate => crate_::UPDATE_FN,
            ClassUpdate::Grass => grass::UPDATE_FN,
            ClassUpdate::Flyer => flyer::UPDATE_FN,
            ClassUpdate::TeleporterPad => teleporter::UPDATE_FN,
            ClassUpdate::ItemOffer => item_offer::UPDATE_FN,
            ClassUpdate::Debris => debris::UPDATE_FN,
            ClassUpdate::Flash => debris::FLASH_UPDATE_FN,
            ClassUpdate::PathPlatform => path_platform::UPDATE_FN,
            ClassUpdate::Checkpoint => checkpoint::UPDATE_FN,
            ClassUpdate::Flow => flow::UPDATE_FN,
            ClassUpdate::SwingTarget => swing_target::UPDATE_FN,
            ClassUpdate::Critter => critter::UPDATE_FN,
            ClassUpdate::FxPiece => crate::moby_update::creature::fx::PIECE_UPDATE_FN,
            ClassUpdate::ExplosionLight => crate::moby_update::creature::fx::LIGHT_UPDATE_FN,
            ClassUpdate::Amoeboid => amoeboid::UPDATE_FN,
            ClassUpdate::FireField => fire_field::UPDATE_FN,
            ClassUpdate::TextureScroll => fire_field::SCROLL_UPDATE_FN,
            ClassUpdate::Spinner => props::SPINNER_FN,
            ClassUpdate::Elevator => props::ELEVATOR_FN,
            ClassUpdate::SlidingDoor => props::DOOR_FN,
            ClassUpdate::Shootable => props::SHOOTABLE_FN,
            ClassUpdate::Collapse => props::COLLAPSE_FN,
            ClassUpdate::CameraMoby => crate::follow_camera::camera_moby::UPDATE_FN,
            ClassUpdate::Rock => breakables::novalis::ROCK_FN,
            ClassUpdate::ShellWall => breakables::novalis::SHELL_WALL_FN,
            ClassUpdate::Wall => breakables::novalis::WALL_FN,
            ClassUpdate::Pipe => breakables::novalis::PIPE_FN,
            ClassUpdate::Spray => breakables::novalis::SPRAY_FN,
            ClassUpdate::Remains => breakables::REMAINS_FN,
            ClassUpdate::CameraTrigger => camera_trigger::UPDATE_FN,
            ClassUpdate::MissionNpc => mission_npc::UPDATE_FN,
            ClassUpdate::HingedBridge => hinged_bridge::UPDATE_FN,
            ClassUpdate::CutsceneFx => cutscene_fx::UPDATE_FN,
            ClassUpdate::AmmoPickup => pickup::AMMO_UPDATE_FN,
            ClassUpdate::Nanotech => pickup::NANOTECH_UPDATE_FN,
            ClassUpdate::Splash => splash::UPDATE_FN,
            ClassUpdate::GoldBolt => gold_bolt::UPDATE_FN,
            ClassUpdate::Infobot => infobot::UPDATE_FN,
            ClassUpdate::PyroGlow => pyro_glow::UPDATE_FN,
            ClassUpdate::BuriedBolts => buried_bolts::UPDATE_FN,
            ClassUpdate::RcRange => rc_range::UPDATE_FN,
            ClassUpdate::Visibomb => visibomb::UPDATE_FN,
            ClassUpdate::MouseHouse => mouse::HOUSE_FN,
            ClassUpdate::Mouse => mouse::MOUSE_FN,
            ClassUpdate::MouseShot => mouse::SHOT_FN,
            ClassUpdate::ActivationZone => activation_zone::UPDATE_FN,
            ClassUpdate::FloorSwitch => floor_switch::UPDATE_FN,
            ClassUpdate::BlasterShot => blaster_shot::UPDATE_FN,
            ClassUpdate::RynoMissile => ryno_missile::UPDATE_FN,
            ClassUpdate::DevastatorMissile => devastator_missile::UPDATE_FN,
            ClassUpdate::ThrusterFlame => thruster_flame::UPDATE_FN,
            ClassUpdate::Chicken => chicken::UPDATE_FN,
            ClassUpdate::Feather => chicken::FEATHER_FN,
            ClassUpdate::Decoy => decoy::UPDATE_FN,
            ClassUpdate::Mine => mine::UPDATE_FN,
            ClassUpdate::Drone => drone::UPDATE_FN,
            ClassUpdate::DoomCanister => doom_canister::UPDATE_FN,
            ClassUpdate::DoomBot => doom_bot::UPDATE_FN,
            ClassUpdate::Ship => crate::travel::ship::UPDATE_FN,
            ClassUpdate::Breakable(i) => breakables::RECIPES[i as usize].func,
            ClassUpdate::Water(i) => crate::water::managers::PORTS[i as usize].func,
            ClassUpdate::Sea(i) => crate::water::sea::PORTS[i as usize].func,
            ClassUpdate::Unit(i) => units::PORTS[i as usize].func,
        }
    }

    /// The port's key in a moby's update slot (`Moby::update_fn` / `ClassInfo::update_fn`): its address on its reference
    /// level, with the reference level folded into the high bits when that level is not 01 (bit 31 + level << 24). Two
    /// ports reversed on different levels can sit at the same overlay address (level 12's drone shot and level 14's
    /// pop-up turret shot are both 0x2ece00), so the address alone does not name one port; level-01 ports keep their
    /// plain address (the external updates and the constants compare against those).
    pub fn key(self) -> u32 {
        match self.reference_level() {
            1 => self.address(),
            l => 0x8000_0000 | (l << 24) | self.address(),
        }
    }

    /// The port behind an update-slot key ([`ClassUpdate::key`]).
    pub fn from_address(a: u32) -> Option<ClassUpdate> { ClassUpdate::every().find(|u| u.key() == a) }

    /// [`ClassUpdate::ALL`] and every break-template copy (`Breakable(i)`).
    pub fn every() -> impl Iterator<Item = ClassUpdate> { ClassUpdate::ALL.into_iter().chain(breakables::ids().map(ClassUpdate::Breakable)).chain(crate::water::managers::ids().map(ClassUpdate::Water)).chain(crate::water::sea::ids().map(ClassUpdate::Sea)).chain(units::ids().map(ClassUpdate::Unit)) }

    /// The classes the level01 table maps to this function.
    pub fn classes(self) -> &'static [i16] {
        match self {
            ClassUpdate::Bomb => &bomb::CLASSES,
            ClassUpdate::Fireball => &bomb::FIREBALL_CLASSES,
            ClassUpdate::BoltCrank => &bolt_crank::CLASSES,
            ClassUpdate::CrankRotator => &bolt_crank::ROTATOR_CLASSES,
            ClassUpdate::CrankSlider => &bolt_crank::SLIDER_CLASSES,
            ClassUpdate::PathEnemy => &path_enemy::CLASSES,
            ClassUpdate::PathEnemyShot => &path_enemy::SHOT_CLASSES,
            ClassUpdate::Dropship => &dropship::CLASSES,
            ClassUpdate::Gunship => &gunship::CLASSES,
            ClassUpdate::GunshipShell => &gunship::SHELL_CLASSES,
            ClassUpdate::GunshipFire => &gunship::FIRE_CLASSES,
            ClassUpdate::GunshipEmber => &gunship::EMBER_CLASSES,
            ClassUpdate::EnemySpawner => &enemy_spawner::CLASSES,
            ClassUpdate::Vendor => &vendor::CLASSES,
            ClassUpdate::TalkingNpc => &talking_npc::CLASSES,
            ClassUpdate::Bolt => &bolt::CLASSES,
            ClassUpdate::Crate => &crate_::CLASSES,
            ClassUpdate::Grass => &grass::CLASSES,
            ClassUpdate::Flyer => &flyer::CLASSES,
            ClassUpdate::TeleporterPad => &teleporter::CLASSES,
            ClassUpdate::ItemOffer => &item_offer::CLASSES,
            ClassUpdate::Debris => &debris::CLASSES,
            ClassUpdate::Flash => &debris::FLASH_CLASSES,
            ClassUpdate::PathPlatform => &path_platform::CLASSES,
            ClassUpdate::Checkpoint => &checkpoint::CLASSES,
            ClassUpdate::Flow => &flow::CLASSES,
            ClassUpdate::SwingTarget => &swing_target::CLASSES,
            ClassUpdate::Critter => &critter::CLASSES,
            ClassUpdate::FxPiece => &crate::moby_update::creature::fx::PIECE_CLASSES,
            ClassUpdate::ExplosionLight => &[crate::moby_update::creature::fx::LIGHT_CLASS],
            ClassUpdate::Amoeboid => &amoeboid::CLASSES,
            ClassUpdate::FireField => &fire_field::CLASSES,
            ClassUpdate::TextureScroll => &fire_field::SCROLL_CLASSES,
            ClassUpdate::Spinner => &props::SPINNER_CLASSES,
            ClassUpdate::Elevator => &props::ELEVATOR_CLASSES,
            ClassUpdate::SlidingDoor => &props::DOOR_CLASSES,
            ClassUpdate::Shootable => &props::SHOOTABLE_CLASSES,
            ClassUpdate::Collapse => &props::COLLAPSE_CLASSES,
            ClassUpdate::CameraMoby => &crate::follow_camera::camera_moby::CLASSES,
            ClassUpdate::Rock => &breakables::novalis::ROCK_CLASSES,
            ClassUpdate::ShellWall => &breakables::novalis::SHELL_WALL_CLASSES,
            ClassUpdate::Wall => &breakables::novalis::WALL_CLASSES,
            ClassUpdate::Pipe => &breakables::novalis::PIPE_CLASSES,
            ClassUpdate::Spray => &breakables::novalis::SPRAY_CLASSES,
            ClassUpdate::Remains => &breakables::REMAINS_CLASSES,
            ClassUpdate::CameraTrigger => &camera_trigger::CLASSES,
            ClassUpdate::MissionNpc => &mission_npc::CLASSES,
            ClassUpdate::HingedBridge => &hinged_bridge::CLASSES,
            ClassUpdate::CutsceneFx => &cutscene_fx::CLASSES,
            ClassUpdate::AmmoPickup => &pickup::AMMO_CLASSES,
            ClassUpdate::Nanotech => &pickup::NANOTECH_CLASSES,
            ClassUpdate::Splash => &splash::CLASSES,
            ClassUpdate::GoldBolt => &gold_bolt::CLASSES,
            ClassUpdate::Infobot => &infobot::CLASSES,
            ClassUpdate::PyroGlow => &pyro_glow::CLASSES,
            ClassUpdate::BuriedBolts => &buried_bolts::CLASSES,
            ClassUpdate::RcRange => &rc_range::CLASSES,
            ClassUpdate::Visibomb => &visibomb::CLASSES,
            ClassUpdate::MouseHouse => &mouse::HOUSE_CLASSES,
            ClassUpdate::Mouse => &mouse::MOUSE_CLASSES,
            ClassUpdate::MouseShot => &mouse::SHOT_CLASSES,
            ClassUpdate::ActivationZone => &activation_zone::CLASSES,
            ClassUpdate::FloorSwitch => &floor_switch::CLASSES,
            ClassUpdate::BlasterShot => &blaster_shot::CLASSES,
            ClassUpdate::RynoMissile => &ryno_missile::CLASSES,
            ClassUpdate::DevastatorMissile => &devastator_missile::CLASSES,
            ClassUpdate::ThrusterFlame => &thruster_flame::CLASSES,
            ClassUpdate::Chicken => &chicken::CLASSES,
            ClassUpdate::Feather => &chicken::FEATHER_CLASSES,
            ClassUpdate::Decoy => &decoy::CLASSES,
            ClassUpdate::Mine => &mine::CLASSES,
            ClassUpdate::Drone => &drone::CLASSES,
            ClassUpdate::DoomCanister => &doom_canister::CLASSES,
            ClassUpdate::DoomBot => &doom_bot::CLASSES,
            ClassUpdate::Ship => &crate::travel::ship::CLASSES,
            ClassUpdate::Breakable(i) => breakables::RECIPES[i as usize].classes,
            ClassUpdate::Water(i) => crate::water::managers::PORTS[i as usize].classes,
            ClassUpdate::Sea(i) => crate::water::sea::PORTS[i as usize].classes,
            ClassUpdate::Unit(i) => units::PORTS[i as usize].classes,
        }
    }
}

/// The registry by class number alone: the Rust port of `o_class`'s update on level 01 (the classes each port
/// lists), if any. Levels use [`LevelPorts`], which reads the level's own class table.
///
/// Only the ports whose reference is level 01 take part: a port reversed on another level lists that level's class
/// numbers, which on level 01 (and on every level whose table has no entry for them) are other classes or none at all
/// (e.g. 367 is level 06's slider, U204, and a crate chunk without an update on level 01). Those ports run only
/// where a level's table names their code ([`LevelPorts`]).
pub fn for_class(o_class: i16) -> Option<ClassUpdate> { ClassUpdate::every().find(|u| u.reference_level() == 1 && u.classes().contains(&o_class)) }

impl ClassUpdate {
    /// The level whose overlay holds [`ClassUpdate::address`] (the swing target was reversed on level 03, every
    /// other port on level 01).
    pub const fn reference_level(self) -> u32 {
        match self {
            ClassUpdate::SwingTarget => 3,
            ClassUpdate::ActivationZone => activation_zone::REFERENCE_LEVEL,
            ClassUpdate::FloorSwitch => floor_switch::REFERENCE_LEVEL,
            ClassUpdate::Breakable(i) => breakables::RECIPES[i as usize].level,
            ClassUpdate::Water(i) => crate::water::managers::PORTS[i as usize].level,
            ClassUpdate::Sea(i) => crate::water::sea::PORTS[i as usize].level,
            ClassUpdate::Unit(i) => units::PORTS[i as usize].level,
            _ => 1,
        }
    }
}

/// The ported updates of one level (docs/plan/level_generalisation.md C1): the game calls the function the
/// level's class table `lvl.vtbl` names for the class, so a class runs a port when its table entry is **the same
/// code** as the port's reference function (`rc_formats::level_overlay::Relocation`: the reference overlay's
/// function matched in the level's overlay up to relocations). The class numbers a port lists are only its
/// level-01 users: the same function also runs other classes on other levels (the body pieces of every level,
/// the grass of 00 / 02 / 08, …), and those run the port too. A class the table does not list falls back to the
/// class-number registry [`for_class`]; an entry with no update (0) or with unported code runs nothing.
#[derive(Clone, Debug, Default)]
pub struct LevelPorts {
    /// o_class → (its update function in the level's overlay, the port of it), for every class of the level's
    /// table (update 0: none; port None: not ported).
    table: std::collections::HashMap<i16, (u32, Option<ClassUpdate>)>,
    /// o_class → the external update (a level-01 label handed to [`LevelPorts::from_overlays`]) its table entry is.
    external: std::collections::HashMap<i16, u32>,
}

impl LevelPorts {
    /// No class table: every class by number ([`for_class`]; unit tests and levels whose overlay is missing).
    pub fn by_class_number() -> LevelPorts { LevelPorts::default() }

    /// From the level's overlay and the reference overlays (`reference(level)`: the parsed overlay of
    /// [`ClassUpdate::reference_level`], None when missing: those ports then match nothing on this level).
    /// `external`: level-01 addresses of updates ported outside this module (the engine's particle emitters and
    /// water), looked up the same way ([`LevelPorts::external`]).
    pub fn from_overlays(target: &rc_formats::level_overlay::LevelOverlay, reference: &dyn Fn(u32) -> Option<std::sync::Arc<rc_formats::level_overlay::LevelOverlay>>, external: &[u32]) -> LevelPorts {
        use rc_formats::level_overlay::Relocation;
        let vtbl = target.vtbl();
        let mut refs: Vec<(u32, std::sync::Arc<rc_formats::level_overlay::LevelOverlay>)> = Vec::new();
        for u in ClassUpdate::every() {
            let l = u.reference_level();
            if refs.iter().all(|(k, _)| *k != l) { if let Some(o) = reference(l) { refs.push((l, o)); } }
        }
        // Each port's copies in this overlay.
        let mut copies: std::collections::HashMap<u32, ClassUpdate> = std::collections::HashMap::new();
        let mut ext_copies: std::collections::HashMap<u32, u32> = std::collections::HashMap::new();
        for (l, ov) in &refs {
            let rel = Relocation::new(ov, target);
            for u in ClassUpdate::every().filter(|u| u.reference_level() == *l) {
                for c in rel.copies(u.address()) { copies.entry(c).or_insert(u); }
            }
            if *l == 1 {
                for &a in external { for c in rel.copies(a) { ext_copies.entry(c).or_insert(a); } }
            }
        }
        // An update that starts with `jr ra; nop` does nothing: the empty-update unit (`units::empty`).
        let empty = units::row(units::empty::REFERENCE_LEVEL, units::empty::UPDATE_FN).map(ClassUpdate::Unit);
        let port_of = |f: u32| copies.get(&f).copied().or_else(|| empty.filter(|_| f != 0 && target.code(f, 2).is_some_and(units::empty::is_empty)));
        let table = vtbl.iter().map(|e| (e.o_class as i16, (e.update, port_of(e.update)))).collect();
        let external = vtbl.iter().filter_map(|e| Some((e.o_class as i16, *ext_copies.get(&e.update)?))).collect();
        LevelPorts { table, external }
    }

    /// The port that runs `o_class` on this level.
    pub fn get(&self, o_class: i16) -> Option<ClassUpdate> {
        // The player's ship: `InitLevelRenderGlobals` 0x255958 installs `ShipUpdate` 0x2a1c40 on the ship it creates
        // (+0x74), whatever the level's class table lists for 531..533 (level 01's entry is the empty update).
        if crate::travel::ship::CLASSES.contains(&o_class) { return Some(ClassUpdate::Ship); }
        match self.table.get(&o_class) {
            Some(&(_, u)) => u,
            None => for_class(o_class),
        }
    }

    /// The level-table address the port uses for `o_class` (`ClassInfo::update_fn`: the port's level-01 label).
    pub fn update_fn(&self, o_class: i16) -> Option<u32> { self.get(o_class).map(ClassUpdate::key) }

    /// The external update (one of the labels given to [`LevelPorts::from_overlays`]) `o_class` runs on this level.
    pub fn external(&self, o_class: i16) -> Option<u32> { self.external.get(&o_class).copied() }

    /// Whether the level's class table lists `o_class`.
    pub fn in_table(&self, o_class: i16) -> bool { self.table.contains_key(&o_class) }

    /// The level's update function of `o_class` (None: not in the table; 0: no update).
    pub fn level_update(&self, o_class: i16) -> Option<u32> { self.table.get(&o_class).map(|e| e.0) }

    /// Whether the loader fills `o_class`'s joint lists on this level: its port reads them, or a port of the level
    /// reads that class's joint points (a unit port's `joints`).
    pub fn needs_joint_lists(&self, o_class: i16) -> bool {
        self.get(o_class).is_some_and(ClassUpdate::needs_joint_lists) || self.table.values().filter_map(|e| e.1).any(|u| u.reads_joints_of(o_class))
    }

    /// The classes of the level's table with an update function that is not ported.
    pub fn unported(&self) -> Vec<i16> {
        let mut v: Vec<i16> = self.table.iter().filter(|(_, (f, u))| *f != 0 && u.is_none()).map(|(c, _)| *c).collect();
        v.sort();
        v
    }
}

/// `(*moby+0x74)(moby)`.
pub fn dispatch(u: ClassUpdate, w: &mut World, id: MobyId) {
    match u {
        ClassUpdate::Bomb => bomb::update(w, id),
        ClassUpdate::Fireball => bomb::fireball_update(w, id),
        ClassUpdate::BoltCrank => bolt_crank::update(w, id),
        ClassUpdate::CrankRotator => bolt_crank::rotator_update(w, id),
        ClassUpdate::CrankSlider => bolt_crank::slider_update(w, id),
        ClassUpdate::PathEnemy => path_enemy::update(w, id),
        ClassUpdate::PathEnemyShot => path_enemy::shot_update(w, id),
        ClassUpdate::Dropship => dropship::update(w, id),
        ClassUpdate::Gunship => gunship::update(w, id),
        ClassUpdate::GunshipShell => gunship::shell_update(w, id),
        ClassUpdate::GunshipFire => gunship::fire_update(w, id),
        ClassUpdate::GunshipEmber => gunship::ember_update(w, id),
        ClassUpdate::EnemySpawner => enemy_spawner::update(w, id),
        ClassUpdate::Vendor => vendor::update(w, id),
        ClassUpdate::TalkingNpc => talking_npc::update(w, id),
        ClassUpdate::Bolt => bolt::update(w, id),
        ClassUpdate::Crate => crate_::update(w, id),
        ClassUpdate::Grass => grass::update(w, id),
        ClassUpdate::Flyer => flyer::update(w, id),
        ClassUpdate::TeleporterPad => teleporter::update(w, id),
        ClassUpdate::ItemOffer => item_offer::update(w, id),
        ClassUpdate::Debris => debris::update(w, id),
        ClassUpdate::Flash => debris::flash_update(w, id),
        ClassUpdate::PathPlatform => path_platform::update(w, id),
        ClassUpdate::Checkpoint => checkpoint::update(w, id),
        ClassUpdate::Flow => flow::update(w, id),
        ClassUpdate::SwingTarget => swing_target::update(w, id),
        ClassUpdate::Critter => critter::update(w, id),
        ClassUpdate::FxPiece => crate::moby_update::creature::fx::piece_update(w, id),
        ClassUpdate::ExplosionLight => crate::moby_update::creature::fx::light_update(w, id),
        ClassUpdate::Amoeboid => amoeboid::update(w, id),
        ClassUpdate::FireField => fire_field::update(w, id),
        ClassUpdate::TextureScroll => fire_field::scroll_update(w, id),
        ClassUpdate::Spinner => props::spinner_update(w, id),
        ClassUpdate::Elevator => props::elevator_update(w, id),
        ClassUpdate::SlidingDoor => props::door_update(w, id),
        ClassUpdate::Shootable => props::shootable_update(w, id),
        ClassUpdate::Collapse => props::collapse_update(w, id),
        ClassUpdate::CameraMoby => crate::follow_camera::camera_moby::update(w, id),
        ClassUpdate::Rock => breakables::novalis::rock_update(w, id),
        ClassUpdate::ShellWall => breakables::novalis::shell_wall_update(w, id),
        ClassUpdate::Wall => breakables::novalis::wall_update(w, id),
        ClassUpdate::Pipe => breakables::novalis::pipe_update(w, id),
        ClassUpdate::Spray => breakables::novalis::spray_update(w, id),
        ClassUpdate::Remains => breakables::remains_update(w, id),
        ClassUpdate::CameraTrigger => camera_trigger::update(w, id),
        ClassUpdate::MissionNpc => mission_npc::update(w, id),
        ClassUpdate::HingedBridge => hinged_bridge::update(w, id),
        ClassUpdate::CutsceneFx => cutscene_fx::update(w, id),
        ClassUpdate::AmmoPickup => pickup::ammo_update(w, id),
        ClassUpdate::Nanotech => pickup::nanotech_update(w, id),
        ClassUpdate::Splash => splash::update(w, id),
        ClassUpdate::GoldBolt => gold_bolt::update(w, id),
        ClassUpdate::Infobot => infobot::update(w, id),
        ClassUpdate::PyroGlow => pyro_glow::update(w, id),
        ClassUpdate::BuriedBolts => buried_bolts::update(w, id),
        ClassUpdate::RcRange => rc_range::update(w, id),
        ClassUpdate::Visibomb => visibomb::update(w, id),
        ClassUpdate::MouseHouse => mouse::house_update(w, id),
        ClassUpdate::Mouse => mouse::mouse_update(w, id),
        ClassUpdate::MouseShot => mouse::shot_update(w, id),
        ClassUpdate::ActivationZone => activation_zone::update(w, id),
        ClassUpdate::FloorSwitch => floor_switch::update(w, id),
        ClassUpdate::BlasterShot => blaster_shot::update(w, id),
        ClassUpdate::RynoMissile => ryno_missile::update(w, id),
        ClassUpdate::DevastatorMissile => devastator_missile::update(w, id),
        ClassUpdate::ThrusterFlame => thruster_flame::update(w, id),
        ClassUpdate::Chicken => chicken::update(w, id),
        ClassUpdate::Feather => chicken::feather_update(w, id),
        ClassUpdate::Decoy => decoy::update(w, id),
        ClassUpdate::Mine => mine::update(w, id),
        ClassUpdate::Drone => drone::update(w, id),
        ClassUpdate::DoomCanister => doom_canister::update(w, id),
        ClassUpdate::DoomBot => doom_bot::update(w, id),
        ClassUpdate::Ship => crate::travel::ship::update(w, id),
        ClassUpdate::Breakable(i) => breakables::update(w, id, i),
        ClassUpdate::Water(i) => crate::water::managers::update(w, id, i),
        ClassUpdate::Sea(i) => crate::water::sea::update(w, id, i),
        ClassUpdate::Unit(i) => units::update(w, id, i),
    }
}

/// Classes whose update reads joint points (`FUN_002645a8`, [`World::joint_point`]): the loader fills
/// [`Services::joint_lists`](crate::moby_update::Services) for them from the class blob.
pub fn needs_joint_lists(o_class: i16) -> bool { for_class(o_class).is_some_and(ClassUpdate::needs_joint_lists) }

impl ClassUpdate {
    /// The ports whose update reads joint points ([`needs_joint_lists`] by port, for [`LevelPorts`]).
    pub fn needs_joint_lists(self) -> bool {
        match self {
            ClassUpdate::Unit(i) => units::PORTS[i as usize].classes.iter().any(|c| units::PORTS[i as usize].joints.contains(c)),
            // (TalkingNpc, Visibomb: their manipulators' target joints, `crate::moby_update::manip`; Vendor: the menu's
            // four arm manipulators on its joint lists 0x14..0x17, `menus::vendor::arm_manipulators`.)
            _ => matches!(self, ClassUpdate::Flyer | ClassUpdate::PathEnemy | ClassUpdate::Gunship | ClassUpdate::GoldBolt | ClassUpdate::Mouse | ClassUpdate::TalkingNpc | ClassUpdate::Visibomb | ClassUpdate::Vendor),
        }
    }

    /// Whether this port reads the joint points of `o_class`'s mobys (its own classes: [`ClassUpdate::needs_joint_lists`]).
    pub fn reads_joints_of(self, o_class: i16) -> bool {
        match self {
            ClassUpdate::Unit(i) => units::PORTS[i as usize].joints.contains(&o_class),
            // The vendor turns its hologram's joints (`manip::attach` on the child).
            ClassUpdate::Vendor => o_class == vendor::HOLOGRAM,
            _ => self.needs_joint_lists() && self.classes().contains(&o_class),
        }
    }
}
