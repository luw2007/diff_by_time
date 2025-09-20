# Homebrew Formula 最简发布流程

以下步骤使用源码构建（无 bottle），需要用户本地具备 Rust toolchain。默认仓库为 `luw2007/diff_by_time`，请按实际信息替换。

## 0. 发布准备
- 确认 `Cargo.toml` 中的版本号（例如 `0.1.6`），并在主仓库打对应 tag：
  ```bash
  git tag -a v0.1.6 -m "Release v0.1.6"
  git push origin v0.1.6
  ```
- 创建 GitHub Release，使用上面的 tag。GitHub 会自动生成 `https://github.com/luw2007/diff_by_time/archive/refs/tags/v0.1.6.tar.gz`。

## 1. 创建 Tap 与初始配方
```bash
brew tap-new luw2007/tap
brew create --set-name dt --tap luw2007/tap \
  https://github.com/luw2007/diff_by_time/archive/refs/tags/v0.1.6.tar.gz
```
- `brew create` 会打开编辑器，生成 `Formula/dt.rb`。

或使用项目内置脚本生成/复制 Formula：

```bash
# 生成到 package/Formula/dt.rb（若 tag 未发布会留下 sha256 占位符）
make tap-formula

# 将 Formula 复制到本地 tap（如已存在该路径）
TAP_DIR=$(brew --repository)/Library/Taps/luw2007/homebrew-tap make tap-formula-to-tap
```

## 2. 编辑 Formula
将文件改成以下模板：
```ruby
class Dt < Formula
  desc "Diff and run commands with time-based history"
  homepage "https://github.com/luw2007/diff_by_time"
  url "https://github.com/luw2007/diff_by_time/archive/refs/tags/v0.1.6.tar.gz"
  sha256 "REPLACE_WITH_TARBALL_SHA256"
  license "MIT"

  depends_on "rust" => :build

  def install
    system "cargo", "install", *std_cargo_args(path: ".")
  end

  test do
    output = shell_output("#{bin}/dt --version")
    assert_match version.to_s, output
  end
end
```
- 如果项目目录结构有变化，把 `path: "."` 改成实际 crate 目录。

## 3. 计算 tarball 的 sha256
确保 tag 已推送并存在于 GitHub：
```bash
brew fetch --build-from-source luw2007/tap/dt
```
- 运行后 Homebrew 会输出下载的路径与对应 sha256，复制并填入 Formula。
- 也可直接使用：
  ```bash
curl -L -o v0.1.6.tar.gz https://github.com/luw2007/diff_by_time/archive/refs/tags/v0.1.6.tar.gz
shasum -a 256 v0.1.6.tar.gz
  ```

## 4. 本地验证
```bash
brew install --build-from-source luw2007/tap/dt
brew test luw2007/tap/dt
brew uninstall dt
```

## 5. 推送 Tap
Tap 仓库目录默认位于 `$(brew --repository)/Library/Taps/luw2007/homebrew-tap`。
```bash
cd $(brew --repository)/Library/Taps/luw2007/homebrew-tap
git status
# 提交 Formula/dt.rb
git add Formula/dt.rb
git commit -m "feat: add dt formula"
git push origin main
```

## 6. 用户安装
```bash
brew tap luw2007/tap
brew install luw2007/tap/dt
```

> 后续发版只需：更新 tag + release → 修改 Formula 中的 `url` 和 `sha256` → 提交到 tap。待自动化或 bottle 需求时，再扩展 CI。
