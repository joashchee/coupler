//! **Music in the game's journals**, pure and unit-tested: the Music
//! Editor's song written into a journal entry as TMF, the Text Music
//! Format (`neumetik_midi::tmf`), and TMF found in what the game prints.
//!
//! A clan's journal is its forum. Clan Crafting makes "a Journal of
//! <type> <clan>" (`resources/skills/clancraft.txt`, a `GenJournal`
//! whose WRITE is the clan's JOURNAL privilege, `ClanCrafting.java`), and
//! every journal item reads and writes the database journal of its name
//! (`StdJournal.java`, `DBWriteJournal(Name(), ...)`), which
//! `CLANFORUMDATA` (`coffeemud.ini`) also mounts as the clan's web forum.
//! So every copy of the item, and the forum on the web, has the entries.
//!
//! **Writing** (`Poster`): WRITE <journal> (`Skill_Write.java`, a skill
//! that can fail) asks, one line each (`StdJournal`'s `TYP_WRITE`):
//! "Is this a private message (y/N)?" (unless the journal is private or
//! a mailbox, which asks "To whom:"), "Enter a subject: ", then "Enter
//! your message" and ": ", the whole entry one line, no length limit
//! but the database's (a MySQL `text`, 64 KB: `MOST`). "Journal entry
//! added." says it's in. Each answer is sent only once its question is
//! asked, so a WRITE that failed never has an answer taken for a command
//! ("N" is north). A refusal (`REFUSALS`), the usual prompt back after
//! the message without "Journal entry added.", or no word from the game
//! in `WAITS` is a failure, and the game's line the reason.
//!
//! **Reading** (`Finder`): READ prints an entry as "FROM:", "TO  :",
//! "DATE:", "SUBJ: " and the text, through the output filter, which
//! wraps at spaces (`CoffeeFilter.fullOutFilter`). TMF is only letters,
//! digits, `-` and `_` in groups, from `TMF1` to `=`, so a line starting
//! with `TMF1` starts it and the lines up to the `=` are the rest. They
//! aren't shown or said (a screen reader would spell out the code);
//! what was found is.

use std::time::Duration;

use crate::clock::Instant;

/// The longest TMF a journal entry takes: under MySQL's `text`, 65,535 bytes.
pub const MOST: usize = 60_000;
/// How long the game has to ask the next question, or say it's done.
pub const WAITS: Duration = Duration::from_secs(20);
/// What the game says when a WRITE doesn't start (`Skill_Write.java`,
/// `StdAbility.java`'s costs, `StdJournal`'s write mask, an unknown
/// command's "Huh?"): until a question comes, any line with one of
/// these ends it. Chatter in between doesn't.
const REFUSALS: [&str; 11] = [
    "Huh?",
    "You are too stupid to actually write",
    "What would you like to write on?",
    "You don't have that.",
    "You don't see '",
    "You can't write",
    " but mess",
    "You are not allowed to write on",
    "You don't have enough",
    "You can't do that",
    "You are too tired",
];
/// The most lines one TMF runs over before it's taken to be broken off.
const MOST_LINES: usize = 2_000;

// ---- Reading ----

/// What a line of the game's was, for TMF.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Seen {
    /// Not TMF: shown as it is.
    Show,
    /// TMF, with more to come: not shown.
    Hide,
    /// The end of TMF: not shown. The whole text, and the entry's subject.
    Done { text: String, subject: Option<String> },
    /// TMF broken off by a line that isn't any: that line is shown.
    Broken,
}

/// Reads TMF out of the game's lines.
#[derive(Default)]
pub struct Finder {
    /// The last entry's subject ("SUBJ: "), until its TMF or another.
    subject: Option<String>,
    /// The TMF so far, and how many lines.
    text: Option<(String, usize)>,
}

fn is_tmf_word(word: &str) -> bool {
    !word.is_empty() && word.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'_')
}

impl Finder {
    /// A finished line, its text without color. Call it with every line, in order.
    pub fn line(&mut self, line: &str) -> Seen {
        let trimmed = line.trim();
        let words: Vec<&str> = trimmed.split_whitespace().collect();
        if let Some((text, lines)) = self.text.as_mut() {
            if words.iter().all(|w| is_tmf_word(w) || *w == "=") && *lines < MOST_LINES {
                text.push(' ');
                text.push_str(trimmed);
                *lines += 1;
                if words.contains(&"=") {
                    return self.done();
                }
                return Seen::Hide;
            }
            self.text = None;
            return Seen::Broken;
        }
        if let Some(subject) = trimmed.strip_prefix("SUBJ:") {
            self.subject = Some(subject.trim().to_string());
            return Seen::Show;
        }
        if words.first() == Some(&neumetik_midi::tmf::MAGIC) && words[1..].iter().all(|w| is_tmf_word(w) || *w == "=") {
            self.text = Some((trimmed.to_string(), 1));
            if words.contains(&"=") {
                return self.done();
            }
            return Seen::Hide;
        }
        Seen::Show
    }

    fn done(&mut self) -> Seen {
        let (text, _) = self.text.take().unwrap_or_default();
        Seen::Done { text, subject: self.subject.take() }
    }

    /// Whether TMF is being read now.
    #[cfg(test)]
    pub fn reading(&self) -> bool {
        self.text.is_some()
    }

    /// The connection's over: whatever was half read is let go.
    pub fn leave(&mut self) {
        *self = Finder::default();
    }
}

// ---- Writing ----

/// Where the conversation with the game is.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Stage {
    /// WRITE sent: a question comes next, or a refusal.
    Asked,
    /// The y/N answered.
    Public,
    /// The subject sent: the message is asked for next.
    Subject,
    /// The message sent: "Journal entry added." comes next.
    Sent,
}

/// What to do next.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Step {
    /// Send this line, unechoed.
    Send(String),
    /// Finished: written, or why not.
    Done(Result<(), String>),
}

/// Writes TMF into a journal, answering WRITE's questions as they come.
#[derive(Debug)]
pub struct Poster {
    subject: String,
    text: String,
    stage: Stage,
    /// When the game last moved the conversation on.
    since: Instant,
    /// The last line the game said that wasn't a question.
    last: String,
}

/// A subject the game takes as typed: one line, none of the characters
/// its filters change (`CoffeeFilter.simpleInFilter` and the output's),
/// never empty, never a MOTD (`StdJournal` cuts that off).
pub fn subject_of(title: &str) -> String {
    let kept: String = title.chars().filter(|c| !c.is_control() && !"^%\\<>&~`'\"!()".contains(*c)).collect();
    let kept = kept.split_whitespace().collect::<Vec<_>>().join(" ");
    let kept: String = kept.chars().take(60).collect();
    let upper = kept.to_ascii_uppercase();
    if kept.is_empty() || ["MOTD", "MOTM", "MOTY"].iter().any(|m| upper.starts_with(m)) {
        format!("Music {kept}").trim().to_string()
    } else {
        kept
    }
}

impl Poster {
    /// Starts writing `text` (TMF) into `journal` (the item's name as
    /// typed) under `subject`: the poster, and the first line to send.
    pub fn new(journal: &str, subject: &str, text: String, now: Instant) -> Result<(Poster, String), String> {
        let journal = journal.split_whitespace().collect::<Vec<_>>().join(" ");
        if journal.is_empty() {
            return Err("Which journal? Name it as you'd READ it, like journal.".into());
        }
        if text.len() > MOST {
            return Err(format!("The music is {} characters as text, more than a journal entry holds ({MOST}). Make it shorter.", text.len()));
        }
        let poster = Poster { subject: subject_of(subject), text, stage: Stage::Asked, since: now, last: String::new() };
        Ok((poster, format!("write {journal}")))
    }

    /// The text being written, for its size.
    pub fn characters(&self) -> usize {
        self.text.len()
    }

    /// A read's lines (their text without color) and its unfinished
    /// line, the prompt or a question: what to do next, if anything.
    pub fn read(&mut self, lines: &[String], prompt: Option<&str>, now: Instant) -> Option<Step> {
        let mut message_asked = false;
        for line in lines {
            let line = line.trim();
            if line.is_empty() {
                continue;
            }
            if line.contains("Journal entry added.") {
                return Some(Step::Done(Ok(())));
            }
            if line.starts_with("Enter your message") {
                message_asked = true;
                continue;
            }
            if line == "Aborted." || line.starts_with("I'm sorry, there is no such user") {
                return Some(Step::Done(Err(self.failed(line))));
            }
            if self.stage == Stage::Asked && REFUSALS.iter().any(|r| line.contains(r)) {
                return Some(Step::Done(Err(self.failed(line))));
            }
            self.last = line.to_string();
        }
        let prompt = prompt.map(str::trim).unwrap_or("");
        let step = match self.stage {
            _ if prompt.starts_with("To whom") => {
                self.stage = Stage::Sent;
                // An empty name is no one: the game says so and stops.
                return Some(Step::Send(String::new()));
            }
            Stage::Asked if prompt.starts_with("Is this a private message") => {
                self.stage = Stage::Public;
                Step::Send("N".into())
            }
            Stage::Asked | Stage::Public if prompt.starts_with("Enter a subject") => {
                self.stage = Stage::Subject;
                Step::Send(self.subject.clone())
            }
            Stage::Subject if message_asked || prompt == ":" => {
                self.stage = Stage::Sent;
                Step::Send(std::mem::take(&mut self.text))
            }
            // The message sent and the usual prompt back without "Journal entry added.": it didn't take.
            Stage::Sent if !prompt.is_empty() && lines.iter().any(|l| !l.trim().is_empty()) => {
                let last = self.last.clone();
                return Some(Step::Done(Err(self.failed(&last))));
            }
            _ => {
                return (now.saturating_duration_since(self.since) > WAITS).then(|| Step::Done(Err("The game didn't answer the WRITE, so the music wasn't written.".into())));
            }
        };
        self.since = now;
        Some(step)
    }

    /// Whether it's waited too long for the game: a reason to give up.
    pub fn timed_out(&self, now: Instant) -> Option<String> {
        (now.saturating_duration_since(self.since) > WAITS).then(|| "The game didn't answer the WRITE, so the music wasn't written.".into())
    }

    fn failed(&self, said: &str) -> String {
        match self.stage {
            Stage::Sent => format!("The game didn't add the entry: {said}"),
            _ if said.is_empty() => "The game didn't take the WRITE, so the music wasn't written.".into(),
            _ => format!("The music wasn't written: {said}"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn lines(text: &[&str]) -> Vec<String> {
        text.iter().map(|s| s.to_string()).collect()
    }

    #[test]
    fn tmf_is_found_and_hidden_however_it_wraps() {
        let song = crate::music::Song::new();
        let text = crate::music::to_tmf(&song, "Our Song");
        let mut f = Finder::default();
        assert_eq!(f.line("FROM: Bob"), Seen::Show);
        assert_eq!(f.line("SUBJ: Our Song"), Seen::Show);
        let words: Vec<&str> = text.split(' ').collect();
        let (first, rest) = words.split_at(2);
        assert_eq!(f.line(&first.join(" ")), Seen::Hide);
        assert!(f.reading());
        let mut last = Seen::Show;
        for chunk in rest.chunks(1) {
            last = f.line(&chunk.join(" "));
        }
        let Seen::Done { text: found, subject } = last else { panic!("{last:?}") };
        assert_eq!(subject.as_deref(), Some("Our Song"));
        assert_eq!(crate::music::from_tmf(&found).unwrap(), (song, "Our Song".to_string()));
        assert_eq!(f.line("R)eply N)ext or RETURN:"), Seen::Show);
    }

    #[test]
    fn tmf_on_one_line() {
        let text = crate::music::to_tmf(&crate::music::Song::new(), "x");
        let mut f = Finder::default();
        assert!(matches!(f.line(&text), Seen::Done { .. }));
        assert!(!f.reading());
    }

    #[test]
    fn talk_about_tmf_is_shown() {
        let mut f = Finder::default();
        assert_eq!(f.line("Bob says 'TMF1 is neat =' to you."), Seen::Show);
        assert_eq!(f.line("TMF1 is a format, I said."), Seen::Show);
        assert!(!f.reading());
    }

    #[test]
    fn a_line_that_isnt_tmf_breaks_it_off() {
        let mut f = Finder::default();
        assert_eq!(f.line("TMF1 AAAA BBBB"), Seen::Hide);
        assert_eq!(f.line("Bob tells you 'hi'"), Seen::Broken);
        assert!(!f.reading());
        assert_eq!(f.line("CCCC ="), Seen::Show);
    }

    #[test]
    fn writing_answers_each_question_as_its_asked() {
        let t = Instant::now();
        let (mut p, first) = Poster::new("  clan   journal ", "Our Song", "TMF1 AAAA =".into(), t).unwrap();
        assert_eq!(first, "write clan journal");
        // Nothing yet.
        assert_eq!(p.read(&[], None, t), None);
        assert_eq!(p.read(&[], Some("Is this a private message (y/N)? "), t), Some(Step::Send("N".into())));
        assert_eq!(p.read(&lines(&["N"]), Some("Enter a subject: "), t), Some(Step::Send("Our Song".into())));
        assert_eq!(p.read(&lines(&["Enter your message"]), Some(": "), t), Some(Step::Send("TMF1 AAAA =".into())));
        assert_eq!(p.characters(), 0, "sent once");
        assert_eq!(p.read(&lines(&["Journal entry added."]), Some("<100Hp 50m 80mv> "), t), Some(Step::Done(Ok(()))));
    }

    #[test]
    fn a_write_that_fails_sends_nothing_more() {
        let t = Instant::now();
        let (mut p, _) = Poster::new("journal", "x", "TMF1 =".into(), t).unwrap();
        let step = p.read(&lines(&["You attempt to write on a journal, but mess up."]), Some("<100Hp 50m 80mv> "), t);
        assert_eq!(step, Some(Step::Done(Err("The music wasn't written: You attempt to write on a journal, but mess up.".into()))));
        let (mut p, _) = Poster::new("journal", "x", "TMF1 =".into(), t).unwrap();
        // Chatter first isn't a refusal.
        assert_eq!(p.read(&lines(&["Alice says 'hello'"]), Some("> "), t), None);
        let step = p.read(&lines(&["You are not allowed to write on a Journal of Clan Foo"]), Some("> "), t);
        assert!(matches!(step, Some(Step::Done(Err(_)))));
        // The entry not added.
        let (mut p, _) = Poster::new("journal", "x", "TMF1 =".into(), t).unwrap();
        p.read(&[], Some("Enter a subject: "), t);
        p.read(&lines(&["Enter your message"]), Some(": "), t);
        assert!(matches!(p.read(&lines(&["Something went wrong."]), Some("> "), t), Some(Step::Done(Err(_)))));
        // A private journal: no one's named, so the game stops.
        let (mut p, _) = Poster::new("journal", "x", "TMF1 =".into(), t).unwrap();
        assert_eq!(p.read(&[], Some("To whom:"), t), Some(Step::Send(String::new())));
        assert!(matches!(p.read(&lines(&["I'm sorry, there is no such user."]), Some("> "), t), Some(Step::Done(Err(_)))));
    }

    #[test]
    fn a_silent_game_times_out() {
        let t = Instant::now();
        let (mut p, _) = Poster::new("journal", "x", "TMF1 =".into(), t).unwrap();
        assert_eq!(p.timed_out(t + Duration::from_secs(5)), None);
        assert!(p.timed_out(t + WAITS + Duration::from_secs(1)).is_some());
        assert!(matches!(p.read(&[], None, t + WAITS + Duration::from_secs(1)), Some(Step::Done(Err(_)))));
    }

    #[test]
    fn too_long_or_nowhere() {
        let t = Instant::now();
        assert!(Poster::new("journal", "x", "x".repeat(MOST + 1), t).is_err());
        assert!(Poster::new("  ", "x", "TMF1 =".into(), t).is_err());
    }

    #[test]
    fn subjects_the_game_keeps() {
        assert_eq!(subject_of("Bob's \"Song\" (live)!"), "Bobs Song live");
        assert_eq!(subject_of("MOTD tune"), "Music MOTD tune");
        assert_eq!(subject_of("  "), "Music");
        assert_eq!(subject_of(&"a".repeat(100)).len(), 60);
    }
}
