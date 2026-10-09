use super::screens::{Quad, Statics, View};
use super::*;

fn shop() -> ShopTable {
    let mut records = vec![[0u8; 0x18]; crate::moby_update::interact::SHOP_RECORDS];
    let mut set = |i: usize, price: i32, unit: u16, max: u16| {
        records[i][0..4].copy_from_slice(&price.to_le_bytes());
        records[i][4..8].copy_from_slice(&(price / 2).to_le_bytes());
        records[i][8..10].copy_from_slice(&unit.to_le_bytes());
        records[i][0xa..0xc].copy_from_slice(&(unit * 5).to_le_bytes());
        records[i][0xe..0x10].copy_from_slice(&max.to_le_bytes());
    };
    set(10, 0, 5, 40);
    set(15, 2500, 1, 200);
    set(16, 2500, 1, 240);
    set(12, 1000, 0, 0);
    ShopTable { records }
}

#[test]
fn list_is_stock_then_owned_ammo() {
    let mut gs = GameState::zeroed(rc_formats::save_game::ChunkTables { global: Vec::new(), level: Vec::new() });
    gs.global.vendor = [0x4a, 16, 0xff, 0, 0, 0, 0, 0, 0, 0, 0, 0];
    gs.global.owned[10] = 1;
    gs.global.owned[15] = 1;
    gs.global.owned[12] = 1;
    let l = build_list(&gs, &shop(), false);
    assert_eq!(l, vec![
        Entry { item: 10, ammo: true, locked: false },
        Entry { item: 16, ammo: false, locked: false },
        Entry { item: 15, ammo: true, locked: false },
    ]);
    let r = build_list(&gs, &shop(), true);
    assert_eq!(r.iter().map(|e| (e.item, e.ammo)).collect::<Vec<_>>(), vec![(10, true), (15, true)], "remote: ammo only");
}

#[test]
fn price_text_formats() {
    assert_eq!(price_text(500), b"500");
    assert_eq!(price_text(2500), b"2,500");
    assert_eq!(price_text(150000), b"150,000");
    assert_eq!(price_text(60050), b"60,050");
}

#[test]
fn vendor_camera_faces_the_vendor() {
    let yaw = 0.3f32;
    let (s, c) = yaw.sin_cos();
    let rows = [[c, s, 0.0, 0.0], [-s, c, 0.0, 0.0], [0.0, 0.0, 1.0, 0.0], [0.0; 4]];
    let (eye, r) = vendor_camera([10.0, 20.0, 5.0], rows, yaw);
    assert!((eye[0] - (10.0 + 3.8 * c)).abs() < 1e-5 && (eye[1] - (20.0 + 3.8 * s)).abs() < 1e-5 && (eye[2] - 6.5).abs() < 1e-5);
    // Forward points from the eye back to the vendor.
    let to = [10.0 - eye[0], 20.0 - eye[1]];
    let l = (to[0] * to[0] + to[1] * to[1]).sqrt();
    assert!((r[0][0] - to[0] / l).abs() < 1e-5 && (r[0][1] - to[1] / l).abs() < 1e-5);
}

#[test]
fn euler_rows_match_the_moby_matrix() {
    for e in [[0.3f32, -0.7, 1.9], [0.0, 0.0, 2.5], [-1.2, 0.4, 0.0]] {
        let a = euler_rows(e);
        let b = rc_formats::moby_light::rotation_rows(e);
        for i in 0..3 {
            for k in 0..3 { assert!((a[i][k] - f32::from_bits(b[i][k])).abs() < 1e-4, "{e:?} row {i} {k}"); }
        }
    }
}

#[test]
fn screen_quad_insets_and_shrinks_about_its_centre() {
    // A 2×1 monitor in the x/z plane.
    let q = Quad::new([[0.0, 0.0, 0.0], [2.0, 0.0, 0.0], [0.0, 0.0, 1.0]], [0.1, 0.05, 0.1, 0.05]);
    assert!((q.c[0] - 0.1).abs() < 1e-6 && (q.c[2] - 0.05).abs() < 1e-6);
    assert!((q.a[0] - 1.8).abs() < 1e-6 && (q.b[2] - 0.9).abs() < 1e-6);
    let h = q.shrink(0.5);
    let (c0, c1) = (q.far(), h.far());
    let mid = |q: &Quad, f: [f32; 3]| [(q.c[0] + f[0]) / 2.0, (q.c[2] + f[2]) / 2.0];
    assert_eq!(mid(&q, c0), mid(&h, c1), "same centre");
    assert!((h.a[0] - 0.9).abs() < 1e-6 && (h.b[2] - 0.45).abs() < 1e-6);
    // Zero: a point at the centre.
    assert!(q.shrink(0.0).a.iter().all(|&v| v == 0.0));
}

#[test]
fn projection_centres_the_view_axis() {
    let v = View::game([0.0, 0.0, 0.0], [[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]]);
    assert_eq!(v.project([5.0, 0.0, 0.0]), Some([256.0, 208.0]));
    // Left (+y) is to the left on screen, up (+z) is up.
    let p = v.project([5.0, 1.0, 1.0]).unwrap();
    assert!(p[0] < 256.0 && p[1] < 208.0);
    assert!((256.0 - p[0] - 256.0 / (5.0 * 0.63)).abs() < 1e-3);
    assert_eq!(v.project([-1.0, 0.0, 0.0]), None, "behind");
    let t = v.target();
    assert_eq!(t.project([5.0, 0.0, 0.0]), Some([256.0, 64.0]));
}

#[test]
fn ticker_static_always_runs_and_other_screens_burst() {
    let mut s = Statics::default();
    let mut rng = Rng::new();
    let d = s.step(222.0, 40.0, 0, &mut rng);
    assert_eq!(d.len(), 1);
    assert_eq!(d[0].fx, screens::NOISE_FX);
    // Between bursts the ticker's counter is floored at 0x18: 2·(0x80 − |0x18 − 0x80|) = 0x30.
    assert_eq!(d[0].rgba >> 24, 0x30, "the ticker's resting static");
    // A burst on screen 3: +2 per frame; `subtract_integer_with_clamp` 0x221110 is abs(), so the noise fades in
    // (c 4 → 0x40), holds at 0x80 (0x40..0xc0) and fades out (→ 0 at 0x100), then the counter resets.
    s.burst[3] = 2;
    let mut alphas = Vec::new();
    for _ in 0..200 {
        let d = s.step(100.0, 50.0, 3, &mut rng);
        if let Some(n) = d.iter().find(|x| x.fx == screens::NOISE_FX) { alphas.push(n.rgba >> 24); }
        if s.burst[3] == 0 { break; }
    }
    assert_eq!(alphas.len(), 127, "c = 0x04..0x100 step 2, then the reset");
    assert_eq!(&alphas[..4], &[8, 12, 16, 20]);
    assert_eq!(alphas[30], 0x80, "full from c = 0x40");
    assert_eq!(alphas[93], 0x80, "still full at c = 0xc0");
    assert_eq!(&alphas[123..], &[12, 8, 4, 0]);
}

#[test]
fn scan_bar_fades_in_and_out() {
    // FX 0x1c on screens 1..5: alpha 0x100 − |c − 0xfe| (c before the +2) capped at 0x50, over c = 2..0x1fe.
    let mut s = Statics::default();
    let mut rng = Rng::new();
    s.bar[2] = 2;
    let mut alphas = Vec::new();
    for _ in 0..300 {
        let d = s.step(120.0, 90.0, 2, &mut rng);
        if let Some(b) = d.iter().find(|x| x.fx == screens::BAR_FX) { alphas.push(b.rgba >> 24); }
        if s.bar[2] == 0 { break; }
    }
    assert_eq!(alphas.len(), 255, "c = 2..0x1fe step 2");
    assert_eq!(&alphas[..3], &[4, 6, 8]);
    assert_eq!(alphas[38], 0x50, "full from c = 0x4e");
    assert_eq!(alphas[214], 0x50, "still full at c = 0x1ae");
    assert_eq!(alphas[215], 0x4e);
    assert_eq!(&alphas[252..], &[4, 2, 0]);
}

#[test]
fn power_factor_follows_the_counters() {
    let mut out = VendorOut::default();
    let gs = GameState::zeroed(rc_formats::save_game::ChunkTables { global: Vec::new(), level: Vec::new() });
    let mut v = Vendor::open(VendorTables { shop: shop(), ..Default::default() }, &gs, false, &mut out);
    assert_eq!(out.sounds, vec![sound::OPEN]);
    assert_eq!(out.anim, vec![VendorAnim::HardCut { seq: 2, frame: 0 }, VendorAnim::Speed(0.5)]);
    assert!(!v.world_runs(), "FadeToBlack(4) blocks");
    v.pre_fade = 0;
    assert!(v.world_runs());
    v.sub = 1;
    v.power_on = true;
    v.power_t = 8;
    assert_eq!(v.power(), 0.0);
    v.power_t = 2;
    assert_eq!(v.power(), 0.75);
    v.power_on = false;
    assert_eq!(v.power(), 1.0);
    v.exit_req = true;
    v.exit_t = 4;
    assert_eq!(v.power(), 0.5);
    assert!(!v.world_runs() && v.screens_shown());
}

#[test]
fn ticker_glyphs_are_half_bright_textured_quads() {
    // `fun_00238310(2.0, text, −scroll, 8)`: `DrawTexturedQuad(x, 8, 18, 18, u, v, 9, 9, 0x80404040, LED font)`.
    let mut out = VendorOut::default();
    let gs = GameState::zeroed(rc_formats::save_game::ChunkTables { global: Vec::new(), level: Vec::new() });
    let mut v = Vendor::open(VendorTables { shop: shop(), ..Default::default() }, &gs, false, &mut out);
    v.tables.led_cell = vec![-1; 64];
    v.tables.led_cell[0x21] = 0x0090_0120; // 'A': u = 0x120 >> 4 = 18, v = 0x90 >> 4 = 9
    v.tables.led_adv = vec![10; 64];
    v.ticker = b"A".to_vec();
    v.ticker_scroll = -20;
    let glyphs = [[rc_formats::font::Glyph::default(); rc_formats::font::GLYPHS]; 3];
    let hud = crate::hud::HudAssets { icons: vec![], frame_sizes: vec![(64, 64)], glyphs, messages: vec![] };
    let a = MenuAssets::new(hud, Overlay::default());
    let d = v.screen_content(screens::TICKER, (230, 35), &a, &gs, 0);
    assert_eq!(d[0], MenuDraw::FrameQuad { frame: 0, x: 16, y: 8, w: 18, h: 18, u: 18, v: 9, tw: 9, th: 9, rgba: LED_RGBA });
    assert_eq!(LED_RGBA, 0x8040_4040);
}

/// `OpenVendorMenu`'s arms: with fewer than 8 entries all four records pull the strip's ends in by 0x681 per missing
/// entry (records 0 / 1 copy 2 / 3), and the vendor's own joint lists are loaded for them.
#[test]
fn a_short_list_pulls_in_all_four_arms() {
    let d = (5 * 0x681) as f32;
    assert_eq!(arm_manipulators(2), [(0x14, [0.0, d, 0.0]), (0x15, [0.0, -d, 0.0]), (0x16, [0.0, d, 0.0]), (0x17, [0.0, -d, 0.0])]);
    assert!(arm_manipulators(8).iter().all(|(_, t)| *t == [0.0; 3]));
    assert!(crate::moby_update::classes::ClassUpdate::Vendor.needs_joint_lists());
}

/// The carousel (≥ 8 entries): a step moves the offset ±0x38, the strip's draw eases it back 4 a frame and draws the
/// selection frame only once it is back at 0 (the old port never eased it: the strip stayed shifted, the frame
/// hidden, and the ±0x39 limit refused or doubled the next steps).
#[test]
fn the_carousel_eases_back_after_a_step() {
    let mut out = VendorOut::default();
    let gs = GameState::zeroed(rc_formats::save_game::ChunkTables { global: Vec::new(), level: Vec::new() });
    let mut v = Vendor::open(VendorTables { shop: shop(), ..Default::default() }, &gs, false, &mut out);
    v.items = (0..10).map(|i| Entry { item: 10 + i, ammo: true, locked: false }).collect();
    v.scroll = -0x38;
    let mut frames = 0;
    while v.scroll != 0 {
        v.ease_strip();
        assert!(!v.strip_settled);
        frames += 1;
    }
    assert_eq!(frames, 14);
    v.ease_strip();
    assert!(v.strip_settled && v.scroll == 0);
    // Fewer than 8: no carousel, nothing eased.
    v.items.truncate(5);
    v.scroll = 8;
    v.ease_strip();
    assert_eq!(v.scroll, 8);
}
