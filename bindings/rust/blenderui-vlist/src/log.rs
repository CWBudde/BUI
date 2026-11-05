//! A file-backed append-only message log: [`ItemSource`] for lists that do
//! not fit (or should not sit) in RAM. Two files: `<base>.idx` holds one
//! fixed 16-byte record per message, `<base>.txt` the text arena. A fetch is
//! two positioned reads (`seek_read` on Windows, `read_at` on Unix), so
//! reading a page costs the same at item 10 and item 10 million and no
//! file position is shared. `bytes(range)` is two 8-byte reads, which is
//! what lets the height index be built over 10M messages without touching
//! them. Ids are `index + 1`.
//!
//! Record layout (little endian): text offset `u40` (5 bytes), text length
//! `u16`, time `u32`, sender `u16`, flags `u8`, media height in 4 px units
//! `u8`, one spare byte.

use std::fs::File;
use std::io::{self, BufWriter, Write};
use std::ops::Range;
use std::path::{Path, PathBuf};

use crate::source::{ItemSource, Message, Page, Record, FLAG_PLACEHOLDER};

const REC: usize = 16;
const MAX_TEXT: usize = u16::MAX as usize;

fn paths(base: &Path) -> (PathBuf, PathBuf) {
    let name = base.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
    (base.with_file_name(format!("{name}.idx")), base.with_file_name(format!("{name}.txt")))
}

/// A positioned read that fills `buf` (no shared file position).
fn read_exact_at(f: &File, buf: &mut [u8], off: u64) -> io::Result<()> {
    #[cfg(windows)]
    {
        use std::os::windows::fs::FileExt;
        let mut done = 0;
        while done < buf.len() {
            let n = f.seek_read(&mut buf[done..], off + done as u64)?;
            if n == 0 {
                return Err(io::ErrorKind::UnexpectedEof.into());
            }
            done += n;
        }
        Ok(())
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::FileExt;
        f.read_exact_at(buf, off)
    }
}

/// Appends messages to a log.
pub struct LogWriter {
    idx: BufWriter<File>,
    txt: BufWriter<File>,
    arena: u64,
    count: u64,
}

impl LogWriter {
    /// Create (truncating) the log at `base`.
    pub fn create(base: &Path) -> io::Result<Self> {
        let (i, t) = paths(base);
        Ok(Self { idx: BufWriter::with_capacity(1 << 20, File::create(i)?), txt: BufWriter::with_capacity(1 << 20, File::create(t)?), arena: 0, count: 0 })
    }

    /// Append `m` (its id is ignored: ids are positions). Text past 64 KB is cut at a character boundary.
    pub fn append(&mut self, m: &Message) -> io::Result<()> {
        let mut len = m.text.len().min(MAX_TEXT);
        while !m.text.is_char_boundary(len) {
            len -= 1;
        }
        let mut r = [0u8; REC];
        r[..5].copy_from_slice(&self.arena.to_le_bytes()[..5]);
        r[5..7].copy_from_slice(&(len as u16).to_le_bytes());
        r[7..11].copy_from_slice(&m.time.to_le_bytes());
        r[11..13].copy_from_slice(&m.sender.to_le_bytes());
        r[13] = m.flags;
        r[14] = (m.media_h / 4).min(255) as u8;
        self.idx.write_all(&r)?;
        self.txt.write_all(&m.text.as_bytes()[..len])?;
        self.arena += len as u64;
        self.count += 1;
        Ok(())
    }

    /// Messages written.
    #[must_use]
    pub const fn count(&self) -> u64 {
        self.count
    }

    /// Flush and close.
    pub fn finish(mut self) -> io::Result<()> {
        self.idx.flush()?;
        self.txt.flush()
    }
}

/// The read side.
pub struct LogSource {
    idx: File,
    txt: File,
    len: usize,
    arena: u64,
    version: u64,
}

struct Rec {
    off: u64,
    len: u16,
    time: u32,
    sender: u16,
    flags: u8,
    media_h: u16,
}

impl Rec {
    fn parse(b: &[u8]) -> Self {
        let mut o = [0u8; 8];
        o[..5].copy_from_slice(&b[..5]);
        Self {
            off: u64::from_le_bytes(o),
            len: u16::from_le_bytes([b[5], b[6]]),
            time: u32::from_le_bytes([b[7], b[8], b[9], b[10]]),
            sender: u16::from_le_bytes([b[11], b[12]]),
            flags: b[13],
            media_h: u16::from(b[14]) * 4,
        }
    }
}

impl LogSource {
    /// Open the log at `base`.
    pub fn open(base: &Path) -> io::Result<Self> {
        let (i, t) = paths(base);
        let (idx, txt) = (File::open(i)?, File::open(t)?);
        let mut s = Self { idx, txt, len: 0, arena: 0, version: 0 };
        s.reload()?;
        Ok(s)
    }

    /// Pick up messages appended since (bumps the version when there are some).
    pub fn reload(&mut self) -> io::Result<()> {
        let len = (self.idx.metadata()?.len() / REC as u64) as usize;
        let arena = self.txt.metadata()?.len();
        if len != self.len || arena != self.arena {
            self.version += 1;
        }
        (self.len, self.arena) = (len, arena);
        Ok(())
    }

    /// Offset of record `i`'s text (`len` gives the arena size).
    fn text_off(&self, i: usize) -> u64 {
        if i >= self.len {
            return self.arena;
        }
        let mut b = [0u8; 8];
        read_exact_at(&self.idx, &mut b[..5], (i * REC) as u64).map_or(self.arena, |()| u64::from_le_bytes(b))
    }

    fn placeholders(page: &mut Page, range: Range<usize>) {
        let a = page.text_mut().len();
        for i in range {
            page.push_span(&Record { id: i as u64 + 1, time: 0, sender: 0, flags: FLAG_PLACEHOLDER, media_h: 0, text: "" }, a..a);
        }
    }
}

impl ItemSource for LogSource {
    fn len(&self) -> usize {
        self.len
    }

    fn version(&self) -> u64 {
        self.version
    }

    fn bytes(&self, r: Range<usize>) -> u64 {
        let (a, b) = (r.start.min(self.len), r.end.min(self.len));
        self.text_off(b.max(a)) - self.text_off(a)
    }

    fn fetch(&self, range: Range<usize>, page: &mut Page) {
        let end = range.end.min(self.len);
        page.reset(range.start);
        if range.start >= end {
            return;
        }
        // the records, through a stack buffer in chunks; spans are relative to the first text offset
        let mut buf = [0u8; REC * 128];
        let (mut base, mut stop, mut i) = (0u64, 0u64, range.start);
        while i < end {
            let n = (end - i).min(128);
            if read_exact_at(&self.idx, &mut buf[..n * REC], (i * REC) as u64).is_err() {
                page.reset(range.start);
                Self::placeholders(page, range.start..end);
                return;
            }
            for (k, b) in buf[..n * REC].as_chunks::<REC>().0.iter().enumerate() {
                let m = Rec::parse(b);
                if i + k == range.start {
                    base = m.off;
                }
                stop = m.off + u64::from(m.len);
                let a = (m.off - base) as usize;
                let r = Record { id: (i + k) as u64 + 1, time: m.time, sender: m.sender, flags: m.flags, media_h: m.media_h, text: "" };
                page.push_span(&r, a..a + m.len as usize);
            }
            i += n;
        }
        let mut bytes = std::mem::take(page.scratch_mut());
        bytes.resize((stop - base) as usize, 0);
        if read_exact_at(&self.txt, &mut bytes, base).is_err() {
            *page.scratch_mut() = bytes;
            page.reset(range.start);
            Self::placeholders(page, range.start..end);
            return;
        }
        page.text_mut().push_str(&String::from_utf8_lossy(&bytes));
        *page.scratch_mut() = bytes;
    }

    fn id_at(&self, i: usize) -> Option<u64> {
        (i < self.len).then(|| i as u64 + 1)
    }

    fn index_of(&self, id: u64) -> Option<usize> {
        let i = usize::try_from(id.checked_sub(1)?).ok()?;
        (i < self.len).then_some(i)
    }
}
