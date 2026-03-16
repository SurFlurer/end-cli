"""Main generator logic."""

from pathlib import Path
from typing import Dict, List, Set, Tuple

from .lua_loader import load_all_lua_files
from .manual_data import (
    load_machines,
    load_manual_recipes,
    get_item_display_names_from_recipes,
)
from .models import (
    LuaRecipe,
    RecipeToml,
    PowerRecipeToml,
    ItemDisplay,
    FacilitiesToml,
    ThermalBankDisplay,
)


# Items to exclude from the catalog (not actual items, just virtual/power)
EXCLUDED_ITEMS: Set[str] = {
    "Power",  # Virtual item for power recipes, not a real item
}


def collect_all_items(recipes: List[LuaRecipe]) -> Set[str]:
    """Collect all unique item names from recipes."""
    items: Set[str] = set()
    for recipe in recipes:
        for ing in recipe.ingredients:
            items.add(ing.item)
        for prod in recipe.products:
            items.add(prod.item)
    return items


def filter_wanted_items(items: Set[str]) -> Set[str]:
    """Filter items to only include wanted ones."""
    return {item for item in items if item not in EXCLUDED_ITEMS}


def should_include_recipe(recipe: LuaRecipe) -> bool:
    """Check if a recipe should be included.
    
    Excludes recipes that:
    - Produce excluded items
    - Use excluded items as ingredients
    """
    for prod in recipe.products:
        if prod.item in EXCLUDED_ITEMS:
            return False
    
    for ing in recipe.ingredients:
        if ing.item in EXCLUDED_ITEMS:
            return False
    
    return True


def convert_to_toml_recipe(lua_recipe: LuaRecipe) -> RecipeToml | None:
    """Convert a LuaRecipe to RecipeToml, filtering if needed."""
    if not should_include_recipe(lua_recipe):
        return None
    
    if not lua_recipe.facility:
        return None
    
    filtered_ingredients = [
        ing for ing in lua_recipe.ingredients
        if ing.item not in EXCLUDED_ITEMS
    ]
    
    filtered_products = [
        prod for prod in lua_recipe.products
        if prod.item not in EXCLUDED_ITEMS
    ]
    
    return RecipeToml(
        facility=lua_recipe.facility,
        time_s=lua_recipe.time,
        ingredients=filtered_ingredients,
        products=filtered_products,
    )


def convert_power_recipe(lua_recipe: LuaRecipe) -> PowerRecipeToml | None:
    """Convert a LuaRecipe for Thermal Bank to PowerRecipeToml."""
    if lua_recipe.facility != "Thermal Bank":
        return None
    
    power_output: int | None = None
    for prod in lua_recipe.products:
        if prod.item == "Power":
            power_output = prod.count
            break
    
    if power_output is None:
        return None
    
    if len(lua_recipe.ingredients) != 1:
        return None
    
    ingredient = lua_recipe.ingredients[0]
    
    if ingredient.item in EXCLUDED_ITEMS:
        return None
    
    return PowerRecipeToml(
        power_w=power_output,
        time_s=lua_recipe.time,
        ingredient=ingredient,
    )


def generate_recipes(
    wiki_data_dir: Path,
    manual_data_path: Path
) -> Tuple[List[RecipeToml], List[PowerRecipeToml]]:
    """Generate recipe TOML data from Lua files."""
    all_lua_recipes = load_all_lua_files(wiki_data_dir)
    
    toml_recipes: List[RecipeToml] = []
    power_recipes: List[PowerRecipeToml] = []
    
    for facility, recipes in all_lua_recipes.items():
        if facility == "Thermal Bank":
            for lua_recipe in recipes:
                power_recipe = convert_power_recipe(lua_recipe)
                if power_recipe:
                    power_recipes.append(power_recipe)
        else:
            for lua_recipe in recipes:
                toml_recipe = convert_to_toml_recipe(lua_recipe)
                if toml_recipe:
                    toml_recipes.append(toml_recipe)
    
    # Add manual recipes
    manual_recipes = load_manual_recipes(manual_data_path)
    for manual in manual_recipes:
        toml_recipes.append(RecipeToml(
            facility=manual.facility,
            time_s=manual.time_s,
            ingredients=manual.ingredients,
            products=manual.products,
        ))
    
    # Sort recipes by facility, then by first product name, then by first ingredient name
    def recipe_sort_key(r: RecipeToml) -> Tuple[str, str, str]:
        first_product = r.products[0].item if r.products else ""
        first_ingredient = r.ingredients[0].item if r.ingredients else ""
        return (r.facility, first_product, first_ingredient)
    
    toml_recipes.sort(key=recipe_sort_key)
    
    # Sort power recipes by power output
    power_recipes.sort(key=lambda r: r.power_w)
    
    return toml_recipes, power_recipes


def generate_items(
    all_lua_recipes: Dict[str, List[LuaRecipe]],
    manual_data_path: Path
) -> List[ItemDisplay]:
    """Generate item TOML data."""
    all_items: Set[str] = set()
    for _, recipes in all_lua_recipes.items():
        for recipe in recipes:
            if should_include_recipe(recipe):
                all_items.update(collect_all_items([recipe]))
    
    wanted_items = filter_wanted_items(all_items)
    items = get_item_display_names_from_recipes(wanted_items, manual_data_path)
    
    # Sort by key
    items.sort(key=lambda i: i.key)
    
    return items


def generate_facilities(manual_data_path: Path) -> FacilitiesToml:
    """Generate facilities TOML data."""
    machines = load_machines(manual_data_path)
    machines.sort(key=lambda m: m.key)
    
    return FacilitiesToml(
        machines=machines,
        thermal_bank=ThermalBankDisplay()
    )


def format_inline_table(item: str, count: int) -> str:
    """Format a stack as an inline table."""
    return f'{{ item = "{item}", count = {count} }}'


def format_recipes_toml(recipes: List[RecipeToml], power_recipes: List[PowerRecipeToml]) -> str:
    """Format recipes.toml with exact matching format to original."""
    lines: List[str] = [
        "# WARNING: This file is auto-generated by the data generator.",
        "# Do not edit manually. Edit scripts/data_generator/manual_data.toml instead.",
        "",
    ]

    for recipe in recipes:
        lines.append("[[recipes]]")
        lines.append(f'facility = "{recipe.facility}"')
        lines.append(f"time_s = {recipe.time_s}")

        # Format ingredients as inline array of tables
        if recipe.ingredients:
            lines.append(f"ingredients = [{', '.join(format_inline_table(ing.item, ing.count) for ing in recipe.ingredients)}]")
        else:
            lines.append("ingredients = []")

        # Format products as inline array of tables
        if recipe.products:
            lines.append(f"products = [{', '.join(format_inline_table(prod.item, prod.count) for prod in recipe.products)}]")
        else:
            lines.append("products = []")

        lines.append("")

    for power_recipe in power_recipes:
        lines.append("[[power_recipes]]")
        lines.append(f"power_w = {power_recipe.power_w}")
        lines.append(f"time_s = {power_recipe.time_s}")
        lines.append(f"ingredient = {format_inline_table(power_recipe.ingredient.item, power_recipe.ingredient.count)}")
        lines.append("")

    return '\n'.join(lines)


def format_items_toml(items: List[ItemDisplay]) -> str:
    """Format items.toml with exact matching format to original."""
    lines: List[str] = [
        "# WARNING: This file is auto-generated by the data generator.",
        "# Do not edit manually. Edit scripts/data_generator/manual_data.toml instead.",
        "",
    ]
    
    for item in items:
        lines.append("[[items]]")
        lines.append(f'key = "{item.key}"')
        lines.append(f'en = "{item.en}"')
        if item.zh:
            lines.append(f'zh = "{item.zh}"')
        if item.fluid:
            lines.append('fluid = true')
        lines.append("")
    
    return '\n'.join(lines)


def format_facilities_toml(facilities: FacilitiesToml) -> str:
    """Format facilities.toml with exact matching format to original."""
    lines: List[str] = [
        "# WARNING: This file is auto-generated by the data generator.",
        "# Do not edit manually. Edit scripts/data_generator/manual_data.toml instead.",
        "",
    ]
    
    for machine in facilities.machines:
        lines.append("[[machines]]")
        lines.append(f'key = "{machine.key}"')
        lines.append(f"power_w = {machine.power_w}")
        lines.append(f'en = "{machine.en}"')
        lines.append(f'zh = "{machine.zh}"')
        if machine.regions:
            regions_str = ', '.join(f'"{r}"' for r in machine.regions)
            lines.append(f"regions = [{regions_str}]")
        lines.append("")
    
    lines.append("[thermal_bank]")
    lines.append(f'key = "{facilities.thermal_bank.key}"')
    lines.append(f'en = "{facilities.thermal_bank.en}"')
    lines.append(f'zh = "{facilities.thermal_bank.zh}"')
    lines.append("")
    
    return '\n'.join(lines)


def generate_all(
    wiki_data_dir: Path,
    manual_data_path: Path
) -> Tuple[str, str, str]:
    """Generate all TOML content from wiki data.
    
    Returns:
        Tuple of (recipes_toml, items_toml, facilities_toml)
    """
    # Generate recipes
    recipes, power_recipes = generate_recipes(wiki_data_dir, manual_data_path)
    
    # Load Lua recipes to get all items
    all_lua_recipes = load_all_lua_files(wiki_data_dir)
    
    # Generate items
    items = generate_items(all_lua_recipes, manual_data_path)
    
    # Generate facilities
    facilities = generate_facilities(manual_data_path)
    
    # Format output
    recipes_toml = format_recipes_toml(recipes, power_recipes)
    items_toml = format_items_toml(items)
    facilities_toml = format_facilities_toml(facilities)
    
    return recipes_toml, items_toml, facilities_toml
