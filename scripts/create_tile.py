#!/usr/bin/env python3
import os
import sys
import re
from pathlib import Path

def snake_to_pascal(snake_str):
    """Convert snake_case to PascalCase"""
    return ''.join(word.capitalize() for word in snake_str.split('_'))

def get_random_color():
    """Get a random tailwind color for the tile"""
    colors = [
        "RED_500", "BLUE_500", "GREEN_500", "YELLOW_500", "PURPLE_500",
        "PINK_500", "INDIGO_500", "CYAN_500", "ORANGE_500", "LIME_500"
    ]
    import random
    return random.choice(colors)

def update_variants_mod(tile_name, tile_path):
    """Add mod declaration to src/tiles/variants.rs"""
    variants_file = Path("src/tiles/variants.rs")
    
    if not variants_file.exists():
        print(f"Warning: {variants_file} not found, skipping mod update")
        return
    
    with open(variants_file, 'r') as f:
        content = f.read()
    
    # Determine the mod path based on tile_path
    if "/" in tile_path:
        # Handle subdirectories - create nested mod structure
        parts = tile_path.split("/")
        mod_line = f"pub mod {parts[-1]};"  # Just the tile name
        
        # Check if we need to add parent mod declarations
        subdir = "/".join(parts[:-1])
        parent_mod = f"pub mod {parts[0]};"
        
        # Add parent mod if it doesn't exist
        if parent_mod not in content and len(parts) > 1:
            # Add parent mod declaration
            if content.strip():
                content = content.rstrip() + f"\npub mod {parts[0]};\n"
            else:
                content = f"pub mod {parts[0]};\n"
        
        # Create/update parent mod file if needed
        if len(parts) > 1:
            parent_mod_file = Path(f"src/tiles/variants/{parts[0]}.rs")
            if not parent_mod_file.exists():
                parent_mod_file.parent.mkdir(parents=True, exist_ok=True)
                with open(parent_mod_file, 'w') as f:
                    f.write(f"pub mod {parts[-1]};\n")
            else:
                # Add to existing parent mod file
                with open(parent_mod_file, 'r') as f:
                    parent_content = f.read()
                if mod_line not in parent_content:
                    parent_content = parent_content.rstrip() + f"\n{mod_line}\n"
                    with open(parent_mod_file, 'w') as f:
                        f.write(parent_content)
    else:
        # Simple case - just add the mod declaration
        mod_line = f"pub mod {tile_name};"
        
        if mod_line not in content:
            # Insert alphabetically
            lines = content.strip().split('\n')
            mod_lines = [line for line in lines if line.startswith('pub mod ')]
            other_lines = [line for line in lines if not line.startswith('pub mod ')]
            
            mod_lines.append(mod_line)
            mod_lines.sort()
            
            new_content = '\n'.join(mod_lines + other_lines).strip() + '\n'
            
            with open(variants_file, 'w') as f:
                f.write(new_content)

def update_tiles_plugin(struct_name, tile_name, tile_path):
    """Add plugin import and registration to src/tiles.rs"""
    tiles_file = Path("src/tiles.rs")
    
    if not tiles_file.exists():
        print(f"Warning: {tiles_file} not found, skipping plugin update")
        return
    
    with open(tiles_file, 'r') as f:
        content = f.read()
    
    # Determine import path
    if "/" in tile_path:
        import_path = tile_path.replace("/", "::")
    else:
        import_path = tile_name
    
    import_line = f"use crate::tiles::variants::{import_path}::{struct_name}Plugin;"
    plugin_name = f"{struct_name}Plugin"
    
    # Add import if it doesn't exist
    if import_line not in content:
        # Find the imports section and add alphabetically
        import_pattern = r'(use crate::tiles::variants::[^;]+;)'
        imports = re.findall(import_pattern, content)
        
        if imports:
            # Add to existing imports alphabetically
            imports.append(import_line)
            imports.sort()
            
            # Replace all imports with sorted ones
            new_imports = '\n'.join(imports)
            content = re.sub(r'use crate::tiles::variants::[^;]+;(?:\n)?', '', content)
            
            # Insert imports after the bevy import
            bevy_import_match = re.search(r'use bevy::prelude::\*;', content)
            if bevy_import_match:
                insert_pos = bevy_import_match.end()
                content = content[:insert_pos] + '\n' + new_imports + content[insert_pos:]
        else:
            # No existing variant imports, add after bevy import
            bevy_import_match = re.search(r'use bevy::prelude::\*;', content)
            if bevy_import_match:
                insert_pos = bevy_import_match.end()
                content = content[:insert_pos] + '\n' + import_line + content[insert_pos:]
    
    # Add plugin to the plugins tuple if it doesn't exist
    # Check specifically in the plugins section, not the entire content
    plugins_section_pattern = r'\.add_plugins\(\(\s*(.*?)\s*\)\);'
    plugins_match = re.search(plugins_section_pattern, content, re.DOTALL)
    plugin_already_in_tuple = False
    
    if plugins_match:
        current_plugins_text = plugins_match.group(1)
        plugin_already_in_tuple = plugin_name in current_plugins_text
    
    if not plugin_already_in_tuple:
        # Find the plugins tuple and add the new plugin
        # More specific pattern that matches the exact structure
        plugins_pattern = r'\.add_plugins\(\(\s*(.*?)\s*\)\);'
        match = re.search(plugins_pattern, content, re.DOTALL)
        
        if match:
            current_plugins_text = match.group(1).strip()
            if current_plugins_text:
                # Parse existing plugins - split by comma and clean up
                plugins_list = []
                for plugin in current_plugins_text.split(','):
                    plugin = plugin.strip()
                    if plugin:  # Skip empty strings
                        plugins_list.append(plugin)
                
                plugins_list.append(plugin_name)
                plugins_list.sort()
                
                # Format the plugins nicely
                formatted_plugins = ',\n                '.join(plugins_list) + ','
                new_plugins_section = f'.add_plugins((\n                {formatted_plugins}\n            ));'
            else:
                # Empty tuple, just add the plugin
                new_plugins_section = f'.add_plugins((\n                {plugin_name},\n            ));'
            
            content = re.sub(plugins_pattern, new_plugins_section, content, flags=re.DOTALL)
        else:
            print(f"Warning: Could not find .add_plugins pattern in tiles.rs")
    
    with open(tiles_file, 'w') as f:
        f.write(content)

def create_tile(tile_path):
    """Create a new tile file from template"""
    if not tile_path:
        print("Error: No tile name provided")
        sys.exit(1)
    
    # Extract tile name from path
    tile_name = os.path.basename(tile_path)
    
    # Validate tile name (snake_case)
    if not re.match(r'^[a-z][a-z0-9_]*[a-z0-9]$', tile_name) and not re.match(r'^[a-z]$', tile_name):
        print(f"Error: Tile name '{tile_name}' must be in snake_case (e.g., 'my_tile_name')")
        sys.exit(1)
    
    # Convert to PascalCase for struct name
    struct_name = snake_to_pascal(tile_name)
    
    # Read template
    script_dir = Path(__file__).parent
    template_path = script_dir / "templates" / "create-tile.txt"
    
    if not template_path.exists():
        print(f"Error: Template file not found at {template_path}")
        sys.exit(1)
    
    with open(template_path, 'r') as f:
        template_content = f.read()
    
    # Replace template variables
    tile_color = get_random_color()
    content = template_content.replace("{{TILE_NAME_STRUCT}}", struct_name)
    content = content.replace("{{TILE_COLOR}}", tile_color)
    
    # Create output directory if needed
    output_dir = Path("src/tiles/variants")
    if "/" in tile_path:
        # Handle subdirectories
        subdir = os.path.dirname(tile_path)
        output_dir = output_dir / subdir
        output_dir.mkdir(parents=True, exist_ok=True)
    
    # Create output file
    output_file = output_dir / f"{tile_name}.rs"
    
    if output_file.exists():
        print(f"Error: File {output_file} already exists")
        sys.exit(1)
    
    with open(output_file, 'w') as f:
        f.write(content)
    
    # Update mod declarations in variants.rs
    update_variants_mod(tile_name, tile_path)
    
    # Update plugin imports and registrations in tiles.rs
    update_tiles_plugin(struct_name, tile_name, tile_path)
    
    print(f"Created tile: {output_file}")
    print(f"Struct name: {struct_name}")
    print(f"Plugin name: {struct_name}Plugin")
    print(f"Color: {tile_color}")
    print("Updated mod declarations and plugin registrations")

if __name__ == "__main__":
    if len(sys.argv) != 2:
        print("Usage: python3 create_tile.py <tile-path>")
        print("Example: python3 create_tile.py my_tile_name")
        print("Example: python3 create_tile.py optional-subfolder/my_tile_name")
        sys.exit(1)
    
    create_tile(sys.argv[1])