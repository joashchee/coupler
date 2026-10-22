// Copyright 2026 ansiapps. Neumetik MIDI, MIT licensed: see LICENSE.

//! RIFF chunks, as SoundFonts and DLS files are made of, and little-endian
//! numbers read out of them. A chunk that says it's longer than what's
//! left is cut to what's there: plenty of banks in the wild are a few
//! bytes out.

pub struct Chunk<'a> {
    pub id: [u8; 4],
    pub data: &'a [u8],
}

impl<'a> Chunk<'a> {
    /// A `LIST` (or `RIFF`): its type and what's in it.
    pub fn list(&self) -> Option<([u8; 4], &'a [u8])> {
        if (&self.id == b"LIST" || &self.id == b"RIFF") && self.data.len() >= 4 {
            Some((fourcc(self.data, 0), &self.data[4..]))
        } else {
            None
        }
    }
}

/// The chunks one after another in `data`.
pub fn chunks(data: &[u8]) -> impl Iterator<Item = Chunk<'_>> {
    let mut at = 0usize;
    std::iter::from_fn(move || {
        if at + 8 > data.len() {
            return None;
        }
        let id = fourcc(data, at);
        let size = u32_at(data, at + 4) as usize;
        let start = at + 8;
        let end = start.saturating_add(size).min(data.len());
        at = end + (size & 1);
        Some(Chunk { id, data: &data[start..end] })
    })
}

/// The first chunk `id` in `data`, or the first list of type `id`.
pub fn find<'a>(data: &'a [u8], id: &[u8; 4]) -> Option<&'a [u8]> {
    chunks(data).find_map(|c| match c.list() {
        Some((kind, body)) if &kind == id => Some(body),
        _ if &c.id == id => Some(c.data),
        _ => None,
    })
}

pub fn fourcc(d: &[u8], at: usize) -> [u8; 4] {
    let mut id = [0u8; 4];
    if let Some(s) = d.get(at..at + 4) {
        id.copy_from_slice(s);
    }
    id
}

pub fn u16_at(d: &[u8], at: usize) -> u16 {
    d.get(at..at + 2).map_or(0, |b| u16::from_le_bytes([b[0], b[1]]))
}

pub fn i16_at(d: &[u8], at: usize) -> i16 {
    u16_at(d, at) as i16
}

pub fn u32_at(d: &[u8], at: usize) -> u32 {
    d.get(at..at + 4).map_or(0, |b| u32::from_le_bytes([b[0], b[1], b[2], b[3]]))
}

pub fn i32_at(d: &[u8], at: usize) -> i32 {
    u32_at(d, at) as i32
}

/// A fixed-length name, up to its first NUL, trimmed.
pub fn name(d: &[u8]) -> String {
    let end = d.iter().position(|&b| b == 0).unwrap_or(d.len());
    String::from_utf8_lossy(&d[..end]).trim().to_string()
}
