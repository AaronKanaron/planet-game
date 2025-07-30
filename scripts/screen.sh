#!/bin/bash

# Countdown from 5
for i in {5..1}; do
    echo "$i..."
    sleep 1
done

echo "Taking screenshot!"

# Create timestamp in dd-mm-yyyy-hh:mm format
timestamp=$(date "+%d-%m-%Y-%H:%M")

# Take screenshot and save to scripts/images directory
screencapture "scripts/images/$timestamp.png"

echo "Screenshot saved to scripts/images/$timestamp.png"