//! How many are playing now, for the greeting at launch: MSSP, the Mud
//! Server Status Protocol (telnet option 70), the way MUD lists ask a
//! server for its figures. Pure and unit-tested; `lib.rs` opens the
//! socket.
//!
//! CoffeeMUD offers MSSP (`IAC WILL MSSP`) to every connection as it
//! opens, before the login (`DefaultSession`, unless the server turns
//! MSSP off). Coupler answers `IAC DO MSSP`, and the server sends its
//! table at once (`IAC SB MSSP`, each `MSSP_VAR` name then one or more
//! `MSSP_VAL` values, `IAC SE`; `CMProtocols.getMSSPPackage`).
//! `PLAYERS` is how many are online on that server
//! (`Sessions.numLocalOnline`). Everything else offered is refused, so
//! nothing compresses or waits on an answer, and nothing about the
//! player is sent: the probe never logs in, and hangs up as soon as the
//! figure comes.
//!
//! The same table names the server's version, `CODEBASE` ("CoffeeMUD
//! v5.11.0.4", `CMProps.Str.MUDVER`, set from `MUD.HOST_VERSION`), which
//! is checked against [`BUILT_FOR`], the version of the source Coupler
//! was made from (`reference/CoffeeMud/`, snapshot `720aec6`): a game
//! that's moved on may say things the studies never saw, so the
//! greeting warns (`fit`).
//!
//! The play count beside it is Coupler's own, kept on this Mac
//! (`session.rs`, `played.json`): Coupler has no server to count
//! anyone else's (CLAUDE.md rule 1).

const IAC: u8 = 255;
const DONT: u8 = 254;
const DO: u8 = 253;
const WONT: u8 = 252;
const WILL: u8 = 251;
const SB: u8 = 250;
const SE: u8 = 240;
const OPT_MSSP: u8 = 70;
const MSSP_VAR: u8 = 1;
const MSSP_VAL: u8 = 2;

/// CoffeeMUD's version in the snapshot Coupler was made from
/// (`application/MUD.java`, `HOST_VERSION`). Change it with the snapshot.
pub const BUILT_FOR: &str = "5.11.0.4";

#[derive(Default, Clone, Copy, PartialEq)]
enum State {
    #[default]
    Data,
    Iac,
    Negotiate(u8),
    Sub,
    SubIac,
}

/// One probe's telnet, cut down to MSSP. Feed it what the socket reads;
/// write back what it returns.
#[derive(Default)]
pub struct Probe {
    state: State,
    sub: Vec<u8>,
    /// The `PLAYERS` figure, once the table has come.
    players: Option<u32>,
    /// The version in `CODEBASE`, once the table has come.
    version: Option<String>,
    /// The table came (with or without `PLAYERS`): nothing more to wait for.
    done: bool,
}

impl Probe {
    /// Bytes from the server. Returns the replies to send.
    pub fn feed(&mut self, input: &[u8]) -> Vec<u8> {
        let mut replies = Vec::new();
        for &b in input {
            self.state = match self.state {
                State::Data if b == IAC => State::Iac,
                State::Data => State::Data,
                State::Iac => match b {
                    WILL | WONT | DO | DONT => State::Negotiate(b),
                    SB => {
                        self.sub.clear();
                        State::Sub
                    }
                    _ => State::Data,
                },
                State::Negotiate(cmd) => {
                    match cmd {
                        WILL if b == OPT_MSSP => replies.extend([IAC, DO, b]),
                        WILL => replies.extend([IAC, DONT, b]),
                        DO => replies.extend([IAC, WONT, b]),
                        _ => {}
                    }
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
                        if sub.first() == Some(&OPT_MSSP) {
                            self.players = value(&sub[1..], b"PLAYERS").and_then(|v| v.trim().parse().ok());
                            self.version = value(&sub[1..], b"CODEBASE").and_then(|v| version_of(&v));
                            self.done = true;
                        }
                        State::Data
                    }
                    IAC => {
                        self.sub.push(IAC);
                        State::Sub
                    }
                    _ => {
                        self.sub.clear();
                        State::Data
                    }
                },
            };
        }
        replies
    }

    /// Whether the table has come.
    pub fn done(&self) -> bool {
        self.done
    }

    /// How many are online, if the server said.
    pub fn players(&self) -> Option<u32> {
        self.players
    }

    /// The server's CoffeeMUD version ("5.11.0.4"), if it said.
    pub fn version(&self) -> Option<&str> {
        self.version.as_deref()
    }
}

/// The version in a `CODEBASE` value: "CoffeeMUD v5.11.0.4" is
/// "5.11.0.4". None when there's no dotted number in it.
pub fn version_of(codebase: &str) -> Option<String> {
    let start = codebase.find(|c: char| c.is_ascii_digit())?;
    let version: String = codebase[start..].chars().take_while(|c| c.is_ascii_digit() || *c == '.').collect();
    let version = version.trim_end_matches('.');
    version.contains('.').then(|| version.to_string())
}

/// How the server's version stands to [`BUILT_FOR`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub enum Fit {
    Same,
    /// The game is newer than the source Coupler was made from.
    Newer,
    /// The game is older.
    Older,
}

/// `version` against [`BUILT_FOR`], part by part as numbers ("5.11.0.10"
/// is newer than "5.11.0.4"; a missing part counts as 0).
pub fn fit(version: &str) -> Fit {
    let parts = |v: &str| v.split('.').map(|p| p.parse::<u64>().unwrap_or(0)).collect::<Vec<_>>();
    let (theirs, ours) = (parts(version), parts(BUILT_FOR));
    let n = theirs.len().max(ours.len());
    let at = |v: &[u64], i: usize| v.get(i).copied().unwrap_or(0);
    match (0..n).map(|i| at(&theirs, i).cmp(&at(&ours, i))).find(|o| o.is_ne()) {
        Some(std::cmp::Ordering::Greater) => Fit::Newer,
        Some(_) => Fit::Older,
        None => Fit::Same,
    }
}

/// `name`'s first value in an MSSP table, as text.
fn value(table: &[u8], wanted: &[u8]) -> Option<String> {
    let mut name: Option<&[u8]> = None;
    let mut rest = table;
    while let Some((&mark, after)) = rest.split_first() {
        let end = after.iter().position(|&b| b == MSSP_VAR || b == MSSP_VAL).unwrap_or(after.len());
        let (field, next) = after.split_at(end);
        match mark {
            MSSP_VAR => name = Some(field),
            MSSP_VAL if name == Some(wanted) => return Some(String::from_utf8_lossy(field).into_owned()),
            _ => {}
        }
        rest = next;
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    fn table(pairs: &[(&str, &str)]) -> Vec<u8> {
        let mut t = vec![IAC, SB, OPT_MSSP];
        for (name, value) in pairs {
            t.push(MSSP_VAR);
            t.extend(name.as_bytes());
            t.push(MSSP_VAL);
            t.extend(value.as_bytes());
        }
        t.extend([IAC, SE]);
        t
    }

    #[test]
    fn mssp_is_taken_and_the_rest_refused() {
        let mut p = Probe::default();
        let replies = p.feed(&[IAC, WILL, OPT_MSSP, IAC, WILL, 86, IAC, DO, 24, b'H', b'i']);
        assert_eq!(replies, vec![IAC, DO, OPT_MSSP, IAC, DONT, 86, IAC, WONT, 24]);
        assert!(!p.done());
    }

    #[test]
    fn players_is_read_from_the_table() {
        let mut p = Probe::default();
        let mut bytes = b"Connecting to CoffeeMUD...\r\n".to_vec();
        bytes.extend(table(&[("NAME", "CoffeeMUD"), ("PLAYERS", "37"), ("UPTIME", "1759500000")]));
        // Split anywhere: a read can end mid-table.
        let (a, b) = bytes.split_at(40);
        p.feed(a);
        assert!(!p.done());
        p.feed(b);
        assert!(p.done());
        assert_eq!(p.players(), Some(37));
    }

    #[test]
    fn a_list_of_values_and_a_missing_figure() {
        let mut t = vec![IAC, SB, OPT_MSSP, MSSP_VAR];
        t.extend(b"PORT");
        for port in ["23", "2323"] {
            t.push(MSSP_VAL);
            t.extend(port.as_bytes());
        }
        t.extend([MSSP_VAR]);
        t.extend(b"PLAYERS");
        t.extend([MSSP_VAL, b'0', IAC, SE]);
        let mut p = Probe::default();
        p.feed(&t);
        assert_eq!(p.players(), Some(0));

        let mut p = Probe::default();
        p.feed(&table(&[("NAME", "CoffeeMUD")]));
        assert!(p.done());
        assert_eq!(p.players(), None);
    }

    #[test]
    fn the_version_is_read_from_the_codebase() {
        let mut p = Probe::default();
        p.feed(&table(&[("CODEBASE", "CoffeeMUD v5.11.0.4"), ("PLAYERS", "3")]));
        assert_eq!(p.version(), Some("5.11.0.4"));
        let mut p = Probe::default();
        p.feed(&table(&[("PLAYERS", "3")]));
        assert_eq!(p.version(), None);
        assert_eq!(version_of("CoffeeMUD v5.12."), Some("5.12".into()));
        assert_eq!(version_of("CoffeeMUD"), None);
        assert_eq!(version_of("CoffeeMUD v5"), None);
    }

    #[test]
    fn versions_are_compared_as_numbers() {
        assert_eq!(fit(BUILT_FOR), Fit::Same);
        assert_eq!(fit("5.11.0.4.0"), Fit::Same);
        assert_eq!(fit("5.11.0.10"), Fit::Newer);
        assert_eq!(fit("5.12"), Fit::Newer);
        assert_eq!(fit("5.11.0.3"), Fit::Older);
        assert_eq!(fit("5.10.9.9"), Fit::Older);
    }

    #[test]
    fn another_subnegotiation_is_not_the_table() {
        let mut p = Probe::default();
        p.feed(&[IAC, SB, 24, 1, IAC, SE]);
        assert!(!p.done());
    }
}
