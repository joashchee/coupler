// Copyright 2026 ansiapps. Neumetik: see README.md for its license.

//! General MIDI's 128 instruments, in GM's order, each built from
//! Neumetik's engines. The names are GM's own.

use super::patch::Lfo;
use super::patch::Patch as P;
use super::patch::Wave::*;

const fn piano(name: &'static str, index: f32) -> P {
    P::fm(name, 1.0, index)
        .index_env(0.5, 0.15)
        .feedback(0.15)
        .tine(0.25, 1.0, 7.0, 1.5, 0.08)
        .amp(0.002, 5.0, 0.0, 0.3)
        .key_decay(0.8)
        .fx(0.0, 0.15)
}

const fn string_section(name: &'static str, attack: f32, cutoff: f32) -> P {
    P::analog(name, Saw)
        .unison(5, 16.0)
        .lp(cutoff, 0.1)
        .amp(attack, 1.0, 1.0, 0.5)
        .vibrato(5.0, 6.0, 0.3)
        .fx(0.4, 0.35)
        .gain(0.55)
}

const fn solo_string(name: &'static str, cutoff: f32) -> P {
    P::analog(name, Saw)
        .osc2(Pulse, 0.3, 3.0)
        .pw(0.3)
        .lp(cutoff, 0.15)
        .key_track(0.7)
        .amp(0.07, 1.0, 1.0, 0.25)
        .vibrato(5.5, 22.0, 0.25)
        .fx(0.1, 0.3)
        .gain(0.55)
}

const fn brass(name: &'static str, cutoff: f32, amount: f32) -> P {
    P::analog(name, Saw)
        .lp(cutoff, 0.15)
        .key_track(0.8)
        .vel_filter(1.0)
        .fenv(0.04, 0.25, 0.6, 0.2, amount)
        .amp(0.03, 1.0, 1.0, 0.15)
        .vibrato(5.0, 12.0, 0.4)
        .fx(0.0, 0.25)
        .gain(0.6)
}

const fn sax(name: &'static str, cutoff: f32) -> P {
    P::fm(name, 1.0, 1.8)
        .index_env(0.15, 0.65)
        .feedback(0.6)
        .noise(0.03)
        .lp(cutoff, 0.15)
        .key_track(0.8)
        .amp(0.03, 1.0, 1.0, 0.12)
        .vibrato(5.0, 15.0, 0.35)
        .fx(0.0, 0.25)
        .gain(0.7)
}

const fn reed(name: &'static str, wave: super::patch::Wave, pw: f32, cutoff: f32) -> P {
    P::analog(name, wave)
        .pw(pw)
        .noise(0.02)
        .lp(cutoff, 0.25)
        .key_track(0.8)
        .amp(0.035, 1.0, 1.0, 0.1)
        .vibrato(5.0, 10.0, 0.4)
        .fx(0.0, 0.25)
        .gain(0.6)
}

const fn pipe(name: &'static str, breath: f32, cutoff: f32) -> P {
    P::analog(name, Triangle)
        .osc2(Sine, 0.6, 0.0)
        .noise(breath)
        .lp(cutoff, 0.1)
        .key_track(1.0)
        .amp(0.04, 1.0, 1.0, 0.12)
        .vibrato(5.0, 12.0, 0.3)
        .fx(0.0, 0.3)
        .gain(0.7)
}

/// GM's instruments, program 0 (Acoustic Grand Piano) to 127 (Gunshot).
pub static GM: [P; 128] = TABLE;

/// The same, for the GS variations to build on while compiling (a
/// static can't be read then).
#[allow(clippy::large_const_arrays)]
pub const TABLE: [P; 128] = [
    // Piano.
    piano("Acoustic Grand Piano", 2.0),
    piano("Bright Acoustic Piano", 2.9),
    piano("Electric Grand Piano", 2.4).tine(0.3, 1.0, 3.0, 2.0, 0.2).fx(0.3, 0.15),
    piano("Honky-tonk Piano", 2.2).tine(0.8, 1.0, 1.0, 1.5, 4.0).detune(14.0),
    P::fm("Electric Piano 1", 1.0, 1.0).index_env(1.0, 0.3).tine(0.45, 1.0, 14.0, 1.0, 0.25).amp(0.002, 5.0, 0.0, 0.3).key_decay(0.7).lfo(Lfo::Sine, 4.0, 0.0, 0.08).fx(0.2, 0.2),
    P::fm("Electric Piano 2", 1.0, 1.4).index_env(0.8, 0.2).tine(0.8, 1.0, 14.0, 1.4, 0.4).detune(7.0).amp(0.002, 4.0, 0.0, 0.3).key_decay(0.7).fx(0.5, 0.2),
    P::pluck("Harpsichord", 2.5, 0.95).hp(350.0, 0.0).gain(0.8).fx(0.0, 0.2),
    P::analog("Clavi", Pulse).pw(0.12).lp(700.0, 0.35).fenv(0.0, 0.15, 0.1, 0.05, 3.0).amp(0.0, 1.5, 0.0, 0.05).key_decay(0.6).gain(0.7),
    // Chromatic percussion.
    P::fm("Celesta", 3.0, 1.0).index_env(0.3, 0.0).amp(0.001, 1.8, 0.0, 0.3).key_decay(0.5).fx(0.0, 0.3),
    P::fm("Glockenspiel", 3.5, 1.5).index_env(0.2, 0.0).tine(0.3, 2.76, 1.0, 0.0, 1.0).amp(0.0, 2.5, 0.0, 0.5).fx(0.0, 0.3).gain(0.7),
    P::fm("Music Box", 4.0, 0.8).index_env(0.5, 0.0).tine(0.3, 2.0, 1.0, 0.0, 0.8).amp(0.0, 2.0, 0.0, 0.4).fx(0.0, 0.35).gain(0.8),
    P::fm("Vibraphone", 4.0, 0.6).index_env(0.3, 0.0).amp(0.001, 3.0, 0.0, 0.4).lfo(Lfo::Sine, 5.5, 0.0, 0.35).fx(0.0, 0.25),
    P::fm("Marimba", 1.0, 0.3).index_env(0.1, 0.0).tine(0.6, 3.9, 1.0, 0.0, 0.06).amp(0.001, 0.5, 0.0, 0.1).key_decay(0.4).fx(0.0, 0.2),
    P::fm("Xylophone", 3.0, 1.0).index_env(0.05, 0.0).tine(0.5, 3.0, 1.0, 0.0, 0.08).amp(0.0, 0.35, 0.0, 0.08).fx(0.0, 0.2).gain(0.8),
    P::fm("Tubular Bells", 3.5, 2.5).index_env(1.5, 0.2).tine(0.3, 2.76, 5.4, 1.0, 2.0).amp(0.001, 6.0, 0.0, 1.0).fx(0.0, 0.35).gain(0.7),
    P::pluck("Dulcimer", 2.5, 0.8).gain(0.8).fx(0.2, 0.2),
    // Organ.
    P::organ("Drawbar Organ", [0.8, 0.8, 0.8, 0.0, 0.0, 0.0, 0.0]).vibrato(6.5, 8.0, 0.0).fx(0.4, 0.15),
    P::organ("Percussive Organ", [0.0, 0.8, 0.0, 0.6, 0.0, 0.0, 0.0]).tine(0.5, 3.0, 1.0, 0.0, 0.25).vibrato(6.5, 6.0, 0.0).fx(0.3, 0.15),
    P::organ("Rock Organ", [0.8, 0.8, 0.8, 0.5, 0.3, 0.3, 0.4]).drive(1.5).lfo(Lfo::Sine, 6.5, 10.0, 0.15).fx(0.3, 0.15),
    P::organ("Church Organ", [0.6, 0.8, 0.5, 0.7, 0.4, 0.5, 0.5]).amp(0.08, 1.0, 1.0, 0.6).fx(0.0, 0.6),
    P::analog("Reed Organ", Pulse).pw(0.3).osc2(Saw, 0.4, 2.0).lp(1800.0, 0.1).amp(0.05, 1.0, 1.0, 0.15).fx(0.2, 0.25).gain(0.5),
    P::analog("Accordion", Saw).osc2(Saw, 0.8, 10.0).lp(2500.0, 0.15).amp(0.03, 1.0, 1.0, 0.1).lfo(Lfo::Sine, 5.0, 0.0, 0.08).fx(0.0, 0.2).gain(0.45),
    P::analog("Harmonica", Pulse).pw(0.35).noise(0.04).lp(2000.0, 0.25).amp(0.04, 1.0, 1.0, 0.1).vibrato(5.5, 15.0, 0.3).fx(0.0, 0.2).gain(0.5),
    P::analog("Tango Accordion", Square).osc2(Square, 0.8, 6.0).lp(2200.0, 0.15).amp(0.03, 1.0, 1.0, 0.1).fx(0.0, 0.2).gain(0.45),
    // Guitar.
    P::pluck("Acoustic Guitar (nylon)", 2.5, 0.55).fx(0.0, 0.2),
    P::pluck("Acoustic Guitar (steel)", 3.0, 0.8).fx(0.15, 0.2),
    P::pluck("Electric Guitar (jazz)", 2.0, 0.45).lp(1500.0, 0.1).fx(0.0, 0.2),
    P::pluck("Electric Guitar (clean)", 3.0, 0.7).fx(0.4, 0.2),
    P::pluck("Electric Guitar (muted)", 0.25, 0.5).lp(1500.0, 0.0),
    P::analog("Overdriven Guitar", Saw).osc2(Saw, 0.7, 8.0).drive(4.0).lp(2200.0, 0.3).fenv(0.0, 0.3, 0.5, 0.2, 1.0).amp(0.003, 3.0, 0.4, 0.15).fx(0.2, 0.2).gain(0.55),
    P::analog("Distortion Guitar", Saw).osc2(Square, 0.7, 10.0).drive(9.0).lp(3000.0, 0.2).amp(0.003, 3.0, 0.5, 0.15).fx(0.2, 0.2).gain(0.5),
    P::fm("Guitar harmonics", 2.0, 0.2).amp(0.001, 2.0, 0.0, 0.3).fx(0.2, 0.3),
    // Bass.
    P::pluck("Acoustic Bass", 1.5, 0.35).lp(900.0, 0.0).gain(1.8),
    P::pluck("Electric Bass (finger)", 2.0, 0.5).lp(1500.0, 0.0).gain(1.8),
    P::pluck("Electric Bass (pick)", 1.8, 0.75).gain(1.5),
    P::analog("Fretless Bass", Triangle).osc2(Saw, 0.3, 0.0).lp(700.0, 0.3).amp(0.02, 2.0, 0.5, 0.15).vibrato(5.0, 10.0, 0.4).gain(0.9),
    P::pluck("Slap Bass 1", 1.2, 0.9).tine(0.5, 3.0, 1.0, 0.0, 0.03).gain(1.1),
    P::pluck("Slap Bass 2", 1.0, 1.0).tine(0.7, 4.0, 1.0, 0.0, 0.04).gain(1.1),
    P::analog("Synth Bass 1", Saw).osc2(Square, 0.6, 0.0).ratio(0.5).lp24(400.0, 0.5).fenv(0.0, 0.25, 0.2, 0.1, 3.0).amp(0.002, 1.0, 0.8, 0.08).gain(0.7),
    P::fm("Synth Bass 2", 1.0, 2.0).index_env(0.2, 0.1).feedback(0.4).amp(0.001, 1.5, 0.5, 0.08),
    // Strings.
    solo_string("Violin", 3200.0),
    solo_string("Viola", 2400.0),
    solo_string("Cello", 1600.0),
    solo_string("Contrabass", 900.0),
    string_section("Tremolo Strings", 0.05, 2600.0).lfo(Lfo::Sine, 8.0, 0.0, 0.7),
    P::pluck("Pizzicato Strings", 0.4, 0.5).fx(0.0, 0.35),
    P::pluck("Orchestral Harp", 3.0, 0.5).fx(0.0, 0.35),
    P::fm("Timpani", 1.5, 1.0).index_env(0.1, 0.0).noise(0.15).lp(900.0, 0.0).amp(0.002, 1.5, 0.0, 0.5).fx(0.0, 0.35).gain(1.2),
    // Ensemble.
    string_section("String Ensemble 1", 0.15, 3000.0),
    string_section("String Ensemble 2", 0.6, 2500.0),
    P::analog("SynthStrings 1", Saw).osc2(Saw, 0.8, 10.0).lp(2500.0, 0.1).amp(0.1, 1.0, 1.0, 0.5).fx(0.6, 0.3).gain(0.45),
    P::analog("SynthStrings 2", Pulse).osc2(Saw, 0.6, 8.0).pwm(0.6).lfo(Lfo::Sine, 0.6, 0.0, 0.0).lp(2000.0, 0.15).amp(0.4, 1.0, 1.0, 0.6).fx(0.6, 0.35).gain(0.45),
    P::analog("Choir Aahs", Saw).osc2(Saw, 0.8, 7.0).bp(900.0, 0.35).amp(0.2, 1.0, 1.0, 0.6).vibrato(5.0, 12.0, 0.2).fx(0.5, 0.45).gain(0.9),
    P::analog("Voice Oohs", Triangle).osc2(Sine, 0.6, 5.0).lp(800.0, 0.2).amp(0.15, 1.0, 1.0, 0.5).vibrato(5.0, 10.0, 0.2).fx(0.4, 0.4).gain(0.8),
    P::analog("Synth Voice", Square).osc2(Saw, 0.4, 6.0).bp(1000.0, 0.5).amp(0.1, 1.0, 1.0, 0.4).vibrato(5.0, 10.0, 0.2).fx(0.4, 0.4).gain(0.8),
    P::analog("Orchestra Hit", Saw).unison(5, 20.0).noise(0.3).lp(4000.0, 0.1).fenv(0.0, 0.4, 0.0, 0.2, -2.0).amp(0.001, 0.5, 0.0, 0.25).fx(0.0, 0.4).gain(0.8),
    // Brass.
    brass("Trumpet", 900.0, 2.5),
    brass("Trombone", 600.0, 2.0),
    brass("Tuba", 380.0, 1.5),
    P::analog("Muted Trumpet", Saw).bp(1500.0, 0.5).amp(0.03, 1.0, 1.0, 0.12).vibrato(5.0, 10.0, 0.4).fx(0.0, 0.25).gain(0.8),
    brass("French Horn", 500.0, 1.0).amp(0.06, 1.0, 1.0, 0.25).fx(0.0, 0.4),
    brass("Brass Section", 900.0, 2.5).unison(3, 12.0),
    P::analog("SynthBrass 1", Saw).osc2(Saw, 0.8, 12.0).lp24(600.0, 0.2).fenv(0.08, 0.4, 0.5, 0.3, 3.0).amp(0.03, 1.0, 1.0, 0.2).fx(0.3, 0.2).gain(0.55),
    P::analog("SynthBrass 2", Square).osc2(Saw, 0.8, 8.0).lp24(700.0, 0.15).fenv(0.15, 0.6, 0.5, 0.3, 2.0).amp(0.1, 1.0, 1.0, 0.3).fx(0.4, 0.25).gain(0.55),
    // Reed.
    sax("Soprano Sax", 3000.0),
    sax("Alto Sax", 2400.0),
    sax("Tenor Sax", 1800.0),
    sax("Baritone Sax", 1200.0),
    reed("Oboe", Pulse, 0.2, 1800.0),
    reed("English Horn", Pulse, 0.3, 1200.0),
    reed("Bassoon", Pulse, 0.25, 700.0),
    reed("Clarinet", Square, 0.5, 1800.0),
    // Pipe.
    pipe("Piccolo", 0.05, 5000.0),
    pipe("Flute", 0.08, 3000.0),
    pipe("Recorder", 0.03, 2500.0),
    pipe("Pan Flute", 0.25, 3000.0),
    pipe("Blown Bottle", 0.4, 1200.0).amp(0.1, 1.0, 1.0, 0.15),
    pipe("Shakuhachi", 0.3, 2500.0).vibrato(5.0, 25.0, 0.2),
    P::analog("Whistle", Sine).amp(0.03, 1.0, 1.0, 0.1).vibrato(6.0, 20.0, 0.2).fx(0.0, 0.25).gain(0.7),
    P::analog("Ocarina", Sine).osc2(Triangle, 0.3, 0.0).noise(0.02).amp(0.03, 1.0, 1.0, 0.1).vibrato(5.0, 10.0, 0.3).fx(0.0, 0.25).gain(0.7),
    // Synth lead.
    P::analog("Lead 1 (square)", Square).osc2(Square, 0.7, 6.0).lp(4000.0, 0.1).gain(0.45),
    P::analog("Lead 2 (sawtooth)", Saw).osc2(Saw, 0.8, 8.0).lp(5000.0, 0.15).gain(0.45),
    P::analog("Lead 3 (calliope)", Triangle).noise(0.1).lp(2500.0, 0.0).vibrato(5.5, 10.0, 0.2).gain(0.7),
    P::analog("Lead 4 (chiff)", Square).noise(0.3).lp(2500.0, 0.2).fenv(0.0, 0.05, 0.0, 0.1, 2.0).amp(0.002, 1.0, 0.8, 0.1).gain(0.5),
    P::analog("Lead 5 (charang)", Saw).osc2(Square, 0.6, 5.0).drive(4.0).lp(3000.0, 0.2).gain(0.5),
    P::analog("Lead 6 (voice)", Saw).osc2(Triangle, 0.5, 4.0).bp(900.0, 0.5).vibrato(5.0, 12.0, 0.2).fx(0.3, 0.3).gain(0.9),
    P::analog("Lead 7 (fifths)", Saw).osc2(Saw, 0.8, 0.0).ratio(1.4983).lp(3500.0, 0.15).gain(0.45),
    P::analog("Lead 8 (bass + lead)", Saw).osc2(Saw, 0.7, 4.0).ratio(0.5).lp(2000.0, 0.25).fenv(0.0, 0.4, 0.4, 0.1, 1.5).gain(0.5),
    // Synth pad.
    P::fm("Pad 1 (new age)", 2.0, 1.2).index_env(1.5, 0.3).tine(0.3, 4.0, 1.0, 0.0, 0.8).amp(0.3, 2.0, 0.7, 1.0).fx(0.6, 0.5).gain(0.8),
    P::analog("Pad 2 (warm)", Saw).osc2(Saw, 0.8, 7.0).lp(1200.0, 0.1).amp(0.4, 1.0, 1.0, 1.0).fx(0.6, 0.4).gain(0.45),
    P::analog("Pad 3 (polysynth)", Saw).osc2(Saw, 0.8, 10.0).lp(2500.0, 0.15).fenv(0.0, 0.5, 0.4, 0.4, 1.0).amp(0.01, 1.0, 1.0, 0.4).fx(0.5, 0.3).gain(0.45),
    P::analog("Pad 4 (choir)", Saw).osc2(Saw, 0.8, 6.0).bp(850.0, 0.35).amp(0.5, 1.0, 1.0, 1.0).vibrato(5.0, 10.0, 0.3).fx(0.6, 0.5).gain(0.9),
    P::analog("Pad 5 (bowed)", Saw).osc2(Triangle, 0.6, 4.0).lp(1500.0, 0.2).wobble(0.3).lfo(Lfo::Sine, 0.3, 0.0, 0.0).amp(0.5, 1.0, 1.0, 0.8).fx(0.4, 0.45).gain(0.5),
    P::fm("Pad 6 (metallic)", 3.5, 1.5).index_env(2.0, 0.6).amp(0.3, 1.0, 1.0, 1.5).fx(0.5, 0.5).gain(0.7),
    P::analog("Pad 7 (halo)", Triangle).osc2(Saw, 0.5, 6.0).bp(1200.0, 0.5).amp(0.6, 1.0, 1.0, 1.2).fx(0.6, 0.6).gain(0.9),
    P::analog("Pad 8 (sweep)", Saw).osc2(Saw, 0.8, 9.0).lp(500.0, 0.6).wobble(2.0).lfo(Lfo::Sine, 0.15, 0.0, 0.0).amp(0.3, 1.0, 1.0, 1.0).fx(0.5, 0.45).gain(0.45),
    // Synth effects.
    P::fm("FX 1 (rain)", 7.1, 2.0).index_env(0.1, 0.1).noise(0.15).tine(0.3, 5.0, 1.0, 0.0, 0.4).amp(0.002, 1.5, 0.1, 0.8).fx(0.3, 0.6).gain(0.6),
    P::analog("FX 2 (soundtrack)", Saw).osc2(Saw, 0.7, 9.0).lp(900.0, 0.3).fenv(1.5, 2.0, 0.6, 1.0, 1.5).tine(0.2, 2.0, 1.0, 0.0, 1.0).amp(0.6, 1.0, 1.0, 1.5).fx(0.5, 0.6).gain(0.45),
    P::fm("FX 3 (crystal)", 5.0, 1.5).index_env(0.3, 0.1).tine(0.5, 3.0, 1.0, 0.0, 1.0).amp(0.001, 2.5, 0.2, 1.0).fx(0.3, 0.5).gain(0.7),
    P::fm("FX 4 (atmosphere)", 1.0, 1.5).index_env(0.5, 0.4).tine(0.3, 2.0, 5.0, 0.7, 0.3).amp(0.05, 2.0, 0.5, 1.0).fx(0.6, 0.5).gain(0.8),
    P::analog("FX 5 (brightness)", Saw).osc2(Saw, 0.8, 8.0).lp(5000.0, 0.1).tine(0.3, 4.0, 1.0, 0.0, 1.0).amp(0.2, 1.0, 1.0, 0.8).fx(0.5, 0.5).gain(0.4),
    P::analog("FX 6 (goblins)", Saw).osc2(Saw, 0.8, 25.0).lp(600.0, 0.7).wobble(1.5).lfo(Lfo::Sine, 0.3, 0.0, 0.0).amp(1.0, 1.0, 1.0, 1.0).fx(0.3, 0.5).gain(0.45),
    P::analog("FX 7 (echoes)", Square).osc2(Saw, 0.5, 5.0).lp(1500.0, 0.2).amp(0.2, 1.0, 0.3, 1.0).fx(0.3, 0.8).gain(0.5),
    P::fm("FX 8 (sci-fi)", 1.41, 3.0).index_env(2.0, 0.5).amp(0.1, 1.0, 1.0, 1.0).lfo(Lfo::Sine, 3.0, 300.0, 0.0).fx(0.3, 0.5).gain(0.7),
    // Ethnic.
    P::pluck("Sitar", 3.0, 0.95).drive(2.0).tine(0.3, 2.0, 3.0, 1.0, 1.0).gain(0.8).fx(0.0, 0.3),
    P::pluck("Banjo", 1.0, 0.9).gain(0.9),
    P::pluck("Shamisen", 0.8, 0.85).gain(0.9),
    P::pluck("Koto", 1.8, 0.7).gain(0.9).fx(0.0, 0.3),
    P::fm("Kalimba", 1.0, 0.2).index_env(0.05, 0.0).tine(0.4, 6.0, 1.0, 0.0, 0.03).amp(0.001, 1.0, 0.0, 0.2).fx(0.0, 0.3),
    P::analog("Bag pipe", Saw).osc2(Saw, 0.6, 0.0).ratio(0.5).drive(2.0).bp(1500.0, 0.3).amp(0.05, 1.0, 1.0, 0.1).gain(0.8),
    solo_string("Fiddle", 3500.0).vibrato(6.0, 25.0, 0.15),
    P::analog("Shanai", Pulse).pw(0.2).bp(1600.0, 0.5).amp(0.03, 1.0, 1.0, 0.1).vibrato(5.5, 20.0, 0.2).fx(0.0, 0.25).gain(0.9),
    // Percussive.
    P::fm("Tinkle Bell", 3.6, 1.5).index_env(0.2, 0.0).amp(0.0, 1.2, 0.0, 0.3).fx(0.0, 0.3).gain(0.7),
    P::fm("Agogo", 2.7, 1.0).index_env(0.05, 0.0).amp(0.0, 0.5, 0.0, 0.1).fx(0.0, 0.2).gain(0.8),
    P::fm("Steel Drums", 2.0, 1.4).index_env(0.1, 0.0).tine(0.4, 3.0, 1.0, 0.0, 0.3).amp(0.001, 1.0, 0.0, 0.2).fx(0.0, 0.3),
    P::fm("Woodblock", 3.1, 1.0).index_env(0.01, 0.0).amp(0.0, 0.12, 0.0, 0.05),
    P::fm("Taiko Drum", 1.6, 1.0).index_env(0.05, 0.0).noise(0.4).lp(700.0, 0.0).amp(0.001, 0.6, 0.0, 0.3).fx(0.0, 0.35).gain(1.3),
    P::analog("Melodic Tom", Sine).sweep(7.0).fenv(0.0, 0.3, 0.0, 0.1, 0.0).amp(0.001, 0.5, 0.0, 0.2).fx(0.0, 0.3).gain(1.1),
    P::analog("Synth Drum", Triangle).noise(0.1).sweep(12.0).fenv(0.0, 0.2, 0.0, 0.1, 0.0).amp(0.001, 0.4, 0.0, 0.2).fx(0.0, 0.3).gain(1.1),
    P::analog("Reverse Cymbal", Noise).hp(6000.0, 0.0).amp(1.8, 0.05, 0.0, 0.05).fx(0.0, 0.3).gain(0.5),
    // Sound effects.
    P::analog("Guitar Fret Noise", Noise).bp(2500.0, 0.5).sweep(5.0).amp(0.005, 0.12, 0.0, 0.05).gain(0.7),
    P::analog("Breath Noise", Noise).lp(1500.0, 0.1).amp(0.1, 0.4, 0.3, 0.2).gain(0.6),
    P::analog("Seashore", Noise).lp(800.0, 0.2).wobble(2.0).lfo(Lfo::Sine, 0.12, 0.0, 0.0).amp(1.0, 1.0, 1.0, 2.0).fx(0.0, 0.5).gain(0.6),
    P::analog("Bird Tweet", Sine).transpose(24.0).lfo(Lfo::Saw, 12.0, 600.0, 0.3).amp(0.01, 0.4, 0.3, 0.1).fx(0.0, 0.4).gain(0.5),
    P::analog("Telephone Ring", Square).osc2(Square, 1.0, 0.0).ratio(1.25).lfo(Lfo::Square, 18.0, 0.0, 1.0).lp(3000.0, 0.0).gain(0.3),
    P::analog("Helicopter", Noise).osc2(Square, 0.3, 0.0).ratio(0.25).lp(400.0, 0.3).lfo(Lfo::Square, 12.0, 0.0, 1.0).amp(0.5, 1.0, 1.0, 0.8).gain(0.8),
    P::analog("Applause", Noise).bp(2000.0, 0.2).lfo(Lfo::Random, 17.0, 0.0, 0.6).amp(0.3, 1.0, 1.0, 1.0).fx(0.0, 0.5).gain(0.6),
    P::analog("Gunshot", Noise).osc2(Sine, 0.6, 0.0).ratio(0.25).lp(1500.0, 0.0).fenv(0.0, 0.3, 0.0, 0.2, 1.0).sweep(12.0).amp(0.0, 0.5, 0.0, 0.2).fx(0.0, 0.4).gain(1.2),
];
