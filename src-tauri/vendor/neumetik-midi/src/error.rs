// Copyright 2026 ansiapps. Neumetik MIDI, MIT licensed: see LICENSE.

use std::fmt;

/// Why a MIDI file or a sound bank couldn't be read.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Error {
    /// It doesn't start like a MIDI file (or a RIFF RMID holding one).
    NotMidi,
    /// It ends partway through its header.
    Truncated,
    /// Its header says a quarter note is no ticks long.
    BadTiming,
    /// It has no notes or other messages in it.
    NoMusic,
    /// It isn't a SoundFont or a DLS file.
    NotSoundBank,
    /// It's a SoundFont or DLS file, but damaged, or with no instruments
    /// in it that can be played.
    BadSoundBank,
    /// It's a compressed SoundFont (SF3, its samples Ogg Vorbis), which
    /// isn't read: use the SF2 it was made from.
    CompressedSoundFont,
    /// A SoundFont that can't be written: a zone plays a sample or
    /// instrument it hasn't, a stereo pair doesn't point at each other, a
    /// loop is outside its sample, or it's past what the format holds.
    Unwritable,
    /// It isn't a WAV file of PCM or float samples.
    NotWav,
    /// It isn't TMF text, or it was damaged ([`crate::tmf`]).
    NotTmf,
}

impl Error {
    /// The number the C API returns for it (always negative).
    pub fn code(self) -> i32 {
        match self {
            Error::NotMidi => -1,
            Error::Truncated => -2,
            Error::BadTiming => -3,
            Error::NoMusic => -4,
            Error::NotSoundBank => -6,
            Error::BadSoundBank => -7,
            Error::CompressedSoundFont => -8,
            Error::Unwritable => -9,
            Error::NotWav => -10,
            Error::NotTmf => -11,
        }
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Error::NotMidi => "it isn't a MIDI file",
            Error::Truncated => "it ends too soon",
            Error::BadTiming => "its timing is zero",
            Error::NoMusic => "it has no music in it",
            Error::NotSoundBank => "it isn't a SoundFont or a DLS file",
            Error::BadSoundBank => "it's damaged, or has no instruments that can be played",
            Error::CompressedSoundFont => "it's a compressed SoundFont (SF3), which can't be played: use an SF2",
            Error::Unwritable => "it can't be written as a SoundFont: something in it is missing or too big",
            Error::NotWav => "it isn't a WAV file of PCM or float samples",
            Error::NotTmf => "it isn't TMF music, or it was damaged on the way",
        })
    }
}

impl std::error::Error for Error {}

impl From<Error> for String {
    fn from(e: Error) -> String {
        e.to_string()
    }
}
