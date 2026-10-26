//! The everyday sounds of play that CoffeeMUD names no MSP sound for:
//! someone coming into the room or leaving it, and the player sitting,
//! standing, sleeping, waking, handling things, dressing and trading.
//! Each is recorded as Coupler's own pair, `coupler.event` (`"you sit"`),
//! all of them listed from the start, so the player's triggers can give
//! each a sound; where none does, coffeemud.sf2's plays
//! (`soundbank::event_default`). Pure and unit-tested; shared with the
//! web build.
//!
//! Comings and goings are GMCP's `room.enter` and `room.leave` (pushed
//! when another creature comes or goes and the player can sense it,
//! `StdMOB`, right after the line the player sees), each with the line
//! that says how and where (`Comings`). CoffeeMUD words them
//! (`MUDTracker.move`) "<S-NAME> <arrive word> from <where>." and
//! "<S-NAME> <leave word> <direction>.", the words its race's (an orc
//! "arrives", a snake "slithers in"), its gait's (the GAIT command, its
//! choices `GAIT_LIST` in `lists.ini`: AMBLE "ambles in", THUNDER
//! "thunders in", TIPTOE "tiptoes in"…) or what it's doing (flying,
//! swimming, sneaking, falling: `Sense.getPresentDispositionVerb`); a
//! flight is "<S-NAME> flees <direction>." The word tells the `Manner`,
//! each its own value and its own sound, and the direction is sent with
//! what it sets off, so the sound is heard from where they came or went
//! (`Place`). The player's own doings are the game's lines to them, as CoffeeMUD's
//! commands word them for the one who acts (`<S-NAME> sit(s) down and
//! take(s) a rest.` is "You sit down and take a rest."): `Commands/`
//! `Sit`, `Stand`, `Sleep`, `Wake`, `Get`, `Drop`, `Give`, `Put`, `Wear`,
//! `Wield`, `Hold`, `Remove`, `Buy` and `Sell`. Others doing the same
//! read "Bob sits down…", never "You", so only the player's count. Not
//! GMCP's `char.status` `pos`: it waits for the next tick. Money handled
//! (got, dropped, given, put away, looted, or the bank's and the
//! auction's: `Deposit`, `Withdraw`, `Borrow`, `Pay`, `Bid`) is `Coins`,
//! whatever the verb (`loot::is_money`).

use std::time::Duration;

use serde::Serialize;

use crate::clock::Instant;

/// Coupler's own key for them in the hooks list.
pub const EVENT_KEY: &str = "coupler.event";

/// How someone comes or goes, by the word the game uses.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Manner {
    /// Walks, ambles, strides, hops, rides, the plain "arrives".
    Walks,
    /// Creeps, tiptoes, sneaks, slithers, glides, floats: hardly heard.
    Creeps,
    /// Thunders, stomps, storms, rolls.
    Stomps,
    Flies,
    Swims,
    Falls,
}

impl Manner {
    pub const ALL: [Manner; 6] = [Manner::Walks, Manner::Creeps, Manner::Stomps, Manner::Flies, Manner::Swims, Manner::Falls];
}

/// Each word CoffeeMUD comes or goes by (the races', the gaits', the
/// dispositions', a mount's), as the manner it's heard in.
const WORDS: [(&str, Manner); 52] = [
    ("arrives", Manner::Walks),
    ("leaves", Manner::Walks),
    ("walks", Manner::Walks),
    ("runs", Manner::Walks),
    ("ambles", Manner::Walks),
    ("skips", Manner::Walks),
    ("stumbles", Manner::Walks),
    ("wanders", Manner::Walks),
    ("waddles", Manner::Walks),
    ("drags", Manner::Walks),
    ("dances", Manner::Walks),
    ("saunters", Manner::Walks),
    ("meanders", Manner::Walks),
    ("strolls", Manner::Walks),
    ("strides", Manner::Walks),
    ("sashays", Manner::Walks),
    ("shuffles", Manner::Walks),
    ("hops", Manner::Walks),
    ("flops", Manner::Walks),
    ("struts", Manner::Walks),
    ("shambles", Manner::Walks),
    ("sleepwalks", Manner::Walks),
    ("climbs", Manner::Walks),
    ("sits", Manner::Walks),
    ("rides", Manner::Walks),
    ("sails", Manner::Walks),
    ("tiptoes", Manner::Creeps),
    ("glides", Manner::Creeps),
    ("slides", Manner::Creeps),
    ("drifts", Manner::Creeps),
    ("creeps", Manner::Creeps),
    ("crawls", Manner::Creeps),
    ("slithers", Manner::Creeps),
    ("scurries", Manner::Creeps),
    ("scuttles", Manner::Creeps),
    ("skitters", Manner::Creeps),
    ("sneaks", Manner::Creeps),
    ("prowls", Manner::Creeps),
    ("floats", Manner::Creeps),
    ("thunders", Manner::Stomps),
    ("stomps", Manner::Stomps),
    ("storms", Manner::Stomps),
    ("rolls", Manner::Stomps),
    ("flies", Manner::Flies),
    ("soars", Manner::Flies),
    ("swims", Manner::Swims),
    ("sloshes", Manner::Swims),
    ("sinks", Manner::Swims),
    ("jets", Manner::Swims),
    ("falls", Manner::Falls),
    // Fleeing is a happening of its own; a word that's never a manner.
    ("flees", Manner::Walks),
    ("flee", Manner::Walks),
];

/// Something that happened, as its pair's value says it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Happening {
    Arrives(Manner),
    Leaves(Manner),
    Flees,
    Sit,
    Stand,
    Sleep,
    Wake,
    Get,
    Drop,
    Give,
    Put,
    Wear,
    Remove,
    Buy,
    Sell,
    /// Money changing hands: coins got, dropped, given, banked or bid.
    Coins,
}

impl Happening {
    pub const ALL: [Happening; 26] = [
        Happening::Arrives(Manner::Walks),
        Happening::Arrives(Manner::Creeps),
        Happening::Arrives(Manner::Stomps),
        Happening::Arrives(Manner::Flies),
        Happening::Arrives(Manner::Swims),
        Happening::Arrives(Manner::Falls),
        Happening::Leaves(Manner::Walks),
        Happening::Leaves(Manner::Creeps),
        Happening::Leaves(Manner::Stomps),
        Happening::Leaves(Manner::Flies),
        Happening::Leaves(Manner::Swims),
        Happening::Leaves(Manner::Falls),
        Happening::Flees,
        Happening::Sit,
        Happening::Stand,
        Happening::Sleep,
        Happening::Wake,
        Happening::Get,
        Happening::Drop,
        Happening::Give,
        Happening::Put,
        Happening::Wear,
        Happening::Remove,
        Happening::Buy,
        Happening::Sell,
        Happening::Coins,
    ];

    /// Its value in the hooks list.
    pub fn name(self) -> &'static str {
        match self {
            Happening::Arrives(Manner::Walks) => "someone arrives",
            Happening::Arrives(Manner::Creeps) => "someone creeps in",
            Happening::Arrives(Manner::Stomps) => "someone stomps in",
            Happening::Arrives(Manner::Flies) => "someone flies in",
            Happening::Arrives(Manner::Swims) => "someone swims in",
            Happening::Arrives(Manner::Falls) => "someone falls in",
            Happening::Leaves(Manner::Walks) => "someone leaves",
            Happening::Leaves(Manner::Creeps) => "someone creeps away",
            Happening::Leaves(Manner::Stomps) => "someone stomps away",
            Happening::Leaves(Manner::Flies) => "someone flies away",
            Happening::Leaves(Manner::Swims) => "someone swims away",
            Happening::Leaves(Manner::Falls) => "someone falls away",
            Happening::Flees => "someone flees",
            Happening::Sit => "you sit",
            Happening::Stand => "you stand",
            Happening::Sleep => "you sleep",
            Happening::Wake => "you wake",
            Happening::Get => "you get",
            Happening::Drop => "you drop",
            Happening::Give => "you give",
            Happening::Put => "you put",
            Happening::Wear => "you wear",
            Happening::Remove => "you remove",
            Happening::Buy => "you buy",
            Happening::Sell => "you sell",
            Happening::Coins => "you handle coins",
        }
    }
}

/// Where a sound is heard from: the way someone came from or went, as
/// the game's direction letter (`N`, `NE`, `U`), for the frontend to
/// place it (left and right, higher and lower, as Immersive's exits).
#[derive(Serialize, Debug, Clone, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct Place {
    pub dir: &'static str,
    /// Going that way, rather than coming from it.
    pub toward: bool,
}

/// Someone coming or going, with where from or to if the line said.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Movement {
    pub happening: Happening,
    pub place: Option<Place>,
}

/// Where an arrival comes from, as the game says it ("from the north"):
/// on land, aboard a ship, in a caravan. "Out of nowhere" is nowhere.
const FROM: [(&str, &str); 28] = [
    ("the north", "N"),
    ("the south", "S"),
    ("the east", "E"),
    ("the west", "W"),
    ("above", "U"),
    ("below", "D"),
    ("the northeast", "NE"),
    ("the northwest", "NW"),
    ("the southeast", "SE"),
    ("the southwest", "SW"),
    ("foreward", "N"),
    ("aft", "S"),
    ("starboard", "E"),
    ("portside", "W"),
    ("forward starboard", "NE"),
    ("forward portside", "NW"),
    ("aft starboard", "SE"),
    ("aft portside", "SW"),
    ("back", "S"),
    ("right", "E"),
    ("left", "W"),
    ("up", "U"),
    ("down", "D"),
    ("forward right", "NE"),
    ("forward left", "NW"),
    ("back right", "SE"),
    ("back left", "SW"),
    ("out of nowhere", ""),
];

/// Where a departure goes, as the game names the direction, lowercased.
const TOWARD: [(&str, &str); 32] = [
    ("north", "N"),
    ("south", "S"),
    ("east", "E"),
    ("west", "W"),
    ("up", "U"),
    ("down", "D"),
    ("northeast", "NE"),
    ("northwest", "NW"),
    ("southeast", "SE"),
    ("southwest", "SW"),
    ("foreward", "N"),
    ("aft", "S"),
    ("starboard", "E"),
    ("portside", "W"),
    ("above", "U"),
    ("below", "D"),
    ("fore starboard", "NE"),
    ("fore portside", "NW"),
    ("aft starboard", "SE"),
    ("aft portside", "SW"),
    ("back", "S"),
    ("right", "E"),
    ("left", "W"),
    ("fore right", "NE"),
    ("fore left", "NW"),
    ("back right", "SE"),
    ("back left", "SW"),
    // A gate: through it, nowhere on the compass.
    ("there", ""),
    ("through", ""),
    ("in", ""),
    ("out", ""),
    ("away", ""),
];

/// A line that may say someone came or went, until GMCP says they did.
#[derive(Debug, Clone)]
struct Seen {
    /// The line, lowercased: it starts with their name.
    text: String,
    arrives: bool,
    /// The coming-and-going word found in it, if any.
    manner: Option<Manner>,
    flees: bool,
    dir: &'static str,
    at: Instant,
}

/// How long a line waits for its GMCP (sent straight after it).
const WAIT: Duration = Duration::from_secs(3);

/// The comings and goings: each line that reads like one is kept a moment
/// (`line`), and GMCP's `room.enter` or `room.leave` (`gmcp`) takes the
/// one that starts with that creature's name, for how and where. With no
/// such line (a word of a gait CoffeeMUD added later, a line in another
/// language), it's someone arriving or leaving, from nowhere in
/// particular. A line no GMCP follows (a say that reads like one) is
/// forgotten.
#[derive(Debug, Default)]
pub struct Comings {
    seen: Vec<Seen>,
}

impl Comings {
    /// A finished line of the game's: kept if it reads like a coming or
    /// a going.
    pub fn line(&mut self, text: &str, now: Instant) {
        self.seen.retain(|s| now.duration_since(s.at) < WAIT);
        if let Some(seen) = read(text, now) {
            if self.seen.len() >= 16 {
                self.seen.remove(0);
            }
            self.seen.push(seen);
        }
    }

    /// A GMCP message: someone coming into the room or leaving it, with
    /// the line that said how, if one did.
    pub fn gmcp(&mut self, package: &str, data: &str, now: Instant) -> Option<Movement> {
        let arrives = if package.eq_ignore_ascii_case("room.enter") {
            true
        } else if package.eq_ignore_ascii_case("room.leave") {
            false
        } else {
            return None;
        };
        self.seen.retain(|s| now.duration_since(s.at) < WAIT);
        let name = serde_json::from_str::<String>(data).unwrap_or_default().to_lowercase();
        let found = self
            .seen
            .iter()
            .rposition(|s| s.arrives == arrives && !name.is_empty() && s.text.starts_with(&name) && s.text[name.len()..].starts_with(' '))
            .map(|i| self.seen.remove(i));
        let happening = match (&found, arrives) {
            (Some(s), false) if s.flees => Happening::Flees,
            (s, true) => Happening::Arrives(s.as_ref().and_then(|s| s.manner).unwrap_or(Manner::Walks)),
            (s, false) => Happening::Leaves(s.as_ref().and_then(|s| s.manner).unwrap_or(Manner::Walks)),
        };
        let place = found.filter(|s| !s.dir.is_empty()).map(|s| Place { dir: s.dir, toward: !arrives });
        Some(Movement { happening, place })
    }
}

/// A line read as a coming ("… from the north.") or a going ("…
/// north."), or neither.
fn read(text: &str, now: Instant) -> Option<Seen> {
    let text = text.trim().to_lowercase();
    let body = text.strip_suffix('.').or_else(|| text.strip_suffix('!'))?;
    // A coming: "<name> <word> [in] from <where>".
    if let Some(i) = body.rfind(" from ") {
        if let Some((_, dir)) = FROM.iter().find(|(from, _)| body[i + 6..] == **from) {
            let words: Vec<&str> = body[..i].split(' ').collect();
            let manner = words.iter().skip(1).find_map(|w| WORDS.iter().find(|(word, _)| word == w)).map(|(_, m)| *m);
            return Some(Seen { text: text.clone(), arrives: true, manner, flees: false, dir, at: now });
        }
    }
    // A going: "<name> <word> <direction>", the word found before it.
    let (toward, dir) = TOWARD.iter().find(|(toward, _)| body.strip_suffix(toward).is_some_and(|rest| rest.ends_with(' ')))?;
    let rest = &body[..body.len() - toward.len()];
    let words: Vec<&str> = rest.split_whitespace().collect();
    let found = words.iter().enumerate().skip(1).find_map(|(i, w)| WORDS.iter().find(|(word, _)| word == w).map(|(word, m)| (i, *word, *m)));
    // A going needs its word, or nearly anything ending "north." would do.
    let (_, word, manner) = found?;
    let flees = word.starts_with("flee");
    Some(Seen { text: text.clone(), arrives: false, manner: Some(manner), flees, dir, at: now })
}

/// The lines CoffeeMUD's commands say to the one doing it, by how they
/// start; the first that fits counts, so the longer come first.
const OWN: [(&str, Happening); 23] = [
    ("You sit down and take a rest.", Happening::Sit),
    ("You sit on ", Happening::Sit),
    ("You stand up.", Happening::Stand),
    ("You lay down and take a nap.", Happening::Sleep),
    ("You wake up.", Happening::Wake),
    ("You awake and get up.", Happening::Wake),
    ("You awake and sit up.", Happening::Wake),
    ("You get ", Happening::Get),
    ("You drop ", Happening::Drop),
    ("You give ", Happening::Give),
    // Putting a light out is neither.
    ("You put out ", Happening::Put),
    ("You put on ", Happening::Wear),
    ("You put ", Happening::Put),
    ("You wield ", Happening::Wear),
    ("You hold ", Happening::Wear),
    ("You remove ", Happening::Remove),
    ("You buy ", Happening::Buy),
    ("You sell ", Happening::Sell),
    // Only ever money here (`line`).
    ("You deposit ", Happening::Coins),
    ("You withdraw ", Happening::Coins),
    ("You borrow ", Happening::Coins),
    ("You pay ", Happening::Coins),
    ("You bid ", Happening::Coins),
];

/// A finished line of the game's: one of the player's own doings. The
/// line may follow the prompt it was typed at (`<20hp 10m 30mv> You get
/// a sword.`).
pub fn line(text: &str) -> Option<Happening> {
    let text = text.trim();
    let own = if text.starts_with("You ") { text } else { &text[text.find("> You ")? + 2..] };
    let (start, happening) = OWN.iter().find(|(start, _)| own.starts_with(start))?;
    let money = crate::loot::is_money(own);
    match happening {
        Happening::Get | Happening::Drop | Happening::Give | Happening::Put if money && *start != "You put out " => Some(Happening::Coins),
        // A thing banked or posted, not money: no sound of its own.
        Happening::Coins if !money => None,
        Happening::Put if *start == "You put out " => None,
        // "You put a ring on your finger." is wearing it; "You put a
        // ring in a pouch" (or on a table) is putting it.
        Happening::Put if own.contains(" on your ") => Some(Happening::Wear),
        // A full line, said as the command says it; one that only starts
        // the same isn't this ("You sit down and take a rest, at last").
        _ if start.ends_with('.') && own != *start => None,
        _ => Some(*happening),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A line, then its GMCP, as CoffeeMUD sends them.
    fn moved(line: &str, package: &str, name: &str) -> Option<Movement> {
        let now = Instant::now();
        let mut c = Comings::default();
        c.line(line, now);
        c.gmcp(package, &format!("\"{name}\""), now)
    }

    fn place(dir: &'static str, toward: bool) -> Option<Place> {
        Some(Place { dir, toward })
    }

    #[test]
    fn comings_and_goings_are_gmcp() {
        let now = Instant::now();
        let mut c = Comings::default();
        assert_eq!(c.gmcp("room.enter", "\"an orc\"", now), Some(Movement { happening: Happening::Arrives(Manner::Walks), place: None }));
        assert_eq!(c.gmcp("Room.Leave", "\"Bob\"", now).map(|m| m.happening), Some(Happening::Leaves(Manner::Walks)));
        assert_eq!(c.gmcp("room.info", "{}", now), None);
    }

    #[test]
    fn how_and_where_from_the_line() {
        let cases = [
            ("An orc arrives from the north.", "room.enter", "an orc", Happening::Arrives(Manner::Walks), place("N", false)),
            ("A garter snake slithers in from the southwest.", "room.enter", "a garter snake", Happening::Arrives(Manner::Creeps), place("SW", false)),
            ("Bob ambles in from the east.", "room.enter", "Bob", Happening::Arrives(Manner::Walks), place("E", false)),
            ("Bob thunders in from below.", "room.enter", "Bob", Happening::Arrives(Manner::Stomps), place("D", false)),
            ("A sparrow flies in from above.", "room.enter", "a sparrow", Happening::Arrives(Manner::Flies), place("U", false)),
            ("A trout swims in from the west.", "room.enter", "a trout", Happening::Arrives(Manner::Swims), place("W", false)),
            ("A rock falls in from above.", "room.enter", "a rock", Happening::Arrives(Manner::Falls), place("U", false)),
            ("A thief sneaks in from out of nowhere.", "room.enter", "a thief", Happening::Arrives(Manner::Creeps), None),
            ("Bob rides a horse in from the south.", "room.enter", "Bob", Happening::Arrives(Manner::Walks), place("S", false)),
            ("A sailor arrives from portside.", "room.enter", "a sailor", Happening::Arrives(Manner::Walks), place("W", false)),
            ("An orc leaves north.", "room.leave", "an orc", Happening::Leaves(Manner::Walks), place("N", true)),
            ("Bob tiptoes northeast.", "room.leave", "Bob", Happening::Leaves(Manner::Creeps), place("NE", true)),
            ("Bob drags himself up.", "room.leave", "Bob", Happening::Leaves(Manner::Walks), place("U", true)),
            ("Bob stomps down.", "room.leave", "Bob", Happening::Leaves(Manner::Stomps), place("D", true)),
            ("A sparrow flies south.", "room.leave", "a sparrow", Happening::Leaves(Manner::Flies), place("S", true)),
            ("An orc flees west.", "room.leave", "an orc", Happening::Flees, place("W", true)),
            ("Bob rides a horse east.", "room.leave", "Bob", Happening::Leaves(Manner::Walks), place("E", true)),
        ];
        for (line, package, name, happening, want) in cases {
            assert_eq!(moved(line, package, name), Some(Movement { happening, place: want }), "{line}");
        }
    }

    #[test]
    fn a_line_waits_only_for_its_own() {
        let now = Instant::now();
        let mut c = Comings::default();
        // Two in one read, each taken by its own name.
        c.line("An orc arrives from the north.", now);
        c.line("A goblin creeps in from the east.", now);
        assert_eq!(c.gmcp("room.enter", "\"a goblin\"", now).and_then(|m| m.place), place("E", false));
        assert_eq!(c.gmcp("room.enter", "\"an orc\"", now).and_then(|m| m.place), place("N", false));
        // Taken once: the next is from nowhere said.
        assert_eq!(c.gmcp("room.enter", "\"an orc\"", now).and_then(|m| m.place), None);
        // An arrival's line isn't a departure's; another's name isn't theirs.
        c.line("An orc arrives from the north.", now);
        assert_eq!(c.gmcp("room.leave", "\"an orc\"", now).and_then(|m| m.place), None);
        assert_eq!(c.gmcp("room.enter", "\"an orcish guard\"", now).and_then(|m| m.place), None);
        // A line that no GMCP follows is forgotten.
        c.line("Bob says 'an orc arrives from the north.'", now);
        let later = now + WAIT;
        assert_eq!(c.gmcp("room.enter", "\"an orc\"", later).and_then(|m| m.place), None);
    }

    #[test]
    fn not_every_line_ending_north() {
        let now = Instant::now();
        for line in ["A path leads north.", "You see a gate to the north.", "The wind blows from the north and the east."] {
            assert!(read(line, now).is_none(), "{line}");
        }
    }

    #[test]
    fn the_players_own_doings() {
        let cases = [
            ("You sit down and take a rest.", Some(Happening::Sit)),
            ("You sit on a wooden bench.", Some(Happening::Sit)),
            ("You stand up.", Some(Happening::Stand)),
            ("You lay down and take a nap.", Some(Happening::Sleep)),
            ("You wake up.", Some(Happening::Wake)),
            ("You awake and get up.", Some(Happening::Wake)),
            ("You awake and sit up.", Some(Happening::Wake)),
            ("You get a long sword.", Some(Happening::Get)),
            ("You get a ration from a backpack.", Some(Happening::Get)),
            ("You drop a ration.", Some(Happening::Drop)),
            ("You give a ration to the shopkeeper.", Some(Happening::Give)),
            ("You put a ration in a backpack.", Some(Happening::Put)),
            ("You put a ration on a table.", Some(Happening::Put)),
            ("You put on a leather cap.", Some(Happening::Wear)),
            ("You put a gold ring on your left finger.", Some(Happening::Wear)),
            ("You wield a long sword.", Some(Happening::Wear)),
            ("You hold a torch.", Some(Happening::Wear)),
            ("You remove a leather cap.", Some(Happening::Remove)),
            ("You buy a ration from the grocer.", Some(Happening::Buy)),
            ("You sell a dagger to the smith.", Some(Happening::Sell)),
            ("You get 12 gold coins from the body of an orc.", Some(Happening::Coins)),
            ("You get 3 silver pieces.", Some(Happening::Coins)),
            ("You drop 1 gold coin.", Some(Happening::Coins)),
            ("You give 10 gold coins to the shopkeeper.", Some(Happening::Coins)),
            ("You put 5 copper bits in a pouch.", Some(Happening::Coins)),
            ("You deposit 100 gold coins into your account with the banker.", Some(Happening::Coins)),
            ("You withdraw 50 gold coins from your account with the banker.", Some(Happening::Coins)),
            ("You borrow 200 gold coins from the banker.", Some(Happening::Coins)),
            ("You pay Bob 20 gold coins.", Some(Happening::Coins)),
            ("You bid 30 gold coins on a sword with the auctioneer.", Some(Happening::Coins)),
            ("You get 5 Copper Crowns from the body of a guard.", Some(Happening::Coins)),
            ("You give 3 conch(es) to a merman.", Some(Happening::Coins)),
            ("You drop 12 drachmas.", Some(Happening::Coins)),
            ("You get a bag of coins.", Some(Happening::Get)),
            ("You deposit a long sword with the banker.", None),
        ];
        for (text, want) in cases {
            assert_eq!(line(text), want, "{text}");
        }
    }

    #[test]
    fn after_the_prompt_too() {
        assert_eq!(line("<20hp 10m 30mv> You get a long sword."), Some(Happening::Get));
        assert_eq!(line("  You stand up.  "), Some(Happening::Stand));
    }

    #[test]
    fn not_others_nor_lookalikes() {
        for text in [
            "Bob sits down and takes a rest.",
            "Bob gets a long sword.",
            "You put out a torch.",
            "You stand up. Then you fall.",
            "You say 'You get nothing.'",
            "Bob says 'You get nothing.'",
            "You are hungry.",
            "You getaway driver.",
        ] {
            assert_eq!(line(text), None, "{text}");
        }
    }

    #[test]
    fn every_value_is_its_own() {
        let mut names: Vec<&str> = Happening::ALL.iter().map(|h| h.name()).collect();
        names.sort();
        names.dedup();
        assert_eq!(names.len(), Happening::ALL.len());
    }
}
