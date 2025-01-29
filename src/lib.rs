//! Protobuf to Zod Schema Converter
//!
//! This library provides functionality to convert Protocol Buffer (protobuf) definitions
//! to Zod schemas. It includes a parser for protobuf files, an intermediate representation,
//! and a generator for Zod schemas.

use std::error::Error;
use std::fmt;

pub mod parser;
pub mod visitor;
pub mod generator;

/// Errors that can occur during the conversion process
#[derive(Debug)]
pub enum ConversionError {
    FileReadError(std::io::Error),
    ParseError(String),
    GenerationError(String),
}

impl fmt::Display for ConversionError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ConversionError::FileReadError(err) => write!(f, "File read error: {}", err),
            ConversionError::ParseError(msg) => write!(f, "Parse error: {}", msg),
            ConversionError::GenerationError(msg) => write!(f, "Generation error: {}", msg),
        }
    }
}

impl Error for ConversionError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            ConversionError::FileReadError(err) => Some(err),
            _ => None,
        }
    }
}

impl From<std::io::Error> for ConversionError {
    fn from(err: std::io::Error) -> Self {
        ConversionError::FileReadError(err)
    }
}
