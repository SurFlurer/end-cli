# Data Generator

Generates TOML data files for `end-io` crate from wiki Lua files.

## Overview

This tool converts Lua recipe data from `wiki_data/` into TOML format for use by the Rust application.

- **Input**: 
  - Lua files in `../../wiki_data/` (dynamically discovered, no hardcoded list)
  - Manual data in `manual_data.toml` (item names, machine definitions, manual recipes)
- **Output**: TOML files in `../../crates/end_io/src/`
  - `recipes.toml` - Machine recipes and power recipes
  - `items.toml` - Item display names (English and Chinese)
  - `facilities.toml` - Machine and thermal bank definitions

## Usage

### Generate TOML files

```bash
# From project root
cd scripts/data_generator
uv run data-generator generate

# Or with dry-run to preview changes
uv run data-generator generate --dry-run
```

### Check if TOML files are up to date (for CI)

```bash
uv run data-generator check
```

Exits with code 0 if files match, 1 otherwise.

## Architecture

- `lua_loader.py` - Loads Lua files using Lupa (Lua runtime for Python)
- `manual_data.py` - Loads manual data from external TOML file
- `generator.py` - Main generation logic, filtering, and TOML formatting
- `cli.py` - Command-line interface
- `manual_data.toml` - External data file for item names, machines, and manual recipes

## Data Flow

1. Load Lua recipe data from all `*.lua` files in `wiki_data/`
2. Load manual data from `manual_data.toml`
3. Filter out virtual items (e.g., Power)
4. Join with manual display name data
5. Generate TOML with proper formatting
6. Write to `crates/end_io/src/`

## Configuration

### manual_data.toml

This file contains:

- **[[machines]]** - Machine facility definitions with power consumption and display names
- **[thermal_bank]** - Thermal bank facility definition
- **[[items]]** - Item display names (English and Chinese translations)
- **[[manual_recipes]]** - Recipes not present in Lua files (e.g., Fluid Pump)

To add new items or update translations, edit this file.

## Special Handling

### Thermal Bank (Power Recipes)

The `bank.lua` file contains power generation recipes. These are converted to `power_recipes` in TOML, which have a different structure (single ingredient, power output in watts).

### Manual Recipes

Some recipes like Fluid Pump are not in the Lua files and are defined in `manual_data.toml`.

### Dynamic Lua File Discovery

All `*.lua` files in the wiki_data directory are automatically discovered and loaded. The facility name is read from the `facility` field in each recipe (set by the Lua code), not inferred from the filename.
