# Infdev 实验引擎（Windows x64）

当前提供独立图形化飞行探索，以及单独的**基础 Survival 实验检查点**。
支持地表0、铁矿层-1、金矿层-2、宝石层-3、黑曜石地牢-4；已接通采集、工具、冶炼、苹果/饥饿循环及区块死亡箱；尚无完整Survival、怪物、天空、船只或多人。普通游戏仍使用固定世界，
`Legacy224`、`Java230Flora`、`Java230World`及既有存档不切换规则。
Phase 4A历史说明见[归档记录](history/infdev4a.md)。

## 窗口试玩

```powershell
cargo build --release --workspace --locked
./target/release/minicraft-rust.exe --savedir target/infdev-play
```

主菜单选择 **Infdev (Experimental)** → **New Layered World (v3)**，填写名称与有符号64位Seed，
选择Create。新建不改变普通游戏的模式、世界列表或默认`Java230World`。
默认实验根目录为`SAVEDIR/experiments`，GUI列表扫描其中的`infdev-v2`、`infdev-v3`和`infdev-survival-v4`，名称带版本标识。
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

## 基础 Survival（v4）

在实验世界列表选择 **New Survival World (v4)**。它创建独立目录，不把飞行世界转换成生存世界，
也不读取普通玩家存档。新玩家使用固定世界已有`Player`的空背包默认值、10点生命和10点耐力。
启动只在原点32格内寻找自然安全出生点；找不到时拒绝创建，不会铺草地伪造出生区。
沿用`Inventory`的27格容量、原有ItemId、ToolItem耐久、Tile碰撞判定、Recipe/ToolRecipe事务及HUD/背包渲染组件。
无限坐标适配器保存i64全局Tile位置和Player的格内像素；不是把有限World.tick整体移植到无限地图。

最短操作链：

1. WASD/方向键逐像素行走，树、岩石、矿石和工作台阻挡移动。空手面向树反复按C采集，掉落物会自动拾取。
2. 采集至少20 Wood。按Z，在手工列表选择Workbench并按C制作，消耗10 Wood。
3. 按X/E打开背包，选Workbench按C装备，面向草地/泥地按C放置；面向工作台按Enter打开真实合成菜单。
4. Wood Axe、Wood Pickaxe各需5 Wood；使用木镐采石后，可用5 Wood + 5 Stone制作Rock工具。
   斧、镐、铲使用原有木/石/铁/金/宝石工具配方；工作台另可制作Furnace（20 Stone）和Anvil（5 Iron）。
   没有改写原配方的材料或数量，武器、装备等未接线配方仍不开放。
5. 站上楼梯按V/P下矿，使用镐采集铁/金/宝石/青金石。返回对应楼梯可回地表；负坐标和区块边界仍适用。
6. 采集至少30 Iron Ore及10 Coal：熔炉每次把3 Iron Ore + 1 Coal变为1 Iron（铁锭），金矿对应Gold；
   在工作台用5 Iron制作Anvil，放置后以5 Wood + 5 Iron制作Iron Pickaxe。
   熔炉/铁砧均用背包装备后C放置、Enter打开菜单、V/P拆回；它们使用原游戏的即时配方交易，没有独立燃烧计时或内部库存。
7. 自然树每次受击有1/100概率掉1 Apple，空手也可获得；没有保底或凭空赠送。选择Apple后按C食用，
   消耗一个苹果与2点耐力，恢复1点饥饿；饱食或耐力为0时不消耗。森林中的大量树木是当前可探索获取的食物来源，
   下矿前建议准备苹果。动物、农业、钓鱼及烹饪尚未接入，其他食物不会自然出现。
8. 按Q丢一个所持物品、Shift+Q丢整堆；工具按实例丢弃并保留耐久，1秒后可自动拾回。
9. Esc先关闭背包/合成面板，再按Esc执行Save & Quit；重进恢复位置、生命、耐力、库存、工具、掉落物及地图修改。

装备铲子可把草变成泥土、挖泥土/沙子得到物品并留下洞。有限放置沿用已有TileItem规则：
Dirt填洞/流体、Plank铺水面、Stone或StoneBrick铺桥，以及放回已采集的花。未接线的树苗等放置会拒绝并保留物品。
V/P面向工作台、熔炉或铁砧可拆回相应真实物品掉落；楼梯上优先执行跨层。

新建v4使用显式`food_rules: JavaNormal`，按Java Normal难度消耗饥饿（400 tick/计数、7计数/饥饿点；
移动、缺耐力、回血有额外消耗）；饥饿>5可缓慢回血，饥饿为0每120 tick扣1生命、最低保留3生命。
菜单暂停计时。存档保留全部饥饿/回血计数器；新规则重生会恢复饥饿及计时。
旧v4缺少此字段时使用`Legacy`，保持原来不消耗饥饿、不自动回血的行为，没有自动升级。
边界：目前没有装备、农业、动物或怪物。
水中不回复耐力且持续消耗，耗尽后溺水伤害；熔岩造成伤害。死亡后按Enter回初始出生点，
出生点被阻挡会拒绝重生。**新建v4使用死亡箱；旧v4仍保留库存**，具体规则见下节。
下矿前请准备足够工具，尚无Boss/地下家具生成。
最多保留512个掉落实例和64个工作台/熔炉/铁砧；达到上限时拒绝新增操作，物品不会被静默删除。掉落暂不自然过期。
尚未推进随机方块tick或树/岩石的伤害衰减；部分采集伤害会持久化，不代表这些时序已与Java完整对等。
耐力与伤害由后台游戏tick推进，菜单暂停生存计时；GUI不执行磁盘访问或地形采样。

v4格式为`MinicraftRust.Experimental.Infdev.Survival.v4`，使用原`JavaLayersV1`地形、`MCIFCH04`区块头。
工作站类型、掉落、游戏RNG及原有Player/Inventory数据位于有校验的不可变`sessions/`对象中（单对象256KiB上限）。
同一个checkpoint原子发布session与区块根；LRU保存也使用完整会话，避免地图已采走而库存未提交。
GC保护当前、备份和内存引用；玩家对象损坏时回退完整检查点或拒绝，不会重置玩家或重生资源。
旧工作站缺少`kind`按Workbench读取，旧`Legacy`与Workbench默认字段再次保存时仍省略，保持旧session规范表示。
未知食物规则/工作站类型和其他未知/无法原样表示的session字段拒绝读取，损坏恢复继续保留GC hold。包含新食物规则或新家具的存档不保证被旧EXE降级读取。
v2/v3保持旧字节及生成含义，目前没有v2/v3→v4升级路径；使用独立新世界试玩。

## 区块实体与死亡箱（v4兼容扩展）

新建Survival显式保存`death_rule: ChestNormal`；旧v4缺少字段时按`KeepInventory`读取，
再次保存仍省略该默认值，保持原有死亡保留库存的语义。没有自动升级、v5目录或新生成器；
旧v2/v3、固定世界及Windows RC不变。新实体存档不承诺被旧EXE降级读取。

死亡箱直接移动原`Inventory`，保留堆叠数量、工具耐久和水壶实例（含历史UNKNOWN水量）。
active只是背包引用，清空引用而不额外复制手持物。死亡箱在死亡深度和i64全局Tile位置生成；
按顶部`LAST DEATH`坐标返回，走上箱子自动拾回，也可面朝箱子按Enter从邻格取回。
满背包只取能容纳的部分，剩余内容留在箱内；反复触碰不重复获得物品，取空才删除箱子。
不能用PowerGlove把死亡箱变成可携带家具。出生和跨层不自动远程拾回。
死亡地点被实体/实心地形阻挡、页面已满、ID耗尽或页大小超限时，明确保留玩家全部库存；
不会为了生成箱子清地形或吞物品。当前未接入的装备/怪物系统不因此宣称完成。

死亡、取出物品和首次死亡收据会立即请求检查点；玩家session、地图根与实体根通过同一个
`checkpoint.json`原子发布。失败时保留完整RAM所有权供重试；异常终止只恢复最后一次完整提交。
不可变实体页位于`entities/`，复用原AVL、文件同步、排他锁、备份与GC。
每页绑定depth/区块X/Y；实体用页内单调ID及0–63局部坐标，覆盖正负i64 Tile边界。
每区块最多64实体、每页256KiB；实体LRU与地形LRU各至多16页，视野实体快照至多4页。
取空页保留ID水位，避免重复使用旧身份。session保留实体根存在收据，丢失实体根不能被当成空世界。
未知实体类型/实例字段、错误页坐标或对象损坏导致完整检查点回退或拒绝；不独立重置玩家/箱子。
GC校验并保护当前、备份及内存可达实体与库存；恢复产生hold后继续拒绝回收。

**与固定Java Infdev的区别及证据：**

- 原版`DeathChest`按难度存续450/300/150秒；当前实验Survival固定Normal，起始18000 tick。
  仅玩家当前实体区块在存活且关闭菜单时计时，离线/卸载暂停。到0后保留受保护的可拾回箱子，
  不执行Java原版的过期散落/删除。这是防止尚未完善的无限实体生命周期吞物品的安全扩展。
- 邻格Enter拾回、不可阻挡玩家及放置失败保留库存也是明确的安全扩展，不宣称逐帧等同Java。
- 新Oracle实际执行固定源码的构造、满背包、工具耐久和重复触碰。
  原版`WateringCanItem.copy()`重新构造空壶，死亡箱复制731/23水量后变为0/0；
  Rust有意保留731/23及UNKNOWN，不移植这一数据丢失缺陷。
  源码为`entity/furniture/DeathChest.java`、`item/WateringCanItem.java:138-139`，
  实测输出在`target/parity/infdev4a/death-chest.json`。

目前分页实体仅接通静态死亡箱。原掉落物与工作站仍在有界session中保存（512/64上限），
未转换为可移动分页实体；没有怪物AI、离线模拟或通用无限容器更新器。水壶运行玩法尚未接入Infdev，
但共享物品模型中的已有实例经过死亡箱/磁盘测试保持完整。

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

GUI新飞行世界存于`ROOT/infdev-v3/NAME/`，格式为`MinicraftRust.Experimental.Infdev.v3`，
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
`python scripts/infdev4a/window-smoke.py`运行飞行、Survival各一对及死亡恢复三个独立窗口进程，用同一菜单/探索器和编程Input验证
自然生成、正负区块、楼梯至-4并返回、各层编辑、地下Save & Quit及重开。它不是人类键盘试玩；PNG为真实软件渲染输出。
窗口脚本还通过独立v4菜单从空背包执行采集、工作台、木/石工具、熔炉、铁砧和铁镐制作，真实矿/煤扣除，苹果自然掉落与食用，正常碰撞移动、下矿返回及重启检查；
`--survival-only`仅运行这一组窗口流程。它不会注入库存、传送角色或预置资源。
`python scripts/infdev4a/window-smoke.py --death-only`单独运行死亡链路：从空背包真实采集，
下矿后受控致死；在死亡检查点成功后强制终止进程，重开、复活并沿真实楼梯返回取物，
再次Save & Quit、第三进程恢复和GC校验。受控条件只改变生命值，不注入库存、摆放资源或传送。
自动输入经过实际菜单/窗口/worker/容器；这是原生窗口自动化，不是玩家真人验收。
本次实测地下-1层`(-42,-24)`，工具与Acorn/Apple/Stone/Coal/Iron Ore均来自正常采集。

新增回归覆盖部分拾回、满包、工具/不同水量实例、重复死亡、受阻/满页、过期保留、128实体页LRU、
正负i64边界、五层、磁盘恢复、死亡/拾回发布前中断、写入失败、损坏整体回退及GC备份保护。
完整门禁继续用`scripts/validate-windows.ps1 -JavaOracle`，原679项基线全部保留。
最终结果复用`target/validation/current/infdev-final-validation.md`及同目录日志，不另建阶段日志树。
本次配方依据固定世界`item/recipe.rs`及固定Infdev的`item/Recipes.java`；苹果与食物规则依据
`level/tile/TreeTile.java:149-168`、`item/FoodItem.java:47-54`和`entity/mob/Player.java:398-450`。
复用原ItemId/Inventory/ToolItem，无新增物品ID、Tile ID或世界命名空间；Java的树随机源与Rust会话RNG不同，
这里对齐规则和数量，不宣称每次掉落的随机序列或全帧玩家更新顺序逐次相同。
下次优先扩展可移动区块实体与工作站/掉落分页，补齐其生命周期；同时评估更大世界的分批校验/GC；
不重解释已有生成器版本，不将当前检查点当作完整无限世界游戏。
