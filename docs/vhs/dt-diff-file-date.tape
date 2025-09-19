# VHS script: demonstrate dt diff with visible changes using filesystem and date

Set Shell "bash"
Set FontSize 14
Set Width 100
Set Height 30
Set Theme "Dracula"

Output "docs/gallery/dt-diff-file-date.gif"

# Stable, readable output
Type "export DT_ALT_SCREEN=false"
Enter
Type "export LANG=en_US.UTF-8 LC_ALL=C"
Enter

# Prepare a demo file that will change over time (no env vars to keep VHS parser happy)
Type "rm -rf /tmp/dt_demo_vhs; mkdir -p /tmp/dt_demo_vhs; : > /tmp/dt_demo_vhs/file.txt"
Enter
Type "date '+%F %T' > /tmp/dt_demo_vhs/file.txt; echo alpha >> /tmp/dt_demo_vhs/file.txt; cat /tmp/dt_demo_vhs/file.txt"
Enter
Sleep 600ms

# Record first run
Type "cargo run -- run \"cat /tmp/dt_demo_vhs/file.txt\""
Enter
Sleep 1200ms

# Change the file content after a short delay so the diff is visible
Type "sleep 1; echo --- >> /tmp/dt_demo_vhs/file.txt; date '+%F %T' >> /tmp/dt_demo_vhs/file.txt; echo beta >> /tmp/dt_demo_vhs/file.txt; cat /tmp/dt_demo_vhs/file.txt"
Enter
Sleep 800ms

# Record second run
Type "cargo run -- run \"cat /tmp/dt_demo_vhs/file.txt\""
Enter
Sleep 1200ms

# Show the diff between the two runs of the same command
# Since there are exactly two records, dt diff will render the diff directly
Type "cargo run -- diff \"cat /tmp/dt_demo_vhs/file.txt\""
Enter
Sleep 1800ms
