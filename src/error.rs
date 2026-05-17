// SPDX-License-Identifier: GPL-3.0-or-later

//! Representation of all possible errors that may be raised when using this tool.
//!

use std::{
    fmt::{Display, Formatter, Result},
    io,
    num::ParseIntError,
};

/// Enumeration of different types of errors that may be raised while using
/// this tool.
#[derive(Debug)]
pub enum MyError {
    Logging(log::SetLoggerError),
    Config(dotenvy::Error),
    IO(io::Error),
    Parse(ParseIntError),
    DateTime(jiff::Error),
    Command((/* cmd */ String, /* stderr */ String)),
    Runtime(String),
}

impl Display for MyError {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        match self {
            MyError::Logging(x) => write!(f, "Log setup failed: {:?}", x),
            MyError::Config(x) => write!(f, "Load config parameters error: {:?}", x),
            MyError::IO(x) => write!(f, "I/O error: {:?}", x),
            MyError::Parse(x) => write!(f, "Parse integer error: {:?}", x),
            MyError::DateTime(x) => write!(f, "Date-time error: {:?}", x),
            MyError::Command(x) => write!(f, "Command (`{}`) failed: {}", x.0, x.1),
            MyError::Runtime(x) => write!(f, "Runtime error: {:}", x),
        }
    }
}

impl From<log::SetLoggerError> for MyError {
    fn from(value: log::SetLoggerError) -> Self {
        MyError::Logging(value)
    }
}

impl From<dotenvy::Error> for MyError {
    fn from(value: dotenvy::Error) -> Self {
        MyError::Config(value)
    }
}

impl From<std::io::Error> for MyError {
    fn from(value: std::io::Error) -> Self {
        Self::IO(value)
    }
}

impl From<ParseIntError> for MyError {
    fn from(value: ParseIntError) -> Self {
        Self::Parse(value)
    }
}

impl From<jiff::Error> for MyError {
    fn from(value: jiff::Error) -> Self {
        Self::DateTime(value)
    }
}
