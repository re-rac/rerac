//! The after-images (`rc_game::afterimage`, docs/plan/hero_states.md "After-images"): the game's "speed blur" behind
//! the Thruster-Pack jumps and the Comet-Strike's wrench. The game draws each ghost as a moby of its owner's class
//! (`0x277428`: mode 0x80a, alpha +0x23 = 0x28 / 0x14 / 0x0a …), in its owner's current anim keys at a placement a few
//! ticks old; MobyProc draws it blended (mode bit 8, the fading group), lit with the owner's light word.
//!
//! Here each ghost is an extra instance of the owner's class ([`moby_render::ExtraMobys`]: its own record and palette
//! range, four slots per record), posed by `moby_anim::evaluate_posed` from the ghost's keys (no pose layers, no joint
//! modifiers: the ghost moby has none), placed at the ghost's rows / position with the owner's scale, drawn with the
//! ghost's alpha as the vertex alpha and mode 0x80a (`MobyBlend` picks the blended group: alpha blend on display
//! bytes, no depth write, sorted back to front). Owners: Ratchet (his level class, placed instance of class 0) and the
//! wrench (class 71 from the gadget table, `crate::moby_attach::load_blobs`). Native: plain meshes and a shader
//! blend, no frame-buffer feedback.
//!
//! Not modelled: the ghosts of Ratchet use his high LOD (the game sets their LOD distance +0x72 to 0: always the low
//! LOD); MobyProc's draw-distance / frustum culls and distance fade on the ghosts; a snapshot key (blend from a
//! snapshot) uses the owner's current snapshot frame.

use crate::gameplay::Play;
use crate::moby_render::{self, ExtraMobys, MobyMaterial, SlotLook};
use bevy::prelude::*;
use bevy::render::storage::ShaderBuffer;
use rc_formats::moby::LevelMobyClass;
use rc_formats::moby_anim::{self, AnimState, MobyAnimClass, MobyFrame};
use rc_formats::moby_light as light;
use rc_game::afterimage::{Place, Pose, Trail, MAX_GHOSTS};

/// The ghosts' mode bits (`0x277428`: +0x34 = 0x80a).
const GHOST_MODE: u16 = 0x80a;

pub struct AfterImagePlugin;

impl Plugin for AfterImagePlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            PostUpdate,
            draw.before(bevy::camera::visibility::VisibilitySystems::VisibilityPropagate).before(bevy::asset::AssetEventSystems),
        );
    }
}

/// An owner class: its model and animation, the scale its ghosts are drawn at.
struct Owner {
    class: LevelMobyClass,
    anim: MobyAnimClass,
    scale: f32,
}

/// The ghosts' record set: two records (Ratchet's, the wrench's) of [`MAX_GHOSTS`] slots each.
struct Ghosts {
    extra: ExtraMobys,
    /// Palette matrices per slot.
    pal: u32,
    records: Vec<u8>,
    palette: Vec<u8>,
    owners: [Option<Owner>; 2],
    drawn: Option<u64>,
}

/// The owners: Ratchet's placed class-0 instance and the gadget table's wrench.
fn owners(lv: &crate::level_load::LoadedLevel) -> [Option<Owner>; 2] {
    let m = &lv.mobys;
    let ratchet = m.instances.iter().zip(&m.placed).find(|(i, p)| i.o_class == 0 && p.is_some()).and_then(|(_, p)| {
        let p = (*p)?;
        Some(Owner { class: m.classes[p.class].clone(), anim: m.anim[p.class].clone(), scale: p.scale })
    });
    let wrench = crate::moby_attach::load_blobs().ok().and_then(|(_, gadgets)| {
        let g = gadgets.into_iter().find(|g| g.moby.o_class == rc_formats::gadget::WRENCH_O_CLASS)?;
        let seqs = moby_anim::parse_sequences(&g.blob, &g.moby.class).ok()?;
        let scale = g.moby.class.header.scale;
        Some(Owner { anim: MobyAnimClass::new(&g.moby.class, seqs), class: g.moby, scale })
    });
    [ratchet, wrench]
}

fn state_of(p: &Pose) -> AnimState {
    AnimState { seq_a: p.seq_a, frame_a: p.frame_a, seq_b: p.seq_b, frame_b: p.frame_b, t: p.t, speed: 1.0, rate: 1.0, flags: 0, trigger_count: 0, skip_advance: false }
}

#[allow(clippy::too_many_arguments)]
fn draw(
    mut state: Local<Option<Ghosts>>,
    generation: Res<crate::level_switch::LevelGeneration>,
    play: Option<Res<Play>>,
    level: Res<crate::Level>,
    mut commands: Commands,
    mut buffers: ResMut<Assets<ShaderBuffer>>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut images: ResMut<Assets<Image>>,
    mut materials: ResMut<Assets<MobyMaterial>>,
) {
    // A runtime level change (crate::level_switch): the ghosts' entities are gone.
    if generation.is_changed() { *state = None; }
    let Some(p) = play else { return };
    let lv = &level.0;
    let g = state.get_or_insert_with(|| {
        let owners = owners(lv);
        let pal = owners.iter().flatten().map(|o| (o.anim.joint_count as u32).max(ExtraMobys::max_skinned_joint(&o.class) as u32 + 1)).max().unwrap_or(1).max(1);
        let slots = 2 * MAX_GHOSTS;
        let records = vec![0; slots * moby_render::EXTRA_RECORD_SIZE];
        let palette = crate::moby_anim::identity_palette(slots as u32 * pal);
        let extra = ExtraMobys::new(lv, records.clone(), palette.clone(), &mut buffers);
        Ghosts { extra, pal, records, palette, owners, drawn: None }
    });
    let counter = p.game.counter;
    if g.drawn == Some(counter) { return; }
    g.drawn = Some(counter);
    let hero = &p.game.hero;
    let trails: [&Trail; 2] = [&hero.fx.trails.hero, &hero.fx.trails.wrench];
    // Ratchet's light word and ambient (the ghosts and the items copy his +0x38..+0x3f).
    let (light_word, ambient) = p.game.mobys.hero().and_then(|h| p.game.mobys.mobys.get(h)).map_or((0, [0x40; 3]), |m| (m.light, [m.ambient[0], m.ambient[1], m.ambient[2]]));
    let snaps: [Option<&MobyFrame>; 2] = [p.ratchet.snapshot.as_ref(), hero.items.slot.item.as_ref().and_then(|i| i.snapshot.as_ref())];
    let (mut rec_changed, mut pal_changed) = (false, false);
    let pal_bytes = g.pal as usize * 64;
    for (k, trail) in trails.iter().enumerate() {
        let shown: Vec<_> = trail.shown().collect();
        for j in 0..MAX_GHOSTS {
            let slot = k * MAX_GHOSTS + j;
            let want = shown.get(j).copied().zip(g.owners[k].as_ref()).filter(|((_, _, _), o)| o.class.o_class as i16 == trail.owner_class);
            let Some(((ghost, place, pose), owner)) = want else {
                g.extra.show_slot(&mut commands, lv, slot as u32, None, &mut meshes, &mut images, &mut materials, &mut buffers);
                continue;
            };
            let Place { rows, position } = place;
            let r3 = [0, 1, 2].map(|i| [rows[i][0], rows[i][1], rows[i][2]]);
            let model = moby_render::extra_model(r3, owner.scale, [position[0], position[1], position[2]]);
            let look = SlotLook { model, alpha: ghost.alpha, fading: false, mode: GHOST_MODE, glow: 0, shine: 0, e: [[0.0; 3]; 3], late: true };
            g.extra.show_slot(&mut commands, lv, slot as u32, Some((&owner.class, look)), &mut meshes, &mut images, &mut materials, &mut buffers);
            let bits = [0, 1, 2].map(|i| rows[i].map(f32::to_bits));
            let lights = lv.mobys.lighting.as_ref().map(|l| light::moby_lights(&bits, &l.bank, light_word, ambient, 0x80));
            let rec = moby_render::extra_record(&model, lights.as_ref(), slot as u32 * g.pal);
            let at = slot * moby_render::EXTRA_RECORD_SIZE;
            if g.records[at..at + rec.len()] != rec[..] {
                g.records[at..at + rec.len()].copy_from_slice(&rec);
                rec_changed = true;
            }
            let snap = if pose.seq_a == moby_anim::SNAPSHOT_SEQ { snaps[k] } else { None };
            let f = moby_anim::evaluate_posed(&owner.anim, &state_of(&pose), snap, &[], &[]);
            let bytes: Vec<u8> = f.iter().take(g.pal as usize).flat_map(|r| r.iter().flatten().flat_map(|v| v.to_le_bytes())).collect();
            let at = slot * pal_bytes;
            if g.palette[at..at + bytes.len()] != bytes[..] {
                g.palette[at..at + bytes.len()].copy_from_slice(&bytes);
                pal_changed = true;
            }
        }
    }
    if rec_changed {
        crate::asset_write::set_buffer(&mut buffers, &g.extra.instances, &g.records);
    }
    if pal_changed {
        crate::asset_write::set_buffer(&mut buffers, &g.extra.palette, &g.palette);
    }
}
