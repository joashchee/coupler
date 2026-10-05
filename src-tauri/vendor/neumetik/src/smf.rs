// Copyright 2026 ansiapps. Neumetik: see README.md for its license.

//! Standard MIDI Files (formats 0, 1 and 2, and RIFF's RMID wrapper)
//! read into one list of events in seconds, every track merged and the
//! tempo map applied. Pure and unit-tested.

#[derive(Clone, Debug, PartialEq)]
pub enum Message {
    /// A channel message: its status byte and up to two data bytes.
    Channel(u8, u8, u8),
    /// A system exclusive message, without its F0 and F7.
    SysEx(Vec<u8>),
}

#[derive(Clone, Debug, PartialEq)]
pub struct Event {
    pub seconds: f64,
    pub message: Message,
}

struct Reader<'a> {
    bytes: &'a [u8],
    at: usize,
}

impl<'a> Reader<'a> {
    fn byte(&mut self) -> Result<u8, String> {
        let b = *self.bytes.get(self.at).ok_or("it ends too soon")?;
        self.at += 1;
        Ok(b)
    }

    fn take(&mut self, n: usize) -> Result<&'a [u8], String> {
        let end = self.at.checked_add(n).filter(|&e| e <= self.bytes.len()).ok_or("it ends too soon")?;
        let s = &self.bytes[self.at..end];
        self.at = end;
        Ok(s)
    }

    fn u16(&mut self) -> Result<u16, String> {
        let b = self.take(2)?;
        Ok(u16::from_be_bytes([b[0], b[1]]))
    }

    fn u32(&mut self) -> Result<u32, String> {
        let b = self.take(4)?;
        Ok(u32::from_be_bytes([b[0], b[1], b[2], b[3]]))
    }

    /// A variable-length number: seven bits a byte, the top bit says more.
    fn vlq(&mut self) -> Result<u32, String> {
        let mut n = 0u32;
        for _ in 0..4 {
            let b = self.byte()?;
            n = (n << 7) | u32::from(b & 0x7f);
            if b & 0x80 == 0 {
                return Ok(n);
            }
        }
        Err("a length is too long".into())
    }
}

/// An event at a tick, in the order the file had it.
struct Timed {
    tick: u64,
    order: usize,
    what: What,
}

enum What {
    Message(Message),
    /// Microseconds a quarter note.
    Tempo(u32),
}

/// Every event in the file, by time.
pub fn parse(bytes: &[u8]) -> Result<Vec<Event>, String> {
    let bytes = unwrap_rmid(bytes);
    let mut r = Reader { bytes, at: 0 };
    if r.take(4)? != b"MThd" {
        return Err("it isn't a MIDI file".into());
    }
    let len = r.u32()? as usize;
    let start = r.at;
    let format = r.u16()?;
    let tracks = r.u16()?;
    let division = r.u16()?;
    r.at = start + len;
    if division == 0 {
        return Err("its timing is zero".into());
    }

    let mut timed = Vec::new();
    // Format 2's tracks are songs one after another.
    let mut offset = 0u64;
    for _ in 0..tracks {
        if r.at + 8 > bytes.len() {
            break;
        }
        let id = r.take(4)?;
        let len = r.u32()? as usize;
        let body = r.take(len.min(bytes.len() - r.at))?;
        if id != b"MTrk" {
            continue;
        }
        let end = read_track(body, offset, &mut timed);
        if format == 2 {
            offset = end;
        }
    }
    if timed.is_empty() {
        return Err("it has no music in it".into());
    }
    timed.sort_by_key(|t| (t.tick, t.order));

    // Ticks to seconds, through the tempo map.
    let smpte = division & 0x8000 != 0;
    let ticks_per_second = if smpte {
        let fps = match (division >> 8) as u8 as i8 {
            -29 => 29.97,
            f => -f64::from(f),
        };
        fps * f64::from(division & 0xff)
    } else {
        0.0
    };
    let per_quarter = f64::from(division & 0x7fff);
    let mut tempo = 500_000.0;
    let (mut last_tick, mut seconds) = (0u64, 0f64);
    let mut events = Vec::with_capacity(timed.len());
    for t in timed {
        let ticks = (t.tick - last_tick) as f64;
        seconds += if smpte { ticks / ticks_per_second } else { ticks * tempo / 1e6 / per_quarter };
        last_tick = t.tick;
        match t.what {
            What::Tempo(us) => tempo = f64::from(us.max(1)),
            What::Message(message) => events.push(Event { seconds, message }),
        }
    }
    Ok(events)
}

/// A RIFF RMID file is a MIDI file in a wrapper: the MIDI file inside it.
fn unwrap_rmid(bytes: &[u8]) -> &[u8] {
    if bytes.starts_with(b"RIFF") {
        if let Some(at) = bytes.windows(4).position(|w| w == b"MThd") {
            return &bytes[at..];
        }
    }
    bytes
}

/// Reads one track's events into `out`, returning the tick it ends on.
/// A damaged track keeps what was read before the damage.
fn read_track(body: &[u8], offset: u64, out: &mut Vec<Timed>) -> u64 {
    let mut r = Reader { bytes: body, at: 0 };
    let mut tick = offset;
    let mut running = 0u8;
    while r.at < body.len() {
        let Ok(delta) = r.vlq() else { break };
        tick += u64::from(delta);
        let Ok(mut status) = r.byte() else { break };
        let order = out.len();
        let mut push = |what| out.push(Timed { tick, order, what });
        match status {
            0xff => {
                let (Ok(kind), Ok(len)) = (r.byte(), r.vlq()) else { break };
                let Ok(data) = r.take(len as usize) else { break };
                match kind {
                    0x2f => break,
                    0x51 if data.len() == 3 => {
                        push(What::Tempo(u32::from(data[0]) << 16 | u32::from(data[1]) << 8 | u32::from(data[2])));
                    }
                    _ => {}
                }
            }
            0xf0 | 0xf7 => {
                let Ok(len) = r.vlq() else { break };
                let Ok(data) = r.take(len as usize) else { break };
                let data = data.strip_suffix(&[0xf7]).unwrap_or(data);
                push(What::Message(Message::SysEx(data.to_vec())));
            }
            _ => {
                let first = if status < 0x80 {
                    // Running status: this byte is the first data byte.
                    if running == 0 {
                        break;
                    }
                    let b = status;
                    status = running;
                    b
                } else {
                    running = status;
                    let Ok(b) = r.byte() else { break };
                    b
                };
                let second = if matches!(status & 0xf0, 0xc0 | 0xd0) {
                    0
                } else {
                    let Ok(b) = r.byte() else { break };
                    b
                };
                push(What::Message(Message::Channel(status, first & 0x7f, second & 0x7f)));
            }
        }
    }
    tick
}

#[cfg(test)]
pub mod tests {
    use super::*;

    /// A little MIDI file: one track per `tracks`, at 96 ticks a quarter.
    pub fn file(format: u16, tracks: &[Vec<u8>]) -> Vec<u8> {
        let mut out = b"MThd".to_vec();
        out.extend_from_slice(&6u32.to_be_bytes());
        out.extend_from_slice(&format.to_be_bytes());
        out.extend_from_slice(&(tracks.len() as u16).to_be_bytes());
        out.extend_from_slice(&96u16.to_be_bytes());
        for t in tracks {
            out.extend_from_slice(b"MTrk");
            out.extend_from_slice(&(t.len() as u32 + 4).to_be_bytes());
            out.extend_from_slice(t);
            out.extend_from_slice(&[0, 0xff, 0x2f, 0]);
        }
        out
    }

    #[test]
    fn tracks_merge_by_time_with_the_tempo() {
        // 120 bpm, then 60 bpm a quarter in.
        let tempo = vec![0, 0xff, 0x51, 3, 0x07, 0xa1, 0x20, 96, 0xff, 0x51, 3, 0x0f, 0x42, 0x40];
        // A note at once, running status for its off two quarters later.
        let notes = vec![0, 0x90, 60, 100, 0x81, 0x40, 60, 0];
        let events = parse(&file(1, &[tempo, notes])).unwrap();
        assert_eq!(events.len(), 2);
        assert_eq!(events[0].message, Message::Channel(0x90, 60, 100));
        // Half a second, then a second for the second quarter.
        assert!((events[1].seconds - 1.5).abs() < 1e-9, "{}", events[1].seconds);
    }

    #[test]
    fn sysex_comes_through_without_its_frame() {
        let gs_reset = vec![0, 0xf0, 10, 0x41, 0x10, 0x42, 0x12, 0x40, 0x00, 0x7f, 0x00, 0x41, 0xf7];
        let events = parse(&file(0, &[gs_reset])).unwrap();
        assert_eq!(events[0].message, Message::SysEx(vec![0x41, 0x10, 0x42, 0x12, 0x40, 0x00, 0x7f, 0x00, 0x41]));
    }

    #[test]
    fn format_two_plays_its_songs_in_turn() {
        let a = vec![0, 0x90, 60, 100, 96, 0x80, 60, 0];
        let b = vec![0, 0x90, 64, 100];
        let events = parse(&file(2, &[a, b])).unwrap();
        assert!((events[2].seconds - 0.5).abs() < 1e-9);
    }

    #[test]
    fn a_broken_file_is_refused_or_kept_as_far_as_it_goes() {
        assert!(parse(b"MThd").is_err());
        assert!(parse(b"hello there").is_err());
        let mut f = file(0, &[vec![0, 0x90, 60, 100, 10, 0x90]]);
        f.truncate(f.len() - 4);
        assert_eq!(parse(&f).unwrap().len(), 1);
    }

    #[test]
    fn an_rmid_wrapper_is_opened() {
        let mut riff = b"RIFF\0\0\0\0RMIDdata\0\0\0\0".to_vec();
        riff.extend(file(0, &[vec![0, 0x90, 60, 100]]));
        assert_eq!(parse(&riff).unwrap().len(), 1);
    }
}
