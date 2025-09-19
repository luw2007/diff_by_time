# VHS script to render dt diff interaction into a GIF

Set Shell "bash"
Set FontSize 14
Set Width 120
Set Height 120
Set Theme "Dracula"

Output "docs/gallery/dt-diff.gif"

# Ensure deterministic UI behavior
Type "export DT_ALT_SCREEN=false"
Enter
Type "export LANG=en_US.UTF-8 LC_ALL=C"
Enter

# Move to project root (adjust if needed)
Type "cd $PWD"
Enter

# Seed demo data if necessary (optional, user may provide their own)
# Type "./package/demo_seed.sh"; Enter

# Launch interactive diff for a known simple command
Type "cargo run -- diff \"date\""
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
