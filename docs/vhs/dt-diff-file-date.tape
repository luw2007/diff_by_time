# VHS script: demonstrate dt diff with visible changes using filesystem and date

Set Shell "bash"
Set FontSize 14
Set Width 1280
Set Height 800
Set Theme "Dracula"

Output "docs/gallery/dt-diff-file-date.gif"

# Use packaged release binary (no env exports)
Type "cd package"
Enter
Type "cd $(ls -td dt-* | head -1)"
Enter

# Prepare a demo file that will change over time (no env vars to keep VHS parser happy)
Type "rm -rf /tmp/dt_demo_vhs; mkdir -p /tmp/dt_demo_vhs; : > /tmp/dt_demo_vhs/file.txt"
Enter
Type "date '+%F %T' > /tmp/dt_demo_vhs/file.txt; echo alpha >> /tmp/dt_demo_vhs/file.txt; cat /tmp/dt_demo_vhs/file.txt"
Enter
Sleep 600ms

# Record first run using packaged dt
Type "./dt run cat /tmp/dt_demo_vhs/file.txt"
Enter
Sleep 1200ms

# Change the file content after a short delay so the diff is visible
Type "sleep 1; echo --- >> /tmp/dt_demo_vhs/file.txt; date '+%F %T' >> /tmp/dt_demo_vhs/file.txt; echo beta >> /tmp/dt_demo_vhs/file.txt; cat /tmp/dt_demo_vhs/file.txt"
Enter
Sleep 800ms

# Record second run
Type "./dt run cat /tmp/dt_demo_vhs/file.txt"
Enter
Sleep 1200ms

# Show the diff between the two runs of the same command
# Since there are exactly two records, dt diff will render the diff directly
Type "./dt diff cat /tmp/dt_demo_vhs/file.txt"
Enter
Sleep 1800ms
