#!/bin/bash

# Countdown from 5
for i in {5..1}; do
    echo "$i..."
    sleep 1
done

echo "Taking screenshot!"

# Create timestamp in dd-mm-yyyy-hh_mm format
timestamp=$(date "+%d-%m-%Y-%H_%M")

# Take screenshot and save to scripts/images directory
screencapture "scripts/images/$timestamp.png"

# brew install imagemagick
# Convert PNG to compressed JPG
magick convert "scripts/images/$timestamp.png" -quality 75 "scripts/images/$timestamp.jpg"

# Remove the original PNG file
rm "scripts/images/$timestamp.png"

echo "Screenshot saved to scripts/images/$timestamp.jpg"
