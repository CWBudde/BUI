//! Where the items come from.
//!
//! An [`ItemSource`] answers `len`, an aggregate of text bytes per range (so
//! an estimate needs no per-item data), and `fetch(range, page)`: the
//! records of a range copied into a reusable [`Page`] (a metadata vector and
//! one text buffer: after the first fetches, fetching allocates nothing).
//! Items not loaded yet come back as placeholders; [`ItemSource::ensure_loaded`]
//! asks for them without blocking. The in-memory [`VecSource`] is here; the
//! file-backed log is [`crate::log`].

use std::ops::Range;

/// The group header (avatar, name, time) is shown above this item.
pub const FLAG_HEADER: u8 = 1;
/// A date divider is shown above this item.
pub const FLAG_DIVIDER: u8 = 2;
/// Not loaded yet: the text is empty and the height an estimate.
pub const FLAG_PLACEHOLDER: u8 = 4;
/// Sent by the local user.
pub const FLAG_OWN: u8 = 8;

/// One item. Everything that decides its height is in the record itself
/// (neighbour-dependent layout was decided when it was inserted and lives
/// in `flags`), so a height depends on its own record only.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct Record<'a> {
    /// Stable, ascending with the index.
    pub id: u64,
    /// Seconds since the epoch.
    pub time: u32,
    pub sender: u16,
    pub flags: u8,
    /// Height of an attached image in px (0: none). Known before the image
    /// loads, so loading it shifts nothing.
    pub media_h: u16,
    pub text: &'a str,
    /// The attached image's file path ("" with `media_h` > 0: a placeholder).
    /// Drawn through `PaintSink::image`: only visible items load it.
    pub media: &'a str,
}

#[derive(Copy, Clone, Debug)]
struct Meta {
    id: u64,
    time: u32,
    sender: u16,
    flags: u8,
    media_h: u16,
    text: (u32, u32),
    media: (u32, u32),
}

impl Meta {
    const fn new(r: &Record<'_>, text: (u32, u32), media: (u32, u32)) -> Self {
        Self { id: r.id, time: r.time, sender: r.sender, flags: r.flags, media_h: r.media_h, text, media }
    }
}

/// A reusable run of records.
#[derive(Default)]
pub struct Page {
    start: usize,
    metas: Vec<Meta>,
    text: String,
    scratch: Vec<u8>,
}

impl Page {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Empty the page, to hold records from `start` on.
    pub fn reset(&mut self, start: usize) {
        self.start = start;
        self.metas.clear();
        self.text.clear();
    }

    /// Append the next record.
    pub fn push(&mut self, r: &Record<'_>) {
        let a = self.text.len() as u32;
        self.text.push_str(r.text);
        let text = (a, self.text.len() as u32);
        let media = self.push_media(r);
        self.metas.push(Meta::new(r, text, media));
    }

    /// The text buffer, for sources that read straight into it; then call
    /// [`Page::push_span`] for each record.
    pub const fn text_mut(&mut self) -> &mut String {
        &mut self.text
    }

    /// A byte buffer sources use for reads (take it, use it, put it back: it keeps its capacity).
    pub const fn scratch_mut(&mut self) -> &mut Vec<u8> {
        &mut self.scratch
    }

    /// Append a record whose text (`r.text` is ignored) is already in the buffer at `text`.
    pub fn push_span(&mut self, r: &Record<'_>, text: Range<usize>) {
        let media = self.push_media(r);
        self.metas.push(Meta::new(r, (text.start as u32, text.end as u32), media));
    }

    /// `r.media` appended to the buffer (empty: no bytes).
    fn push_media(&mut self, r: &Record<'_>) -> (u32, u32) {
        let a = self.text.len() as u32;
        self.text.push_str(r.media);
        (a, self.text.len() as u32)
    }

    /// The items the page holds.
    #[must_use]
    pub const fn range(&self) -> Range<usize> {
        self.start..self.start + self.metas.len()
    }

    #[must_use]
    pub fn record(&self, item: usize) -> Option<Record<'_>> {
        let m = self.metas.get(item.checked_sub(self.start)?)?;
        let text = self.text.get(m.text.0 as usize..m.text.1 as usize)?;
        let media = self.text.get(m.media.0 as usize..m.media.1 as usize)?;
        Some(Record { id: m.id, time: m.time, sender: m.sender, flags: m.flags, media_h: m.media_h, text, media })
    }
}

/// See the module docs.
pub trait ItemSource {
    fn len(&self) -> usize;

    fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Changes whenever the items change; a [`Page`] is refetched after it moves.
    fn version(&self) -> u64;

    /// Text bytes in `range` (an aggregate: O(1) for a source with an offsets table).
    fn bytes(&self, range: Range<usize>) -> u64;

    /// Fill `page` with the records of `range` (clamped to `len`), in order.
    /// Items not loaded yet are [`FLAG_PLACEHOLDER`] records.
    fn fetch(&self, range: Range<usize>, page: &mut Page);

    /// The stable id of `index`.
    fn id_at(&self, index: usize) -> Option<u64>;

    /// The index of id `id` (ids ascend with the index: a binary search).
    fn index_of(&self, id: u64) -> Option<usize>;

    /// Ask for `range` to be loaded; never blocks. A synchronous source has nothing to do.
    fn ensure_loaded(&self, _range: Range<usize>) {}

    /// Stop loading `range` (the user scrolled away).
    fn cancel(&self, _range: Range<usize>) {}
}

/// An owned message.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Message {
    pub id: u64,
    pub time: u32,
    pub sender: u16,
    pub flags: u8,
    pub media_h: u16,
    pub text: String,
    /// The attached image's file path (see [`Record::media`]).
    pub media: String,
}

impl Message {
    #[must_use]
    pub fn record(&self) -> Record<'_> {
        Record { id: self.id, time: self.time, sender: self.sender, flags: self.flags, media_h: self.media_h, text: &self.text, media: &self.media }
    }
}

/// An in-memory source.
pub struct VecSource {
    items: Vec<Message>,
    /// Text bytes before each item: `bytes(a..b)` is a subtraction.
    cum: Vec<u64>,
    version: u64,
}

impl Default for VecSource {
    fn default() -> Self {
        Self::new()
    }
}

impl VecSource {
    #[must_use]
    pub fn new() -> Self {
        Self { items: Vec::new(), cum: vec![0], version: 0 }
    }

    #[must_use]
    pub fn from_messages(items: Vec<Message>) -> Self {
        let mut s = Self::new();
        s.items.reserve(items.len());
        for m in items {
            s.push(m);
        }
        s
    }

    pub fn push(&mut self, m: Message) {
        self.cum.push(self.cum[self.items.len()] + m.text.len() as u64);
        self.items.push(m);
        self.version += 1;
    }

    /// Insert older messages before everything (a history page; their ids precede the first).
    pub fn prepend(&mut self, mut older: Vec<Message>) {
        older.append(&mut self.items);
        let v = self.version + 1;
        *self = Self::from_messages(older);
        self.version = v;
    }

    /// Replace item `i`'s text (streaming growth, an edit).
    pub fn set_text(&mut self, i: usize, text: &str) {
        let old = self.items[i].text.len() as u64;
        self.items[i].text.clear();
        self.items[i].text.push_str(text);
        let d = text.len() as u64;
        for c in &mut self.cum[i + 1..] {
            *c = *c - old + d;
        }
        self.version += 1;
    }

    /// Remove item `i`.
    pub fn remove(&mut self, i: usize) -> Message {
        let m = self.items.remove(i);
        let w = m.text.len() as u64;
        self.cum.remove(i + 1);
        for c in &mut self.cum[i + 1..] {
            *c -= w;
        }
        self.version += 1;
        m
    }

    #[must_use]
    pub fn get(&self, i: usize) -> Option<&Message> {
        self.items.get(i)
    }
}

impl ItemSource for VecSource {
    fn len(&self) -> usize {
        self.items.len()
    }
    fn version(&self) -> u64 {
        self.version
    }
    fn bytes(&self, r: Range<usize>) -> u64 {
        let (a, b) = (r.start.min(self.items.len()), r.end.min(self.items.len()));
        self.cum[b.max(a)] - self.cum[a]
    }
    fn fetch(&self, range: Range<usize>, page: &mut Page) {
        let end = range.end.min(self.items.len());
        page.reset(range.start);
        for m in self.items.get(range.start..end).unwrap_or_default() {
            page.push(&m.record());
        }
    }
    fn id_at(&self, i: usize) -> Option<u64> {
        self.items.get(i).map(|m| m.id)
    }
    fn index_of(&self, id: u64) -> Option<usize> {
        self.items.binary_search_by_key(&id, |m| m.id).ok()
    }
}

/// A shared or boxed source is a source (so one source can feed several
/// consumers, or be chosen at run time).
macro_rules! forward_source {
    ($($ptr:ty),*) => {$(
        impl<T: ItemSource + ?Sized> ItemSource for $ptr {
            fn len(&self) -> usize {
                (**self).len()
            }
            fn version(&self) -> u64 {
                (**self).version()
            }
            fn bytes(&self, range: Range<usize>) -> u64 {
                (**self).bytes(range)
            }
            fn fetch(&self, range: Range<usize>, page: &mut Page) {
                (**self).fetch(range, page);
            }
            fn id_at(&self, index: usize) -> Option<u64> {
                (**self).id_at(index)
            }
            fn index_of(&self, id: u64) -> Option<usize> {
                (**self).index_of(id)
            }
            fn ensure_loaded(&self, range: Range<usize>) {
                (**self).ensure_loaded(range);
            }
            fn cancel(&self, range: Range<usize>) {
                (**self).cancel(range);
            }
        }
    )*};
}

forward_source!(Box<T>, std::rc::Rc<T>, std::sync::Arc<T>);
