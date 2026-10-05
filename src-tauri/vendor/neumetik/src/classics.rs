// Copyright 2026 ansiapps. Neumetik: see README.md for its license.

//! Neumetik's classic banks: the synthesizer sounds everyone knows, from
//! the 1980s to now, each rebuilt from Neumetik's engines (not sampled,
//! not copied: the same idea, made again). Bank Select MSB 80 is the
//! eighties, 81 the nineties, 82 the 2000s to now; the program is the
//! patch's place in its list. Each says what it's after.

use super::patch::Lfo;
use super::patch::Patch as P;
use super::patch::Wave::*;

/// Bank 80: the 1980s, digital FM and the great analog polysynths.
pub const EIGHTIES: &[P] = &[
    P::fm("Solid Tine EP", 1.0, 1.1).index_env(1.2, 0.25).tine(0.7, 1.0, 14.0, 1.2, 0.3).detune(4.0).amp(0.001, 5.0, 0.0, 0.3).key_decay(0.7).fx(0.5, 0.25)
        .after("Yamaha DX7 (1983), E.PIANO 1: the ballad piano of the decade"),
    P::fm("FM Brass Bass", 1.0, 3.0).index_env(0.25, 0.25).feedback(0.7).amp(0.001, 1.0, 0.7, 0.08).gain(0.9)
        .after("Yamaha DX7, BASS 1 (\"Take My Breath Away\", 1986)"),
    P::fm("FM Tubular Bells", 3.5, 3.0).index_env(2.0, 0.15).tine(0.4, 2.76, 5.4, 1.2, 2.5).amp(0.001, 7.0, 0.0, 1.5).fx(0.0, 0.45).gain(0.7)
        .after("Yamaha DX7, TUB BELLS"),
    P::fm("FM Marimba", 1.0, 0.6).index_env(0.08, 0.0).tine(0.5, 4.0, 1.0, 0.0, 0.05).amp(0.001, 0.7, 0.0, 0.2).fx(0.0, 0.3)
        .after("Yamaha DX7, MARIMBA"),
    P::fm("Lately Bass", 1.0, 2.5).index_env(0.4, 0.3).feedback(0.9).lp(1800.0, 0.2).amp(0.001, 1.5, 0.6, 0.06).gain(0.9)
        .after("Yamaha TX81Z (1987), LatelyBass: house and new jack swing"),
    P::analog("Sync Lead", Saw).osc2(Saw, 1.0, 0.0).ratio(1.0).sync(14.0).fenv(0.0, 0.6, 0.2, 0.3, 0.0).lp(5000.0, 0.1).amp(0.003, 1.0, 1.0, 0.15).gain(0.45)
        .after("Sequential Prophet-5, Sync (The Cars, \"Let's Go\")"),
    P::analog("Jump Brass", Saw).osc2(Saw, 0.9, 10.0).lp24(900.0, 0.15).fenv(0.01, 0.6, 0.6, 0.3, 2.5).amp(0.01, 1.0, 1.0, 0.25).fx(0.6, 0.25).gain(0.5)
        .after("Oberheim OB-Xa (Van Halen, \"Jump\", 1984)"),
    P::analog("Poly Brass", Saw).osc2(Saw, 0.8, 12.0).lp24(600.0, 0.2).fenv(0.12, 0.5, 0.5, 0.3, 3.0).amp(0.06, 1.0, 1.0, 0.3).vibrato(5.0, 6.0, 0.5).fx(0.4, 0.3).gain(0.5)
        .after("Sequential Prophet-5 and Oberheim brass"),
    P::analog("Chorus Strings", Saw).osc2(Pulse, 0.7, 0.0).pw(0.3).pwm(0.8).lfo(Lfo::Sine, 0.5, 0.0, 0.0).lp(2500.0, 0.1).amp(0.25, 1.0, 1.0, 0.7).fx(0.9, 0.3).gain(0.45)
        .after("Roland Juno-60 (1982): a saw and a moving pulse through its chorus"),
    P::analog("Pulse Sub Bass", Pulse).pw(0.4).osc2(Square, 0.8, 0.0).ratio(0.5).lp24(500.0, 0.35).fenv(0.0, 0.3, 0.3, 0.1, 2.0).amp(0.002, 1.0, 1.0, 0.08).gain(0.6)
        .after("Roland Juno-106 (1984), a sub-oscillator bass"),
    P::analog("Ladder Lead", Saw).osc2(Saw, 0.8, 7.0).lp24(1500.0, 0.35).fenv(0.0, 0.4, 0.6, 0.2, 1.5).vibrato(5.5, 15.0, 0.4).drive(0.8).gain(0.5)
        .after("Moog Minimoog: three oscillators and the ladder filter"),
    P::analog("Ladder Bass", Saw).osc2(Square, 0.8, 4.0).ratio(0.5).lp24(300.0, 0.3).fenv(0.0, 0.25, 0.3, 0.1, 2.5).amp(0.002, 1.0, 1.0, 0.06).drive(0.8).gain(0.6)
        .after("Moog Minimoog, the fat bass"),
    P::analog("Acid Bass", Saw).lp24(350.0, 0.85).fenv(0.0, 0.2, 0.0, 0.1, 3.5).vel_filter(2.0).amp(0.001, 1.0, 0.9, 0.05).drive(1.5).gain(0.6)
        .after("Roland TB-303 (1981), the squelch of acid house (1987 on)"),
    P::analog("Vox Humana", Saw).unison(4, 18.0).bp(1100.0, 0.25).amp(0.15, 1.0, 1.0, 0.6).vibrato(5.0, 8.0, 0.2).fx(0.7, 0.4).gain(0.8)
        .after("Moog Polymoog, Vox Humana (Gary Numan, \"Cars\")"),
    P::analog("Wave Choir", Saw).osc2(Triangle, 0.7, 5.0).bp(800.0, 0.45).wobble(0.3).lfo(Lfo::Sine, 0.4, 0.0, 0.0).amp(0.2, 1.0, 1.0, 0.7).fx(0.6, 0.5).gain(0.9)
        .after("PPG Wave 2.2, its wavetable choir"),
    P::fm("Fantasia", 2.0, 1.5).index_env(0.6, 0.2).tine(0.6, 3.0, 1.0, 0.0, 0.8).amp(0.01, 2.0, 0.6, 1.2).fx(0.7, 0.55).gain(0.7)
        .after("Roland D-50 (1987), Fantasia: a bell on a pad"),
    P::analog("Soundtrack Pad", Saw).osc2(Saw, 0.7, 9.0).lp(800.0, 0.3).fenv(2.0, 2.5, 0.6, 1.5, 1.8).tine(0.15, 2.0, 1.0, 0.0, 1.0).amp(0.8, 1.0, 1.0, 2.0).fx(0.6, 0.65).gain(0.45)
        .after("Roland D-50, Soundtrack"),
    P::fm("Digital Pluck", 1.0, 2.5).index_env(0.15, 0.0).tine(0.5, 2.0, 1.0, 0.0, 0.4).amp(0.001, 1.2, 0.0, 0.3).fx(0.6, 0.4).gain(0.8)
        .after("Roland D-50, Digital Native Dance"),
    P::fm("House Piano", 1.0, 2.3).index_env(0.3, 0.2).tine(0.4, 2.0, 1.0, 1.0, 0.2).amp(0.001, 2.0, 0.0, 0.2).key_decay(0.6).fx(0.2, 0.25)
        .after("Korg M1 (1988), Piano 8': every house piano of the early nineties"),
    P::organ("House Organ", [0.0, 0.9, 0.0, 0.9, 0.0, 0.3, 0.0]).tine(0.5, 3.0, 1.0, 0.0, 0.08).amp(0.002, 0.4, 0.4, 0.08).gain(0.9)
        .after("Korg M1, Organ 2 (Robin S, \"Show Me Love\")"),
    P::analog("Universe Pad", Saw).osc2(Triangle, 0.6, 7.0).lp(1800.0, 0.2).tine(0.25, 3.0, 2.0, 1.0, 1.5).amp(0.5, 1.0, 1.0, 1.5).fx(0.7, 0.6).gain(0.45)
        .after("Korg M1, Universe"),
    P::fm("Gong Hit", 1.4, 4.0).index_env(3.0, 0.3).feedback(0.3).amp(0.001, 6.0, 0.0, 2.0).transpose(-12.0).fx(0.0, 0.6).gain(0.6)
        .after("New England Digital Synclavier, the gong (\"Beat It\")"),
    P::analog("Orch Hit", Saw).unison(7, 25.0).osc2(Square, 0.5, 0.0).ratio(0.5).noise(0.3).lp(5000.0, 0.1).fenv(0.0, 0.3, 0.0, 0.2, -2.5).amp(0.0, 0.45, 0.0, 0.2).fx(0.0, 0.45).gain(0.8)
        .after("Fairlight CMI, ORCH5"),
    P::analog("Breathy Shakuhachi", Triangle).osc2(Sine, 0.5, 0.0).noise(0.35).bp(1500.0, 0.2).sweep(-1.0).fenv(0.0, 0.15, 0.0, 0.1, 0.0).amp(0.03, 1.0, 1.0, 0.2).vibrato(5.0, 20.0, 0.3).fx(0.0, 0.4).gain(0.8)
        .after("E-mu Emulator II, Shakuhachi (Peter Gabriel, \"Sledgehammer\")"),
    P::analog("Warm Poly Pad", Saw).osc2(Saw, 0.9, 8.0).lp24(1500.0, 0.2).fenv(0.3, 1.5, 0.5, 1.0, 1.0).amp(0.2, 1.0, 1.0, 0.8).fx(0.5, 0.4).gain(0.5)
        .after("Roland Jupiter-8 (1981)"),
    P::analog("Poly Arp", Square).osc2(Saw, 0.7, 6.0).lp24(1800.0, 0.4).fenv(0.0, 0.15, 0.1, 0.1, 2.0).amp(0.001, 0.4, 0.2, 0.1).fx(0.3, 0.3).gain(0.5)
        .after("Roland Jupiter-8's arpeggiator (Duran Duran, \"Rio\")"),
    P::analog("Blade Brass", Saw).osc2(Saw, 0.9, 9.0).lp(700.0, 0.3).fenv(0.3, 1.5, 0.6, 1.0, 2.0).amp(0.15, 1.0, 1.0, 1.2).vibrato(5.0, 10.0, 0.8).fx(0.6, 0.65).gain(0.5)
        .after("Yamaha CS-80 (Vangelis, Blade Runner)"),
    P::analog("Syn Tom", Triangle).noise(0.15).sweep(14.0).fenv(0.0, 0.4, 0.0, 0.1, 0.0).lp(2000.0, 0.0).amp(0.001, 0.8, 0.0, 0.3).fx(0.0, 0.35).gain(1.2)
        .after("Simmons SDS-V and the Syndrum"),
    P::analog("Countdown Brass", Saw).osc2(Saw, 0.9, 15.0).unison(3, 10.0).lp24(1200.0, 0.15).fenv(0.02, 0.4, 0.7, 0.3, 2.0).amp(0.02, 1.0, 1.0, 0.25).fx(0.6, 0.3).gain(0.45)
        .after("Yamaha TX816 and Korg Polysix (Europe, \"The Final Countdown\")"),
    P::analog("Jump Square", Square).osc2(Square, 0.8, 7.0).lp(3000.0, 0.2).fenv(0.0, 0.3, 0.6, 0.1, 1.0).vibrato(6.0, 10.0, 0.4).gain(0.45)
        .after("Roland SH-101 (1982), a mono lead"),
    P::analog("Rubber Bass", Pulse).pw(0.2).lp24(400.0, 0.5).fenv(0.0, 0.12, 0.0, 0.05, 3.0).amp(0.001, 0.5, 0.5, 0.05).gain(0.65)
        .after("Roland SH-101, the electro bass"),
    P::analog("Tears Pad", Pulse).pw(0.25).pwm(0.7).lfo(Lfo::Sine, 0.8, 0.0, 0.0).osc2(Saw, 0.5, 6.0).lp(1600.0, 0.2).amp(0.35, 1.0, 1.0, 0.8).fx(0.8, 0.45).gain(0.5)
        .after("Roland Juno-106, pulse width pads (synth-pop)"),
];

/// Bank 81: the 1990s, rave, house, trance, jungle and G-funk.
pub const NINETIES: &[P] = &[
    P::analog("Hoover", Saw).unison(5, 45.0).osc2(Saw, 0.7, 0.0).ratio(0.5).pwm(0.0).lp(4000.0, 0.1).sweep(-3.0).fenv(0.0, 0.25, 0.0, 0.2, 0.0).vibrato(6.0, 25.0, 0.0).fx(0.9, 0.3).gain(0.5)
        .after("Roland Alpha Juno, \"What the...\" (Human Resource, \"Dominator\"; The Prodigy)"),
    P::analog("Mentasm Stab", Saw).unison(5, 40.0).osc2(Saw, 0.8, 0.0).ratio(2.0).lp(3000.0, 0.3).fenv(0.0, 0.3, 0.3, 0.2, 1.0).sweep(-2.0).amp(0.001, 0.8, 0.5, 0.25).fx(0.8, 0.4).gain(0.5)
        .after("Roland Alpha Juno (Second Phase, \"Mentasm\", 1991)"),
    P::analog("Supersaw", Saw).unison(7, 35.0).lp(6000.0, 0.1).amp(0.005, 1.0, 1.0, 0.4).fx(0.5, 0.4).gain(0.5)
        .after("Roland JP-8000 (1996), the supersaw: every trance anthem"),
    P::analog("Reese Bass", Saw).osc2(Saw, 1.0, 22.0).lp24(600.0, 0.2).amp(0.003, 1.0, 1.0, 0.1).drive(0.8).gain(0.75)
        .after("Kevin Saunderson as Reese, \"Just Want Another Chance\" (1988): drum and bass"),
    P::fm("Rave Piano", 1.0, 2.8).index_env(0.25, 0.2).tine(0.5, 2.0, 1.0, 1.5, 0.2).amp(0.001, 1.5, 0.0, 0.15).fx(0.2, 0.35).gain(0.9)
        .after("Korg M1's piano, chopped into rave stabs"),
    P::analog("Chord Stab", Saw).osc2(Saw, 0.9, 12.0).lp(2500.0, 0.3).fenv(0.0, 0.2, 0.0, 0.1, 1.5).amp(0.001, 0.4, 0.0, 0.15).fx(0.5, 0.5).gain(0.5)
        .after("Detroit techno and deep house's chord memory stabs"),
    P::organ("Dance Organ Bass", [0.9, 0.9, 0.0, 0.7, 0.0, 0.0, 0.0]).tine(0.4, 2.0, 1.0, 0.0, 0.05).amp(0.002, 0.3, 0.3, 0.06).gain(0.9)
        .after("Korg M1 organ played as a bass (Eurodance)"),
    P::analog("Glass Pad", Triangle).osc2(Saw, 0.4, 7.0).hp(300.0, 0.0).tine(0.4, 4.0, 1.0, 0.0, 2.0).amp(0.4, 1.0, 1.0, 1.5).fx(0.7, 0.65).gain(0.6)
        .after("Roland JD-800 (1991), its crystal pads"),
    P::analog("Vector Spectrum", Saw).osc2(Square, 0.6, 5.0).bp(1200.0, 0.5).wobble(1.5).lfo(Lfo::Sine, 0.25, 0.0, 0.0).amp(0.4, 1.0, 1.0, 1.5).fx(0.7, 0.6).gain(0.6)
        .after("Korg Wavestation (1990), a wave sequence moving"),
    P::analog("Goa Lead", Saw).osc2(Saw, 0.8, 12.0).lp24(1500.0, 0.6).wobble(1.0).lfo(Lfo::Sine, 3.0, 0.0, 0.0).fenv(0.0, 0.3, 0.4, 0.2, 1.5).fx(0.4, 0.4).gain(0.5)
        .after("Psytrance's resonant leads, the Nord Lead and JP-8000"),
    P::analog("Trance Pluck", Saw).unison(5, 25.0).lp(800.0, 0.3).fenv(0.0, 0.25, 0.0, 0.1, 3.0).amp(0.001, 0.5, 0.0, 0.3).fx(0.5, 0.5).gain(0.85)
        .after("Access Virus and JP-8000 plucks"),
    P::analog("Gated Trance Pad", Saw).unison(5, 30.0).lp(3500.0, 0.1).lfo(Lfo::Square, 8.0, 0.0, 1.0).amp(0.05, 1.0, 1.0, 0.3).fx(0.5, 0.5).gain(0.5)
        .after("The trance gate of the late nineties"),
    P::analog("Nord Sync", Saw).osc2(Saw, 1.0, 0.0).sync(20.0).fenv(0.0, 0.3, 0.3, 0.2, 0.0).lp(4500.0, 0.2).amp(0.002, 1.0, 1.0, 0.15).gain(0.45)
        .after("Clavia Nord Lead (1995), its sync sweep"),
    P::analog("Hypersaw Lead", Saw).unison(7, 28.0).osc2(Saw, 0.6, 0.0).ratio(2.0).lp(5000.0, 0.2).vibrato(5.0, 12.0, 0.3).fx(0.5, 0.4).gain(0.45)
        .after("Access Virus (1997), Hypersaw"),
    P::analog("808 Boom", Sine).sweep(12.0).fenv(0.0, 0.08, 0.0, 0.1, 0.0).amp(0.001, 2.5, 0.0, 0.3).drive(0.6).gain(1.2)
        .after("Roland TR-808's kick, tuned and played as a bass (jungle, hip hop)"),
    P::analog("G-Funk Whistle", Sine).osc2(Triangle, 0.2, 0.0).amp(0.04, 1.0, 1.0, 0.2).vibrato(5.5, 30.0, 0.3).fx(0.0, 0.35).gain(0.7)
        .after("Minimoog sine leads (Dr. Dre, \"Nuthin' but a 'G' Thang\")"),
    P::analog("Deep House Bass", Square).osc2(Sine, 0.8, 0.0).ratio(0.5).lp(500.0, 0.3).fenv(0.0, 0.2, 0.3, 0.1, 1.0).amp(0.002, 1.0, 1.0, 0.08).gain(0.65)
        .after("Juno-106 and Korg bass modules in deep house"),
    P::analog("Ambient Drone", Saw).osc2(Saw, 0.8, 5.0).ratio(1.5).lp(700.0, 0.4).wobble(1.0).lfo(Lfo::Sine, 0.07, 0.0, 0.0).amp(2.0, 1.0, 1.0, 3.0).fx(0.7, 0.8).gain(0.45)
        .after("The Orb and Aphex Twin's ambient pads"),
    P::analog("Euro Pizz", Saw).osc2(Square, 0.5, 6.0).lp(2500.0, 0.2).fenv(0.0, 0.1, 0.0, 0.1, 2.0).amp(0.001, 0.25, 0.0, 0.1).fx(0.4, 0.4).gain(0.5)
        .after("Eurodance's pizzicato synth riffs"),
    P::analog("Acid Squelch", Saw).lp24(500.0, 0.9).fenv(0.0, 0.15, 0.0, 0.1, 4.0).vel_filter(3.0).amp(0.001, 1.0, 0.8, 0.05).drive(3.0).gain(0.55)
        .after("A TB-303 overdriven (Josh Wink, \"Higher State of Consciousness\")"),
    P::analog("Big Beat Saw", Saw).osc2(Square, 0.8, 10.0).drive(5.0).lp(1800.0, 0.5).wobble(0.6).lfo(Lfo::Sine, 2.0, 0.0, 0.0).gain(0.45)
        .after("The Chemical Brothers' distorted synths"),
    P::fm("Wurly Keys", 1.0, 1.5).index_env(0.4, 0.3).feedback(0.3).tine(0.3, 1.0, 5.0, 1.0, 0.2).amp(0.002, 3.0, 0.0, 0.2).lfo(Lfo::Sine, 5.5, 0.0, 0.15).fx(0.2, 0.25)
        .after("The Wurlitzer electric piano in trip hop"),
    P::fm("Trinity Bell Pad", 3.0, 1.2).index_env(1.0, 0.4).tine(0.3, 7.0, 1.0, 0.0, 1.0).amp(0.3, 2.0, 0.8, 1.5).fx(0.7, 0.6).gain(0.6)
        .after("Korg Trinity (1995) and Triton bell pads"),
    P::analog("Jungle Sub", Sine).osc2(Triangle, 0.3, 0.0).amp(0.003, 1.0, 1.0, 0.1).gain(1.0)
        .after("The sine sub-bass of jungle"),
];

/// Bank 82: the 2000s to now, EDM, dubstep, trap, synthwave and lo-fi.
pub const NOW: &[P] = &[
    P::analog("Wobble Bass", Saw).osc2(Square, 0.8, 8.0).ratio(0.5).lp24(300.0, 0.6).wobble(3.5).lfo(Lfo::Sine, 3.0, 0.0, 0.0).drive(2.0).gain(0.55)
        .after("Native Instruments Massive (2007), the dubstep wobble"),
    P::fm("Growl Bass", 1.0, 4.0).index_env(0.5, 0.6).feedback(1.2).bp(800.0, 0.6).wobble(2.0).lfo(Lfo::Saw, 6.0, 0.0, 0.0).drive(4.0).gain(0.6)
        .after("FM growls in Serum and Operator (brostep, 2010s)"),
    P::analog("Neuro Reese", Saw).osc2(Saw, 1.0, 30.0).unison(3, 20.0).lp24(700.0, 0.5).wobble(1.2).lfo(Lfo::Sine, 0.5, 0.0, 0.0).drive(3.0).gain(0.45)
        .after("Neurofunk drum and bass"),
    P::analog("Future Bass Chords", Saw).unison(7, 40.0).lp(4500.0, 0.1).lfo(Lfo::Saw, 2.0, 0.0, 0.8).amp(0.01, 1.0, 1.0, 0.3).vibrato(5.0, 10.0, 0.0).fx(0.6, 0.45).gain(0.5)
        .after("Xfer Serum supersaws, sidechained (Flume, 2014)"),
    P::analog("Big Room Lead", Saw).unison(7, 45.0).osc2(Saw, 0.6, 0.0).ratio(2.0).lp(7000.0, 0.1).fx(0.5, 0.5).gain(0.45)
        .after("Sylenth1 and Spire leads of EDM's big room"),
    P::analog("Prog Pluck", Saw).osc2(Saw, 0.8, 12.0).lp24(600.0, 0.4).fenv(0.0, 0.2, 0.0, 0.1, 3.5).amp(0.001, 0.4, 0.0, 0.2).fx(0.3, 0.5).gain(0.5)
        .after("LennarDigital Sylenth1 (2006) plucks (deadmau5)"),
    P::fm("Tropical Pluck", 1.0, 0.8).index_env(0.1, 0.0).tine(0.5, 4.0, 1.0, 0.0, 0.08).amp(0.001, 0.6, 0.0, 0.2).fx(0.3, 0.45)
        .after("Tropical house's marimba plucks (Kygo, 2015)"),
    P::analog("808 Glide", Sine).osc2(Triangle, 0.25, 0.0).sweep(5.0).fenv(0.0, 0.06, 0.0, 0.1, 0.0).amp(0.001, 3.0, 0.0, 0.2).drive(2.5).gain(1.1)
        .after("The trap 808, distorted and long"),
    P::analog("Hyperpop Lead", Square).osc2(Saw, 0.7, 15.0).ratio(2.0).hp(200.0, 0.0).vibrato(6.0, 18.0, 0.0).drive(1.5).fx(0.5, 0.3).gain(0.4)
        .after("SOPHIE and A. G. Cook's bright, plastic leads"),
    P::analog("Synthwave Lead", Saw).osc2(Square, 0.7, 7.0).lp(3000.0, 0.25).vibrato(5.0, 15.0, 0.4).fx(0.7, 0.5).gain(0.45)
        .after("Synthwave's eighties revival (Kavinsky, \"Nightcall\")"),
    P::analog("Synthwave Bass", Saw).osc2(Saw, 0.8, 8.0).lp24(700.0, 0.3).fenv(0.0, 0.15, 0.2, 0.1, 2.0).amp(0.001, 0.6, 0.6, 0.08).gain(0.55)
        .after("Synthwave's rolling sixteenth-note bass"),
    P::fm("Lo-fi Keys", 1.0, 1.0).index_env(0.8, 0.2).tine(0.3, 1.0, 14.0, 0.8, 0.2).lp(1800.0, 0.0).lfo(Lfo::Sine, 0.6, 18.0, 0.1).amp(0.002, 3.5, 0.0, 0.3).noise(0.01).fx(0.3, 0.35)
        .after("Lo-fi hip hop's warbling Rhodes"),
    P::analog("Vapor Pad", Saw).osc2(Triangle, 0.6, 9.0).lp(1100.0, 0.2).lfo(Lfo::Sine, 0.3, 12.0, 0.0).amp(0.6, 1.0, 1.0, 1.5).transpose(-1.0).fx(0.8, 0.7).gain(0.5)
        .after("Vaporwave's slowed, detuned pads"),
    P::analog("Dirty Electro", Saw).osc2(Square, 0.9, 20.0).drive(6.0).lp24(1500.0, 0.5).fenv(0.0, 0.2, 0.3, 0.1, 1.5).gain(0.45)
        .after("Electro house's distorted saws (Justice, Deadmau5's \"Ghosts\")"),
    P::analog("Filter House Chords", Saw).osc2(Saw, 0.8, 8.0).lp24(500.0, 0.5).wobble(2.5).lfo(Lfo::Sine, 0.12, 0.0, 0.0).fx(0.4, 0.3).gain(0.5)
        .after("French house's filter sweeps (Daft Punk, Stardust)"),
    P::analog("Uplifting Lead", Saw).unison(7, 30.0).osc2(Square, 0.5, 0.0).ratio(2.0).lp(5500.0, 0.15).vibrato(5.5, 15.0, 0.4).fx(0.5, 0.6).gain(0.45)
        .after("Uplifting trance leads of the 2000s"),
    P::analog("Hardstyle Screech", Saw).unison(5, 50.0).bp(1800.0, 0.6).drive(7.0).sweep(2.0).fenv(0.0, 0.1, 0.0, 0.1, 0.0).fx(0.4, 0.4).gain(0.45)
        .after("Hardstyle's screeching leads"),
    P::analog("Riser", Noise).osc2(Saw, 0.4, 25.0).hp(400.0, 0.4).fenv(4.0, 0.1, 1.0, 0.3, 4.0).sweep(-12.0).amp(4.0, 1.0, 1.0, 0.3).fx(0.4, 0.6).gain(0.5)
        .after("The EDM build-up's noise riser"),
    P::analog("Sub Sine", Sine).amp(0.003, 1.0, 1.0, 0.08).gain(1.1)
        .after("The pure sine sub under modern bass music"),
    P::analog("Shimmer Pad", Saw).unison(5, 20.0).hp(600.0, 0.0).tine(0.4, 2.0, 1.0, 0.0, 3.0).amp(1.0, 1.0, 1.0, 2.5).fx(0.8, 0.9).gain(0.4)
        .after("Shimmer reverb pads of ambient and post-rock"),
    P::analog("Vowel Lead", Saw).osc2(Saw, 0.7, 6.0).bp(900.0, 0.7).wobble(1.2).lfo(Lfo::Sine, 1.5, 0.0, 0.0).fx(0.3, 0.3).gain(0.8)
        .after("Talk-box style formant leads (Daft Punk, \"Digital Love\")"),
    P::analog("Chiptune Pulse", Pulse).pw(0.125).amp(0.0, 1.0, 1.0, 0.02).gain(0.4)
        .after("The Game Boy's 12.5% pulse (LSDJ chiptune)"),
    P::analog("Chiptune Triangle", Triangle).amp(0.0, 1.0, 1.0, 0.02).gain(0.7)
        .after("The NES's triangle bass"),
    P::fm("Glass Bell Pluck", 3.0, 2.0).index_env(0.15, 0.0).tine(0.4, 7.0, 1.0, 0.0, 0.3).amp(0.001, 1.2, 0.0, 0.5).fx(0.4, 0.6).gain(0.7)
        .after("Operator and FM8 bell plucks"),
    P::analog("Wonky Bass", Square).osc2(Saw, 0.6, 15.0).lp24(600.0, 0.4).vibrato(4.0, 25.0, 0.0).fenv(0.0, 0.2, 0.3, 0.1, 1.5).gain(0.5)
        .after("Wonky and beat-scene detuned synth bass (Flying Lotus)"),
    P::analog("Phonk Cowbell", Square).osc2(Square, 1.0, 0.0).ratio(1.48).bp(1200.0, 0.3).amp(0.0, 0.5, 0.0, 0.1).drive(2.0).gain(0.5)
        .after("Drift phonk's 808 cowbell melodies (2020s)"),
];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_classic_says_what_its_after() {
        for p in EIGHTIES.iter().chain(NINETIES).chain(NOW) {
            assert!(!p.name.is_empty() && !p.after.is_empty(), "{}", p.name);
        }
    }
}
