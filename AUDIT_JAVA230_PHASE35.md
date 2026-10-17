# Java 2.3.0-dev2 Phase 3.5：实例、实体与真实世界迁移

日期：2026-10-17，Windows x64 MSVC。本报告区分官方 Save 能表达的数据、Rust 的后续运行行为和仍未实现的对等性。**本轮不是任意 Java 世界 100% 兼容声明，也不是完整 World 生成对等声明。**

## 基线与保护范围

开始工作区干净，实际 HEAD 为 `dbf430bf0dd2f79ced9f3789c8ca35af6f822f2c`。从该点建立 `feat/java230-migration-parity`，完整继承 Phase2 `c9d2e05723ff637f1b18dcd17bc549186d88728b`、Phase1 和224修复。没有从旧 main 开始。已阅读224两份审计、230三份审计与最终验证、现有两套 Oracle 和实际调用链。

重新执行远程 `git ls-remote`：2.2.X=`47227a89fe05f1e3fe962f070f25c53111254a1f`；2.3.0-dev=`4dc00332e24a727b96be7b885d5cbbdd9c85263c`；main=`42ba9192fcc229bcf615453a6504d415c777feaf`。主要参考为固定 dev2，不把 Infdev3 当成稳定目标。下面 Java 路径均相对于该版本 `src/client/java/minicraft/`；224对照来自固定224源码。没有修改上游游戏源码。

旧142物品、既有 ItemId、Tile0–69、OrnateWood70、Legacy224 和 Java230Flora 均保留；本轮没有依赖升级、新生成版本、大地图架构变更或资源替换。原始403资源与扩展资源不改，仅接用已经存在的 filled watering can 图及其资源包路径。

## 1. 实例级物品

### 源码与数据模型

Java `item/WateringCanItem.java` 每对象独立 `content`（容量1800），`getData()` 产生 `Watering Can_content`，`Inventory`/`Save.writeInventory` 和箱子各项分别保存。官方 Load 不存在全背包共享水量。Rust 旧模型用数量堆叠加 Player.watering_content，无法无损表示731和23两把壶。

最小扩展位于 `item/inventory.rs`：`ItemStack.watering_content: Option<u16>`，非水壶必须None；水壶必须count1，已知水量0..1800。工具仍是独立 ToolItem，未改既有名字/ID/耐久表示。水壶禁止按名称合并，转移用完整stack，失败保留原实例；`add(WateringCan,n)` 只表示创建n把独立空壶。`ActiveItem::Slot {index,revision}` 精确引用某壶；删除/拆槽使旧引用失效，取消选择，不能错用移位后的另一壶。普通物品和工具原选择方式保留。

`world/item_instances.rs`、`tile_attack.rs`、`player_actions.rs`、`level_update.rs` 与 entity arena/tick 接通使用、补水、死亡箱、掉落和拾回；`hud.rs`、`inventory_ui.rs`、`assets.rs`、`entity_render.rs` 接通实际内容与图像，未知水量显示 `?`。`save.rs` 的Java库存字段写出每把壶的状态，活动引用只计一次；未知水量明确拒绝Java字段导出，不能伪造0。这里不承诺完整Java230世界导出。

### 旧原生存档迁移

新增 `Player.item_instance_version` 缺失为0，新建/迁移后为1；以后不重复从旧共享字段补水。未知未来版本拒绝，超容量旧共享值拒绝。

- 旧玩家库存恰好只有一把无实例字段的壶：唯一可归属的历史共享水量迁入这把壶一次。
- 旧玩家有多把壶，或旧箱子/死亡箱/地面壶没有独立记录：保留None（UNKNOWN），不把共享值复制到每把壶，也不当作空壶。必须实际对水补满才建立已知水量；原共享值继续保留为历史信息。
- 旧堆叠壶拆成独立槽/掉落实体，保留顺序/数量；地面实体保留年龄、精确坐标、速度、高度、寿命等全部字段，仅为新增实例分配唯一ID。有明确状态却count>1的损坏输入拒绝，拆分数量与分配有边界。
- 原生旧容量溢出的stack仍按既有规则落在玩家旁，不静默删掉。新增字段兼容旧档读取；新档没有承诺可被旧EXE降级读取。

### 真实运行证据

新增 `WateringState35.java` 在原 Oracle 中直接调用官方 WateringCanItem.interact、Inventory、Save.writeInventory 与 Load.updateInv。`watering-instances35.json` 记录731/23分别使用后730/23、730/22，只补第二把得到730/1800；满背包返回原731对象；实际库存字段和加载内容一致。裸 Farmland 消耗1水但不改data，空壶不消耗；修正了Rust原先给裸Farmland写7的差异，1/60长花随机流不改。

**上游自身的数据丢失不复刻**：实际Java Oracle证实 `WateringCan.copy()`、批量复制和 `DeathChest(Player)` 会把带水壶复制为空壶，按名字removeItems还可能移除另一把。Rust在这些路径保留完整实例。这是明确的数据保护差异，不声称复现Java缺陷。

`world/item_instance_tests.rs` 通过实际 World 动作覆盖独立使用/补水、箱子、满包、死亡拾回、丢弃拾回、磁盘恢复、旧单壶/多壶、未知水量、旧地面motion拆分及槽引用失效。真实 `migration-instances.zip` 官方Save导入后，实际取箱、使用、死亡、两次磁盘重载仍保持各壶水量。测试的标准是实例/数量守恒，不是仅序列化成功。

## 2. 实体对等性

### 普通敌怪与动物

Java参考：`entity/mob/{EnemyMob,Mob,MobAi,Zombie,Slime,Skeleton,Creeper,Snake,Knight,PassiveMob,Cow,Pig,Sheep}.java`；官方 `Save.writeEntity` 保存当前HP和Enemy lvl，羊保存cut；不保存AI随机数、走动计数、速度临时变化等。

| 状态/行为 | 本轮实现与边界 |
|---|---|
| 普通六类敌怪等级 | Mob新增serde兼容level=1、difficulty=1、score_mode=false；合法1..4，进入构造、伤害、掉落、得分和渲染，不能读取后丢弃 |
| 生命 | base×lvl²×2^difficulty；高等级Snake base8、低等级7；Boss固定2000/5000、只支持level1。导入HP必须1..max，原HP不截断 |
| 动物 | PassiveMob=5+factor×difficulty，Cow factor5、Pig/Sheep factor3；真实正常世界Cow10触发旧Rust5上限拒绝，已修正新Java实例构造 |
| 伤害与攻击 | 按源species/lvl/difficulty接触伤害；Skeleton箭伤lvl、间隔500/(lvl+5)，Creeper伤害/爆炸范围乘lvl；等级进入真实更新 |
| 掉落与得分 | 六类普通掉落表及key概率按实际源码；普通得分50×lvl，Boss/动物独立。源RNG不持久化，不承诺载入后和Java每次随机抽样同步 |
| 旧原生实体 | 既有max_health原样保留，不能把旧Cow5或旧Zombie5自动升成新模型；旧serde字段缺失有默认值 |
| Java224 | 同源公式已核实，精确版本2.2.4的Cow/Pig/Sheep导入也修正HP截断；早期Java路径不扩大猜测，旧原生构造不改 |

`EntityState35.java` 的 `entity-state35.json` 包含6种敌怪×4等级×3难度共72条实际构造、接触伤害、同种子掉落；另有9条实际动物样本。Rust比较实际值，不用自己的输出当Oracle。`entity/migration_tests35.rs` 验证hurt窗口、等级/HP存读、死亡掉落、异常等级和不重算旧max。自然生成的新实例和Spawner也进入新属性模型。

**尚未完成**：所有species逐tick移动、速度/寻路/碰撞、Boss攻击轨迹与Java完整同步仍UNKNOWN/存在既有近似；不能把属性Oracle称作完整敌怪AI对等。Java Save无法表达的瞬时AI/RNG不声称无损续接。

### Spawner

Java `entity/furniture/Spawner.java`：Save保存species/lvl/坐标/层，不保存100HP、spawnTick或rnd；加载按构造重新初始化。新增 `SpawnerState {level,spawn_ticks}`，导入合法配置，原生后续保存实际timer和HP。Java未保存timer不能虚构源剩余时间；Rust使用明确的重启种子初始化200..500倒计时。

真实tick执行人口压力、上限、128像素玩家范围、地表白天/照明、周围合法方块过滤和对应等级刷怪。任何工具可伤害，pickaxe加成/Haste/Creative即时破坏、500分；按官方方法不扣工具耐久、不产生Survival掉落。资源沿用既有图像与实体调度。Oracle七个场景覆盖地下、白天、夜晚、火把、远玩家、满人口、全阻挡，实际官方工具伤害也比较。

限制明确：Java独立timer RNG及效果粒子不保证逐次位置随机流对齐；Creative空手循环等级尚未接通。高等级Spawner尚没有可表达等级的家具物品实例，PowerGlove拾起会丢配置，因此**拒绝该拾起且保持实体**；level1仍可通过既有物品放置。不能把此限制说成完整Creative搬运对等。Boss Spawner不在声明范围，导入明确拒绝。

### KnightStatue

Java `KnightStatue.java` 保存bossHealth，touches不保存；源加载从0次触摸开始。Rust导入1..5000，三次真实交互产生该HP的ObsidianKnight；1731HP构造phase0，实际首tick进入phase1，玩家缺席恢复同HP雕像；阻止同层重复Boss。Rust原生保存自身tap/HP/phase，不在每次重载重置。

`migration-entities35.zip` 真正通过官方Save写出1731雕像；额外 World 集成回归从导入后取出源箱内Pickaxe11、攻击源Zombie3/90HP、使用源SheepSpawner刷怪/受工具伤害、敲雕像两次→磁盘恢复→第三次1731HP Boss→实际tick→再保存恢复。源Entities字节始终未改。

剩余：Java ObsidianKnight构造会重建Boss房间，Rust未完整复刻；切离层后的Boss回雕像受现有非活动层调度影响，不能承诺离层瞬间与Java同步。完整Boss阶段攻击仍非全对等。

## 3. 迁移覆盖率与事务保护

| 来源世界/Fixture | 已验证结果 | 运行验证与范围 |
|---|---|---|
| Phase3 base/colors/score60 | 继续成功 | 原地图/Player/工具/花/床/染缸/OrnateWood/进度/跨层/磁盘回归继续保留 |
| migration-instances | 由拒绝变为无损支持 | 官方731/23水壶、Zombie3；独立使用、取箱、死亡、保存恢复 |
| migration-entities35 | 支持 | 18个六species/lvl1,3,4伤血实例、原Zombie3、三种Spawner、Statue1731及原彩色内容/容器；World战斗/刷怪/触发/磁盘恢复 |
| normal-survival-42 | 支持 | 官方World.initWorld直接Save，无删改实体/地形；全部六层、7Spawner、Statue5000、17DungeonChest、4普通箱、AW及家具 |
| normal-runtime-42 | 支持 | 上述正常世界实际Level.tick/跨楼梯后Save；115普通敌怪、12动物及全部原Boss/结构/容器；全部持久字段、实际native tick和磁盘恢复 |
| 未知实体/非法实例/版本或层数错误 | 明确拒绝 | 不删实体、不降成level1、不替换未知方块、不伪造水量；无可见半成品 |
| 其它未覆盖Java世界 | 未保证 | 仍受严格版本/字段/范围/容量门禁；不把两份正常世界推广为任意世界兼容 |

`WorldAssembly35.java` 用固定源码真实 `World.initWorld`、正常Settings/seed输入、官方 `new Save`。无图形环境下反射只填写UI本来填写的seed小部件，离线Analytics传输只防联网，不替换游戏生成。初始世界保留所有生成器、Boss、宝箱/loot。随后实际走已生成楼梯并调用World.changeLevel，地表到-4每层60次官方Level.tick，共300次；InputHandler仅支持官方进度保存，不伪造键盘、不删除不支持实体。这是**headless脚本遍历**，不是GUI真人游玩。runtime使用官方未持久化的new Random，因此记录的快照逐项比较，不伪称跨JVM敌怪坐标确定。

重要有效源边界：Java `Level.checkChestCount` 混用20/23和21/24像素因子，在128图生成了 `DungeonChest[1224:2194:...:true:5]`，超出地图而有真实库存。导入只为-4层DungeonChest接受由该算法界定的有限坐标包络，**保留原坐标和全部loot，不移动/删除箱子**；其它实体仍严格地图内。native tick/渲染/碰撞的边界检查和该实际箱子的磁盘恢复均验证。其它Seed中generateSpawnerDungeon的极端偏移未证实，不扩张成无上限允许，仍可能准确拒绝。

事务式 `saveload/migration.rs` 未放宽：源只读、精确dev2、32文件/单16MiB/总128MiB、无链接/子目录/路径穿越/用户ZIP；目标任何已存在路径拒绝；独占stage→转换完整验证→原子不覆盖发布。成功/失败前后原源字节比较，重复目标、中断/竞争发布/损坏备份门禁保留。新增9组实例负面变异包括容器1801水量、零/超HP、无效Spawner等级/Boss配置/额外字段、Statue0/5001、非箱实体越界，均整档拒绝且无残留世界。

保存全世界时校验所有玩家、箱子、死亡箱、地面stack与实体状态；反序列化后也验证，不能把不合法实例留待运行时隐式修正。新档不承诺旧EXE可读取。

## 4. 完整World与历史生成版本

原始LevelGen黄金保留：224 Seed42逐格、Seed256哈希/噪声double位模式；230原有地表八场景；230四种尺寸/seed组合的24张原始六层图。这些不等于World.initWorld。

新增实际完整World Oracle包含Random(worldSeed).nextLong每层seed、raw hash、装配后Tile/data/实体/loot、全部楼梯连接。固定128 Seed42两次独立JVM初始Save每一文件逐字节一致；17对下楼梯均在子层同坐标接上楼梯。

| depth | 官方层seed | raw→World装配改变格数 | 完整JavaWorld与历史Rust Java230Flora不同格数/首次坐标 |
|---|---|---|---|
| -3 | -6169532649852302182 | 28 | 3178 / (64,2) |
| -2 | 5111195811822994797 | 34 | 3439 / (64,2) |
| -1 | 5694868678511409995 | 376 | 4425 / (64,2) |
| 0 | -5843495416241995736 | 110 | 8937 / (85,7) |
| 1 | -5025562857975149833 | 35 | 4908 / (29,11) |
| -4 | -1782466964123969572 | 530 | 9470 / (5,4) |

来源 `World.java:139–148` 派生seed，`Level.java:201–245` 结构/父层楼梯/天空房屋/补箱/Boss，`Structure.java` Boss房间与地牢模板；实际全部实体和地图差异见忽略日志 `phase35-world-assembly-differences.json`。这些是确认的差异，未修改测试预期掩盖。Java初始chestCount14但17箱全部锁定是上游运行计数缺陷；官方Load重新逐一数锁箱为17，不混同初始total。

本轮不新增未经完整验证的生成版本，不改Legacy224/Java230Flora任何历史语义或Seed-only结果。完整state/备份优先；导入state不可恢复时拒绝，不重新生成。完整World装配、宝箱模板/战利品/初始Boss布局需要独立版本化工作，留待后续。

## 5. 相关玩法与实际修复

箱内工具GUI已补齐：两面行数包括stack+tools，显示耐久，移动精确实例；原容器工具虽然在磁盘但不可选问题解决。高HP Boss头顶百分比原用u16饱和乘100，满血2000会显示32%，与实际生命不符；现改u32中间值，显示100%，不修改生命本身。真实World.tick的craft开箱→方向选第二把Pickaxe11→select放入→full bag取回拒绝→扩容取回→exit→存读，确保另一把17未变、活动引用正确清理、水壶不变。普通容量/最后耐久/材料扣除测试继续保留。

最后地牢锁箱5 GoldApple已接入菜单和旧交互两个入口的共享解锁逻辑：实际消耗钥匙并首次从锁定变为解锁，且本层无剩余锁箱时，地面掉落五个独立苹果；无钥匙/非最后箱/重复打开不额外奖励。官方 `Save.writeEntity → Load.loadEntity → DungeonChest.use` 五步Oracle保存在 `dungeon-chest-reward35.json`，并用其真实实体字符串做受控导入菜单与每步磁盘恢复回归。这不是对正常世界样本删数据或修改其初始chestCount14；原始完整世界与runtime样本保持原样。按照官方Load从锁态恢复计数，不复刻新建世界漏计counter缺陷。

正常世界实际tick发现零损伤Wall原代码 `(data>0).then_some(data-1)` 会先求值，debug下溢panic；Java WallTile.java:127–133有显式damage>0。已改checked_sub，0/1/u16::MAX回归，未修改源世界或降低地图检查。

彩色羊生成/染色/剪毛/死亡掉落本轮未移植；Java本身Save不保存羊色，导入不能凭空恢复历史颜色。标牌多行编辑、完整AI和Boss房间差异继续记录，不借扩展兼容率而忽略有效源字段。

## 6. 工程、回归与性能

只使用已有target。Java两个run.ps1都重新核验固定源码/资源Git blob并编译，dependency jar只供第三方类；230正常World独立JVM避免WorldSelectDisplay静态worldsDir被先前Fixture污染。开发时遇到该驱动静态缓存问题和未初始化InputHandler导致官方进度保存失败，修正驱动后重放；没有改固定源逻辑。记录的确定性JSON和所有migration官方Save文件与真实重放比较，runtime只与自己的实际source快照比较。

局部已实际执行：水壶World回归11项（含最后复核增加的快捷交互）、普通属性/Spawner/Statue运行测试、真实World combat一项、完整normal+runtime迁移一项、assembly三项、wall三项、旧230局部38项及224动物回归；不以局测代替最终Workspace。最终复核还补齐了ActiveItem::Slot水壶快捷箱子交互，否则会被原Stack-only分支误当空手；新增真实Input回归保证转移所选23壶，满箱时731壶不变。真实失败及其修复证据保留：旧Cow上限拒绝、官方越界地牢箱拒绝、零损伤墙下溢；新增runtime测试曾误以为玩家携带工具，改为实际取出官方箱内工具11再测试，未添加虚构装备。

最终全部代码/文档提交后的HEAD执行：

```powershell
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --locked -- -D warnings
$env:RUST_TEST_THREADS = "4"
cargo test --release --workspace --locked
cargo build --release --workspace --locked
```

另执行Release EXE --self-check、224地图/噪声、230地表/六层/Phase1配方/Phase2水花及Survival、Phase3导入安全回归、新实例/实体/真实世界磁盘恢复、128/256/512生成与磁盘Smoke。**本轮测试目录共623项（client603/common7/server6/双客户端e2e7）；实际最终通过/失败、退出码、耗时和完整SHA集中在 `target/parity/java230/phase35-final-validation.md`，交付回复给出统计；本节列命令不冒充尚未执行的通过结果。**

15秒测试未删除、未改阈值/计算断言：`registry_tests::save_resume_is_deterministic_and_two_day_soak_stays_bounded` 用Instant包住完整两日CPU模拟，无音频/渲染/磁盘。Phase3默认并行失败、单独7.85秒及4线程全量通过，说明墙钟预算与地图/迁移CPU测试争用相关。CI设置RUST_TEST_THREADS=4降低争用，仍完整运行两日。Phase3.5默认并行的补充实测结果也单独记录，不隐瞒失败，不把4线程结果当默认稳定证据。

实际统一门禁已通过：Release Workspace **623项（603+7+6+7），0失败、0忽略**，格式、严格Clippy、Release构建与EXE自检通过；旧224地图/噪声、230地表八场景及24张raw全层对照通过。默认并行的额外一次全量也通过（11.20秒，client8.61秒），不代表未来任意CPU负载恒稳，CI仍保留4线程。原两日15秒断言原样保留。128/256/512实际生成单次86/142/155毫秒、花166/652/3296，均磁盘恢复成功；不是统计性能基准或RSS测量。

实际Release CLI对instances、entities35、正常初始及正常runtime四份官方世界均成功；对每个源mob的species/位置/HP/lvl及独立水壶再次比较，成功前后源SHA256不变，重复目标拒绝且目标哈希不变。1801非法水量CLI非零失败、源不变、无可见半成品。官方重放与24份确定性JSON/Save样本比较通过；非确定runtime仅与自己的真实snapshot比较。最终文档/百分比修正提交后继续在最终HEAD重跑四项门禁，精确结果在忽略记录，不把前一HEAD的结果冒称最终。

Windows是唯一实际运行平台。Linux/macOS **NOT RUN**；三平台GitHub Actions **NOT RUN**，工作流存在不代表远程验证。无图形headless动作不等于真人GUI，未重复要求音频/窗口/按键/资源包旧验收。

## 7. Git与Windows人工验收

分支 `feat/java230-migration-parity`，父基线 `dbf430bf0dd2f79ced9f3789c8ca35af6f822f2c`。功能检查点 `d0b0a40d9d61a1743f403fa72f273b3622b39ac1`；本轮本地提交、最终完整SHA与干净状态见最终门禁记录/交付；不为在本文记录自身SHA循环提交。无Push、Tag、Release、merge main、历史重写或玩家真实存档改动。

1. 优先用已生成官方样本 `target/parity/java230/output/migration/saves/migration-entities35`（实际目录见输出）或固定dev2游戏的**完整副本**，关闭Java后保留源。不能用Infdev档，不在原世界目录转换。
2. 创建独立测试根 `target/parity/java230/manual35/saves`。运行 `target/release/minicraft-parity.exe java230-import "源目录绝对路径" "manual35/saves/JavaCopy绝对路径"`；目标必须不存在。全局进度可加明确的Unlocks.json路径。不把fixture ZIP直接传CLI。
3. 运行 `target/release/minicraft-rust.exe --savedir "manual35绝对路径"`。样本地表源箱在(168,136)含23水壶与Pickaxe11，玩家库存含731水壶。分别装备、浇水和补水，另壶数值不变；用容器菜单转工具/水壶并取回，检查耐久/水量显示。
4. 在测试副本检查等级敌怪颜色/战斗、靠近Spawner观察刷怪，用工具破坏（Survival不掉Spawner）；地牢Statue连续三次交互，Boss出现。样本Statue源生命1731，普通新建世界Statue5000。不要要求逐格人工重验数值Oracle。高级Spawner暂不能PowerGlove搬运，预期保持原实体。
5. Save & Quit再加载，确认水壶独立状态、容器工具、Boss/雕像和进度延续；用另一旧Rust世界**副本**继续游玩，旧多壶UNKNOWN须实际补水，不能期待虚构历史水量。
6. 对同一目标重复导入应拒绝；用损坏/不支持输入到另一新目标也应拒绝。前后Get-FileHash比较源Java文件与已存在目标，确认都不变、菜单无半成品。

本轮自动化覆盖以上有效源数据与动作，但颜色/菜单可读性、Spawner视觉和Boss触发显示仍需Windows真人验收。完整AI、World装配、彩色羊与高级Spawner物品实例仍是明确后续边界。
