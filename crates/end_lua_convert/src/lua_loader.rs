use crate::model::{FacilityPowerToml, V1Recipe, V1Stack};
use crate::validate::parse_positive_u32_from_f64;
use crate::{Error, Result};
use mlua::{HookTriggers, Lua, Table, Value, VmState};
use std::collections::BTreeMap;
use std::fs;
use std::path::Path;
use std::sync::{
    Arc,
    atomic::{AtomicUsize, Ordering},
};

const LUA_HOOK_STRIDE: u32 = 1_000;
const LUA_MAX_INSTRUCTIONS: usize = 2_000_000;

pub(crate) fn load_facility_power(path: &Path) -> Result<BTreeMap<String, i64>> {
    let src = fs::read_to_string(path).map_err(|source| Error::Io {
        path: path.to_path_buf(),
        source,
    })?;
    let parsed: FacilityPowerToml = toml::from_str(&src).map_err(|source| Error::TomlParse {
        path: path.to_path_buf(),
        source,
    })?;
    Ok(parsed.facility_power)
}

pub(crate) fn load_recipes_from_dir(dir: &Path) -> Result<Vec<V1Recipe>> {
    if !dir.is_dir() {
        return Err(Error::Schema {
            path: dir.to_path_buf(),
            message: "recipe directory does not exist".to_string(),
        });
    }

    let mut lua_paths = Vec::new();
    for entry in fs::read_dir(dir).map_err(|source| Error::Io {
        path: dir.to_path_buf(),
        source,
    })? {
        let entry = entry.map_err(|source| Error::Io {
            path: dir.to_path_buf(),
            source,
        })?;
        let path = entry.path();
        if path.extension().and_then(|e| e.to_str()) == Some("lua") {
            lua_paths.push(path);
        }
    }
    lua_paths.sort();

    let mut out = Vec::new();
    for path in lua_paths {
        let mut file_recipes = load_recipes_from_file(&path)?;
        out.append(&mut file_recipes);
    }

    if out.is_empty() {
        return Err(Error::Schema {
            path: dir.to_path_buf(),
            message: "no recipes loaded".to_string(),
        });
    }

    Ok(out)
}

fn load_recipes_from_file(path: &Path) -> Result<Vec<V1Recipe>> {
    let src = fs::read_to_string(path).map_err(|source| Error::Io {
        path: path.to_path_buf(),
        source,
    })?;

    let lua = Lua::new();
    let steps = Arc::new(AtomicUsize::new(0));
    let steps_for_hook = Arc::clone(&steps);
    lua.set_hook(
        HookTriggers::new().every_nth_instruction(LUA_HOOK_STRIDE),
        move |_lua, _debug| {
            let executed = steps_for_hook.fetch_add(LUA_HOOK_STRIDE as usize, Ordering::Relaxed)
                + LUA_HOOK_STRIDE as usize;
            if executed > LUA_MAX_INSTRUCTIONS {
                return Err(mlua::Error::RuntimeError(format!(
                    "instruction limit exceeded ({LUA_MAX_INSTRUCTIONS})"
                )));
            }
            Ok(VmState::Continue)
        },
    );
    let globals = lua.globals();
    for name in [
        "os", "io", "package", "debug", "require", "dofile", "loadfile",
    ] {
        globals.set(name, Value::Nil).map_err(|source| Error::Lua {
            path: path.to_path_buf(),
            source,
        })?;
    }

    let table: Table = lua
        .load(&src)
        .set_name(path.to_string_lossy().as_ref())
        .eval()
        .map_err(|source| Error::Lua {
            path: path.to_path_buf(),
            source,
        })?;

    let len = table.len().map_err(|source| Error::Lua {
        path: path.to_path_buf(),
        source,
    })? as usize;

    let mut out = Vec::with_capacity(len);
    for idx in 1..=len {
        let recipe_tbl: Table = table.get(idx as i64).map_err(|source| Error::Lua {
            path: path.to_path_buf(),
            source,
        })?;

        let facility: String = recipe_tbl.get("facility").map_err(|source| Error::Lua {
            path: path.to_path_buf(),
            source,
        })?;

        let time_f64: f64 = recipe_tbl.get("time").map_err(|source| Error::Lua {
            path: path.to_path_buf(),
            source,
        })?;
        let time_s = parse_positive_u32_from_f64(path, format!("recipe[{idx}] time"), time_f64)?;

        let ingredients_tbl: Table =
            recipe_tbl.get("ingredients").map_err(|source| Error::Lua {
                path: path.to_path_buf(),
                source,
            })?;

        let mut ingredients = Vec::new();
        for value in ingredients_tbl.sequence_values::<Table>() {
            let stack_tbl = value.map_err(|source| Error::Lua {
                path: path.to_path_buf(),
                source,
            })?;
            let item: String = stack_tbl.get("item").map_err(|source| Error::Lua {
                path: path.to_path_buf(),
                source,
            })?;
            let count: i64 = stack_tbl.get("count").map_err(|source| Error::Lua {
                path: path.to_path_buf(),
                source,
            })?;
            ingredients.push(V1Stack { item, count });
        }

        let products_tbl: Table = recipe_tbl.get("products").map_err(|source| Error::Lua {
            path: path.to_path_buf(),
            source,
        })?;

        let mut products = Vec::new();
        for value in products_tbl.sequence_values::<Table>() {
            let stack_tbl = value.map_err(|source| Error::Lua {
                path: path.to_path_buf(),
                source,
            })?;
            let item: String = stack_tbl.get("item").map_err(|source| Error::Lua {
                path: path.to_path_buf(),
                source,
            })?;
            let count: i64 = stack_tbl.get("count").map_err(|source| Error::Lua {
                path: path.to_path_buf(),
                source,
            })?;
            products.push(V1Stack { item, count });
        }

        if ingredients.is_empty() || products.is_empty() {
            return Err(Error::Schema {
                path: path.to_path_buf(),
                message: format!("recipe[{idx}] has empty ingredient/product list"),
            });
        }

        out.push(V1Recipe {
            ingredients,
            products,
            facility,
            time_s,
        });
    }

    Ok(out)
}
