//! **The Gadgets / Weapons pages' 3D Ratchet animations** (G-UI-027): the stream player of `LoadHandGadget` 0x297d70
//! (addresses level01.elf; the boot's copies are `fun_002265d8` / `fun_00226670` / `fun_00226718` / `fun_00225e70`).
//!
//! * **The table** 0x1b9868 and the install records 0x1b9f58 ([`Tables`], read from the level's overlay). When the widget creates a hand, head or feet moby (the last of
//!   them this frame, item 1..0x23, not the item it last queued, 0x1ba18c), the queue is cleared (`fun_00226718`) and
//!   the item's jobs queued (`fun_002265d8`, at most 8): its animation (mode 0, start at once) and, when it has a wrap
//!   count, the animation that follows (mode 2, sequence 1 for the hand, nothing else).
//! * **The entries** are global `ratchet_seqs` lumps: entry `e` streams lump `e` into Ratchet's class as sequence
//!   `base + e` (`base` = the class's sequence count, 0x1ba23c from `LoadItemIdsFromSave`). The port has every lump at
//!   hand: a queued job is ready at once (the game's two stream buffers, statuses 1 / 2 and `start_audio_stream_read`
//!   are its disc reads).
//! * **The player** `FUN_00299a78` (each widget update, after `LoadHandGadget`, [`Player::tick`]): with jobs queued,
//!   Ratchet's last advance wrapping (+0x70 bit 1) counts down the current animation's wraps; the head job starts by its
//!   mode (0 at once, 1 on a wrap, 2 when the wraps ran out, 3 dropped) if its item is 0 or still the pending hand, head
//!   or feet item (else dropped). Starting it ([`Start`]): popped; the previous job's installed sequences removed
//!   (`fun_002267b8`) and its own installed (`fun_00226848`: the `hud_seqs` lumps `first..first + count` into the
//!   classes and sequence slots of the install records); Ratchet blended to it (`MobyAnimBlendEx(e, 0, 10, 5)`); a head item
//!   (slot type 2) blends the head moby to the job's hand sequence and cuts the hand moby to 1, else the hand moby
//!   blends to it; the props deleted and up to three made at Ratchet's place, on their sequence (cut, then blended over
//!   10 ticks; class 0x4a three times as large), with an empty update.
//!
//! **Not modelled**: the item-0x12 hand node 0x1ba350 (its values are zeros); the stream break of `fun_00226718`; a prop
//! of class 0 (rows 8 and 30: a second Ratchet on sequence 0; neither item is on the Gadgets page's grids) is not drawn.

/// The table's and the install records' level01 addresses ([`Tables::read`] relocates them per level).
pub const TABLE_ADDR: u32 = 0x1b9868;
pub const INSTALLS_ADDR: u32 = 0x1b9f58;
const ROWS: u32 = 36;
const RECORDS: u32 = 17;

/// The table (0x30 bytes per item: `[install first, install count, entry, wraps, hand sequence, then entry, prop class,
/// prop sequence ×3]`) and the install records (`hud_seqs` lump `k` becomes sequence `.1` of class `.0`) a level's
/// overlay holds (36 rows, 17 records).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Tables {
    pub rows: Vec<[i32; 12]>,
    pub installs: Vec<(i16, u8)>,
}

impl Tables {
    /// From a level's overlay (relocated against level01's); `None` when it does not hold them.
    pub fn read(ov: &crate::menus::Overlay) -> Option<Tables> {
        if !ov.maps(TABLE_ADDR) || !ov.maps(INSTALLS_ADDR) { return None; }
        let (t, i) = (ov.at(TABLE_ADDR), ov.at(INSTALLS_ADDR));
        let rows = (0..ROWS).map(|r| (0..12u32).map(|k| ov.i32(t + 0x30 * r + 4 * k)).collect::<Option<Vec<_>>>().map(|v| std::array::from_fn(|k| v[k]))).collect::<Option<Vec<_>>>()?;
        let installs = (0..RECORDS).map(|k| Some((ov.i32(i + 8 * k)? as i16, ov.i32(i + 8 * k + 4)? as u8))).collect::<Option<Vec<_>>>()?;
        Some(Tables { rows, installs })
    }
}

/// The queue's size (`fun_002265d8`: 8 records of 0x38 bytes from 0x1ba440).
pub const MAX_JOBS: usize = 8;
/// The items the table covers (`LoadHandGadget`: 0 < item < 0x24).
pub const ITEMS: i32 = 0x24;
/// The class whose props are made three times as large (`*(moby + 0x2c) *= 3`).
pub const BIG_PROP: i16 = 0x4a;

/// A queued job (the 0x38-byte record without its status word).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Job {
    /// +0x04: the `ratchet_seqs` entry plus the base (Ratchet's sequence).
    pub seq: i32,
    /// +0x08: 0 at once, 1 on a wrap, 2 when the wraps ran out, 3 dropped.
    pub mode: i32,
    /// +0x0c: the wraps before the next job (mode 2) may start.
    pub wraps: i32,
    /// +0x10: the item (0: any).
    pub item: i32,
    /// +0x14: the hand (or head) moby's sequence.
    pub item_seq: i32,
    /// +0x18..+0x2c: the props (class −1: none).
    pub props: [(i32, i32); 3],
    /// +0x30 / +0x34: the install records `first..first + count`.
    pub install: (i32, i32),
}

/// What a started job does to the widget's mobys ([`Player::tick`]); the engine applies it in this order.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Start {
    /// The (class, sequence) slots emptied (`fun_002267b8`) and filled with a `hud_seqs` lump (`fun_00226848`).
    pub uninstall: Vec<(i16, u8)>,
    pub install: Vec<(usize, i16, u8)>,
    /// Ratchet's sequence (`MobyAnimBlendEx(seq, 0, 10, 5)`).
    pub ratchet_seq: u8,
    /// A head item (slot type 2): the head moby blends to `item_seq` and the hand moby is cut to 1; else the hand
    /// moby blends to `item_seq`.
    pub head: bool,
    pub item_seq: u8,
    /// The props made at Ratchet's place (the old ones deleted).
    pub props: [Option<(i16, u8)>; 3],
}

/// The stream player's state.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Player {
    pub tables: Tables,
    pub queue: Vec<Job>,
    /// 0x1ba60c: the current animation's wraps left.
    pub left: i32,
    /// 0x1ba18c: the item last queued (−1 at the widget's enter).
    pub last: i32,
    /// 0x1ba218 / 0x1ba21c: the install records in place.
    pub installed: (i32, i32),
}

impl Player {
    /// The widget's enter `FUN_00297ad0`: the queue cleared (`fun_00226718`), no item queued yet.
    pub fn enter(tables: Tables) -> Player { Player { tables, last: -1, ..Default::default() } }

    /// `LoadHandGadget`'s tail with `item` the last hand / head / feet item whose moby it made this frame; `base` is
    /// Ratchet's class sequence count.
    pub fn queue_item(&mut self, item: i32, base: i32) {
        if item == self.last || !(0 < item && item < ITEMS) { return; }
        self.last = item;
        // fun_00226718.
        self.queue.clear();
        self.left = 0;
        let Some(&r) = self.tables.rows.get(item as usize) else { return };
        let props = [(r[6], r[7]), (r[8], r[9]), (r[10], r[11])];
        self.push(Job { seq: r[2] + base, mode: 0, wraps: r[3], item, item_seq: r[4], props, install: (r[0], r[1]) });
        if r[3] != 0 {
            self.push(Job { seq: r[5] + base, mode: 2, wraps: 0, item, item_seq: 1, props: [(-1, 0); 3], install: (0, 0) });
        }
    }

    /// `fun_002265d8`: a full queue drops the job.
    fn push(&mut self, j: Job) {
        if self.queue.len() < MAX_JOBS { self.queue.push(j); }
    }

    /// `FUN_00299a78`, once per widget update: `wrapped` = Ratchet's last advance wrapped (+0x70 bit 1), `pending` = the
    /// page's pending hand, head and feet items, `slot_of(item)` = its definition's slot type (+0x08).
    pub fn tick(&mut self, wrapped: bool, pending: [i32; 3], slot_of: impl Fn(i32) -> i32) -> Option<Start> {
        if self.queue.is_empty() { return None; }
        let mut ran_out = false;
        if wrapped {
            if self.left != 0 { self.left -= 1; }
            ran_out = self.left == 0;
        }
        let j = self.queue[0];
        if j.item != 0 && !pending.contains(&j.item) {
            self.queue.remove(0);
            return None;
        }
        let go = match j.mode {
            0 => true,
            1 => wrapped,
            2 => ran_out,
            3 => {
                self.queue.remove(0);
                false
            }
            _ => false,
        };
        if !go { return None; }
        self.left = j.wraps;
        self.queue.remove(0);
        let installs = &self.tables.installs;
        let records = |(first, n): (i32, i32)| (first.max(0)..first.max(0) + n.max(0)).filter_map(|k| installs.get(k as usize).map(|&(c, s)| (k as usize, c, s))).collect::<Vec<_>>();
        let uninstall = records(self.installed).into_iter().map(|(_, c, s)| (c, s)).collect();
        let install = records(j.install);
        self.installed = j.install;
        let props = j.props.map(|(c, s)| (c != -1).then_some((c as i16, s as u8)));
        Some(Start { uninstall, install, ratchet_seq: j.seq as u8, head: slot_of(j.item) == 2, item_seq: j.item_seq as u8, props })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Level01's rows (0x1b9868).
    const TABLE: [[i32; 12]; 36] = [
        [0, 0, 7, 1, 1, 7, -1, 0, -1, 0, -1, 0],
        [0, 0, 7, 1, 1, 7, -1, 0, -1, 0, -1, 0],
        [0, 0, 7, 1, 1, 7, -1, 0, -1, 0, -1, 0],
        [0, 0, 7, 1, 1, 7, -1, 0, -1, 0, -1, 0],
        [0, 0, 7, 1, 1, 7, -1, 0, -1, 0, -1, 0],
        [0, 0, 25, 1, 1, 7, -1, 0, -1, 0, -1, 0],
        [0, 0, 26, 1, 1, 7, -1, 0, -1, 0, -1, 0],
        [0, 0, 21, 1, 1, 7, -1, 0, -1, 0, -1, 0],
        [0, 0, 7, 1, 1, 7, -1, 0, 0, 0, 0, 0],
        [0, 0, 22, 1, 1, 6, -1, 0, -1, 0, -1, 0],
        [1, 1, 4, 1, 1, 7, 657, 1, -1, 0, -1, 0],
        [0, 0, 0, 1, 1, 6, -1, 0, -1, 0, -1, 0],
        [0, 0, 8, 1, 1, 7, -1, 0, -1, 0, -1, 0],
        [14, 1, 15, 1, 4, 7, -1, 0, -1, 0, -1, 0],
        [0, 0, 14, 1, 1, 7, -1, 0, -1, 0, -1, 0],
        [0, 1, 3, 1, 5, 7, -1, 0, -1, 0, -1, 0],
        [0, 0, 10, 1, 1, 6, -1, 0, -1, 0, -1, 0],
        [2, 1, 5, 0, 1, 7, 74, 3, -1, 0, -1, 0],
        [0, 0, 2, 0, 1, 7, -1, 0, -1, 0, -1, 0],
        [0, 0, 1, 1, 1, 7, -1, 0, -1, 0, -1, 0],
        [7, 3, 13, 1, 1, 7, 186, 8, 186, 9, 186, 10],
        [16, 1, 23, 1, 1, 7, 270, 4, -1, 0, -1, 0],
        [0, 0, 18, 1, 1, 6, -1, 0, -1, 0, -1, 0],
        [0, 0, 9, 1, 1, 6, -1, 0, -1, 0, -1, 0],
        [0, 0, 17, 1, 1, 7, -1, 0, -1, 0, -1, 0],
        [6, 1, 16, 1, 1, 7, 203, 3, -1, 0, -1, 0],
        [12, 1, 24, 1, 3, 7, -1, 0, -1, 0, -1, 0],
        [0, 0, 27, 1, 1, 6, -1, 0, -1, 0, -1, 0],
        [0, 0, 11, 1, 1, 7, -1, 0, -1, 0, -1, 0],
        [0, 0, 12, 1, 1, 7, -1, 0, -1, 0, -1, 0],
        [0, 0, 6, 1, 1, 6, -1, 0, 0, 0, 0, 0],
        [10, 2, 19, 1, 3, 7, 634, 8, -1, 0, -1, 0],
        [15, 1, 20, 1, 1, 7, -1, 0, -1, 0, -1, 0],
        [0, 0, 6, 1, 1, 6, -1, 0, -1, 0, -1, 0],
        [0, 0, 6, 1, 1, 6, -1, 0, -1, 0, -1, 0],
        [0, 0, 6, 1, 1, 6, -1, 0, -1, 0, -1, 0],
    ];

    /// Level01's install records (0x1b9f58; boot 0x1d59d8).
    const INSTALLS: [(i16, u8); 17] = [
        (168, 5), (657, 1), (74, 3), (497, 1), (497, 2), (497, 3), (203, 3), (186, 8), (186, 9), (186, 10), (483, 3),
        (634, 8), (188, 3), (433, 3), (163, 4), (619, 3), (270, 4),
    ];

    fn level01() -> Tables { Tables { rows: TABLE.to_vec(), installs: INSTALLS.to_vec() } }

    /// Every level's overlay holds level01's table and records, each at its own address (the relocation's data map).
    #[test]
    fn the_tables_read_on_every_level() {
        let overlay = |l: u32| std::fs::read(rc_formats::test_data::level_dir(l).join("overlay.bin")).ok();
        let Some(reference) = overlay(1) else { eprintln!("skipped: no extracted/"); return };
        assert_eq!(Tables::read(&crate::menus::Overlay::parse(&reference).unwrap()), Some(level01()));
        for l in 2..19 {
            let Some(b) = overlay(l) else { continue };
            assert_eq!(Tables::read(&crate::menus::Overlay::relocated(&b, &reference).unwrap()), Some(level01()), "level {l}");
        }
    }

    #[test]
    fn an_item_plays_its_animation_then_the_follow_up_after_its_wraps() {
        let mut p = Player::enter(level01());
        // Item 13 (row 13): entry 15 with the hand on sequence 4 and hud_seqs lump 14 (class 163's sequence 4), one
        // wrap, then entry 7 with the hand back on 1.
        p.queue_item(13, 100);
        assert_eq!(p.queue.len(), 2);
        let s = p.tick(false, [13, 0, 0], |_| 0).expect("mode 0 starts at once");
        assert_eq!((s.ratchet_seq, s.item_seq, s.head), (115, 4, false));
        assert_eq!(s.install, vec![(14, 163, 4)]);
        assert!(s.uninstall.is_empty());
        assert_eq!(s.props, [None; 3]);
        assert_eq!(p.tick(false, [13, 0, 0], |_| 0), None, "the follow-up waits for the wrap");
        let s = p.tick(true, [13, 0, 0], |_| 0).expect("one wrap runs the count out");
        assert_eq!((s.ratchet_seq, s.item_seq), (107, 1));
        assert_eq!(s.uninstall, vec![(163, 4)]);
        assert!(s.install.is_empty());
        assert!(p.queue.is_empty());
        // The same item again queues nothing; another item's job is dropped once it is no longer pending.
        p.queue_item(13, 100);
        assert!(p.queue.is_empty());
        p.queue_item(20, 100);
        assert_eq!(p.tick(false, [13, 0, 0], |_| 0), None);
        assert!(p.queue.len() == 1, "the dropped head job leaves the follow-up");
    }

    #[test]
    fn props_and_head_items() {
        let mut p = Player::enter(level01());
        p.queue_item(20, 0);
        let s = p.tick(false, [0, 0, 20], |_| 1).unwrap();
        assert_eq!(s.props, [Some((186, 8)), Some((186, 9)), Some((186, 10))]);
        assert_eq!(s.install.iter().map(|r| r.0).collect::<Vec<_>>(), vec![7, 8, 9]);
        let mut p = Player::enter(level01());
        p.queue_item(26, 0);
        assert!(p.tick(false, [0, 26, 0], |_| 2).unwrap().head);
    }
}
