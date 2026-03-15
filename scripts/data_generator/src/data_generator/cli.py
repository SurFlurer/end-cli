"""Command-line interface for the data generator."""

import sys
from pathlib import Path
from typing import Optional

import click

from .generator import generate_all


# Default paths relative to project root
DEFAULT_WIKI_DATA_DIR = Path("wiki_data")
DEFAULT_OUTPUT_DIR = Path("crates/end_io/src")
DEFAULT_MANUAL_DATA = Path("scripts/data_generator/manual_data.toml")


def find_project_root() -> Path:
    """Find the project root by looking for Cargo.toml."""
    current = Path.cwd()
    
    # Check if we're in the scripts/data_generator directory
    if current.name == "data_generator" and current.parent.name == "scripts":
        return current.parent.parent
    
    # Check if we're in the project root
    if (current / "Cargo.toml").exists():
        return current
    
    # Check parent directories
    for parent in current.parents:
        if (parent / "Cargo.toml").exists():
            return parent
    
    # Fall back to current directory
    return current


def resolve_paths(
    wiki_data_dir: Optional[Path],
    output_dir: Optional[Path],
    manual_data_path: Optional[Path],
) -> tuple[Path, Path, Path]:
    """Resolve wiki_data, output, and manual data paths."""
    project_root = find_project_root()
    
    if wiki_data_dir is None:
        wiki_data_dir = project_root / DEFAULT_WIKI_DATA_DIR
    elif not wiki_data_dir.is_absolute():
        wiki_data_dir = project_root / wiki_data_dir
    
    if output_dir is None:
        output_dir = project_root / DEFAULT_OUTPUT_DIR
    elif not output_dir.is_absolute():
        output_dir = project_root / output_dir
    
    if manual_data_path is None:
        manual_data_path = project_root / DEFAULT_MANUAL_DATA
    elif not manual_data_path.is_absolute():
        manual_data_path = project_root / manual_data_path
    
    return wiki_data_dir, output_dir, manual_data_path


@click.group()
@click.version_option(version="0.1.0")
def cli():
    """Generate TOML data files for end-game from wiki Lua files."""
    pass


@cli.command()
@click.option(
    "--wiki-data-dir",
    "-w",
    type=click.Path(path_type=Path, file_okay=False, dir_okay=True),
    help="Directory containing wiki Lua files (default: wiki_data)",
)
@click.option(
    "--output-dir",
    "-o",
    type=click.Path(path_type=Path, file_okay=False, dir_okay=True),
    help="Output directory for TOML files (default: crates/end_io/src)",
)
@click.option(
    "--manual-data",
    "-m",
    type=click.Path(path_type=Path, file_okay=True, dir_okay=False),
    help="Manual data TOML file (default: scripts/data_generator/manual_data.toml)",
)
@click.option(
    "--dry-run",
    "-n",
    is_flag=True,
    help="Print what would be written without writing files",
)
def generate(
    wiki_data_dir: Optional[Path],
    output_dir: Optional[Path],
    manual_data: Optional[Path],
    dry_run: bool,
) -> None:
    """Generate TOML files from wiki Lua data."""
    wiki_data_dir, output_dir, manual_data_path = resolve_paths(
        wiki_data_dir, output_dir, manual_data
    )
    
    # Validate paths exist
    if not wiki_data_dir.exists():
        click.echo(f"Error: Wiki data directory not found: {wiki_data_dir}", err=True)
        sys.exit(1)
    
    if not manual_data_path.exists():
        click.echo(f"Error: Manual data file not found: {manual_data_path}", err=True)
        sys.exit(1)
    
    # Generate TOML content
    try:
        recipes_toml, items_toml, facilities_toml = generate_all(
            wiki_data_dir, manual_data_path
        )
    except Exception as e:
        click.echo(f"Error generating TOML: {e}", err=True)
        sys.exit(1)
    
    output_files = {
        "recipes.toml": recipes_toml,
        "items.toml": items_toml,
        "facilities.toml": facilities_toml,
    }
    
    if dry_run:
        click.echo("Dry run - would write the following files:")
        for filename, content in output_files.items():
            click.echo(f"\n=== {output_dir / filename} ===")
            lines = content.split('\n')[:20]
            for line in lines:
                click.echo(line)
            if len(content.split('\n')) > 20:
                click.echo("...")
    else:
        output_dir.mkdir(parents=True, exist_ok=True)
        
        for filename, content in output_files.items():
            output_path = output_dir / filename
            with open(output_path, 'w', encoding='utf-8') as f:
                f.write(content)
            click.echo(f"Written: {output_path}")
        
        click.echo("\nGeneration complete!")


@cli.command()
@click.option(
    "--wiki-data-dir",
    "-w",
    type=click.Path(path_type=Path, file_okay=False, dir_okay=True),
    help="Directory containing wiki Lua files (default: wiki_data)",
)
@click.option(
    "--output-dir",
    "-o",
    type=click.Path(path_type=Path, file_okay=False, dir_okay=True),
    help="Directory containing existing TOML files (default: crates/end_io/src)",
)
@click.option(
    "--manual-data",
    "-m",
    type=click.Path(path_type=Path, file_okay=True, dir_okay=False),
    help="Manual data TOML file (default: scripts/data_generator/manual_data.toml)",
)
def check(
    wiki_data_dir: Optional[Path],
    output_dir: Optional[Path],
    manual_data: Optional[Path],
) -> None:
    """Check if generated TOML matches existing files (for CI).
    
    Exits with code 0 if files match, 1 otherwise.
    """
    wiki_data_dir, output_dir, manual_data_path = resolve_paths(
        wiki_data_dir, output_dir, manual_data
    )
    
    # Validate paths exist
    if not wiki_data_dir.exists():
        click.echo(f"Error: Wiki data directory not found: {wiki_data_dir}", err=True)
        sys.exit(1)
    
    if not manual_data_path.exists():
        click.echo(f"Error: Manual data file not found: {manual_data_path}", err=True)
        sys.exit(1)
    
    if not output_dir.exists():
        click.echo(f"Error: Output directory not found: {output_dir}", err=True)
        sys.exit(1)
    
    # Generate TOML content
    try:
        recipes_toml, items_toml, facilities_toml = generate_all(
            wiki_data_dir, manual_data_path
        )
    except Exception as e:
        click.echo(f"Error generating TOML: {e}", err=True)
        sys.exit(1)
    
    expected_files = {
        "recipes.toml": recipes_toml,
        "items.toml": items_toml,
        "facilities.toml": facilities_toml,
    }
    
    all_match = True
    
    for filename, expected_content in expected_files.items():
        output_path = output_dir / filename
        
        if not output_path.exists():
            click.echo(f"MISSING: {output_path}", err=True)
            all_match = False
            continue
        
        with open(output_path, 'r', encoding='utf-8') as f:
            actual_content = f.read()
        
        expected_normalized = expected_content.replace('\r\n', '\n').strip()
        actual_normalized = actual_content.replace('\r\n', '\n').strip()
        
        if expected_normalized == actual_normalized:
            click.echo(f"OK: {output_path}")
        else:
            click.echo(f"MISMATCH: {output_path}", err=True)
            all_match = False
    
    if all_match:
        click.echo("\nAll files are up to date!")
        sys.exit(0)
    else:
        click.echo("\nSome files are out of date. Run 'generate' to update them.", err=True)
        sys.exit(1)


if __name__ == "__main__":
    cli()
