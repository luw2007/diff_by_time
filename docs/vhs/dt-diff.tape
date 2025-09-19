# VHS script to render dt diff interaction into a GIF

Set Shell "bash"
Set FontSize 14
Set Width 1280
Set Height 800
Set Theme "Dracula"

Output "docs/gallery/dt-diff.gif"

# Launch diff directly.
Type "dt diff date"
Enter
Sleep 1000ms

# Navigate and select two items
Type "f"
Sleep 300ms
Enter
Sleep 500ms
Backspace
Sleep 300ms
Type "b"
Sleep 500ms
Enter
Sleep 3000ms

