# VHS script to render dt diff interaction into a GIF

Set Shell "bash"
Set FontSize 14
Set Width 1280
Set Height 800
Set Theme "Dracula"

Output "docs/gallery/dt-diff.gif"

# Use packaged release binary
Type "cd package"
Enter
Type "cd $(ls -td dt-* | head -1)"
Enter

# Seed demo data if necessary (optional, user may provide their own)
# Type "./package/demo_seed.sh"; Enter

# Launch interactive diff for a known simple command
Type "./dt diff date"
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
