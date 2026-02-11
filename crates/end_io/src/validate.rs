use crate::schema::StackToml;
use crate::{Error, Result};
use end_model::{DisplayName, ItemId, Key, Stack};
use std::num::NonZeroU32;
use std::path::Path;

/// Resolve a list of stack entries with shared path/field/index context.
///
/// This keeps diagnostics consistent across all list items and callers.
pub(crate) fn resolve_stack_list(
    path: &Path,
    field: &str,
    index: Option<usize>,
    raw: Vec<StackToml>,
    resolve_item: impl Fn(&str) -> Option<ItemId>,
) -> Result<Vec<Stack>> {
    let mut resolved = Vec::with_capacity(raw.len());

    for stack in raw {
        resolved.push(resolve_stack(path, field, index, stack, &resolve_item)?);
    }

    Ok(resolved)
}

/// Parse one stack entry and resolve its `item` key into an internal item id.
pub(crate) fn resolve_stack(
    path: &Path,
    field: &str,
    index: Option<usize>,
    raw: StackToml,
    resolve_item: impl Fn(&str) -> Option<ItemId>,
) -> Result<Stack> {
    let item_key = parse_key(path, &format!("{field}.item"), index, raw.item)?;
    let count = parse_positive_u32(path, &format!("{field}.count"), index, raw.count)?;
    let item = resolve_item(item_key.as_str()).ok_or_else(|| Error::UnknownItem {
        path: path.to_path_buf(),
        key: item_key.to_string(),
    })?;
    Ok(Stack {
        item,
        count: count.get(),
    })
}

/// Validate and normalize a localized display name.
pub(crate) fn parse_display_name(
    path: &Path,
    field: &str,
    index: Option<usize>,
    value: String,
) -> Result<DisplayName> {
    DisplayName::try_from(value).map_err(|source| Error::Schema {
        path: path.to_path_buf(),
        field: field.to_string(),
        index,
        message: source.to_string(),
    })
}

/// Parse an optional display name only when provided.
pub(crate) fn parse_optional_display_name(
    path: &Path,
    field: &str,
    index: Option<usize>,
    value: Option<String>,
) -> Result<Option<DisplayName>> {
    value
        .map(|text| parse_display_name(path, field, index, text))
        .transpose()
}

/// Validate a user key string and convert it into the strongly typed key wrapper.
pub(crate) fn parse_key(
    path: &Path,
    field: &str,
    index: Option<usize>,
    key: String,
) -> Result<Key> {
    Key::try_from(key).map_err(|source| Error::Schema {
        path: path.to_path_buf(),
        field: field.to_string(),
        index,
        message: source.to_string(),
    })
}

/// Parse a positive integer while preserving detailed schema diagnostics.
pub(crate) fn parse_positive_u32(
    path: &Path,
    field: &str,
    index: Option<usize>,
    value: i64,
) -> Result<NonZeroU32> {
    if value < 1 {
        return Err(Error::Schema {
            path: path.to_path_buf(),
            field: field.to_string(),
            index,
            message: format!("must be >= 1, got {value}"),
        });
    }
    let parsed = u32::try_from(value).map_err(|_| Error::Schema {
        path: path.to_path_buf(),
        field: field.to_string(),
        index,
        message: format!("out of range for u32: {value}"),
    })?;
    NonZeroU32::new(parsed).ok_or_else(|| Error::Schema {
        path: path.to_path_buf(),
        field: field.to_string(),
        index,
        message: format!("must be >= 1, got {value}"),
    })
}

/// Parse a non-negative integer while preserving detailed schema diagnostics.
pub(crate) fn parse_non_negative_u32(
    path: &Path,
    field: &str,
    index: Option<usize>,
    value: i64,
) -> Result<u32> {
    if value < 0 {
        return Err(Error::Schema {
            path: path.to_path_buf(),
            field: field.to_string(),
            index,
            message: format!("must be >= 0, got {value}"),
        });
    }
    u32::try_from(value).map_err(|_| Error::Schema {
        path: path.to_path_buf(),
        field: field.to_string(),
        index,
        message: format!("out of range for u32: {value}"),
    })
}
