#!/bin/bash

# Script to create a new tile using the Python generator
# Usage: ./scripts/create-tile.sh <tile-path>
# Example: ./scripts/create-tile.sh my_tile_name
# Example: ./scripts/create-tile.sh optional-subfolder/my_tile_name

set -e  # Exit on any error

# Get the directory where this script is located
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

# Check if argument is provided
if [ $# -ne 1 ]; then
    echo "Usage: $0 <tile-path>"
    echo "Example: $0 my_tile_name"
    echo "Example: $0 optional-subfolder/my_tile_name"
    exit 1
fi

# Run the Python script
python3 "$SCRIPT_DIR/create_tile.py" "$1"