# VHS script to render dt run + diff-code flow into a GIF

Set Shell "bash"
Set FontSize 14
Set Width 1280
Set Height 800
Set Theme "Dracula"

Output "docs/gallery/dt-run.gif"

# Data is prepared by scripts/demo_prep.sh (baseline 'a' for date).
# Show brief usage first, then run twice with -d a to immediately show diffs.
Type "./package/latest/dt --help"
Enter
Sleep 700ms
Type "./package/latest/dt --data-dir /tmp/dt_demo_data run -d a date"
Enter
Sleep 800ms
Type "./package/latest/dt --data-dir /tmp/dt_demo_data run -d a date"
Enter
Sleep 800ms
