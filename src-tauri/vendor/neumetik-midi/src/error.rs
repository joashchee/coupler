// Copyright 2026 ansiapps. Neumetik MIDI, MIT licensed: see LICENSE.

use std::fmt;

/// Why a MIDI file couldn't be read.
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
}

impl Error {
    /// The number the C API returns for it (always negative).
    pub fn code(self) -> i32 {
        match self {
            Error::NotMidi => -1,
            Error::Truncated => -2,
            Error::BadTiming => -3,
            Error::NoMusic => -4,
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
        })
    }
}

impl std::error::Error for Error {}

impl From<Error> for String {
    fn from(e: Error) -> String {
        e.to_string()
    }
}
