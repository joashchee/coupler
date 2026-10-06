// Copyright 2026 ansiapps. Neumetik MIDI, MIT licensed: see LICENSE.

use std::collections::VecDeque;

use crate::smf::{self, Message};
use crate::{Error, Synth};

/// A queued event: a channel message, or a system exclusive message kept
/// in the sequencer's pool, so playing it frees nothing.
enum Queued {
    Channel(u8, u8, u8),
    /// Where its bytes are in the pool, and how many.
    SysEx(usize, usize),
}

/// MIDI files played one after another through one synthesizer, as they
/// come: each queued file starts where the last one ends (at its last
/// event), with no gap, and whatever was ringing (a held note's release,
/// the reverb) carries on into it. For music that's written as it plays.
///
/// [`queue`](Sequencer::queue) allocates; [`render`](Sequencer::render)
/// doesn't, and frees nothing either.
pub struct Sequencer {
    synth: Synth,
    rate: f64,
    /// Events still to play, by frame.
    events: VecDeque<(u64, Queued)>,
    /// System exclusive messages' bytes, and how many at its front have
    /// been played (dropped at the next queue).
    pool: Vec<u8>,
    played: usize,
    /// The frame played up to, and the frame the queue ends at.
    now: u64,
    end: u64,
}

impl Sequencer {
    pub fn new(sample_rate: u32) -> Sequencer {
        let synth = Synth::new(sample_rate);
        Sequencer { rate: f64::from(synth.sample_rate()), synth, events: VecDeque::new(), pool: Vec::new(), played: 0, now: 0, end: 0 }
    }

    /// Queues a MIDI file to start when everything queued before it ends
    /// (or now, if that's passed). Returns its length in frames.
    pub fn queue(&mut self, bytes: &[u8]) -> Result<u64, Error> {
        let events = smf::parse(bytes)?;
        // What's been played of the pool goes now, off the audio's path.
        if self.played > 0 {
            self.pool.drain(..self.played);
            for (_, q) in self.events.iter_mut() {
                if let Queued::SysEx(at, _) = q {
                    *at -= self.played;
                }
            }
            self.played = 0;
        }
        let start = self.end.max(self.now);
        let length = events.last().map_or(0, |e| (e.seconds * self.rate).round() as u64);
        for e in events {
            let q = match e.message {
                Message::Channel(s, a, b) => Queued::Channel(s, a, b),
                Message::SysEx(data) => {
                    self.pool.extend_from_slice(&data);
                    Queued::SysEx(self.pool.len() - data.len(), data.len())
                }
            };
            self.events.push_back((start + (e.seconds * self.rate).round() as u64, q));
        }
        self.end = start + length;
        Ok(length)
    }

    /// Frames queued and not yet played.
    pub fn ahead(&self) -> u64 {
        self.end.saturating_sub(self.now)
    }

    /// The synthesizer it plays through.
    pub fn synth(&self) -> &Synth {
        &self.synth
    }

    pub fn synth_mut(&mut self) -> &mut Synth {
        &mut self.synth
    }

    /// Renders the next frames into `left` and `right` (the shorter's
    /// length), playing each event on its frame. Past the queue's end it
    /// keeps rendering what's still ringing.
    pub fn render(&mut self, left: &mut [f32], right: &mut [f32]) {
        let n = left.len().min(right.len());
        let mut done = 0;
        while done < n {
            while self.events.front().is_some_and(|(at, _)| *at <= self.now) {
                let Some((_, q)) = self.events.pop_front() else { break };
                match q {
                    Queued::Channel(s, a, b) => self.synth.message(&Message::Channel(s, a, b)),
                    Queued::SysEx(at, len) => {
                        self.synth.send_sysex(&self.pool[at..at + len]);
                        self.played = at + len;
                    }
                }
            }
            let next = self.events.front().map_or(u64::MAX, |(at, _)| *at);
            let step = ((n - done) as u64).min(next - self.now) as usize;
            self.synth.render(&mut left[done..done + step], &mut right[done..done + step]);
            done += step;
            self.now += step as u64;
        }
    }

    #[cfg(test)]
    pub(crate) fn pool_len(&self) -> usize {
        self.pool.len()
    }

    /// Renders interleaved stereo, left then right: `out.len() / 2` frames.
    pub fn render_interleaved(&mut self, out: &mut [f32]) {
        crate::output::interleaved(out, |l, r| self.render(l, r));
    }

    /// Renders interleaved stereo as 16-bit samples, clipped at full scale.
    pub fn render_i16(&mut self, out: &mut [i16]) {
        crate::output::interleaved_i16(out, |l, r| self.render(l, r));
    }
}
