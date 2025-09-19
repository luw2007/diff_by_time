# Changelog

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
