#!/bin/bash

# dt packaging script
# Build a distributable bundle with binary and docs

set -e

PROJECT_NAME="dt"
# Read version from Cargo.toml
VERSION=$(sed -n 's/^version = "\(.*\)"/\1/p' Cargo.toml | head -n1)
TARGET_DIR="target/release"
BINARY_NAME="dt"
PACKAGE_NAME="${PROJECT_NAME}-${VERSION}"

echo "📦 Start packaging ${PROJECT_NAME} v${VERSION}..."

# Create package directory
mkdir -p "package/${PACKAGE_NAME}"

# Build release binary to ensure latest changes are packaged
echo "🔨 Building release binary..."
cargo build --release >/dev/null

# Copy binary
echo "📋 Copying binary..."
cp "${TARGET_DIR}/${BINARY_NAME}" "package/${PACKAGE_NAME}/"

# Copy docs
echo "📚 Copying docs..."
cp "README.md" "package/${PACKAGE_NAME}/" 2>/dev/null || echo "⚠️  README.md missing"
cp "LICENSE" "package/${PACKAGE_NAME}/" 2>/dev/null || echo "⚠️  LICENSE missing"
cp "THIRD_PARTY_NOTICES.md" "package/${PACKAGE_NAME}/" 2>/dev/null || echo "⚠️  THIRD_PARTY_NOTICES.md missing"

# Maintain a stable symlink to the latest package for demos/recordings
ln -sfn "${PACKAGE_NAME}" "package/latest"

# Create English usage guide
cat > "package/${PACKAGE_NAME}/USAGE.md" << 'EOF'
# dt - Command Execution Time Diff Tool

## Installation

1. Copy the `dt` binary to a directory in your PATH, for example:
   ```bash
   sudo cp dt /usr/local/bin/
   ```

2. Or use it directly in the current directory:
   ```bash
   ./dt --help
   ```

## Basic Usage

### Execute commands and record
```bash
# Simple commands
dt run "ls -la"

# Commands with pipes (need to be quoted)
dt run "ls | head -5"
dt run "ps aux | grep dt"
dt run "find . -name '*.rs' | wc -l"

# Run and immediately diff with a short code (-d is alias for --diff-code)
dt run -d ab "ls | head -5"
```

### Compare command output differences
```bash
# Compare different executions of the same command
dt diff "ls | head -5"
```

#### Interactive Diff UI (keys)
- j/k or ↑/↓: move selection
- Enter: pick first and second records to compare
- Tab/Space: toggle select current record
- o or ←/→: toggle preview between stdout/stderr
- Backspace: delete last filter char
- Delete: clear filter input
- Esc: quit
- Shift+Backspace or Ctrl+X: delete the highlighted record (two-press confirm)
  - First press shows a confirmation message in the status bar
  - Press again to permanently delete the record and refresh the list

### View history records
```bash
dt ls
dt ls "ls | head" --json
```

### Clean history records
```bash
# Clean by command search (supports dry-run)
dt clean search "ls" --dry-run
dt clean search "ls"

# Clean by file (supports dry-run)
dt clean file /path/to/file --dry-run
dt clean file /path/to/file

# Clean all records
dt clean all
```

## Configuration

Configuration file is located at `~/.dt/config.toml`:

```toml
[storage]
max_retention_days = 365  # Maximum retention days
auto_archive = true        # Auto archive

[display]
max_history_shown = 10     # Maximum history records to show
language = "auto"          # Language setting (auto/en/zh)
```

## Data Directory

By default dt stores data under `~/.dt/`. To isolate environments (e.g., for demos/tests), override the data directory:

```bash
# Use a custom directory for index and records
dt --data-dir /tmp/dt_demo_data run "date"
dt --data-dir /tmp/dt_demo_data diff "date"
```

## Licenses

This software includes third-party components (e.g., fuzzy-matcher with Skim-style algorithm). See `THIRD_PARTY_NOTICES.md` in the package for license details.

## Features

- ✅ Support for simple commands and piped commands
- ✅ Colored diff output
- ✅ Auto archive historical data
- ✅ Multi-language support (Chinese/English)
- ✅ Date filtering selection (skim-like)
- ✅ Configuration file management
- ✅ Clean by file and command search
EOF

# Create installer script
cat > "package/${PACKAGE_NAME}/install.sh" << 'EOF'
#!/bin/bash

# dt installer

set -e

BINARY_NAME="dt"
INSTALL_DIR="/usr/local/bin"

# Require root to install into /usr/local/bin
if [[ $EUID -ne 0 ]]; then
   echo "⚠️  This script needs root to install into /usr/local/bin"
   echo "💡 Try: sudo ./install.sh"
   echo "💡 Or copy manually: cp dt ~/.local/bin/  (if ~/.local/bin is in PATH)"
   exit 1
fi

echo "🚀 Installing ${BINARY_NAME}..."

# Copy binary
cp "${BINARY_NAME}" "${INSTALL_DIR}/"

# Make executable
chmod +x "${INSTALL_DIR}/${BINARY_NAME}"

echo "✅ Installed!"
echo ""
echo "📝 Usage:"
echo "   ${BINARY_NAME} --help"
echo ""
echo "🗂️  Config file will be created at ~/.dt/config.toml on first run"
EOF

chmod +x "package/${PACKAGE_NAME}/install.sh"

# Create archives
echo "🗜️  Creating archives..."
cd package
tar -czf "${PACKAGE_NAME}-$(uname -m)-$(uname -s | tr '[:upper:]' '[:lower:]').tar.gz" "${PACKAGE_NAME}/"

# Create zip for Windows users
zip -r "${PACKAGE_NAME}-$(uname -m)-$(uname -s | tr '[:upper:]' '[:lower:]').zip" "${PACKAGE_NAME}/"

cd ..

echo ""
echo "✅ Packaging completed!"
echo ""
echo "📦 Generated files:"
ls -la package/*.tar.gz package/*.zip 2>/dev/null || echo "Only tar.gz created"
echo ""
echo "📂 Package includes:"
echo "  - ${BINARY_NAME} binary"
echo "  - USAGE.md"
echo "  - THIRD_PARTY_NOTICES.md"
echo "  - install.sh"
echo "  - README.md (if present)"
echo "  - LICENSE (if present)"
echo ""
echo "🚀 Distribution notes:"
echo "  - Linux/macOS: use .tar.gz"
echo "  - Windows: use .zip"
echo "  - Run install.sh or copy the binary into PATH manually"
