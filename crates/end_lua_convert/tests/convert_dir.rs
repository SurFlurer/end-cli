use end_lua_convert::convert_dir;
use std::fs;
use tempfile::tempdir;

#[test]
fn convert_dir_generates_expected_toml_files() {
    let tmp = tempdir().expect("create temp dir");
    let input_dir = tmp.path().join("input");
    let recipe_dir = input_dir.join("recipe");
    fs::create_dir_all(&recipe_dir).expect("create recipe dir");

    fs::write(
        input_dir.join("facility_power.toml"),
        r#"
[facility_power]
"Refining Unit" = 120
"#,
    )
    .expect("write facility_power.toml");

    fs::write(
        recipe_dir.join("simple.lua"),
        r#"
return {
  {
    facility = "Refining Unit",
    time = 1,
    ingredients = {
      { item = "Originium Ore", count = 1 },
    },
    products = {
      { item = "Originium Powder", count = 1 },
    },
  },
}
"#,
    )
    .expect("write recipe lua");

    let output = convert_dir(&input_dir).expect("convert should succeed");
    assert!(output.items_toml.contains("[[items]]"));
    assert!(output.items_toml.contains("Originium Ore"));
    assert!(output.items_toml.contains("Originium Powder"));

    assert!(output.facilities_toml.contains("[[machines]]"));
    assert!(output.facilities_toml.contains("Refining Unit"));
    assert!(output.facilities_toml.contains("thermal_bank"));

    assert!(output.recipes_toml.contains("[[recipes]]"));
    assert!(output.recipes_toml.contains("Refining Unit"));
}
