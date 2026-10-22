// Copyright 2026 ansiapps. Neumetik MIDI, MIT licensed: see LICENSE.

//! Small SoundFont and DLS files made from scratch, for the tests and the
//! docs' examples.

/// A sample: its data, rate, the key it sounds at, and its loop.
pub struct Sample {
    pub data: Vec<i16>,
    pub rate: u32,
    pub root: u8,
    pub looped: Option<(u32, u32)>,
}

/// A cycle of a sine `period` samples long, `cycles` times over, looped
/// round its last cycle.
pub fn sine(period: usize, cycles: usize, rate: u32, root: u8) -> Sample {
    let data = (0..period * cycles).map(|i| ((i as f32 / period as f32 * std::f32::consts::TAU).sin() * 16_000.0) as i16).collect();
    let end = (period * cycles) as u32;
    Sample { data, rate, root, looped: Some((end - period as u32, end)) }
}

fn chunk(id: &[u8; 4], body: &[u8]) -> Vec<u8> {
    let mut c = id.to_vec();
    c.extend_from_slice(&(body.len() as u32).to_le_bytes());
    c.extend_from_slice(body);
    if body.len() % 2 == 1 {
        c.push(0);
    }
    c
}

fn list(id: &[u8; 4], kind: &[u8; 4], parts: &[Vec<u8>]) -> Vec<u8> {
    let mut body = kind.to_vec();
    for p in parts {
        body.extend_from_slice(p);
    }
    chunk(id, &body)
}

fn name20(name: &str) -> [u8; 20] {
    let mut n = [0u8; 20];
    for (d, s) in n.iter_mut().zip(name.bytes().take(19)) {
        *d = s;
    }
    n
}

/// A zone: generators (number, amount) and the sample it plays.
pub type Zone = (Vec<(u16, i16)>, u16);

/// A preset: bank, program, name and its instrument's zones.
pub type Preset<'a> = (u16, u16, &'a str, Vec<Zone>);

/// A SoundFont: each preset plays an instrument of its own.
pub fn sf2(presets: &[Preset], samples: &[Sample]) -> Vec<u8> {
    let le16 = |v: &mut Vec<u8>, x: u16| v.extend_from_slice(&x.to_le_bytes());
    let le32 = |v: &mut Vec<u8>, x: u32| v.extend_from_slice(&x.to_le_bytes());
    let mut smpl = Vec::new();
    let mut shdr = Vec::new();
    let mut at = 0u32;
    for (i, s) in samples.iter().enumerate() {
        for x in &s.data {
            smpl.extend_from_slice(&x.to_le_bytes());
        }
        smpl.extend_from_slice(&[0; 92]);
        shdr.extend_from_slice(&name20(&format!("sample {i}")));
        let (ls, le) = s.looped.unwrap_or((0, 0));
        for x in [at, at + s.data.len() as u32, at + ls, at + le, s.rate] {
            le32(&mut shdr, x);
        }
        shdr.extend_from_slice(&[s.root, 0]);
        le16(&mut shdr, 0);
        le16(&mut shdr, 1);
        at += s.data.len() as u32 + 46;
    }
    shdr.extend_from_slice(&[0; 46]);

    let (mut phdr, mut pbag, mut pgen) = (Vec::new(), Vec::new(), Vec::new());
    let (mut inst, mut ibag, mut igen) = (Vec::new(), Vec::new(), Vec::new());
    for (i, (bank, program, name, zones)) in presets.iter().enumerate() {
        phdr.extend_from_slice(&name20(name));
        le16(&mut phdr, *program);
        le16(&mut phdr, *bank);
        le16(&mut phdr, i as u16);
        phdr.extend_from_slice(&[0; 12]);
        le16(&mut pbag, (pgen.len() / 4) as u16);
        le16(&mut pbag, 0);
        le16(&mut pgen, 41);
        le16(&mut pgen, i as u16);
        inst.extend_from_slice(&name20(name));
        le16(&mut inst, (ibag.len() / 4) as u16);
        for (gens, sample) in zones {
            le16(&mut ibag, (igen.len() / 4) as u16);
            le16(&mut ibag, 0);
            for (op, amount) in gens {
                le16(&mut igen, *op);
                igen.extend_from_slice(&amount.to_le_bytes());
            }
            le16(&mut igen, 53);
            le16(&mut igen, *sample);
        }
    }
    phdr.extend_from_slice(&name20("EOP"));
    phdr.extend_from_slice(&[0; 4]);
    le16(&mut phdr, presets.len() as u16);
    phdr.extend_from_slice(&[0; 12]);
    le16(&mut pbag, (pgen.len() / 4) as u16);
    le16(&mut pbag, 0);
    pgen.extend_from_slice(&[0; 4]);
    inst.extend_from_slice(&name20("EOI"));
    le16(&mut inst, (ibag.len() / 4) as u16);
    le16(&mut ibag, (igen.len() / 4) as u16);
    le16(&mut ibag, 0);
    igen.extend_from_slice(&[0; 4]);

    let mut ifil = Vec::new();
    le16(&mut ifil, 2);
    le16(&mut ifil, 1);
    let info = list(b"LIST", b"INFO", &[chunk(b"ifil", &ifil), chunk(b"INAM", b"Test Font\0")]);
    let sdta = list(b"LIST", b"sdta", &[chunk(b"smpl", &smpl)]);
    let pdta = list(
        b"LIST",
        b"pdta",
        &[
            chunk(b"phdr", &phdr),
            chunk(b"pbag", &pbag),
            chunk(b"pmod", &[0; 10]),
            chunk(b"pgen", &pgen),
            chunk(b"inst", &inst),
            chunk(b"ibag", &ibag),
            chunk(b"imod", &[0; 10]),
            chunk(b"igen", &igen),
            chunk(b"shdr", &shdr),
        ],
    );
    list(b"RIFF", b"sfbk", &[info, sdta, pdta])
}

/// A connection block: source, control, destination, and its scale.
#[cfg(test)]
pub type Connection = (u16, u16, u16, i32);

#[cfg(test)]
fn art1(connections: &[Connection]) -> Vec<u8> {
    let mut a = Vec::new();
    a.extend_from_slice(&8u32.to_le_bytes());
    a.extend_from_slice(&(connections.len() as u32).to_le_bytes());
    for (s, c, d, scale) in connections {
        for x in [*s, *c, *d, 0] {
            a.extend_from_slice(&x.to_le_bytes());
        }
        a.extend_from_slice(&scale.to_le_bytes());
    }
    list(b"LIST", b"lart", &[chunk(b"art1", &a)])
}

#[cfg(test)]
fn wsmp(root: u8, looped: Option<(u32, u32)>) -> Vec<u8> {
    let mut w = Vec::new();
    w.extend_from_slice(&20u32.to_le_bytes());
    w.extend_from_slice(&u16::from(root).to_le_bytes());
    w.extend_from_slice(&0i16.to_le_bytes());
    w.extend_from_slice(&0i32.to_le_bytes());
    w.extend_from_slice(&0u32.to_le_bytes());
    w.extend_from_slice(&u32::from(looped.is_some()).to_le_bytes());
    if let Some((s, e)) = looped {
        for x in [16, 0, s, e - s] {
            w.extend_from_slice(&x.to_le_bytes());
        }
    }
    chunk(b"wsmp", &w)
}

/// A DLS region: its keys, the sample's number, and its own connections.
#[cfg(test)]
pub type DlsRegion = ((u8, u8), u32, Vec<Connection>);

/// A DLS instrument: bank, program, drums, name, regions, and its own
/// connections.
#[cfg(test)]
pub type DlsInstrument<'a> = (u8, u8, bool, &'a str, Vec<DlsRegion>, Vec<Connection>);

/// A DLS level 1 file.
#[cfg(test)]
pub fn dls(instruments: &[DlsInstrument], samples: &[Sample]) -> Vec<u8> {
    let mut lins = Vec::new();
    for (bank, program, drums, name, regions, art) in instruments {
        let mut insh = Vec::new();
        insh.extend_from_slice(&(regions.len() as u32).to_le_bytes());
        insh.extend_from_slice(&(u32::from(*bank) << 8 | if *drums { 0x8000_0000 } else { 0 }).to_le_bytes());
        insh.extend_from_slice(&u32::from(*program).to_le_bytes());
        let mut rgns = Vec::new();
        for ((lo, hi), sample, conns) in regions {
            let mut rgnh = Vec::new();
            for x in [u16::from(*lo), u16::from(*hi), 0, 127, 0, 0] {
                rgnh.extend_from_slice(&x.to_le_bytes());
            }
            let mut wlnk = Vec::new();
            for x in [0u16, 0] {
                wlnk.extend_from_slice(&x.to_le_bytes());
            }
            for x in [1u32, *sample] {
                wlnk.extend_from_slice(&x.to_le_bytes());
            }
            let mut parts = vec![chunk(b"rgnh", &rgnh), chunk(b"wlnk", &wlnk)];
            if !conns.is_empty() {
                parts.push(art1(conns));
            }
            rgns.push(list(b"LIST", b"rgn ", &parts));
        }
        let mut name0 = name.as_bytes().to_vec();
        name0.push(0);
        let mut parts = vec![chunk(b"insh", &insh), list(b"LIST", b"lrgn", &rgns)];
        if !art.is_empty() {
            parts.push(art1(art));
        }
        parts.push(list(b"LIST", b"INFO", &[chunk(b"INAM", &name0)]));
        lins.push(list(b"LIST", b"ins ", &parts));
    }
    let mut waves = Vec::new();
    let mut offsets = Vec::new();
    let mut at = 0u32;
    for s in samples {
        let mut fmt = Vec::new();
        for x in [1u16, 1] {
            fmt.extend_from_slice(&x.to_le_bytes());
        }
        for x in [s.rate, s.rate * 2] {
            fmt.extend_from_slice(&x.to_le_bytes());
        }
        for x in [2u16, 16] {
            fmt.extend_from_slice(&x.to_le_bytes());
        }
        let data: Vec<u8> = s.data.iter().flat_map(|x| x.to_le_bytes()).collect();
        let wave = list(b"LIST", b"wave", &[chunk(b"fmt ", &fmt), wsmp(s.root, s.looped), chunk(b"data", &data)]);
        offsets.push(at);
        at += wave.len() as u32;
        waves.push(wave);
    }
    let mut ptbl = Vec::new();
    ptbl.extend_from_slice(&8u32.to_le_bytes());
    ptbl.extend_from_slice(&(offsets.len() as u32).to_le_bytes());
    for o in offsets {
        ptbl.extend_from_slice(&o.to_le_bytes());
    }
    let mut colh = Vec::new();
    colh.extend_from_slice(&(instruments.len() as u32).to_le_bytes());
    list(
        b"RIFF",
        b"DLS ",
        &[chunk(b"colh", &colh), list(b"LIST", b"lins", &lins), chunk(b"ptbl", &ptbl), list(b"LIST", b"wvpl", &waves), list(b"LIST", b"INFO", &[chunk(b"INAM", b"Test DLS\0")])],
    )
}
