use mlua::{HookTriggers, Lua, Table, Value, VmState};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{
    Arc,
    atomic::{AtomicUsize, Ordering},
};
use thiserror::Error;

/// Result alias for conversion operations.
pub type Result<T> = std::result::Result<T, Error>;

const LUA_HOOK_STRIDE: u32 = 1_000;
const LUA_MAX_INSTRUCTIONS: usize = 2_000_000;

/// Errors raised while converting v1 Lua recipes into v2 TOML files.
#[derive(Debug, Error)]
pub enum Error {
    #[error("failed to read {path}: {source}")]
    Io {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },

    #[error("failed to parse TOML {path}: {source}")]
    TomlParse {
        path: PathBuf,
        #[source]
        source: toml::de::Error,
    },

    #[error("failed to serialize TOML: {source}")]
    TomlSerialize {
        #[source]
        source: toml::ser::Error,
    },

    #[error("lua error in {path}: {source}")]
    Lua {
        path: PathBuf,
        #[source]
        source: mlua::Error,
    },

    #[error("schema error in {path}: {message}")]
    Schema { path: PathBuf, message: String },

    #[error("missing zh translation for {kind} `{key}`")]
    MissingI18n { kind: &'static str, key: String },

    #[error("missing power_w for facility `{facility}` in facility_power.toml")]
    MissingFacilityPower { facility: String },
}

#[derive(Debug, Clone)]
struct V1Stack {
    item: String,
    count: i64,
}

#[derive(Debug, Clone)]
struct V1Recipe {
    ingredients: Vec<V1Stack>,
    products: Vec<V1Stack>,
    facility: String,
    time_s: u32,
}

#[derive(Debug, Deserialize)]
struct FacilityPowerToml {
    #[serde(default)]
    facility_power: BTreeMap<String, i64>,
}

#[derive(Debug, Serialize)]
struct ItemsToml {
    items: Vec<ItemToml>,
}

#[derive(Debug, Serialize)]
struct ItemToml {
    key: String,
    en: String,
    zh: String,
}

#[derive(Debug, Serialize)]
struct FacilitiesToml {
    machines: Vec<MachineToml>,
    thermal_bank: ThermalBankToml,
}

#[derive(Debug, Serialize)]
struct MachineToml {
    key: String,
    power_w: u32,
    en: String,
    zh: String,
}

#[derive(Debug, Serialize)]
struct ThermalBankToml {
    key: String,
    en: String,
    zh: String,
}

#[derive(Debug, Serialize)]
struct RecipesToml {
    recipes: Vec<RecipeToml>,
    power_recipes: Vec<PowerRecipeToml>,
}

#[derive(Debug, Serialize, Clone)]
struct StackToml {
    item: String,
    count: u32,
}

#[derive(Debug, Serialize)]
struct RecipeToml {
    facility: String,
    time_s: u32,
    ingredients: Vec<StackToml>,
    products: Vec<StackToml>,
}

#[derive(Debug, Serialize)]
struct PowerRecipeToml {
    ingredient: StackToml,
    power_w: u32,
    time_s: u32,
}

/// Serialized output files generated from one conversion run.
#[derive(Debug, Clone)]
pub struct ConvertOutput {
    pub items_toml: String,
    pub facilities_toml: String,
    pub recipes_toml: String,
}

/// Convert one v1 input directory into in-memory TOML outputs for v2.
pub fn convert_dir(input_dir: &Path) -> Result<ConvertOutput> {
    let facility_power_path = input_dir.join("facility_power.toml");
    let recipe_dir = input_dir.join("recipe");

    let facility_power = load_facility_power(&facility_power_path)?;
    let recipes = load_recipes_from_dir(&recipe_dir)?;

    let mut recipe_rows = Vec::new();
    let mut power_recipe_rows = Vec::new();
    let mut item_set = BTreeSet::new();
    let mut machine_set = BTreeSet::new();

    for recipe in recipes {
        if recipe.facility == "Thermal Bank" {
            let (ingredient, power_w) = thermal_to_power(&recipe, &recipe_dir)?;
            item_set.insert(ingredient.item.clone());
            power_recipe_rows.push(PowerRecipeToml {
                ingredient,
                power_w,
                time_s: recipe.time_s,
            });
            continue;
        }

        machine_set.insert(recipe.facility.clone());

        let mut ingredients = Vec::with_capacity(recipe.ingredients.len());
        for stack in recipe.ingredients {
            let count = parse_positive_u32(
                &recipe_dir,
                format!("recipe ingredient count for item `{}`", stack.item),
                stack.count,
            )?;
            item_set.insert(stack.item.clone());
            ingredients.push(StackToml {
                item: stack.item,
                count,
            });
        }

        let mut products = Vec::with_capacity(recipe.products.len());
        for stack in recipe.products {
            let count = parse_positive_u32(
                &recipe_dir,
                format!("recipe product count for item `{}`", stack.item),
                stack.count,
            )?;
            item_set.insert(stack.item.clone());
            products.push(StackToml {
                item: stack.item,
                count,
            });
        }

        recipe_rows.push(RecipeToml {
            facility: recipe.facility,
            time_s: recipe.time_s,
            ingredients,
            products,
        });
    }

    recipe_rows.sort_by(|a, b| {
        a.facility
            .cmp(&b.facility)
            .then(a.time_s.cmp(&b.time_s))
            .then(a.ingredients.len().cmp(&b.ingredients.len()))
            .then(a.products.len().cmp(&b.products.len()))
    });

    power_recipe_rows.sort_by(|a, b| {
        a.ingredient
            .item
            .cmp(&b.ingredient.item)
            .then(a.time_s.cmp(&b.time_s))
            .then(a.power_w.cmp(&b.power_w))
    });

    let mut items = Vec::with_capacity(item_set.len());
    for item_key in item_set {
        let zh = item_zh(&item_key).ok_or_else(|| Error::MissingI18n {
            kind: "item",
            key: item_key.clone(),
        })?;
        items.push(ItemToml {
            key: item_key.clone(),
            en: item_key,
            zh,
        });
    }

    let mut machines = Vec::with_capacity(machine_set.len());
    for facility_key in machine_set {
        let power_w_i64 =
            *facility_power
                .get(&facility_key)
                .ok_or_else(|| Error::MissingFacilityPower {
                    facility: facility_key.clone(),
                })?;
        let power_w = parse_positive_u32(
            &facility_power_path,
            format!("facility_power `{facility_key}`"),
            power_w_i64,
        )?;

        let zh = facility_zh(&facility_key).ok_or_else(|| Error::MissingI18n {
            kind: "facility",
            key: facility_key.clone(),
        })?;

        machines.push(MachineToml {
            key: facility_key.clone(),
            power_w,
            en: facility_key,
            zh: zh.to_string(),
        });
    }

    let thermal_bank = ThermalBankToml {
        key: "Thermal Bank".to_string(),
        en: "Thermal Bank".to_string(),
        zh: facility_zh("Thermal Bank")
            .ok_or_else(|| Error::MissingI18n {
                kind: "facility",
                key: "Thermal Bank".to_string(),
            })?
            .to_string(),
    };

    let items_toml = toml::to_string_pretty(&ItemsToml { items })
        .map_err(|source| Error::TomlSerialize { source })?;
    let facilities_toml = toml::to_string_pretty(&FacilitiesToml {
        machines,
        thermal_bank,
    })
    .map_err(|source| Error::TomlSerialize { source })?;
    let recipes_toml = toml::to_string_pretty(&RecipesToml {
        recipes: recipe_rows,
        power_recipes: power_recipe_rows,
    })
    .map_err(|source| Error::TomlSerialize { source })?;

    Ok(ConvertOutput {
        items_toml,
        facilities_toml,
        recipes_toml,
    })
}

fn load_facility_power(path: &Path) -> Result<BTreeMap<String, i64>> {
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

fn load_recipes_from_dir(dir: &Path) -> Result<Vec<V1Recipe>> {
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

fn thermal_to_power(recipe: &V1Recipe, path: &Path) -> Result<(StackToml, u32)> {
    if recipe.ingredients.len() != 1 {
        return Err(Error::Schema {
            path: path.to_path_buf(),
            message: "thermal bank recipe must have exactly one ingredient".to_string(),
        });
    }

    if recipe.products.len() != 1 {
        return Err(Error::Schema {
            path: path.to_path_buf(),
            message: "thermal bank recipe must have exactly one product".to_string(),
        });
    }

    let power_product = &recipe.products[0];
    if power_product.item != "Power" {
        return Err(Error::Schema {
            path: path.to_path_buf(),
            message: format!(
                "thermal bank recipe product must be `Power`, got `{}`",
                power_product.item
            ),
        });
    }

    let ingredient = &recipe.ingredients[0];
    let ingredient_count = parse_positive_u32(
        path,
        format!("thermal ingredient count for `{}`", ingredient.item),
        ingredient.count,
    )?;
    let power_w = parse_positive_u32(path, "thermal Power.count".to_string(), power_product.count)?;

    Ok((
        StackToml {
            item: ingredient.item.clone(),
            count: ingredient_count,
        },
        power_w,
    ))
}

fn parse_positive_u32(path: &Path, field: String, value: i64) -> Result<u32> {
    if value < 1 {
        return Err(Error::Schema {
            path: path.to_path_buf(),
            message: format!("{field} must be >= 1, got {value}"),
        });
    }
    u32::try_from(value).map_err(|_| Error::Schema {
        path: path.to_path_buf(),
        message: format!("{field} out of range for u32: {value}"),
    })
}

fn parse_positive_u32_from_f64(path: &Path, field: String, value: f64) -> Result<u32> {
    if !value.is_finite() {
        return Err(Error::Schema {
            path: path.to_path_buf(),
            message: format!("{field} must be finite, got {value}"),
        });
    }

    let nearest = value.round();
    let delta = (value - nearest).abs();
    if delta > 1e-9 {
        return Err(Error::Schema {
            path: path.to_path_buf(),
            message: format!(
                "{field} must be integer seconds, got {value} (nearest {nearest}, delta {delta})"
            ),
        });
    }

    if nearest < 1.0 || nearest > u32::MAX as f64 {
        return Err(Error::Schema {
            path: path.to_path_buf(),
            message: format!("{field} out of range for u32: {value}"),
        });
    }

    Ok(nearest as u32)
}

fn facility_zh(name: &str) -> Option<&'static str> {
    match name {
        "Refining Unit" => Some("精炼炉"),
        "Shredding Unit" => Some("粉碎机"),
        "Fitting Unit" => Some("配件机"),
        "Moulding Unit" => Some("塑形机"),
        "Seed-Picking Unit" => Some("采种机"),
        "Planting Unit" => Some("种植机"),
        "Gearing Unit" => Some("装备原件机"),
        "Filling Unit" => Some("灌装机"),
        "Packaging Unit" => Some("封装机"),
        "Grinding Unit" => Some("研磨机"),
        "Reactor Crucible" => Some("反应池"),
        "Forge of the Sky" => Some("天有洪炉"),
        "Separating Unit" => Some("拆解机"),
        "Thermal Bank" => Some("热容池"),
        _ => None,
    }
}

fn item_zh(name: &str) -> Option<String> {
    let direct = match name {
        "Originium Ore" => Some("源石矿"),
        "Originium Powder" => Some("源石粉"),
        "Dense Originium Powder" => Some("浓缩源石粉"),
        "Origocrust" => Some("晶体外壳"),
        "Origocrust Powder" => Some("晶体外壳粉"),
        "Dense Origocrust Powder" => Some("浓缩晶体外壳粉"),
        "Packed Origocrust" => Some("封装晶体外壳"),
        "Ferrium Ore" => Some("铁矿"),
        "Ferrium" => Some("铁"),
        "Ferrium Part" => Some("铁制零件"),
        "Ferrium Powder" => Some("铁粉"),
        "Dense Ferrium Powder" => Some("浓缩铁粉"),
        "Ferrium Component" => Some("铁制部件"),
        "Ferrium Bottle" => Some("铁质瓶"),
        "Amethyst Ore" => Some("紫晶矿"),
        "Amethyst Fiber" => Some("紫晶纤维"),
        "Amethyst Part" => Some("紫晶零件"),
        "Amethyst Powder" => Some("紫晶粉"),
        "Amethyst Component" => Some("紫晶部件"),
        "Amethyst Bottle" => Some("紫晶质瓶"),
        "Cryston Part" => Some("晶通零件"),
        "Cryston Fiber" => Some("晶通纤维"),
        "Cryston Powder" => Some("晶通粉"),
        "Cryston Component" => Some("晶通部件"),
        "Cryston Bottle" => Some("晶通质瓶"),
        "Xiranite" => Some("希然石"),
        "Liquid Xiranite" => Some("液态希然石"),
        "Xiranite Component" => Some("希然石部件"),
        "Clean Water" => Some("清水"),
        "Jincao" => Some("金草"),
        "Jincao Powder" => Some("金草粉"),
        "Jincao Solution" => Some("金草溶液"),
        "Jincao Drink" => Some("金草饮料"),
        "Yazhen" => Some("雅针"),
        "Yazhen Powder" => Some("雅针粉"),
        "Yazhen Solution" => Some("雅针溶液"),
        "Yazhen Syringe (C)" => Some("雅针注射器(C)"),
        "Sandleaf" => Some("沙叶"),
        "Sandleaf Powder" => Some("沙叶粉"),
        "Buckflower" => Some("荞愈花"),
        "Buckflower Powder" => Some("荞愈花粉"),
        "Ground Buckflower Powder" => Some("研磨荞愈花粉"),
        "Citrome" => Some("柑实"),
        "Citrome Powder" => Some("柑实粉"),
        "Ground Citrome Powder" => Some("研磨柑实粉"),
        "Buck Capsule (A)" => Some("精选荞愈胶囊"),
        "Buck Capsule (B)" => Some("优质荞愈胶囊"),
        "Buck Capsule (C)" => Some("荞愈胶囊"),
        "Canned Citrome (A)" => Some("精选柑实罐头"),
        "Canned Citrome (B)" => Some("优质柑实罐头"),
        "Canned Citrome (C)" => Some("柑实罐头"),
        "LC Valley Battery" => Some("低容谷地电池"),
        "SC Valley Battery" => Some("中容谷地电池"),
        "HC Valley Battery" => Some("高容谷地电池"),
        "LC Wuling Battery" => Some("低容五陵电池"),
        "Carbon" => Some("碳"),
        "Carbon Powder" => Some("碳粉"),
        "Dense Carbon Powder" => Some("浓缩碳粉"),
        "Stabilized Carbon" => Some("稳定碳"),
        "Steel" => Some("钢"),
        "Steel Part" => Some("钢制零件"),
        "Steel Bottle" => Some("钢质瓶"),
        "Wood" => Some("木材"),
        "Power" => Some("电力"),
        "Industrial Explosive" => Some("工业炸药"),
        "Aketine" => Some("阿刻汀"),
        "Aketine Powder" => Some("阿刻汀粉"),
        "Aketine Seed" => Some("阿刻汀种子"),
        "Amber Rice" => Some("琥珀米"),
        "Amber Rice Seed" => Some("琥珀米种子"),
        "Redjade Ginseng" => Some("赤玉人参"),
        "Reed Rye" => Some("芦苇黑麦"),
        "Tartpepper" => Some("酸椒"),
        "Bumper-Rich" => Some("富缓冲剂"),
        "Burdo-Muck" => Some("伯尔多泥"),
        _ => None,
    };
    if let Some(v) = direct {
        return Some(v.to_string());
    }

    if let Some(base) = name.strip_suffix(" Seed") {
        return Some(format!("{}种子", item_zh(base)?));
    }

    if let Some((base, modifier)) = split_parenthesized(name) {
        let base_zh = item_zh(base)?;
        let modifier_zh = item_zh(modifier).unwrap_or_else(|| modifier.to_string());
        return Some(format!("{base_zh}（{modifier_zh}）"));
    }

    None
}

fn split_parenthesized(s: &str) -> Option<(&str, &str)> {
    let open = s.rfind(" (")?;
    if !s.ends_with(')') {
        return None;
    }
    let base = &s[..open];
    let inner = &s[(open + 2)..s.len() - 1];
    if base.is_empty() || inner.is_empty() {
        return None;
    }
    Some((base, inner))
}
