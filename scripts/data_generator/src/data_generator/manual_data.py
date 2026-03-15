"""Manual data loader - loads from external TOML file."""

from pathlib import Path
from typing import List, Dict, Any
import toml

from .models import ItemDisplay, MachineDisplay, ManualRecipe, Stack


def load_manual_data(manual_data_path: Path) -> dict[str, Any]:
    """Load manual data from TOML file."""
    with open(manual_data_path, 'r', encoding='utf-8') as f:
        return toml.load(f)


def load_machines(manual_data_path: Path) -> List[MachineDisplay]:
    """Load machine definitions from manual data TOML."""
    data = load_manual_data(manual_data_path)
    machines: List[MachineDisplay] = []
    
    machines_list: list[Any] = data.get('machines', [])
    for m in machines_list:
        if isinstance(m, dict):
            key_val: Any = m.get('key')
            power_w_val: Any = m.get('power_w')
            en_val: Any = m.get('en')
            zh_val: Any = m.get('zh')
            regions_val: Any = m.get('regions')
            machines.append(MachineDisplay(
                key=str(key_val) if key_val is not None else '',
                power_w=int(power_w_val) if power_w_val is not None else 0,
                en=str(en_val) if en_val is not None else '',
                zh=str(zh_val) if zh_val is not None else '',
                regions=list(regions_val) if isinstance(regions_val, list) else []
            ))
    
    return machines


def load_item_display_names(manual_data_path: Path) -> Dict[str, tuple[str, str]]:
    """Load item display names from manual data TOML."""
    data = load_manual_data(manual_data_path)
    items: Dict[str, tuple[str, str]] = {}
    
    items_list: list[Any] = data.get('items', [])
    for item in items_list:
        if isinstance(item, dict):
            key_val: Any = item.get('key')
            en_val: Any = item.get('en')
            zh_val: Any = item.get('zh')
            if key_val is not None:
                items[str(key_val)] = (
                    str(en_val) if en_val is not None else '',
                    str(zh_val) if zh_val is not None else ''
                )
    
    return items


def load_manual_recipes(manual_data_path: Path) -> List[ManualRecipe]:
    """Load manual recipes from manual data TOML."""
    data = load_manual_data(manual_data_path)
    recipes: List[ManualRecipe] = []
    
    recipes_list: list[Any] = data.get('manual_recipes', [])
    for r in recipes_list:
        if isinstance(r, dict):
            ingredients: List[Stack] = []
            ing_list: list[Any] = r.get('ingredients', [])
            for ing in ing_list:
                if isinstance(ing, dict):
                    item_val: Any = ing.get('item')
                    count_val: Any = ing.get('count')
                    ingredients.append(Stack(
                        item=str(item_val) if item_val is not None else '',
                        count=int(count_val) if count_val is not None else 0
                    ))
            
            products: List[Stack] = []
            prod_list: list[Any] = r.get('products', [])
            for prod in prod_list:
                if isinstance(prod, dict):
                    item_val = prod.get('item')
                    count_val = prod.get('count')
                    products.append(Stack(
                        item=str(item_val) if item_val is not None else '',
                        count=int(count_val) if count_val is not None else 0
                    ))
            
            facility_val: Any = r.get('facility')
            time_s_val: Any = r.get('time_s')
            recipes.append(ManualRecipe(
                facility=str(facility_val) if facility_val is not None else '',
                time_s=int(time_s_val) if time_s_val is not None else 0,
                ingredients=ingredients,
                products=products
            ))
    
    return recipes


def get_item_display_names_from_recipes(
    recipe_items: set[str],
    manual_data_path: Path
) -> List[ItemDisplay]:
    """Generate ItemDisplay list from items found in recipes.
    
    Uses manual data TOML for known items.
    """
    item_names = load_item_display_names(manual_data_path)
    items: List[ItemDisplay] = []
    
    for item_key in sorted(recipe_items):
        if item_key in item_names:
            en, zh = item_names[item_key]
            items.append(ItemDisplay(key=item_key, en=en, zh=zh))
        else:
            # Default: use key as English name, empty Chinese
            items.append(ItemDisplay(key=item_key, en=item_key, zh=""))
    
    return items
