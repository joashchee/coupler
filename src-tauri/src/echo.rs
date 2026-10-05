//! **The commands' one-line answers**, which Immersive's narrator says in
//! short instead of writing them. Pure and unit-tested; `session.rs`
//! tells it each line the player sends and gives it each read's lines
//! after `speech.rs` has kinded them.
//!
//! The study behind it is docs/coffeemud-commands.md: every command in
//! CoffeeMUD's `Commands/` (snapshot `c1e556f`), those that print a block
//! (lists, tables, boxes, columns, a room, help text, a menu, or start
//! something that does, like a move or a fight) and those that answer in
//! one line. Only the second kind is here: [`COMMANDS`], each with the
//! words that call it.
//!
//! How a reply is told: CoffeeMUD answers each command, then sends its
//! prompt. Each line sent queues the command its first word names
//! (exactly one of its access words, the server's own first, exact pass,
//! `EnglishParser.findCommand`; an abbreviation that only the inexact
//! pass would find isn't guessed at), and the next read that has lines of
//! the game's takes it off the queue:
//!
//! - A line (or a line the game wrapped, joined back up) matching one of
//!   the command's [`ECHOES`], or one every command shares, is an echo.
//! - Otherwise, when the reply is a single line and the prompt came back
//!   after it, that line is an echo too, said as the game wrote it.
//!
//! Anything else (a block, a line that came on its own while waiting)
//! stays as it is. Talk (`say`, `tell`) is the journal's, never an echo.

use std::collections::VecDeque;
use std::time::Duration;

use serde::Serialize;

use crate::clock::Instant;
use crate::speech::LineKind;

/// How long a command waits for its answer.
const WAITS: Duration = Duration::from_secs(5);
/// Commands waiting at most; older is dropped first.
const KEPT: usize = 8;
/// A wrapped answer spans at most this many lines.
const WRAPPED: usize = 3;

/// A command that answers in one line, and the words that call it.
#[derive(Debug, Serialize)]
pub struct Command {
    /// CoffeeMUD's class name (`Commands/Sit.java`).
    pub name: &'static str,
    pub words: &'static [&'static str],
}

/// A one-line answer and what the narrator says for it.
#[derive(Debug, Serialize)]
pub struct Echo {
    /// Stable: the player's own words for it are kept by it.
    pub id: &'static str,
    /// The commands that answer with it; none means any of [`COMMANDS`].
    pub commands: &'static [&'static str],
    /// The line as the player sees it, `*` for a name or a number.
    pub line: &'static str,
    /// What the narrator says: `{1}`, `{2}` are the line's `*`s.
    pub says: &'static str,
}

macro_rules! commands {
    ($($name:literal: [$($word:literal),* $(,)?]),* $(,)?) => {
        &[$(Command { name: $name, words: &[$($word),*] }),*]
    };
}

macro_rules! echoes {
    ($($id:literal [$($cmd:literal),*] $line:literal => $says:literal),* $(,)?) => {
        &[$(Echo { id: $id, commands: &[$($cmd),*], line: $line, says: $says }),*]
    };
}

/// The player commands that answer in one line (docs/coffeemud-commands.md,
/// "One line"), less talk and those that only ask a question.
pub const COMMANDS: &[Command] = commands! {
    "AFK": ["AFK"],
    "ANSI": ["ANSI", "COLOR", "COLOUR"],
    "Activate": ["ACTIVATE", "ACT", "A", ">"],
    "AutoAssist": ["AUTOASSIST"],
    "AutoAttack": ["AUTOATTACK"],
    "AutoDraw": ["AUTODRAW"],
    "AutoExits": ["AUTOEXITS"],
    "AutoGold": ["AUTOGOLD"],
    "AutoGuard": ["AUTOGUARD", "GUARD"],
    "AutoImprovement": ["AUTOIMPROVEMENT"],
    "AutoLoot": ["AUTOLOOT"],
    "AutoMap": ["AUTOMAP"],
    "AutoMelee": ["AUTOMELEE"],
    "AutoNotify": ["AUTONOTIFY"],
    "AutoRun": ["AUTORUN"],
    "AutoTellNotify": ["AUTOTELLNOTIFY"],
    "AutoWeather": ["AUTOWEATHER"],
    "Autoforward": ["AUTOFORWARD"],
    "Bid": ["BID"],
    "Borrow": ["BORROW"],
    "Brief": ["BRIEF"],
    "Buy": ["BUY"],
    "ClanAccept": ["CLANACCEPT"],
    "ClanApply": ["CLANAPPLY"],
    "ClanAssign": ["CLANASSIGN"],
    "ClanDeclare": ["CLANDECLARE"],
    "ClanDonateSet": ["CLANDONATESET"],
    "ClanExile": ["CLANEXILE"],
    "ClanHomeSet": ["CLANHOMESET"],
    "ClanMorgueSet": ["CLANMORGUESET"],
    "ClanReject": ["CLANREJECT"],
    "Close": ["CLOSE", "CLOS", "CLO", "CL"],
    "Compare": ["COMPARE", "COMP"],
    "Compress": ["COMPRESS"],
    "Consider": ["CONSIDER"],
    "Deactivate": ["DEACTIVATE", "DEACT", "DEA", "<"],
    "Deposit": ["DEPOSIT"],
    "DieRoll": ["DIEROLL", "DROLL"],
    "Dig": ["DIG"],
    "Disembark": ["DISEMBARK"],
    "Dismount": ["DISMOUNT"],
    "Display": ["DISPLAY", "SHOW"],
    "Draw": ["DRAW"],
    "Dress": ["DRESS"],
    "DrinkCmd": ["DRINK", "DR", "DRI"],
    "Drop": ["DROP", "DRO"],
    "Duel": ["DUEL"],
    "Eat": ["EAT"],
    "Emote": ["EMOTE", ",", ";", ":"],
    "Empty": ["EMPTY", "EMP"],
    "Feed": ["FEED"],
    "Fill": ["FILL"],
    "Follow": ["FOLLOW", "FOL", "FO", "F"],
    "Friends": ["FRIENDS"],
    "Gait": ["GAIT", "NOGAIT"],
    "Get": ["GET", "G"],
    "Give": ["GIVE", "GI"],
    "Hold": ["HOLD", "HOL", "HO", "H"],
    "Ignore": ["IGNORE"],
    "Knock": ["KNOCK"],
    "LineWrap": ["LINEWRAP"],
    "Lock": ["LOCK", "LOC"],
    "MXP": ["MXP"],
    "Mend": ["MEND"],
    "Mood": ["MOOD"],
    "Mount": ["MOUNT", "BOARD", "RIDE", "M"],
    "NOMXP": ["NOMXP"],
    "NoANSI": ["NOANSI", "NOCOLOR", "NOCOLOUR"],
    "NoBattleSpam": ["NOBATTLESPAM"],
    "NoFollow": ["NOFOLLOW", "NOFOL"],
    "NoSell": ["NOSELL"],
    "NoSounds": ["NOSOUNDS", "NOMSP"],
    "NoSpam": ["NOSPAM"],
    "NoTeach": ["NOTEACH"],
    "Open": ["OPEN", "OP", "O"],
    "Order": ["ORDER"],
    "Package": ["PACKAGE"],
    "PageBreak": ["PAGEBREAK"],
    "Pay": ["PAY"],
    "Pose": ["POSE", "NOPOSE"],
    "Pour": ["POUR"],
    "Practice": ["PRACTICE", "PRAC"],
    "Put": ["PUT", "PU", "P"],
    "Quiet": ["QUIET"],
    "Rebuke": ["REBUKE"],
    "Remove": ["REMOVE", "REM"],
    "Request": ["REQUEST"],
    "Sell": ["SELL"],
    "Serve": ["SERVE"],
    "Sheath": ["SHEATH"],
    "Sit": ["SIT", "REST", "R"],
    "Sleep": ["SLEEP", "SL"],
    "Sniff": ["SNIFF", "SMELL"],
    "Sounds": ["SOUNDS", "MSP"],
    "Split": ["SPLIT"],
    "Stand": ["STAND", "ST", "STA", "STAN"],
    "Take": ["TAKE"],
    "Teach": ["TEACH"],
    "Throw": ["THROW", "TOSS"],
    "TypeCmd": ["TYPE", "="],
    "Undress": ["UNDRESS"],
    "Unlock": ["UNLOCK", "UNL", "UN"],
    "Visible": ["VISIBLE", "VIS"],
    "Wear": ["WEAR"],
    "Weather": ["WEATHER"],
    "Wield": ["WIELD"],
    "Wimpy": ["WIMPY"],
    "Withdraw": ["WITHDRAW"],
    "CommandJournal": ["BUG", "IDEA", "TYPO", "TASK", "STUCK"],
};

/// What the narrator says for each answer, the shortest that keeps its
/// facts. In order: the first that matches wins, so a longer form comes
/// before a shorter one it contains. Lines from the commands' own source
/// (`L("...")` as the doer sees it: `<S-NAME>` is "You", `(s)` dropped).
pub const ECHOES: &[Echo] = echoes! {
    // ---- Shared by many ----
    "any.not-here" [] "You don't see '*' here." => "No {1} here.",
    "any.not-here-2" [] "I don't see '*' here." => "No {1} here.",
    "any.not-here-3" [] "I don't see * here." => "No {1} here.",
    "any.no-one-here" [] "I don't see anyone called * here." => "No {1} here.",
    "any.not-have" [] "You don't seem to have '*'." => "You have no {1}.",
    "any.not-carrying" [] "You don't seem to be carrying that." => "Not carrying that.",
    "any.remove-first" [] "You must remove that first." => "Remove it first.",
    "any.toggle-bad" [] "Illegal * argument: '*'. Try ON or OFF, or nothing to toggle." => "{1} takes on or off.",
    "any.not-implemented" [] "This command is not implemented." => "Not implemented.",
    "any.not-shopkeeper" [] "* is not a shopkeeper!" => "{1} isn't a shopkeeper.",
    "any.no-sale" [] "* doesn't appear to have any '*' for sale. Try LIST." => "{1} has no {2}.",
    "any.not-available" [] "That doesn't appear to be available. Try LIST." => "Not available.",
    "any.dont-know" [] "You don't seem to know *." => "You don't know {1}.",

    // ---- Away, color, sound ----
    "afk.on" ["AFK"] "You are now listed as AFK." => "Away.",
    "afk.off" ["AFK"] "You are no longer AFK." => "Back.",
    "afk.missed" ["AFK"] "You missed: *." => "Missed {1}.",
    "ansi.again" ["ANSI"] "ANSI * colour re-enabled." => "{1} color on.",
    "ansi.on" ["ANSI"] "ANSI * colour enabled." => "{1} color on.",
    "ansi.already" ["ANSI"] "ANSI * color is already enabled." => "{1} color already on.",
    "ansi.already-on" ["ANSI"] "ANSI is already enabled." => "Color already on.",
    "noansi.off" ["NoANSI"] "ANSI colour disabled." => "Color off.",
    "noansi.already" ["NoANSI"] "ANSI is already disabled." => "Color already off.",
    "msp.on" ["Sounds"] "MSP Sound/Music enabled." => "Game sound on.",
    "msp.forced" ["Sounds"] "MSP Sound/Music has been forceably enabled." => "Game sound forced on.",
    "msp.already" ["Sounds"] "MSP Sound/Music is already enabled." => "Game sound already on.",
    "msp.none" ["Sounds"] "Your client does not appear to support MSP." => "No game sound here.",
    "nomsp.off" ["NoSounds"] "MSP Sound/Music disabled." => "Game sound off.",
    "nomsp.already" ["NoSounds"] "MSP Sound/Music already disabled." => "Game sound already off.",
    "mxp.on" ["MXP"] "MXP codes enabled." => "MXP on.",
    "mxp.already" ["MXP"] "MXP codes are already enabled." => "MXP already on.",
    "mxp.none" ["MXP"] "Your client does not appear to support MXP." => "No MXP here.",
    "nomxp.off" ["NOMXP"] "MXP codes are disabled." => "MXP off.",
    "nomxp.already" ["NOMXP"] "MXP codes are already disabled." => "MXP already off.",

    // ---- The AUTO settings ----
    "autoassist.on" ["AutoAssist"] "Autoassist has been turned on." => "Autoassist on.",
    "autoassist.off" ["AutoAssist"] "Autoassist has been turned off." => "Autoassist off.",
    "autoattack.on" ["AutoAttack"] "Auto-attack has been turned on." => "Autoattack on.",
    "autoattack.off" ["AutoAttack"] "Auto-attack has been turned off." => "Autoattack off.",
    "autoattack.moot" ["AutoAttack"] "Because of the combat system, this command doesn't really apply." => "Doesn't apply here.",
    "autodraw.on" ["AutoDraw"] "Auto weapon drawing has been turned on. *" => "Autodraw on.",
    "autodraw.off" ["AutoDraw"] "Auto weapon drawing has been turned off. *" => "Autodraw off.",
    "autoexits.on" ["AutoExits"] "Autoexits has been turned on." => "Autoexits on.",
    "autoexits.off" ["AutoExits"] "Autoexits has been turned off." => "Autoexits off.",
    "autogold.on" ["AutoGold"] "Autogold has been turned on." => "Autogold on.",
    "autogold.off" ["AutoGold"] "Autogold has been turned off." => "Autogold off.",
    "autoguard.on" ["AutoGuard"] "You are now on guard. You will no longer follow group leaders." => "On guard.",
    "autoguard.off" ["AutoGuard"] "You are no longer on guard. You will now follow group leaders." => "Off guard.",
    "autoimprove.on" ["AutoImprovement"] "Skill improvement notifications are now on." => "Skill notices on.",
    "autoimprove.off" ["AutoImprovement"] "Skill improvement notifications are now off." => "Skill notices off.",
    "autoloot.on" ["AutoLoot"] "Autolooting has been turned on." => "Autoloot on.",
    "autoloot.off" ["AutoLoot"] "Autolooting has been turned off." => "Autoloot off.",
    "automap.on" ["AutoMap"] "Automap has been turned on." => "Automap on.",
    "automap.off" ["AutoMap"] "Automap has been turned off." => "Automap off.",
    "automelee.on" ["AutoMelee"] "Automelee has been turned on. *" => "Automelee on.",
    "automelee.off" ["AutoMelee"] "Automelee has been turned off. *" => "Automelee off.",
    "autonotify.on" ["AutoNotify"] "Notification of the arrival of your FRIENDS is now on." => "Friend alerts on.",
    "autonotify.off" ["AutoNotify"] "Notification of the arrival of your FRIENDS is now off." => "Friend alerts off.",
    "autorun.on" ["AutoRun"] "Auto-Run has been turned on." => "Autorun on.",
    "autorun.off" ["AutoRun"] "Auto-Run has been turned off." => "Autorun off.",
    "autotell.on" ["AutoTellNotify"] "Auto Tell Notify has been turned on." => "Tell alerts on.",
    "autotell.off" ["AutoTellNotify"] "Auto Tell Notify has been turned off." => "Tell alerts off.",
    "autoweather.on" ["AutoWeather"] "Weather descriptions are now on." => "Weather on.",
    "autoweather.off" ["AutoWeather"] "Weather descriptions are now off." => "Weather off.",
    "autoforward.on" ["Autoforward"] "Autoemail forwarding has been turned on." => "Mail forwarding on.",
    "autoforward.off" ["Autoforward"] "Autoemail forwarding has been turned off." => "Mail forwarding off.",
    "autoforward.none" ["Autoforward"] "This feature is not activated." => "Not available.",

    // ---- Other settings ----
    "brief.on" ["Brief"] "Brief room descriptions are now on." => "Brief on.",
    "brief.off" ["Brief"] "Brief room descriptions are now off." => "Brief off.",
    "compress.on" ["Compress"] "Compressed views are now active." => "Compressed on.",
    "compress.already-on" ["Compress"] "Compressed views are already active." => "Already compressed.",
    "compress.off" ["Compress"] "Compressed views are now inactive." => "Compressed off.",
    "compress.already-off" ["Compress"] "Compressed views are already inactive." => "Already uncompressed.",
    "linewrap.ask" ["LineWrap"] "Change your line wrap to what? Your current line wrap setting is: *. *" => "Line wrap is {1}.",
    "linewrap.bad" ["LineWrap"] "'*' is not a valid setting. Enter a number larger than 10 or 'disable'." => "{1} isn't valid.",
    "linewrap.set" ["LineWrap"] "Your new line wrap setting is: *." => "Line wrap {1}.",
    "pagebreak.ask" ["PageBreak"] "Change your page break to what? Your current page break setting is: *. *" => "Page break is {1}.",
    "pagebreak.bad" ["PageBreak"] "'*' is not a valid setting. Enter a number larger than 0 or 'disable'." => "{1} isn't valid.",
    "pagebreak.set" ["PageBreak"] "Your new page break setting is: *." => "Page break {1}.",
    "nobattlespam.off" ["NoBattleSpam"] "No Battle Spam has been turned off. You will now see combat messages again." => "All combat messages.",
    "nobattlespam.on" ["NoBattleSpam"] "No Battle Spam has been turned on. You will no longer see many combat messages." => "Fewer combat messages.",
    "nospam.off" ["NoSpam"] "No Spam has been turned off. You will now see 'spammy' messages again." => "Spammy messages shown.",
    "nospam.on" ["NoSpam"] "No Spam has been turned on. You will no longer see some 'spammy' messages." => "Spammy messages hidden.",
    "noteach.off" ["NoTeach"] "You may now teach, train, or learn." => "Teaching on.",
    "noteach.on" ["NoTeach"] "You are no longer teaching, training, or learning." => "Teaching off.",
    "quiet.on" ["Quiet"] "Quiet mode is now on. You will no longer receive channel messages or tells." => "Quiet on.",
    "quiet.off" ["Quiet"] "Quiet mode is now off. You may now receive channel messages and tells." => "Quiet off.",
    "wimpy.ask" ["Wimpy"] "Change your wimp level to what?" => "Wimpy to what?",
    "wimpy.bad" ["Wimpy"] "You can't change your wimp level to '*'" => "Can't set wimpy to {1}.",
    "wimpy.set" ["Wimpy"] "Your wimp level has been changed to * hit points." => "Wimpy {1}.",
    "nofollow.off" ["NoFollow"] "You are no longer accepting new followers or vassals." => "No new followers.",
    "nofollow.on" ["NoFollow"] "You are now accepting new followers and vassals." => "Followers welcome.",
    "nofollow.none" ["NoFollow"] "No one is following you!" => "No one follows you.",
    "pose.stop" ["Pose"] "You stop posing." => "Pose dropped.",
    "pose.none" ["Pose"] "You are not currently posing." => "Not posing.",
    "pose.now" ["Pose"] "Your current pose is: * (constant)" => "Pose: {1}.",
    "pose.here" ["Pose"] "Your current pose here is: * (here only)" => "Pose here: {1}.",

    // ---- Sitting, standing, sleeping ----
    "sit.down" ["Sit"] "You sit down and take a rest." => "Sitting.",
    "sit.up" ["Sit"] "You awake and sit up." => "Awake, sitting.",
    "sit.already" ["Sit"] "You are already sitting!" => "Already sitting.",
    "sit.on" ["Sit"] "You sit on *." => "Sitting on {1}.",
    "stand.up" ["Stand"] "You stand up." => "Standing.",
    "stand.wake" ["Stand"] "You wake up." => "Awake.",
    "stand.already" ["Stand"] "You are already standing!" => "Already standing.",
    "stand.not" ["Stand"] "You may not stand up." => "Can't stand.",
    "sleep.nap" ["Sleep"] "You lay down and take a nap." => "Sleeping.",
    "sleep.already" ["Sleep"] "You are already asleep!" => "Already asleep.",
    "sleep.on" ["Sleep"] "You sleep on *." => "Sleeping on {1}.",

    // ---- Things ----
    "get.from" ["Get", "Draw"] "You get * from *." => "Got {1}.",
    "get.it" ["Get"] "You get *." => "Got {1}.",
    "get.not-in" ["Get"] "You don't see '*' in *." => "No {1} in {2}.",
    "get.closed" ["Get", "Draw"] "* is closed." => "{1} is closed.",
    "get.nothing" ["Get"] "You don't see anything here." => "Nothing here.",
    "draw.it" ["Draw"] "You draw * from *." => "Drew {1}.",
    "drop.it" ["Drop"] "You drop *." => "Dropped {1}.",
    "put.out" ["Put"] "You put out *." => "Put out {1}.",
    "put.in" ["Put"] "You put * in *." => "Put {1} in {2}.",
    "put.into" ["Put"] "You put * into *." => "Put {1} in {2}.",
    "put.on" ["Put"] "You put * on *." => "Put {1} on {2}.",
    "put.where" ["Put"] "Where should I put the *?" => "Put {1} where?",
    "put.no-container" ["Put"] "I don't see a * here." => "No {1} here.",
    "give.it" ["Give", "Split"] "You give * to *." => "Gave {1} to {2}.",
    "give.not" ["Give", "Pay"] "Yea, you don't want to do that." => "Better not.",
    "pay.it" ["Pay"] "You pay * *." => "Paid {1} {2}.",
    "take.it" ["Take"] "You take * from *." => "Took {1} from {2}.",
    "wear.on-your" ["Wear"] "You put * on your *." => "{1} on your {2}.",
    "wear.on" ["Wear"] "You put on *." => "Wearing {1}.",
    "wear.where" ["Wear"] "You can't wear anything on your '*'" => "Nothing goes on {1}.",
    "wield.it" ["Wield", "Wear", "Hold"] "You wield *." => "Wielding {1}.",
    "hold.it" ["Hold", "Wear"] "You hold *." => "Holding {1}.",
    "remove.it" ["Remove"] "You remove *." => "Removed {1}.",
    "remove.not-worn" ["Remove"] "You don't seem to be wearing that." => "Not wearing that.",
    "sheath.it" ["Sheath"] "You sheath * in *." => "Sheathed {1}.",
    "sheath.nothing" ["Sheath"] "You don't seem to be wielding anything you can sheath." => "Nothing to sheath.",
    "sheath.no-sheath" ["Sheath"] "You are not wearing an appropriate sheath." => "No sheath for it.",
    "dress.it" ["Dress"] "You put * on *." => "Put {1} on {2}.",
    "undress.it" ["Undress"] "You take * off *." => "Took {1} off {2}.",
    "package.it" ["Package"] "You package up * *(s)." => "Packaged {1} {2}.",
    "nosell.on" ["NoSell"] "* is now marked unsellable." => "{1} won't be sold.",
    "nosell.for-now" ["NoSell"] "* is now marked *TEMPORARILY* unsellable." => "{1} won't be sold for now.",
    "nosell.off" ["NoSell"] "* is now sellable again." => "{1} can be sold.",

    // ---- Eating and drinking ----
    "eat.from" ["Eat"] "You eat * from *." => "Ate {1}.",
    "eat.it" ["Eat"] "You eat *." => "Ate {1}.",
    "eat.bite-in" ["Eat"] "You take a bite of * in *." => "Bit {1}.",
    "eat.bite" ["Eat"] "You take a bite of *." => "Bit {1}.",
    "eat.full" ["Eat", "Feed"] "You are full." => "Full.",
    "eat.fed" ["Eat", "Feed"] "You are no longer hungry." => "Not hungry.",
    "drink.of" ["DrinkCmd"] "You take a drink of * from *." => "Drank {1}.",
    "drink.from" ["DrinkCmd"] "You take a drink from *." => "Drank from {1}.",
    "drink.slaked" ["DrinkCmd"] "You are no longer thirsty." => "Not thirsty.",
    "drink.full" ["DrinkCmd"] "You have drunk all you can." => "Can't drink more.",
    "drink.empty" ["DrinkCmd", "Pour", "Fill"] "* is empty." => "{1} is empty.",
    "fill.it" ["Fill"] "You fill * from *." => "Filled {1}.",
    "fill.full" ["Fill"] "* is full." => "{1} is full.",
    "pour.into" ["Pour"] "You pour * into *." => "Poured {1} into {2}.",
    "pour.onto" ["Pour"] "You pour * onto *." => "Poured {1} on {2}.",
    "pour.out" ["Pour"] "You pour * out." => "Poured out {1}.",
    "feed.it" ["Feed"] "You feed * to *." => "Fed {2} {1}.",

    // ---- Doors and lids ----
    "open.it" ["Open"] "You open *." => "Opened {1}.",
    "open.already" ["Open"] "The * is already *!" => "Already {2}.",
    "close.it" ["Close"] "You close *." => "Closed {1}.",
    "close.already" ["Close"] "The * is already *." => "Already {2}.",
    "door.nothing" ["Open", "Close"] "There is nothing to * that way!" => "Nothing to {1}.",
    "lock.it" ["Lock"] "You lock *." => "Locked {1}.",
    "unlock.it" ["Unlock"] "You unlock *." => "Unlocked {1}.",
    "knock.it" ["Knock"] "You knock on *." => "Knocked on {1}.",

    // ---- Riding ----
    "mount.it" ["Mount"] "You mount *." => "Mounted {1}.",
    "dismount.not" ["Dismount"] "But you aren't riding anything?!" => "Not riding.",
    "disembark.not" ["Disembark"] "But you aren't on anything?!" => "Not on anything.",

    // ---- Shops and banks ----
    "buy.it" ["Buy"] "You buy * from *." => "Bought {1}.",
    "sell.it" ["Sell"] "You sell * to *." => "Sold {1}.",
    "deposit.account" ["Deposit"] "You deposit * into your account with *." => "Deposited {1}.",
    "deposit.it" ["Deposit"] "You deposit * with *." => "Deposited {1}.",
    "deposit.mail" ["Deposit"] "You mail *." => "Mailed {1}.",
    "withdraw.box" ["Withdraw"] "You withdraw * from your postal box with *." => "Took {1} from your box.",
    "withdraw.it" ["Withdraw"] "You withdraw * from your account with *." => "Withdrew {1}.",
    "borrow.it" ["Borrow"] "You borrow * from *." => "Borrowed {1}.",
    "bid.it" ["Bid"] "You bid * on * with *." => "Bid {1} on {2}.",
    "request.add" ["Request"] "You list a new item request with *." => "Request listed.",
    "request.cancel" ["Request"] "You cancel all item requests with *." => "Requests cancelled.",

    // ---- People ----
    "follow.it" ["Follow"] "You follow *." => "Following {1}.",
    "follow.stop" ["Follow"] "You stop following *." => "Stopped following {1}.",
    "follow.not-here" ["Follow"] "I don't see them here." => "Not here.",
    "follow.closed" ["Follow"] "* is not accepting followers." => "{1} takes no followers.",
    "serve.it" ["Serve"] "You swear fealty to *." => "Serving {1}.",
    "rebuke.it" ["Rebuke"] "You rebuke *." => "Rebuked {1}.",
    "order.it" ["Order"] "You order * to '*'." => "Ordered {1}: {2}.",
    "practice.it" ["Practice"] "* practices '*' with you." => "Practiced {2}.",
    "practice.points" ["Practice"] "You have * practice points. Enter HELP PRACTICE for more information." => "{1} practices.",
    "duel.it" ["Duel"] "You have challenged * to a duel, which * * * seconds to consider." => "Challenged {1}.",
    "throw.at" ["Throw"] "You throw * at *." => "Threw {1} at {2}.",
    "friends.add" ["Friends"] "The Player '*' has been added to your friends list." => "{1} is a friend.",
    "friends.remove" ["Friends"] "The Player '*' has been removed from your friends list." => "{1} is no longer a friend.",
    "friends.list" ["Friends"] "Your listed friends are: *" => "Friends: {1}.",
    "friends.none" ["Friends"] "You have no friends listed. Use FRIENDS ADD to add more." => "No friends listed.",
    "ignore.add-account" ["Ignore"] "The Account '*' has been added to your ignore list." => "Ignoring {1}.",
    "ignore.add" ["Ignore"] "The Player/Account '*' has been added to your ignore list." => "Ignoring {1}.",
    "ignore.remove-account" ["Ignore"] "The Account '*' has been removed from your ignore list." => "Not ignoring {1}.",
    "ignore.remove" ["Ignore"] "The Player '*' has been removed from your ignore list." => "Not ignoring {1}.",
    "ignore.list" ["Ignore"] "You are ignoring: *" => "Ignoring {1}.",
    "ignore.none" ["Ignore"] "You have no names on your ignore list. Use IGNORE ADD to add more." => "Ignoring no one.",
    "list.no-player" ["Friends", "Ignore"] "No player by that name was found." => "No such player.",
    "list.already" ["Friends", "Ignore"] "That name is already on your list." => "Already listed.",
    "list.not-on" ["Friends", "Ignore"] "That name '*' does not appear on your list. Watch your casing!" => "{1} isn't listed.",

    // ---- The rest ----
    "sniff.it" ["Sniff"] "You sniff *." => "Sniffed {1}.",
    "sniff.around" ["Sniff"] "You sniff around." => "Sniffed around.",
    "visible.already" ["Visible"] "You are not invisible or hidden!" => "Already visible.",
    "dig.done" ["Dig"] "You finish digging." => "Done digging.",
    "dieroll.die" ["DieRoll"] "You make a * on a * roll." => "Rolled {1} on {2}.",
    "dieroll.made" ["DieRoll"] "You make a(n) * * roll." => "Made the {1} {2} roll.",
    "dieroll.failed" ["DieRoll"] "You fail a(n) * * roll." => "Failed the {1} {2} roll.",
    "journal.sent" ["CommandJournal"] "Your * message has been sent. Thank you." => "{1} sent.",
};

/// Both tables, for the list the player edits.
#[derive(Serialize)]
pub struct Listing {
    pub commands: &'static [Command],
    pub echoes: &'static [Echo],
}

pub fn listing() -> Listing {
    Listing { commands: COMMANDS, echoes: ECHOES }
}

/// An answer found in a read: where it is, and what the narrator says.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Echoed {
    /// The first of its lines in the read.
    pub line: usize,
    /// How many lines it took (the game wraps long ones).
    pub lines: usize,
    /// Which of [`ECHOES`]; none for an answer said as it was written.
    pub id: Option<&'static str>,
    /// What its `*`s stood for, in order.
    pub values: Vec<String>,
    /// What the narrator says unless the player changed it.
    pub says: String,
}

#[derive(Default)]
pub struct Echoes {
    /// Each line sent, oldest first: the one-line command it named, if any,
    /// and when.
    waiting: VecDeque<(Option<usize>, Instant)>,
}

/// Spaces collapsed, ends trimmed: wrapping and padding don't count.
fn squeeze(s: &str) -> String {
    s.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// The command a typed line names, as the server's exact pass finds it:
/// its first word, or its first character when that isn't a letter or a
/// digit (`'hello` is `'`).
fn command_of(line: &str) -> Option<usize> {
    let word = line.split_whitespace().next()?;
    let first = word.chars().next()?;
    let word = if word.chars().count() > 1 && !first.is_alphanumeric() { first.to_string() } else { word.to_uppercase() };
    COMMANDS.iter().position(|c| c.words.contains(&word.as_str()))
}

/// Whether `text` is `pattern`, each `*` standing for one or more
/// characters (the fewest that fit), letters' case aside. The `*`s' text
/// goes in `values`.
fn wild(pattern: &str, text: &str, values: &mut Vec<String>) -> bool {
    let Some(star) = pattern.find('*') else { return pattern.eq_ignore_ascii_case(text) };
    let (head, rest) = (&pattern[..star], &pattern[star + 1..]);
    if text.len() < head.len() || !text.is_char_boundary(head.len()) || !text[..head.len()].eq_ignore_ascii_case(head) {
        return false;
    }
    let text = &text[head.len()..];
    for (end, _) in text.char_indices().skip(1).chain(std::iter::once((text.len(), ' '))) {
        if end == 0 {
            continue;
        }
        values.push(text[..end].to_string());
        if wild(rest, &text[end..], values) {
            return true;
        }
        values.pop();
    }
    false
}

/// `says` with `{1}`, `{2}`... filled in.
pub fn fill(says: &str, values: &[String]) -> String {
    let mut out = says.to_string();
    for (i, v) in values.iter().enumerate() {
        out = out.replace(&format!("{{{}}}", i + 1), v);
    }
    out
}

/// The answer `text` is to command `command`, if it's one of its own or
/// a shared one.
fn find(command: &str, text: &str) -> Option<(&'static Echo, Vec<String>)> {
    ECHOES.iter().filter(|e| e.commands.is_empty() || e.commands.contains(&command)).find_map(|e| {
        let mut values = Vec::new();
        wild(&squeeze(e.line), text, &mut values).then_some((e, values))
    })
}

impl Echoes {
    /// A line the player sent.
    pub fn typed(&mut self, line: &str, now: Instant) {
        if self.waiting.len() == KEPT {
            self.waiting.pop_front();
        }
        self.waiting.push_back((command_of(line), now));
    }

    /// The connection or the character changed: nothing is waiting.
    pub fn clear(&mut self) {
        self.waiting.clear();
    }

    /// Finds the answer in one read's finished lines (their plain text)
    /// and makes their kind `Echo`. `partial` is the unfinished line the
    /// read left (the prompt, once the game has answered); `playing`,
    /// whether a character is in the game (before, every line is the
    /// login's).
    pub fn lines(&mut self, lines: &[String], kinds: &mut [LineKind], partial: Option<&str>, playing: bool, now: Instant) -> Vec<Echoed> {
        while self.waiting.front().is_some_and(|(_, at)| now.duration_since(*at) > WAITS) {
            self.waiting.pop_front();
        }
        if !playing {
            self.waiting.clear();
            return Vec::new();
        }
        let theirs: Vec<usize> = (0..lines.len().min(kinds.len()))
            .filter(|&i| matches!(kinds[i], LineKind::Game | LineKind::Combat) && !lines[i].trim().is_empty())
            .collect();
        let Some(&(command, _)) = self.waiting.front() else { return Vec::new() };
        if theirs.is_empty() {
            return Vec::new();
        }
        let prompted = partial.is_some_and(|p| !p.trim().is_empty());
        let Some(command) = command else {
            // A block's command: the next one's answer comes after it.
            if prompted {
                self.waiting.pop_front();
            }
            return Vec::new();
        };
        let name = COMMANDS[command].name;
        let mut found = Vec::new();
        let mut i = 0;
        while i < theirs.len() {
            let mut joined = String::new();
            let mut hit = None;
            for n in 1..=WRAPPED {
                // Only lines one after another can be one wrapped line.
                if i + n > theirs.len() || theirs[i + n - 1] != theirs[i] + n - 1 {
                    break;
                }
                if !joined.is_empty() {
                    joined.push(' ');
                }
                joined.push_str(&squeeze(&lines[theirs[i + n - 1]]));
                if let Some((echo, values)) = find(name, &joined) {
                    hit = Some((n, echo, values));
                    break;
                }
            }
            match hit {
                Some((n, echo, values)) => {
                    let says = fill(echo.says, &values);
                    found.push(Echoed { line: theirs[i], lines: n, id: Some(echo.id), values, says });
                    i += n;
                }
                None => i += 1,
            }
        }
        if found.is_empty() && theirs.len() == 1 && prompted {
            let text = squeeze(&lines[theirs[0]]);
            found.push(Echoed { line: theirs[0], lines: 1, id: None, values: Vec::new(), says: text });
        }
        if !found.is_empty() || prompted {
            self.waiting.pop_front();
        }
        for e in &found {
            for kind in &mut kinds[e.line..e.line + e.lines] {
                *kind = LineKind::Echo;
            }
        }
        found
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use LineKind::*;

    fn strings(lines: &[&str]) -> Vec<String> {
        lines.iter().map(|s| s.to_string()).collect()
    }

    fn read(e: &mut Echoes, lines: &[&str], partial: Option<&str>) -> (Vec<LineKind>, Vec<Echoed>) {
        let lines = strings(lines);
        let mut kinds = vec![Game; lines.len()];
        let found = e.lines(&lines, &mut kinds, partial, true, Instant::now());
        (kinds, found)
    }

    #[test]
    fn every_id_is_its_own_and_every_command_is_known() {
        let mut ids = std::collections::HashSet::new();
        for e in ECHOES {
            assert!(ids.insert(e.id), "{} twice", e.id);
            for c in e.commands {
                assert!(COMMANDS.iter().any(|k| k.name == *c), "{} names {c}, not a one-line command", e.id);
            }
            let stars = e.line.matches('*').count();
            for n in 1..=stars + 2 {
                let mark = format!("{{{n}}}");
                assert!(n <= stars || !e.says.contains(&mark), "{} says {mark} with {stars} stars", e.id);
            }
        }
    }

    #[test]
    fn no_word_calls_two_commands() {
        let mut words = std::collections::HashSet::new();
        for c in COMMANDS {
            for w in c.words {
                assert!(words.insert(*w), "{w} twice");
            }
        }
    }

    #[test]
    fn wildcards_take_the_fewest_characters() {
        let mut v = Vec::new();
        assert!(wild("You get * from *.", "You get a sword from a chest.", &mut v));
        assert_eq!(v, vec!["a sword", "a chest"]);
        v.clear();
        assert!(!wild("You get *.", "You got a sword.", &mut v));
        assert!(v.is_empty());
        assert!(wild("you DROP *.", "You drop it.", &mut v));
    }

    #[test]
    fn the_command_is_its_first_word_exactly() {
        assert_eq!(command_of("sit").map(|i| COMMANDS[i].name), Some("Sit"));
        assert_eq!(command_of("  get sword chest").map(|i| COMMANDS[i].name), Some("Get"));
        assert_eq!(command_of(":waves").map(|i| COMMANDS[i].name), Some("Emote"));
        // Not an access word: the server's inexact pass might find anything.
        assert_eq!(command_of("sitt"), None);
        assert_eq!(command_of("look"), None);
        assert_eq!(command_of(""), None);
    }

    #[test]
    fn an_answer_is_found_and_said_short() {
        let mut e = Echoes::default();
        e.typed("sit", Instant::now());
        let (kinds, found) = read(&mut e, &["<20hp>", "You sit down and take a rest."], Some("<20hp> "));
        assert_eq!(kinds, vec![Game, Echo]);
        assert_eq!(found, vec![Echoed { line: 1, lines: 1, id: Some("sit.down"), values: vec![], says: "Sitting.".into() }]);
        // Answered: the next read is nobody's.
        let (kinds, found) = read(&mut e, &["You sit down and take a rest."], Some("> "));
        assert_eq!((kinds, found), (vec![Game], vec![]));
    }

    #[test]
    fn values_fill_the_narrators_words() {
        let mut e = Echoes::default();
        e.typed("get sword chest", Instant::now());
        let (_, found) = read(&mut e, &["You get a long sword from a wooden chest."], Some("> "));
        assert_eq!(found[0].says, "Got a long sword.");
        assert_eq!(found[0].values, vec!["a long sword", "a wooden chest"]);
    }

    #[test]
    fn a_wrapped_answer_is_joined_back_up() {
        let mut e = Echoes::default();
        e.typed("quiet", Instant::now());
        let (kinds, found) = read(&mut e, &["Quiet mode is now on.  You will no longer receive channel", "messages or tells."], Some("> "));
        assert_eq!(kinds, vec![Echo, Echo]);
        assert_eq!(found[0].lines, 2);
        assert_eq!(found[0].says, "Quiet on.");
    }

    #[test]
    fn get_all_is_an_echo_a_line() {
        let mut e = Echoes::default();
        e.typed("get all", Instant::now());
        let (kinds, found) = read(&mut e, &["You get a bread.", "You get a torch."], Some("> "));
        assert_eq!(kinds, vec![Echo, Echo]);
        assert_eq!(found.iter().map(|f| f.says.as_str()).collect::<Vec<_>>(), vec!["Got a bread.", "Got a torch."]);
    }

    #[test]
    fn a_lone_unknown_answer_is_said_as_written() {
        let mut e = Echoes::default();
        e.typed("weather", Instant::now());
        let (kinds, found) = read(&mut e, &["The sky is clear."], Some("> "));
        assert_eq!(kinds, vec![Echo]);
        assert_eq!(found[0].id, None);
        assert_eq!(found[0].says, "The sky is clear.");
    }

    #[test]
    fn unknown_lines_among_others_stay() {
        let mut e = Echoes::default();
        e.typed("sit", Instant::now());
        let (kinds, _) = read(&mut e, &["A rat arrives from the north.", "You sit down and take a rest."], Some("> "));
        assert_eq!(kinds, vec![Game, Echo]);
        e.typed("weather", Instant::now());
        let (kinds, found) = read(&mut e, &["A rat arrives.", "The sky is clear."], Some("> "));
        assert_eq!((kinds, found), (vec![Game, Game], vec![]));
    }

    #[test]
    fn a_blocks_answer_is_skipped_for_the_next() {
        let mut e = Echoes::default();
        let now = Instant::now();
        e.typed("look", now);
        e.typed("sit", now);
        let (kinds, _) = read(&mut e, &["The Square", "A wide square."], Some("> "));
        assert_eq!(kinds, vec![Game, Game]);
        let (kinds, _) = read(&mut e, &["You sit down and take a rest."], Some("> "));
        assert_eq!(kinds, vec![Echo]);
    }

    #[test]
    fn without_a_prompt_only_known_answers_count() {
        let mut e = Echoes::default();
        e.typed("weather", Instant::now());
        let (kinds, _) = read(&mut e, &["A rat arrives."], None);
        assert_eq!(kinds, vec![Game]);
        let (kinds, _) = read(&mut e, &["The sky is clear."], Some("> "));
        assert_eq!(kinds, vec![Echo]);
    }

    #[test]
    fn talk_and_the_time_are_never_echoes() {
        let mut e = Echoes::default();
        e.typed("sit", Instant::now());
        let lines = strings(&["You sit down and take a rest."]);
        let mut kinds = vec![Talk];
        assert!(e.lines(&lines, &mut kinds, Some("> "), true, Instant::now()).is_empty());
        assert_eq!(kinds, vec![Talk]);
    }

    #[test]
    fn nothing_before_the_game_or_after_waiting_too_long() {
        let mut e = Echoes::default();
        let now = Instant::now();
        e.typed("sit", now);
        let lines = strings(&["You sit down and take a rest."]);
        let mut kinds = vec![Game];
        assert!(e.lines(&lines, &mut kinds, Some("> "), false, now).is_empty());
        e.typed("sit", now);
        let later = now + WAITS + Duration::from_secs(1);
        assert!(e.lines(&lines, &mut kinds, Some("> "), true, later).is_empty());
        assert_eq!(kinds, vec![Game]);
    }

    #[test]
    fn shared_answers_fit_every_command() {
        let mut e = Echoes::default();
        e.typed("open chest", Instant::now());
        let (_, found) = read(&mut e, &["You don't see 'chest' here."], Some("> "));
        assert_eq!(found[0].says, "No chest here.");
    }
}
