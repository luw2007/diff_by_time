#!/bin/bash

# dt 打包脚本
# 将编译的二进制文件和相关文档打包分发

set -e

PROJECT_NAME="dt"
VERSION="0.1.0"
TARGET_DIR="target/release"
BINARY_NAME="dt"
PACKAGE_NAME="${PROJECT_NAME}-${VERSION}"

echo "📦 开始打包 ${PROJECT_NAME} v${VERSION}..."

# 创建打包目录
mkdir -p "package/${PACKAGE_NAME}"

# 复制二进制文件
echo "📋 复制二进制文件..."
cp "${TARGET_DIR}/${BINARY_NAME}" "package/${PACKAGE_NAME}/"

# 复制文档
echo "📚 复制文档..."
cp "README.md" "package/${PACKAGE_NAME}/" 2>/dev/null || echo "⚠️  README.md 不存在"
cp "LICENSE" "package/${PACKAGE_NAME}/" 2>/dev/null || echo "⚠️  LICENSE 不存在"
cp "THIRD_PARTY_NOTICES.md" "package/${PACKAGE_NAME}/" 2>/dev/null || echo "⚠️  THIRD_PARTY_NOTICES.md 不存在"

# 创建使用说明（根据当前配置的语言）
detect_language() {
    if [[ -f "$HOME/.dt/config.toml" ]]; then
        local lang=$(grep 'language' "$HOME/.dt/config.toml" | cut -d'"' -f2)
        case $lang in
            "zh"|"cn"|"chinese") echo "zh" ;;
            "en"|"english") echo "en" ;;
            *)
                # 检测系统语言
                case ${LANG%_*} in
                    "zh") echo "zh" ;;
                    *) echo "en" ;;
                esac
                ;;
        esac
    else
        # 检测系统语言
        case ${LANG%_*} in
            "zh") echo "zh" ;;
            *) echo "en" ;;
        esac
    fi
}

LANGUAGE=$(detect_language)

echo "🌍 检测到语言: $LANGUAGE"

if [[ "$LANGUAGE" == "zh" ]]; then
    cat > "package/${PACKAGE_NAME}/USAGE.md" << 'EOF'
# dt - 命令执行时间差比较工具

## 安装

1. 将 `dt` 二进制文件复制到你的 PATH 中的某个目录，例如：
   ```bash
   sudo cp dt /usr/local/bin/
   ```

2. 或者直接在当前目录使用：
   ```bash
   ./dt --help
   ```

## 基本用法

### 执行命令并记录
```bash
# 简单命令
dt run "ls -la"

# 带管道的命令（需要用引号包裹）
dt run "ls | head -5"
dt run "ps aux | grep dt"
dt run "find . -name '*.rs' | wc -l"

# 执行后立即与短码对比（-d 等同于 --diff-code）
dt run -d ab "ls | head -5"
```

### 比较命令输出差异
```bash
# 比较同一命令的不同执行结果
dt diff "ls | head -5"
```

### 查看历史记录
```bash
dt list
```

### 清理历史记录
```bash
# 按命令搜索清理
dt clean search "ls"

# 按文件清理
dt clean file /path/to/file

# 清理所有记录
dt clean all
```

## 配置

配置文件位于 `~/.dt/config.toml`：

```toml
[storage]
max_retention_days = 365  # 最大保留天数
auto_archive = true        # 自动归档

[display]
max_history_shown = 10     # 最多显示历史记录数
language = "auto"          # 语言设置 (auto/en/zh)
```

## 许可证

本软件包含第三方组件（例如：fuzzy-matcher，Skim 风格算法），相关许可信息见包内的 `THIRD_PARTY_NOTICES.md`。

## 特性

- ✅ 支持简单命令和管道命令
- ✅ 彩色diff输出
- ✅ 自动归档历史数据
- ✅ 多语言支持（中文/英文）
 - ✅ 日期过滤选择（skim风格）
- ✅ 配置文件管理
- ✅ 按文件和命令搜索清理
EOF
else
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

### View history records
```bash
dt list
```

### Clean history records
```bash
# Clean by command search
dt clean search "ls"

# Clean by file
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
fi

# 创建安装脚本
cat > "package/${PACKAGE_NAME}/install.sh" << 'EOF'
#!/bin/bash

# dt 安装脚本

set -e

BINARY_NAME="dt"
INSTALL_DIR="/usr/local/bin"

# 检查是否有root权限
if [[ $EUID -ne 0 ]]; then
   echo "⚠️  此脚本需要root权限来安装到 /usr/local/bin"
   echo "💡 可以使用以下命令运行："
   echo "   sudo ./install.sh"
   echo "💡 或者手动复制："
   echo "   cp dt ~/.local/bin/  (如果 ~/.local/bin 在 PATH 中)"
   exit 1
fi

echo "🚀 正在安装 ${BINARY_NAME}..."

# 复制二进制文件
cp "${BINARY_NAME}" "${INSTALL_DIR}/"

# 设置可执行权限
chmod +x "${INSTALL_DIR}/${BINARY_NAME}"

echo "✅ 安装完成！"
echo ""
echo "📝 使用方法："
echo "   ${BINARY_NAME} --help"
echo ""
echo "🗂️  配置文件将在首次运行时创建于 ~/.dt/config.toml"
EOF

chmod +x "package/${PACKAGE_NAME}/install.sh"

# 创建压缩包
echo "🗜️  创建压缩包..."
cd package
tar -czf "${PACKAGE_NAME}-$(uname -m)-$(uname -s | tr '[:upper:]' '[:lower:]').tar.gz" "${PACKAGE_NAME}/"

# 创建 zip 包（Windows用户）
zip -r "${PACKAGE_NAME}-$(uname -m)-$(uname -s | tr '[:upper:]' '[:lower:]').zip" "${PACKAGE_NAME}/"

cd ..

echo ""
echo "✅ 打包完成！"
echo ""
echo "📦 生成的文件："
ls -la package/*.tar.gz package/*.zip 2>/dev/null || echo "只有 tar.gz 文件生成"
echo ""
echo "📂 内容包含："
echo "  - ${BINARY_NAME} 二进制文件"
echo "  - USAGE.md 使用说明"
echo "  - THIRD_PARTY_NOTICES.md 第三方许可证说明"
echo "  - install.sh 安装脚本"
echo "  - README.md (如果存在)"
echo "  - LICENSE (如果存在)"
echo ""
echo "🚀 分发说明："
echo "  - Linux/macOS 用户：使用 .tar.gz 文件"
echo "  - Windows 用户：使用 .zip 文件"
echo "  - 运行 install.sh 或手动复制二进制文件到 PATH 中"
