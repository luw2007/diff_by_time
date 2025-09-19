# VHS script to render dt diff interaction into a GIF

Set Shell "bash"
Set FontSize 14
Set Width 1280
Set Height 800
Set Theme "Dracula"

Output "docs/gallery/dt-diff.gif"

# Data is prepared by scripts/demo_prep.sh. Launch diff directly.
Type "./package/latest/dt diff date"
Enter
Sleep 800ms

# Navigate and select two items
Type "j"
Sleep 200ms
Type "Enter"
Sleep 300ms
Type "j"
Sleep 200ms
Type "Enter"
Sleep 500ms

# Toggle preview target
Type "o"
Sleep 400ms

# Quit the viewer
Type "q"
Sleep 200ms
