#!/bin/bash

#
# BEFORE RUNNING THIS SCRIPT!
#
# Install the Arduino CLI, ensure the index is up-to-date and the required board is installed.
# This script was tested on an Arduino Leonardo board, make changes if necessary.
#

# check if the first argument is empty
if [ -z "$1" ]; then
    echo "./build_arduino.sh <port>"
    exit 1
fi

# check if the port is valid
if [ ! -e "$1" ]; then
    echo "The port $1 does not exist."
    exit 1
fi

# build and upload the sketch
arduino-cli lib install FastLED
arduino-cli compile -b arduino:avr:leonardo -p "$1" -u --warnings all arduino*
