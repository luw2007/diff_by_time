# VHS script to render dt run + diff-code flow into a GIF

Set Shell "bash"
Set FontSize 14
Set Width 1280
Set Height 800
Set Theme "Dracula"

Output "docs/gallery/dt-run.gif"

Type "export DT_ALT_SCREEN=false"
Enter
Type "export LANG=en_US.UTF-8 LC_ALL=C"
Enter

Type "cd $PWD"
Enter

# Run and record a simple command
Type "cargo run -- run echo hello"
Enter
Sleep 1200ms

# Exit after message
Sleep 400ms
