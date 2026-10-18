# Infdev 实验引擎（Windows x64）

当前提供命令行探索游标：Seed → 按需生成64×64区块 → 跨边界移动 → 保存、卸载、重启恢复。
尚未接入GUI、Survival、实体、地下层、船只或多人。普通游戏仍使用固定世界，
`Legacy224`、`Java230Flora`、`Java230World`及既有存档不切换规则。
Phase 4A历史说明见[归档记录](history/infdev4a.md)。

## 运行

在仓库根目录使用PowerShell。实验根目录必须独立于玩家存档；名称已存在时明确拒绝。

```powershell
cargo build --release --locked -p minicraft-parity
New-Item -ItemType Directory -Force target/experiments | Out-Null
./target/release/minicraft-parity.exe infdev-surface-create target/experiments Surface42 42
```

进入后逐行输入：

```text
view
move -65 -1
set flower 11
save
unload
view
gc
quit
```

重新打开：

```powershell
./target/release/minicraft-parity.exe infdev-open target/experiments/infdev-v2/Surface42
```

`status`显示坐标、已保存区块数和缓存；`visit X Y`可直达有符号i64 Tile坐标。
`move DX DY`逐格移动，每轴限±256；负坐标使用floorDiv/floorMod，溢出明确拒绝。
`set`接受grass、water、sand、rock、tree、flower、cactus、stairs和u16 data，是诊断编辑入口。
`save`、`unload`、`quit`及正常输入结束会保存；强制终止仅保证此前成功发布的检查点。
显式`gc`要求先保存脏区块。CLI常驻缓存为4区块，库接口支持1–64区块。

## 地形版本与Java证据

固定上游为`42ba9192fcc229bcf615453a6504d415c777feaf`（GPL-3.0）。
实际调用链为`LevelGen.advanceChunk` → `LevelNoise` → `Biomes` → 各地表Biome。
移植七种地表群系的权重及生成分支：Rock、Forest、Ocean、River、Desert、RiverBank、Surface。
支持树、仙人掌、花卉data 0–11及原版地表材料；不把注册的19花种都宣称为自然生成。
本轮对照止于原版`UNFINISHED_STAIRS`阶段，不包括跨层楼梯连接或完整World初始化。

Java的Surface/Desert/Rock楼梯选择使用`new Random(System.nanoTime())`，不能逐格复现。
Rust在相同候选和概率上使用Seed及完整i64坐标哈希，属于明确的确定性扩展。
Oracle保留Java实际结果，严格比较群系、data和确定性Tile；时间随机候选单独校验并报告差异。
不以过滤候选后的结果宣称完整Tile相等。

存储坐标覆盖i64 Tile空间，但噪声输入保留Java signed-int语义：取坐标低32位，
因此地形噪声有2³² Tile周期；时间随机楼梯的Rust替代选择使用完整i64坐标。
原版HashMap群系遍历顺序也由Oracle核查。当前不宣称任意JDK或全部极限坐标已逐格验证。

## 存储、兼容与恢复

新世界存于`ROOT/infdev-v2/NAME/`，格式为`MinicraftRust.Experimental.Infdev.v2`，
生成器明确标记`JavaSurfaceV1`。不可变区块对象及AVL索引页分别存于`chunks/`、`nodes/`，
按哈希前缀分桶；固定大小checkpoint保存根引用，不再将全部区块索引写入manifest。
索引查询和修改路径为O(log N)，取消4A的8192区块上限；索引页缓存最多512页。
对象先写入并同步，再原子发布checkpoint及备份；Windows排他锁阻止同目录并发写入。
校验和用于检测损坏，不是认证签名；不承诺所有硬件断电场景零损失。

旧`infdev4a-v1`仍能按原规则打开。复制升级使用新目标，不原地转换：

```powershell
./target/release/minicraft-parity.exe infdev-upgrade target/experiments/infdev4a-v1/Old42 target/experiments Copied42
```

升级只读访问源数据，不恢复或改写源checkpoint；源锁文件缺失时明确拒绝，不创建它。
复制后的生成器为`LegacyHeightV1`，已保存区块及未来未访问区域都保留4A生成契约。
它不会静默升级成Java地表规则。未知格式、生成器或固定世界数据明确拒绝。

每128次实际持久提交按保存序号自动GC，跨重启继续计数；空操作不推进提交序号。
GC先验证当前checkpoint、备份及内存根，保护全部可达索引页和区块，再删除已识别的不可达对象。
未知文件保留；验证或删除失败会停止并报告，自动GC失败不撤销已经成功保存的检查点。
备份恢复或发现歧义会创建`gc-recovery.hold`，阻止后续回收；不要直接删除该标记来清空间。
应先保留完整目录副本并调查损坏。已登记对象损坏时尝试完整备份，否则拒绝，绝不重新生成代替。
打开时完整校验和GC均需O(N)扫描及临时可达集合；当前仍受磁盘、文件数和内存限制。

## 验证与下次入口

`pwsh -File scripts/infdev4a/run-terrain.ps1`编译并核验固定原版源码，复用`target/parity/infdev4a/`。
随后运行`./target/release/minicraft-parity.exe infdev-terrain-check target/parity/infdev4a`。
现有观察包含15个正负区块、61,440格、七群系，并覆盖signed-i32两端区块；另有50条原版噪声位模式样本。
i64极限属于Rust边界/恢复测试，不能当作Java完整极限坐标Oracle。
`cargo test --release --locked -p minicraft-client experimental::infdev`运行局部回归。
真实进程测试使用`python scripts/infdev4a/smoke.py --release`，复用Release parity；仅清理本次独占测试目录。
最终结果复用`target/validation/current/infdev-final-validation.md`及同目录日志，不另建阶段日志树。
下次优先评估打开校验/GC规模成本，再设计地下层、楼梯连接和GUI接线；不重解释已有生成器版本。
