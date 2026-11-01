on run argv
    set volumePath to item 1 of argv
    tell application "Finder"
        set volumeFolder to (POSIX file volumePath) as alias
        open volumeFolder
        set targetWindow to container window of volumeFolder
        set current view of targetWindow to icon view
        set toolbar visible of targetWindow to false
        set statusbar visible of targetWindow to false
        set bounds of targetWindow to {100, 100, 900, 580}
        set viewOptions to icon view options of targetWindow
        set arrangement of viewOptions to not arranged
        set icon size of viewOptions to 112
        set text size of viewOptions to 14
        set background picture of viewOptions to file ".background:background.png" of volumeFolder
        set position of item "DiagCraft.app" of volumeFolder to {394, 245}
        set position of item "Applications" of volumeFolder to {662, 245}
        update volumeFolder without registering applications
        delay 2
        close targetWindow
    end tell
end run
