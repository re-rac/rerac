//! The moby content of the page menu's 3D widgets (`rc_game::menus::pause::gadgets`, docs/plan/gadgets.md §3): the 3D
//! Ratchet (0x297d70 / 0x291800) wearing the page's pending items and the item preview (0x291c38 / 0x292010) of the
//! Gadgets and Weapons pages. They are drawn by two canvases of crate::screen_canvas composed **over** the HUD
//! ([`Canvases::create_over_hud`]: `PageMenuDraw` copies a widget's render target onto its navy panel, and the port's
//! panels are HUD primitives).
//!
//! **How the game draws them**: each widget renders its mobys (`DrawMobyList(m, 1)`, the menu camera 0x167240 =
//! (256, 256, 64) looking along +x, the menu light set 14) into its own render target cleared to the navy
//! 0x80100808, which `PageMenuDraw` copies onto the widget's panel (4 aspect crop, 8 centre crop): the models sit on
//! the camera axis, so they appear at the panel's centre [L: the target's projection centre is not traced; the
//! reference screenshot shows them centred].
//!
//! **Here**: per widget a canvas whose view is the panel's rectangle, the game projection's focal lengths and the
//! panel's centre as the view axis, cleared to the navy; the mobys are `ExtraMobys` instances on the canvas's layer,
//! lit with the menu light, the menu view folded into their model matrices (`C_main · C_menu⁻¹ · model`: the canvas
//! camera carries the main transform, like crate::menu_render's frame mobys).
//!
//! * 3D Ratchet: class 0 at camera + (4, 0, −0.6), turned by π (`FUN_00297ad0`), on his idle sequence 0 [L: the
//!   game streams per-item animations, `fun_002265d8` with the table 0x1b9870, not ported]; Clank and the pending
//!   pack on the back list (the Heli-Pack's class 607 spins its rotor sequence 6 eight times, then folds to sequence 1:
//!   `LoadHandGadget` and the callback 0x224fc0, [`back_update`]); the pending
//!   hand item on its attach list (sequence 1); the pending head item and boots posed from his joints
//!   (`rc_game::hero::worn::pose_from_host`); the Persuader (0x197, on his joint list 0x1e), the Map-o-Matic (0x266)
//!   and the Bolt Grabber (0x26a, list 0x1d) when owned (`fun_002250f0`: on sequence 0, advanced, at the list's
//!   matrix with its rows normalised). Not drawn: the drones (class 0x1df, `spawn_extra479`'s orbit; the hero's drone
//!   counts 0x141346 / 0x141347 are not kept by the port: G-UI-002).
//! * Item preview: the focused cell's item at camera + the table 0x1c4988 offset, rotated, cut to its sequence then
//!   advanced every menu frame; Clank with it for a back item.
//! * The end page's Helpdesk girl (`rc_game::menus::pause::media`, G-CUT-008): class 0x7a5 at camera + (2.2, 0, −1.6),
//!   turned by π, posed by the widget's own animation state (`GirlView`); her sequences 1..3 come from the global lump
//!   `post_credits_helpdesk_girl_seq` ([`girl_anim_class`], also given to the page menu for her state machine).

use crate::moby_render::{self, ExtraMobys, MobyMaterial};
use crate::screen_canvas::{CanvasId, CanvasView, Canvases};
use bevy::prelude::*;
use bevy::render::storage::ShaderBuffer;
use bevy::transform::TransformSystems;
use rc_formats::gadget;
use rc_formats::moby::LevelMobyClass;
use rc_formats::moby_anim::{self, AnimState, MobyAnimClass, Rows};
use rc_formats::moby_light::{self, V4};
use rc_game::hero::worn;
use rc_game::menus::pause::frame;
use rc_game::menus::pause::gadgets::GadgetsView;

/// The page's 3D widgets as the last menu frame drew them (written by crate::menu_render) and the menu frame count.
#[derive(Resource, Default, Debug)]
pub struct GadgetsPreview {
    pub view: GadgetsView,
    /// The end page's Helpdesk girl (`rc_game::menus::pause::media::MediaMenu::girl_view`).
    pub girl: Option<rc_game::menus::pause::gadgets::GirlView>,
    pub frame: u64,
}

/// Ratchet's joint lists `FUN_0022a940` evaluates (crate::moby_attach): the attach word indexes them.
const HERO_LISTS: [usize; 9] = [0, 1, 2, 3, 4, 5, 6, 29, 30];
const BACK_ATTACH: usize = 5;
const CLANK_O_CLASS: i32 = 601;
const HELI_O_CLASS: i16 = 607;
const SONIC_O_CLASS: i16 = 0x1b1;
/// The 3D Ratchet's extras by `ModelView::extras` order (the Persuader, the Map-o-Matic, the Bolt Grabber) and the
/// index of the joint list each sits on in [`HERO_LISTS`] (0x1e → 8, 0x1d → 7).
const EXTRA_CLASSES: [i16; 3] = [0x197, 0x266, 0x26a];
const EXTRA_LISTS: [usize; 3] = [8, 7, 7];
/// The 3D Ratchet's offset from the menu camera (`FUN_00297ad0`: x + 4, z − 0.6) and yaw π.
const MODEL_OFFSET: [f32; 3] = [4.0, 0.0, -0.6];

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Role {
    Ratchet,
    Clank,
    Back,
    Hand,
    Head,
    BootL,
    BootR,
    /// The item preview's moby and its Clank.
    Item,
    ItemClank,
    /// The Weapons page's ammo model (`rc_game::menus::pause::pages::ammo_model_draw`) and the Items page's gold bolt
    /// (`pages::gold_draw`).
    Ammo,
    GoldBolt,
    /// The end page's Helpdesk girl.
    Girl,
    /// The 3D Ratchet's Persuader (0x197), Map-o-Matic (0x266) and Bolt Grabber (0x26a) (`LoadHandGadget` with their
    /// items owned; update `fun_002250f0`: at his joint list 0x1e (the Persuader) or 0x1d).
    Extra,
}

struct Part {
    role: Role,
    o_class: i16,
    anim: MobyAnimClass,
    scale: f32,
    base: u32,
    slots: u32,
    entities: Vec<Entity>,
    shown: bool,
    state: AnimState,
    /// The item the state was cut for (its part changes show).
    cut_for: i32,
    /// The moby's +0x20 (the 3D Ratchet's Heli-Pack: its rotor cycles left, [`back_update`]) and its blend snapshot.
    count: u8,
    snap: Option<moby_anim::MobyFrame>,
}

#[derive(Resource)]
struct PreviewRt {
    parts: Vec<Part>,
    extra: ExtraMobys,
    chains: Vec<(usize, Vec<u8>)>,
    bank: Option<rc_formats::tfrag_light::LightBank>,
    menu_cam: Mat4,
    /// The canvases of the 3D Ratchet, the item preview, the ammo model, the gold bolt and the Helpdesk girl.
    canvases: [CanvasId; 5],
    last_frame: u64,
}

/// The 3D Ratchet's back item callback `0x224fc0`, its animation part, then `MobyAnimAdvance`: when the last advance
/// wrapped (+0x70 bit 1), the Heli-Pack on its rotor sequence 6 counts a cycle off +0x20 and at 0 blends to sequence 1
/// (the blades folded) over 10 ticks; otherwise it stays on 6 (cut, no blend), and off 6 it is cut to 1. Another pack
/// is cut to sequence 1.
fn back_update(p: &mut Part) {
    let s = &mut p.state;
    if s.flags & 2 != 0 {
        let (seq, ticks) = if p.o_class != HELI_O_CLASS {
            (1, 0)
        } else if s.seq_a == 6 {
            p.count = p.count.wrapping_sub(1);
            if p.count == 0 { (1, 10) } else { (6, 0) }
        } else {
            (1, 0)
        };
        if s.seq_b != seq { moby_anim::set_sequence(s, &p.anim, seq, 0, ticks, &mut p.snap); }
    }
    moby_anim::advance(&mut p.state, &p.anim);
}

pub struct MenuModelsPlugin;

impl Plugin for MenuModelsPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<GadgetsPreview>().add_systems(crate::level_switch::LevelUnload, (crate::level_switch::reset::<GadgetsPreview>, crate::level_switch::remove::<PreviewRt>));
        if !crate::gameplay::enabled() { return; }
        app.add_systems(PreUpdate, setup).add_systems(PostUpdate, update.before(crate::screen_canvas::apply).before(TransformSystems::Propagate));
    }
}

#[allow(clippy::too_many_arguments)]
fn setup(
    mut done: Local<bool>,
    generation: Res<crate::level_switch::LevelGeneration>,
    mut commands: Commands,
    level: Res<crate::Level>,
    canvases: Option<ResMut<Canvases>>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut images: ResMut<Assets<Image>>,
    mut materials: ResMut<Assets<MobyMaterial>>,
    mut buffers: ResMut<Assets<ShaderBuffer>>,
) {
    // Once per level (crate::level_switch: a runtime level change runs it again).
    if generation.is_changed() { *done = false; }
    if *done { return; }
    let Some(mut canvases) = canvases else { return };
    *done = true;
    let ids = [
        canvases.create_over_hud(&mut commands, &mut images, "menu 3D Ratchet"),
        canvases.create_over_hud(&mut commands, &mut images, "menu item preview"),
        canvases.create_over_hud(&mut commands, &mut images, "menu ammo model"),
        canvases.create_over_hud(&mut commands, &mut images, "menu gold bolt"),
        canvases.create_over_hud(&mut commands, &mut images, "menu helpdesk girl"),
    ];
    let lv = &level.0;
    let m = &lv.mobys;
    let (ratchet_blob, gadgets) = match crate::moby_attach::load_blobs() {
        Ok(b) => b,
        Err(e) => {
            eprintln!("menu models: no classes ({e:#}): no 3D widgets");
            return;
        }
    };
    let level_class = |o: i32| m.classes.iter().position(|c| c.o_class == o).map(|ci| (m.classes[ci].clone(), m.anim[ci].clone()));
    let Some((ratchet, ratchet_anim)) = level_class(gadget::RATCHET_O_CLASS) else { return };
    let chains: Vec<(usize, Vec<u8>)> =
        HERO_LISTS.iter().enumerate().filter_map(|(i, &l)| gadget::joint_list(&ratchet_blob, &ratchet.class.header, l).ok().map(|(a, _)| (i, a))).collect();
    // Every class the widgets can show: the level's (Ratchet, Clank, packs, head items, boots, the drone 0x1df) and
    // the gadget table's (the hand items).
    let mut classes: Vec<(LevelMobyClass, MobyAnimClass)> = Vec::new();
    for o in [601, 607, 608, 609, 433, 1289, 1290, 173, 195, 0x1df] {
        if let Some(c) = level_class(o) { classes.push(c); }
    }
    for g in &gadgets {
        let Ok(seqs) = moby_anim::parse_sequences(&g.blob, &g.moby.class) else { continue };
        classes.push((g.moby.clone(), MobyAnimClass::new(&g.moby.class, seqs)));
    }
    let mut specs: Vec<(Role, LevelMobyClass, MobyAnimClass, usize)> = vec![(Role::Ratchet, ratchet, ratchet_anim, 0)];
    for (c, a) in &classes {
        let o = c.o_class;
        let roles: &[Role] = match o {
            601 => &[Role::Clank, Role::ItemClank],
            607..=609 => &[Role::Back, Role::Item],
            433 | 1289 | 1290 => &[Role::Head, Role::Item],
            173 | 195 => &[Role::BootL, Role::BootR, Role::Item],
            0x1df => &[Role::Item],
            _ => &[Role::Hand, Role::Item],
        };
        for &r in roles {
            let layer = matches!(r, Role::Item | Role::ItemClank) as usize;
            specs.push((r, c.clone(), a.clone(), layer));
        }
    }
    // The 3D Ratchet's extras (crate docs: `fun_002250f0`).
    for o in EXTRA_CLASSES {
        if let Some((c, a)) = level_class(o as i32) { specs.push((Role::Extra, c, a, 0)); }
    }
    // The ammo pickups' classes (item definitions +0x3a; `SpawnHandGadgetMoby` makes one only when the level has the
    // class) and the gold bolt 0x46e.
    for o in [226, 204, 222, 1006, 214, 225, 213, 223, 1438, 1447, 1449] {
        if let Some((c, a)) = level_class(o) { specs.push((Role::Ammo, c, a, 2)); }
    }
    if let Some((c, a)) = level_class(rc_game::menus::pause::pages::GOLD_BOLT_CLASS) { specs.push((Role::GoldBolt, c, a, 3)); }
    // The Helpdesk girl (class 0x7a5 with her streamed sequences) when the level has her class.
    if let Some((c, a)) = level_class(rc_game::menus::pause::media::GIRL_CLASS as i32) {
        let a = girl_anim_class(&a).unwrap_or(a);
        specs.push((Role::Girl, c, a, 4));
    }
    let mut parts = Vec::new();
    let mut palette_len = 0u32;
    for (role, class, anim, _) in &specs {
        let slots = (anim.joint_count as u32).max(ExtraMobys::max_skinned_joint(class) as u32 + 1).max(1);
        parts.push(Part {
            role: *role, o_class: class.o_class as i16, anim: anim.clone(), scale: class.class.header.scale, base: palette_len, slots,
            entities: Vec::new(), shown: false, state: AnimState::spawn(anim), cut_for: -1, count: 0, snap: None,
        });
        palette_len += slots;
    }
    let records = vec![0u8; parts.len() * moby_render::EXTRA_RECORD_SIZE];
    let mut extra = ExtraMobys::new(lv, records, crate::moby_anim::identity_palette(palette_len), &mut buffers);
    for (k, (p, (_, class, _, layer))) in parts.iter_mut().zip(&specs).enumerate() {
        p.entities = extra.spawn(&mut commands, lv, class, k as u32, Transform::IDENTITY, "menu 3D widget", &mut meshes, &mut images, &mut materials);
        // The widgets overwrite their mobys' mode bits after creating them (+0x34 = 0: the 3D Ratchet 0x297ad0, the
        // preview 0x291c38; 4: `LoadHandGadget` 0x297d70), so none is on the glow list (`ExtraMobys::clear_glow`).
        extra.clear_glow(&mut commands, k as u32);
        for &e in &p.entities { commands.entity(e).insert((canvases.layer(ids[*layer]), Visibility::Hidden)); }
    }
    // The menu light (set 14, crate::menu_render's frame mobys).
    // The level's overlay with its address map against level 01's (as crate::menu_render reads it).
    let root = crate::level_load::extracted_root();
    let bytes = crate::disc_source::level_file(&root, crate::level_load::level_index(), "overlay.bin").unwrap_or_default();
    let ov = match crate::disc_source::level_file(&root, 1, "overlay.bin") {
        Ok(r) => rc_game::menus::Overlay::relocated(&bytes, &r),
        Err(_) => rc_game::menus::Overlay::parse(&bytes),
    }
    .unwrap_or_default();
    let q = |a: u32| -> [f32; 4] { std::array::from_fn(|k| f32::from_bits(ov.u32(ov.at(a) + 4 * k as u32).unwrap_or(0))) };
    let bank = lv.mobys.lighting.as_ref().map(|l| {
        let mut bank = l.bank.clone();
        bank.sets[frame::LIGHT_SET] = rc_formats::tfrag_light::DirLightSet { color_a: q(frame::LIGHT_COLOR_ADDR), dir_a: q(frame::LIGHT_DIR_ADDR), color_b: [0.0; 4], dir_b: [0.0; 4] };
        bank
    });
    let menu_cam = Transform::from_translation(crate::tfrag_render::game_to_bevy(frame::CAMERA_POS)).looking_to(Vec3::X, Vec3::Y).to_matrix();
    println!("menu models: {} parts ({} palette slots) on four canvases over the HUD", parts.len(), palette_len);
    commands.insert_resource(PreviewRt { parts, extra, chains, bank, menu_cam, canvases: ids, last_frame: 0 });
}

/// The Helpdesk girl's class with the three sequences of `post_credits_helpdesk_girl_seq` (TOC 0x1610) in its slots
/// 1..3 (`fun_002256e8` state 1: WAD-decompressed; a table of three 8-byte entries, each's first word the offset of a
/// sequence whose pointers are relative to itself, `relocate_asset_entry_pointers`). None when the lump is not read.
pub fn girl_anim_class(level_anim: &MobyAnimClass) -> Option<MobyAnimClass> {
    let root = crate::level_load::extracted_root();
    let raw = crate::disc_source::read(&root, "global/post_credits_helpdesk_girl_seq.bin").map_err(|e| eprintln!("menu models: no Helpdesk girl sequences ({e:#})")).ok()?;
    let d = if rc_formats::wad::is_wad(&raw) { rc_formats::wad::decompress(&raw).ok()? } else { raw };
    let mut a = level_anim.clone();
    if a.sequences.len() < 4 { a.sequences.resize(4, None); }
    for k in 0..3usize {
        let o = u32::from_le_bytes(d.get(8 * k..8 * k + 4)?.try_into().ok()?) as usize;
        a.sequences[k + 1] = Some(moby_anim::parse_sequence(d.get(o..)?, 0).map_err(|e| eprintln!("menu models: Helpdesk girl sequence {}: {e:#}", k + 1)).ok()?);
    }
    Some(a)
}

fn rows_f32(r: &[V4; 3]) -> [[f32; 3]; 3] { r.map(|v| [0, 1, 2].map(|k| f32::from_bits(v[k]))) }

/// A part's placement this frame: rows (game), position, its pose, whether its state advances.
struct Placed {
    rows: [V4; 3],
    pos: [f32; 3],
    pose: Vec<Rows>,
}

/// The widgets each frame: the canvases follow the view; the parts' poses, records and visibility.
#[allow(clippy::too_many_arguments)]
fn update(
    mut commands: Commands,
    rt: Option<ResMut<PreviewRt>>,
    gp: Res<GadgetsPreview>,
    mode: Option<Res<crate::menu_render::MenuMode>>,
    play: Option<Res<crate::gameplay::Play>>,
    mut canvases: ResMut<Canvases>,
    main: Query<&Transform, With<crate::fly_cam::FlyCam>>,
    mut transforms: Query<&mut Transform, Without<crate::fly_cam::FlyCam>>,
    mut vis: Query<&mut Visibility>,
    spawn_hidden: Query<(), With<crate::moby_spawn::SpawnHidden>>,
    mut buffers: ResMut<Assets<ShaderBuffer>>,
) {
    let Some(mut rt) = rt else { return };
    let rt = &mut *rt;
    let Some(main_t) = main.iter().next().copied() else { return };
    let in_menu = mode.is_some_and(|m| m.state.mode == rc_game::menus::mode::Mode::Menu);
    let view = if in_menu { gp.view } else { GadgetsView::default() };
    let girl = if in_menu { gp.girl.clone() } else { None };
    let rects = [view.model.map(|m| m.rect), view.preview.map(|p| p.rect), view.ammo.map(|p| p.rect), view.gold_bolt.map(|p| p.rect), girl.as_ref().map(|g| g.rect)];
    // Each widget's canvas: the panel, the game projection's focal lengths, the view axis on the panel's centre, the
    // navy 0x80100808 clear.
    let proj = crate::game_camera::GameProjection::default();
    let focal = [crate::game_camera::SCREEN_W * 0.5 / proj.tan_x, crate::game_camera::SCREEN_H * 0.5 / proj.tan_y];
    for (k, (id, rect)) in rt.canvases.iter().zip(rects).enumerate() {
        // The Items page's gold-bolt panel shares its target with the 2D text (`DrawItemsMenu` 0x292528 draws the moby,
        // then "Found / Used / Remain" over it): its canvas is cleared transparent so the panel's navy and text (HUD
        // primitives, under the canvas) show around the bolt [L: the bolt lands over the text where they overlap].
        let clear = if k == 3 { Color::NONE } else { Color::srgb_u8(0x08, 0x08, 0x10) };
        let view = rect.map(|[x, y, w, h]| {
            let r = [x as f32, y as f32, w as f32, h as f32];
            CanvasView { rect: r, focal, centre: [r[0] + r[2] * 0.5, r[1] + r[3] * 0.5], clear }
        });
        canvases.show_exact(*id, view, (proj.tan_x, proj.tan_y));
    }
    // The animation steps since the last frame (the mobys advance once per menu frame, MobyUpdateLoop 0x2793d8).
    let steps = gp.frame.saturating_sub(rt.last_frame).min(8);
    rt.last_frame = gp.frame;
    let defs = play.as_ref().and_then(|p| p.game.item_data.as_ref());
    let class_of = |id: i32| defs.map(|d| d.def(id)).filter(|d| d.o_class > 0).map(|d| (d.o_class as i16, d.attach));
    let mut placed: Vec<Option<Placed>> = (0..rt.parts.len()).map(|_| None).collect();
    // The 3D Ratchet.
    if let Some(mv) = view.model {
        let want = |role: Role| -> Option<i16> {
            match role {
                Role::Ratchet => Some(0),
                Role::Clank => Some(CLANK_O_CLASS as i16),
                Role::Back => class_of(mv.equip[3]).map(|c| c.0),
                Role::Hand => class_of(mv.equip[0]).map(|c| c.0),
                Role::Head => class_of(mv.equip[2]).map(|c| c.0),
                Role::BootL | Role::BootR => class_of(mv.equip[1]).map(|c| c.0),
                _ => None,
            }
        };
        let rows = moby_light::rotation_rows([0.0, 0.0, std::f32::consts::PI]);
        let pos = [0, 1, 2].map(|k| frame::CAMERA_POS[k] + MODEL_OFFSET[k]);
        let Some(ri) = rt.parts.iter().position(|p| p.role == Role::Ratchet) else { return };
        {
            let r = &mut rt.parts[ri];
            if r.cut_for != 0 {
                moby_anim::hard_cut(&mut r.state, &r.anim, 0, 0);
                r.cut_for = 0;
            }
            for _ in 0..steps { moby_anim::advance(&mut r.state, &r.anim); }
        }
        let (rstate, ranim, rscale) = (rt.parts[ri].state, rt.parts[ri].anim.clone(), rt.parts[ri].scale);
        let chains: Vec<&[u8]> = rt.chains.iter().map(|c| c.1.as_slice()).collect();
        let ps = moby_anim::evaluate_chains(&ranim, &rstate, None, &chains);
        let ws: Vec<(usize, Rows)> = rt.chains.iter().zip(&ps).map(|(c, p)| (c.0, moby_anim::attach_matrix(p, &rows, pos, rscale))).collect();
        let w_of = |list: usize, normalise: bool| -> Option<([V4; 3], [f32; 3])> {
            let &(_, w) = ws.iter().find(|(l, _)| *l == list)?;
            let mut r: [V4; 3] = [0, 1, 2].map(|i| w[i].map(f32::to_bits));
            if normalise { moby_anim::normalise_columns(&mut r); }
            Some((r, [w[3][0], w[3][1], w[3][2]]))
        };
        placed[ri] = Some(Placed { rows, pos, pose: moby_anim::evaluate_with_snapshot(&ranim, &rstate, None) });
        let hand_attach = class_of(mv.equip[0]).map_or(0, |c| c.1.max(0) as usize);
        for (k, p) in rt.parts.iter_mut().enumerate() {
            if matches!(p.role, Role::Ratchet | Role::Item | Role::ItemClank) || (p.role != Role::Extra && want(p.role) != Some(p.o_class)) { continue; }
            match p.role {
                Role::Back => {
                    if p.cut_for != p.o_class as i32 {
                        // `LoadHandGadget`: a new moby; the Heli-Pack blends to its rotor sequence 6 over 10 ticks
                        // (`fun_00212f90(m, 6, 0, 10)` unless B is 6 already) with +0x20 = 8.
                        p.state = AnimState::spawn(&p.anim);
                        p.snap = None;
                        p.count = 0;
                        if p.o_class == HELI_O_CLASS {
                            if p.state.seq_b != 6 { moby_anim::set_sequence(&mut p.state, &p.anim, 6, 0, 10, &mut p.snap); }
                            p.count = 8;
                        }
                        p.cut_for = p.o_class as i32;
                    }
                    for _ in 0..steps { back_update(p); }
                    let Some((r, at)) = w_of(BACK_ATTACH, true) else { continue };
                    placed[k] = Some(Placed { rows: r, pos: at, pose: moby_anim::evaluate_with_snapshot(&p.anim, &p.state, p.snap.as_ref()) });
                }
                Role::Clank | Role::Hand => {
                    let seq = 1;
                    let key = seq as i32 * 1000 + p.o_class as i32;
                    if p.cut_for != key {
                        p.state = AnimState::spawn(&p.anim);
                        let s = seq.min(p.anim.sequences.len().saturating_sub(1) as u8);
                        moby_anim::hard_cut(&mut p.state, &p.anim, s, 0);
                        p.cut_for = key;
                    }
                    for _ in 0..steps { moby_anim::advance(&mut p.state, &p.anim); }
                    let (list, norm) = if p.role == Role::Hand { (hand_attach, hand_attach != 6) } else { (BACK_ATTACH, true) };
                    let Some((r, at)) = w_of(list, norm) else { continue };
                    placed[k] = Some(Placed { rows: r, pos: at, pose: moby_anim::evaluate_with_snapshot(&p.anim, &p.state, None) });
                }
                Role::Extra => {
                    let Some(e) = EXTRA_CLASSES.iter().position(|&c| c == p.o_class) else { continue };
                    if !mv.extras[e] { continue; }
                    if p.cut_for != p.o_class as i32 {
                        p.state = AnimState::spawn(&p.anim);
                        p.cut_for = p.o_class as i32;
                    }
                    // fun_002250f0: MobyAnimAdvance, then the joint matrix (rows normalised).
                    for _ in 0..steps { moby_anim::advance(&mut p.state, &p.anim); }
                    let Some((r, at)) = w_of(EXTRA_LISTS[e], true) else { continue };
                    placed[k] = Some(Placed { rows: r, pos: at, pose: moby_anim::evaluate_with_snapshot(&p.anim, &p.state, None) });
                }
                Role::Head | Role::BootL | Role::BootR => {
                    let (joints, extra, list): (&[u8], usize, usize) = match p.role {
                        Role::BootL => (&worn::LEFT_BOOT_JOINTS, 0, worn::LEFT_BOOT_ATTACH),
                        Role::BootR => (&worn::RIGHT_BOOT_JOINTS, 0, worn::RIGHT_BOOT_ATTACH),
                        _ if p.o_class == SONIC_O_CLASS => (&worn::SONIC_SUMMONER_JOINTS, 6, worn::HEAD_ATTACH),
                        _ => (&worn::HEAD_JOINTS, 0, worn::HEAD_ATTACH),
                    };
                    let Some((r, at)) = w_of(list, false) else { continue };
                    let Some(f) = worn::pose_from_host(&ranim, &rstate, None, &p.anim, joints, extra) else { continue };
                    placed[k] = Some(Placed { rows: r, pos: at, pose: moby_anim::evaluate_with_snapshot(&p.anim, &worn::posed_state(), Some(&f)) });
                }
                _ => {}
            }
        }
    }
    // The item preview.
    if let Some(pv) = view.preview {
        let rows = moby_light::rotation_rows(pv.rot);
        let pos = [0, 1, 2].map(|k| frame::CAMERA_POS[k] + pv.offset[k]);
        for (k, p) in rt.parts.iter_mut().enumerate() {
            let show = match p.role {
                Role::Item => p.o_class as i32 == pv.o_class,
                Role::ItemClank => pv.clank,
                _ => false,
            };
            if !show { continue; }
            let key = pv.item * 1000 + pv.seq as i32;
            if p.cut_for != key {
                p.state = AnimState::spawn(&p.anim);
                let s = pv.seq.min(p.anim.sequences.len().saturating_sub(1) as u8);
                moby_anim::hard_cut(&mut p.state, &p.anim, s, 0);
                p.cut_for = key;
            }
            for _ in 0..steps { moby_anim::advance(&mut p.state, &p.anim); }
            placed[k] = Some(Placed { rows, pos, pose: moby_anim::evaluate_with_snapshot(&p.anim, &p.state, None) });
        }
    }
    // The ammo model and the gold bolt: one moby each, on its sequence 0.
    for (v, role) in [(view.ammo, Role::Ammo), (view.gold_bolt, Role::GoldBolt)] {
        let Some(pv) = v else { continue };
        let rows = moby_light::rotation_rows(pv.rot);
        let pos = [0, 1, 2].map(|k| frame::CAMERA_POS[k] + pv.offset[k]);
        for (k, p) in rt.parts.iter_mut().enumerate() {
            if p.role != role || p.o_class as i32 != pv.o_class { continue; }
            if p.cut_for != pv.o_class {
                p.state = AnimState::spawn(&p.anim);
                moby_anim::hard_cut(&mut p.state, &p.anim, 0, 0);
                p.cut_for = pv.o_class;
            }
            for _ in 0..steps { moby_anim::advance(&mut p.state, &p.anim); }
            placed[k] = Some(Placed { rows, pos, pose: moby_anim::evaluate_with_snapshot(&p.anim, &p.state, None) });
        }
    }
    // The Helpdesk girl: posed by the widget's animation state (rc_game::menus::pause::media::girl_update / girl_tick).
    if let Some(gv) = girl.as_ref() {
        let rows = moby_light::rotation_rows([0.0, 0.0, std::f32::consts::PI]);
        let off = rc_game::menus::pause::media::GIRL_OFFSET;
        let pos = [0, 1, 2].map(|k| frame::CAMERA_POS[k] + off[k]);
        for (k, p) in rt.parts.iter_mut().enumerate() {
            if p.role != Role::Girl { continue; }
            p.state = gv.anim;
            placed[k] = Some(Placed { rows, pos, pose: moby_anim::evaluate_with_snapshot(&p.anim, &gv.anim, gv.snapshot.as_ref()) });
        }
    }
    // Records, palettes, visibility.
    let fold = main_t.to_matrix() * rt.menu_cam.inverse();
    let mut palette = buffers.get(&rt.extra.palette).and_then(|b| b.data.clone()).unwrap_or_default();
    let mut records = buffers.get(&rt.extra.instances).and_then(|b| b.data.clone()).unwrap_or_default();
    let before = (palette.clone(), records.clone());
    for (k, (p, pl)) in rt.parts.iter_mut().zip(&placed).enumerate() {
        let show = pl.is_some();
        if p.shown != show {
            p.shown = show;
            for &e in &p.entities { if let Ok(mut v) = vis.get_mut(e) { v.set_if_neq(if show { Visibility::Inherited } else { Visibility::Hidden }); } }
        }
        if show {
            // The gameplay setup hides the moby entities tagged with Ratchet's index by `SpawnHidden` (crate::menu_render).
            for &e in &p.entities { if spawn_hidden.contains(e) { commands.entity(e).remove::<crate::moby_spawn::SpawnHidden>(); } }
        }
        let Some(pl) = pl else { continue };
        let at = p.base as usize * 64;
        for (i, b) in pl.pose.iter().take(p.slots as usize).flat_map(|r| r.iter().flatten().flat_map(|v| v.to_le_bytes())).enumerate() {
            if let Some(d) = palette.get_mut(at + i) { *d = b; }
        }
        let lights = rt.bank.as_ref().map(|bank| moby_light::moby_lights(&pl.rows, bank, frame::LIGHT_WORD, frame::AMBIENT, 0x80));
        let model = fold * moby_render::extra_model(rows_f32(&pl.rows), p.scale, pl.pos);
        let r = moby_render::extra_record(&model, lights.as_ref(), p.base);
        let o = k * moby_render::EXTRA_RECORD_SIZE;
        if let Some(d) = records.get_mut(o..o + r.len()) { d.copy_from_slice(&r); }
        let t = Transform::from_matrix(model);
        for &e in &p.entities {
            if let Ok(mut tr) = transforms.get_mut(e) { if *tr != t { *tr = t; } }
        }
    }
    if (palette.clone(), records.clone()) != before {
        if let Some(mut b) = buffers.get_mut(&rt.extra.palette) { b.data = Some(palette); }
        if let Some(mut b) = buffers.get_mut(&rt.extra.instances) { b.data = Some(records); }
    }
}
