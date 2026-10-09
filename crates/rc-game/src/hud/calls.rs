//! The HUD's call channel: what the game code asks of the slot machine (`queue_animation_update` 0x24ad98 and the
//! handle calls `FUN_0024b4b0` keep-up, `FUN_0024b090` set-flags, `fun_001ff480` release), made where the game makes
//! them and applied by the HUD before its next update loop ([`super::HudState::apply_calls`]).
//!
//! **Why a channel.** The game calls the slot functions directly from the moby loop, the hero and the menus; the
//! port runs the HUD in the engine's render, after the game ticks of the frame. A consumer therefore records its call
//! here (the game side: [`Calls`] in `Services::hud`, which every class update reaches through `World::svc`; the
//! engine side: `rc_engine::hud_render::HudFeed::calls`) and the HUD replays the new ones, in order, at the start of
//! its next tick. A call is identified by its request (slot, flags, icon, element, max: the arguments the game
//! compares), so a handle the game would keep is the request itself here ([`Call::KeepUp`] etc. act on the slot whose
//! last request equals it, which is the slot whose handle the game's caller holds) [L].
//!
//! **A new consumer** (a boss, a vehicle, a race) only queues its slot: [`Calls::queue`] with a [`Request`] (the
//! element, its slot and flags, its icon and max), and when the element shows a value it does not own, publishes the
//! value every tick with [`Calls::data`] (the game's data pointer: [`super::Element::Boss`] reads key `k`). Releasing
//! is [`Calls::set_flags`] (`FUN_0024b090(h, 0)`: the element times out) or [`Calls::release`] (`fun_001ff480(h)`: the
//! empty element replaces it at once). The boss meter in one line: [`Calls::boss_meter`].

use super::{Element, Request};
use std::collections::VecDeque;

/// One call of the game into the slot machine.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Call {
    /// `queue_animation_update(slot | flags, icon, init, update, draw, data, max)` 0x24ad98.
    Queue(Request),
    /// `FUN_0024b4b0(handle, n)`: the slot holding the request (applied) keeps its timer at ≥ `n`.
    KeepUp(Request, i32),
    /// `FUN_0024b090(handle, flags)`: the request's flags replaced (0: no longer persistent, it times out).
    SetFlags(Request, u32),
    /// `fun_001ff480(handle)`: the slot holding the request gets the empty element (max 0), timer 0.
    Release(Request),
    /// `HudShowHealth` 0x24a498 + `FUN_0024b4b0(h, n)`, then the bolt counter `queue_animation_update(2, 0x754e, …)` +
    /// `FUN_0024b4b0(h, n)`: the "show health and bolts" pair of the quick select's opening (0x242930), `PageMenuClose`
    /// 0x28c6c8 and `InLevelFrameUpdate`'s L3 (0x2aba68), all with `n = ScaleTicks(180)`.
    ShowHealthBolts(i32),
    /// The value behind a data key (the game's `*data` of an element whose value a consumer owns).
    Data(u32, i32),
}

/// The game side's call log (`Services::hud`): appended by the classes, read by the HUD from its cursor.
#[derive(Clone, Debug, Default)]
pub struct Calls {
    log: VecDeque<(u64, Call)>,
    next: u64,
}

/// Calls kept for a reader that falls behind (a frame holds at most a second of ticks).
const KEEP: usize = 1024;

impl Calls {
    /// Records `c`.
    pub fn push(&mut self, c: Call) {
        self.log.push_back((self.next, c));
        self.next += 1;
        while self.log.len() > KEEP { self.log.pop_front(); }
    }

    /// `queue_animation_update` with `r`.
    pub fn queue(&mut self, r: Request) { self.push(Call::Queue(r)); }

    /// `FUN_0024b4b0(r's handle, n)`.
    pub fn keep_up(&mut self, r: Request, n: i32) { self.push(Call::KeepUp(r, n)); }

    /// `FUN_0024b090(r's handle, flags)`.
    pub fn set_flags(&mut self, r: Request, flags: u32) { self.push(Call::SetFlags(r, flags)); }

    /// `fun_001ff480(r's handle)`.
    pub fn release(&mut self, r: Request) { self.push(Call::Release(r)); }

    /// The value of data key `key` this tick.
    pub fn data(&mut self, key: u32, value: i32) { self.push(Call::Data(key, value)); }

    /// The boss meter (slot 6 | 0x10, the 0x24b418 / 0x24b538 pair, the boss draw: level06 0x2f9a28, level18
    /// 0x2f7288): publishes `value` under `key` and queues the meter (an unchanged request is a no-op, as in the game);
    /// returns the request, for [`Calls::set_flags`] / [`Calls::release`] when the boss is done.
    pub fn boss_meter(&mut self, key: u32, value: i32, max: i32) -> Request {
        let r = Request::boss(key, max);
        self.data(key, value);
        self.queue(r);
        r
    }

    /// The calls after sequence number `cursor` (exclusive; `None`: all kept), and the cursor to keep. A cursor past
    /// the log's end (a new log: the level was reloaded) reads it from its start.
    pub fn since(&self, cursor: Option<u64>) -> (Vec<Call>, Option<u64>) {
        let cursor = cursor.filter(|&c| c < self.next);
        let out = self.log.iter().filter(|(s, _)| cursor.is_none_or(|c| *s > c)).map(|(_, c)| *c).collect();
        (out, self.next.checked_sub(1))
    }
}

/// A data key for a moby's pvar word at `offset`, as the game's data pointer: the pvars belong to the placed instance
/// (`spawn_id`), so a death reload that compacts the moby table leaves the key (and the HUD's request) unchanged, as
/// the game's pointer is; a moby without a placement keys by its slot.
pub fn pvar_key(spawn_id: i16, id: usize, offset: usize) -> u32 {
    let k = if spawn_id >= 0 { spawn_id as u32 } else { 0x8000 | id as u32 };
    (k << 16) | offset as u32
}

impl Request {
    /// The boss meter's request (module doc).
    pub fn boss(key: u32, max: i32) -> Request { Request { slot: 6, flags: 0x10, icon: 0xffff, element: Element::Boss { key }, max } }
}
