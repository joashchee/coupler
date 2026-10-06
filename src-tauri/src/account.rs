//! **The account menu**: CoffeeMUD's account menu (an account's
//! characters, and what can be done with the account) read into a form
//! the frontend shows as a dialog and the narrator says in a few words
//! (components/AccountMenuDialog.tsx), instead of a screen of text and a
//! command typed. Pure and unit-tested; `session.rs` tells it each line
//! sent and gives it each read's lines and unfinished line.
//!
//! What the game sends (`Libraries/CharCreation.java`, `acctmenu*`):
//!
//! - The menu (`resources/help/acctmenu.txt`) unless the account turned
//!   it off (MENU): ` Account Menu`, then a line a letter and `)` each,
//!   `L)ist characters`, `N)ew character (50 remaining)`, `I)mport
//!   character` and `E)xport character` (only with CANEXPORT), `D)elete/
//!   Retire character`, `H)elp`, `M)enu OFF`, `P)assword change`,
//!   `E)mail change` (only where e-mail isn't disabled), `Q)uit
//!   (logout)`, and ` (Enter your character name to login)`.
//! - The characters only when asked (`L`, or with the menu off): the
//!   Account command's list (`Commands/Account.java`, `getAccountList`
//!   with `NAME,LAST,REMAIN`), a header `Name  Race  Level  Class  Last
//!   Remain` (fields a game has disabled left out), a row per character
//!   padded to the header's columns, `*` after one playing now, then
//!   `Total hours played: N`.
//! - The prompt, `Command or Name (?): ` (`acctmenuPrompt`).
//!
//! The menu is open while that prompt is the unfinished line; any other
//! question (a y/N, a new character's name, a password) closes it. Each
//! time it opens without a list read, `wants_list` says to ask for one,
//! once, so the dialog can name the characters to pick from.

use serde::Serialize;

/// One of the account's characters, as the list shows it.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Character {
    pub name: String,
    pub race: String,
    pub level: String,
    pub class: String,
    /// When last played, as the game writes it, or empty.
    pub last: String,
    /// Days left before an idle character is purged, or empty.
    pub remain: String,
    /// Playing now (on another connection).
    pub online: bool,
}

/// The account menu as the dialog shows it.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Menu {
    /// The characters, once a list was read; None before.
    pub characters: Option<Vec<Character>>,
    /// How many more characters the account may have ("50", "Unlimited").
    pub remaining: Option<String>,
    /// What the game offers beside playing (the menu's letters), when
    /// the menu was shown: import, export, email.
    pub import: bool,
    pub export: bool,
    pub email: bool,
    pub total_hours: Option<u32>,
}

/// What changed.
#[derive(Debug, Clone, PartialEq)]
pub enum Update {
    Open(Menu),
    Closed,
}

#[derive(Default)]
pub struct Account {
    menu: Option<Menu>,
    /// The last menu read, kept while a question is asked in between.
    known: Option<Menu>,
    /// Asked for the list since the menu last opened.
    asked: bool,
    /// The list being read: its columns (name, start) and rows so far.
    columns: Option<Vec<(String, usize)>>,
    rows: Vec<Character>,
}

/// The menu's prompt, `Command or Name (?): `.
pub fn is_prompt(partial: &str) -> bool {
    partial.trim_start().to_lowercase().starts_with("command or name")
}

/// The list's header: each column's name and where it starts.
fn header(text: &str) -> Option<Vec<(String, usize)>> {
    let mut columns = Vec::new();
    let mut at = 0;
    for word in text.split(' ') {
        if !word.is_empty() {
            columns.push((word.to_string(), at));
        }
        at += word.chars().count() + 1;
    }
    let names: Vec<&str> = columns.iter().map(|(n, _)| n.as_str()).collect();
    (names.first() == Some(&"Name") && names.len() >= 2 && names.iter().all(|n| ["Name", "Race", "Level", "Class", "Last", "Remain"].contains(n))).then_some(columns)
}

/// A row cut at the header's columns.
fn row(text: &str, columns: &[(String, usize)]) -> Option<Character> {
    let chars: Vec<char> = text.chars().collect();
    let cell = |i: usize| {
        let start = columns[i].1.min(chars.len());
        let end = columns.get(i + 1).map_or(chars.len(), |c| c.1.min(chars.len()));
        chars[start..end].iter().collect::<String>().trim().to_string()
    };
    let name = cell(0);
    if name.is_empty() || !name.chars().all(char::is_alphanumeric) {
        return None;
    }
    let mut c = Character { name, race: String::new(), level: String::new(), class: String::new(), last: String::new(), remain: String::new(), online: false };
    for (i, (column, _)) in columns.iter().enumerate().skip(1) {
        let mut value = cell(i);
        if i + 1 == columns.len() {
            // After the last column: `*` playing now, `(tells)`, `(email)`, `(post)`.
            c.online = value.contains('*');
            value = value.split(['*', '(']).next().unwrap_or("").trim().to_string();
        }
        match column.as_str() {
            "Race" => c.race = value,
            "Level" => c.level = value,
            "Class" => c.class = value,
            "Last" => c.last = value,
            "Remain" => c.remain = value,
            _ => {}
        }
    }
    Some(c)
}

impl Account {
    /// A read's finished lines (plain text) and its unfinished one.
    /// Returns the menu when it opens or changes, Closed when it closes.
    pub fn lines(&mut self, texts: &[String], partial: Option<&str>) -> Option<Update> {
        let mut menu = self.known.clone().unwrap_or(Menu { characters: None, remaining: None, import: false, export: false, email: false, total_hours: None });
        let mut seen = false;
        for text in texts {
            let t = text.trim();
            if let Some(columns) = header(t) {
                self.columns = Some(columns);
                self.rows.clear();
                continue;
            }
            if let Some(columns) = self.columns.clone() {
                if let Some(hours) = t.strip_prefix("Total hours played:") {
                    menu.characters = Some(std::mem::take(&mut self.rows));
                    menu.total_hours = hours.trim().parse().ok();
                    self.columns = None;
                    seen = true;
                    continue;
                }
                if let Some(c) = row(text.trim_end(), &columns) {
                    self.rows.push(c);
                    continue;
                }
                if !t.is_empty() {
                    self.columns = None;
                }
            }
            if t.eq_ignore_ascii_case("account menu") {
                menu.import = false;
                menu.export = false;
                menu.email = false;
                seen = true;
            }
            // `N)ew character (50 remaining)`, `I)mport character`, …
            let mut cs = t.chars();
            if let (Some(letter), Some(')')) = (cs.next(), cs.next()) {
                let rest = cs.as_str().trim().to_lowercase();
                match letter {
                    'N' if rest.starts_with("ew character") => {
                        menu.remaining = t.split('(').nth(1).and_then(|r| r.split_whitespace().next()).map(str::to_string);
                        seen = true;
                    }
                    'I' if rest.starts_with("mport") => menu.import = true,
                    'E' if rest.starts_with("xport") => menu.export = true,
                    'E' if rest.starts_with("mail") => menu.email = true,
                    _ => {}
                }
            }
        }
        let open = partial.is_some_and(is_prompt);
        if open {
            let changed = seen || self.menu.is_none();
            self.known = Some(menu.clone());
            self.menu = Some(menu.clone());
            return changed.then_some(Update::Open(menu));
        }
        if seen {
            self.known = Some(menu);
        }
        if partial.is_some_and(|p| !p.trim().is_empty()) && self.menu.take().is_some() {
            return Some(Update::Closed);
        }
        None
    }

    /// Whether to ask for the list now: the menu's open, no list was read
    /// this time, and none asked yet. Counts as asked when it says yes.
    pub fn wants_list(&mut self) -> bool {
        let Some(menu) = &self.menu else { return false };
        if menu.characters.is_some() || self.asked {
            return false;
        }
        self.asked = true;
        true
    }

    /// A character came into the game: the menu is gone till a logout.
    pub fn entered(&mut self) -> bool {
        let was = self.menu.is_some();
        self.menu = None;
        self.known = None;
        self.asked = false;
        was
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const PROMPT: &str = "Command or Name (?): ";

    fn strings(lines: &[&str]) -> Vec<String> {
        lines.iter().map(|s| s.to_string()).collect()
    }

    const MENU: &[&str] = &[
        "",
        "Welcome back to CoffeeMud!",
        " Account Menu",
        " L)ist characters",
        " N)ew character (48 remaining)",
        " I)mport character",
        " D)elete/Retire character",
        " H)elp",
        " M)enu OFF",
        " P)assword change",
        " E)mail change",
        " Q)uit (logout)",
        "",
        " (Enter your character name to login)",
    ];

    #[test]
    fn the_menu_opens_and_asks_for_the_list_once() {
        let mut a = Account::default();
        let Some(Update::Open(menu)) = a.lines(&strings(MENU), Some(PROMPT)) else { panic!("the menu should open") };
        assert_eq!(menu.remaining.as_deref(), Some("48"));
        assert!(menu.import && menu.email && !menu.export);
        assert_eq!(menu.characters, None);
        assert!(a.wants_list());
        assert!(!a.wants_list());
    }

    #[test]
    fn reads_the_characters() {
        let mut a = Account::default();
        a.lines(&strings(MENU), Some(PROMPT));
        let list = strings(&[
            "Name   Race   Level Class    Last                  Remain ",
            "Bob    Human  12    Fighter  10-03-2026 9:14pm            ",
            "Ann    Elf    3     Mage     09-30-2026 1:02am     12 days *",
            "",
            "Total hours played: 41",
        ]);
        let Some(Update::Open(menu)) = a.lines(&list, Some(PROMPT)) else { panic!("the list should update the menu") };
        let chars = menu.characters.unwrap();
        assert_eq!(chars.len(), 2);
        assert_eq!((chars[0].name.as_str(), chars[0].race.as_str(), chars[0].level.as_str(), chars[0].class.as_str()), ("Bob", "Human", "12", "Fighter"));
        assert_eq!(chars[0].last, "10-03-2026 9:14pm");
        assert!(!chars[0].online);
        assert_eq!((chars[1].remain.as_str(), chars[1].online), ("12 days", true));
        assert_eq!(menu.total_hours, Some(41));
        // Read: not asked for again, and the menu's letters are still known.
        assert!(!a.wants_list());
        assert!(menu.import);
    }

    #[test]
    fn a_question_closes_it_and_the_menu_comes_back() {
        let mut a = Account::default();
        a.lines(&strings(MENU), Some(PROMPT));
        assert_eq!(a.lines(&[], Some("Quit -- are you sure (y/N)?")), Some(Update::Closed));
        // Back at the prompt (N): opens again, its letters remembered.
        let Some(Update::Open(menu)) = a.lines(&strings(&["Aborted."]), Some(PROMPT)) else { panic!("should reopen") };
        assert!(menu.email);
        // The same prompt again, nothing new: no update.
        assert_eq!(a.lines(&[], Some(PROMPT)), None);
    }

    #[test]
    fn with_menus_off_only_the_list_shows() {
        let mut a = Account::default();
        let list = strings(&["Name Level ", "Zed  7     ", "Total hours played: 2"]);
        let Some(Update::Open(menu)) = a.lines(&list, Some(PROMPT)) else { panic!("should open") };
        assert_eq!(menu.characters.unwrap()[0].level, "7");
        assert_eq!(menu.remaining, None);
        assert!(!a.wants_list());
    }

    #[test]
    fn the_game_isnt_the_menu() {
        let mut a = Account::default();
        assert_eq!(a.lines(&strings(&["Name: Bob"]), Some("<10hp> ")), None);
        a.lines(&strings(MENU), Some(PROMPT));
        assert!(a.entered());
        assert!(!a.wants_list());
    }
}
