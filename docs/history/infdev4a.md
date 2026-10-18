# Phase 4A：独立区块与存储检查点

2026-10-18，Windows x64。分支 `feat/infdev-world` 基于已推送的干净 main
`38705c4e54e59ffd11b0fb5f353c84553481cd88`。本分支只本地提交，不 Push/合并/打包。
main 对应 [Windows CI 39100566881](https://github.com/Zaxaerith/Minicraft-Rust/actions/runs/39100566881)
已实际全部通过；此前 Debug 性能预算失败及修正见 [维护记录](../maintenance.md)。

## 上游证据与范围

重新获取上游 main 后仍为 `42ba9192fcc229bcf615453a6504d415c777feaf`，本轮固定于此。
路径相对其 `src/client/java/minicraft/`：

| 源码 | 实际规则 | 本检查点 |
|---|---|---|
| `level/ChunkManager.java` | 64×64；稀疏嵌套 Map；floorDiv/floorMod；内部 x+y×64；stage0..4 | 64×64、有符号坐标、按需缓存；只持久化完整区块 |
| `level/LevelGen.java:82` | noise→biome→terrain→未连接楼梯；LevelNoise 使用全局区块偏移 | 原 height 噪声、全局坐标；不移植整套群系/楼梯 |
| `level/Level.java:488–525` | 玩家周边7×7推进加载；有父层时完成楼梯后置DONE | 独立探索游标及1..64区块LRU缓存，未接入旧Level |
| `level/noise/LevelNoise.java` | 10尺度/10通道；getHeight取通道8，double加权原float | 50个Java样本双精度位模式一致 |
| `saveload/Save.java:271–305` | LevelN目录、index、t.x.y/d.x.y逗号文本；只写DONE；x外y内 | 独立原生格式，**不宣称Java Infdev导入/导出兼容** |
| `level/biome/SurfaceBiome.java` | 部分楼梯用 `new Random(System.nanoTime())` | 不复制时间随机源，不声称完整地形确定性对等 |

上游 `loadChunk` 没有为无父层、stage3的天空分支置DONE；尚无区块卸载/事务式发布的完整链路。
这些是当前固定源码的限制，不将其等同于已完成的生产级无限世界。
Save的通用writeToFile会清空共享data；**不能**只看局部循环就误报累积data缺陷。

原始 Simplex 的3D梯度周期为48向量，历史Rust为56向量，首次Oracle在seed0/(0,0)即发现差异。
新增 `Simplex::infdev` 仅供实验规则使用，复用相同的前46向量及独立原版尾部/周期；旧 `Simplex::new`、全部固定世界路径不变。
没有改旧黄金期望。Java源提取首次遇到上游eol属性产生CRLF，随后改为直接复制原始Git blob，3个源码仍逐字节核验。

## 已实现的实际功能链

代码：`crates/minicraft-client/src/experimental/infdev/`，执行入口在parity的 `infdev_console.rs`。

- 世界Seed→全局噪声→按需完整64×64区块。无全世界预分配；访问远坐标不分配中间区域。
- i32有符号Tile坐标（含负边界），局部索引0..4095；一步移动检查溢出，目标加载成功才更新游标。
- Grass/Water/Sand/Rock及独立u16 data可编辑。**材质阈值是Rust的最小实验扩展**，不是Java群系选择或完整Tile Oracle。
- 容量受限LRU；淘汰前检查点、显式卸载、独立进程退出重开后保持坐标、Tile/data及已保存区块。
- 实际命令行11×7文本视图；这是探索游标，**没有Survival碰撞、实体、背包、采集或图形游戏接线**。

旧Legacy224/Java230Flora/Java230World及其Seed-only、正常创建入口完全不切换。
未触碰稳定Item/Tile ID、旧存档字段、Cargo.lock、资源或Windows RC ZIP。

## 格式与失败恢复

新目录仅为 `ROOT/infdev4a-v1/NAME/`，名称严格校验、create_dir独占创建、已有目标一律拒绝。
manifest标记 `MinicraftRust.Experimental.Infdev4A.v1` / `InfdevHeightPrototypeV1`，无兼容默认猜测。
固定世界state/world.json明确拒绝；未知字段/版本、重复或非法坐标、损坏和超限均拒绝或整份备份回退，绝不依Seed重生成已经登记的区块。

区块为不可变12,312字节对象：magic、Seed、坐标、4096个实验ground/u16 data；64位FNV校验仅用于检测损坏，**不是认证签名**。
先写入并sync所有对象，再复用既有atomic_write提交一个manifest及备份。manifest包含完整对象索引与游标位置。
对象槽位通过create_new独占创建；掉电残留的未提交对象保留，重试另用槽位，不覆盖旧对象。
加载时校验整个checkpoint的每个对象；若需回退，恢复完整上一checkpoint并明确提示，保留备份。
Windows排他共享句柄阻止同时写入，进程被终止后操作系统释放，无陈旧租约锁死。

测试模拟了提交前中断/残缺对象，并对真正CLI进程执行TerminateProcess；不是物理拔电、存储控制器断电测试。
Windows上依赖既有文件sync及原子替换能力，不宣称所有硬件掉电场景零损失。
原型限制：最多8192个登记区块、manifest2MiB、缓存64块；旧对象/孤儿对象暂不GC，长期存储压缩和更大索引后续设计。
这是可验证4A检查点，**不是已完成无限世界规模治理**。

## 实际验证

- `scripts/infdev4a/run.ps1`：编译固定原始LevelNoise/Simplex，3个blob核验；5种signed Seed×10坐标共50条bit-exact样本（包括i32边界）。源输出与小型受控TSV一致。
- 9项局部测试：坐标/溢出、50条Oracle、生成顺序、移动/淘汰/保存/卸载重开、提交中断及残缺孤儿、损坏整体回退/拒绝、重复/未知版本/固定世界拒绝、Windows共享拒绝重试、超大元数据/缺失对象。
- `python scripts/infdev4a/smoke.py`：真实CLI进程跨到(-65,-1)、rock/data731、卸载及重启；另一个进程改23后强制终止，再开仍731；重复目标拒绝。只使用本调用独占临时fixture并清理本调用数据。
- 格式、严格Clippy通过；完整4线程Release Workspace **642 PASS**（旧633+新9，client622/common7/server6/双客户端7），0失败/0忽略，client5.07s。原15秒模拟预算保留。
- 全部旧224/230原始地图、完整World、迁移、实例/容器/死亡箱回归随Workspace通过；main整理阶段已重跑真实Java驱动，210个World条目/19JSON/5官方Save及224样本一致。本次未重复耗时Java固定世界编译，未改变其源码/样本。
- Debug parity CLI构建与真实运行通过；没有打包新的游戏RC/正式Release，没有启动GUI验收。原RC ZIP SHA仍为 `8c9d81838c79e55527aa6d1665ae2cc0f813ae28261da18fdd229382002067b7`。

必要失败记录：最初用历史3D梯度导致新Oracle首样本失败，修正版本分离后全部50条通过；材质测试原先错误假设任意两个Seed在单区块内必定映射不同材质，改为验证实际连续噪声Seed差异及生成确定性，而非伪造材质输出。新Clippy建议改用as_chunks，未豁免lint。

最终SHA、Git状态、命令退出码和CI结果汇总放 `target/validation/current/infdev-final-validation.md`（忽略），避免自引用提交。
本轮Phase4代码只本地提交；不Push、Tag、Release或合并main。缺失：普通图形游戏与跨层、群系、怪物、结构、Java Infdev存档转换、长期对象GC/索引分页、载具等。

## 手动运行（可选，无需重复GUI验收）

从仓库根打开PowerShell，使用一个已存在的独立实验根目录：

```powershell
cargo build --locked -p minicraft-parity
New-Item -ItemType Directory -Force target/experiments | Out-Null
./target/debug/minicraft-parity.exe infdev-create target/experiments MyExperiment 42
```

逐行输入 `move -65 -1`、`set rock 731`、`save`、`unload`、`view`、`quit`。
再次运行 `./target/debug/minicraft-parity.exe infdev-open target/experiments/infdev4a-v1/MyExperiment`，输入view应仍显示坐标(-65,-1)和data731。
不使用真实玩家存档作实验，不手改旧世界类型。普通游戏EXE仍创建Java230World。
