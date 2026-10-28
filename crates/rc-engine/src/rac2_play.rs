//! Going Commando's game tick in the engine (rc_game2: [`rc_game2::hero`], [`rc_game2::camera`]): Ratchet runs,
//! jumps and falls on the level's collision, his moby and pose follow him, the camera follows him
//! (docs/plan/rac2_bringup.md §6.3 F). The pieces that are the same in both games come from the R&C1 side: the pad
//! record (`rc_game::pad`), Ratchet's animation player (`rc_game::hero::anim::RatchetAnim`), the moby renderer's
//! drive of a static instance (crate::moby_render) and the play camera (crate::play_camera).
//!
//! * **Start** (the first tick after the level loaded, and after the front end's New Game): Ratchet is the level's
//!   class-0 instance (its placement: Aranos (247.7, 194, 49.9), facing +y); the hero spawns there, idle; the camera
//!   snaps behind him.
//! * **Tick** (`FixedUpdate`, 60 Hz): the pad (keyboard / gamepad, or `RC_PLAY_SCRIPT`) into the PAD record, the hero
//!   update, the camera; Ratchet's instance gets his position and rows (`MobyOcclusion::drive`) and his animation
//!   state (`MobyAnim`); the camera goes to `PlayView`.
//!
//! Environment: `RC_PLAY=0` (no tick), `RC_PLAY_FLY=1` (start on the fly camera; Tab switches), `RC_PLAY_TRACE=1`
//! (one line per tick: state, position, velocity, air ticks, sequence).

use crate::game_camera::CameraSource;
use crate::input_map::{self, PadFrame, Script};
use crate::moby_anim::MobyAnim;
use crate::moby_render::MobyOcclusion;
use crate::play_camera::{self, PlayView};
use bevy::prelude::*;
use rc_game::follow_camera::CameraView;
use rc_game::hero::anim::RatchetAnim;
use rc_game::pad::PadState;
use rc_game::ps2v::Pf;
use rc_game2::camera::{CamInput, FollowCamera};
use rc_game2::hero::physics::Env;
use rc_game2::hero::{hero_update, Hero};

pub struct Rac2PlayPlugin;

impl Plugin for Rac2PlayPlugin {
    fn build(&self, app: &mut App) {
        if std::env::var("RC_PLAY").is_ok_and(|v| v.trim() == "0") { return; }
        let fly = std::env::var("RC_PLAY_FLY").is_ok_and(|v| v.trim() == "1");
        let script = std::env::var("RC_PLAY_SCRIPT").ok().filter(|s| !s.trim().is_empty()).map(|s| Script::parse(&s).unwrap_or_else(|e| panic!("RC_PLAY_SCRIPT: {e}")));
        println!("{}", input_map::CONTROLS);
        app.insert_resource(if fly { CameraSource::Fly } else { CameraSource::Play })
            .insert_resource(Rac2Script(script))
            .init_resource::<PadFrame>()
            .add_systems(PreStartup, |world: &mut World| {
                let input = input_map::SdlGamepadInput::new().unwrap_or_else(|e| {
                    eprintln!("Failed to initialize SdlGamepadInput: {}", e);
                    std::process::exit(1);
                });
                world.insert_non_send(input);
            })
            .add_systems(PreUpdate, (input_map::sample_system, switch_camera).chain().after(bevy::input::InputSystems))
            .add_systems(FixedUpdate, tick)
            .add_systems(RunFixedMainLoop, play_camera::apply.in_set(RunFixedMainLoopSystems::AfterFixedMainLoop));
    }
}

#[derive(Resource)]
struct Rac2Script(Option<Script>);

/// The running game.
#[derive(Resource)]
struct Play {
    hero: Hero,
    anim: RatchetAnim,
    /// Ratchet's class (index into `LevelMobys::classes` / `anim`) and static instance.
    class: usize,
    instance: usize,
    scale: f32,
    pad: PadState,
    camera: FollowCamera,
    ticks: u64,
    trace: bool,
}

fn switch_camera(keys: Res<ButtonInput<KeyCode>>, mut source: ResMut<CameraSource>) {
    if keys.just_pressed(KeyCode::Tab) {
        *source = if *source == CameraSource::Play { CameraSource::Fly } else { CameraSource::Play };
        println!("rac2: camera: {:?}", *source);
    }
}

#[allow(clippy::too_many_arguments)]
fn tick(
    mut commands: Commands,
    level: Res<crate::Level>,
    play: Option<ResMut<Play>>,
    front: Option<Res<crate::rac2::Rac2>>,
    frame: Res<PadFrame>,
    script: Res<Rac2Script>,
    source: Res<CameraSource>,
    mut occl: Option<ResMut<MobyOcclusion>>,
    mut anim: Option<ResMut<MobyAnim>>,
) {
    // The front end's layer is up: the game waits (as R&C1's tick does under its front end).
    if front.is_some_and(|f| !f.done()) { return; }
    let lv = &level.0;
    let Some(mut p) = play else {
        if let Some(p) = start(lv) {
            commands.insert_resource(PlayView { view: camera_view(&p.camera), tan_half_fov: play_camera::GAME_TAN_HALF_FOV, far: crate::game_camera::FAR });
            commands.insert_resource(p);
        }
        return;
    };
    let p = &mut *p;
    let Some(coll) = lv.collision.as_ref() else { return };
    let input = match &script.0 {
        Some(s) => s.at(p.ticks),
        None if *source == CameraSource::Play => frame.0,
        None => rc_game::pad::PadInput::neutral(),
    };
    p.pad.update(Some(&input.bytes()), false);
    let class = &lv.mobys.anim[p.class];
    {
        let env = Env { coll, pad: &p.pad, cam_yaw: p.camera.yaw(), cam_rows: p.camera.rows, mirror: false };
        let mut ctl = p.anim.ctl(class);
        hero_update(&mut p.hero, &env, &mut ctl);
    }
    p.camera.update(&cam_input(&p.hero, Some(coll), [p.pad.rx.to_f32(), p.pad.ry.to_f32()]));
    if p.trace {
        let v = rc_game::hero::anim::AnimCtl::view(&p.anim.ctl(class));
        let h = &p.hero;
        println!(
            "rac2 tick {}: state {:#x}/{} pos ({:.2}, {:.2}, {:.2}) vel ({:.3}, {:.3}, {:.3}) air {} h {:.2} slope {:.2} seq {}/{} f {:.1} spd {:.2} cap {} at ({:.2}, {:.2}, {:.2}) n ({:.2}, {:.2}, {:.2}) capsule {:.2}/{:.2}/{:.2} cam ({:.2}, {:.2}, {:.2}) yaw {:.2} pitch {:.2}",
            p.ticks, h.state, h.substate, h.pos[0], h.pos[1], h.pos[2], h.vel[0], h.vel[1], h.vel[2], h.air_ticks, h.height, h.slope, v.seq_a, v.seq_b, v.frame, h.anim_speed,
            h.cap_hit, h.contact_point[0], h.contact_point[1], h.contact_point[2], h.contact_normal[0], h.contact_normal[1], h.contact_normal[2], h.cap_bottom, h.cap_top, h.cap_radius,
            p.camera.pos[0], p.camera.pos[1], p.camera.pos[2], p.camera.yaw(), p.camera.pitch()
        );
    }
    if let Some(o) = occl.as_mut() {
        o.drive(p.instance, p.hero.pos, p.hero.rows, p.scale, None, false);
        if let (Some(a), Some(k)) = (anim.as_mut(), o.anim_index(p.instance)) {
            let mut s = p.anim.state;
            s.skip_advance = true;
            if let Some(i) = a.instances.get_mut(k) { i.state = s; }
            if let Some(slot) = a.snapshots.get_mut(k) { *slot = p.anim.snapshot.clone(); }
        }
    }
    commands.insert_resource(PlayView { view: camera_view(&p.camera), tan_half_fov: play_camera::GAME_TAN_HALF_FOV, far: crate::game_camera::FAR });
    p.ticks += 1;
}

/// The hero at Ratchet's instance (class 0).
fn start(lv: &crate::level_load::LoadedLevel) -> Option<Play> {
    let m = &lv.mobys;
    let Some(instance) = m.instances.iter().position(|i| i.o_class == 0) else {
        eprintln!("rac2: no class-0 instance on this level: no Ratchet, no game tick");
        return None;
    };
    let class = m.classes.iter().position(|c| c.o_class == 0)?;
    let inst = &m.instances[instance];
    let scale = m.placed.get(instance).copied().flatten().map_or(1.0, |pl| pl.scale);
    let hero = Hero::spawn(inst.position, inst.rotation[2]);
    let camera = FollowCamera::new(&cam_input(&hero, lv.collision.as_ref(), [0.0; 2]));
    println!("rac2: Ratchet at ({:.1}, {:.1}, {:.1}), yaw {:.2}: game tick on", inst.position[0], inst.position[1], inst.position[2], inst.rotation[2]);
    Some(Play {
        anim: RatchetAnim::new(&m.anim[class]),
        hero, class, instance, scale, pad: PadState::default(), camera, ticks: 0,
        trace: std::env::var("RC_PLAY_TRACE").is_ok_and(|v| v.trim() == "1"),
    })
}

/// The camera's inputs: the options' yaw and pitch normal (`0x1a7bb0` / `0x1a7bac`, the defaults).
fn cam_input<'a>(hero: &'a Hero, coll: Option<&'a rc_formats::collision::Collision>, rstick: [f32; 2]) -> CamInput<'a> {
    CamInput { hero, coll, rstick, yaw_normal: true, pitch_normal: true }
}

fn camera_view(c: &FollowCamera) -> CameraView {
    let v = |a: [f32; 3]| [Pf::f(a[0]), Pf::f(a[1]), Pf::f(a[2]), Pf::ZERO];
    CameraView { pos: v(c.pos), euler: [Pf::ZERO, Pf::f(c.pitch()), Pf::f(c.yaw()), Pf::ZERO], rows: c.rows.map(v), class: 0, flyby_phase: 0 }
}
