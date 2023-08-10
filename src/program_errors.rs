use std::{error::Error, fmt::Display};

#[derive(Debug, PartialEq)]
pub enum ProgramError {
    NameNotDetected(String),
    NotEnoughUnderscores(String),
    StringNotNumber(String),
    AsciiErrorInFileName(String),
}
impl Error for ProgramError {}
impl Display for ProgramError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ProgramError::NameNotDetected(e) => write!(f, "{:#?}", e),
            ProgramError::NotEnoughUnderscores(e) => write!(f, "{:#?}", e),
            ProgramError::StringNotNumber(e) => write!(f, "{:#?}", e),
            ProgramError::AsciiErrorInFileName(e) => write!(f, "{:#?}", e),
        }
    }
}
