// Copyright 2026 ansiapps. Neumetik MIDI, MIT licensed: see LICENSE.

//! A SoundFont written out: `INFO`, `sdta` (each sample followed by 46
//! silent ones, as the specification asks) and `pdta`'s nine tables, each
//! ending in its terminal record. A zone's generators go key range first,
//! velocity range next and its target last, the order the specification
//! requires.

use super::generator::{INSTRUMENT, KEY_RANGE, SAMPLE, VEL_RANGE};
use super::{Link, SoundFont, Zone};
use crate::Error;

pub(super) fn chunk(id: &[u8; 4], body: &[u8]) -> Vec<u8> {
    let mut c = Vec::with_capacity(body.len() + 9);
    c.extend_from_slice(id);
    c.extend_from_slice(&(body.len() as u32).to_le_bytes());
    c.extend_from_slice(body);
    if body.len() % 2 == 1 {
        c.push(0);
    }
    c
}

pub(super) fn list(kind: &[u8; 4], parts: &[Vec<u8>]) -> Vec<u8> {
    let mut body = kind.to_vec();
    for p in parts {
        body.extend_from_slice(p);
    }
    chunk(b"LIST", &body)
}

/// A name in 20 bytes, cut on a character's boundary, NUL-padded.
fn name20(name: &str) -> [u8; 20] {
    let mut n = [0u8; 20];
    let mut end = name.len().min(19);
    while !name.is_char_boundary(end) {
        end -= 1;
    }
    n[..end].copy_from_slice(&name.as_bytes()[..end]);
    n
}

/// An `INFO` string: NUL-terminated, at most `most` bytes with it.
fn text(id: &[u8; 4], s: &str, most: usize) -> Vec<u8> {
    let mut end = s.len().min(most - 1);
    while !s.is_char_boundary(end) {
        end -= 1;
    }
    let mut b = s.as_bytes()[..end].to_vec();
    b.push(0);
    if b.len() % 2 == 1 {
        b.push(0);
    }
    chunk(id, &b)
}

fn le16(v: &mut Vec<u8>, x: u16) {
    v.extend_from_slice(&x.to_le_bytes());
}

fn le32(v: &mut Vec<u8>, x: u32) {
    v.extend_from_slice(&x.to_le_bytes());
}

/// A list of zones (bags, generators and modulators) being written.
#[derive(Default)]
struct Zones {
    bags: Vec<u8>,
    gens: Vec<u8>,
    mods: Vec<u8>,
    gen_count: usize,
    mod_count: usize,
    bag_count: usize,
}

impl Zones {
    /// A zone; `target` is the generator its target goes in, and how many
    /// there are to point at.
    fn add(&mut self, z: &Zone, target: u16, count: usize) -> Result<(), Error> {
        le16(&mut self.bags, u16::try_from(self.gen_count).map_err(|_| Error::Unwritable)?);
        le16(&mut self.bags, u16::try_from(self.mod_count).map_err(|_| Error::Unwritable)?);
        self.bag_count += 1;
        let mut gen = |op: u16, amount: [u8; 2]| {
            le16(&mut self.gens, op);
            self.gens.extend_from_slice(&amount);
            self.gen_count += 1;
        };
        if let Some((lo, hi)) = z.keys {
            gen(KEY_RANGE, [lo.min(127), hi.min(127)]);
        }
        if let Some((lo, hi)) = z.vels {
            gen(VEL_RANGE, [lo.min(127), hi.min(127)]);
        }
        let mut seen = [false; 64];
        for &(op, amount) in &z.generators {
            // The ranges and targets are the zone's own fields; a number
            // past the specification's, or one given twice, isn't written.
            if [KEY_RANGE, VEL_RANGE, INSTRUMENT, SAMPLE].contains(&op) || op >= 60 || seen[usize::from(op)] {
                continue;
            }
            seen[usize::from(op)] = true;
            gen(op, amount.to_le_bytes());
        }
        if let Some(t) = z.target {
            if t >= count {
                return Err(Error::Unwritable);
            }
            gen(target, (t as u16).to_le_bytes());
        }
        for m in &z.modulators {
            for x in [m.source, m.destination, m.amount as u16, m.amount_source, m.transform] {
                le16(&mut self.mods, x);
            }
            self.mod_count += 1;
        }
        Ok(())
    }

    /// The zones of a preset or instrument: its global zone, if it sets
    /// anything, then its own zones that play something.
    fn add_all(&mut self, global: &Zone, zones: &[Zone], target: u16, count: usize) -> Result<(), Error> {
        if !global.is_empty() {
            let mut g = global.clone();
            g.target = None;
            self.add(&g, target, count)?;
        }
        for z in zones.iter().filter(|z| z.target.is_some()) {
            self.add(z, target, count)?;
        }
        Ok(())
    }

    fn bag_index(&self) -> Result<u16, Error> {
        u16::try_from(self.bag_count).map_err(|_| Error::Unwritable)
    }

    /// The terminal bag, generator and modulator.
    fn finish(&mut self) -> Result<(), Error> {
        le16(&mut self.bags, u16::try_from(self.gen_count).map_err(|_| Error::Unwritable)?);
        le16(&mut self.bags, u16::try_from(self.mod_count).map_err(|_| Error::Unwritable)?);
        self.gens.extend_from_slice(&[0; 4]);
        self.mods.extend_from_slice(&[0; 10]);
        Ok(())
    }
}

pub fn write(sf: &SoundFont) -> Result<Vec<u8>, Error> {
    let total: usize = sf.samples.iter().map(|s| s.data.len() + 46).sum();
    if total > (u32::MAX / 2) as usize || sf.samples.len() >= 65_535 || sf.instruments.len() >= 65_535 || sf.presets.len() >= 65_535 {
        return Err(Error::Unwritable);
    }

    let mut smpl = Vec::with_capacity(total * 2);
    let mut shdr = Vec::with_capacity((sf.samples.len() + 1) * 46);
    for (i, s) in sf.samples.iter().enumerate() {
        let start = (smpl.len() / 2) as u32;
        let len = s.data.len() as u32;
        if s.has_loop() && s.loop_end > len {
            return Err(Error::Unwritable);
        }
        for x in &s.data {
            smpl.extend_from_slice(&x.to_le_bytes());
        }
        smpl.extend_from_slice(&[0; 92]);
        let (kind, other) = match s.link {
            Link::Mono => (1u16, 0),
            Link::Right(o) => (2, o),
            Link::Left(o) => (4, o),
        };
        let paired = match (s.link, sf.samples.get(other).map(|o| o.link)) {
            (Link::Mono, _) => true,
            (Link::Left(_), Some(Link::Right(back))) | (Link::Right(_), Some(Link::Left(back))) => back == i,
            _ => false,
        };
        if !paired {
            return Err(Error::Unwritable);
        }
        shdr.extend_from_slice(&name20(&s.name));
        let (ls, le) = if s.has_loop() { (s.loop_start, s.loop_end) } else { (0, 0) };
        for x in [start, start + len, start + ls, start + le, s.rate] {
            le32(&mut shdr, x);
        }
        shdr.push(s.root.min(127));
        shdr.push(s.correction as u8);
        le16(&mut shdr, other as u16);
        le16(&mut shdr, kind);
    }
    shdr.extend_from_slice(&name20("EOS"));
    shdr.extend_from_slice(&[0; 26]);

    let mut inst = Vec::new();
    let mut izones = Zones::default();
    for i in &sf.instruments {
        inst.extend_from_slice(&name20(&i.name));
        le16(&mut inst, izones.bag_index()?);
        izones.add_all(&i.global, &i.zones, SAMPLE, sf.samples.len())?;
    }
    inst.extend_from_slice(&name20("EOI"));
    le16(&mut inst, izones.bag_index()?);
    izones.finish()?;

    let mut phdr = Vec::new();
    let mut pzones = Zones::default();
    for p in &sf.presets {
        phdr.extend_from_slice(&name20(&p.name));
        le16(&mut phdr, p.program);
        le16(&mut phdr, p.bank);
        le16(&mut phdr, pzones.bag_index()?);
        phdr.extend_from_slice(&[0; 12]);
        pzones.add_all(&p.global, &p.zones, INSTRUMENT, sf.instruments.len())?;
    }
    phdr.extend_from_slice(&name20("EOP"));
    phdr.extend_from_slice(&[0; 4]);
    le16(&mut phdr, pzones.bag_index()?);
    phdr.extend_from_slice(&[0; 12]);
    pzones.finish()?;

    let mut ifil = Vec::new();
    le16(&mut ifil, 2);
    le16(&mut ifil, 4);
    let i = &sf.info;
    let mut info = vec![chunk(b"ifil", &ifil), text(b"isng", if i.engine.is_empty() { "EMU8000" } else { &i.engine }, 256), text(b"INAM", if i.name.is_empty() { "Untitled" } else { &i.name }, 256)];
    for (id, s, most) in [(b"ICRD", &i.date, 256), (b"IENG", &i.author, 256), (b"IPRD", &i.product, 256), (b"ICOP", &i.copyright, 256), (b"ICMT", &i.comment, 65_536), (b"ISFT", &i.software, 256)] {
        if !s.is_empty() {
            info.push(text(id, s, most));
        }
    }

    let pdta = list(
        b"pdta",
        &[
            chunk(b"phdr", &phdr),
            chunk(b"pbag", &pzones.bags),
            chunk(b"pmod", &pzones.mods),
            chunk(b"pgen", &pzones.gens),
            chunk(b"inst", &inst),
            chunk(b"ibag", &izones.bags),
            chunk(b"imod", &izones.mods),
            chunk(b"igen", &izones.gens),
            chunk(b"shdr", &shdr),
        ],
    );
    let info = list(b"INFO", &info);
    let sdta = list(b"sdta", &[chunk(b"smpl", &smpl)]);
    let mut body = b"sfbk".to_vec();
    body.reserve(info.len() + sdta.len() + pdta.len());
    body.extend_from_slice(&info);
    body.extend_from_slice(&sdta);
    body.extend_from_slice(&pdta);
    Ok(chunk(b"RIFF", &body))
}
