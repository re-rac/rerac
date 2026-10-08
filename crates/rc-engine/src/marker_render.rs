//! The guns' screen markers (level01 0x1694c0: `FUN_0020fb60` registers, `FUN_0020fc40` draws them in `DrawWorld`
//! after the 3D, with ALPHA 0x44; `rc_game::targeting::Markers`): the Blaster's red first-person crosshair and green
//! target marker (FX 0x26), the Devastator's (0x27) and the R.Y.N.O.'s (0x23). Each is one `fun_001f5ab0` sprite
//! (PRIM 0x154: TRISTRIP, textured, blended) of `40·size` pixels centred on the projection of its point (the screen
//! centre without one), turned by its angle. They go into the 2D pass ([`crate::hud_render`]) ahead of the HUD's own
//! primitives, the order `DrawWorld` gives them (the world's 2D overlays before the HUD pass). Not drawn in
//! Ratchet's hold 0x72 (the draw empties the list).

use crate::hud_render::{Prim, Tex};
use rc_game::targeting::{marker_corners, project_tan, screen_sprite_corners, MARKER_UV};

/// The markers of the tick that just ran as 2D primitives (game pixels, the camera of that tick).
pub fn prims(play: &crate::gameplay::Play) -> Vec<Prim> {
    let g = &play.game;
    if g.hero.state == 0x72 { return Vec::new(); }
    let tick = g.counter.wrapping_sub(1);
    let cam = g.camera.out;
    let (eye, rows) = (cam.pos_f32(), cam.rows_f32());
    let mut out = Vec::new();
    for m in g.hero.weapons.markers.of_tick(tick) {
        let c = match m.at {
            None => [rc_game::targeting::SCREEN[0] * 0.5, rc_game::targeting::SCREEN[1] * 0.5],
            Some(p) => match project_tan(eye, rows, p, play.svc.view_tan_x) {
                Some(c) => c,
                None => continue,
            },
        };
        let q = marker_corners(m, c);
        let pos = q.map(|v| [v[0].round() as i32, v[1].round() as i32]);
        out.push(Prim { tex: Tex::Fx(m.fx), pos, uv: MARKER_UV, rgba: m.rgba, scissor: [0, crate::hud_render::W - 1, 0, crate::hud_render::H - 1], repeat: false, nearest: false, boxed: false, uv16: false });
    }
    // The classes' screen sprites of that tick (the Veldin boss beam's flash 1898, `0x2fb690`).
    for s in play.svc.screen_sprites.iter().filter(|s| s.tick == tick) {
        let Some(c) = project_tan(eye, rows, s.at, play.svc.view_tan_x) else { continue };
        let q = screen_sprite_corners(c, s.half);
        let pos = q.map(|v| [v[0].round() as i32, v[1].round() as i32]);
        out.push(Prim { tex: Tex::Fx(s.fx as usize), pos, uv: s.uv, rgba: s.rgba, scissor: [0, crate::hud_render::W - 1, 0, crate::hud_render::H - 1], repeat: false, nearest: false, boxed: false, uv16: false });
    }
    out
}
