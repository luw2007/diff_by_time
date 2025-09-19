# VHS script to render dt run + diff-code flow into a GIF

Set Shell "bash"
Set FontSize 14
Set Width 1280
Set Height 800
Set Theme "Dracula"

Output "docs/gallery/dt-run.gif"

# Show brief usage first, then run twice with -d a to immediately show diffs.
Type "dt --help"
Enter
Sleep 700ms

Type "dt run date"
Enter
Sleep 800ms

Type "dt run -d a date"
Enter
Sleep 3000ms
