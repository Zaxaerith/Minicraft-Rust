# Java 2.3.0-dev2 Phase 3.7：Windows x64 本地 RC

日期：2026-10-18。仅维护 Windows x64 MSVC；本轮不增加玩法、不更新依赖或游戏版本号，不改玩家真实存档。

## 修改与工程清理

开始工作区干净，实际基线为 `feat/java230-world-assembly` / `4fe8026a037524daebbbad914a82624885cb0461`，从该点创建 `chore/java230-windows-rc`。完整阅读Phase3.6报告，继承其全部修复。

- 检查旧main到基线16个提交、337个变更路径和当前受控测试ZIP。合法Java Oracle/fixture保留；没有把它们当成个人存档删掉。
- 发现早于本轮就跟踪的9张 `crates/minicraft-client/target/parity/render-frame/rust/*.png`（97,326字节）；仅取消跟踪，9份磁盘文件及历史提交均保留。`.gitignore` 改为递归忽略 `target/`，不再漏掉crate内缓存。没有发现其它被跟踪的个人存档、凭据或构建缓存。
- CI收为 `windows-latest`，保留原有格式/编译/测试/Clippy/优化回归/自检及4线程设置。没有通过Push触发远程CI，也没有把未运行的CI记为通过。
- README重写为当前功能、Windows安装构建/运行、224副本兼容加载、dev2事务式导入、限制及署名说明，去掉过时阶段日志。
- 新增 `THIRD_PARTY_NOTICES.md`：保留原作者/上游credits、资源来源、Windows锁定42项依赖（含构建辅助）、实际MIT/Unicode许可。Microsoft运行库官方许可原DOCX及可读文本保存在 `licenses/`；不假称未Push的本地对应源码已经在GitHub。
- 新增 `scripts/package-windows-rc.ps1`（PowerShell7）。读取现有Release，要求干净Git状态；检查x64 PE、真实导入DLL、签名及非debug/preview的VS Redist运行库；明确文件白名单、文件哈希和ZIP清单。不自动编译、发布或升级版本，默认拒绝覆盖已有RC。

本轮没有修改任何Rust游戏源码、Cargo.lock、旧ItemId/Tile ID、生成规则或原始403资源。旧Python打包脚本保留为历史入口，README明确当前入口为Windows脚本。

## RC产物与许可证

- ZIP：`target/rc/java230-dev2/Minicraft-Rust-java230-dev2-windows-x64-rc.zip`
- 大小：2306910 字节（约2.20MiB）。
- SHA-256：`8c9d81838c79e55527aa6d1665ae2cc0f813ae28261da18fdd229382002067b7`。
- 完整逐文件大小/SHA-256：同目录 `Minicraft-Rust-java230-dev2-windows-x64-rc.files.json`；ZIP校验文件为 `.zip.sha256`；包内另有 `MANIFEST.sha256`。
- 包内 `BUILD-INFO.txt` 的源码修订为 **`5bc92a90f6383cc71ad5123c57093cba454ed879`**，即本轮工程与打包提交。后续本审计提交不改变EXE、包内容或对应游戏代码；不为记录自身SHA循环重打包。
- 根EXE SHA-256：`a3299efd0cffd3365bcf1e4a3f013dd897067656d4bbf545a1e9491137cc60de`，使用现有Release编译产物。
- 实际非系统运行依赖仅 `vcruntime140.dll`：VS release x64 Redist中的14.51.36247.0，微软签名有效，非debug/preview；未从System32复制。Windows自带UCRT/OpenGL/其它系统DLL不放入ZIP。运行不需要Java、Rust、SDL2、仓库或外置资源。
- 这是私人本地RC，不是公开发行。今后对外提供二进制须提供精确对应源码和许可证；包内SOURCE说明未声称远程已有本地提交。GPL、上游credits、第三方许可和原始Microsoft许可均保留。

ZIP内部唯一根目录为 `Minicraft-Rust-java230-dev2-windows-x64-rc/`，实际15个文件：

| 文件 | 解压字节数 |
|---|---:|
| `BUILD-INFO.txt` | 1035 |
| `CREDITS.txt` | 233 |
| `LICENSE` | 35141 |
| `MANIFEST.sha256` | 1242 |
| `RC-README.txt` | 1184 |
| `README.md` | 8722 |
| `SOURCE.txt` | 785 |
| `THIRD_PARTY_NOTICES.md` | 32842 |
| `licenses/EXPANSION230-MANIFEST.json` | 26286 |
| `licenses/MICROSOFT-VC-RUNTIME-LICENSE.docx` | 39553 |
| `licenses/MICROSOFT-VC-RUNTIME-LICENSE.txt` | 12037 |
| `licenses/RUST-COPYRIGHT-library.html` | 1499465 |
| `licenses/UPSTREAM-RESOURCES.md` | 2668 |
| `minicraft-rust.exe` | 4488704 |
| `vcruntime140.dll` | 178616 |

清单和内容逐项复核：无target/cache、PDB、Java源码/类/jar、测试世界、个人数据或重复开发副本；唯一EXE是游戏本体，不捆绑开发用parity/server程序。Rust runtime许可HTML来自实际工具链，不将整套编译器文档或缓存收入包。

## 验收结果

使用原有Cargo target，实际执行并全部exit0：

```powershell
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --locked -- -D warnings
$env:RUST_TEST_THREADS = '4'
$env:MINICRAFT_PARITY_RENDER_OUTPUT = "$PWD/target/parity/render-frame/rust"
cargo test --release --workspace --locked
cargo build --release --workspace --locked
```

**基线633项，最终仍633 PASS、0失败、0忽略**（client613/common7/server6/双客户端e2e7）。client实际14.36s，保留两日模拟原15秒预算；没有为变绿删除测试、改期望或扩大阈值。

- 重用 `assembly36_tests`：6项全部通过（2.91s），128四Seed及256/512代表Seed的新世界、全部阶段、实际楼梯调度、独立目录写盘/加载、复制改名、未知版本/备份恢复。它是自动化真实World/存储路径回归，**不是GUI Save & Quit已验收声明**。
- `scripts/parity224/run.ps1`、`scripts/parity230/run-world36.ps1`：614/722固定源/资源blob核验及实际Java编译运行通过；6组完整World的210个pack条目逐字节匹配既有fixture。19确定性JSON、5组官方Save、普通世界无探针/有探针/原fixture各文件一致，锁箱奖励和224原样本继续通过。未固定的runtime AI随机轨迹不伪称跨JVM相同。
- Release CLI `java224-generation`、`java224-noise`、`java230-generation`、`java230-depths` 全部exit0；完整World逐项比较通过上述原有测试。没有重写一套重复测试。
- 从ZIP实际解压到 `target/rc/java230-dev2/isolated run/解压/`（路径含空格与中文），使用另外的空工作目录，PATH仅含Windows/System32。**随包EXE `--self-check` exit0**，资源和音频数据自检通过；读取实际进程模块，VCRUNTIME确实加载自解压包内DLL，而非开发工具目录。
- 同一解压EXE以无窗口CLI导入独立的官方dev2世界副本：exit0，得到Seed42、六张128完整地图和33个持久实体；源文件每项SHA-256均不变。重复导入exit1，已有state哈希不变。所有数据只在RC忽略目录，没有触碰玩家实际Java/Rust世界。
- 打包脚本负例：未提交工作区拒绝生成；已有ZIP无显式 `-Replace` 时拒绝覆盖且哈希不变。15个ZIP条目、解压长度和SHA-256与外部清单一致。

日志统一位于 `target/rc/java230-dev2/validation/`；Java参考执行自身仍复用原有忽略 `target/parity/`。Oracle命令、源比较和重跑证据见 `validation/oracles.md`。初次用Windows PowerShell5.1调用旧Oracle遇到缺少GetRelativePath、以及中断/沙箱jar操作失败，保留记录；最终使用PowerShell7非交互、提升权限后全部通过，未降低固定源码校验。

## GUI边界、已知问题与人工检查

已实际启动解压版并观察到正常标题画面。Computer Use交互验收受到外部输入中止；随后用户明确表示“不需要computer use验收，我自己会验收”，因此停止所有窗口操作，**不声称已通过RC GUI新建/采集/Save & Quit流程**。隔离新建/存读的自动化证据如上。人工试玩由用户完成，本轮不再占用桌面。

未发现新增的Windows二进制加载或资源缺失问题；没有干净虚拟机/全部显卡和音频设备的覆盖。现有窗口显示Version2.2.4，RC标签说明dev2移植内容，未擅自升级正式版本或改存档标记。已有近似AI、上游越界箱、高级Spawner拒绝搬运、非完整多人同步等限制仍见README。本轮不扩展这些玩法。

只需少量人工验收：

1. 将整个ZIP解压到你选的新目录，用 `--savedir` 指向新的测试目录启动；确认标题、正常进世界和音画显示。
2. 新建Survival，采集一些材料/花，保存退出、关闭EXE，再启动同一测试目录，检查玩家、库存和采集位置保持。
3. 如需检查升级，用旧世界的副本加载保存；如需检查迁移，用Java dev2完整副本导入一个不存在的新目标，再保存重进。不使用真实原档做试验。

不要求重复人工核对已由Oracle验证的所有格子/数值。

## Git

分支：`chore/java230-windows-rc`。工程/README/许可/打包本地提交：`5bc92a90f6383cc71ad5123c57093cba454ed879`。本文件为随后审计提交；其最终完整HEAD、本轮提交清单和干净工作区复核写入忽略的 `target/rc/java230-dev2/validation/phase37-final-validation.md` 及最终答复，以避免提交自身SHA循环。

没有Push、Tag、GitHub Release、合并main或改写历史。未删除历史成果/个人世界，未开始Infdev或其它平台开发。
