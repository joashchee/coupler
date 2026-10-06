// Copyright 2026 ansiapps. Neumetik MIDI, MIT licensed: see LICENSE.

//! A song and the player for a game's music: play, pause, stop, seek,
//! loop (between the file's own loop points or any two you set), volume
//! and fades, speed, and channels muted for music in layers.

use crate::smf::{self, Event};
use crate::synth::{State, Synth};
use crate::Error;

/// A MIDI file, read once, ready to play. Reading it allocates; playing it
/// doesn't, so a game can read songs while loading and hand them to a
/// [`Player`] later.
#[derive(Clone, Debug)]
pub struct Song {
    events: Vec<Event>,
    end: f64,
    loop_start: Option<f64>,
    loop_end: Option<f64>,
}

impl Song {
    /// Reads a Standard MIDI File (format 0, 1 or 2, or RIFF RMID).
    pub fn from_midi(bytes: &[u8]) -> Result<Song, Error> {
        let parsed = smf::parse_song(bytes)?;
        Ok(Song { events: parsed.events, end: parsed.end, loop_start: parsed.loop_start, loop_end: parsed.loop_end })
    }

    /// How long it is, in seconds, to its last End of Track.
    pub fn duration(&self) -> f64 {
        self.end
    }

    /// The loop the file marks, in seconds: a `loopStart` marker (or
    /// CC 111) and a `loopEnd` marker, or the end if it marks a start only.
    pub fn loop_points(&self) -> Option<(f64, f64)> {
        let start = self.loop_start?;
        Some((start, self.loop_end.unwrap_or(self.end)))
    }
}

/// Whether a [`Player`] is playing.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum PlayState {
    /// Not playing: never started, stopped, or played to its end.
    Stopped,
    Playing,
    /// Held where it was, silent, until [`Player::play`].
    Paused,
}

/// Plays a [`Song`] through a [`Synth`]. Load a song, call
/// [`play`](Player::play), and call [`render`](Player::render) (or
/// [`render_interleaved`](Player::render_interleaved),
/// [`render_i16`](Player::render_i16)) for every buffer of sound.
///
/// Nothing it does while rendering allocates, locks or panics. It isn't
/// shared between threads by itself: call it from one thread at a time
/// (in most engines, the game loop that fills the audio stream).
pub struct Player {
    synth: Synth,
    rate: f64,
    song: Option<Song>,
    /// The next event to play, and the song's time, in seconds.
    cursor: usize,
    at: f64,
    state: PlayState,
    finished: bool,
    looping: bool,
    /// Loop points set by the game, over the song's own.
    loop_start: Option<f64>,
    loop_end: Option<f64>,
    /// Where the loop starts, worked out when the loop is set: the time,
    /// the next event there, and the channels as they stand there.
    loop_from: (f64, usize, State),
    speed: f64,
}

impl Player {
    /// A player at `sample_rate` frames a second.
    pub fn new(sample_rate: u32) -> Player {
        let synth = Synth::new(sample_rate);
        Player {
            rate: f64::from(synth.sample_rate()),
            synth,
            song: None,
            cursor: 0,
            at: 0.0,
            state: PlayState::Stopped,
            finished: false,
            looping: false,
            loop_start: None,
            loop_end: None,
            loop_from: (0.0, 0, State::new()),
            speed: 1.0,
        }
    }

    /// Reads a MIDI file and loads it, stopped at its start. The game's
    /// loop points are cleared; looping stays as it was.
    pub fn load(&mut self, bytes: &[u8]) -> Result<(), Error> {
        self.load_song(Song::from_midi(bytes)?);
        Ok(())
    }

    /// Loads a song already read, stopped at its start, and hands back the
    /// one it replaces.
    pub fn load_song(&mut self, song: Song) -> Option<Song> {
        let old = self.song.replace(song);
        self.loop_start = None;
        self.loop_end = None;
        self.stop();
        self.refresh_loop();
        old
    }

    /// Takes the song out, stopping.
    pub fn unload(&mut self) -> Option<Song> {
        self.stop();
        self.song.take()
    }

    pub fn song(&self) -> Option<&Song> {
        self.song.as_ref()
    }

    /// Plays: from where it's paused or stopped, or from the start if it
    /// played to its end.
    pub fn play(&mut self) {
        if self.song.is_none() {
            return;
        }
        if self.finished {
            self.seek(0.0);
        }
        self.state = PlayState::Playing;
    }

    /// Holds it where it is, silent. [`play`](Player::play) carries on.
    pub fn pause(&mut self) {
        if self.state == PlayState::Playing {
            self.state = PlayState::Paused;
        }
    }

    /// Stops, silences it and goes back to the start.
    pub fn stop(&mut self) {
        self.state = PlayState::Stopped;
        self.seek(0.0);
    }

    pub fn state(&self) -> PlayState {
        self.state
    }

    pub fn is_playing(&self) -> bool {
        self.state == PlayState::Playing
    }

    /// Whether it played to its end (and isn't looping); [`play`](Player::play)
    /// starts it again.
    pub fn is_finished(&self) -> bool {
        self.finished
    }

    /// Goes to `seconds` into the song: what's sounding stops, and every
    /// channel's instrument and controllers are set as the song has them
    /// there. Playing or paused, it stays so.
    pub fn seek(&mut self, seconds: f64) {
        let end = self.duration();
        let t = if seconds.is_finite() { seconds.clamp(0.0, end) } else { 0.0 };
        let (cursor, state) = self.chase(t);
        self.synth.all_sound_off();
        self.synth.set_state(state);
        self.cursor = cursor;
        self.at = t;
        self.finished = false;
    }

    /// Where it's got to in the song, in seconds.
    pub fn position(&self) -> f64 {
        self.at
    }

    /// The song's length in seconds (0 with none loaded).
    pub fn duration(&self) -> f64 {
        self.song.as_ref().map_or(0.0, Song::duration)
    }

    /// Loops the song (or stops looping), between its loop points.
    pub fn set_looping(&mut self, looping: bool) {
        self.looping = looping;
    }

    pub fn is_looping(&self) -> bool {
        self.looping
    }

    /// Sets where the loop starts and ends, in seconds, over the file's own
    /// loop points; `None` for either goes back to the file's (or the
    /// song's start and end).
    pub fn set_loop_points(&mut self, start: Option<f64>, end: Option<f64>) {
        self.loop_start = start.filter(|s| s.is_finite() && *s >= 0.0);
        self.loop_end = end.filter(|e| e.is_finite() && *e > 0.0);
        self.refresh_loop();
    }

    /// Where the loop starts and ends, in seconds, as it will play.
    pub fn loop_points(&self) -> (f64, f64) {
        let Some(song) = &self.song else { return (0.0, 0.0) };
        let file = song.loop_points();
        let start = self.loop_start.or(file.map(|f| f.0)).unwrap_or(0.0).min(song.end);
        let end = self.loop_end.or(file.map(|f| f.1)).unwrap_or(song.end).min(song.end);
        if end > start {
            (start, end)
        } else {
            (0.0, song.end)
        }
    }

    /// Plays faster or slower (1 as written, 0.25 to 4), its pitch kept.
    pub fn set_speed(&mut self, speed: f64) {
        self.speed = if speed.is_finite() { speed.clamp(0.25, 4.0) } else { 1.0 };
    }

    pub fn speed(&self) -> f64 {
        self.speed
    }

    /// The overall volume, 0 to 1 (up to 4 for quiet music).
    pub fn set_volume(&mut self, volume: f32) {
        self.synth.set_volume(volume);
    }

    pub fn volume(&self) -> f32 {
        self.synth.volume()
    }

    /// Fades the volume to `volume` over `seconds`: 0 to fade out (it keeps
    /// playing, silent), back to 1 to fade in.
    pub fn fade_to(&mut self, volume: f32, seconds: f32) {
        self.synth.fade_to(volume, seconds);
    }

    /// Mutes or unmutes a channel (0 to 15; 9 is the drums). It plays on,
    /// unheard, so it comes back in time: for music in layers.
    pub fn set_channel_muted(&mut self, channel: u8, muted: bool) {
        self.synth.set_channel_muted(channel, muted);
    }

    pub fn channel_muted(&self, channel: u8) -> bool {
        self.synth.channel_muted(channel)
    }

    /// The synthesizer it plays through: what a channel plays, how many
    /// voices are sounding.
    pub fn synth(&self) -> &Synth {
        &self.synth
    }

    /// The synthesizer it plays through, to play notes over the music (a
    /// sound effect, a sting) or change an instrument.
    pub fn synth_mut(&mut self) -> &mut Synth {
        &mut self.synth
    }

    /// Renders the next frames into `left` and `right` (the shorter's
    /// length), playing the song's events on their frames.
    pub fn render(&mut self, left: &mut [f32], right: &mut [f32]) {
        let n = left.len().min(right.len());
        let mut done = 0;
        while done < n {
            if self.state == PlayState::Paused {
                left[done..n].fill(0.0);
                right[done..n].fill(0.0);
                return;
            }
            let Some(next) = (if self.state == PlayState::Playing { self.advance() } else { None }) else {
                // Stopped: what's still ringing, and any notes played live.
                self.synth.render(&mut left[done..n], &mut right[done..n]);
                return;
            };
            let frames = ((next - self.at) * self.rate / self.speed).ceil().max(1.0);
            let step = (n - done).min(frames as usize);
            self.synth.render(&mut left[done..done + step], &mut right[done..done + step]);
            self.at += step as f64 * self.speed / self.rate;
            done += step;
        }
    }

    /// Renders interleaved stereo, left then right: `out.len() / 2` frames.
    pub fn render_interleaved(&mut self, out: &mut [f32]) {
        crate::output::interleaved(out, |l, r| self.render(l, r));
    }

    /// Renders interleaved stereo as 16-bit samples, clipped at full scale.
    pub fn render_i16(&mut self, out: &mut [i16]) {
        crate::output::interleaved_i16(out, |l, r| self.render(l, r));
    }

    /// Plays the events that are due, loops if it's time, and says when the
    /// next thing happens; `None` once it's played to its end.
    fn advance(&mut self) -> Option<f64> {
        self.song.as_ref()?;
        let (_, loop_end) = self.loop_points();
        if self.looping && self.at >= loop_end {
            let (start, cursor, state) = self.loop_from;
            // Notes held over the end are let go; the channels go back to
            // how they stand at the loop's start.
            self.synth.all_notes_off();
            self.synth.set_state(state);
            self.cursor = cursor;
            self.at = start + (self.at - loop_end).min(loop_end - start);
        }
        let song = self.song.as_ref()?;
        while let Some(e) = song.events.get(self.cursor).filter(|e| e.seconds <= self.at) {
            self.synth.message(&e.message);
            self.cursor += 1;
        }
        let end = if self.looping { loop_end } else { song.end };
        if !self.looping && self.at >= song.end && self.cursor >= song.events.len() {
            self.state = PlayState::Stopped;
            self.finished = true;
            return None;
        }
        Some(song.events.get(self.cursor).map_or(end, |e| e.seconds.min(end)))
    }

    /// The next event at `t`, and the channels as they stand there.
    fn chase(&self, t: f64) -> (usize, State) {
        let mut state = State::new();
        let Some(song) = &self.song else { return (0, state) };
        let mut cursor = 0;
        for e in song.events.iter().take_while(|e| e.seconds < t) {
            state.chase(&e.message);
            cursor += 1;
        }
        (cursor, state)
    }

    fn refresh_loop(&mut self) {
        let (start, _) = self.loop_points();
        let (cursor, state) = self.chase(start);
        self.loop_from = (start, cursor, state);
    }
}
