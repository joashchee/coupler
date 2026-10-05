//! Telling when the player leaves their character while still connected:
//! CoffeeMUD's LOGOUT (or LOGOFF), back to the account menu, and SWITCH,
//! to another character of the account. Pure and unit-tested; `session.rs`
//! feeds it and, when one happens, stops what the old character set off.
//!
//! CoffeeMUD sends no GMCP when a character leaves (DefaultSession.logout
//! only takes the MOB out of the game), so each is read from what the
//! game itself does, never from the command typed, which can be refused
//! ("You can't switch to 'x'.", "You must wait a few more minutes...")
//! or abbreviated:
//!
//! - **LOGOUT** asks first: `Logout -- are you sure (y/N)?`
//!   (Commands/Logoff.java, a CONFIRM prompt, so a line starting with Y
//!   says yes). That prompt as the unfinished line, then a yes sent, is a
//!   logout. Only a prompt that's new arms it: a "no" leaves it on screen
//!   until the next line, and the command after that mustn't count.
//! - **SWITCH** (Commands/Switch.java) brings a different character in on
//!   the same session, and the game's `char.base` then names them: at
//!   once for one not in the game (`GMCP_PING_ALL`), within its ~16 s
//!   poll for a live switch. A `char.base` with another name than the
//!   last is a switch. The first after connecting, or after a logout,
//!   is just the character coming in.

/// The game's question before a logout (Logoff.java, `promptPrint`).
const LOGOUT_PROMPT: &str = "Logout -- are you sure (y/N)?";

/// How the player left their character.
#[derive(Debug, Clone, PartialEq)]
pub enum Left {
    /// Back to the account menu (or the login), still connected.
    Logout,
    /// Now playing another character: their name, as `char.base` gave it.
    Switch(String),
}

#[derive(Default)]
pub struct Character {
    /// The character playing now, from `char.base`; None until one comes in.
    name: Option<String>,
    /// The unfinished line last seen, so only a prompt that's new arms a logout.
    last_partial: Option<String>,
    /// The game has just asked whether to log out.
    logout_asked: bool,
}

impl Character {
    /// The unfinished line after a read (the prompt line), or None.
    pub fn partial(&mut self, text: Option<&str>) {
        if text == self.last_partial.as_deref() {
            return;
        }
        self.last_partial = text.map(str::to_string);
        if text.is_some_and(|t| t.contains(LOGOUT_PROMPT)) {
            self.logout_asked = true;
        }
    }

    /// A line the player sent: a yes to the logout question is a logout.
    pub fn sent(&mut self, line: &str) -> Option<Left> {
        let asked = std::mem::take(&mut self.logout_asked);
        if asked && line.trim_start().starts_with(['y', 'Y']) {
            self.name = None;
            return Some(Left::Logout);
        }
        None
    }

    /// A GMCP message: a `char.base` naming someone else is a switch.
    pub fn gmcp(&mut self, package: &str, data: &str) -> Option<Left> {
        if !package.eq_ignore_ascii_case("char.base") {
            return None;
        }
        let value: serde_json::Value = serde_json::from_str(data).ok()?;
        let name = value.get("name")?.as_str()?.trim();
        if name.is_empty() {
            return None;
        }
        match self.name.replace(name.to_string()) {
            Some(was) if !was.eq_ignore_ascii_case(name) => Some(Left::Switch(name.to_string())),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const PROMPT: &str = "Logout -- are you sure (y/N)?";

    #[test]
    fn a_yes_to_the_logout_question_is_a_logout() {
        let mut c = Character::default();
        c.partial(Some(PROMPT));
        assert_eq!(c.sent(" yes"), Some(Left::Logout));
        // Asked once, answered once.
        assert_eq!(c.sent("y"), None);
    }

    #[test]
    fn a_no_or_another_line_is_not() {
        let mut c = Character::default();
        c.partial(Some(PROMPT));
        assert_eq!(c.sent("n"), None);
        // The same prompt still on screen doesn't ask again: "yell" is a command.
        c.partial(Some(PROMPT));
        assert_eq!(c.sent("yell hello"), None);
        // Without the question, a y is just a y.
        c.partial(Some("<10Hp 20m 30mv> "));
        assert_eq!(c.sent("y"), None);
    }

    #[test]
    fn the_logout_question_asked_again_counts_again() {
        let mut c = Character::default();
        c.partial(Some(PROMPT));
        assert_eq!(c.sent("n"), None);
        c.partial(Some("<10Hp 20m 30mv> "));
        c.partial(Some(PROMPT));
        assert_eq!(c.sent("Y"), Some(Left::Logout));
    }

    #[test]
    fn another_name_in_char_base_is_a_switch() {
        let mut c = Character::default();
        // The first is just coming in.
        assert_eq!(c.gmcp("Char.Base", r#"{"name":"Ada","class":"Fighter"}"#), None);
        assert_eq!(c.gmcp("char.base", r#"{"name":"Ada","class":"Fighter"}"#), None);
        assert_eq!(c.gmcp("char.base", r#"{"name":"Bo"}"#), Some(Left::Switch("Bo".into())));
        assert_eq!(c.gmcp("char.vitals", r#"{"name":"Cy"}"#), None);
        assert_eq!(c.gmcp("char.base", "not json"), None);
    }

    #[test]
    fn after_a_logout_the_next_character_is_just_coming_in() {
        let mut c = Character::default();
        c.gmcp("char.base", r#"{"name":"Ada"}"#);
        c.partial(Some(PROMPT));
        assert_eq!(c.sent("y"), Some(Left::Logout));
        assert_eq!(c.gmcp("char.base", r#"{"name":"Bo"}"#), None);
    }
}
