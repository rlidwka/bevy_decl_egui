use thiserror::Error;

use super::reader::Reader;

#[derive(Error, Debug)]
pub enum Error {
    #[error("invalid type {actual}, expected {expected} (at {at})")]
    InvalidType { actual: String, expected: String, at: String },
    #[error("invalid value {actual}, expected {expected} (at {at})")]
    InvalidValue { actual: String, expected: String, at: String },
    #[error("invalid length {actual}, expected {expected} (at {at})")]
    InvalidLength { actual: usize, expected: String, at: String },
    #[error("unknown variant {actual}, expected one of {expected} (at {at})")]
    UnknownVariant { actual: String, expected: String, at: String },
    #[error("unknown field `{field}`, expected one of {expected} (at {at})")]
    UnknownField { field: String, expected: String, at: String },
    #[error("duplicate field `{field}` (at {at})")]
    DuplicateField { field: String, at: String },
    #[error("missing field `{field}` (at {at})")]
    MissingField { field: String, at: String },
    #[error("unexpected operator `{op}` (at {at})")]
    UnexpectedOperator { op: String, at: String },
    #[error("unexpected remainder `{remainder}` (at {at})")]
    UnexpectedRemainder { remainder: String, at: String },
    #[error("failed to deserialize: {error} (at {at})")]
    DeserializeError {
        error: jomini::DeserializeError,
        at: String,
    },
    #[error("failed to parse: {error} (at {at})")]
    ScalarError {
        error: jomini::ScalarError,
        at: String,
    },
    #[error("{message} (at {at})")]
    Custom {
        message: String,
        at: String,
    },
}

impl Error {
    pub fn invalid_type(reader: &Reader, actual: &str, expected: &str) -> Self {
        Error::InvalidType {
            actual: actual.to_owned(),
            expected: expected.to_owned(),
            at: reader.path(),
        }
    }

    pub fn invalid_value(reader: &Reader, actual: &str, expected: &str) -> Self {
        Error::InvalidValue {
            actual: actual.to_owned(),
            expected: expected.to_owned(),
            at: reader.path(),
        }
    }

    pub fn invalid_length(reader: &Reader, actual: usize, expected: &str) -> Self {
        Error::InvalidLength {
            actual,
            expected: expected.to_owned(),
            at: reader.path(),
        }
    }

    pub fn unknown_variant(reader: &Reader, actual: &str, expected: &'static [&'static str]) -> Self {
        Error::UnknownVariant {
            actual: actual.to_owned(),
            expected: expected
                .iter()
                .map(|s| format!("`{}`", s))
                .collect::<Vec<_>>()
                .join(", "),
            at: reader.path(),
        }
    }

    pub fn unknown_field(reader: &Reader, field: &str, expected: &'static [&'static str]) -> Self {
        Error::UnknownField {
            field: field.to_owned(),
            expected: expected
                .iter()
                .map(|s| format!("`{}`", s))
                .collect::<Vec<_>>()
                .join(", "),
            at: reader.path(),
        }
    }

    pub fn duplicate_field(reader: &Reader, field: &str) -> Self {
        Error::DuplicateField {
            field: field.to_owned(),
            at: reader.path(),
        }
    }

    pub fn missing_field(reader: &Reader, field: &str) -> Self {
        Error::MissingField {
            field: field.to_owned(),
            at: reader.path(),
        }
    }

    pub fn unexpected_operator(reader: &Reader, op: jomini::text::Operator) -> Self {
        Error::UnexpectedOperator {
            op: op.to_string(),
            at: reader.path(),
        }
    }

    pub fn unexpected_remainder(reader: &Reader, remainder: &str) -> Self {
        Error::UnexpectedRemainder {
            remainder: remainder.to_owned(),
            at: reader.path(),
        }
    }

    pub fn deserialize_error(reader: &Reader, error: jomini::DeserializeError) -> Self {
        Error::DeserializeError {
            error,
            at: reader.path(),
        }
    }

    pub fn scalar_error(reader: &Reader, error: jomini::ScalarError) -> Self {
        Error::ScalarError {
            error,
            at: reader.path(),
        }
    }

    pub fn custom<T: std::fmt::Display>(reader: &Reader, msg: T) -> Self {
        Error::Custom {
            message: msg.to_string(),
            at: reader.path(),
        }
    }
}
