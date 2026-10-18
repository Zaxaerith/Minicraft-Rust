# Java 2.3.0-dev2 Phase 3：条件式迁移、原始六层对照与装饰补齐

日期：2026-10-17（Asia/Hong_Kong）。这是已验证范围的阶段性交付，**不是通用完整 Java 世界迁移完成声明**。包含不能无损表示内容的有效 Java 世界仍整档拒绝；常规 Java 世界通常含 Spawner、KnightStatue 或带等级敌怪，因此也可能被拒绝。不能删去这些数据来使导入成功。

## 基线与来源

开始工作区干净，HEAD=`c9d2e05723ff637f1b18dcd17bc549186d88728b`，在该点建立 `feat/java230-world-migration`，完整继承 Phase1 `bdc960cc334baf8f5cbbf5943bd0222573a80471` 和224修复 `eac5b77001ca065a86c0750be47090b43386f395`。已阅读四份历史审计、既有最终验证和两套脚本。没有从旧main开始。

实际 `git ls-remote` 复查：2.2.X=`47227a89fe05f1e3fe962f070f25c53111254a1f`；2.3.0-dev=`4dc00332e24a727b96be7b885d5cbbdd9c85263c`；main=`42ba9192fcc229bcf615453a6504d415c777feaf`。不采用错误v2.2.4 Tag作稳定源码，也不采用Infdev实现dev2功能。

Java引用路径均相对于固定版本 `src/client/java/minicraft/`。本机固定源码在 `.reference/release-2.3.0-dev`、224在 `.reference/release-2.2.x/minicraft-plus-revived-47227a89fe05f1e3fe962f070f25c53111254a1f`。`scripts/parity230/run.ps1` 和224脚本先核验所有源码/资源Git blob，再完整编译固定源码；旧jar只提供第三方依赖，新classes及固定resources优先。没有修改上游游戏源码。

## 1. 实际磁盘格式与转换矩阵

重要证据：dev2 `level/ChunkManager.java:8–33` 是有限宽高、64×64、short[][]容器；`saveload/Save.java:270–294` 仍写完整 `LevelN.miniplussave` 和 `LevelNdata.miniplussave`。**没有区块文件，也没有Infdev无限稀疏存储。** 224/dev2的Game、Player、Inventory、地图磁盘结构基本相同；注册表、Flower data、彩色床实体名发生语义变化，不能据此绕过版本校验。

| 数据类别 | Java224 | Java dev2 | Rust当前编码/策略 | 丢失风险/拒绝边界 |
|---|---|---|---|---|
| 版本/元数据 | Game十字段（Save224:180–190） | 十字段，版本2.3.0-dev2（Save230:212–222） | world.json与原生format2 state；精确识别dev2，记录source_version | 未知版本、额外字段拒绝；native game_version仍是既有2.2.4-rust格式标记，未冒充Java230导出 |
| Seed/尺寸/难度/模式/主题 | Game世界Seed，Level宽高/各层Seed；Score复合字段 | 相同；Score可选10/20/40/60/120（Settings.java:22） | 保留世界/各层Seed、难度、模式、剩余ticks与初始分钟；正方形128/256/512 | Java不把主题/地形类型存入这些世界文件，也无已存天气字段；保留地图，不能声称恢复未知主题 |
| 六层与深度 | Java数组[-3,-2,-1,0,1,-4] | 相同（World.java:29,62–68） | 保留源数组/深度，跨层按depth查找 | 必须六层、尺寸一致，错层/缺层拒绝，不套用旧native索引 |
| Tile与坐标布局 | 磁盘名称、列优先x外y内，data独立 | 同布局；运行时新增ID | 显式源版本映射，转native行优先，保留u16数据 | 未知名称/数字token/长度错误拒绝，不替换Grass/Air |
| 流体/火把/楼梯/门/围栏/标牌 | 名称+data，Torch/Sign/Fence data是底层ID | 同规则但底层ID有碰撞；signs.json的level是depth（Save230:312–323） | 底层ID独立映射；sign深度转索引，多行以换行保存 | 无法表示的递归或有实例状态底层拒绝；四行GUI仍非完全对等 |
| 花卉/羊毛/装饰 | Flower旧形状数据；6色羊毛 | Flower ordinal0–18，新增10羊毛ID60–69，OrnateWood59 | Flower230 native69，羊毛native59–68，OrnateWood追加70 | 旧Flower2及旧ID不变，不能整数直传 |
| 家具/颜色 | 类名[x:y:状态:层索引]，床普通类名 | Bed写彩色name（Save230:445）；Crafter写type含DyeVat | 既有家具kind/state，床色state；Lantern ordinal；容器精确替换；locked转反向native位 | Spawner等级、KnightStatue bossHealth不能表示时拒绝；Java本来不保存TNT引线和堆肥进度 |
| 玩家/出生点 | Player15字段（Save224:305–338） | 同15字段（Save230:337–370） | 位置/生命/额外生命/饥饿/护甲/积分/出生点/药效/衣色与配方；原衣色整数另保留 | Java本来不存朝向、耐力、攻击动画、恢复计数器等，不能虚构源进度；超native表示范围报错 |
| 物品/工具 | 名称_count或工具名称_dur | 同编码，Java堆叠上限100 | 准确名称和明确别名；native上限999，总数守恒，工具独立实例 | 槽布局可合并，不能保证原堆顺序；非法数量/耐久/容量拒绝 |
| held与背包 | Save先写active再bag，无标志 | 相同；Load.java:1142–1179全部放bag，不恢复held | 每项只入库一次，active=None；容量不足拒绝 | 不把第一项复制成独立held；Power Glove按官方Load:1159跳过临时占位物 |
| 水壶实例 | 每个Watering Can_content | 同样独立实例 | 当前native仍是数量+玩家共享watering_content | **任意位置出现水壶即拒绝dev2整档**，不拿第一壶内容替代多壶状态 |
| 实体/动物/死亡箱 | Mob生命、Enemy等级、Sheep cut、死亡箱time | 同基本布局（Save230:404–447） | Cow/Pig/Sheep生命与cut、level1 Boss、已支持家具；死亡箱剩余ticks与全部内容 | 普通敌怪等级/未知实体拒绝。Java本地Save主动不写ItemEntity/箭/火花/粒子，也不存羊色/AI阶段/RNG |
| 时间/进度 | Game时间；Player已解锁配方；advancements owner/时间戳；全局Unlocks | 同结构，recipe owner扩展 | dev2 owner映射java230:，保留源JSON/时间；已完成任务不重新领奖；可显式提供全局Unlocks.json完整保留 | 未提供全局文件不能恢复账户成就；未知owner原文保留不代表其玩法已接线 |
| 未知/未来字段 | 旧别名与历史迁移门禁 | 只声明准确dev2布局 | 新native字段serde default；未知源文件/实体/物品/方块拒绝 | 不是任意未来JSON透传器；Sign额外未知元数据无专门模型。损坏/截断不能假成功 |

详细转换实现在 `crates/minicraft-client/src/saveload/load/java230.rs`，复用 `load.rs` 的既有解析和世界状态转换，没有复制第二套通用存读档系统。新增 `java_level_seeds`、`java_global_unlocks` 都有兼容默认值，不破坏旧native格式。

### Tile映射门禁

| Java ID | 语义 | native ID |
|---|---|---|
| 0–58（除2） | 既有对应稳定方块，逐项match核实名称 | 对应0–58；实现仍显式匹配，不作整数强转 |
| 2 | dev2 Flower，data=19种ordinal | 69 Flower230；旧native2保留旧语义 |
| 59 | Ornate Wood | 70 |
| 60,61,62,63,64 | Brown/Purple/Cyan/LightGray/Gray Wool | 59,60,61,62,63 |
| 65,66,67,68,69 | Pink/Lime/LightBlue/Magenta/Orange Wool | 64,65,66,67,68 |
| Torch/Sign/Fence的data | 源ID按本表重映射 | 例如Java62 Cyan -> native61，不能原样存62 |

`migration-registry.json` 保存实际70项Java名称/ID及Items名单；映射源码给出每一项。旧native0–69未插入或重排；旧142项物品顺序仍有逐项fixture门禁。

### 实体支持表

| 源实体类别 | 当前转换 |
|---|---|
| Player | 校验重复实体位置/层与Player文件一致；用一个native玩家表示 |
| Cow/Pig/Sheep | 生命、坐标、层、sheared；源不保存羊色 |
| AirWizard/ObsidianKnight level1 | 保留生命及位置；其他不能表达的等级拒绝 |
| 普通敌怪 | 因native没有实例级lvl及完整缩放语义，明确拒绝 |
| Chest/DungeonChest/DeathChest | 精确库存/工具，锁位转换、剩余期限；不混入默认loot |
| 彩色Bed、DyeVat、已知Crafter/Lantern/其他已表示家具 | 按实际名称/ordinal映射，严格字段数 |
| SignTileEntity | 验证对应Sign格，文本由signs.json恢复，不重复生成实体 |
| Spawner/KnightStatue/未知类 | 无法无损表示状态，整档拒绝 |
| loose ItemEntity等 | 官方本地Save不输出；输入出现未支持实体仍拒绝，不声称恢复源已省略内容 |

## 2. 真实Fixture与安全发布

扩展已有 `scripts/parity230/Oracle230.java`，新增同一框架内 `Migration230.java`。真正调用官方 `new Save(worldname)`，没有修改源游戏Save。四份小ZIP只用于仓库测试打包，**用户ZIP导入不支持**：

- `migration-base.zip`：六张实际raw LevelGen地图、固定42..47各层Seed、可控到达区、世界时间及玩家状态。不是完整World.initWorld生成样本。
- `migration-colors.zip`：held Iron Pickaxe17、总Wood1008、Cyan Wool4/White Lily2/Red Dye3/Ornate Wood2/Key1，Violet18、Cyan羊毛及Torch62、四行Sign、实际楼梯对、Cyan Bed、DyeVat、箱内工具11、死亡箱工具7/花2/time123、剪过的羊和牛、锁定和空锁态反例箱。官方实际Load.loadInventory证实held归入bag且active未恢复。
- `migration-score60.zip`：官方Score60设置，12345剩余ticks。缺失可选adv/sign的成功用例是明确标注的Rust变异测试。
- `migration-instances.zip`：实际水壶731与23及level3 Zombie，必须拒绝。

实际Save先对原始terrain加小范围安全区及测试实体是可复现fixture设置；不冒充自然完整世界布局。仅提交约二十余KiB/份压缩样本及小JSON；完整TSV/classes/skins/log留在忽略target。`migration-unlocks.json`来自实际成就API后官方Save.writeUnlocks。原始403资源文件未替换。

`migration.rs` 流程：只读且受限的平面目录快照 → 源版本/字段/地图/物品预检 → 目标父目录内独占隐藏stage → 转换 → native序列化并重新完整校验 → 原子写state和metadata → 独占目录发布。最多32文件、每份16MiB、总128MiB，全局Unlocks另限1MiB；拒绝链接、子目录、未知文件、越界路径、档案输入和非UTF8。

目标任何既有路径（含空目录、损坏世界、未被菜单识别的文件）都不能覆盖。Windows实测普通rename会替换并发创建的空目录，故改用拒绝替换的MoveFileW；Linux/macOS分别用renameat2(RENAME_NOREPLACE)/renamex_np(RENAME_EXCL)，未本机执行。文档依据：[Microsoft MoveFileW](https://learn.microsoft.com/en-us/windows/win32/api/winbase/nf-winbase-movefilew)、[Linux rename](https://man7.org/linux/man-pages/man2/rename.2.html)、[Apple声明](https://raw.githubusercontent.com/apple-oss-distributions/xnu/main/bsd/sys/stdio.h)。同卷发布；不支持的文件系统应返回错误，不回退为覆盖操作。Windows不承诺未实测的断电持久性；Unix发布后尝试父目录sync，失败明确提示已安装但sync失败。

正常失败删除本次独占stage，不改源/旧目标；强杀残留stage位于隐藏嵌套目录，菜单不视为世界，重试不清理未知孤儿。测试注入发布前失败及竞争空目标。导入标记为imported-java，完整state和backup均丢失时**拒绝重新生成**。Java230菜单内自动原地转换仍被禁止，必须显式新目标CLI。旧224路径不因此变成dev2宽松路径。

## 3. 实际玩法与修复证据

`world/migration_tests.rs` 执行真实Java Save → 导入 →实际World →游戏操作→磁盘Save/Quit→Load：

- 对全部98304个Tile/data逐格核对列/行转换、花18和Torch底层碰撞；六层/各层Seed/位置/出生点/生命/饥饿/衣色/时间及原始进度核对。
- held工具耐久17只出现一次，总Wood1008不变；箱内工具11、死亡箱工具7保持独立；配方owner/时间戳保存。
- 原Violet采摘拾回，在实际导入的DyeVat菜单制作Purple Dye；原楼梯跨层往返；Cyan Bed睡眠与出生点；死亡箱123期限随睡眠/实际tick减少，取回数量守恒。
- 实际箱子菜单消耗源Key，取出Gem2/Gold3；原生磁盘恢复严格比较全部状态（只将已知HashSet完成集合排序，其他数组不删比较）。

真实发现并修正：Java DungeonChest.isLocked与native unlocked位相反，先前直接转bool颠倒锁；旧224容器导入还先填默认loot再叠加原库存。两条路径现正确转换并精确替换，容量不足报错。官方224 Save.writeEntity的锁定Wood9箱与未锁空箱fixture，以及27/28独立工具容量边界验证此修复。实际菜单以前绕过锁检查也已修复，无钥匙不打开，一把钥匙只扣一次，状态重载不反锁。

验证期间暴露并纠正的测试/驱动问题：测试把“打开箱菜单”误认为“取出物品”，现执行真实转移；Java物品名为Gold而非Gold Ingot，驱动修正后重新生成；水壶拒绝等门禁未放宽。新增Score60白名单遗漏经Settings源码和官方Save fixture证实并修正。旧配方目录1到2静态升级，保留已有完成/时间戳，真实Phase2形状（从未解锁OrnateWood）回归后可在实际菜单首次制作。

边界：自动化调用真实World动作不等于真人GUI验收。标牌四行数据可保存，现有单行编辑界面仍有限制；箱内工具可以无损存读，但现有箱UI工具选择/转移尚非完整Java。最后一个地牢锁箱解锁奖励5个Gold Apple尚未对齐；普通敌怪AI/等级、Spawner/Statue实例、水壶实例模型为后续专项。当前没有删除或重置这些输入来制造成功。

负面门禁包括未知版本、ID/名称、实体、物品、耐久、超容量、错尺寸/深度/缺层、损坏/截断、超大文件、未知文件、子目录/路径穿越/ZIP、重复目标、发布中断及竞争目标。源字节及旧目标逐项不变断言；没有仅检查JSON可反序列化。

## 4. 固定世界生成范围

保留Legacy224和Java230Flora历史语义，未新增一个夸大完成度的版本名。固定224/dev2 core/World.java相同；LevelGen主要实际改动是地表花卉随机调用（dev2:426,431–435），其余地下/天空/地牢主要是ChunkManager接口改写。

新增 `generated-depths.json` 和 `java230-depths` 对比真实24张raw地图：128 Seed42/256、256 Seed42、512 Seed42，每种六层[-3,-2,-1,0,1,-4]，全Tile/data比较并报告首次坐标差异。**原始六层数据已一致，无须改写地下算法或更新旧224黄金值。** 旧224 Seed42逐格、Seed256黄金哈希、噪声位模式，以及230地表八场景仍独立保留。

未完成的完整世界差异必须保留：Java World.java:139–148逐层Random(worldSeed).nextLong，Rust既有组装各层直接使用worldSeed；Java Level.java:201–245结构/parent楼梯/天空房屋/补箱/Boss次序与Rust不同。Java:1163–1180地牢花园或宝箱模板二选一，Rust structure.rs现同模板Flower/Grass；Java:296–354补地牢箱，天空Boss生于中心而native现房屋位置。raw Oracle不覆盖这些阶段，不能说“完整Java230世界生成已完成”。本轮没有强改已有Seed-only或Phase2世界结果。

地图磁盘完整state继续优先加载，不因新内容重新生成；缺标记按Legacy224，旧Phase2仍Java230Flora。imported-java state缺失不作Seed恢复。旧metadata/native备份、copy/rename、旧物品和世界版本门禁都保留。

## 5. 剩余内容与本轮装饰移植

注册表运行差集核实：在Phase1完成项之外，独立缺失物品为Ornate Wood；本轮追加ItemId（第203项）与Tile70，未改变旧142项物品、旧0–69方块。注册→工作台1Wood制1→在Hole/Water/Cloud放置→斧头拆除/掉落→保存恢复全部接线；Lava放置失败、错误工具不拆、最后一次耐久仍掉落、天空恢复Cloud、满包/不足材料不扣除都有真实Java或World回归。

Java参考：item/TileItem.java:43（plank旧物品贴图），Recipes.java:63；level/tile/DecorTile.java（Axe、4-tier耐力、一次耐久，无随机伤害抽样，深度1恢复Cloud否则Hole）；Tiles.java的OrnateWood59；resources/recipes.json的ornate_wood解锁。Rust：item/{mod,tile_item,recipe,crafting}.rs，level/tile/mod.rs，world/tile_attack.rs，assets.rs及扩展解锁目录。没有把OrnateWood擅自加入Java不支持的Torch/Fence可放基础表。

仅新引入 `assets/expansion230/textures/tile/ornate_wood.png`，逐字复制固定dev2原图，manifest记录sourceCommit和SHA256，GPL-3.0及原项目署名保留；物品复用原plank。扩展图片总107，解锁定义94（含原93），其余403旧资源未改。没有依赖升级。

尚有明确dev2行为差异：彩色羊自然生成/染色/按颜色剪毛和死亡掉落（Sheep.java:78–113），native仍仅白羊毛；Java本身Save未保存羊色，导入不能发明历史颜色。配方定义110→187：新增79、移除2、修改14；扩展覆盖新增和必要变更，Loom的white_wool别名等价，官方完成进度仍保留。Java移除普通bed与arcane_fertilizer解锁，Rust为旧世界保留旧配方兼容行为。教程/任务/成就定义未发生版本变化。

Sandstone/Raw Ethereal仅在dev2 DecorTile枚举、未正常注册，不列为遗漏玩法。Infdev Boat、Biome和稀疏区块单列为非本轮目标。资源的旧白羊毛图与dev2 white_wool有轻微色差，未无故替换旧403资源；不声称像素渲染完全相同。随机战斗和典型场景Java/Rust渲染对等仍UNKNOWN，没有以原生截图冒充Java图。

## 6. 工程验证与准确边界

开发过程中局部38项java230测试、独立224箱/菜单/容量回归、严格workspace Clippy均已实际通过；开发期Windows增量缓存硬链接不可用警告只导致拷贝，没有为消除环境提示改依赖/断言。Java两套完整固定源码Oracle实际重编译通过，后续驱动增量编译使用同一已验证classes；Phase1/2 JSON fixture语义复核保持不变。

最终代码提交后，在**同一最终HEAD**统一执行以下门禁，完整命令、退出码、测试总数、计时、精确SHA及Smoke结果集中保存于 `target/parity/java230/phase3-final-validation.md`。该文件不提交，避免为记录自身SHA循环提交；交付时应结合它阅读本节，不把存在命令视为通过。

```powershell
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo test --release --workspace --locked
cargo build --release --workspace --locked
# 另：Release EXE --self-check；java224-generation/noise；java230-generation/depths；
# 实际java230-import成功/拒绝/重复目标；128/256/512真实生成与磁盘恢复Smoke。
```

首次全量Release发现3个历史目录长度断言仍期待70方块/44工作台配方（其他561项通过），按已执行Java OrnateWood证据更新为71/45，并新增末尾产物/成本及旧索引不变断言。未删除测试或更新旧黄金地图。首轮失败日志保留，修正后在新的最终HEAD重跑门禁。

实测最终代码验收：Release Workspace **584项通过、0失败、0忽略**（client564/common7/server6/双客户端e2e7），fmt、严格Clippy、Release workspace build与真实EXE自检通过。默认并行度曾使既有两日模拟触发15秒预算（563项功能通过）；相同测试单独7.85秒通过，再以 `RUST_TEST_THREADS=4` 完整运行14.68秒通过，未放宽预算或跳过断言。首次失败和隔离证据均保留。最终门禁沿用4线程以降低资源竞争。

Release实际Java对照：224六层地图/噪声double位模式、230原有地表八场景、新24张raw全层逐格比较均通过；所有已提交dev2 JSON/四份官方Save ZIP内容与实际Java重放一致，107张扩展图片与固定源及SHA256一致。128/256/512真实Rust World单次生成分别87/142/153毫秒，花166/652/3296，均磁盘保存恢复通过；不是统计基准或RSS。实际Release CLI接受colors世界、拒绝双水壶样本、拒绝重复目标，接受/拒绝源文件哈希及已存在目标哈希均保持不变。最终文档提交不改变上述代码，仍在该提交HEAD重跑四项门禁，精确结果见最终记录。

本机为Windows x64 MSVC/Rust2024，现有target复用，无第二套构建目录。最终EXE通过隐藏Start-Process取得真实退出码；自检savedir和生成产物限于target。全量测试渲染输出指定忽略target路径，避免修改原跟踪图片。Linux/macOS运行 **NOT RUN**；三平台GitHub Actions **NOT RUN**，工作流存在不等于远程通过。CI定义保留，未发现需为本轮修改的明确流程错误；新增平台独占发布函数仅Windows有实测。

## 7. Git与验收

本地分支 `feat/java230-world-migration`；首个检查点 `d4d0a03258cba33eb783ce6d4ff92ed441ce310a` 为严格事务式dev2导入及raw全层Oracle。后续提交补装饰/容器回归与本审计。**最终完整SHA、提交列表和干净状态以最终验证记录及交付回复为准**，不会为把自身最终SHA写入本文件产生循环提交。没有Push、Tag、Release、merge main或改写历史，没有修改玩家真实存档。

Windows玩家只需验收改动相关GUI：

1. 在**固定dev2 EXE**用一个测试世界副本Save & Quit，保留原档完整副本；游戏关闭后复制世界目录为只读来源，不用Infdev档。若含不支持的敌怪/Spawner/Statue/水壶，预期明确拒绝；不要删数据绕过门禁，可用本项目官方Save生成的migration-colors样本做已支持路径验收。
2. 在项目根PowerShell创建独立测试数据父目录，如 `New-Item -ItemType Directory -Force target/parity/java230/manual/saves`。执行 `./target/release/minicraft-parity.exe java230-import "完整Java世界目录" "完整manual/saves/Java230Copy目录"`。目标必须尚不存在；可选最后参数为明确选定的全局Unlocks.json。使用控制台parity工具便于看错误，不在Java原目录转换。
3. 启动 `./target/release/minicraft-rust.exe --savedir "manual数据根的完整路径"`，加载Java230Copy。仅核对显示和可操作性：玩家装备/背包/教程任务菜单、Cyan Bed颜色、DyeVat菜单、Ornate Wood纹理；走已保存楼梯访问地下，确认不是新世界出生界面。纯坐标/数字/Tile全格已有自动化，不要求人工逐格重验。
4. 在支持的样本中采一株Violet，制Purple Dye；工作台用Wood做Ornate Wood，铺坑/水后用斧拆除；使用彩色床。Save & Quit重进，采过的花不重生，Ornate Wood与床色/出生点仍在。箱内工具GUI现有限制，不把看不到其选择项误认为转换已删掉（自动化验证原始实例仍在）。
5. 在**另一份测试副本**引入不支持内容或直接用migration-instances来源导入到另一新目录，预期非零退出、无可见半成品世界。前后Get-FileHash比较Java源文件，重复导入同一成功目标应拒绝且原native状态哈希不变。

这些GUI操作本轮尚未由真人完成，不重复要求音频、按键、窗口或未修改资源包的旧验收。下一阶段优先实例级库存和敌怪/Spawner/Statue无损模型、完整World装配Oracle，再扩展可成功迁移的真实世界类别。
