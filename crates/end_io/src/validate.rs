use crate::schema::StackToml;
use crate::{Error, Result};
use end_model::{ItemId, Stack};
use std::collections::HashSet;
use std::num::NonZeroU32;
use std::path::{Path, PathBuf};

pub(crate) fn resolve_stack_list(
    path: &Path,
    field: &str,
    index: Option<usize>,
    raw: Vec<StackToml>,
    resolve_item: impl Fn(&str) -> Option<ItemId>,
) -> Result<Vec<Stack>> {
    let mut resolved = Vec::with_capacity(raw.len());
    let mut seen = HashSet::new();

    for stack in raw {
        let s = resolve_stack(path, field, index, stack, &resolve_item)?;
        if !seen.insert(s.item) {
            return Err(Error::Schema {
                path: path.to_path_buf(),
                field: field.to_string(),
                index,
                message: "duplicate item in same list".to_string(),
            });
        }
        resolved.push(s);
    }

    Ok(resolved)
}

pub(crate) fn resolve_stack(
    path: &Path,
    field: &str,
    index: Option<usize>,
    raw: StackToml,
    resolve_item: impl Fn(&str) -> Option<ItemId>,
) -> Result<Stack> {
    let item_key = validate_key(path, &format!("{field}.item"), index, raw.item)?;
    let count = parse_positive_u32(path, &format!("{field}.count"), index, raw.count)?;
    let item = resolve_item(item_key.as_str()).ok_or_else(|| Error::UnknownItem {
        path: path.to_path_buf(),
        key: item_key,
    })?;
    Ok(Stack {
        item,
        count: count.get(),
    })
}

pub(crate) fn validate_non_empty(
    len: usize,
    path: &Path,
    field: &str,
    index: Option<usize>,
) -> Result<()> {
    if len == 0 {
        return Err(Error::Schema {
            path: path.to_path_buf(),
            field: field.to_string(),
            index,
            message: "must not be empty".to_string(),
        });
    }
    Ok(())
}

pub(crate) fn validate_text(
    path: &Path,
    field: &str,
    index: Option<usize>,
    value: &str,
) -> Result<()> {
    if value.trim().is_empty() {
        return Err(Error::Schema {
            path: path.to_path_buf(),
            field: field.to_string(),
            index,
            message: "must not be blank".to_string(),
        });
    }
    Ok(())
}

pub(crate) fn validate_key(
    path: &Path,
    field: &str,
    index: Option<usize>,
    key: String,
) -> Result<String> {
    if key.trim().is_empty() {
        return Err(Error::Schema {
            path: path.to_path_buf(),
            field: field.to_string(),
            index,
            message: "key must not be blank".to_string(),
        });
    }
    if key != key.trim() {
        return Err(Error::Schema {
            path: path.to_path_buf(),
            field: field.to_string(),
            index,
            message: "key must not have leading/trailing spaces".to_string(),
        });
    }
    Ok(key)
}

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

pub(crate) fn load_data_file(
    data_dir: Option<&Path>,
    filename: &str,
    builtin: &'static str,
) -> Result<(PathBuf, String)> {
    match data_dir {
        Some(dir) => {
            let path = dir.join(filename);
            let src = std::fs::read_to_string(&path).map_err(|source| Error::Io {
                path: path.clone(),
                source,
            })?;
            Ok((path, src))
        }
        None => Ok((
            PathBuf::from(format!("<builtin>/{filename}")),
            builtin.to_string(),
        )),
    }
}
