use std::{error::Error, fmt::Display};

#[derive(Debug, PartialEq)]
pub enum ProgramError {
    NameNotDetected(String),
    CannotReadDirectory(String),
    NotEnoughUnderscores(String),
    StringNotNumber(String),
    AsciiErrorInFileName(String),
    NoPath(String),
    FailedToDelete(String),
    FailedToReadDir(String),
}
impl Error for ProgramError {}
impl Display for ProgramError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ProgramError::NameNotDetected(e) => write!(f, "{:#?}", e),
            ProgramError::NotEnoughUnderscores(e) => write!(f, "{:#?}", e),
            ProgramError::StringNotNumber(e) => write!(f, "{:#?}", e),
            ProgramError::AsciiErrorInFileName(e) => write!(f, "{:#?}", e),
            ProgramError::NoPath(e) => write!(f, "{:#?}", e),
            ProgramError::CannotReadDirectory(e) => write!(f, "{:#?}", e),
            ProgramError::FailedToDelete(e) => write!(f, "{:#?}", e),
            ProgramError::FailedToReadDir(e) => write!(f, "{:#?}", e),
        }
    }
}
