//! Telnet for a MUD: splits the server's byte stream into text and
//! commands, answers option negotiation, and undoes MCCP2 compression.
//! Pure (no socket), so every rule here is unit-tested.
//!
//! What Coupler agrees to follows Sip, CoffeeMUD's own client
//! (`js/telnetparser.js`; notes in Diskette's docs/coupler-notes.md),
//! with these differences on purpose:
//! - NAWS sends 16-bit sizes and doubles any 0xFF byte (Sip sends one byte
//!   and never escapes).
//! - SGA is accepted when the server offers it (Sip refuses it by default).
//! - LINEMODE is refused: Coupler sends whole lines, and Sip's WILL was
//!   never followed by the subnegotiation it promises.
//! - MXP and MSP are refused until Coupler draws and plays them, so
//!   CoffeeMUD falls back to plain ANSI rather than sending tags Coupler
//!   would print as text. (Every protocol CoffeeMUD offers, and why
//!   each is taken or not: docs/coffeemud-gmcp.md, 6.1.)
//! - MSDP is accepted for four variables: `WORLD_TIME` (the game's hour,
//!   for `daytime.rs`; GMCP has no time) and the opponent's exact health,
//!   maximum and range (`combat.rs`; GMCP has only a percentage). Coupler
//!   asks the server to REPORT them, and the server sends each at once
//!   and again each time it changes (`CMProtocols.processMsdpResult`,
//!   `pingMsdp`). Nothing else is asked for, so nothing else comes. Not CoffeeMUD's MSDP-over-GMCP
//!   (a GMCP `MSDP` message): it answers with no package name and writes
//!   later changes into the text unframed.
//! - Each option remembers its state, so a repeated WILL/DO for an option
//!   already on gets no second answer and negotiation can't loop (a
//!   refusal is always answered; the server doesn't reply to it).
//! - NEW-ENVIRON never sends a user or account name.

use flate2::{Decompress, FlushDecompress, Status};

pub const IAC: u8 = 255;
const DONT: u8 = 254;
const DO: u8 = 253;
const WONT: u8 = 252;
const WILL: u8 = 251;
const SB: u8 = 250;
const GA: u8 = 249;
const SE: u8 = 240;
const EOR_CMD: u8 = 239;

const OPT_ECHO: u8 = 1;
const OPT_SGA: u8 = 3;
const OPT_TTYPE: u8 = 24;
const OPT_EOR: u8 = 25;
const OPT_NAWS: u8 = 31;
const OPT_NEW_ENVIRON: u8 = 39;
const OPT_MSDP: u8 = 69;
const OPT_MCCP2: u8 = 86;
const OPT_GMCP: u8 = 201;

const TTYPE_IS: u8 = 0;
const TTYPE_SEND: u8 = 1;
const ENV_IS: u8 = 0;
const ENV_SEND: u8 = 1;
const ENV_VAR: u8 = 0;
const ENV_VALUE: u8 = 1;
const ENV_USERVAR: u8 = 3;
const MSDP_VAR: u8 = 1;
const MSDP_VAL: u8 = 2;
const MSDP_TABLE_OPEN: u8 = 3;
const MSDP_TABLE_CLOSE: u8 = 4;
const MSDP_ARRAY_OPEN: u8 = 5;
const MSDP_ARRAY_CLOSE: u8 = 6;

/// The MSDP variables Coupler asks the server to report: the hour
/// (`daytime.rs`) and the opponent's exact figures (`combat.rs`, which
/// says why the other OPPONENT_ variables are left out).
const MSDP_REPORT: &[&str] = &["WORLD_TIME", "OPPONENT_HEALTH", "OPPONENT_HEALTH_MAX", "OPPONENT_RANGE"];

pub const CLIENT_NAME: &str = "Coupler";

/// MTTS bits: ANSI, UTF-8, 256 colors, truecolor (the same as Sip's).
const MTTS: u32 = 1 | 4 | 8 | 256;
const TERMINAL_TYPE: &str = "ANSI-TRUECOLOR";

/// GMCP packages Coupler tells the server it understands. Grows as the
/// UI learns to show them (Char.Vitals for gauges, Room.Info for the map).
const GMCP_SUPPORTS: &[&str] = &["Core 1", "Char 1", "Room 1", "Comm 1"];

/// Something the session should act on, in stream order.
#[derive(Debug, PartialEq)]
pub enum Event {
    /// Game text, still encoded (UTF-8 from CoffeeMUD), ANSI codes and all.
    Text(Vec<u8>),
    /// GA or EOR: the text so far is a prompt waiting for input.
    Prompt,
    /// The server echoes (true) or has stopped (false). When it does, the
    /// input is a password: hide it and keep it out of history.
    ServerEcho(bool),
    /// A GMCP message: package name and its JSON (unparsed, may be empty).
    Gmcp(String, String),
    /// An MSDP variable with a plain value: its name and the value.
    Msdp(String, String),
}

#[derive(Default, Clone, Copy, PartialEq)]
enum State {
    #[default]
    Data,
    Iac,
    /// After WILL/WONT/DO/DONT: waiting for the option byte.
    Negotiate(u8),
    /// Inside IAC SB <option> … collecting.
    Sub,
    SubIac,
}

/// One connection's telnet state. Feed it everything the socket reads;
/// send `replies` back to the server.
pub struct Telnet {
    state: State,
    sub: Vec<u8>,
    /// Options the server has agreed to do (its WILLs we accepted).
    remote_on: [bool; 256],
    /// Options Coupler has agreed to do (DOs we accepted).
    local_on: [bool; 256],
    ttype_sent: u8,
    width: u16,
    height: u16,
    version: String,
    inflate: Option<Decompress>,
}

/// What one `feed` produced.
#[derive(Default, Debug)]
pub struct Output {
    pub events: Vec<Event>,
    /// Bytes to write back to the server.
    pub replies: Vec<u8>,
}

impl Output {
    fn text(&mut self, b: u8) {
        match self.events.last_mut() {
            Some(Event::Text(t)) => t.push(b),
            _ => self.events.push(Event::Text(vec![b])),
        }
    }
}

impl Telnet {
    pub fn new(version: &str) -> Self {
        Telnet {
            state: State::Data,
            sub: Vec::new(),
            remote_on: [false; 256],
            local_on: [false; 256],
            ttype_sent: 0,
            width: 80,
            height: 24,
            version: version.to_string(),
            inflate: None,
        }
    }

    /// Everything read from the socket, compressed or not.
    pub fn feed(&mut self, input: &[u8]) -> Result<Output, String> {
        let mut out = Output::default();
        let mut rest = input.to_vec();
        loop {
            if self.inflate.is_some() {
                let (plain, after_end) = self.inflate_chunk(&rest)?;
                // Compression started and ended within this chunk: the
                // bytes after the end are plain again.
                let restart = self.process(&plain, &mut out);
                debug_assert!(restart.is_none(), "MCCP2 can't start inside a compressed stream");
                match after_end {
                    Some(tail) => rest = tail,
                    None => return Ok(out),
                }
            } else {
                match self.process(&rest, &mut out) {
                    // IAC SB MCCP2 IAC SE: everything after it is compressed.
                    Some(at) => {
                        self.inflate = Some(Decompress::new(true));
                        rest = rest[at..].to_vec();
                    }
                    None => return Ok(out),
                }
            }
        }
    }

    /// The window changed size, in character cells. Returns the NAWS
    /// update to send, if the server asked for sizes.
    pub fn resize(&mut self, width: u16, height: u16) -> Option<Vec<u8>> {
        self.width = width;
        self.height = height;
        self.local_on[OPT_NAWS as usize].then(|| self.naws())
    }

    /// A GMCP message for the server, e.g. `("Core.Ping", "")`.
    pub fn gmcp(package: &str, json: &str) -> Vec<u8> {
        let mut msg = vec![IAC, SB, OPT_GMCP];
        push_escaped(&mut msg, package.as_bytes());
        if !json.is_empty() {
            msg.push(b' ');
            push_escaped(&mut msg, json.as_bytes());
        }
        msg.extend([IAC, SE]);
        msg
    }

    /// Decompresses a chunk. Returns the plain bytes and, if the stream
    /// ended in this chunk, the (uncompressed) bytes after its end.
    fn inflate_chunk(&mut self, input: &[u8]) -> Result<(Vec<u8>, Option<Vec<u8>>), String> {
        let z = self.inflate.as_mut().expect("inflate_chunk without a stream");
        let mut plain = Vec::with_capacity(input.len() * 4);
        let mut pos = 0;
        loop {
            if plain.capacity() - plain.len() < 4096 {
                plain.reserve(16 * 1024);
            }
            let (before_in, before_out) = (z.total_in(), z.total_out());
            let status = z
                .decompress_vec(&input[pos..], &mut plain, FlushDecompress::None)
                .map_err(|e| format!("The server's compressed data was damaged ({e})."))?;
            pos += (z.total_in() - before_in) as usize;
            let stuck = z.total_in() == before_in && z.total_out() == before_out;
            match status {
                Status::StreamEnd => {
                    self.inflate = None;
                    return Ok((plain, Some(input[pos..].to_vec())));
                }
                // All input read and room left over: nothing more to come
                // until the next read. `stuck` guards against a stream
                // that can make no progress at all.
                _ if (pos >= input.len() && plain.len() < plain.capacity()) || stuck => return Ok((plain, None)),
                _ => {}
            }
        }
    }

    /// Runs plain (uncompressed) bytes through the state machine. Returns
    /// the offset just past IAC SB MCCP2 IAC SE if compression starts.
    fn process(&mut self, input: &[u8], out: &mut Output) -> Option<usize> {
        for (i, &b) in input.iter().enumerate() {
            self.state = match self.state {
                State::Data if b == IAC => State::Iac,
                State::Data => {
                    out.text(b);
                    State::Data
                }
                State::Iac => match b {
                    IAC => {
                        out.text(IAC);
                        State::Data
                    }
                    WILL | WONT | DO | DONT => State::Negotiate(b),
                    SB => {
                        self.sub.clear();
                        State::Sub
                    }
                    GA | EOR_CMD => {
                        out.events.push(Event::Prompt);
                        State::Data
                    }
                    _ => State::Data, // NOP, AYT and friends
                },
                State::Negotiate(cmd) => {
                    self.negotiate(cmd, b, out);
                    State::Data
                }
                State::Sub if b == IAC => State::SubIac,
                State::Sub => {
                    self.sub.push(b);
                    State::Sub
                }
                State::SubIac => match b {
                    SE => {
                        let sub = std::mem::take(&mut self.sub);
                        self.state = State::Data;
                        if self.subnegotiation(&sub, out) {
                            return Some(i + 1);
                        }
                        State::Data
                    }
                    IAC => {
                        self.sub.push(IAC);
                        State::Sub
                    }
                    // Malformed: an IAC that isn't IAC or SE. Drop the
                    // subnegotiation and carry on as a command.
                    _ => {
                        self.sub.clear();
                        State::Data
                    }
                },
            };
        }
        None
    }

    fn negotiate(&mut self, cmd: u8, opt: u8, out: &mut Output) {
        let o = opt as usize;
        match cmd {
            WILL => {
                let accept = matches!(opt, OPT_ECHO | OPT_SGA | OPT_EOR | OPT_GMCP | OPT_MCCP2 | OPT_MSDP);
                if accept && self.remote_on[o] {
                    return; // already agreed: no second answer
                }
                if !accept {
                    out.replies.extend([IAC, DONT, opt]);
                    return;
                }
                self.remote_on[o] = true;
                out.replies.extend([IAC, DO, opt]);
                match opt {
                    OPT_ECHO => out.events.push(Event::ServerEcho(true)),
                    OPT_GMCP => out.replies.extend(self.gmcp_hello()),
                    OPT_MSDP => out.replies.extend(msdp_report()),
                    _ => {}
                }
            }
            WONT if self.remote_on[o] => {
                self.remote_on[o] = false;
                out.replies.extend([IAC, DONT, opt]);
                if opt == OPT_ECHO {
                    out.events.push(Event::ServerEcho(false));
                }
            }
            DO => {
                let accept = matches!(opt, OPT_SGA | OPT_TTYPE | OPT_NAWS | OPT_NEW_ENVIRON);
                if accept && self.local_on[o] {
                    return;
                }
                if !accept {
                    out.replies.extend([IAC, WONT, opt]);
                    return;
                }
                self.local_on[o] = true;
                out.replies.extend([IAC, WILL, opt]);
                if opt == OPT_NAWS {
                    out.replies.extend(self.naws());
                }
            }
            DONT if self.local_on[o] => {
                self.local_on[o] = false;
                out.replies.extend([IAC, WONT, opt]);
            }
            _ => {}
        }
    }

    /// Handles IAC SB … IAC SE. Returns true when MCCP2 starts.
    fn subnegotiation(&mut self, sub: &[u8], out: &mut Output) -> bool {
        let Some((&opt, data)) = sub.split_first() else { return false };
        match opt {
            OPT_MCCP2 if self.remote_on[OPT_MCCP2 as usize] => return true,
            OPT_TTYPE if data.first() == Some(&TTYPE_SEND) => out.replies.extend(self.ttype()),
            OPT_NEW_ENVIRON if data.first() == Some(&ENV_SEND) => out.replies.extend(self.environ(&data[1..])),
            OPT_GMCP if self.remote_on[OPT_GMCP as usize] => {
                let msg = String::from_utf8_lossy(data);
                let (package, json) = msg.split_once(' ').unwrap_or((&msg, ""));
                out.events.push(Event::Gmcp(package.to_string(), json.trim().to_string()));
            }
            OPT_MSDP if self.remote_on[OPT_MSDP as usize] => {
                out.events.extend(msdp_values(data).into_iter().map(|(name, value)| Event::Msdp(name, value)));
            }
            _ => {}
        }
        false
    }

    fn naws(&self) -> Vec<u8> {
        let mut msg = vec![IAC, SB, OPT_NAWS];
        push_escaped(&mut msg, &self.width.to_be_bytes());
        push_escaped(&mut msg, &self.height.to_be_bytes());
        msg.extend([IAC, SE]);
        msg
    }

    /// MTTS: client name, then terminal type, then "MTTS <bits>", which
    /// repeats so the server knows the list has ended.
    fn ttype(&mut self) -> Vec<u8> {
        let name = match self.ttype_sent {
            0 => CLIENT_NAME.to_uppercase(),
            1 => TERMINAL_TYPE.to_string(),
            _ => format!("MTTS {MTTS}"),
        };
        self.ttype_sent = self.ttype_sent.saturating_add(1);
        let mut msg = vec![IAC, SB, OPT_TTYPE, TTYPE_IS];
        push_escaped(&mut msg, name.as_bytes());
        msg.extend([IAC, SE]);
        msg
    }

    /// Answers NEW-ENVIRON SEND with the variables asked for (all of them
    /// when none are named). Never USER or ACCT.
    fn environ(&self, asked: &[u8]) -> Vec<u8> {
        let mtts = MTTS.to_string();
        let vars: [(&str, &str); 5] = [
            ("CLIENT_NAME", CLIENT_NAME),
            ("CLIENT_VERSION", &self.version),
            ("CHARSET", "UTF-8"),
            ("MTTS", &mtts),
            ("TERMINAL_TYPE", TERMINAL_TYPE),
        ];
        let wanted: Vec<String> = asked
            .split(|&b| b == ENV_VAR || b == ENV_USERVAR)
            .filter(|n| !n.is_empty())
            .map(|n| String::from_utf8_lossy(n).to_uppercase())
            .collect();
        let mut msg = vec![IAC, SB, OPT_NEW_ENVIRON, ENV_IS];
        for (name, value) in vars {
            if wanted.is_empty() || wanted.iter().any(|w| w == name) {
                msg.push(ENV_VAR);
                push_escaped(&mut msg, name.as_bytes());
                msg.push(ENV_VALUE);
                push_escaped(&mut msg, value.as_bytes());
            }
        }
        msg.extend([IAC, SE]);
        msg
    }

    fn gmcp_hello(&self) -> Vec<u8> {
        let hello = format!(r#"{{"client":"{CLIENT_NAME}","version":"{}"}}"#, self.version);
        let supports = serde_json::to_string(GMCP_SUPPORTS).expect("static list");
        let mut msg = Self::gmcp("Core.Hello", &hello);
        msg.extend(Self::gmcp("Core.Supports.Set", &supports));
        msg
    }
}

/// Asks the server to report Coupler's MSDP variables: one REPORT with
/// an array of them. Not a REPORT each: CoffeeMUD reads a message into a
/// map by name (`CMProtocols.msdpStringify`), so only one would count.
fn msdp_report() -> Vec<u8> {
    let mut msg = vec![IAC, SB, OPT_MSDP, MSDP_VAR];
    msg.extend(b"REPORT");
    msg.extend([MSDP_VAL, MSDP_ARRAY_OPEN]);
    for name in MSDP_REPORT {
        msg.push(MSDP_VAL);
        push_escaped(&mut msg, name.as_bytes());
    }
    msg.extend([MSDP_ARRAY_CLOSE, IAC, SE]);
    msg
}

/// The variables in an MSDP message whose values are plain text. A
/// table or array value is skipped whole (Coupler asks for none).
fn msdp_values(data: &[u8]) -> Vec<(String, String)> {
    // Where the next control byte (VAR, VAL, an open or a close) is.
    let next = |from: usize| data[from..].iter().position(|&b| b <= MSDP_ARRAY_CLOSE).map_or(data.len(), |n| from + n);
    let mut found = Vec::new();
    let mut i = 0;
    while i < data.len() {
        if data[i] != MSDP_VAR {
            i += 1;
            continue;
        }
        let name_end = next(i + 1);
        let name = String::from_utf8_lossy(&data[i + 1..name_end]).into_owned();
        // Past the VAL, if there is one.
        i = (name_end + 1).min(data.len());
        if matches!(data.get(i), Some(&(MSDP_TABLE_OPEN | MSDP_ARRAY_OPEN))) {
            let mut depth = 0;
            while i < data.len() {
                match data[i] {
                    MSDP_TABLE_OPEN | MSDP_ARRAY_OPEN => depth += 1,
                    MSDP_TABLE_CLOSE | MSDP_ARRAY_CLOSE => depth -= 1,
                    _ => {}
                }
                i += 1;
                if depth == 0 {
                    break;
                }
            }
        } else {
            let end = next(i);
            found.push((name, String::from_utf8_lossy(&data[i..end]).into_owned()));
            i = end;
        }
    }
    found
}

/// Appends bytes, doubling any IAC so the server reads it as data.
fn push_escaped(msg: &mut Vec<u8>, bytes: &[u8]) {
    for &b in bytes {
        msg.push(b);
        if b == IAC {
            msg.push(IAC);
        }
    }
}

/// A line the user typed, ready for the socket: UTF-8 (which never
/// contains 0xFF, but escape anyway), ending in CR LF.
pub fn encode_line(line: &str) -> Vec<u8> {
    let mut out = Vec::with_capacity(line.len() + 2);
    push_escaped(&mut out, line.as_bytes());
    out.extend(b"\r\n");
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use flate2::{write::ZlibEncoder, Compression};
    use std::io::Write;

    fn feed(t: &mut Telnet, bytes: &[u8]) -> Output {
        t.feed(bytes).unwrap()
    }

    fn text(out: &Output) -> Vec<u8> {
        out.events
            .iter()
            .filter_map(|e| match e {
                Event::Text(t) => Some(t.clone()),
                _ => None,
            })
            .flatten()
            .collect()
    }

    #[test]
    fn plain_text_passes_through() {
        let mut t = Telnet::new("0.1.0");
        let out = feed(&mut t, b"Welcome to CoffeeMUD\r\n");
        assert_eq!(text(&out), b"Welcome to CoffeeMUD\r\n");
        assert!(out.replies.is_empty());
    }

    #[test]
    fn doubled_iac_is_data() {
        let mut t = Telnet::new("0.1.0");
        let out = feed(&mut t, &[b'a', IAC, IAC, b'b']);
        assert_eq!(text(&out), vec![b'a', IAC, b'b']);
    }

    #[test]
    fn commands_split_across_reads() {
        let mut t = Telnet::new("0.1.0");
        let a = feed(&mut t, &[b'x', IAC]);
        let b = feed(&mut t, &[WILL]);
        let c = feed(&mut t, &[OPT_ECHO, b'y']);
        assert_eq!(text(&a), b"x");
        assert!(b.replies.is_empty());
        assert_eq!(c.replies, vec![IAC, DO, OPT_ECHO]);
        assert_eq!(c.events[0], Event::ServerEcho(true));
        assert_eq!(text(&c), b"y");
    }

    #[test]
    fn echo_toggles_password_mode_once() {
        let mut t = Telnet::new("0.1.0");
        let on = feed(&mut t, &[IAC, WILL, OPT_ECHO, IAC, WILL, OPT_ECHO]);
        assert_eq!(on.events, vec![Event::ServerEcho(true)]);
        assert_eq!(on.replies, vec![IAC, DO, OPT_ECHO], "a repeated WILL gets no second answer");
        let off = feed(&mut t, &[IAC, WONT, OPT_ECHO]);
        assert_eq!(off.events, vec![Event::ServerEcho(false)]);
        assert_eq!(off.replies, vec![IAC, DONT, OPT_ECHO]);
    }

    #[test]
    fn refuses_what_coupler_cant_show_yet() {
        let mut t = Telnet::new("0.1.0");
        let out = feed(&mut t, &[IAC, WILL, 91, IAC, WILL, 90, IAC, DO, 34]);
        assert_eq!(out.replies, vec![IAC, DONT, 91, IAC, DONT, 90, IAC, WONT, 34]);
    }

    #[test]
    fn msdp_reports_its_variables_in_one_array() {
        let mut t = Telnet::new("0.1.0");
        let out = feed(&mut t, &[IAC, WILL, OPT_MSDP]);
        let mut expect = vec![IAC, DO, OPT_MSDP, IAC, SB, OPT_MSDP, MSDP_VAR];
        expect.extend(b"REPORT");
        expect.extend([MSDP_VAL, MSDP_ARRAY_OPEN]);
        for name in ["WORLD_TIME", "OPPONENT_HEALTH", "OPPONENT_HEALTH_MAX", "OPPONENT_RANGE"] {
            expect.push(MSDP_VAL);
            expect.extend(name.as_bytes());
        }
        expect.extend([MSDP_ARRAY_CLOSE, IAC, SE]);
        assert_eq!(out.replies, expect);
        // Offered again: no second answer, no second REPORT.
        assert!(feed(&mut t, &[IAC, WILL, OPT_MSDP]).replies.is_empty());

        let mut msg = vec![IAC, SB, OPT_MSDP, MSDP_VAR];
        msg.extend(b"WORLD_TIME");
        msg.push(MSDP_VAL);
        msg.extend(b"12/3/7 HR:2");
        msg.extend([IAC, SE]);
        assert_eq!(feed(&mut t, &msg).events, vec![Event::Msdp("WORLD_TIME".into(), "12/3/7 HR:2".into())]);

        // The answer to a REPORT outside a fight: the opponent's, empty.
        let mut msg = vec![IAC, SB, OPT_MSDP, MSDP_VAR];
        msg.extend(b"OPPONENT_HEALTH");
        msg.extend([MSDP_VAL, MSDP_VAR]);
        msg.extend(b"OPPONENT_RANGE");
        msg.extend([MSDP_VAL, IAC, SE]);
        assert_eq!(
            feed(&mut t, &msg).events,
            vec![Event::Msdp("OPPONENT_HEALTH".into(), String::new()), Event::Msdp("OPPONENT_RANGE".into(), String::new())]
        );
    }

    #[test]
    fn msdp_values_skip_tables_and_arrays() {
        let mut data = vec![MSDP_VAR];
        data.extend(b"ROOM");
        data.extend([MSDP_VAL, MSDP_TABLE_OPEN, MSDP_VAR, b'A', MSDP_VAL, MSDP_ARRAY_OPEN, MSDP_VAL, b'x', MSDP_ARRAY_CLOSE, MSDP_TABLE_CLOSE]);
        data.push(MSDP_VAR);
        data.extend(b"LEVEL");
        data.push(MSDP_VAL);
        data.extend(b"10");
        data.push(MSDP_VAR);
        data.extend(b"EMPTY");
        data.push(MSDP_VAL);
        assert_eq!(msdp_values(&data), vec![("LEVEL".into(), "10".into()), ("EMPTY".into(), String::new())]);
        // Cut short: no panic.
        assert_eq!(msdp_values(&[MSDP_VAR, b'X']), vec![("X".into(), String::new())]);
    }

    #[test]
    fn msdp_ignored_unless_agreed() {
        let mut t = Telnet::new("0.1.0");
        let mut msg = vec![IAC, SB, OPT_MSDP, MSDP_VAR];
        msg.extend(b"WORLD_TIME");
        msg.extend([MSDP_VAL, b'1', IAC, SE]);
        assert!(feed(&mut t, &msg).events.is_empty());
    }

    #[test]
    fn prompt_on_ga_and_eor() {
        let mut t = Telnet::new("0.1.0");
        let out = feed(&mut t, &[b'>', IAC, GA, b'<', IAC, EOR_CMD]);
        assert_eq!(
            out.events,
            vec![Event::Text(b">".to_vec()), Event::Prompt, Event::Text(b"<".to_vec()), Event::Prompt]
        );
    }

    #[test]
    fn naws_is_16_bit_and_escaped() {
        let mut t = Telnet::new("0.1.0");
        t.resize(255, 300);
        let out = feed(&mut t, &[IAC, DO, OPT_NAWS]);
        assert_eq!(
            out.replies,
            vec![IAC, WILL, OPT_NAWS, IAC, SB, OPT_NAWS, 0, IAC, IAC, 1, 44, IAC, SE]
        );
        assert_eq!(t.resize(100, 40), Some(vec![IAC, SB, OPT_NAWS, 0, 100, 0, 40, IAC, SE]));
    }

    #[test]
    fn no_naws_until_asked() {
        let mut t = Telnet::new("0.1.0");
        assert_eq!(t.resize(100, 40), None);
    }

    #[test]
    fn mtts_cycle() {
        let mut t = Telnet::new("0.1.0");
        let send = [IAC, SB, OPT_TTYPE, TTYPE_SEND, IAC, SE];
        let answers: Vec<String> = (0..4)
            .map(|_| {
                let r = feed(&mut t, &send).replies;
                String::from_utf8(r[4..r.len() - 2].to_vec()).unwrap()
            })
            .collect();
        assert_eq!(answers, ["COUPLER", "ANSI-TRUECOLOR", "MTTS 269", "MTTS 269"]);
    }

    #[test]
    fn new_environ_answers_only_whats_asked_and_never_user() {
        let mut t = Telnet::new("0.1.0");
        let mut ask = vec![IAC, SB, OPT_NEW_ENVIRON, ENV_SEND, ENV_VAR];
        ask.extend(b"USER");
        ask.push(ENV_VAR);
        ask.extend(b"CLIENT_NAME");
        ask.extend([IAC, SE]);
        let r = feed(&mut t, &ask).replies;
        let mut want = vec![IAC, SB, OPT_NEW_ENVIRON, ENV_IS, ENV_VAR];
        want.extend(b"CLIENT_NAME");
        want.push(ENV_VALUE);
        want.extend(b"Coupler");
        want.extend([IAC, SE]);
        assert_eq!(r, want);
    }

    #[test]
    fn gmcp_hello_and_messages() {
        let mut t = Telnet::new("0.1.0");
        let out = feed(&mut t, &[IAC, WILL, OPT_GMCP]);
        assert_eq!(&out.replies[..3], &[IAC, DO, OPT_GMCP]);
        let hello = String::from_utf8_lossy(&out.replies);
        assert!(hello.contains(r#"Core.Hello {"client":"Coupler","version":"0.1.0"}"#));
        assert!(hello.contains(r#"Core.Supports.Set ["Core 1","Char 1","Room 1","Comm 1"]"#));

        let mut msg = vec![IAC, SB, OPT_GMCP];
        msg.extend(br#"Char.Vitals {"hp":10,"maxhp":20}"#);
        msg.extend([IAC, SE]);
        let out = feed(&mut t, &msg);
        assert_eq!(out.events, vec![Event::Gmcp("Char.Vitals".into(), r#"{"hp":10,"maxhp":20}"#.into())]);
    }

    #[test]
    fn gmcp_ignored_unless_agreed() {
        let mut t = Telnet::new("0.1.0");
        let mut msg = vec![IAC, SB, OPT_GMCP];
        msg.extend(b"Core.Ping");
        msg.extend([IAC, SE]);
        assert!(feed(&mut t, &msg).events.is_empty());
    }

    fn zlib(data: &[u8]) -> Vec<u8> {
        let mut e = ZlibEncoder::new(Vec::new(), Compression::default());
        e.write_all(data).unwrap();
        e.finish().unwrap()
    }

    #[test]
    fn mccp2_in_one_read_then_split_reads() {
        let mut t = Telnet::new("0.1.0");
        let out = feed(&mut t, &[IAC, WILL, OPT_MCCP2]);
        assert_eq!(out.replies, vec![IAC, DO, OPT_MCCP2]);

        let mut stream = b"before".to_vec();
        stream.extend([IAC, SB, OPT_MCCP2, IAC, SE]);
        let compressed = zlib(b"hello, compressed world\r\n");
        stream.extend(&compressed[..5]);
        let a = feed(&mut t, &stream);
        let b = feed(&mut t, &compressed[5..]);
        let mut all = text(&a);
        all.extend(text(&b));
        assert_eq!(all, b"beforehello, compressed world\r\n");
    }

    #[test]
    fn mccp2_end_returns_to_plain() {
        let mut t = Telnet::new("0.1.0");
        feed(&mut t, &[IAC, WILL, OPT_MCCP2]);
        let mut stream = vec![IAC, SB, OPT_MCCP2, IAC, SE];
        stream.extend(zlib(b"squeezed "));
        stream.extend(b"plain");
        assert_eq!(text(&feed(&mut t, &stream)), b"squeezed plain");
    }

    #[test]
    fn mccp2_ignored_unless_agreed() {
        let mut t = Telnet::new("0.1.0");
        let mut stream = vec![IAC, SB, OPT_MCCP2, IAC, SE];
        stream.extend(b"plain");
        assert_eq!(text(&feed(&mut t, &stream)), b"plain");
    }

    #[test]
    fn encode_line_ends_in_crlf() {
        assert_eq!(encode_line("look"), b"look\r\n");
    }
}
