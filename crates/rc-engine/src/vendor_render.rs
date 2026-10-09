//! What the Gadgetron vendor draws (docs/plan/interaction.md §9), from the state crate::interact_render keeps:
//!
//! * **The hologram's beam on approach** (class 11 states 1 / 2, level01 0x2bb128; the Gadgetron logo itself, class
//!   1143, is the dynamic moby the vendor creates, drawn with its metal pass by the dynamic-moby path, its two joints
//!   turned by the vendor's manipulators: `rc_game::moby_update::manip`): the class's draw callback 0x2ba9c0,
//!   the projector beam, 4 FX 0x18 quads over the 9 vertices of 0x1d77e0 (x / y scaled by the hologram's size pvar
//!   +0x90), V scrolled by gp−0x586c (+0x01 per drawn vendor, wrapping by −1), turned by the camera's yaw 0x167258
//!   (`fun_001fa030` with the Euler (0, 0, yaw)), ALPHA 0x44; then the **scan plane** (G-UI-006): one FX 0x1b quad, its
//!   phase t = gp−0x5868 (+0.01 per drawn vendor, back to 0 past 1), the square ±1.2·t·size about the vendor, its −x
//!   edge at z 1.1 + 1.4·t and its +x edge 0.2 lower, turned by the camera's yaw, UV (0, 0)..(1, 1), RGBA
//!   ((1 − t)·128) << 24 | 0x808080; then the four glow points (`FUN_002781d0`, crate::interact_render).
//! * **Mode 5** (`DrawWorld_Mode5` 0x2b4020): the item hologram (+0x200 ammo / +0x300 weapon) in the world before
//!   the vendor, the hologram cone (`VendorDrawHologramCone` 0x2b3cc8: FX 0x18, ALPHA 0x44, V scroll), the six screens
//!   placed on the vendor's monitor joints, the popup while buying, the glass quads (FX 0x19) and then the HUD.
//!   A screen's black target, content and static are 2D primitives of the HUD pass at the screen's rectangle (the target
//!   copied 1:1 at rest, squeezed while powering on / off); the static (`FUN_002b2cd8`: CLAMP_1 = 0, REPEAT; the
//!   noise added with ALPHA_1 0x68) and the glass quads go to the HUD's static layer (`Hud2dHook::statics`), over the
//!   canvases; the item panel's and the salesman's 3D parts are
//!   crate::screen_canvas canvases under the HUD pass: the item model (+0x100) with the class-13 backdrop, and the
//!   salesman (class 12 with the `vendor.bin` sequences), each drawn with the target's zoom-1.0 view and light set 14
//!   (the vendor's light: colour (0.9, 0.9, 0.6), direction vendor rows · (−0.42, −0.7, −0.577), ambient 0x202020).
//!
//! Differences: the world behind the menu is drawn live (the game re-uploads a still snapshot taken with the vendor
//! hidden; the port's world does not move in the menu either, but it can hide parts of the vendor where scenery is
//! nearer); the screens' 2D pass draws after the HUD's own elements (the game draws the HUD last; they do not overlap
//! on Novalis); the static layer composes in linear light over what is under it (exact over the black screens; over
//! the 3D item and salesman slightly brighter than the GS's display-byte blend where a scan line is partly
//! transparent).

use crate::hud_render::{Hud2d, Hud2dHook, HudBuild, Prim, Tex};
use crate::interact_render::{ScreenDraw, VendorRt};
use crate::moby_render::{self, ExtraMobys, MobyMaterial};
use crate::screen_canvas::{CanvasId, CanvasView, Canvases};
use crate::text_render::TextState;
use bevy::platform::collections::HashMap;
use bevy::prelude::*;
use bevy::render::storage::ShaderBuffer;
use rc_formats::moby::LevelMobyClass;
use rc_formats::moby_anim::{self, AnimState, MobyAnimClass, Rows};
use rc_formats::moby_light::{self, V4};
use rc_game::menus::vendor::{self as vendor, screens, ScreenMoby};
use rc_game::menus::MenuDraw;

/// Extra-moby slots.
const SLOT_ITEM: u32 = 0;
const SLOT_BACKDROP: u32 = 1;
const SLOT_SALESMAN: u32 = 2;
const SLOT_HOLO: u32 = 3;
const SLOT_POPUP: u32 = 4;
/// The popup's class-13 moby +0x500 (its quantity case).
const SLOT_POPUP_PANEL: u32 = 5;
const SLOTS: u32 = SLOT_POPUP_PANEL + 1;
/// Palette entries per slot (the salesman has 92 joints).
const JOINTS: u32 = 128;
/// The FX textures (the cones and the beam; the beam's scan plane).
const CONE_FX: usize = 0x18;
const SCAN_FX: usize = 0x1b;
/// Light set 14 and the screen mobys' light word.
const LIGHT_SET: usize = 14;
const LIGHT_WORD: u32 = 0x0e0e;

#[derive(Resource)]
struct VendorGfx {
    extra: ExtraMobys,
    /// Entities per (slot, class) and what each slot shows.
    ents: HashMap<(u32, i16), Vec<Entity>>,
    shown: HashMap<u32, i16>,
    /// The gadget table's classes (the weapons' models).
    gadgets: std::sync::Arc<Vec<(LevelMobyClass, MobyAnimClass)>>,
    item_canvas: CanvasId,
    salesman_canvas: CanvasId,
    /// The popup's target while it shows the +0x500 moby.
    popup_canvas: CanvasId,
    cone: crate::fx_draw::FxSlots,
    /// The beam's V scroll 0x161394 and the tick it last advanced.
    beam_scroll: f32,
    beam_tick: Option<u64>,
    /// gp−0x5868: the scan plane's phase (+0.01 per drawn beam, 0 past 1).
    scan: f32,
    /// FX 0x19's size (the glass quads' texels).
    glass_size: (i32, i32),
}

pub struct VendorRenderPlugin;

impl Plugin for VendorRenderPlugin {
    fn build(&self, app: &mut App) {
        if !crate::gameplay::enabled() { return; }
        app.add_systems(crate::level_switch::LevelUnload, crate::level_switch::remove::<VendorGfx>);
        app.add_systems(PreUpdate, setup).add_systems(Update, draw.after(crate::menu_render::MenuPrims).before(HudBuild));
    }
}

#[allow(clippy::too_many_arguments)]
fn setup(
    mut done: Local<bool>,
    generation: Res<crate::level_switch::LevelGeneration>,
    mut commands: Commands,
    level: Res<crate::Level>,
    mut canvases: ResMut<Canvases>,
    mut images: ResMut<Assets<Image>>,
    mut buffers: ResMut<Assets<ShaderBuffer>>,
) {
    // Once per level (crate::level_switch: a runtime level change runs it again).
    if generation.is_changed() { *done = false; }
    if *done { return; }
    *done = true;
    let lv = &level.0;
    let gadgets: Vec<(LevelMobyClass, MobyAnimClass)> = match crate::moby_attach::load_blobs() {
        Ok((_, g)) => g
            .iter()
            .filter_map(|g| moby_anim::parse_sequences(&g.blob, &g.moby.class).ok().map(|s| (g.moby.clone(), MobyAnimClass::new(&g.moby.class, s))))
            .collect(),
        Err(e) => {
            eprintln!("vendor render: no gadget classes ({e:#})");
            Vec::new()
        }
    };
    let records = vec![0u8; SLOTS as usize * moby_render::EXTRA_RECORD_SIZE];
    let extra = ExtraMobys::new(lv, records, crate::moby_anim::identity_palette(SLOTS * JOINTS), &mut buffers);
    let item_canvas = canvases.create(&mut commands, &mut images, "vendor item panel");
    let salesman_canvas = canvases.create(&mut commands, &mut images, "vendor salesman");
    let popup_canvas = canvases.create(&mut commands, &mut images, "vendor popup");
    let glass_size = lv.hud.as_ref().and_then(|h| h.fx.get(screens::GLASS_FX).cloned().flatten()).map_or((64, 64), |t| (t.width as i32, t.height as i32));
    println!("vendor render: {} gadget classes, glass FX {glass_size:?}", gadgets.len());
    commands.insert_resource(VendorGfx {
        extra,
        ents: HashMap::default(),
        shown: HashMap::default(),
        gadgets: std::sync::Arc::new(gadgets),
        item_canvas,
        salesman_canvas,
        popup_canvas,
        cone: Default::default(),
        beam_scroll: 0.0,
        beam_tick: None,
        scan: 0.0,
        glass_size,
    });
}

/// A slot's content this frame.
struct SlotDraw<'a> {
    class: &'a LevelMobyClass,
    rows: [[f32; 3]; 3],
    pos: [f32; 3],
    scale: f32,
    pose: Vec<Rows>,
    light: u32,
    ambient: [u8; 3],
}

fn rows_bits(r: &[[f32; 3]; 3]) -> [V4; 3] { r.map(|v| [v[0].to_bits(), v[1].to_bits(), v[2].to_bits(), 0]) }

/// A screen moby's class geometry and animation (the level's, the salesman's with its `vendor.bin` sequences, or a gadget's).
fn find_class<'a>(lv: &'a crate::level_load::LoadedLevel, gadgets: &'a [(LevelMobyClass, MobyAnimClass)], salesman: Option<&'a MobyAnimClass>, o: i16) -> Option<(&'a LevelMobyClass, &'a MobyAnimClass)> {
    let m = &lv.mobys;
    if let Some(ci) = m.classes.iter().position(|c| c.o_class == o as i32) {
        let anim = if o == vendor::salesman::CLASS { salesman? } else { &m.anim[ci] };
        return Some((&m.classes[ci], anim));
    }
    gadgets.iter().find(|(c, _)| c.o_class == o as i32).map(|(c, a)| (c, a))
}

/// A frozen pose on the first frame of seq `seq` (or 0).
fn still(anim: &MobyAnimClass, seq: u8) -> Vec<Rows> {
    let mut s = AnimState::spawn(anim);
    let seq = if anim.sequence(seq).is_some() { seq } else { 0 };
    moby_anim::hard_cut(&mut s, anim, seq, 0);
    s.speed = 0.0;
    moby_anim::evaluate_with_snapshot(anim, &s, None)
}

/// The screens' 2D primitives (module docs): per screen its black target (the canvases clear their own) and content in
/// target pixels, mapped onto the rectangle (returned), and its static into the static layer's passes (`statics`).
fn screen_prims(sd: &ScreenDraw, glyphs: &[rc_formats::font::GlyphTable; 3], frame_sizes: &[(i32, i32)], canvas: bool, statics: &mut [Vec<Prim>; 3]) -> Vec<Prim> {
    let p = sd.placed;
    let (tw, th) = ((p.full[2] - 1.0).max(1.0), (p.full[3] - 1.0).max(1.0));
    let (x0, x1) = (p.drawn[0].min(p.drawn[0] + p.drawn[2]), p.drawn[0].max(p.drawn[0] + p.drawn[2]));
    let (y0, y1) = (p.drawn[1].min(p.drawn[1] + p.drawn[3]), p.drawn[1].max(p.drawn[1] + p.drawn[3]));
    if x1 - x0 < 0.5 || y1 - y0 < 0.5 { return Vec::new(); }
    let (sx, sy) = (p.drawn[2] / tw, p.drawn[3] / th);
    let (ix0, ix1, iy0, iy1) = (x0.round() as i32, x1.round() as i32, y0.round() as i32, y1.round() as i32);
    let scissor = [ix0, ix1 - 1, iy0, iy1 - 1];
    let mut h = Hud2d::default();
    h.frame_sizes = frame_sizes.to_vec();
    let mut st = TextState::default();
    let mut local: Vec<Prim> = Vec::new();
    if !canvas {
        // The target's clear (FUN_00223470 black, copied with ALPHA 0x64 = replace): opaque black.
        local.push(Prim { tex: Tex::None, pos: [[0, 0], [screens::TARGET.0, 0], [0, screens::TARGET.1], [screens::TARGET.0, screens::TARGET.1]], uv: [[0, 0]; 4], rgba: 0x8000_0000, scissor: [0, 0, 0, 0], repeat: false, nearest: false, boxed: false, uv16: false });
    }
    for d in &sd.content {
        match d {
            MenuDraw::Hud(x) => crate::text_render::execute(&mut h, &mut st, glyphs, std::slice::from_ref(x)),
            MenuDraw::Rect { x0, y0, x1, y1, rgba } => h.rect(y0 - 1, y1 - 1, x0 - 1, x1 - 1, *rgba),
            MenuDraw::FrameQuad { frame, x, y, w, h: qh, u, v, tw, th, rgba } => {
                let (x1, y1, u1, v1) = (x + w, y + qh, u + tw, v + th);
                h.prims.push(Prim { tex: Tex::Frame(*frame), pos: [[*x, *y], [x1, *y], [*x, y1], [x1, y1]], uv: [[*u, *v], [u1, *v], [*u, v1], [u1, v1]], rgba: *rgba, scissor: [0, 0, 0, 0], repeat: false, nearest: false, boxed: false, uv16: false });
            }
            MenuDraw::SpriteUv { frame, x0, y0, x1, y1, u0, v0, u1, v1, alpha, .. } => {
                let rgba = ((*alpha as u32) & 0xff) << 24 | 0x007f_7f7f;
                let (px0, py0, px1, py1) = (x0 / 16, y0 / 16, x1 / 16, y1 / 16);
                let (ua, ub, va, vb) = (u0 / 16, u1 / 16, v0 / 16, v1 / 16);
                h.prims.push(Prim { tex: Tex::Frame(*frame), pos: [[px0, py0], [px1, py0], [px0, py1], [px1, py1]], uv: [[ua, va], [ub, va], [ua, vb], [ub, vb]], rgba, scissor: [0, 0, 0, 0], repeat: false, nearest: false, boxed: false, uv16: false });
            }
            _ => {}
        }
    }
    local.extend(h.prims);
    // The target is 512×128: texels outside it are not drawn.
    let map = |q: [i32; 2]| [(p.drawn[0] + q[0] as f32 * sx).round() as i32, (p.drawn[1] + q[1] as f32 * sy).round() as i32];
    // The static (`FUN_002b2cd8`) sets CLAMP_1 = 0 (`VU1_addGSregister(8, 0)`) before its draws: its texel ranges run
    // far past the textures (noise 0x1a is 32×32 at offsets up to 199, the bar 0x1c 16×16 over the whole screen), so
    // they tile (under the 2D pass' inherited CLAMP they smeared the edge texels into long lines and a flat grey). The
    // noise is added (ALPHA_1 0x68, FIX = its alpha): static layer pass 1; the bar and the popup's glass pass 2.
    for f in &sd.fx {
        let mut s = Hud2d::default();
        s.strip_glyph(f.fx, f.x as i32, f.y as i32, f.w as i32, f.h as i32, f.u, f.v, f.u1 - f.u, f.v1 - f.v, f.rgba);
        for q in s.prims {
            statics[if f.additive { 1 } else { 2 }].push(Prim { pos: q.pos.map(map), scissor, repeat: true, nearest: sd.nearest, ..q });
        }
    }
    local
        .into_iter()
        .map(|mut q| {
            q.pos = q.pos.map(map);
            q.scissor = scissor;
            q.nearest = sd.nearest;
            q
        })
        .collect()
}

/// The asset stores the draw writes.
type DrawAssets<'w> = (
    ResMut<'w, Assets<Mesh>>,
    ResMut<'w, Assets<Image>>,
    ResMut<'w, Assets<MobyMaterial>>,
    ResMut<'w, Assets<crate::fx_draw::FxPrimMaterial>>,
    ResMut<'w, Assets<ShaderBuffer>>,
);

#[allow(clippy::too_many_arguments)]
fn draw(
    mut commands: Commands,
    gfx: Option<ResMut<VendorGfx>>,
    vr: Res<VendorRt>,
    play: Option<Res<crate::gameplay::Play>>,
    level: Res<crate::Level>,
    mut hook: ResMut<Hud2dHook>,
    mut canvases: ResMut<Canvases>,
    fog: Option<Res<crate::game_camera::GameFog>>,
    mut vis: Query<&mut Visibility>,
    mut transforms: Query<&mut Transform, Without<crate::fly_cam::FlyCam>>,
    spawn_hidden: Query<(), With<crate::moby_spawn::SpawnHidden>>,
    (mut meshes, mut images, mut materials, mut fx_materials, mut buffers): DrawAssets,
) {
    let (Some(mut gfx), Some(play)) = (gfx, play) else { return };
    let gfx = &mut *gfx;
    let lv = &level.0;
    let d = &vr.draw;
    let gadgets = gfx.gadgets.clone();
    let salesman_anim = vr.vendor.as_ref().and_then(|v| v.tables.classes.salesman.as_ref());
    // ---- The screens' 2D primitives (before the menus' fades: the fade covers them).
    if let Some(lh) = lv.hud.as_ref() {
        let sizes: Vec<(i32, i32)> = lh.frames.iter().map(|t| (t.width as i32, t.height as i32)).collect();
        let mut prims: Vec<Prim> = Vec::new();
        let mut statics: [Vec<Prim>; 3] = Default::default();
        for sd in &d.screens {
            // The 3D targets clear on their canvas (the popup only while it shows its +0x500 moby).
            let canvas = sd.s == screens::ITEM || sd.s == screens::SALESMAN || (sd.s == screens::POPUP && d.scene.popup_panel.is_some());
            prims.extend(screen_prims(sd, &lh.glyphs, &sizes, canvas, &mut statics));
        }
        if let Some(view) = d.view {
            let (gw, gh) = gfx.glass_size;
            let vh = (gh as f32 * screens::GLASS_V1) as i32;
            for q in &d.glass {
                let c = q.corners();
                let pts: Option<Vec<[f32; 2]>> = c.iter().map(|&p| view.project(p)).collect();
                let Some(pts) = pts else { continue };
                let pos = [0, 1, 2, 3].map(|k| [pts[k][0].round() as i32, pts[k][1].round() as i32]);
                // After the screens and their static (the static layer's last pass).
                statics[2].push(Prim { tex: Tex::Fx(screens::GLASS_FX), pos, uv: [[0, 0], [gw, 0], [0, vh], [gw, vh]], rgba: 0x8080_8080, scissor: [0, 511, 0, 415], repeat: false, nearest: false, boxed: false, uv16: false });
            }
        }
        if !prims.is_empty() {
            prims.extend(std::mem::take(&mut hook.prims));
            hook.prims = prims;
        }
        for (k, s) in statics.into_iter().enumerate() { hook.statics[k].extend(s); }
    }
    // ---- The canvases of the item panel and the salesman.
    let canvas_of = |s: usize| d.screens.iter().find(|x| x.s == s).map(|x| {
        // The target's centre (256, 64) (`SetRenderToTextureView(…, 512, 128)`) with the frame's own focal lengths [M: the
        // zoom argument 1.0 does not reach the moby program's scales: with it both models sit at the targets' edges at a
        // third of the size the game shows; with the frame's scales they sit where the game shows them].
        let p = x.placed;
        let (tx, ty) = (crate::game_camera::TAN_HALF_FOV_X, crate::game_camera::TAN_HALF_FOV_X * crate::game_camera::NTSC_Y_RATIO);
        let (sx, sy) = (p.drawn[2] / (p.full[2] - 1.0).max(1.0), p.drawn[3] / (p.full[3] - 1.0).max(1.0));
        CanvasView { rect: p.drawn, focal: [256.0 / tx * sx, 208.0 / ty * sy], centre: [p.drawn[0] + 256.0 * sx, p.drawn[1] + 64.0 * sy], clear: Color::BLACK }
    });
    canvases.show(gfx.item_canvas, canvas_of(screens::ITEM));
    canvases.show(gfx.salesman_canvas, canvas_of(screens::SALESMAN));
    canvases.show(gfx.popup_canvas, if d.scene.popup_panel.is_some() { canvas_of(screens::POPUP) } else { None });
    // ---- The mobys.
    let mut want: Vec<(u32, SlotDraw)> = Vec::new();
    // Light set 14 for the vendor's screen mobys.
    let bank = lv.mobys.lighting.as_ref().map(|l| {
        let mut bank = l.bank.clone();
        if let (Some((_, _, rows)), Some(layout)) = (d.vendor, vr.vendor.as_ref().and_then(|v| v.tables.layout.as_ref())) {
            let dir = vendor::to_world(&rows, [0.0; 3], layout.light_dir);
            bank.sets[LIGHT_SET] = rc_formats::tfrag_light::DirLightSet { color_a: layout.light_color, dir_a: [dir[0], dir[1], dir[2], 0.0], color_b: [0.0; 4], dir_b: [0.0; 4] };
        }
        bank
    });
    let amb = |c: u32| [(c >> 16) as u8, (c >> 8) as u8, c as u8];
    let mut screen_moby = |slot: u32, m: &ScreenMoby, seq_still: u8| {
        let Some((class, anim)) = find_class(lv, &gadgets, salesman_anim, m.class) else { return };
        let pose = match &m.anim {
            Some((s, snap)) => moby_anim::evaluate_with_snapshot(anim, s, snap.as_ref()),
            None => still(anim, seq_still),
        };
        want.push((slot, SlotDraw { class, rows: m.rows, pos: m.position, scale: class.class.header.scale, pose, light: LIGHT_WORD, ambient: amb(m.ambient) }));
    };
    for (k, m) in d.scene.item_panel.iter().enumerate() {
        let slot = if m.class == vendor::BACKDROP_CLASS { SLOT_BACKDROP } else { SLOT_ITEM };
        screen_moby(slot, m, if k == 0 { 1 } else { 0 });
    }
    if let Some(m) = &d.scene.salesman { screen_moby(SLOT_SALESMAN, m, 0); }
    if let Some(m) = &d.scene.hologram { screen_moby(SLOT_HOLO, m, 1); }
    if let Some(m) = &d.scene.popup { screen_moby(SLOT_POPUP, m, 0); }
    if let Some(m) = &d.scene.popup_panel { screen_moby(SLOT_POPUP_PANEL, m, 0); }
    // The beams of the vendors whose hologram shows (the draw callback 0x2ba9c0 the vendor registers).
    let table = &play.game.mobys;
    let logos: Vec<([f32; 3], f32)> = table.mobys.iter().enumerate()
        .filter(|(_, m)| m.o_class == vendor::VENDOR_CLASS && m.state < 0x80)
        .filter_map(|(id, m)| rc_game::moby_update::classes::vendor::hologram(table, id).filter(|h| h.2).map(|(_, s, _)| ([m.position[0], m.position[1], m.position[2]], s)))
        .collect();
    // Records, palettes, entities.
    let mut palette = buffers.get(&gfx.extra.palette).and_then(|b| b.data.clone()).unwrap_or_default();
    let mut records = buffers.get(&gfx.extra.instances).and_then(|b| b.data.clone()).unwrap_or_default();
    let before = (palette.clone(), records.clone());
    let mut now: HashMap<u32, i16> = HashMap::default();
    for (slot, s) in &want {
        let oc = s.class.o_class as i16;
        now.insert(*slot, oc);
        let key = (*slot, oc);
        if !gfx.ents.contains_key(&key) {
            let ents = gfx.extra.spawn(&mut commands, lv, s.class, *slot, Transform::IDENTITY, "vendor screen moby", &mut meshes, &mut images, &mut materials);
            // Mode 5's `DrawMobyList` batch: its last list (the salesman or the popup) has no glow packets, so no screen
            // moby is recoloured (`ExtraMobys::clear_glow`).
            gfx.extra.clear_glow(&mut commands, *slot);
            let layer = match *slot {
                SLOT_ITEM | SLOT_BACKDROP => Some(canvases.layer(gfx.item_canvas)),
                SLOT_SALESMAN => Some(canvases.layer(gfx.salesman_canvas)),
                SLOT_POPUP_PANEL => Some(canvases.layer(gfx.popup_canvas)),
                _ => None,
            };
            for &e in &ents {
                if let Some(l) = &layer { commands.entity(e).insert(l.clone()); }
            }
            gfx.ents.insert(key, ents);
        }
        let base = slot * JOINTS;
        let at = base as usize * 64;
        for (i, b) in s.pose.iter().take(JOINTS as usize).flat_map(|r| r.iter().flatten().flat_map(|v| v.to_le_bytes())).enumerate() {
            if let Some(x) = palette.get_mut(at + i) { *x = b; }
        }
        let lights = bank.as_ref().map(|bank| moby_light::moby_lights(&rows_bits(&s.rows), bank, s.light, s.ambient, 0x80));
        let model = moby_render::extra_model(s.rows, s.scale, s.pos);
        let r = moby_render::extra_record(&model, lights.as_ref(), base);
        let o = *slot as usize * moby_render::EXTRA_RECORD_SIZE;
        if let Some(x) = records.get_mut(o..o + r.len()) { x.copy_from_slice(&r); }
        let t = Transform::from_matrix(model);
        for &e in &gfx.ents[&key] {
            if let Ok(mut tr) = transforms.get_mut(e) {
                if *tr != t { *tr = t; }
            }
            // The gameplay setup hides the moby entities tagged with Ratchet's index (MeshTag 0) by `SpawnHidden`.
            if spawn_hidden.contains(e) { commands.entity(e).remove::<crate::moby_spawn::SpawnHidden>(); }
        }
    }
    for (key, ents) in &gfx.ents {
        let show = now.get(&key.0) == Some(&key.1);
        let was = gfx.shown.get(&key.0) == Some(&key.1);
        if show == was && !show { continue; }
        for &e in ents {
            if let Ok(mut v) = vis.get_mut(e) { v.set_if_neq(if show { Visibility::Inherited } else { Visibility::Hidden }); }
        }
    }
    gfx.shown = now;
    if (&palette, &records) != (&before.0, &before.1) {
        crate::asset_write::set_buffer(&mut buffers, &gfx.extra.palette, &palette);
        crate::asset_write::set_buffer(&mut buffers, &gfx.extra.instances, &records);
    }
    // ---- The cone (menu) and the beams (approach).
    let counter = play.game.counter;
    // The camera's yaw 0x167258 the beams turn by.
    let cam_yaw = play.game.camera.out.euler[2].to_f32();
    let new_tick = gfx.beam_tick != Some(counter) && !logos.is_empty();
    if new_tick { gfx.beam_tick = Some(counter); }
    let mut groups = Vec::new();
    let layout = vr.vendor.as_ref().and_then(|v| v.tables.layout.clone()).or_else(|| vr.layout());
    if let Some(l) = layout.as_ref() {
        let quads = |cone: &vendor::layout::Cone, place: &dyn Fn([f32; 3]) -> [f32; 3], scroll: f32| {
            let mut b = crate::fx_draw::PrimBuf::default();
            for q in &cone.quads {
                let p = q.map(|i| place(cone.verts[i]));
                let st = q.map(|i| [cone.uv[i][0], cone.uv[i][1] + scroll]);
                b.quad(p, st, q.map(|i| cone.rgba[i]));
            }
            crate::fx_draw::FxGroup { fx: CONE_FX, additive: false, subtract: false, prims: b }
        };
        if let (Some((_, pos, rows)), Some(scroll)) = (d.vendor, d.cone) {
            groups.push(quads(&l.cone, &|v| vendor::to_world(&rows, pos, v), scroll));
        }
        for &(p, s) in &logos {
            // Each drawn beam steps the two shared phases (the callback runs once per vendor per frame).
            if new_tick {
                gfx.beam_scroll += 0.01;
                if gfx.beam_scroll > 1.0 { gfx.beam_scroll -= 1.0; }
            }
            // Turned by the camera's yaw (rows of the Euler (0, 0, yaw), the vertex as a row vector); x / y scaled by the
            // hologram's size.
            let (sn, cs) = cam_yaw.sin_cos();
            let place = move |v: [f32; 3]| [p[0] + (v[0] * cs - v[1] * sn) * s, p[1] + (v[0] * sn + v[1] * cs) * s, p[2] + v[2]];
            groups.push(quads(&l.beam, &place, gfx.beam_scroll));
            // The scan plane (FX 0x1b).
            if new_tick {
                gfx.scan += 0.01;
                if gfx.scan > 1.0 { gfx.scan = 0.0; }
            }
            let t = gfx.scan;
            let (a, b) = (-t * 1.2 * s, t * 1.2 * s);
            let z = t * 1.4 + 1.1;
            let corners = [[a, a, z], [b, a, z - 0.2], [a, b, z], [b, b, z - 0.2]];
            let turn = move |v: [f32; 3]| [p[0] + v[0] * cs - v[1] * sn, p[1] + v[0] * sn + v[1] * cs, p[2] + v[2]];
            let rgba = (((1.0 - t) * 128.0) as i32 as u32) << 24 | 0x80_8080;
            let mut pb = crate::fx_draw::PrimBuf::default();
            pb.quad(corners.map(turn), [[0.0, 0.0], [1.0, 0.0], [0.0, 1.0], [1.0, 1.0]], [rgba; 4]);
            groups.push(crate::fx_draw::FxGroup { fx: SCAN_FX, additive: false, subtract: false, prims: pb });
        }
    }
    let fog = fog.map(|f| f.uniform).unwrap_or_else(|| crate::game_camera::TfragFog::new(&lv.fog));
    let tex = lv.particles.textures.as_ref().map(|t| t.fx_textures.as_slice());
    let mut a = crate::fx_draw::FxAssets { meshes: &mut meshes, images: &mut images, materials: &mut fx_materials, fx: tex, fog };
    gfx.cone.show(&mut commands, &mut vis, &mut a, groups, crate::fx_draw::LIST1_BIAS + 50.0, "vendor cone");
}

#[cfg(test)]
mod tests {
    use super::*;
    use rc_game::menus::vendor::screens::{FxDraw, Placed};

    #[test]
    fn the_static_tiles_its_textures() {
        // An idle item panel at 1:1 (123×99 texels on a 123×99 rectangle) with a burst of noise FX 0x1a (32×32) at the
        // offset (150, 90) (`FUN_002b2cd8`: texels 150..150 + (w + 150)) and the scan bar FX 0x1c (16×16) over
        // h + 16: both run far past their textures and must wrap (CLAMP_1 = 0), the screen's other draws must not.
        let placed = Placed { full: [100.0, 60.0, 124.0, 100.0], drawn: [100.0, 60.0, 124.0, 100.0], ..Default::default() };
        let noise = FxDraw { fx: screens::NOISE_FX, x: 0.0, y: 0.0, w: 123.0, h: 99.0, u: 150, v: 90, u1: 150 + 273, v1: 90 + 189, rgba: 0x8080_8080, additive: true };
        let bar = FxDraw { fx: screens::BAR_FX, x: 0.0, y: -12.0, w: 123.0, h: 115.0, u: 0, v: 0, u1: 123, v1: 172, rgba: 0x5050_5050, additive: false };
        let sd = ScreenDraw { s: screens::BUTTONS, placed, content: vec![MenuDraw::Rect { x0: 1, y0: 1, x1: 9, y1: 9, rgba: 0x8080_8080 }], fx: vec![noise, bar], nearest: false };
        let glyphs = [[rc_formats::font::Glyph::default(); rc_formats::font::GLYPHS]; 3];
        let mut statics: [Vec<Prim>; 3] = Default::default();
        let p = screen_prims(&sd, &glyphs, &[], false, &mut statics);
        // The target's black clear and the content rectangle; the static in the static layer: the noise (added) in
        // pass 1, the bar in pass 2, both wrapping.
        assert_eq!(p.iter().map(|q| (q.tex, q.repeat)).collect::<Vec<_>>(), [(Tex::None, false), (Tex::None, false)]);
        assert!(statics[0].is_empty());
        assert_eq!(statics[1].iter().map(|q| (q.tex, q.repeat)).collect::<Vec<_>>(), [(Tex::Fx(screens::NOISE_FX), true)]);
        assert_eq!(statics[2].iter().map(|q| (q.tex, q.repeat)).collect::<Vec<_>>(), [(Tex::Fx(screens::BAR_FX), true)]);
        assert_eq!(statics[1][0].uv, [[150, 90], [423, 90], [150, 279], [423, 279]]);
        assert_eq!(statics[1][0].pos[0], [100, 60]);
        assert!(p.iter().chain(statics.iter().flatten()).all(|q| !q.nearest), "bilinear after the moby draws");
        // The ticker (after the hologram cone's TEX1_1 = 1): every primitive point-sampled.
        let mut statics: [Vec<Prim>; 3] = Default::default();
        let p = screen_prims(&ScreenDraw { s: screens::TICKER, nearest: true, ..sd }, &glyphs, &[], false, &mut statics);
        assert!(p.iter().chain(statics.iter().flatten()).all(|q| q.nearest));
    }
}
