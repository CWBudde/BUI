//! What the owner of the items tells the view when they change. Each call
//! keeps the anchor on the same item (so nothing the user looks at moves)
//! and costs O(log blocks) plus the work of one block.

use super::{ListView, Pos};
use crate::fx::Fx;
use crate::index::IndexError;

impl ListView {
    /// `count` items holding `bytes` of text were added at the end (new
    /// messages). While pinned to the bottom the view follows them.
    pub fn appended(&mut self, count: usize, bytes: u64) -> Result<(), IndexError> {
        self.index.insert_items(self.index.len(), count, bytes)
    }

    /// `count` older items were added at the start (a history page). The
    /// anchor moves with its item: no jump.
    pub fn prepended(&mut self, count: usize, bytes: u64) -> Result<(), IndexError> {
        self.inserted(0, count, bytes)
    }

    /// `count` items were inserted before item `at`.
    pub fn inserted(&mut self, at: usize, count: usize, bytes: u64) -> Result<(), IndexError> {
        self.index.insert_items(at, count, bytes)?;
        if let Pos::Anchor { item, .. } = &mut self.pos
            && at <= *item
        {
            *item += count;
        }
        if let Some((g, _)) = &mut self.goal
            && at <= *g
        {
            *g += count;
        }
        self.page_version = u64::MAX;
        Ok(())
    }

    /// `count` items starting at `at` were removed; they held `bytes` of text.
    pub fn removed(&mut self, at: usize, count: usize, bytes: u64) {
        let n = self.index.remove_items(at, count, bytes);
        if let Pos::Anchor { item, off } = &mut self.pos {
            if *item >= at + n {
                *item -= n;
            } else if *item >= at {
                // the anchor item itself went: the next item takes its place
                *item = at;
                *off = 0;
            }
        }
        if let Some((g, _)) = &mut self.goal {
            if *g >= at + n {
                *g -= n;
            } else if *g >= at {
                self.goal = None;
            }
        }
        self.page_version = u64::MAX;
    }

    /// Item `item` changed (an edit, a streaming token, a new image): it is
    /// re-measured at the next frame. Many calls between frames cost one
    /// measurement per item.
    pub fn changed(&mut self, item: usize) {
        if let Some((h, _)) = self.index.height_of(item) {
            self.index.set_height(item, h, false);
        }
        self.page_version = u64::MAX;
    }

    /// Item `item`'s text changed from `old_bytes` to `new_bytes` bytes (streaming growth):
    /// the aggregate follows and the item is re-measured at the next frame.
    pub fn edited(&mut self, item: usize, old_bytes: u64, new_bytes: u64) {
        self.index.change_bytes(item, old_bytes, new_bytes);
        self.changed(item);
    }

    /// A known height for `item` (an image whose size arrived).
    pub fn set_known_height(&mut self, item: usize, h_px: f64) {
        self.index.set_height(item, Fx::from_px(h_px), true);
    }
}
