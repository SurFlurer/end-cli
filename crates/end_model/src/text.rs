use std::borrow::Borrow;
use std::fmt;
use std::ops::Deref;

use thiserror::Error;

/// Stable key used by model entities.
/// Keys must be non-blank and must not have leading/trailing spaces.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Key(Box<str>);

impl Key {
    pub fn as_str(&self) -> &str {
        self.0.as_ref()
    }

    fn validate(value: &str) -> Result<(), KeyValidationError> {
        if value.trim().is_empty() {
            return Err(KeyValidationError::Blank);
        }
        if value != value.trim() {
            return Err(KeyValidationError::LeadingOrTrailingSpaces);
        }
        Ok(())
    }
}

impl From<Key> for Box<str> {
    fn from(key: Key) -> Self {
        key.0
    }
}

impl TryFrom<String> for Key {
    type Error = KeyValidationError;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        Self::validate(value.as_str())?;
        Ok(Self(value.into_boxed_str()))
    }
}

impl TryFrom<&str> for Key {
    type Error = KeyValidationError;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        Self::validate(value)?;
        Ok(Self(value.into()))
    }
}

impl Borrow<str> for Key {
    fn borrow(&self) -> &str {
        self.as_str()
    }
}

impl AsRef<str> for Key {
    fn as_ref(&self) -> &str {
        self.as_str()
    }
}

impl Deref for Key {
    type Target = str;

    fn deref(&self) -> &Self::Target {
        self.as_str()
    }
}

impl fmt::Display for Key {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

#[derive(Debug, Clone, Error, PartialEq, Eq)]
pub enum KeyValidationError {
    #[error("Key must not be blank")]
    Blank,
    #[error("Key must not have leading/trailing spaces")]
    LeadingOrTrailingSpaces,
}

/// Non-empty localized display name used by model entities.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct DisplayName(Box<str>);

impl DisplayName {
    pub fn as_str(&self) -> &str {
        self.0.as_ref()
    }
}

impl TryFrom<String> for DisplayName {
    type Error = DisplayNameValidationError;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        if value.trim().is_empty() {
            return Err(DisplayNameValidationError::Blank);
        }
        Ok(Self(value.into_boxed_str()))
    }
}

impl TryFrom<&str> for DisplayName {
    type Error = DisplayNameValidationError;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        if value.trim().is_empty() {
            return Err(DisplayNameValidationError::Blank);
        }
        Ok(Self(value.into()))
    }
}

impl AsRef<str> for DisplayName {
    fn as_ref(&self) -> &str {
        self.as_str()
    }
}

impl Deref for DisplayName {
    type Target = str;

    fn deref(&self) -> &Self::Target {
        self.as_str()
    }
}

impl fmt::Display for DisplayName {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

#[derive(Debug, Clone, Error, PartialEq, Eq)]
pub enum DisplayNameValidationError {
    #[error("Display name must not be blank")]
    Blank,
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used)]
    use super::{DisplayName, DisplayNameValidationError, Key, KeyValidationError};

    #[test]
    fn key_rejects_blank_text() {
        let err = Key::try_from("   ").expect_err("blank key should fail");
        assert_eq!(err, KeyValidationError::Blank);
    }

    #[test]
    fn key_rejects_leading_or_trailing_spaces() {
        let err = Key::try_from(" A ").expect_err("spaced key should fail");
        assert_eq!(err, KeyValidationError::LeadingOrTrailingSpaces);
    }

    #[test]
    fn key_accepts_non_blank_trimmed_text() {
        let key = Key::try_from("A").expect("valid key");
        assert_eq!(key.as_str(), "A");
    }

    #[test]
    fn display_name_rejects_blank_text() {
        let err = DisplayName::try_from("   ").expect_err("blank display name should fail");
        assert_eq!(err, DisplayNameValidationError::Blank);
    }

    #[test]
    fn display_name_accepts_non_blank_text() {
        let text = DisplayName::try_from(" Name ").expect("valid display name");
        assert_eq!(text.as_str(), " Name ");
    }
}
