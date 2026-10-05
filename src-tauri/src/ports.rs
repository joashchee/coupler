//! Where Coupler connects (CLAUDE.md rule 2): the host and the games it
//! runs, one per port. The only addresses in Coupler. The desktop dials
//! `HOST` and a port by TCP (`session.rs`); the web build opens the
//! game's own WebSocket on the same host (`web_socket_url`, used by
//! `src-web/`), which CoffeeMUD's web server passes to the same game as
//! the raw telnet stream (`WebMacros/WebSock.java`: `?port=` picks it).

use serde::Serialize;

pub const HOST: &str = "coffeemud.net";

/// One way to play on coffeemud.net: the server runs several games side
/// by side, one per port (docs/coffeemud-ports.md has where each line
/// comes from). `summary` is shown to the player, and read out by a
/// screen reader, so it says what's different in plain words.
#[derive(Serialize, Clone, Copy)]
#[serde(rename_all = "camelCase")]
pub struct Port {
    /// What the frontend passes to `connect`. Never the number.
    pub id: &'static str,
    pub port: u16,
    pub name: &'static str,
    pub summary: &'static str,
    /// Ports with the same world share one saved map.
    #[serde(skip)]
    pub world: &'static str,
}

/// The server also lists Tech (2328), Heroics (2329) and NO (2330). They
/// have no buttons: the game doesn't document them, so there's nothing
/// true to tell a player about them (docs/coffeemud-ports.md).
pub const PORTS: &[Port] = &[
    Port {
        id: "standard",
        port: 23,
        name: "Standard",
        summary: "The main game, and the place to start. Other players can't attack you unless you switch that on. One account holds all your characters.",
        world: "standard",
    },
    Port {
        id: "standard-2323",
        port: 2323,
        name: "Standard, second line",
        summary: "The same game and the same characters as Standard, on port 2323. Use it if your network blocks the first line.",
        world: "standard",
    },
    Port {
        id: "pvp",
        port: 2324,
        name: "Player vs Player",
        summary: "Fighting other players is always on and can't be switched off. No hunger or thirst. Uses your Standard account.",
        world: "standard",
    },
    Port {
        id: "hardcore",
        port: 2325,
        name: "Hardcore",
        summary: "Triple experience for kills, but death is final: a character who dies is deleted. Uses its own account, separate from Standard.",
        world: "hardcore",
    },
    Port {
        id: "roleplay",
        port: 2326,
        name: "Role-Playing",
        summary: "Stay in character: most channels and private tells are off, numbers are shown as words, and you gain levels by training with a guildmaster. One character at a time. Uses your Standard account.",
        world: "standard",
    },
    Port {
        id: "classic",
        port: 2327,
        name: "Classic",
        summary: "The game as it first was: much slower levelling, level 31 at most, fewer classes and races, and no accounts. Each character logs in by its own name.",
        world: "classic",
    },
];

/// The game's WebSocket for a port, for the web build: CoffeeMUD's web
/// server on `HOST`, over TLS, carrying the telnet stream as binary
/// frames. The page's Content-Security-Policy allows only this origin
/// (`web/_headers`).
#[cfg_attr(not(target_arch = "wasm32"), allow(dead_code))]
pub fn web_socket_url(port: &Port) -> String {
    format!("wss://{HOST}/WebSock?port={}", port.port)
}
