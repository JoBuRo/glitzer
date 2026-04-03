Output demo.gif

Set Shell "bash"
Set FontSize 18
Set Width 1400
Set Height 900
Set Theme "OneDark"
Set CursorBlink false
Set Padding 24

Require glitzer
Require git

# Optional: start in your glitzer repo root, or change this path.
Type "glitzer --repo ."
Enter

# Give startup a moment.
Sleep 3s

Screenshot demo.png

# Move through a few hotspots.
Type "j"
Sleep 1s
Type "j"
Sleep 1s
Type "k"
Sleep 1s

# Move through evidence tabs.
Type "l"
Sleep 1s
Type "l"
Sleep 1s
Type "l"
Sleep 1s

# Scroll evidence a bit.
Ctrl+d
Sleep 1s
Ctrl+u
Sleep 1s

# Go back to the first tab.
Type "h"
Sleep 1s
Type "h"
Sleep 1s
Type "h"
Sleep 1s

# Show another hotspot.
Type "j"
Sleep 3s

# Quit cleanly.
Type "q"
Sleep 1s
