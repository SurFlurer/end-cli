"""Lua file loader using lupa."""

from pathlib import Path
from typing import List, Dict, Any
import lupa.lua54 as lupa

from .models import LuaRecipe, Stack


def lua_table_to_dict(lua_table: Any) -> dict[str, Any]:
    """Convert a Lua table to a Python dictionary."""
    if lua_table is None:
        return {}
    
    result: dict[str, Any] = {}
    for key in lua_table.keys():
        value = lua_table[key]
        if lupa.lua_type(value) == 'table':
            keys: list[Any] = list(value.keys())
            if keys and all(isinstance(k, int) for k in keys):
                result[key] = lua_table_to_list(value)
            else:
                result[key] = lua_table_to_dict(value)
        else:
            result[key] = value
    
    return result


def lua_table_to_list(lua_table: Any) -> list[Any]:
    """Convert a Lua table (list-like) to a Python list."""
    if lua_table is None:
        return []
    
    result: list[Any] = []
    try:
        keys = [k for k in lua_table.keys() if isinstance(k, int)]
        keys.sort()
        
        for key in keys:
            value = lua_table[key]
            if lupa.lua_type(value) == 'table':
                inner_keys: list[Any] = list(value.keys())
                if inner_keys and all(isinstance(k, int) for k in inner_keys):
                    result.append(lua_table_to_list(value))
                else:
                    result.append(lua_table_to_dict(value))
            else:
                result.append(value)
    except (AttributeError, TypeError):
        pass
    
    return result


def load_lua_file(lua_path: Path) -> List[LuaRecipe]:
    """Load a Lua file and return list of recipes.
    
    Args:
        lua_path: Path to the Lua file
        
    Returns:
        List of LuaRecipe objects
    """
    lua = lupa.LuaRuntime(unpack_returned_tuples=True)
    
    with open(lua_path, 'r', encoding='utf-8') as f:
        lua_code = f.read()
    
    result: Any = lua.execute(lua_code)
    
    recipes: List[LuaRecipe] = []
    
    if result is None:
        return recipes
    
    recipe_list: list[Any] = lua_table_to_list(result)
    
    for recipe_dict in recipe_list:
        if not isinstance(recipe_dict, dict):
            continue
        
        ingredients: List[Stack] = []
        ing_list: Any = recipe_dict.get('ingredients')
        if isinstance(ing_list, list):
            for ing in ing_list:
                if isinstance(ing, dict):
                    item_val: Any = ing.get('item')
                    count_val: Any = ing.get('count')
                    ingredients.append(Stack(
                        item=str(item_val) if item_val is not None else '',
                        count=int(count_val) if count_val is not None else 0
                    ))
        
        products: List[Stack] = []
        prod_list: Any = recipe_dict.get('products')
        if isinstance(prod_list, list):
            for prod in prod_list:
                if isinstance(prod, dict):
                    item_val = prod.get('item')
                    count_val = prod.get('count')
                    products.append(Stack(
                        item=str(item_val) if item_val is not None else '',
                        count=int(count_val) if count_val is not None else 0
                    ))
        
        facility_val: Any = recipe_dict.get('facility')
        facility: str | None = str(facility_val) if facility_val else None
        
        time_val: Any = recipe_dict.get('time')
        mode_val: Any = recipe_dict.get('mode')
        
        recipe = LuaRecipe(
            time=int(time_val) if time_val is not None else 0,
            mode=str(mode_val) if mode_val is not None else '',
            ingredients=ingredients,
            products=products,
            facility=facility
        )
        recipes.append(recipe)
    
    return recipes


def load_all_lua_files(wiki_data_dir: Path) -> Dict[str, List[LuaRecipe]]:
    """Load all Lua files from wiki_data directory.
    
    Args:
        wiki_data_dir: Path to wiki_data directory
        
    Returns:
        Dictionary mapping facility name to list of recipes
    """
    all_recipes: Dict[str, List[LuaRecipe]] = {}
    
    # Find all .lua files in the directory
    lua_files = sorted(wiki_data_dir.glob('*.lua'))
    
    for lua_path in lua_files:
        recipes = load_lua_file(lua_path)
        if recipes:
            for recipe in recipes:
                if recipe.facility:
                    if recipe.facility not in all_recipes:
                        all_recipes[recipe.facility] = []
                    all_recipes[recipe.facility].append(recipe)
    
    return all_recipes
