# VHS script: demonstrate dt diff with visible changes using filesystem and date

Set Shell "bash"
Set FontSize 14
Set Width 1280
Set Height 800
Set Theme "Dracula"

Output "docs/gallery/dt-diff-file-date.gif"

# Data is prepared by scripts/demo_prep.sh. Just show the diff immediately.
Type "./package/latest/dt --data-dir /tmp/dt_demo_data diff cat /tmp/dt_demo_vhs/file.txt"
Enter
Sleep 1500ms
