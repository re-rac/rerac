//! The flight between planets' scenery (G-LVL-011): what `DrawWorldPaused` (level01 0x2a3b90, the draw of game mode 6
//! sub 4, the same code on all 19 overlays) draws around the flight's actors, from the `transition` lump
//! (`rc_formats::transition`) that `EnterSpaceLoadingLoop` 0x2a5868 loads. The state and the per-frame values are
//! `rc_game::travel::space` ([`FlightDraw`]); crate::travel_render runs the flight (its actors, its camera, its fade on
//! the transition's layer) and hands each frame's [`FlightDraw`] here.
//!
//! | game | what | here |
//! |---|---|---|
//! | `EnterSpaceLoadingLoop`: `LoadSky` | the flight's own sky (6 shells, its textures) | [`update_sky`]: the shells on [`FLIGHT_SKY_LAYER`] (crate::sky_render's shell draws), the sky camera switched to that layer for the flight |
//! | `DrawWorldPaused`: tan ≥ 0.63, `DrawSkyShells` 0x29f260 | per shell k: euler (0, y_k, 0x1604c4 + z_k), the rows ·s_k, the translation 0x160520; tan(hfov/2) at least 0.63 for the sky only | [`update_sky`] (`sky_render::shell_transform`, the shader applies the translation row), [`sky_tan`] |
//! | `fun_0022e8c8` 0x2a31d0 (v 4) | the planet picture `[dest]`: one quad, corners 0x1bdfd0·0x1be010[dest] + 0x1605b0, ST 0x1bdd30, RGBA 0x80808080, ALPHA 0x44, TEST 0x31801 (Z neither tested nor written; drawn before the actors) | [`draw_fx`] (an effect quad behind the actors' depth [L: in the game nothing hides it but the actors drawn after it; here the actors' depth does, which is the same while the picture is farther than the ship]) |
//! | `fun_0022e420` (not the first-arrival flights) | actor 0's trail, FX 0 of the lump's bank | [`draw_fx`] (`rc_game::travel::ship::trail_quads`) |
//! | `fun_0022ea08` 0x2a3310 (v 4, tick > 0x3c) | the caption `[19 + 19·max(lang − 1, 0) + dest]`: `DrawTexturedQuad(0x20, H − 0x58, 0x100, 0x20, 0, 0, 0x100, 0x20)`, RGBA `alpha << 24 \| 0x808080`, alpha min((tick − 0x3c)·2, 0x80), ALPHA 0x44 | [`hud_prims`] |
//! | `ShipDrawCallback` 0x2a70a8 (not the first-arrival flights) | actor 0's canopy glass, FX 1 in sub 4 | [`draw_fx`] (crate::fx_draw's glass quads with the lump's FX 1) |
//! | 0x2a3d08 (0x13d364 > 2 or 0x13d36c ≥ 0: the card busy) | TEST 0x3004b; `DrawTexturedQuad(0x2c, H − 0x60, 64, 64)` FX 2; FX 3 turned by −2π·(vsync % 55)/55 at (1216/16, H − 64), 272/16 square (`fun_00200600`) | [`hud_prims`] |
//! | the black fade `fade·128` | 0x15f3fc | crate::travel_render (`Cinematic::fade`) |
//! | `snd_bank_load_from_ee_cb` / `snd_play_sound_vol_pan_pmpb(bank, v, 0x400, 0, 0, 0)` | the flight's sound bank; sound v at tick 1 | [`audio_requests`] (the audio system's data swapped to the bank: `DoSpaceTransition` stopped every sound and the music before; the next level's swap replaces it) |
//!
//! Not drawn: the particles (the flight's draw has no particle pass); the lump's particle textures and its classes (the
//! classes are byte-identical to the `spaceships` files' the actors are drawn from).

use crate::fly_cam::FlyCam;
use crate::fx_draw::{FxAssets, FxGroup, FxPrimMaterial, FxSlots, PrimBuf};
use crate::game_camera::{game_eye, GameFog, GameProjection, TfragFog};
use crate::sky_render::{SkyCamera, SkyMaterial};
use bevy::camera::visibility::RenderLayers;
use bevy::camera::ClearColorConfig;
use bevy::prelude::*;
use rc_formats::scene::{Region, Scene};
use rc_formats::texture::Texture;
use rc_formats::transition::TransitionLump;
use rc_game::travel::space::{self, FlightDraw, FlightRig};
use std::collections::{HashMap, VecDeque};
use std::sync::{Arc, Mutex, OnceLock};

/// The render layer of the flight's sky (drawn by the sky camera while the flight runs).
pub const FLIGHT_SKY_LAYER: usize = 31;
/// The FX index the planet picture takes after the lump's FX textures (the effect slots' image index).
const PLANET_FX: usize = 4;
/// The planet quad's sort band: before the trail and the glass (crate::fx_draw's list-1 band).
const PLANET_BIAS: f32 = crate::fx_draw::LIST1_BIAS - 1.0e5;

/// The `transition` lump, read once per process.
pub fn lump() -> Option<Arc<TransitionLump>> {
    static LUMP: OnceLock<Option<Arc<TransitionLump>>> = OnceLock::new();
    LUMP.get_or_init(|| {
        let root = crate::level_load::extracted_root();
        let raw = crate::disc_source::read(&root, "global/transition.bin").map_err(|e| eprintln!("flight: transition lump: {e:#}")).ok()?;
        TransitionLump::parse(&raw).map_err(|e| eprintln!("flight: transition lump: {e}")).ok().map(Arc::new)
    })
    .clone()
}

/// The five flight variants (`transition` header[0x14 + v], `EnterSpaceLoadingLoop`).
pub(crate) fn variants() -> Vec<Option<Arc<Scene>>> {
    let Some(l) = lump() else { return vec![None; space::FLIGHT_VARIANTS] };
    (0..space::FLIGHT_VARIANTS)
        .map(|v| {
            let b = l.variant(v).map_err(|e| eprintln!("flight: variant {v}: {e}")).ok()?;
            Scene::from_lump(b, v, Region::Ntsc).map_err(|e| eprintln!("flight: variant {v}: {e}")).ok().map(Arc::new)
        })
        .collect()
}

/// The flight actors' skeletons (classes 531..533 of the `spaceships` files; their joint lists for
/// `MobyAttachToJoint`).
pub(crate) fn rigs() -> HashMap<i32, FlightRig> {
    let root = crate::level_load::extracted_root();
    let mut out = HashMap::new();
    for o_class in 531..=533i32 {
        let one = || -> Option<FlightRig> {
            let file = crate::disc_source::read(&root, &format!("global/spaceships/{:03}.bin", o_class - 530)).ok()?;
            let s = rc_formats::moby_spawn::parse_spaceship(&file).ok()?;
            let class = rc_formats::moby::parse_moby_class(s.class).ok()?;
            let seqs = rc_formats::moby_anim::parse_sequences(s.class, &class).ok()?;
            let anim = rc_formats::moby_anim::MobyAnimClass::new(&class, seqs);
            let lists = crate::travel_render::ship_joint_lists(o_class as i16)?;
            Some(FlightRig { anim, lists })
        };
        match one() {
            Some(r) => { out.insert(o_class, r); }
            None => eprintln!("flight: class {o_class}: no skeleton (the trail takes the ship's points)"),
        }
    }
    out
}

/// An audio request of the flight (from crate::travel_render's frame, run by [`audio_requests`]).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FlightAudio {
    /// The flight's sound bank becomes the sound data.
    Bank,
    /// Sound v of the bank, volume 0x400, pan 0, no pitch.
    Sound(i32),
}

static AUDIO: Mutex<VecDeque<FlightAudio>> = Mutex::new(VecDeque::new());

/// Queues an audio request (crate::travel_render).
pub fn post_audio(a: FlightAudio) { AUDIO.lock().unwrap_or_else(|e| e.into_inner()).push_back(a); }

/// The flight's scenery state.
#[derive(Resource, Default)]
pub struct FlightRender {
    /// This frame's scenery (crate::travel_render writes it every flight frame; None: nothing of the flight drawn).
    pub draw: Option<FlightDraw>,
    /// The flight runs (crate::travel_render: from the flight step's start to its end).
    pub active: bool,
    sky: Vec<(Entity, usize)>,
    sky_data: Option<Arc<crate::sky_render::LevelSky>>,
    /// The lump's FX textures and the planet picture at [`PLANET_FX`], for the destination `fx_dest`.
    fx: Vec<Option<Texture>>,
    fx_dest: Option<i32>,
    planet_slots: FxSlots,
    slots: FxSlots,
    /// The caption (raw GS RGBA) of (dest, language).
    caption: Option<((i32, u32), Arc<Texture>)>,
    /// FX 2 / 3 as HUD pictures (raw GS RGBA).
    card_icons: Option<[Arc<Texture>; 2]>,
    /// 0x15f3f8 for the card icon's turn: one per flight frame.
    vsync: i64,
    /// The canopy glass's near / far state (`ShipDrawCallback`'s globals 0x1604a4.., the boot data's first-draw 1:
    /// the first flight draw sets the ST, later ones keep it). Reset with each flight [L: whether the boot data is
    /// fresh for every flight].
    glass: crate::fx_draw::GlassFade,
}

/// Marks the flight's sky shells.
#[derive(Component)]
struct FlightShell(usize);

pub struct FlightRenderPlugin;

impl Plugin for FlightRenderPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<FlightRender>()
            .add_systems(crate::level_switch::LevelUnload, unload)
            .add_systems(Update, audio_requests)
            .add_systems(
                PostUpdate,
                (update_sky, sky_tan).chain().after(crate::sky_render::follow_main_camera).after(crate::travel_render::view_layers).before(bevy::transform::TransformSystems::Propagate),
            )
            .add_systems(PostUpdate, draw_fx.before(bevy::asset::AssetEventSystems));
    }
}

/// A level change: the scenery's entities are gone (the flight ends with the swap).
fn unload(mut f: ResMut<FlightRender>) {
    let sky_data = f.sky_data.take();
    *f = FlightRender { sky_data, ..Default::default() };
}

/// The lump's sky as a level sky (parsed once).
fn flight_sky() -> Option<Arc<crate::sky_render::LevelSky>> {
    let l = lump()?;
    let block = l.sky_block().map_err(|e| eprintln!("flight: sky: {e}")).ok()?;
    let sky = rc_formats::sky::parse_sky_block(block).map_err(|e| eprintln!("flight: sky: {e}")).ok()?;
    let textures = rc_formats::sky::parse_sky_textures(block, &sky).map_err(|e| eprintln!("flight: sky textures: {e}")).ok()?;
    Some(Arc::new(crate::sky_render::LevelSky { level: u32::MAX, sky, textures, sprite_scratch: Vec::new() }))
}

/// The flight's sky (module docs): spawned with the flight's first drawn frame, each shell's SkyM from the frame's
/// turn and translation, despawned when the flight ends; the sky camera on its layer meanwhile, the main camera loading
/// its colour instead of clearing to black.
#[allow(clippy::too_many_arguments)]
fn update_sky(
    mut commands: Commands,
    mut f: ResMut<FlightRender>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut images: ResMut<Assets<Image>>,
    mut materials: ResMut<Assets<SkyMaterial>>,
    mut shells: Query<(&FlightShell, &mut Transform)>,
    mut sky_cam: Query<(&mut Camera, &mut RenderLayers), With<SkyCamera>>,
    mut main_cam: Query<&mut Camera, (With<FlyCam>, Without<SkyCamera>)>,
) {
    let f = &mut *f;
    let want = f.active && f.draw.is_some();
    if want && f.sky.is_empty() {
        if f.sky_data.is_none() { f.sky_data = flight_sky(); }
        if let Some(ls) = f.sky_data.clone() {
            let s = crate::sky_render::spawn_shells(&mut commands, &ls, None, RenderLayers::layer(FLIGHT_SKY_LAYER), &mut meshes, &mut images, &mut materials, |e, si| {
                e.insert((Transform::IDENTITY, FlightShell(si)));
            });
            println!("flight: the flight's sky: {} shells, {} draws, {} triangles", ls.sky.shells.len(), s.entities.len(), s.triangles);
            f.sky = s.entities;
            for (mut cam, mut layers) in &mut sky_cam {
                *layers = RenderLayers::layer(FLIGHT_SKY_LAYER);
                cam.clear_color = ClearColorConfig::Custom(Color::BLACK);
            }
            for mut cam in &mut main_cam { cam.clear_color = ClearColorConfig::None; }
        }
    } else if !f.active && !f.sky.is_empty() {
        for (e, _) in f.sky.drain(..) { commands.entity(e).despawn(); }
        for (_, mut layers) in &mut sky_cam { *layers = RenderLayers::none(); }
        for mut cam in &mut main_cam { cam.clear_color = ClearColorConfig::Custom(Color::BLACK); }
    }
    let Some(d) = f.draw.as_ref() else { return };
    for (s, mut t) in &mut shells {
        let (euler, scale) = space::flight_shell(s.0, d.sky_turn).unwrap_or(([0.0; 3], 1.0));
        let want = crate::sky_render::shell_transform(euler, scale, [d.shell[0], d.shell[1], d.shell[2]]);
        if *t != want { *t = want; }
    }
}

/// The sky's tan(hfov/2): at least 0.63 (`DrawWorldPaused`), after the sky camera took the main camera's.
fn sky_tan(f: Res<FlightRender>, display: Res<crate::display::DisplaySettings>, mut sky_cam: Query<&mut Projection, With<SkyCamera>>) {
    let Some(d) = f.draw.as_ref().filter(|_| f.active && !f.sky.is_empty()) else { return };
    // Compared in the game's terms (the projection's x tangent carries the Hor+ factor, crate::display).
    let (min_x, _) = crate::display::view_tans(d.sky_tan, display.aspect);
    for mut p in &mut sky_cam {
        let below = matches!(&*p, Projection::Custom(c) if c.get::<GameProjection>().is_some_and(|g| g.tan_x < min_x));
        if below { crate::display::set_view_tans(&mut p, d.sky_tan, display.aspect); }
    }
}

/// The lump's FX textures with the destination's planet picture at [`PLANET_FX`].
fn fx_for(dest: i32) -> Vec<Option<Texture>> {
    let Some(l) = lump() else { return Vec::new() };
    let mut fx = l.fx_textures().map_err(|e| eprintln!("flight: FX textures: {e}")).unwrap_or_default();
    fx.resize(PLANET_FX, None);
    fx.push(l.planet_picture(dest).and_then(|p| p.decode()).map_err(|e| eprintln!("flight: planet picture {dest}: {e}")).ok());
    fx
}

type FlightCam<'w, 's> = Query<'w, 's, &'static Transform, (With<FlyCam>, Without<SkyCamera>)>;

/// The planet picture, the trail and the canopy glass (module docs) on the transition's layer.
#[allow(clippy::too_many_arguments)]
fn draw_fx(
    mut commands: Commands,
    mut f: ResMut<FlightRender>,
    play: Option<Res<crate::gameplay::Play>>,
    level: Res<crate::Level>,
    cams: FlightCam,
    fog: Option<Res<GameFog>>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut images: ResMut<Assets<Image>>,
    mut materials: ResMut<Assets<FxPrimMaterial>>,
    mut vis: Query<&mut Visibility>,
) {
    let f = &mut *f;
    let layer = RenderLayers::layer(crate::travel_render::TRAVEL_LAYER);
    f.planet_slots.layer = Some(layer.clone());
    f.slots.layer = Some(layer);
    let fog = fog.map(|g| g.uniform).unwrap_or_else(|| TfragFog::new(&level.0.fog));
    let (mut planet, mut groups) = (Vec::new(), Vec::new());
    if !f.active { f.glass = Default::default(); }
    if let (Some(d), true, Some(cam_t)) = (f.draw.as_ref(), f.active, cams.iter().next()) {
        if f.fx_dest != Some(d.dest) {
            f.fx = fx_for(d.dest);
            f.fx_dest = Some(d.dest);
            // The planet image slot caches its picture: a new destination needs fresh slots.
            let mut fresh = FxSlots::default();
            fresh.layer = f.planet_slots.layer.clone();
            f.planet_slots = fresh;
        }
        let cam = game_eye(cam_t).to_array();
        if let Some(c) = d.planet {
            let mut b = PrimBuf::default();
            b.quad(c, rc_game::travel::ship::SHADOW_ST, [0x8080_8080; 4]);
            planet.push(FxGroup { fx: PLANET_FX, additive: false, subtract: false, prims: b });
        }
        if let Some(p) = play.as_deref() {
            let tr = &p.svc.travel;
            if d.trail {
                let g = rc_game::travel::ship::trail_quads(&tr.trail, rc_game::travel::ship_index(tr.ship), true);
                let mut b = PrimBuf::default();
                for q in g.quads { b.quad(q.corners, q.st, q.rgba); }
                groups.push(FxGroup { fx: g.fx, additive: g.additive, subtract: g.subtract, prims: b });
            }
            if let (Some((o_class, m)), Some(t)) = (d.glass, level.0.water.fx.ship_glass.as_ref()) {
                if let Some(g) = t.of(o_class) {
                    // `ShipDrawCallback` in mode 6 sub 4: never live (the ST of the first draw stays), FX 1 (the
                    // flight bank's) instead of 0x15.
                    let mut grp = crate::fx_draw::ship_glass_prims(g, &m, cam, false, &mut f.glass);
                    grp.fx = FLIGHT_GLASS_FX;
                    groups.push(grp);
                }
            }
        }
    }
    // The planet picture shows without fog in the game [M: the original's flight frames show it in full colour, where
    // the flight's fog (FLIGHT_FOG) at its ~500 units would darken it almost to FOGCOL].
    let unfogged = TfragFog { color: fog.color.with_w(0.0), ..fog };
    let mut a = FxAssets { meshes: &mut meshes, images: &mut images, materials: &mut materials, fx: Some(&f.fx), fog: unfogged };
    f.planet_slots.show(&mut commands, &mut vis, &mut a, planet, PLANET_BIAS, "flight planet");
    a.fog = fog;
    f.slots.show(&mut commands, &mut vis, &mut a, groups, crate::fx_draw::LIST1_BIAS, "flight");
}

/// The canopy glass's FX texture in the flight (`ShipDrawCallback`, mode 6 sub 4).
pub const FLIGHT_GLASS_FX: usize = 1;

/// A decoded FX texture (alpha 0..0xff) as a HUD picture (raw GS alpha 0..0x80).
fn raw_alpha(t: &Texture) -> Texture {
    let mut rgba = t.rgba.clone();
    for a in rgba.iter_mut().skip(3).step_by(4) { *a = if *a == 0xff { 0x80 } else { *a / 2 }; }
    Texture { width: t.width, height: t.height, rgba }
}

/// The flight's 2D draws (module docs): the caption and the card icon, into the transition's HUD layer
/// (crate::travel_render's `hud_layer`).
pub(crate) fn hud_prims(f: &mut FlightRender, h: &mut crate::hud_render::Hud2d, img: &mut crate::hud_images::HudImages, assets: &mut Assets<Image>, lang: u32, card_busy: bool) {
    use crate::hud_render::H;
    use rc_game::menus::ImageSrc;
    let Some(d) = f.draw.as_ref().filter(|_| f.active) else { return };
    f.vsync += 1;
    if d.caption_alpha != 0 {
        let key = (d.dest, lang);
        if f.caption.as_ref().is_none_or(|c| c.0 != key) {
            f.caption = lump()
                .and_then(|l| l.caption(lang as i32, d.dest).and_then(|p| p.decode_raw()).map_err(|e| eprintln!("flight: caption {}: {e}", d.dest)).ok())
                .map(|t| (key, Arc::new(t)));
        }
        if let Some((_, t)) = &f.caption {
            let src = ImageSrc::Pixels { key: 0x7c00_0000 | (d.dest as u64) << 8 | lang as u64, tex: t.clone() };
            if let Some(s) = img.resolve(&src, assets) {
                h.prims.push(crate::hud_images::prim(s, space::CAPTION_X, H - space::CAPTION_DY, space::CAPTION_W, space::CAPTION_H, 0, 0, space::CAPTION_W, space::CAPTION_H, d.caption_alpha << 24 | 0x80_8080));
            }
        }
    }
    if !card_busy { return; }
    if f.card_icons.is_none() {
        let fx = lump().and_then(|l| l.fx_textures().ok()).unwrap_or_default();
        if let (Some(Some(a)), Some(Some(b))) = (fx.get(2), fx.get(3)) { f.card_icons = Some([Arc::new(raw_alpha(a)), Arc::new(raw_alpha(b))]); }
    }
    let Some([icon, spin]) = f.card_icons.clone() else { return };
    let [x, dy, w, hh] = space::CARD_ICON;
    if let Some(s) = img.resolve(&ImageSrc::Pixels { key: 0x7d00_0002, tex: icon }, assets) {
        h.prims.push(crate::hud_images::prim(s, x, H - dy, w, hh, 0, 0, w, hh, 0x8080_8080));
    }
    if let Some(s) = img.resolve(&ImageSrc::Pixels { key: 0x7d00_0003, tex: spin }, assets) {
        // fun_00200600: a w × h square centred at (cx, cy), turned by angle.
        let angle = (f.vsync.rem_euclid(space::CARD_SPIN_PERIOD as i64)) as f32 * -std::f32::consts::TAU / space::CARD_SPIN_PERIOD as f32;
        let (sn, c) = angle.sin_cos();
        let (cx, cy) = (space::CARD_SPIN_CENTRE[0] as f32, (H - space::CARD_SPIN_CENTRE[1]) as f32);
        let hw = space::CARD_SPIN_SIZE * 0.5;
        let corner = |x: f32, y: f32| [(cx + x * c - y * sn).round() as i32, (cy + x * sn + y * c).round() as i32];
        h.prims.push(crate::hud_render::Prim {
            tex: crate::hud_render::Tex::Dyn(s),
            pos: [corner(-hw, -hw), corner(hw, -hw), corner(-hw, hw), corner(hw, hw)],
            uv: [[0, 0], [w, 0], [0, hh], [w, hh]],
            rgba: 0x8080_8080,
            scissor: [0, crate::hud_render::W - 1, 0, H - 1],
            repeat: false,
            nearest: false,
            boxed: false,
        });
    }
}

/// The flight's audio requests (module docs).
fn audio_requests(mut audio: Option<ResMut<crate::audio_out::AudioOut>>) {
    let reqs: Vec<FlightAudio> = AUDIO.lock().unwrap_or_else(|e| e.into_inner()).drain(..).collect();
    let Some(out) = audio.as_deref_mut() else { return };
    for r in reqs {
        match r {
            FlightAudio::Bank => {
                let Some(l) = lump() else { continue };
                let bank = match l.sound_bank().and_then(rc_formats::sound_bank::parse_bank) {
                    Ok(b) => b,
                    Err(e) => {
                        eprintln!("flight: sound bank: {e}");
                        continue;
                    }
                };
                let data = rc_game::audio::LevelAudio {
                    bank: Arc::new(bank),
                    sounds: Default::default(),
                    instances: Vec::new(),
                    instance_pvars: Vec::new(),
                    music: Default::default(),
                    env_points: Vec::new(),
                    collision: None,
                };
                out.swap_level(data);
                // The IOP keeps the master volumes the level's last `sound_update` set (the flight runs none).
                let sys = out.system();
                for (group, vol) in rc_game::audio::voices::master_volumes(sys.sfx_option, sys.music_option, false, false).into_iter().enumerate() {
                    sys.snd.set_master_volume(group, vol, &mut sys.spu);
                }
                println!("flight: the flight's sound bank loaded");
            }
            FlightAudio::Sound(v) => {
                let sys = out.system();
                let bank = Arc::clone(&sys.data.bank);
                let h = sys.snd.play_sound(&bank, &mut sys.spu, v as u16, 0x400, 0, 0, 0);
                println!("flight: sound {v} of the flight's bank (handle {h})");
            }
        }
    }
}
