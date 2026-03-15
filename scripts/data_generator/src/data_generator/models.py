"""Data models for the generator."""

from pydantic import BaseModel, Field
from typing import List, Optional


class Stack(BaseModel):
    """An item stack (item + count)."""
    item: str
    count: int


class LuaRecipe(BaseModel):
    """Recipe as loaded from Lua files."""
    time: int
    mode: str  # "Solid", "Fluid", "Power"
    ingredients: List[Stack] = Field(default_factory=lambda: list[Stack]())
    products: List[Stack] = Field(default_factory=lambda: list[Stack]())
    facility: Optional[str] = None


class RecipeToml(BaseModel):
    """Recipe in the TOML output format."""
    facility: str
    time_s: int
    ingredients: List[Stack] = Field(default_factory=lambda: list[Stack]())
    products: List[Stack] = Field(default_factory=lambda: list[Stack]())


class PowerRecipeToml(BaseModel):
    """Power recipe in the TOML output format."""
    power_w: int
    time_s: int
    ingredient: Stack


class ItemDisplay(BaseModel):
    """Item display information."""
    key: str
    en: str
    zh: str


class MachineDisplay(BaseModel):
    """Machine facility display information."""
    key: str
    power_w: int
    en: str
    zh: str
    regions: List[str] = Field(default_factory=lambda: list[str]())


class ThermalBankDisplay(BaseModel):
    """Thermal bank display information."""
    key: str = "Thermal Bank"
    en: str = "Thermal Bank"
    zh: str = "热能池"


class ManualRecipe(BaseModel):
    """Manual recipe override (e.g., for Fluid Pump)."""
    facility: str
    time_s: int
    ingredients: List[Stack] = Field(default_factory=lambda: list[Stack]())
    products: List[Stack] = Field(default_factory=lambda: list[Stack]())


class FacilitiesToml(BaseModel):
    """Facilities.toml output structure."""
    machines: List[MachineDisplay]
    thermal_bank: ThermalBankDisplay


class RecipesToml(BaseModel):
    """Recipes.toml output structure."""
    recipes: List[RecipeToml]
    power_recipes: List[PowerRecipeToml]


class ItemsToml(BaseModel):
    """Items.toml output structure."""
    items: List[ItemDisplay]
