# Repository Cleanup Report — 2026-09-05

## Before

- 跟踪文件：28
- 临时/迁移残留：6（naysay.exe + _v06a/b/c.py + _v07c/d.py，全部 gitignored、零引用）
- legacy 源码候选：0（全部 28 个跟踪文件均为当前版本所需）

## Deleted

| 文件 | 依据 |
|------|------|
| `naysay.exe` (4.1MB) | 图标验证时从 CI zip 解压的测试残留；gitignored；可随时重建 |
| `_v06a.py` / `_v06b.py` / `_v06c.py` | v0.6 行编辑/原生渲染迁移脚本，改动已全部提交 |
| `_v07c.py` / `_v07d.py` | v0.7 store.rs 注入与 clippy 修复脚本，同上 |

## Archived

无 —— active tree 中不存在"有历史价值但不再被引用"的文件。
pair 时代旧实现由 git history + CHANGELOG + examples/ 完整保存（D-019/D-023 结论）。

## Kept（易误判但当前需要）

- 根 `naysay.ico`：desktop.ini 相对引用（删除会破坏文件夹图标）；与 assets/ 副本内容相同、用途不同
- `naysay run.cmd`：pair 时代命名但功能当前（双击启动器），零仓库引用
- `RELEASE_NOTES.md`（gitignored）：release.yml `body_path` 引用；内容按版本发布时更新

## Verification

- cargo test：82/82 ✅
- cargo clippy -D warnings：✅ 零警告
- cargo fmt --check：✅

## 数据完整性

- `.naysay/`：空（无用户数据需要迁移），未触碰
- `.git/`：未触碰
- 决策存储 `decisions/`：格式未变，v0.6 数据原样兼容
