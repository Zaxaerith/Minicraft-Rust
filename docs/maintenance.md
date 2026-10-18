# Windows 工作区维护（2026-10-18）

RC `ab54e552e96d00ecf5cb262252e153f44333a223` 从旧 main
`164c1c7aab0b76766fdf85bd5d986951d1fceae4` 快进，保留全部历史，已实际推送。
后续的目录整理和 Windows CI 修正继续在 main 提交；Infdev 实验只在独立功能分支开发。

## 目录与输出约定

```text
assets/                   原始资源、dev2 扩展及来源清单
crates/                   client/common/server/parity 与受控 fixtures
docs/history/             8 份原始阶段审计（内容保留）
scripts/parity224/         固定 2.2.4 Java Oracle 源码与驱动
scripts/parity230/         固定 dev2 Java Oracle 源码与驱动
scripts/validate-windows.ps1   本地完整验证入口
scripts/package-windows-rc.ps1 唯一当前 RC 打包入口
target/validation/current/ 可覆盖的本次命令日志、结果与清理清单
target/validation/history/ 少量历史总结和必要失败证据
target/parity/java224/     可复用参考编译/输出位置
target/parity/java230/     可复用参考编译/output/world36 位置
target/rc/java230-dev2/    最新 RC、清单及其独立运行证据
```

`target` 及私人工作笔记仍被忽略；只放开明确列出的项目文档路径，未将原有私人 docs 纳入 Git。
不以新 Phase 名为每次验证另建完整产物树。现有含存档的历史目录暂时保留，不为整齐而搬动或删除它们。

## 实际清理与保留

- 8 份根目录审计迁入 `docs/history/`，更新索引和实际引用。
- 删除 4 个无调用的一次性源码拆分/资源路径修补脚本：它们引用已不存在的旧 `src/world/tile.rs` / `src/assets.rs`，不适合重复执行。
- 删除旧 Python 打包器：缺少当前运行库与第三方许可收集，已由 Windows 白名单打包器替代。
- 将 RC 验证时使用的真实 Java 输出比较器正式收为 `scripts/compare-java-oracles.py`；不修改任何 golden。
- 移除 170 个已核实的旧日志/完全重复原始地图，7 份小型最终摘要集中保留。
- 仅删除 `target/debug/incremental` 的可再生成 Cargo 缓存（3286 文件、1,887,341,191 逻辑字节）；未清空 target、依赖缓存或 Release。
- 清理前后 412 个存档相关文件逐项 SHA-256 相同。所有 `state.json`、备份、Java Save、进度、RC 独立运行目录、旧 Smoke 世界都保留；不推断它们是否被人试玩过。
- 不确定用途的旧输出脚本/预览、原有忽略的私人 docs、dist 历史 ZIP、test-data、参考源码均保留。记录与清单在 `target/validation/current/cleanup-plan.json`。
- 最新 RC ZIP 大小 2,306,910 字节，SHA-256 仍为 `8c9d81838c79e55527aa6d1665ae2cc0f813ae28261da18fdd229382002067b7`。

清理前 `target`：7755 文件、820 目录、3,679,991,286 逻辑字节、3,691,692,920 实际分配字节。
分配量由 Windows `GetFileInformationByHandleEx(FileStandardInfo)` 读取，非仅以文件长度冒充磁盘占用。
复验后（包含重新构建的必要缓存）`target`：4511 文件、728 目录、1,979,594,447 逻辑字节、1,984,878,952 实际分配字节。
净释放实际分配空间 1,706,813,968 字节；`java230` 输出本身从 1273 降至 1098 文件，所有 151 个目录因含存档等保留。
Git 跟踪文件 876→875，8 份报告均为 100% 内容相同的重命名。
7 个旧本地分支逐一证明 tip 是 main 祖先后使用 `git branch -d` 删除。
最终提交、远端状态保存在 `target/validation/current/`，避免记录自身 SHA 的循环提交。

## 验证与已发现问题

首次整合 main 的本地 Release Workspace：633 PASS、0 失败、0 忽略。
实际远程 Windows CI [39099629287](https://github.com/Zaxaerith/Minicraft-Rust/actions/runs/39099629287)
失败于 Debug 模式的两日模拟 15 秒预算（客户端套件 123.72 秒）；不是未运行的 CI 通过声明。
工作流改为完整 Release 测试，去掉被全量覆盖的重复单项 Release 调用；原预算、模拟长度、结果断言均保留。

本地清理后曾遇到一次 Windows 原子发布“拒绝访问”，原输入/目标保护仍生效，单项复跑通过；原因未确定。
随后一次四线程 Release 和一次独立两日模拟也触发了原性能预算，失败日志保留，未当作通过。
针对进度热路径仅在历史命中集合为空时跳过不可能命中的字符串构造，不更改配方/进度语义或持久字段。原同一两日模拟从独立运行 25.26 秒失败降至 1.19 秒通过；原 15 秒预算和所有结果断言保持。
修复后本地完整 633 项 Release、格式、严格 Clippy、构建、自检和既有 Java 驱动全部通过。6 组 World 的 210 个条目、19 份确定性 JSON、5 组官方 Save 及原 224 样本均匹配；未更新 golden。具体命令及重新生成的 Java Fixture 比较以 `target/validation/current/validation.txt` 与对应日志为准。

本轮仅 Windows x64；不维护或规划其它平台。不触碰个人存档，不执行历史改写、Tag 或 GitHub Release。
