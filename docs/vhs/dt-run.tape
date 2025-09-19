# VHS script to render dt run + diff-code flow into a GIF

Set Shell "bash"
Set FontSize 14
Set Width 1280
Set Height 800
Set Theme "Dracula"

Output "docs/gallery/dt-run.gif"

# Use packaged release binary via stable symlink

# Run and record a simple command
Type "./package/latest/dt run echo hello"
Enter
Sleep 1200ms

# Exit after message
Sleep 400ms
