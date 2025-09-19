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

# Work in the repo root
Type "cd $PWD"
Enter

# Prepare a demo file that will change over time
Type "DEMO_DIR=/tmp/dt_demo_$$; mkdir -p $DEMO_DIR; DEMO_FILE=$DEMO_DIR/file.txt"
Enter
Type "date '+%F %T' > $DEMO_FILE; echo alpha >> $DEMO_FILE; cat $DEMO_FILE"
Enter
Sleep 600ms

# Record first run
Type "cargo run -- run \"cat $DEMO_FILE\""
Enter
Sleep 1200ms

# Change the file content after a short delay so the diff is visible
Type "sleep 1; echo --- >> $DEMO_FILE; date '+%F %T' >> $DEMO_FILE; echo beta >> $DEMO_FILE; cat $DEMO_FILE"
Enter
Sleep 800ms

# Record second run
Type "cargo run -- run \"cat $DEMO_FILE\""
Enter
Sleep 1200ms

# Show the diff between the two runs of the same command
# Since there are exactly two records, dt diff will render the diff directly
Type "cargo run -- diff \"cat $DEMO_FILE\""
Enter
Sleep 1800ms
