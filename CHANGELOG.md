# Changelog
## 0.1.7 — 2025-09-22
- fix(tui): arrow keys in preview return focus to the list while modifier keys keep fine-grained scrolling
- feat(keys): Enter toggles selection from preview and runs the diff automatically once two items are chosen
- chore(ui): show total record count in the selector header and restore the compact navigation hint footer
- docs: refresh README navigation notes and localized key hints

## 0.1.6 — 2025-09-20
- feat(ui): migrate interactive diff UI to ratatui; add preview scrollbar and toggleable stdout/stderr preview
- feat(keys): Enter-to-diff from any focus; q back, Q quit; h/? help overlay (localized EN/ZH)
- fix(preview): sanitize ANSI/\r to prevent layout corruption and cursor jumps
- fix(scrollbar): dynamic thumb + viewport mapping; clamp offset; 100% thumb when content fits
- chore(clean): remove legacy crossterm rendering code and unused helpers
- docs(i18n): add preview help strings; update navigation hints

## 0.1.5 — 2025-09-19
- feat: add `--version`/`-v` flags to print package version for Homebrew tests and scripts

## 0.1.4 — 2025-09-19
- chore: bump version for follow-up release tag (no code changes)

## 0.1.1 — 2025-09-18
- UI: dynamic left panel width based on visible items
- UI: preview header split into two rows (Path on row 1; Preview on row 2)
- UI: vertical separator between list and preview; compact bottom status bar
- Path line: proper truncation and top placement; wrapped continuation lines removed from header
- List items: compact format `code:<x> time:<YYYY-MM-DD HH:MM:SS>`
- Docs: update README Interactive UX
- Style: enforce English-only code comments (project rule)
- Quality: fix clippy warnings; tests passing (9/9)
## 0.1.2 — 2025-09-18
- Remove experimental scrollbar and drag logic; simplify list UI
- Keep keyboard navigation only; clarify preview toggle hint
- Minor cleanups (comments, layout reserve lines)

## 0.1.3 — 2025-09-19
- fix: dt diff Enter 行为 — 已选两条后，回车直接执行（即使光标移动）
- chore: 清理 clippy 告警（dead_code、unnecessary_cast、too_many_arguments）
- chore: 测试辅助迁移到 #[cfg(test)] 模块并隔离；删除临时测试脚本
- ci: 新增 GitHub Actions 构建与打包流程，生成并上传产物
- build: package.sh 集成 cargo-about/cargo-license 自动生成依赖许可证清单
- docs: README 顶部新增非生产可用状态声明（EN/ZH）
