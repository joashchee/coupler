//! **Loot**: what AUTOLOOT and AUTOGOLD took off a body the moment its
//! creature died, for the Loot table (`tables.rs`) and so Immersive's
//! narrator says "Loot collected." once instead of every item. Pure and
//! unit-tested; shared with the web build.
//!
//! CoffeeMUD loots in the same breath as the death (`MUDFight.postDeath`):
//! the killer with AUTOLOOT gets each item the body holds, then everyone
//! with AUTOGOLD gets their share of its money, each by `postGet`, which
//! says "You get <item> from the body of <name>." (`Get.java`; the body
//! is named by `StdRace.getCorpseContainer`). So a "You get … from …"
//! line after "<name> is DEAD!!!" and before the player sends anything
//! is loot. One the player asked for by typing GET is a command's answer
//! (`echo.rs`), not loot.
//!
//! The wiki's AUTOLOOT page (read 2026-10-26) agrees: it "automatically
//! collect[s] any items from the corpses of vanquished foes". Money is
//! known by the game's own list of its currencies (`DENOMINATIONS`).

use std::sync::OnceLock;
use std::time::Duration;

use crate::clock::Instant;
use crate::fight_log::{read_line, Said, YOU};

/// How long after a death its loot can still come: it comes at once, but
/// a read can be cut anywhere.
const WINDOW: Duration = Duration::from_secs(3);

/// One thing taken off a body.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Got {
    /// As the game names it: "a long sword", "12 gold coins".
    pub item: String,
    /// The body: "the body of a rat".
    pub from: String,
}

/// The loot found in one read.
#[derive(Debug, Default, PartialEq, Eq)]
pub struct Found {
    /// Its lines, by their place in the read.
    pub lines: Vec<usize>,
    pub got: Vec<Got>,
    /// The line of the first loot of this death, if it's in this read:
    /// "Loot collected." is said there, once.
    pub first: Option<usize>,
    /// When the death was seen, Unix milliseconds: the haul the loot belongs to.
    pub haul: Option<i64>,
}

#[derive(Debug, Default)]
pub struct Looter {
    /// The last death, while its loot may still come: when, by the
    /// clock and in Unix milliseconds, and whether any has.
    open: Option<(Instant, i64, bool)>,
}

/// The line without a prompt the game wrote it after (`<20hp> You get …`).
fn own(text: &str) -> &str {
    let text = text.trim();
    if text.starts_with("You ") {
        return text;
    }
    text.find("> You ").map_or(text, |at| &text[at + 2..])
}

/// "You get a sword from the body of a rat.": the item and the body.
fn taken(line: &str) -> Option<Got> {
    let rest = line.strip_prefix("You get ")?;
    let rest = rest.strip_suffix('.').or_else(|| rest.strip_suffix('!'))?.trim_end();
    // The last " from ": an item's own name may have one ("a letter from home").
    let at = rest.rfind(" from ")?;
    let (item, from) = (rest[..at].trim(), rest[at + 6..].trim());
    (!item.is_empty() && !from.is_empty()).then(|| Got { item: item.to_string(), from: from.to_string() })
}

impl Looter {
    /// The player sent something: what's got from now on is theirs to
    /// ask for, not the death's.
    pub fn typed(&mut self) {
        self.open = None;
    }

    /// The connection or the character changed.
    pub fn clear(&mut self) {
        self.open = None;
    }

    /// One read's finished lines (their plain text): the loot in them.
    /// `ms` is now in Unix milliseconds.
    pub fn lines(&mut self, texts: &[String], now: Instant, ms: i64) -> Found {
        if self.open.is_some_and(|(at, _, _)| now.duration_since(at) > WINDOW) {
            self.open = None;
        }
        let mut found = Found::default();
        for (i, text) in texts.iter().enumerate() {
            let line = own(text);
            if matches!(read_line(line), Some(Said::Died { who }) if who != YOU) {
                self.open = Some((now, ms, false));
                continue;
            }
            let Some((_, haul, any)) = self.open.as_mut() else { continue };
            let Some(got) = taken(line) else { continue };
            if !*any {
                *any = true;
                found.first = Some(i);
            }
            found.haul = Some(*haul);
            found.lines.push(i);
            found.got.push(got);
        }
        found
    }
}

/// Every denomination of every currency, as the game names it
/// (`MoneyDenomination.name`, "(s)" its plural). coffeemud.net's areas use
/// nine currencies, read from its `HELP WORLD CURRENCY` (its web help,
/// `help.cmvp?HELPTOPIC=WORLD_CURRENCY`, 2026-10-26); the wiki names each
/// area's currency (`|Currency=` on its page) but not their coins. The
/// rates are in `docs/coffeemud-currencies.md`. The last are the source's
/// standards no area of coffeemud.net uses yet (`BeanCounter`); its
/// VICTORY points are bound and never change hands.
const DENOMINATIONS: &[&str] = &[
    // The default: "gold coin(s)".
    "gold coin(s)", "golden note(s)", "whole note(s)", "Archon note(s)",
    // Alramian (Alramus, Cora Isle, the Gambling Hall …): no "(s)", so the
    // same name for one or many.
    "Copper Crowns", "Gold Crowns", "Double Crowns", "Royal Crowns", "Archon Crown",
    // Merconomy (Atlantis): "(es)" isn't a plural the game makes
    // (`BeanCounter.getDenominationName`), so "3 conch(es)" as it stands.
    "sanddollar(s)", "shell(s)", "conch(es)", "nautilus(es)", "archonolus(es)",
    // Copper (Calinth, the Steading of the Hill Giants …).
    "copper bit(s)", "silver bit(s)", "gold bit(s)", "platinum bit(s)", "aluminum bit(s)",
    // Troya (Old Troya).
    "chalko(s)", "obol(s)", "drachma(s)", "mina(s)", "talent(s)",
    // Weed (Cannabis).
    "bud(s)", "stem(s)", "weed bag(s)",
    // Gold (the Dwarven Kingdom, Urgan`ali Dorica …).
    "copper piece(s)", "silver piece(s)", "gold piece(s)", "platinum piece(s)",
    // Charcurrency (Illysia`s Cottage).
    "Character Coin",
    // Darkcredits.
    "dark credit(s)", "dark coin(s)",
    // The source's CREDIT and DOLLAR.
    "credit(s)", "megacredit(s)", "gigacredit(s)", "cent(s)", "dollar(s)", "grand",
];

/// A denomination's names for one and for many, lowercase, as
/// `BeanCounter.getDenominationName` makes them: "gold coin(s)" is "gold
/// coin" and "gold coins"; any other name stands as it is.
fn forms(name: &str) -> [String; 2] {
    let name = name.to_lowercase();
    match name.strip_suffix("(s)") {
        Some(one) => [one.to_string(), format!("{one}s")],
        None => [name.clone(), name],
    }
}

/// Whether a thing the game names is money: CoffeeMUD names coins by
/// their count and denomination and nothing else (`StdCoins.Name`: "12
/// gold coins", "1 Archon note", "5 Copper Crowns"), so a count followed
/// by one of `DENOMINATIONS`.
pub fn is_money(text: &str) -> bool {
    static FORMS: OnceLock<Vec<String>> = OnceLock::new();
    let forms = FORMS.get_or_init(|| DENOMINATIONS.iter().flat_map(|d| forms(d)).collect());
    let lower = text.to_lowercase();
    // Each word's start in the line, so what follows a count can be read.
    let mut at = 0;
    for word in lower.split(' ') {
        let next = at + word.len() + 1;
        let count = word.trim_start_matches('(');
        let count = !count.is_empty() && count.chars().all(|c| c.is_ascii_digit() || c == ',' || c == '.') && count.chars().any(|c| c.is_ascii_digit());
        let rest = lower.get(next..).unwrap_or("");
        if count && forms.iter().any(|f| rest.strip_prefix(f.as_str()).is_some_and(|after| !after.starts_with(|c: char| c.is_alphanumeric()))) {
            return true;
        }
        at = next;
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;

    fn strings(lines: &[&str]) -> Vec<String> {
        lines.iter().map(|s| s.to_string()).collect()
    }

    #[test]
    fn loot_follows_a_death() {
        let mut l = Looter::default();
        let now = Instant::now();
        let found = l.lines(
            &strings(&[
                "You slice a rat.",
                "A rat is DEAD!!!",
                "You gain 25 experience points.",
                "You get a rat pelt from the body of a rat.",
                "You get 3 gold coins from the body of a rat.",
            ]),
            now,
            7,
        );
        assert_eq!(found.lines, vec![3, 4]);
        assert_eq!(found.first, Some(3));
        assert_eq!(found.haul, Some(7));
        assert_eq!(
            found.got,
            vec![Got { item: "a rat pelt".into(), from: "the body of a rat".into() }, Got { item: "3 gold coins".into(), from: "the body of a rat".into() }]
        );
    }

    #[test]
    fn a_read_cut_in_two_says_it_once() {
        let mut l = Looter::default();
        let now = Instant::now();
        let found = l.lines(&strings(&["An orc is DEAD!!!", "You get a club from the body of an orc."]), now, 7);
        assert_eq!(found.first, Some(1));
        let found = l.lines(&strings(&["<20hp 10m> You get a helmet from the body of an orc."]), now, 7);
        assert_eq!((found.lines, found.first), (vec![0], None));
        assert_eq!(found.got[0].item, "a helmet");
    }

    #[test]
    fn what_the_player_asks_for_isnt_loot() {
        let mut l = Looter::default();
        let now = Instant::now();
        l.lines(&strings(&["A rat is DEAD!!!"]), now, 7);
        l.typed();
        assert_eq!(l.lines(&strings(&["You get a rat pelt from the body of a rat."]), now, 7), Found::default());
        // Nor anything long after, nor without a death.
        l.lines(&strings(&["A rat is DEAD!!!"]), now, 7);
        assert!(l.lines(&strings(&["You get a pelt from the body of a rat."]), now + WINDOW + Duration::from_secs(1), 7).lines.is_empty());
        assert!(Looter::default().lines(&strings(&["You get a sword from a chest."]), now, 7).lines.is_empty());
    }

    #[test]
    fn the_players_death_loots_nothing() {
        let mut l = Looter::default();
        let found = l.lines(&strings(&["You are DEAD!!!", "You get a sword from a chest."]), Instant::now(), 7);
        assert!(found.lines.is_empty());
    }

    #[test]
    fn the_last_from_is_the_body() {
        assert_eq!(taken("You get a letter from home from the body of a courier."), Some(Got { item: "a letter from home".into(), from: "the body of a courier".into() }));
        assert_eq!(taken("You get a sword."), None);
        assert_eq!(taken("Bob gets a sword from the body of a rat."), None);
    }

    #[test]
    fn money_is_a_count_and_a_denomination() {
        for text in ["12 gold coins", "You get 1 gold coin from the body of a rat.", "You drop 3 silver pieces.", "You give 1,000 copper bits to Bob.", "You deposit 5 Archon notes into your account with the banker.", "You pay Bob 20 credits.", "You bid 100 dollars on a sword with the auctioneer."] {
            assert!(is_money(text), "{text}");
        }
        // A bag of coins is a bag; a count of anything else isn't money.
        for text in ["a gold ring", "a bag of coins", "You get a sword.", "You drop 3 rations.", "a piece of cheese", "You get 2 apples from a bag.", "You get 3 shellfish.", "You drop 2 gold coinpurses."] {
            assert!(!is_money(text), "{text}");
        }
    }

    #[test]
    fn every_currency_of_the_game_is_money() {
        // Each denomination, one and many, as `getDenominationName` names it.
        for name in DENOMINATIONS {
            let [one, many] = forms(name);
            assert!(is_money(&format!("You get 1 {one} from the body of a rat.")), "{one}");
            assert!(is_money(&format!("You drop 12 {many}.")), "{many}");
        }
        for text in ["You get 5 Copper Crowns from the body of a guard.", "You get 1 Archon Crown.", "You give 3 conch(es) to a merman.", "You drop 2 weed bags.", "You deposit 36 drachmas into your account with the banker.", "You get 100 Character Coin.", "You withdraw 1 dark coin from your account with the banker.", "You get 7 sanddollars from the body of a crab."] {
            assert!(is_money(text), "{text}");
        }
        assert_eq!(forms("gold coin(s)"), ["gold coin", "gold coins"]);
        assert_eq!(forms("Archon Crown"), ["archon crown", "archon crown"]);
    }
}
