# VHS script to render dt run + diff-code flow into a GIF

Set Shell "bash"
Set FontSize 14
Set Width 1280
Set Height 800
Set Theme "Dracula"

Output "docs/gallery/dt-run.gif"

# Use packaged release binary
Type "cd package"
Enter
Type "cd $(ls -td dt-* | head -1)"
Enter

# Run and record a simple command
Type "./dt run echo hello"
Enter
Sleep 1200ms

# Exit after message
Sleep 400ms
