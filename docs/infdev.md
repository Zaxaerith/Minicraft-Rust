# Infdev 实验引擎（Windows x64）

当前提供独立图形化飞行探索：Seed → 按需生成64×64区块 → 跨边界与楼梯上下层 → 保存、卸载、重启恢复。
支持地表0、铁矿层-1、金矿层-2、宝石层-3、黑曜石地牢-4；尚未接入Survival、实体、天空、船只或多人。普通游戏仍使用固定世界，
`Legacy224`、`Java230Flora`、`Java230World`及既有存档不切换规则。
Phase 4A历史说明见[归档记录](history/infdev4a.md)。

## 窗口试玩

```powershell
cargo build --release --workspace --locked
./target/release/minicraft-rust.exe --savedir target/infdev-play
```

主菜单选择 **Infdev (Experimental)** → **New Layered World (v3)**，填写名称与有符号64位Seed，
选择Create。新建不改变普通游戏的模式、世界列表或默认`Java230World`。
默认实验根目录为`SAVEDIR/experiments`，GUI列表扫描其中的`infdev-v2`和`infdev-v3`，名称带版本标识。
使用已有4B世界时，可通过`--infdev-root target/experiments`指定其根目录；不要传单个世界目录。
原4A世界须先用下述CLI复制升级。已有目标绝不覆盖，损坏世界会显示错误而非重新生成。

使用现有移动按键（默认WASD/方向键）探索，角色可飞越树木、水域和岩石，摄像机跟随。
顶部显示深度与Tile坐标，跨越0、±64、±128可检查正负区块。尚未加载的视野显示占位网格。
站在真正的楼梯上按拾起键（默认V/P）上下层；对应楼梯不符时明确拒绝，不修补已保存地图。
面朝自然草地或Oxeye Daisy，按攻击键（默认C/空格）切换前方一格草地/雏菊；地下仅允许Dirt/Rock互换。
楼梯、矿石、流体和建筑拒绝编辑。原版地牢仍可能产生向下候选，-4以下明确拒绝通行。
`Esc`执行 **Save & Quit**，等待保存完成后返回实验世界列表。重新选中世界会恢复位置与修改。
关闭窗口也会等待保存；失败保持会话供重试，关闭窗口后的失败有Windows Retry/Cancel提示。
加载/保存错误有多行提示；Cancel明确放弃本次尚未保存的修改，仅保留上次检查点。

Seed 42的自然楼梯快速试玩坐标（从上到下）：地表`(-32,-20)`、-1层`(-113,-75)`、
-2层`(-115,-88)`、-3层`(-119,-102)`。按顶部坐标移动到该格再按V/P；反向沿同坐标返回。
这些来自真实生成后的搜索与窗口移动，未预先摆放；其他Seed不保证相同坐标。

打开校验、地形生成、保存与GC都在单个后台worker；GUI只读快照，不执行地形采样或磁盘IO。
命令和快照队列各容量1，视野快照最多4区块，存储LRU为16区块；每轮worker至多加载/生成1区块。
初次加载或大型GC期间窗口继续响应，但移动可能等待后台操作。退出时等待是有意的数据保护。

## 命令行诊断

在仓库根目录使用PowerShell。实验根目录必须独立于玩家存档；名称已存在时明确拒绝。

```powershell
cargo build --release --locked -p minicraft-parity
New-Item -ItemType Directory -Force target/experiments | Out-Null
./target/release/minicraft-parity.exe infdev-surface-create target/experiments Surface42 42
# 新的多层实验世界（独立目录，不升级已有Surface42）
./target/release/minicraft-parity.exe infdev-layers-create target/experiments Layers42 42
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
v3额外接受dirt、iron、gold、gem、lapis、lava、obsidian-wall、obsidian、raw-obsidian、ornate-obsidian、up、obsidian-door。
`visit-depth DEPTH X Y`是诊断定位；正常跨层使用站在楼梯后的`stairs`。GUI不提供任意跨层传送。
`save`、`unload`、`quit`及正常输入结束会保存；强制终止仅保证此前成功发布的检查点。
显式`gc`要求先保存脏区块。CLI常驻缓存为4区块，库接口支持1–64区块。

## 地形版本与Java证据

固定上游为`42ba9192fcc229bcf615453a6504d415c777feaf`（GPL-3.0）。
实际调用链为`LevelGen.advanceChunk` → `LevelNoise` → `Biomes` → 各地表Biome。
移植七种地表群系的权重及生成分支：Rock、Forest、Ocean、River、Desert、RiverBank、Surface。
支持树、仙人掌、花卉data 0–11及原版地表材料；不把注册的19花种都宣称为自然生成。
地表对照止于原版`UNFINISHED_STAIRS`阶段；新增地下Iron/Gold/Gem/Dungeon四群系的原始Tile/data对照。
地下噪声沿用原版`zOffset=0`，深度选择群系；各层使用相同显式LevelGen Seed，不能等同有限World.initWorld的nextLong层Seed组装。

Java的Surface/Desert/Rock楼梯选择使用`new Random(System.nanoTime())`，不能逐格复现。
Rust在相同候选和概率上使用Seed及完整i64坐标哈希，属于明确的确定性扩展。
Oracle保留Java实际结果，严格比较群系、data和确定性Tile；时间随机候选单独校验并报告差异。
不以过滤候选后的结果宣称完整Tile相等。

地下楼梯同样来自时间随机数；Rust在原概率与候选上加入深度盐的坐标哈希。
原版`Level.loadChunk`将父层Down连接为子层同坐标Up，洞穴周围填1格Dirt，地牢绘制5×5 dungeonGate。
v3在裁剪区块前计算完整祖先影响边界（最多5格），不向邻居或已保存区块回写，因此加载次序不改变连接。
重叠门房时重新保证所有Up中心属于Rust安全扩展。省略门房Lantern实体、固定世界中心Boss房与天空阶段。
Java地牢7×7随机熔岩装饰会跨区块写入、依赖时间与加载顺序，当前明确未移植；Oracle记录实际最终写入标记与数量。
Java原版连接Oracle实际执行`Level.loadChunk`验证自然楼梯；这证明连接语义，不能宣称时间随机楼梯坐标与Rust相同。

存储坐标覆盖i64 Tile空间，但噪声输入保留Java signed-int语义：取坐标低32位，
因此地形噪声有2³² Tile周期；时间随机楼梯的Rust替代选择使用完整i64坐标。
原版HashMap群系遍历顺序也由Oracle核查。当前不宣称任意JDK或全部极限坐标已逐格验证。

## 存储、兼容与恢复

当前GUI新世界存于`ROOT/infdev-v3/NAME/`，格式为`MinicraftRust.Experimental.Infdev.v3`，
生成器`JavaLayersV1`，索引键为depth/X/Y，checkpoint保存活动深度；区块头为`MCIFCH03`。
已有v2保持`JavaSurfaceV1`/`MCIFCH02`、地表键及旧JSON字节含义，不允许写入新方块或深度。
当前不提供v2→v3转换：新建独立v3，旧v2仍可打开探索，绝不将旧已保存区块按新规则重建。
未知版本或不匹配的格式/生成器/深度明确拒绝。不可变区块对象及AVL索引页分别存于`chunks/`、`nodes/`，
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
同一次GC内相同根只完整验证一次；不同根、每份元数据/位置仍检查，下次GC重新读取验证。
未知文件保留；验证或删除失败会停止并报告，自动GC失败不撤销已经成功保存的检查点。
备份恢复或发现歧义会创建`gc-recovery.hold`，阻止后续回收；不要直接删除该标记来清空间。
应先保留完整目录副本并调查损坏。已登记对象损坏时尝试完整备份，否则拒绝，绝不重新生成代替。
打开时完整校验和GC均需O(N)扫描及临时可达集合；当前仍受磁盘、文件数和内存限制。

## 验证与下次入口

此前v2的Windows本机测量（Release、同次原生权限运行、未清OS缓存；非v3基准或硬性预算）：

| 数据集 | 区块数 | 打开完整校验 | GC | 引擎进程采样峰值Working Set |
|---|---:|---:|---:|---:|
| 互异编辑区块 | 1 | 0.81 ms | 0.99 ms | 4.95 MiB |
| 互异编辑区块 | 256 | 44.84 ms | 56.46 ms | 5.10 MiB |
| 互异编辑区块 | 2048 | 325.96 ms | 354.92 ms | 5.62 MiB |
| 互异编辑区块 | 8205 | 1190.61 ms | 1524.50 ms | 7.02 MiB |
| 自然Java地表 | 64 | 10.89 ms | 15.21 ms | 5.20 MiB |

内存值为独立CLI存储引擎进程，包含进程基线，不是完整游戏/GUI内存承诺。
较早沙箱初测8205区块为2359/5555 ms，权限与文件调用环境不同，不将全部前后差值归功于优化。
`python scripts/infdev4a/measure-storage.py`可复测，10ms采样Working Set/Private Bytes，结果复用当前CSV。

`pwsh -File scripts/infdev4a/run-terrain.ps1`编译并核验固定原版源码，复用`target/parity/infdev4a/`。
随后运行`./target/release/minicraft-parity.exe infdev-terrain-check target/parity/infdev4a`。
地下结果运行`./target/release/minicraft-parity.exe infdev-underground-check target/parity/infdev4a`：
36个地下区块、147,456格，覆盖四层、正负相邻区块和signed-i32边界；确定性字段逐格比较，时间选择和随机装饰分别报告。
现有观察包含15个正负区块、61,440格、七群系，并覆盖signed-i32两端区块；另有50条原版噪声位模式样本。
i64极限属于Rust边界/恢复测试，不能当作Java完整极限坐标Oracle。
`cargo test --release --locked -p minicraft-client experimental::infdev`运行局部回归。
真实进程测试使用`python scripts/infdev4a/smoke.py --release`，复用Release parity；仅清理本次独占测试目录。
`python scripts/infdev4a/window-smoke.py`运行两个真实窗口进程，用同一菜单/探索器和编程Input验证
自然生成、正负区块、楼梯至-4并返回、各层编辑、地下Save & Quit及重开。它不是人类键盘试玩；PNG为真实软件渲染输出。
完整门禁继续用`scripts/validate-windows.ps1 -JavaOracle`，原658项基线全部保留。
最终结果复用`target/validation/current/infdev-final-validation.md`及同目录日志，不另建阶段日志树。
下次优先评估更大世界的分批校验/GC、天空与地牢装饰的确定性设计，再规划独立Survival接线；
不重解释已有生成器版本，不将飞行探索模式当作已完成无限世界游戏。
