# Minicraft-Rust

[Minicraft-Rust](https://github.com/Zaxaerith/Minicraft-Rust) 是基于
[Minicraft+](https://github.com/MinicraftPlus/minicraft-plus-revived) 的 Rust 游戏实现，
目前只维护 **Windows x64（MSVC）**。以 Java 2.2.4 的兼容修复为基础，移植传统固定地图的
**Java 2.3.0-dev2** 玩法；这是开发版、本地 RC，不是上游官方发行版。

## 可以玩什么

- 128、256、512 方形有限世界，地表、天空、地下和黑曜石地牢；Survival、Creative、Hardcore、Score。
- 采矿、种植、钓鱼、合成、装备、战斗、家具、箱子、死亡箱、任务、教程及配方解锁。
- 16 色染料、羊毛与床，染缸、织机、19 种花卉、自然花卉与浇水生花、Ornate Wood、彩色羊。
- 带等级的敌怪、Spawner、KnightStatue 与 Boss；独立水壶水量、工具耐久和容器物品状态。
- 原生存档及备份恢复、世界复制/改名、Java 存档导入、资源包、语言、按键绑定与手柄输入。

正常新建世界使用显式的 `Java230World` 规则，包含层 Seed、楼梯、建筑、宝箱和初始实体。
旧世界优先读取已保存地图；`Legacy224`、`Java230Flora` 历史规则继续保留。
固定场景已用实际 Java 源码进行逐格与持久状态对照，不代表所有 Seed、主题或 AI 都完全一致。

## Windows 运行

解压本地 RC 的整个目录，运行 `minicraft-rust.exe`。图像、音效和游戏数据已嵌入 EXE，
不需要 Java、Rust、仓库或 `.reference`。请保留随包提供的运行库、许可证和说明。

默认配置与存档目录是 `%APPDATA%\playminicraft\mods\Minicraft_Rust`。
要使用独立存档，在解压目录打开 PowerShell：

```powershell
.\minicraft-rust.exe --savedir ".\portable-data"
```

世界位于该目录的 `saves\世界名`。相对路径按当前工作目录解释；多个 RC 使用同一目录会共享存档。
升级前退出游戏，备份完整存档目录；旧 EXE 不保证能读取新存档，不要手改生成版本。

检查随包资源和音频数据（不打开窗口）：

```powershell
.\minicraft-rust.exe --self-check --savedir ".\selfcheck-data"
```

### 默认操作

| 操作 | 键位 |
| --- | --- |
| 移动 / 菜单方向 | W/A/S/D 或方向键 / 菜单用方向键 |
| 确认、使用面前家具 | Enter |
| 攻击、采集、使用或放置手持物品 | C |
| 背包 | X 或 E |
| 随身合成 | Z |
| 拾起可搬家具 | V 或 P |
| 丢一个 / 丢整堆 | Q / Shift+Q |
| 暂停、返回 | Escape |
| 全屏 | F11 |

背包内用 C 装备物品，面向工作站用 Enter 打开合成菜单。
手柄支持方向键/左摇杆、A 确认、B 返回、X 攻击、Y/Start 背包、左肩拾家具。
完整操作可在游戏的控制说明中查看，按键可在 Options → Key Bindings 修改。

## Java 存档导入

**先关闭 Java 游戏并复制完整世界，始终保留原档。** 只支持已有转换规则可完整表示的数据；
无法识别或无损转换的 dev2 内容会拒绝导入，不能把拒绝当成可以删除实体的提示。
不要把 Infdev、其他 Java 2.3 版本或 ZIP 直接交给导入器。

### Java 2.2.4

将整个 Java 世界目录的**副本**放入独立 Rust 存档根的 `saves\Java224Copy`，
目录内应直接包含 `Game.miniplussave`、`Player.miniplussave`、全部 Level 文件等原始文件。
用该独立目录启动 Rust，在世界列表加载副本；首次加载会在副本内写入 Rust `state.json`。
本路径是旧版兼容加载，**不是**下面 dev2 的事务式命令；不要直接选玩家的 Java 原目录。
如需继承 Java 全局解锁，将对应 `Unlocks.json` 复制到这个新建的隔离存档根，勿覆盖已有全局进度。

### Java 2.3.0-dev2

使用已关闭游戏的完整 dev2 世界目录或其副本。目标父目录必须存在，目标世界目录必须不存在：

```powershell
New-Item -ItemType Directory -Force ".\portable-data\saves" | Out-Null
.\minicraft-rust.exe --import-java "C:\JavaWorldCopies\MyDev2World" --import-to ".\portable-data\saves\JavaDev2Copy"
.\minicraft-rust.exe --savedir ".\portable-data"
```

可在导入命令末尾加 `--java-unlocks "C:\JavaWorldCopies\Unlocks.json"`，明确选择 Java 全局解锁。
源目录只读，转换在隔离临时目录完成，校验成功后独占发布；重复目标、损坏数据、未知字段或不支持状态会报错。
导入支持已验证的玩家/进度、六层地图、彩色物品、容器与死亡箱、独立水壶、带等级敌怪、Spawner 和雕像状态。
它不保证任意 Java 世界都能转换，也不承诺把 Rust 新增状态无损导出回 Java。
Java 官方 Save 没有保存的羊色及瞬时 AI/RNG 状态无法从存档恢复。

## 已知限制

- 完整怪物/Boss AI、逐帧轨迹和所有渲染细节尚未全面对等；生成对照覆盖固定场景，未穷举全部主题与地图。
- 新生成规则保留 dev2 原版的部分越界地牢箱及战利品，可能无法正常到达；最后锁箱奖励因此未必能在每张地图步行完成。
- Boss 房修复保护玩家放置的非竞技场方块及容器，和 Java 无条件覆盖房间的行为有意不同。
- 高等级 Spawner 暂不能通过 PowerGlove 无损搬运，拾起会拒绝并保留原实体，不会降成 level 1。
- 本地 JSON-lines 服务器仅提供连接、身份与玩家状态转发基础设施；不兼容 Java 联机协议，也不是完整多人世界同步或服务端权威战斗。
- 普通游戏仍使用有限世界。主菜单的 [Infdev (Experimental)](docs/infdev.md) 提供 Java 地表及四层地下地形、64位区块图形探索、楼梯跨层、受限编辑和保存恢复；尚未接入 Survival、实体、天空或船只。

## Windows 开发与验证

安装 Rust stable 的 `x86_64-pc-windows-msvc` 工具链，以及 Visual Studio / Build Tools 的
**Desktop development with C++**、MSVC x64 链接工具和 Windows 10/11 SDK。
根 `build.rs` 从标准 Windows Kits 安装目录寻找 `x64\rc.exe`，用于嵌入 EXE 图标；只装 Rust 不足以构建。
普通构建不需要 Java、Python 或外置 SDL2 DLL。

```powershell
cargo build --release --workspace --locked
.\target\release\minicraft-rust.exe --savedir ".\target\local-play"
```

工作区包含 `minicraft-client`、`minicraft-common`、`minicraft-server` 和 `minicraft-parity`。
所有 crate 共用现有 `target`；锁定依赖，不必升级 `Cargo.lock` 或修改版本号。

```powershell
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --locked -- -D warnings
$env:RUST_TEST_THREADS = "4"
$env:MINICRAFT_PARITY_RENDER_OUTPUT = "$PWD/target/parity/render-frame/rust"
cargo test --release --workspace --locked
cargo build --release --workspace --locked
.\target\release\minicraft-rust.exe --self-check --savedir ".\target\selfcheck-data"
```

Java Oracle 为可选开发工具，需要固定参考源码、JDK 17、PowerShell 7（pwsh）和已有依赖 jar；
运行方法见 [2.2.4 Oracle](scripts/parity224/README.md) 和 [2.3.0 Oracle](scripts/parity230/README.md)。
完整世界阶段对照使用 `scripts\parity230\run-world36.ps1`。
源码固定于 2.2.4 `47227a89fe05f1e3fe962f070f25c53111254a1f`、
dev2 `4dc00332e24a727b96be7b885d5cbbdd9c85263c`，不能仅按上游 `v2.2.4` tag 判断版本。

生成本地 Windows x64 RC：

```powershell
pwsh -NoProfile -File .\scripts\package-windows-rc.ps1
```

脚本使用现有 Release 产物，输出
`target\rc\java230-dev2\Minicraft-Rust-java230-dev2-windows-x64-rc.zip`、SHA-256 和文件清单，
不自动构建、上传或发布。RC 不包含测试存档、Oracle、Java 参考源码或构建缓存。
工程记录见 [文档索引](docs/README.md)。日常完整验证可运行
`pwsh -NoProfile -File scripts/validate-windows.ps1`；加 `-JavaOracle` 可重新执行固定源码 Oracle。
验证日志复用 `target/validation/current/`，不再按开发阶段累积重复目录。

## 来源、署名与许可证

Minicraft 原作由 **Markus “Notch” Persson** 于 Ludum Dare 22 创作。
Minicraft+ 的维护者包括 **David.b、Dillyg10、Chris J、afyber、Makkkkus、Litorom1**；
原始制作人员名单保留在 [游戏 credits](assets/assets/books/credits.txt)。
Rust 项目保留这些来源及资源署名，按 **GPL-3.0-only** 提供，完整条款见 [LICENSE](LICENSE)。

原版 2.2.4 的 403 个资源保留；固定 dev2 的必要扩展资源来源和哈希见
[资源说明](assets/expansion230/README.md) 与同目录 manifest。
Rust 依赖及运行库的版权和分发说明见 [THIRD_PARTY_NOTICES.md](THIRD_PARTY_NOTICES.md)。

本地 RC 未自动公开分发。向他人提供二进制时，应同时提供该精确构建修订的对应源码、资源和构建材料，
并遵守 GPL-3.0 第 6 条及第三方条款；未 Push 的本地提交不能假称已在远程仓库可下载。
