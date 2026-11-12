#!/usr/bin/env bash
# Emulator smoke for scrolling, the island swipe and the settings sheet (android.yml).
set -euo pipefail
SIZE=$(adb shell wm size | sed 's/.*: //' | tr -d '\r')
W=${SIZE%x*}
H=${SIZE#*x}
echo "screen ${W}x${H}"

# The fixture locks scrolling behind a hidden overlay: the scroll guard must unlock it and a finger
# swipe must move the page (the two screenshots must differ).
adb shell am start -n dev.scyde.browser/.MainActivity -a android.intent.action.VIEW -d http://10.0.2.2:8787/scroll/index.html
sleep 15
adb exec-out screencap -p > shots/qa-scroll-before.png
adb shell input swipe $((W / 2)) $((H * 6 / 10)) $((W / 2)) $((H * 3 / 10)) 400
sleep 2
adb exec-out screencap -p > shots/qa-scroll-after.png
if cmp -s shots/qa-scroll-before.png shots/qa-scroll-after.png; then
  echo "::error::the page did not scroll"
  exit 1
fi

# Swipe left on the island: next tab (after the last one, a new tab).
adb shell input swipe $((W * 7 / 10)) $((H * 93 / 100)) $((W * 2 / 10)) $((H * 93 / 100)) 300
sleep 4
adb exec-out screencap -p > shots/island-swipe.png

# Settings opens as a sheet with a close button.
adb shell am start -n dev.scyde.browser/.MainActivity --es page settings
sleep 8
adb exec-out screencap -p > shots/settings-sheet.png
adb shell pidof dev.scyde.browser >/dev/null || {
  adb logcat -d | grep -E "AndroidRuntime|SCYDE|FATAL" | tail -80
  echo "::error::SCYDE crashed"
  exit 1
}
