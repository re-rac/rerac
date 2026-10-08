//! GS pixel-pipeline state (TEST_1 alpha / depth test, ZBUF mask, ALPHA_1 blend) of every pass the port
//! draws, and how each maps onto Bevy's phases. Every material (tfrag, tie, shrub, billboard, moby, sky)
//! names its draw with a [`GsPass`] and applies it with [`GsPass::specialize`]; the fragment shaders
//! read the alpha-test half from the `GS_ATEST_PASS` / `GS_ATEST_FAIL` / `GS_AREF` shader defs.
//!
//! ## TEST_1 per pass (boot ELF SCUS_971.99 unless noted)
//!
//! | pass | TEST_1 | AREF | source |
//! |---|---|---|---|
//! | tfrag | 0x5360b | 0x60 | `ResetGsRegisters` table 0x13cfc0, re-sent after the sky by `transition_draw_sky` (docs/plan/sky_render_notes.md §3); the tfrag chain sends none (tfrag_render.rs) |
//! | tie, main list (VU1 13507) | 0x5360b | 0x60 | inherited: `TieProc` (fun_00235be8) sends no TEST_1 before its main list |
//! | tie, second list (program at 0x10ad20) | 0x5340b | 0x40 | block 0x1de920 sent at 0x236bc4; **not modelled** (the port draws every tie as the main list) |
//! | shrub mesh, opaque list | 0x5320b | 0x20 | block 0x1dea80 sent at 0x228e24, before the class loop of `ShrubProc` (fun_00228be8) |
//! | shrub mesh, fading list (alpha < 0x80, scratch 0x70003700) | 0x530cb | 0x0c | block 0x1deab0 sent at 0x2296dc, before the second class loop |
//! | shrub billboard pass 1 (alpha 0x80) | 0x5360b | 0x60 | block 0x1deb10 at 0x229970 (docs/plan/shrub_lighting.md §6) |
//! | shrub billboard pass 2 (alpha 1..0x7f) | 0x53001 | – | block 0x1deae0: ATST NEVER + AFAIL RGB_ONLY = blend, never Z |
//! | shrub clip list (VU1 912339) | 0x5320b | 0x20 | block 0x1dea80 again at 0x229d54; the port does not split the clip list off |
//! | moby | 0x5360b | 0x60 | `DrawMobysSetup` `VU1_addGSregister(0x47, 0x5360b)`, per moby 0x1dedc0 (docs/plan/moby_skinning_lighting.md) |
//! | sky gouraud / textured shell | 0x30000 / 0x3180b | – | A+D blocks 0x13d0f0 / 0x13d160: ZTST ALWAYS; ZMSK 0 / 1 (docs/plan/sky_render_notes.md §3) |
//!
//! Every world pass runs with ALPHA_1 = 0x8000000044, `Cout = ((Cs − Cd)·As >> 7) + Cd`, PRIM ABE = 1, ZTST
//! GEQUAL (larger Z is nearer; Bevy's reverse-Z `GreaterEqual` is the same test, later draw wins ties).
//! The TEST_1 words above all have ATE = 1, ATST = GEQUAL, AFAIL = RGB_ONLY (bits 12..13 = 3), ZTE = 1:
//! a pixel with As ≥ AREF writes blended RGB **and Z**; a pixel with As < AREF still writes blended RGB,
//! **not Z**. The GS runs texture mapping → fog → pixel test (alpha, then depth) → blend on each pixel, so
//! the alpha tested is the fragment's As after the texture function (MODULATE: `As = At·Af >> 7`,
//! clamped to 0xff), the same As that is the blend factor; fog changes RGB only (GS User's Manual,
//! "Drawing Environment / Pixel Test").
//!
//! ## In Bevy
//!
//! | [`GsPass`] | phase (`AlphaMode`) | blend | Z write | Z test | fragment discard |
//! |---|---|---|---|---|---|
//! | `Opaque` | Opaque3d (`Opaque`) | none (As = 0x80 everywhere, so the GS result is Cs) | on | GEQUAL | – |
//! | `OpaqueTested { aref }` | AlphaMask3d (`Mask`) | GS equation | on | GEQUAL | As < AREF |
//! | `ColorOnlyLowAlpha { aref }` | Transparent3d (`Blend`) | GS equation | off | GEQUAL | As ≥ AREF |
//! | `BlendNoZ` | Transparent3d (`Blend`) | GS equation | off | GEQUAL | – |
//! | `SkyDome` | Transparent3d on the sky camera | GS equation | on | ALWAYS | – |
//! | `SkyTextured` | Transparent3d on the sky camera | GS equation | off | ALWAYS | – (the 0x3180b test only gates Z, which ZMSK masks) |
//! | `LateTested { aref }` | Transparent3d (`Blend`) | GS equation | on | GEQUAL | As < AREF (moby metal pass: after every AlphaMask3d draw; a shadow caster's cut-out draws) |
//! | `LateOpaque` | Transparent3d (`Blend`) | none | on | GEQUAL | – (a shadow caster's opaque draws: after the shadow pass, crate::shadow_render) |
//! | `EffectMix` | Transparent3d (`Blend`) | GS equation on display bytes (crate::display_blend) | off | GEQUAL | – |
//! | `AdditiveNoZ` | Transparent3d (`Blend`) | `Cs·As + Cd` on display bytes (crate::display_blend) | off | GEQUAL | – |
//! | `EffectLowAlpha { aref }` | Transparent3d (`Blend`) | GS equation on display bytes (crate::display_blend) | off | GEQUAL | As ≥ AREF (a moby glow packet's `ColorOnlyLowAlpha` half) |
//! | `Hud` | Transparent2d on the HUD camera (crate::hud_render) | GS equation | off | ALWAYS | – |
//!
//! `Hud` is TEST_1 0x5380b (ATE GEQUAL 0x80, AFAIL RGB_ONLY, ZTST GEQUAL) with every 2D primitive at Z 0xfffff0,
//! above any 3D Z: the depth test always passes and RGB is written whether or not As ≥ 0x80, so the two-draw
//! split collapses to one blended draw (it would only decide Z and destination-alpha writes, neither visible).
//!
//! A batch (one mesh, one texture state) whose As can differ from 0x80 is drawn twice ([`draws`]): draw A
//! (`OpaqueTested`) keeps the fragments that pass the GS alpha test, blends them and writes Z; draw B
//! (`ColorOnlyLowAlpha`) keeps the ones that fail it and blends them without Z. Together every fragment is
//! drawn once with exactly the GS colour/Z rule. The GS equation is `SrcAlpha, OneMinusSrcAlpha, Add` on RGB
//! with the fragment alpha = As/128 (exact at As = 0 and 0x80; the GPU mixes in linear light where the GS
//! mixes display bytes, and As > 0x80 clamps to 1, which no Novalis fragment reaches).
//!
//! Draw order, and where it differs from the GS (which draws in DMA-chain packet order: sky, tfrag, tie,
//! shrub, billboards, moby, see docs/plan/render_pipeline.md §2):
//! * draw A is in AlphaMask3d, which Bevy renders after the whole Opaque3d phase, so every blended,
//!   Z-writing fragment lands on the fully opaque geometry behind it (in the GS order it lands on whatever
//!   the earlier passes drew). Among AlphaMask3d items Bevy's binned order (pipeline, bind group, mesh)
//!   replaces packet order; that matters only where two such items overlap with 0x60 ≤ As < 0x80;
//! * draw B is in Transparent3d, sorted back to front by entity origin (then by entity, see
//!   determinism.rs), not in packet order;
//! * within one moby class, packet order decides equal-depth overlaps between texture batches (later packet
//!   wins GEQUAL). Bevy draws one class's batches in bin order instead. Measured in bind pose
//!   (`moby_render::tests::moby_cross_batch_coplanar_overlaps`): classes 0 (Ratchet) and 577 have **no**
//!   coplanar, overlapping triangle pairs from different texture batches, so the batch order cannot change
//!   a pixel there; level-wide 3 of 169 Novalis classes have some (725: 6 pairs, 731: 4, 790: 4, one batch
//!   pair each), not ordered (Bevy's `depth_bias` only sorts Transparent3d and Transmissive3d, not the
//!   binned phases). Within one batch the index order is the packet order.
//!
//! `EffectMix` and `AdditiveNoZ` are the effect mobys' draws (crate::moby_render `MobyBlend::Translucent` /
//! `Additive`: one draw, Z tested, not written, sorted with the blended items). They are not GS register models: they
//! blend on the frame's display bytes like every effect (crate::display_blend, user decision 2026-09-27; the shader
//! gets `DISPLAY_BLEND_MIX` / `DISPLAY_BLEND_ADD`, the pipeline targets the effect pass's display-encoded target, and
//! the entity carries `DisplayEffect`).

//! `RC_GS_ALPHA=0` restores the previous mapping (any As ≠ 0x80 → one `BlendNoZ` draw) for comparisons.

use bevy::prelude::*;
use bevy::render::render_resource::{
    BlendComponent, BlendFactor, BlendOperation, BlendState, CompareFunction, RenderPipelineDescriptor,
};
use bevy::shader::ShaderDefVal;
use std::sync::OnceLock;

/// TEST_1 words of the passes (table in the module doc). The ones without an AREF split are kept for the
/// record and checked by the tests.
#[allow(dead_code)]
pub mod test_1 {
    pub const WORLD: u64 = 0x5360b;
    pub const SHRUB_OPAQUE_LIST: u64 = 0x5320b;
    pub const SHRUB_FADING_LIST: u64 = 0x530cb;
    pub const BILLBOARD_FADING: u64 = 0x53001;
    pub const TIE_SECOND_LIST: u64 = 0x5340b;
    pub const SKY_GOURAUD: u64 = 0x30000;
    pub const SKY_TEXTURED: u64 = 0x3180b;
    /// HUD / text / 2D overlays (`fun_00233c28` replays block 0x1c2970 after the AA blit).
    pub const HUD: u64 = 0x5380b;
}

/// GS TEST_1 fields (GS User's Manual register 0x47).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Test1 {
    /// Alpha test enabled.
    pub ate: bool,
    /// 0 NEVER, 1 ALWAYS, 2 LESS, 3 LEQUAL, 4 EQUAL, 5 GEQUAL, 6 GREATER, 7 NOTEQUAL.
    pub atst: u8,
    pub aref: u8,
    /// 0 KEEP, 1 FB_ONLY, 2 ZB_ONLY, 3 RGB_ONLY.
    pub afail: u8,
    /// Depth test enabled.
    pub zte: bool,
    /// 0 NEVER, 1 ALWAYS, 2 GEQUAL, 3 GREATER.
    pub ztst: u8,
}

impl Test1 {
    pub const fn decode(v: u64) -> Self {
        Test1 {
            ate: v & 1 != 0,
            atst: ((v >> 1) & 7) as u8,
            aref: ((v >> 4) & 0xff) as u8,
            afail: ((v >> 12) & 3) as u8,
            zte: (v >> 16) & 1 != 0,
            ztst: ((v >> 17) & 3) as u8,
        }
    }

    /// AREF of an "As ≥ AREF writes Z, else RGB only" test (ATE, GEQUAL, RGB_ONLY), the form of every
    /// world pass.
    pub const fn z_write_aref(v: u64) -> u8 {
        let t = Self::decode(v);
        assert!(t.ate && t.atst == 5 && t.afail == 3 && t.zte && t.ztst == 2);
        t.aref
    }
}

/// AREF of the tfrag, tie (main list), moby and billboard pass-1 passes.
pub const AREF_WORLD: u8 = Test1::z_write_aref(test_1::WORLD);
/// AREF of the shrub mesh opaque list.
pub const AREF_SHRUB_OPAQUE: u8 = Test1::z_write_aref(test_1::SHRUB_OPAQUE_LIST);
/// AREF of the shrub mesh fading list.
pub const AREF_SHRUB_FADING: u8 = Test1::z_write_aref(test_1::SHRUB_FADING_LIST);

/// One draw's GS state, see the module doc.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum GsPass {
    /// As = 0x80 on every fragment: the GS writes Cs and Z.
    Opaque,
    /// The fragments with As ≥ `aref`: blended, Z written.
    OpaqueTested { aref: u8 },
    /// The fragments with As < `aref` (AFAIL = RGB_ONLY): blended, no Z.
    ColorOnlyLowAlpha { aref: u8 },
    /// Blend without alpha test or Z write (billboard pass 2, and the `RC_GS_ALPHA=0` mapping).
    BlendNoZ,
    /// Sky gouraud shell: blended, Z written, ZTST ALWAYS.
    SkyDome,
    /// Sky textured shell: blended, ZMSK, ZTST ALWAYS.
    SkyTextured,
    /// 2D HUD primitives: blended, no Z (see the module doc).
    Hud,
    /// `OpaqueTested` drawn in Transparent3d instead of AlphaMask3d: the moby metal (shine) pass, which the game
    /// draws right after the moby's own packets, so it must land on the moby's AlphaMask3d fragments too.
    LateTested { aref: u8 },
    /// `Cs·As + Cd` on display bytes (crate::display_blend), no Z write (effect mobys flagged additive).
    AdditiveNoZ,
    /// `(Cs − Cd)·As + Cd` on display bytes (crate::display_blend), no Z write (translucent effect mobys).
    EffectMix,
    /// `ColorOnlyLowAlpha` on display bytes (crate::display_blend): the soft edge (As < AREF) of a moby's glow
    /// packets (crate::moby_render, the MobyProc glow list).
    EffectLowAlpha { aref: u8 },
    /// `Opaque` drawn in Transparent3d: a shadow caster's opaque draws, which the game draws after the shadow pass
    /// (crate::moby_render `caster_pass`, crate::shadow_render).
    LateOpaque,
    /// `Opaque` on display bytes (crate::display_blend): a moby drawn in moby order (crate::moby_render
    /// `ExtraMobys::set_ordered`), whose later colour-only halves blend on display bytes in the same order.
    EffectOpaque,
    /// `OpaqueTested` on display bytes, likewise.
    EffectTested { aref: u8 },
}

/// Which fragments a draw discards.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AlphaDiscard {
    None,
    /// As < AREF (the GS alpha test's failing half).
    Below(u8),
    /// As ≥ AREF (its passing half).
    AtOrAbove(u8),
}

/// The pipeline state a [`GsPass`] stands for.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct GsState {
    pub blend: bool,
    /// Additive (`Cs·As + Cd`) instead of the alpha mix.
    pub additive: bool,
    /// Blended on display bytes by the shader (crate::display_blend).
    pub display: bool,
    pub depth_write: bool,
    pub depth_compare: CompareFunction,
    pub discard: AlphaDiscard,
}

/// `(Cs − Cd)·As + Cd` with As = fragment alpha (ALPHA_1 = 0x8000000044; FIX unused).
pub const GS_BLEND: BlendState = BlendState {
    color: BlendComponent { src_factor: BlendFactor::SrcAlpha, dst_factor: BlendFactor::OneMinusSrcAlpha, operation: BlendOperation::Add },
    alpha: BlendComponent::OVER,
};

impl GsPass {
    pub const fn state(self) -> GsState {
        use AlphaDiscard as D;
        use CompareFunction::{Always, GreaterEqual};
        let (blend, depth_write, depth_compare, discard) = match self {
            GsPass::Opaque => (false, true, GreaterEqual, D::None),
            GsPass::OpaqueTested { aref } => (true, true, GreaterEqual, D::Below(aref)),
            GsPass::ColorOnlyLowAlpha { aref } => (true, false, GreaterEqual, D::AtOrAbove(aref)),
            GsPass::BlendNoZ => (true, false, GreaterEqual, D::None),
            GsPass::SkyDome => (true, true, Always, D::None),
            GsPass::SkyTextured => (true, false, Always, D::None),
            GsPass::Hud => (true, false, Always, D::None),
            GsPass::LateTested { aref } => (true, true, GreaterEqual, D::Below(aref)),
            GsPass::AdditiveNoZ | GsPass::EffectMix => (true, false, GreaterEqual, D::None),
            GsPass::EffectLowAlpha { aref } => (true, false, GreaterEqual, D::AtOrAbove(aref)),
            GsPass::LateOpaque => (false, true, GreaterEqual, D::None),
            GsPass::EffectOpaque => (true, true, GreaterEqual, D::None),
            GsPass::EffectTested { aref } => (true, true, GreaterEqual, D::Below(aref)),
        };
        let additive = matches!(self, GsPass::AdditiveNoZ);
        let display = matches!(self, GsPass::AdditiveNoZ | GsPass::EffectMix | GsPass::EffectLowAlpha { .. } | GsPass::EffectOpaque | GsPass::EffectTested { .. });
        GsState { blend, additive, depth_write, depth_compare, discard, display }
    }

    /// The Bevy phase: Opaque3d, AlphaMask3d (drawn after all of Opaque3d) or Transparent3d.
    pub fn alpha_mode(self) -> AlphaMode {
        match self {
            GsPass::Opaque => AlphaMode::Opaque,
            // The cutoff is not read: the shader discards per `GS_AREF`.
            GsPass::OpaqueTested { .. } => AlphaMode::Mask(0.5),
            _ => AlphaMode::Blend,
        }
    }

    /// Sets blend, depth write / compare and the alpha-test shader defs. Call after the material's own
    /// changes; Bevy's mesh pipeline has already set the defaults for [`Self::alpha_mode`].
    pub fn specialize(self, descriptor: &mut RenderPipelineDescriptor) {
        let s = self.state();
        if let Some(ds) = descriptor.depth_stencil.as_mut() {
            ds.depth_write_enabled = Some(s.depth_write);
            ds.depth_compare = Some(s.depth_compare);
        }
        if let Some(f) = descriptor.fragment.as_mut() {
            for t in f.targets.iter_mut().flatten() { t.blend = s.blend.then_some(GS_BLEND); }
            if s.display { f.shader_defs.push(if s.additive { "DISPLAY_BLEND_ADD" } else { "DISPLAY_BLEND_MIX" }.into()); }
            let (def, aref) = match s.discard {
                AlphaDiscard::None => (None, 0),
                AlphaDiscard::Below(a) => (Some("GS_ATEST_PASS"), a),
                AlphaDiscard::AtOrAbove(a) => (Some("GS_ATEST_FAIL"), a),
            };
            if let Some(d) = def { f.shader_defs.push(d.into()); }
            f.shader_defs.push(ShaderDefVal::UInt("GS_AREF".into(), aref as u32));
        }
        // Effect draws: the display-encoded target and its blend (crate::display_blend).
        if s.display { crate::display_blend::specialize(descriptor); }
    }
}

/// Range of GS alpha values (0..0xff, 0x80 = 1.0) a texture or a vertex colour set can produce.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct AlphaRange {
    pub min: u8,
    pub max: u8,
}

impl AlphaRange {
    pub const OPAQUE: AlphaRange = AlphaRange { min: 0x80, max: 0x80 };

    /// Range of the given values (OPAQUE when empty).
    pub fn of(values: impl IntoIterator<Item = u8>) -> Self {
        values.into_iter().fold(None, |r: Option<AlphaRange>, a| {
            Some(match r { None => AlphaRange { min: a, max: a }, Some(r) => AlphaRange { min: r.min.min(a), max: r.max.max(a) } })
        })
        .unwrap_or(Self::OPAQUE)
    }

    pub fn union(self, o: AlphaRange) -> Self { AlphaRange { min: self.min.min(o.min), max: self.max.max(o.max) } }

    pub fn is_opaque(self) -> bool { self == Self::OPAQUE }
}

/// `RC_GS_ALPHA=0`: the previous single-draw `BlendNoZ` mapping.
pub fn gs_alpha_enabled() -> bool {
    static ON: OnceLock<bool> = OnceLock::new();
    *ON.get_or_init(|| !std::env::var("RC_GS_ALPHA").is_ok_and(|v| v.trim() == "0"))
}

/// The draws that reproduce a batch under a Z-writing alpha test with `aref`, given the texel alpha range
/// of its texture (all mip levels) and the vertex (Af) alpha range. Bilinear filtering only interpolates
/// between texels, so As = At·Af >> 7 lies in `[tmin·vmin >> 7, tmax·vmax >> 7]`; a half of the split that
/// no fragment can reach is not drawn.
pub fn draws(aref: u8, texel: AlphaRange, vertex: AlphaRange) -> Vec<GsPass> {
    if texel.is_opaque() && vertex.is_opaque() { return vec![GsPass::Opaque]; }
    if !gs_alpha_enabled() { return vec![GsPass::BlendNoZ]; }
    let lo = ((texel.min as u32 * vertex.min as u32) >> 7).min(0xff);
    let hi = ((texel.max as u32 * vertex.max as u32) >> 7).min(0xff);
    let mut out = Vec::with_capacity(2);
    if hi >= aref as u32 { out.push(GsPass::OpaqueTested { aref }); }
    if lo < aref as u32 { out.push(GsPass::ColorOnlyLowAlpha { aref }); }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_1_words() {
        assert_eq!(AREF_WORLD, 0x60);
        assert_eq!(AREF_SHRUB_OPAQUE, 0x20);
        assert_eq!(AREF_SHRUB_FADING, 0x0c);
        assert_eq!(Test1::z_write_aref(test_1::TIE_SECOND_LIST), 0x40);
        // Billboard pass 2: ATST NEVER, AFAIL RGB_ONLY: every pixel fails, writes RGB, never Z.
        let p2 = Test1::decode(test_1::BILLBOARD_FADING);
        assert_eq!((p2.ate, p2.atst, p2.afail, p2.zte, p2.ztst), (true, 0, 3, true, 2));
        // Sky: no alpha test / an FB_ONLY one (Z masked anyway), ZTST ALWAYS.
        let (g, t) = (Test1::decode(test_1::SKY_GOURAUD), Test1::decode(test_1::SKY_TEXTURED));
        assert_eq!((g.ate, g.zte, g.ztst), (false, true, 1));
        assert_eq!((t.ate, t.atst, t.aref, t.afail, t.ztst), (true, 5, 0x80, 1, 1));
        // HUD: A ≥ 0x80 writes Z, else RGB only; ZTST GEQUAL.
        assert_eq!(Test1::z_write_aref(test_1::HUD), 0x80);
    }

    #[test]
    fn draw_split() {
        let o = AlphaRange::OPAQUE;
        assert_eq!(draws(0x60, o, o), vec![GsPass::Opaque]);
        // Cut-out texture (0 and 0x80 texels): both halves.
        let cut = AlphaRange { min: 0, max: 0x80 };
        assert_eq!(draws(0x60, cut, o), vec![GsPass::OpaqueTested { aref: 0x60 }, GsPass::ColorOnlyLowAlpha { aref: 0x60 }]);
        // Only 0x60..0x80 texels: nothing fails the test.
        assert_eq!(draws(0x60, AlphaRange { min: 0x60, max: 0x80 }, o), vec![GsPass::OpaqueTested { aref: 0x60 }]);
        // Everything below AREF: colour only.
        assert_eq!(draws(0x60, AlphaRange { min: 0, max: 0x40 }, o), vec![GsPass::ColorOnlyLowAlpha { aref: 0x60 }]);
        // Fading opaque texture: vertex alpha 0..0x7f.
        assert_eq!(
            draws(0x0c, o, AlphaRange { min: 0, max: 0x7f }),
            vec![GsPass::OpaqueTested { aref: 0x0c }, GsPass::ColorOnlyLowAlpha { aref: 0x0c }]
        );
    }
}
