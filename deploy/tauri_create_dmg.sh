#!/usr/bin/env bash

create-dmg \
    --volname "Telegramail installer" \
    --volicon "./tauri/icons/icon.icns" \
    --background "./tauri/images/background-dmg.tiff" \
    --window-size 540 380 \
    --icon-size 100 \
    --icon "Telegramail.app" 138 225 \
    --hide-extension "Telegramail.app" \
    --app-drop-link 402 225 \
    "$1" \
    "$2"
