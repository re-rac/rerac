//! The vendor's placement data (docs/plan/interaction.md §9): the offsets of the camera, the exit, the salesman,
//! the item model and the hologram in the vendor's frame, the screens' margins, the item tables and the hologram
//! cone, read from the level overlay by their level-01 addresses (on another level through
//! `menus::Overlay::relocated`, which maps [`DATA_LABELS`]).
//!
//! | label | what |
//! |---|---|
//! | 0x161060 (gp − 0x5ba0) | camera eye (3.8, 0, 1.5) in the vendor's frame (`OpenVendorMenu`) |
//! | 0x161070 (gp − 0x5b90) | Ratchet's place after the exit (3.5, 0, 0) (`VendorExit`) |
//! | 0x161080 (gp − 0x5b80) | the salesman (−0.5, −1.8, 0.2) |
//! | 0x161090 (gp − 0x5b70) | the item panel model's base (0, −1.6, 1.5) (`FUN_002af3f0`) |
//! | 0x1610a0 (gp − 0x5b60) | the item hologram's base (−1.0, 0.05, 1.3) |
//! | 0x1610b0 / 0x1610c0 | the two class-13 mobys (−10.2, −4.25, 0) / (−9.5, −5.85, 0.85) |
//! | 0x1610d0 / 0x1610e0 | light set 14: direction (−0.42, −0.7, −0.577) (× the vendor's rows), colour (0.9, 0.9, 0.6) |
//! | 0x161110.. | the salesman's voice-start key times per talk state and set (`FUN_002aee20` cases 4..6) |
//! | 0x1ca798 | the six screens' margins (4 floats each, `VendorDrawScreens` 0x2b3130) |
//! | 0x1c8d90 + 0x30·item | item panel: rotation x, y, z (+0), offset (+0x10), extra z of a weapon (+0x1c) |
//! | 0x1c9480 + 0x40·item | ammo hologram: rotation x, y (+0), offset (+0x10), class (+0x20, 0: none) |
//! | 0x1c9db0 + 0x30·item | weapon hologram: post offset (+0), rotation x, y (+0x10), offset (+0x20) |
//! | 0x1ca800 / 0x1ca890 / 0x1ca8d8 / 0x1ca900 | the menu's hologram cone: 9 vertices, 9 UVs, 9 RGBA, 4 quads |
//! | 0x1d77e0 / 0x1d7870 / 0x1d78b8 / 0x1d78e0 | the approach beam of class 11's draw callback 0x2ba9c0: the same layout |
//! | 0x1ca4a0 | the weapon demo scene per item (−1: none) |

use crate::menus::Overlay;

/// Items with placement records.
pub const ITEMS: usize = 37;

pub mod addr {
    pub const CAMERA: u32 = 0x161060;
    pub const EXIT: u32 = 0x161070;
    pub const SALESMAN: u32 = 0x161080;
    pub const ITEM_BASE: u32 = 0x161090;
    pub const HOLO_BASE: u32 = 0x1610a0;
    pub const SPIN0: u32 = 0x1610b0;
    pub const SPIN1: u32 = 0x1610c0;
    pub const LIGHT_DIR: u32 = 0x1610d0;
    pub const LIGHT_COLOR: u32 = 0x1610e0;
    pub const TALK_KEYS: u32 = 0x161110;
    pub const MARGINS: u32 = 0x1ca798;
    pub const PANEL_ROT: u32 = 0x1c8d90;
    pub const PANEL_OFF: u32 = 0x1c8da0;
    pub const HOLO_ROT: u32 = 0x1c9480;
    pub const HOLO_OFF: u32 = 0x1c9490;
    pub const HOLO_CLASS: u32 = 0x1c94a0;
    pub const WHOLO_POST: u32 = 0x1c9db0;
    pub const WHOLO_ROT: u32 = 0x1c9dc0;
    pub const WHOLO_OFF: u32 = 0x1c9dd0;
    pub const CONE_VERTS: u32 = 0x1ca800;
    pub const CONE_UV: u32 = 0x1ca890;
    pub const CONE_RGBA: u32 = 0x1ca8d8;
    pub const CONE_QUADS: u32 = 0x1ca900;
    pub const BEAM_VERTS: u32 = 0x1d77e0;
    pub const BEAM_UV: u32 = 0x1d7870;
    pub const BEAM_RGBA: u32 = 0x1d78b8;
    pub const BEAM_QUADS: u32 = 0x1d78e0;
    pub const DEMO: u32 = 0x1ca4a0;
}

/// The labels `Overlay::relocated` maps on the other levels.
pub const DATA_LABELS: &[u32] = &[
    addr::CAMERA, addr::EXIT, addr::SALESMAN, addr::ITEM_BASE, addr::HOLO_BASE, addr::SPIN0, addr::SPIN1, addr::LIGHT_DIR,
    addr::LIGHT_COLOR, addr::TALK_KEYS, addr::MARGINS, addr::PANEL_ROT, addr::PANEL_OFF, addr::HOLO_ROT, addr::HOLO_OFF,
    addr::HOLO_CLASS, addr::WHOLO_POST, addr::WHOLO_ROT, addr::WHOLO_OFF, addr::CONE_VERTS, addr::CONE_UV, addr::CONE_RGBA,
    addr::CONE_QUADS, addr::BEAM_VERTS, addr::BEAM_UV, addr::BEAM_RGBA, addr::BEAM_QUADS, addr::DEMO,
];

/// A cone of 4 textured quads over 9 vertices (`VendorDrawHologramCone` 0x2b3cc8, the class-11 callback 0x2ba9c0).
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Cone {
    /// Vertices in the vendor's frame.
    pub verts: [[f32; 3]; 9],
    pub uv: [[f32; 2]; 9],
    pub rgba: [u32; 9],
    /// Corner indices per quad (GS strip order).
    pub quads: [[usize; 4]; 4],
}

/// The item panel record 0x1c8d90 + 0x30·item.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct PanelPlace {
    pub rot: [f32; 3],
    pub offset: [f32; 3],
    /// +0x1c: added to z once more for a weapon (twice under the discount).
    pub weapon_dz: f32,
}

/// The ammo hologram record 0x1c9480 + 0x40·item.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct AmmoHolo {
    pub rot: [f32; 2],
    pub offset: [f32; 3],
    /// +0x20: the class drawn (0: the ammo entry shows no hologram).
    pub class: i32,
}

/// The weapon hologram record 0x1c9db0 + 0x30·item.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct WeaponHolo {
    /// +0: shifted by after the rotation (in the hologram's own frame).
    pub post: [f32; 3],
    pub rot: [f32; 2],
    pub offset: [f32; 3],
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct VendorLayout {
    pub camera: [f32; 3],
    pub exit: [f32; 3],
    pub salesman: [f32; 3],
    pub item_base: [f32; 3],
    pub holo_base: [f32; 3],
    pub spin: [[f32; 3]; 2],
    pub light_dir: [f32; 3],
    pub light_color: [f32; 4],
    /// Per talk state 4 / 5 / 6, per set k: the key time (1/16 frames) at which the voice starts.
    pub talk_keys: [[i32; 2]; 3],
    /// Per screen: (inset along edge 1, inset along edge 2, shrink of edge 1 (×2), shrink of edge 2 (×2)).
    pub margins: [[f32; 4]; 6],
    pub panel: Vec<PanelPlace>,
    pub ammo_holo: Vec<AmmoHolo>,
    pub weapon_holo: Vec<WeaponHolo>,
    pub cone: Cone,
    pub beam: Cone,
    pub demo: Vec<i32>,
}

fn f(ov: &Overlay, a: u32) -> Option<f32> { ov.u32(a).map(f32::from_bits) }
fn v3(ov: &Overlay, a: u32) -> Option<[f32; 3]> { Some([f(ov, a)?, f(ov, a + 4)?, f(ov, a + 8)?]) }

fn cone(ov: &Overlay, verts: u32, uv: u32, rgba: u32, quads: u32) -> Option<Cone> {
    let (verts, uv, rgba, quads) = (ov.at(verts), ov.at(uv), ov.at(rgba), ov.at(quads));
    let mut c = Cone::default();
    for k in 0..9u32 {
        c.verts[k as usize] = v3(ov, verts + 16 * k)?;
        c.uv[k as usize] = [f(ov, uv + 8 * k)?, f(ov, uv + 8 * k + 4)?];
        c.rgba[k as usize] = ov.u32(rgba + 4 * k)?;
    }
    for q in 0..4u32 {
        for i in 0..4u32 {
            let v = ov.i32(quads + 16 * q + 4 * i)?;
            if !(0..9).contains(&v) { return None; }
            c.quads[q as usize][i as usize] = v as usize;
        }
    }
    Some(c)
}

impl VendorLayout {
    /// From the level overlay (labels through [`Overlay::at`]). None when a table is missing or out of range.
    pub fn load(ov: &Overlay) -> Option<VendorLayout> {
        let at = |a: u32| ov.at(a);
        let v = |a: u32| v3(ov, at(a));
        let mut margins = [[0.0; 4]; 6];
        for (s, m) in margins.iter_mut().enumerate() {
            for (k, x) in m.iter_mut().enumerate() { *x = f(ov, at(addr::MARGINS) + 16 * s as u32 + 4 * k as u32)?; }
        }
        let mut talk_keys = [[0; 2]; 3];
        for (i, t) in talk_keys.iter_mut().enumerate() {
            for (k, x) in t.iter_mut().enumerate() { *x = ov.i32(at(addr::TALK_KEYS) + 8 * i as u32 + 4 * k as u32)?; }
        }
        // A record field the code reaches only from another field's address (`0x1c8da0 − 0x10`, `0x1c9490 ± 0x10`,
        // `0x1c9db0 + 0x10`): placed by that field's mapping on the other levels.
        let field = |a: u32, by: u32| if ov.maps(a) { at(a) } else { at(by).wrapping_add(a).wrapping_sub(by) };
        let (panel_rot, holo_rot, holo_class, wholo_rot) =
            (field(addr::PANEL_ROT, addr::PANEL_OFF), field(addr::HOLO_ROT, addr::HOLO_OFF), field(addr::HOLO_CLASS, addr::HOLO_OFF), field(addr::WHOLO_ROT, addr::WHOLO_POST));
        let n = ITEMS as u32;
        let panel = (0..n)
            .map(|i| {
                let (r, o) = (panel_rot + 0x30 * i, at(addr::PANEL_OFF) + 0x30 * i);
                Some(PanelPlace { rot: v3(ov, r)?, offset: v3(ov, o)?, weapon_dz: f(ov, o + 0xc)? })
            })
            .collect::<Option<Vec<_>>>()?;
        let ammo_holo = (0..n)
            .map(|i| {
                let (r, o, c) = (holo_rot + 0x40 * i, at(addr::HOLO_OFF) + 0x40 * i, holo_class + 0x40 * i);
                Some(AmmoHolo { rot: [f(ov, r)?, f(ov, r + 4)?], offset: v3(ov, o)?, class: ov.i32(c)? })
            })
            .collect::<Option<Vec<_>>>()?;
        let weapon_holo = (0..n)
            .map(|i| {
                let (p, r, o) = (at(addr::WHOLO_POST) + 0x30 * i, wholo_rot + 0x30 * i, at(addr::WHOLO_OFF) + 0x30 * i);
                Some(WeaponHolo { post: v3(ov, p)?, rot: [f(ov, r)?, f(ov, r + 4)?], offset: v3(ov, o)? })
            })
            .collect::<Option<Vec<_>>>()?;
        let demo = (0..n).map(|i| ov.i32(at(addr::DEMO) + 4 * i)).collect::<Option<Vec<_>>>()?;
        let lc = at(addr::LIGHT_COLOR);
        Some(VendorLayout {
            camera: v(addr::CAMERA)?,
            exit: v(addr::EXIT)?,
            salesman: v(addr::SALESMAN)?,
            item_base: v(addr::ITEM_BASE)?,
            holo_base: v(addr::HOLO_BASE)?,
            spin: [v(addr::SPIN0)?, v(addr::SPIN1)?],
            light_dir: v(addr::LIGHT_DIR)?,
            light_color: [f(ov, lc)?, f(ov, lc + 4)?, f(ov, lc + 8)?, f(ov, lc + 12)?],
            talk_keys,
            margins,
            panel,
            ammo_holo,
            weapon_holo,
            cone: cone(ov, addr::CONE_VERTS, addr::CONE_UV, addr::CONE_RGBA, addr::CONE_QUADS)?,
            beam: cone(ov, addr::BEAM_VERTS, addr::BEAM_UV, addr::BEAM_RGBA, addr::BEAM_QUADS).unwrap_or_default(),
            demo,
        })
    }
}


#[cfg(test)]
mod tests {
    use super::*;

    /// Every level's vendor reads the same placement tables as level 01's (Kerwan's lie 0x380 lower; the item and
    /// hologram rotations and the ammo hologram's class are reached only from their neighbours' addresses).
    #[test]
    fn every_level_reads_level_01s_tables() {
        let read = |l: u32| std::fs::read(rc_formats::test_data::level_dir(l).join("overlay.bin")).ok();
        let Some(r) = read(1) else { return };
        let reference = VendorLayout::load(&Overlay::parse(&r).unwrap()).expect("level 01 tables");
        assert_eq!(reference.panel[0xf].rot, [0.0, -1.57, -3.0], "the Blaster's item panel rotation");
        let mut compared = 0;
        for level in (0..19).filter(|&l| l != 1) {
            let Some(t) = read(level) else { continue };
            let Some(l) = VendorLayout::load(&Overlay::relocated(&t, &r).unwrap()) else { continue };
            compared += 1;
            assert_eq!((&l.panel, &l.ammo_holo, &l.weapon_holo), (&reference.panel, &reference.ammo_holo, &reference.weapon_holo), "level {level:02}");
        }
        assert!(compared >= 10, "only {compared} levels have the tables");
    }
}
