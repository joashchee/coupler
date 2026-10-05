//! Opus and WavPack, both ways: played (decoded to samples for a WAV the
//! WebView plays) and made (an imported WAV offered smaller, `assets.rs`).
//! Pure: bytes in, bytes out, unit-tested.
//!
//! - **Opus** in an Ogg file (`.opus`, RFC 7845) through libopus
//!   (`opusic-sys`, BSD-3-Clause), the pages by the `ogg` crate. Lossy,
//!   and the smallest by far: Coupler makes it at 48 kbps a channel
//!   (96 stereo), where sound effects and music lose nothing a player
//!   hears. Opus always runs at 48 kHz, so other rates are resampled
//!   (`rubato`, MIT) and the original rate noted in the header. Made from
//!   one or two channels; played with up to 255.
//! - **WavPack** (`.wv`) through libwavpack (`wavpack-sys`, BSD-3-Clause),
//!   lossless: exactly the WAV's samples, typically half its size or
//!   better. `thorough` is its smallest, `wavpack -hhx6`, about 9% smaller
//!   than plain `-hh` and some 80 times slower (23 seconds a stereo minute
//!   in a release build, against 0.3; Opus takes 0.5).
//!
//! The encoder setup follows Crunchy's (`src-tauri/src/audio/`), the
//! sister app that compresses files for a living.

use std::io::Cursor;
use std::os::raw::{c_int, c_void};

use ogg::reading::PacketReader;
use ogg::writing::{PacketWriteEndInfo, PacketWriter};

/// A WAV file's samples, interleaved.
#[derive(Debug, Clone, PartialEq)]
pub struct Pcm {
    pub channels: u16,
    pub rate: u32,
    pub samples: Samples,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Samples {
    /// Whole numbers as the WAV holds them: `bytes` a sample, scaled to
    /// fill them (a 20-bit sample in 3 bytes is shifted up 4), `bits` of
    /// them meaningful. 8-bit is made signed.
    Int { bits: u16, bytes: u16, data: Vec<i32> },
    /// 32-bit float, about -1 to 1. `exact` unless the WAV was 64-bit.
    Float { exact: bool, data: Vec<f32> },
}

impl Pcm {
    pub fn frames(&self) -> usize {
        let n = match &self.samples {
            Samples::Int { data, .. } => data.len(),
            Samples::Float { data, .. } => data.len(),
        };
        n / usize::from(self.channels.max(1))
    }

    /// Every sample as a float, about -1 to 1.
    pub fn to_f32(&self) -> Vec<f32> {
        match &self.samples {
            Samples::Int { bytes, data, .. } => {
                let scale = 1.0 / (1u64 << (bytes * 8 - 1)) as f32;
                data.iter().map(|&s| s as f32 * scale).collect()
            }
            Samples::Float { data, .. } => data.clone(),
        }
    }
}

/// Reads a WAV file: PCM (8, 16, 24 or 32-bit), float (32 or 64-bit),
/// plain or WAVE_FORMAT_EXTENSIBLE. Anything else (ADPCM, A-law...) is
/// refused, with why.
pub fn read_wav(bytes: &[u8]) -> Result<Pcm, String> {
    if bytes.len() < 12 || &bytes[0..4] != b"RIFF" || &bytes[8..12] != b"WAVE" {
        return Err("it isn't a WAV file".into());
    }
    let u16_at = |b: &[u8], i: usize| u16::from_le_bytes([b[i], b[i + 1]]);
    let u32_at = |b: &[u8], i: usize| u32::from_le_bytes([b[i], b[i + 1], b[i + 2], b[i + 3]]);
    let (mut format, mut data) = (None, None);
    let mut at = 12;
    while at + 8 <= bytes.len() {
        let id = &bytes[at..at + 4];
        let len = u32_at(bytes, at + 4) as usize;
        let body = &bytes[at + 8..(at + 8).saturating_add(len).min(bytes.len())];
        match id {
            b"fmt " if body.len() >= 16 => format = Some(body),
            b"data" => data = Some(body),
            _ => {}
        }
        // Chunks are padded to an even length.
        at = at.saturating_add(8).saturating_add(len).saturating_add(len & 1);
    }
    let (fmt, data) = (format.ok_or("it has no format chunk")?, data.ok_or("it has no sound in it")?);
    let mut tag = u16_at(fmt, 0);
    let channels = u16_at(fmt, 2);
    let rate = u32_at(fmt, 4);
    let container = u16_at(fmt, 14);
    let mut bits = container;
    if tag == 0xFFFE && fmt.len() >= 26 {
        let valid = u16_at(fmt, 18);
        if valid > 0 && valid <= container {
            bits = valid;
        }
        tag = u16_at(fmt, 24);
    }
    if channels == 0 || rate == 0 {
        return Err("its format says no channels or no rate".into());
    }
    let samples = match (tag, container) {
        (1, 8) => Samples::Int { bits, bytes: 1, data: data.iter().map(|&b| i32::from(b) - 128).collect() },
        (1, 16) => Samples::Int { bits, bytes: 2, data: data.as_chunks::<2>().0.iter().map(|c| i32::from(i16::from_le_bytes(*c))).collect() },
        (1, 24) => Samples::Int { bits, bytes: 3, data: data.as_chunks::<3>().0.iter().map(|c| i32::from_le_bytes([0, c[0], c[1], c[2]]) >> 8).collect() },
        (1, 32) => Samples::Int { bits, bytes: 4, data: data.as_chunks::<4>().0.iter().map(|c| i32::from_le_bytes(*c)).collect() },
        (3, 32) => Samples::Float { exact: true, data: data.as_chunks::<4>().0.iter().map(|c| f32::from_le_bytes(*c)).collect() },
        (3, 64) => Samples::Float { exact: false, data: data.as_chunks::<8>().0.iter().map(|c| f64::from_le_bytes(*c) as f32).collect() },
        _ => return Err(format!("its kind of WAV (format {tag}, {container}-bit) isn't one Coupler can read")),
    };
    let mut pcm = Pcm { channels, rate, samples };
    // A last frame cut short is dropped.
    let whole = pcm.frames() * usize::from(channels);
    match &mut pcm.samples {
        Samples::Int { data, .. } => data.truncate(whole),
        Samples::Float { data, .. } => data.truncate(whole),
    }
    Ok(pcm)
}

/// Float samples, about -1 to 1, as 16-bit for `assets::wav`.
pub fn to_i16(samples: &[f32]) -> Vec<i16> {
    samples.iter().map(|s| (s.clamp(-1.0, 1.0) * 32767.0).round() as i16).collect()
}

// ---- Opus ----

/// Opus's one rate, and its usual 20 ms frame there.
const OPUS_RATE: u32 = 48_000;
const OPUS_FRAME: usize = 960;
/// Bits a second for each channel.
const OPUS_BITRATE_PER_CHANNEL: i32 = 48_000;

struct OpusEncoder(*mut opusic_sys::OpusEncoder);

impl Drop for OpusEncoder {
    fn drop(&mut self) {
        // SAFETY: destroying the encoder we created, once.
        unsafe { opusic_sys::opus_encoder_destroy(self.0) };
    }
}

struct OpusDecoder(*mut opusic_sys::OpusMSDecoder);

impl Drop for OpusDecoder {
    fn drop(&mut self) {
        // SAFETY: destroying the decoder we created, once.
        unsafe { opusic_sys::opus_multistream_decoder_destroy(self.0) };
    }
}

/// `pcm` as an Ogg Opus file. One or two channels only. `tell` hears how
/// far along it is, 0 to 1.
pub fn encode_opus(pcm: &Pcm, tell: &mut dyn FnMut(f32)) -> Result<Vec<u8>, String> {
    use opusic_sys::*;
    let channels = usize::from(pcm.channels);
    if !(1..=2).contains(&channels) {
        return Err("Opus is made from one or two channels".into());
    }
    let samples = resample(&pcm.to_f32(), channels, pcm.rate, OPUS_RATE)?;
    let frames = samples.len() / channels;
    let mut err = 0;
    // SAFETY: the encoder lives in `enc` until it drops; each ctl gets the
    // argument type its request takes (opus_int32, or a pointer to one).
    let (enc, lookahead) = unsafe {
        let st = opus_encoder_create(OPUS_RATE as i32, channels as c_int, OPUS_APPLICATION_AUDIO, &mut err);
        if st.is_null() || err != OPUS_OK {
            return Err("the Opus encoder couldn't start".into());
        }
        let enc = OpusEncoder(st);
        let ok = opus_encoder_ctl(enc.0, OPUS_SET_BITRATE_REQUEST, OPUS_BITRATE_PER_CHANNEL * channels as i32) == OPUS_OK
            && opus_encoder_ctl(enc.0, OPUS_SET_VBR_REQUEST, 1 as opus_int32) == OPUS_OK
            && opus_encoder_ctl(enc.0, OPUS_SET_COMPLEXITY_REQUEST, 10 as opus_int32) == OPUS_OK;
        let mut lookahead: opus_int32 = 0;
        if !ok || opus_encoder_ctl(enc.0, OPUS_GET_LOOKAHEAD_REQUEST, &mut lookahead as *mut opus_int32) != OPUS_OK {
            return Err("the Opus encoder couldn't be set up".into());
        }
        (enc, lookahead.max(0) as usize)
    };
    let mut out = Vec::new();
    let mut ogg = PacketWriter::new(&mut out);
    let serial = serial_number();
    let write = |e: std::io::Error| format!("the Opus file couldn't be written ({e})");
    ogg.write_packet(opus_head(pcm.channels as u8, lookahead as u16, pcm.rate), serial, PacketWriteEndInfo::EndPage, 0).map_err(write)?;
    ogg.write_packet(opus_tags(), serial, PacketWriteEndInfo::EndPage, 0).map_err(write)?;
    // The stream is the sound plus the lookahead the decoder skips; the
    // last frame is padded with silence and the last granule trims it.
    let end = (frames + lookahead) as u64;
    let packets = (frames + lookahead).div_ceil(OPUS_FRAME).max(1);
    let mut frame = vec![0f32; OPUS_FRAME * channels];
    let mut packet = vec![0u8; 4000];
    for n in 0..packets {
        let from = (n * OPUS_FRAME * channels).min(samples.len());
        let to = ((n + 1) * OPUS_FRAME * channels).min(samples.len());
        frame.fill(0.0);
        frame[..to - from].copy_from_slice(&samples[from..to]);
        // SAFETY: `frame` holds OPUS_FRAME frames of `channels` samples, and
        // `packet` is bigger than any one Opus packet.
        let len = unsafe { opus_encode_float(enc.0, frame.as_ptr(), OPUS_FRAME as c_int, packet.as_mut_ptr(), packet.len() as i32) };
        if len < 0 {
            return Err("the Opus encoder failed".into());
        }
        let last = n + 1 == packets;
        let granule = (((n + 1) * OPUS_FRAME) as u64).min(end);
        let info = if last { PacketWriteEndInfo::EndStream } else { PacketWriteEndInfo::NormalPacket };
        ogg.write_packet(packet[..len as usize].to_vec(), serial, info, granule).map_err(write)?;
        if n % 50 == 0 {
            tell(n as f32 / packets as f32);
        }
    }
    drop(ogg);
    tell(1.0);
    Ok(out)
}

/// An Ogg Opus file's sound: its channels, 48 kHz, and the samples,
/// interleaved, at most `most_seconds` of them.
pub fn decode_opus(bytes: &[u8], most_seconds: usize) -> Result<(u16, u32, Vec<f32>), String> {
    use opusic_sys::*;
    let mut reader = PacketReader::new(Cursor::new(bytes));
    let mut next = || reader.read_packet().map_err(|e| format!("its Ogg pages are damaged ({e})"));
    let head = next()?.ok_or("it's empty")?;
    let h = &head.data;
    if h.len() < 19 || &h[0..8] != b"OpusHead" {
        return Err("it isn't an Opus file".into());
    }
    let channels = h[9];
    let pre_skip = usize::from(u16::from_le_bytes([h[10], h[11]]));
    let gain_db = f32::from(i16::from_le_bytes([h[16], h[17]])) / 256.0;
    let (streams, coupled, mapping) = match h[18] {
        0 if (1..=2).contains(&channels) => (1, channels - 1, (0..channels).collect::<Vec<u8>>()),
        _ if h.len() >= 21 + usize::from(channels) && channels > 0 => (h[19], h[20], h[21..21 + usize::from(channels)].to_vec()),
        _ => return Err("its channel layout isn't one Opus allows".into()),
    };
    let mut err = 0;
    // SAFETY: `mapping` has an entry for each channel; the decoder lives
    // in `dec` until it drops.
    let dec = unsafe {
        let st = opus_multistream_decoder_create(OPUS_RATE as i32, c_int::from(channels), c_int::from(streams), c_int::from(coupled), mapping.as_ptr(), &mut err);
        if st.is_null() || err != OPUS_OK {
            return Err("its channel layout isn't one Opus allows".into());
        }
        OpusDecoder(st)
    };
    // The comment header.
    next()?.ok_or("it has no sound in it")?;
    let ch = usize::from(channels);
    let most = (most_seconds.saturating_mul(OPUS_RATE as usize) + pre_skip).saturating_mul(ch);
    // 120 ms, the longest an Opus packet holds.
    let mut pcm = vec![0f32; 5760 * ch];
    let (mut samples, mut granule) = (Vec::new(), None);
    while let Some(packet) = next()? {
        // SAFETY: `pcm` has room for 5760 frames of every channel.
        let n = unsafe { opus_multistream_decode_float(dec.0, packet.data.as_ptr(), packet.data.len() as i32, pcm.as_mut_ptr(), 5760, 0) };
        if n < 0 {
            return Err("its sound is damaged".into());
        }
        samples.extend_from_slice(&pcm[..n as usize * ch]);
        if packet.last_in_stream() {
            granule = Some(packet.absgp_page() as usize);
        }
        if samples.len() >= most || packet.last_in_stream() {
            break;
        }
    }
    // The last granule says where the sound ends (the rest is padding);
    // the pre-skip, where it starts.
    if let Some(end) = granule {
        samples.truncate(end.saturating_mul(ch).min(samples.len()));
    }
    samples.truncate(most);
    let mut samples = samples.split_off((pre_skip * ch).min(samples.len()));
    if gain_db != 0.0 {
        let gain = 10f32.powf(gain_db / 20.0);
        samples.iter_mut().for_each(|s| *s *= gain);
    }
    Ok((u16::from(channels), OPUS_RATE, samples))
}

/// Any 32-bit serial number will do; this one differs per file.
fn serial_number() -> u32 {
    let nanos = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map_or(0, |d| d.as_nanos());
    (nanos as u32) ^ ((nanos >> 32) as u32)
}

/// The identification header (RFC 7845 section 5.1), mapping family 0.
fn opus_head(channels: u8, pre_skip: u16, input_rate: u32) -> Vec<u8> {
    let mut h = Vec::with_capacity(19);
    h.extend_from_slice(b"OpusHead");
    h.push(1);
    h.push(channels);
    h.extend_from_slice(&pre_skip.to_le_bytes());
    h.extend_from_slice(&input_rate.to_le_bytes());
    h.extend_from_slice(&0i16.to_le_bytes());
    h.push(0);
    h
}

/// The comment header (RFC 7845 section 5.2): libopus's version, no tags.
fn opus_tags() -> Vec<u8> {
    // SAFETY: libopus returns a static NUL-terminated string.
    let vendor = unsafe { std::ffi::CStr::from_ptr(opusic_sys::opus_get_version_string()) }.to_bytes().to_vec();
    let mut t = Vec::new();
    t.extend_from_slice(b"OpusTags");
    t.extend_from_slice(&(vendor.len() as u32).to_le_bytes());
    t.extend_from_slice(&vendor);
    t.extend_from_slice(&0u32.to_le_bytes());
    t
}

/// Interleaved samples from `from` Hz to `to` Hz, the same length in time.
fn resample(samples: &[f32], channels: usize, from: u32, to: u32) -> Result<Vec<f32>, String> {
    use rubato::{FftFixedIn, Resampler};
    if from == to {
        return Ok(samples.to_vec());
    }
    let failed = |e: &dyn std::fmt::Display| format!("its sound couldn't be resampled for Opus ({e})");
    let mut r = FftFixedIn::<f32>::new(from as usize, to as usize, 1024, 2, channels).map_err(|e| failed(&e))?;
    let frames = samples.len() / channels;
    let wanted = (frames as u64 * u64::from(to) / u64::from(from)) as usize;
    let mut split: Vec<Vec<f32>> = (0..channels).map(|c| samples.iter().skip(c).step_by(channels).copied().collect()).collect();
    // Silence after the sound flushes out the resampler's delay.
    let delay = r.output_delay();
    let mut out: Vec<Vec<f32>> = vec![Vec::with_capacity(wanted + delay); channels];
    let mut at = 0;
    while out[0].len() < wanted + delay {
        let need = r.input_frames_next();
        if at + need > frames {
            split.iter_mut().for_each(|c| c.resize(at + need, 0.0));
        }
        let chunk: Vec<&[f32]> = split.iter().map(|c| &c[at..at + need]).collect();
        let done = r.process(&chunk, None).map_err(|e| failed(&e))?;
        out.iter_mut().zip(done).for_each(|(o, d)| o.extend(d));
        at += need;
    }
    let mut interleaved = Vec::with_capacity(wanted * channels);
    for i in delay..delay + wanted {
        interleaved.extend(out.iter().map(|c| c[i]));
    }
    Ok(interleaved)
}

// ---- WavPack ----

use wavpack_sys::*;

struct Context(*mut WavpackContext);

impl Drop for Context {
    fn drop(&mut self) {
        // SAFETY: closing the context we opened, once.
        unsafe { WavpackCloseFile(self.0) };
    }
}

unsafe extern "C" fn block_out(id: *mut c_void, data: *mut c_void, bcount: i32) -> c_int {
    // SAFETY: `id` is the Vec passed to WavpackOpenFileOutput, alive for
    // the whole encode; `data` holds `bcount` bytes.
    let out = unsafe { &mut *(id as *mut Vec<u8>) };
    out.extend_from_slice(unsafe { std::slice::from_raw_parts(data as *const u8, bcount.max(0) as usize) });
    1
}

/// Whether WavPack can hold `pcm` exactly.
pub fn wavpack_exact(pcm: &Pcm) -> bool {
    !matches!(pcm.samples, Samples::Float { exact: false, .. })
}

/// `pcm` as a WavPack file, lossless: `-hhx6` if `thorough`, else `-hh`.
/// `tell` hears how far along it is.
pub fn encode_wavpack(pcm: &Pcm, thorough: bool, tell: &mut dyn FnMut(f32)) -> Result<Vec<u8>, String> {
    if !wavpack_exact(pcm) {
        return Err("WavPack can't hold 64-bit samples exactly".into());
    }
    let (bits, bytes, data): (u16, u16, Vec<i32>) = match &pcm.samples {
        Samples::Int { bits, bytes, data } => (*bits, *bytes, data.clone()),
        Samples::Float { data, .. } => (32, 4, data.iter().map(|f| f.to_bits() as i32).collect()),
    };
    let channels = usize::from(pcm.channels);
    let mut out: Box<Vec<u8>> = Box::default();
    // SAFETY: the context and `out` outlive every call below (`out` is
    // boxed so its address is stable); buffers hold whole frames.
    unsafe {
        let ctx = Context(WavpackOpenFileOutput(Some(block_out), &mut *out as *mut Vec<u8> as *mut c_void, std::ptr::null_mut()));
        if ctx.0.is_null() {
            return Err("the WavPack encoder couldn't start".into());
        }
        let mut config: WavpackConfig = std::mem::zeroed();
        config.bits_per_sample = c_int::from(bits);
        config.bytes_per_sample = c_int::from(bytes);
        config.num_channels = c_int::from(pcm.channels);
        config.sample_rate = pcm.rate as i32;
        config.channel_mask = match pcm.channels {
            1 => 0x4,
            2 => 0x3,
            _ => 0,
        };
        if thorough {
            config.flags = (CONFIG_VERY_HIGH_FLAG | CONFIG_EXTRA_MODE) as c_int;
            config.xmode = 6;
        } else {
            config.flags = CONFIG_VERY_HIGH_FLAG as c_int;
        }
        if matches!(pcm.samples, Samples::Float { .. }) {
            config.float_norm_exp = 127;
        }
        if WavpackSetConfiguration64(ctx.0, &mut config, pcm.frames() as i64, std::ptr::null()) == 0 || WavpackPackInit(ctx.0) == 0 {
            return Err("WavPack can't hold this kind of WAV".into());
        }
        // A second at a time, so `tell` moves.
        let step = pcm.rate as usize * channels;
        let mut chunk = Vec::with_capacity(step);
        for (n, part) in data.chunks(step).enumerate() {
            chunk.clear();
            chunk.extend_from_slice(part);
            if WavpackPackSamples(ctx.0, chunk.as_mut_ptr(), (part.len() / channels) as u32) == 0 {
                return Err("the WavPack encoder failed".into());
            }
            tell((n * step) as f32 / data.len().max(1) as f32);
        }
        if WavpackFlushSamples(ctx.0) == 0 {
            return Err("the WavPack encoder couldn't finish the file".into());
        }
    }
    tell(1.0);
    Ok(*out)
}

/// A WavPack file being read from memory.
struct Source<'a> {
    bytes: &'a [u8],
    at: usize,
}

unsafe fn source<'a>(id: *mut c_void) -> &'a mut Source<'a> {
    // SAFETY (callers): `id` is the Source handed to WavpackOpenFileInputEx64,
    // alive until the context is closed.
    unsafe { &mut *(id as *mut Source) }
}

unsafe extern "C" fn read_bytes(id: *mut c_void, data: *mut c_void, bcount: i32) -> i32 {
    let s = unsafe { source(id) };
    let n = (bcount.max(0) as usize).min(s.bytes.len().saturating_sub(s.at));
    // SAFETY: `data` has room for `bcount` bytes, and n is no more.
    unsafe { std::ptr::copy_nonoverlapping(s.bytes[s.at..].as_ptr(), data as *mut u8, n) };
    s.at += n;
    n as i32
}

unsafe extern "C" fn write_bytes(_: *mut c_void, _: *mut c_void, _: i32) -> i32 {
    0
}

unsafe extern "C" fn get_pos(id: *mut c_void) -> i64 {
    unsafe { source(id) }.at as i64
}

unsafe extern "C" fn set_pos_abs(id: *mut c_void, pos: i64) -> c_int {
    let s = unsafe { source(id) };
    s.at = (pos.max(0) as usize).min(s.bytes.len());
    0
}

unsafe extern "C" fn set_pos_rel(id: *mut c_void, delta: i64, mode: c_int) -> c_int {
    let s = unsafe { source(id) };
    // SEEK_SET, SEEK_CUR, SEEK_END.
    let base = match mode {
        0 => 0,
        1 => s.at as i64,
        2 => s.bytes.len() as i64,
        _ => return -1,
    };
    s.at = ((base + delta).max(0) as usize).min(s.bytes.len());
    0
}

unsafe extern "C" fn push_back_byte(id: *mut c_void, c: c_int) -> c_int {
    let s = unsafe { source(id) };
    s.at = s.at.saturating_sub(1);
    c
}

unsafe extern "C" fn get_length(id: *mut c_void) -> i64 {
    unsafe { source(id) }.bytes.len() as i64
}

unsafe extern "C" fn can_seek(_: *mut c_void) -> c_int {
    1
}

unsafe extern "C" fn truncate_here(_: *mut c_void) -> c_int {
    -1
}

unsafe extern "C" fn close(_: *mut c_void) -> c_int {
    0
}

/// A WavPack file's sound: its channels, rate, and the samples (about -1
/// to 1), interleaved, at most `most_seconds` of them.
pub fn decode_wavpack(bytes: &[u8], most_seconds: usize) -> Result<(u16, u32, Vec<f32>), String> {
    let mut reader = WavpackStreamReader64 {
        read_bytes: Some(read_bytes),
        write_bytes: Some(write_bytes),
        get_pos: Some(get_pos),
        set_pos_abs: Some(set_pos_abs),
        set_pos_rel: Some(set_pos_rel),
        push_back_byte: Some(push_back_byte),
        get_length: Some(get_length),
        can_seek: Some(can_seek),
        truncate_here: Some(truncate_here),
        close: Some(close),
    };
    let mut from = Source { bytes, at: 0 };
    let mut error = [0 as std::os::raw::c_char; 81];
    // SAFETY: `reader` and `from` outlive the context, which is closed when
    // `ctx` drops at the end of this function; `error` has the 80
    // characters and NUL the library may write.
    unsafe {
        let ctx = WavpackOpenFileInputEx64(&mut reader, &mut from as *mut Source as *mut c_void, std::ptr::null_mut(), error.as_mut_ptr(), OPEN_NORMALIZE as c_int, 0);
        if ctx.is_null() {
            let why = std::ffi::CStr::from_ptr(error.as_ptr()).to_string_lossy().into_owned();
            return Err(if why.is_empty() { "it isn't a WavPack file".into() } else { format!("it couldn't be read ({why})") });
        }
        let ctx = Context(ctx);
        let channels = WavpackGetNumChannels(ctx.0).clamp(1, 255) as usize;
        let rate = WavpackGetSampleRate(ctx.0);
        let float = WavpackGetMode(ctx.0) & MODE_FLOAT as c_int != 0;
        let bytes_per = WavpackGetBytesPerSample(ctx.0).clamp(1, 4) as u32;
        let scale = 1.0 / (1u64 << (bytes_per * 8 - 1)) as f32;
        let most_frames = most_seconds.saturating_mul(rate as usize);
        let mut samples = Vec::new();
        let mut buf = vec![0i32; 4096 * channels];
        while samples.len() < most_frames.saturating_mul(channels) {
            let n = WavpackUnpackSamples(ctx.0, buf.as_mut_ptr(), 4096) as usize;
            if n == 0 {
                break;
            }
            let got = &buf[..n * channels];
            if float {
                samples.extend(got.iter().map(|&s| f32::from_bits(s as u32)));
            } else {
                samples.extend(got.iter().map(|&s| s as f32 * scale));
            }
        }
        samples.truncate(most_frames.saturating_mul(channels));
        if WavpackGetNumErrors(ctx.0) > 0 {
            return Err("its sound is damaged".into());
        }
        Ok((channels as u16, rate, samples))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A WAV file: the header for `tag` (1 PCM, 3 float) and `bits`, then `data`.
    fn wav_file(tag: u16, channels: u16, rate: u32, bits: u16, data: &[u8]) -> Vec<u8> {
        let mut f = Vec::new();
        f.extend_from_slice(b"RIFF");
        f.extend_from_slice(&(36 + data.len() as u32).to_le_bytes());
        f.extend_from_slice(b"WAVEfmt ");
        f.extend_from_slice(&16u32.to_le_bytes());
        f.extend_from_slice(&tag.to_le_bytes());
        f.extend_from_slice(&channels.to_le_bytes());
        f.extend_from_slice(&rate.to_le_bytes());
        f.extend_from_slice(&(rate * u32::from(channels * bits / 8)).to_le_bytes());
        f.extend_from_slice(&(channels * bits / 8).to_le_bytes());
        f.extend_from_slice(&bits.to_le_bytes());
        // A chunk Coupler skips, of odd length (so padded).
        f.extend_from_slice(b"LIST");
        f.extend_from_slice(&3u32.to_le_bytes());
        f.extend_from_slice(b"abc\0");
        f.extend_from_slice(b"data");
        f.extend_from_slice(&(data.len() as u32).to_le_bytes());
        f.extend_from_slice(data);
        f
    }

    /// A second of a 440 Hz tone, stereo 16-bit at 44.1 kHz, a little
    /// different in each ear.
    fn tone() -> Pcm {
        let data = (0..44_100)
            .flat_map(|i| {
                let t = i as f32 / 44_100.0;
                let s = (t * 440.0 * std::f32::consts::TAU).sin() * 0.5;
                [(s * 32767.0) as i32, (s * 0.8 * 32767.0) as i32]
            })
            .collect();
        Pcm { channels: 2, rate: 44_100, samples: Samples::Int { bits: 16, bytes: 2, data } }
    }

    #[test]
    fn wav_files_are_read_in_each_sample_kind() {
        let pcm = read_wav(&wav_file(1, 1, 8000, 16, &[0x00, 0x80, 0xff, 0x7f, 0x01])).unwrap();
        assert_eq!((pcm.channels, pcm.rate), (1, 8000));
        // The half sample at the end is dropped.
        assert_eq!(pcm.samples, Samples::Int { bits: 16, bytes: 2, data: vec![-32768, 32767] });
        let eight = read_wav(&wav_file(1, 1, 8000, 8, &[0, 128, 255])).unwrap();
        assert_eq!(eight.samples, Samples::Int { bits: 8, bytes: 1, data: vec![-128, 0, 127] });
        let deep = read_wav(&wav_file(1, 1, 8000, 24, &[0x00, 0x00, 0x80, 0xff, 0xff, 0x7f])).unwrap();
        assert_eq!(deep.samples, Samples::Int { bits: 24, bytes: 3, data: vec![-8_388_608, 8_388_607] });
        let float = read_wav(&wav_file(3, 1, 8000, 32, &0.25f32.to_le_bytes())).unwrap();
        assert_eq!(float.samples, Samples::Float { exact: true, data: vec![0.25] });
        assert!(read_wav(&wav_file(2, 1, 8000, 4, &[0; 8])).unwrap_err().contains("format 2"));
        assert!(read_wav(b"OggS not a wav").is_err());
    }

    #[test]
    fn wavpack_gives_back_exactly_what_it_was_given() {
        let pcm = tone();
        let packed = encode_wavpack(&pcm, true, &mut |_| ()).unwrap();
        assert_eq!(&packed[0..4], b"wvpk");
        let raw = pcm.frames() * 4;
        assert!(packed.len() < raw / 2, "{} of {raw}", packed.len());
        let (channels, rate, back) = decode_wavpack(&packed, 60).unwrap();
        assert_eq!((channels, rate), (2, 44_100));
        assert_eq!(back, pcm.to_f32());
        // Cut short where asked.
        let cut = Pcm { rate: 10, ..pcm.clone() };
        assert_eq!(decode_wavpack(&encode_wavpack(&cut, false, &mut |_| ()).unwrap(), 2).unwrap().2.len(), 2 * 10 * 2);
        // And 24-bit and float too.
        for samples in [Samples::Int { bits: 24, bytes: 3, data: vec![-8_388_608, 8_388_607, 5, -5] }, Samples::Float { exact: true, data: vec![0.5, -0.25, 1.0, 0.0] }] {
            let small = Pcm { channels: 1, rate: 8000, samples };
            let back = decode_wavpack(&encode_wavpack(&small, false, &mut |_| ()).unwrap(), 1).unwrap().2;
            assert_eq!(back, small.to_f32());
        }
        assert!(decode_wavpack(b"not wavpack at all", 10).is_err());
    }

    #[test]
    fn opus_is_far_smaller_and_sounds_the_same_length() {
        let pcm = tone();
        let mut told = Vec::new();
        let packed = encode_opus(&pcm, &mut |f| told.push(f)).unwrap();
        assert_eq!(told.last(), Some(&1.0));
        assert_eq!(&packed[0..4], b"OggS");
        // Around 96 kbps for a second, against the WAV's 1411.
        assert!(packed.len() < 20_000, "{}", packed.len());
        let (channels, rate, back) = decode_opus(&packed, 60).unwrap();
        assert_eq!((channels, rate), (2, 48_000));
        // A second at 48 kHz, give or take the resampler's rounding.
        assert!((back.len() as i64 / 2 - 48_000).abs() <= 2, "{}", back.len() / 2);
        // Still the tone: as loud, the left ear louder than the right.
        let rms = |c: usize| (back.iter().skip(c).step_by(2).map(|s| s * s).sum::<f32>() / 48_000.0).sqrt();
        assert!((rms(0) - 0.5 / 2f32.sqrt()).abs() < 0.05, "{}", rms(0));
        assert!(rms(1) < rms(0));
        assert!(decode_opus(b"not ogg", 10).is_err());
    }

    #[test]
    fn opus_is_made_from_one_or_two_channels_only() {
        let six = Pcm { channels: 6, rate: 48_000, samples: Samples::Int { bits: 16, bytes: 2, data: vec![0; 60] } };
        assert!(encode_opus(&six, &mut |_| ()).is_err());
        let mono = Pcm { channels: 1, rate: 48_000, samples: Samples::Int { bits: 16, bytes: 2, data: vec![0; 10] } };
        let (channels, _, back) = decode_opus(&encode_opus(&mono, &mut |_| ()).unwrap(), 1).unwrap();
        assert_eq!((channels, back.len()), (1, 10));
    }
}
