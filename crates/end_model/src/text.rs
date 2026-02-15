use std::borrow::Borrow;
use std::fmt;
use std::ops::Deref;

use thiserror::Error;

/// Stable key used by model entities.
/// Keys must be non-blank and must not have leading/trailing spaces.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Key(String);

impl Key {
    pub fn as_str(&self) -> &str {
        self.0.as_str()
    }

    pub fn into_string(self) -> String {
        self.0
    }
}

impl TryFrom<String> for Key {
    type Error = KeyValidationError;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        if value.trim().is_empty() {
            return Err(KeyValidationError::Blank);
        }
        if value != value.trim() {
            return Err(KeyValidationError::LeadingOrTrailingSpaces);
        }
        Ok(Self(value))
    }
}

impl TryFrom<&str> for Key {
    type Error = KeyValidationError;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        Self::try_from(value.to_string())
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

impl From<Key> for String {
    fn from(value: Key) -> Self {
        value.into_string()
    }
}

#[derive(Debug, Clone, Error, PartialEq, Eq)]
pub enum KeyValidationError {
    #[error("key must not be blank")]
    Blank,
    #[error("key must not have leading/trailing spaces")]
    LeadingOrTrailingSpaces,
}

/// Non-empty localized display name used by model entities.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct DisplayName(String);

impl DisplayName {
    pub fn as_str(&self) -> &str {
        self.0.as_str()
    }

    pub fn into_string(self) -> String {
        self.0
    }
}

impl TryFrom<String> for DisplayName {
    type Error = DisplayNameValidationError;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        if value.trim().is_empty() {
            return Err(DisplayNameValidationError::Blank);
        }
        Ok(Self(value))
    }
}

impl TryFrom<&str> for DisplayName {
    type Error = DisplayNameValidationError;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        Self::try_from(value.to_string())
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

impl From<DisplayName> for String {
    fn from(value: DisplayName) -> Self {
        value.into_string()
    }
}

#[derive(Debug, Clone, Error, PartialEq, Eq)]
pub enum DisplayNameValidationError {
    #[error("must not be blank")]
    Blank,
}

#[cfg(test)]
mod tests {
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
