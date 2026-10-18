# Java 2.3 Phase 2：自然花卉与 Survival 兼容基线

## 基线与范围

开始时工作区干净，实际 HEAD 为 `bdc960cc334baf8f5cbbf5943bd0222573a80471`，分支 `feat/java230-colored-items`，其祖先包含 `eac5b77001ca065a86c0750be47090b43386f395` 的 2.2.4 修复。已阅读两份 2.2.4 审计、Phase1 记录、既有验证记录和两套 Oracle 脚本。

本轮从该 HEAD 创建 `feat/java230-survival-flora`，依赖尚未合并 main 的 Phase1。重新查询上游 refs：2.2.X=`47227a89fe05f1e3fe962f070f25c53111254a1f`，2.3.0-dev=`4dc00332e24a727b96be7b885d5cbbdd9c85263c`，main=`42ba9192fcc229bcf615453a6504d415c777feaf`。只使用固定 dev2 源码，不使用 v2.2.4 错位 Tag 或 Infdev 规则。

没有升级 Cargo 依赖、改变 ItemId/Tile ID、替换已有 403 个资源、引入新贴图或修改旧黄金样本。新增 Java 派生数据和驱动沿用 Minicraft+ / GPL-3.0 来源说明。

## Java 原版事实与证据

以下路径均相对于上述固定提交；源码保存在忽略目录 `.reference/release-2.3.0-dev`。`scripts/parity230/run.ps1` 核对所有 client/common Java 及资源 Git blob，编译原样源码，运行时 fresh classes/resources 排在依赖 jar 前。驱动只调用/反射已有方法，没有修改游戏源代码。

| 问题 | 结论及源码证据 |
|---|---|
| 初始生成全部19种花吗 | 否。`level/LevelGen.java:425–438` 使用 `nextInt(4)*nextInt(4)+nextInt(3)`，支持集仅0..11，且单个世界不保证包含其中每一种。Seed42正常128甚至没有11号 Red Tulip。 |
| 何时、何处布置 | `Level.java:194` 初始化地图调用 LevelGen；只在地表0层的 top map 布置。每 `w*h/400` 个花簇30次附近尝试，只替换 Grass。位于沙地/树木处理之后、仙人掌/楼梯之前。五种主题共用同一花簇规则，没有新增生物群系。沙漠主题也只在残存草地长花。 |
| 自动再生/传播吗 | `tile/FlowerTile.java:61–76` 随机 tick 的1/30分支把邻接 Dirt 改成 Grass，不复制花卉。采后 Grass 不会因读档恢复原花。Rust Phase1 已包含 Flower230 的草传播分派，本轮无需重复实现。 |
| 剩余花的生存来源 | `item/WateringCanItem.java:48–91`：在 Water 补满1800；非水地块每次有水使用减1；主动浇 Grass 时1/60长花，种类 `nextInt(FlowerVariant.values().length)`，覆盖全部19种。12..18并非只有注册，已有这条真实可用来源。 |
| 是否可获得全部16染料 | 正常主题可：全部花由地表/浇水获得，Cactus来自沙地，Coal来自采矿，复色沿现有 Recipes。White Dye 唯一直接材料 White Lily=16，不在初始花簇支持集中，需浇水。 |
| 采摘行为 | `FlowerTile.hurt` 掉对应花并恢复Grass。Phase1已修正上游铲子先清data再读取的错误，继续保留该防丢物修复。 |
| 与2.2.4地形差异 | 主体噪声、沙地、树木公式保留；花data从旋转位改成物种，花簇新增一次RNG调用，后续仙人掌/楼梯及校验重试序列改变。ChunkManager成为固定尺寸存储容器；洞穴锁房/楼梯段也有源码调整，本轮不迁移。 |

初始生成实际对照：Seed42、128正常主题原始地图171株花，组装世界后166株；装饰/楼梯连接等既有组装会占用少量位置。Java原始地图与完整Rust世界组装的数量并不混称为同一对象。

### RNG 首次分歧与正常版本差异

首次随机调用分歧在花簇的 `col` 第二次 `nextInt(4)`；此前单次候选地形完全不变。Rust测试把第一候选的花还原为Grass、仙人掌还原为Sand、楼梯还原为Rock后，验证两版基础地形逐格相同。

Java真实 `createAndValidateMap` 为Seed42正常128选择第53候选：第一候选只有1楼梯，接受候选6楼梯。驱动重置原版内部 Random 后反射重放原版 `createTopMap`，找到与实际接受地图完全相同的候选，记录 `validationTrace`，没有复制Rust期望或另写条件选择地图。

因此最终两版地图不只花不同：Seed42原始行优先首差为(62,8)，Java224 WATER/0、Java230 GRASS/0，共8432格name/data不同。这是上游随机校验重试造成的版本差异；新版8个场景已逐格对照真正Java230，而非仅改Rust黄金值。旧路径的逐格/哈希对照仍独立保留。

## 实现与生成版本契约

| 位置 | 实际改动 |
|---|---|
| `level/spec.rs`、`level/generation.rs` | `GenerationVersion::{Legacy224, Java230Flora}`；旧 `surface_with_spec`/`generate_level` 入口保留Legacy；显式新入口仅改变top map花簇种类/data/RNG调用。 |
| `world/mod.rs` | 显式版本构造器、持久化字段及getter；组装世界保留 Flower230 data，避免把不同花清成0号Daisy。其余五层、结构系统不改造。 |
| `worlds.rs`、`app/mod.rs` | UI新建世界写 `Java230Flora`；seed-only恢复按元数据版本构造；存在state直接载入完整地图，不按版本重新生成。 |
| `world/tile_attack.rs` | 接通原版浇Grass的1/60、均匀19花规则，沿用已有补水/扣水、采花和掉落管线。 |
| `world/flora_tests.rs`、`worlds.rs`测试 | 实际World动作、生存材料来源、持续RNG、旧格式迁移、损坏字段及复制重命名回归。 |
| `scripts/parity230/Oracle230.java`、`minicraft-parity` | 在已有工具中扩展真实Java花卉生成/浇水Oracle；增加逐格新版地图比较与真实世界生成/存读盘/自然预览诊断。 |

兼容规则：

- 缺失生成版本的旧2.2.4/Phase1原生state和world.json一律Legacy224；原有Flower旋转位和Flower230物种不会相互解释。历史seed-only也不会升级为新版。
- 生成版本只决定新地图的生成，不冻结游戏交互。旧世界仅加载/等待不会新增花；玩家主动浇草地也能使用新版长花玩法，无需转地图。
- native format仍为2，兼容标签仍为2.2.4-rust；未改变既有字段语义。新增字段有兼容默认值，未知物品仍明确报错。
- state中的未知/null/错误类型生成版本明确报错，尝试有效.bak；无有效state时返回错误，不落回新生成。元数据无合法候选时世界不列入菜单，目录仍保留且禁止同名覆盖；有效元数据.bak可恢复条目。
- 复制/重命名保留新旧元数据和state。存档恢复读取既有地图，包括已采摘/放置花，不重新跑LevelGen。
- 不承诺Phase2存档向旧EXE降级兼容。旧EXE即使忽略新元数据，也未实现相同游戏行为；升级前保留副本。
- Java>=2.3完整世界导入继续明确拒绝，未冒充完成新版ChunkManager、Torch ID和进度转换。

### Rust 适配与尚未移植

花簇公式、概率、19种数据/物品与配方都来自Java，没有添加Rust独占花卉掉落或凭空材料来源。

Java每次浇水 `new Random()`，还把水花粒子/高斯随机与长花混用。Rust使用独立可序列化 JavaRandom，新世界由 `seed XOR 0x464C4F5241323330` 初始化；无字段的历史存档默认seed0流，之后按实际状态保存。它不消耗世界生成、AI或掉落随机流，读档续接稳定。这是确定性工程适配；Java浇水Oracle仅证明19种可达，不声称两端随机时间轨迹相等。

没有迁移浇水的Dirt/邻接Dirt额外变草与水花粒子，也没有改旧的土壤/作物系统。没有自动花卉再生、自然生成12..18种花或新群系；这些不能从注册表推断出来。完整Java230地下层/结构生成及逐帧渲染/AI对照仍为UNKNOWN，不以Java230Flora名称冒充整版LevelGen对等。

## Survival 资源链

工作台Wood10；工作台制作Furnace需Stone20，Anvil需Iron5，Dye Vat需Stone10+Iron5，Loom需Wood10+白Wool5；Anvil制作Watering Can需Iron3。Iron每个需要IronOre3+Coal1；矿石/Coal由既有采矿获得。羊可在地表Grass/Flower/Flower230自然生成，击杀掉Wool，剪刀可剪Wool1..3。没有改变动物掉落或剪刀耐久。

染缸一花转一份染料。直接来源：

| 染料 | 正常可获得来源/混合（各输入未注明时1份） |
|---|---|
| White | White Lily，主动浇草地可获得 |
| Black | Coal |
| Red | Rose / Red Tulip / Poppy |
| Green | Cactus |
| Brown | Green+Red ->2 |
| Blue | Cornflower / Iris / Lapis |
| Purple | Violet，或Blue+Red ->2 |
| Cyan | Blue+Green ->2 |
| Light Gray | Oxeye Daisy / Hydrangea / White Tulip；White+Gray ->2，或White2+Black ->3 |
| Gray | White+Black ->2 |
| Pink | Pink Tulip / Peony / Pink Lily；White+Red ->2 |
| Lime | White+Green ->2 |
| Yellow | Sunflower / Dandelion |
| Light Blue | Blue Orchid / Periwinkle；White+Blue ->2 |
| Magenta | Allium；Purple+Pink ->2，或Red2+White+Blue ->4，或Pink+Red+Blue ->4 |
| Orange | Orange Tulip；Red+Yellow ->2 |

Loom：白Wool1+对应染料1 ->彩Wool1；Wood5+同色Wool3 ->同色Bed1，或White Bed1+染料1 ->彩Bed1。White Bed使用Wood5+白Wool3。所有16色均有材料图路径；不保证出生地立即有每种花，沙漠草地稀少、Hell地表无水等是主题限制，不会为了染料擅自改地形。

自动化闭环不是给定材料的孤立枚举测试：固定Seed42、空27格背包Survival，采树/石/Coal，制作工作台/镐/熔炉，沿已生成楼梯进入-1采铁，回地表冶13 Iron，制作Anvil/Watering Can/Dye Vat；调用真实自然刷怪逻辑获得羊，真实攻击掉Wool，制作并放Loom；从自然花采Rose，染缸转Red Dye，织机转Red Wool与Red Bed，放床/夜间睡眠/出生点恢复；自然草地浇到White Lily后转White Dye，经过丢弃、拾回、死亡箱和保存恢复保持1份。

该headless流程缩略走路定位、等待拾取冷却和无关AI；不增加背包材料、不重设耐力/生命、不改资源方块或强行选择花种。它验证生产动作管线与资源链，不代替玩家GUI操作验收。全部16色的材料可达图、35/57/1完整Java配方多重集合与Phase1所有颜色实操测试共同保留；不宣称人工从零逐色试玩全部16色。

## Oracle、尺寸与持久化验证

新增两份真实fixture：`generated-flora.json`、`watering-flora.json`。旧7份230 fixture重新执行逐份语义一致，224黄金文件没有更新。

- 128正常Seed42/256：每株花的坐标/variant、所有tile/data及邻居逐格相同；多次生成确定。
- 128 Seed42五主题：完整地图hash和逐格对照通过，包括沙漠仅少量草地花、Hell熔岩主题。
- 256/512 Seed42：原始地表每格tile/data对照Java通过；完整六层Rust世界生成、容量/边界、磁盘保存恢复通过。
- 旧路径：224 Seed42六层各16384格、噪声16/32各16384 double位模式通过；Seed42/256黄金哈希、旧142物品/59方块ID、工具最后耐久、满包、死亡箱、材料扣除及原有安全加固均由完整Workspace回归保留。
- 原生兼容测试实际写磁盘：去掉版本/RNG字段的历史形状保留旧Flower data51、Phase1 Flower230 data18和Cyan Wool4；再保存、复制、重命名后仍一致。另有无备份的坏版本报错、有备份恢复而不生成新地图的测试。未访问或修改玩家真实存档。
- 128真实生成地图采花后再次调用Save & Quit使用的writer/loader，Grass与单份花掉落保留；256/512完整state往返相等。额外浇水12000次固定流测试得到19种，跨中途保存后序列/扣水/收获数量一致，通用AI/掉落RNG不变。

首次Release Smoke单次测量（非基准，不把分配量称为RSS）：

| 地图 | 六层生成ms | 初始完整世界花数 | tiles+data容量bytes | 原生state bytes |
|---|---:|---:|---:|---:|
| 128 | 87 | 166 | 294912 | 1256452 |
| 256 | 142 | 652 | 1179648 | 4516678 |
| 512 | 156 | 3296 | 4718592 | 17495865 |

实际峰值进程内存未测量；上述仅6层tile/data Vec容量，不包含实体、JSON、解码资源等。没有新增生产全图扫描阶段，花簇复杂度与Java相同；不保证其他机器/Seed时间。自然预览为 `target/parity/java230/phase2-smoke/natural-flora.png`，真实渲染器绘制既有生成地图，只选择花最密处的摄像机，不摆花或替换地形。

## 工程结果与最终HEAD门禁

实现提交 `9f9848f992034a2055385d993d451eef42520b5f`；本文件及旧世界主动浇水/磁盘迁移补充所在提交为最终HEAD。完整SHA、工作区状态和最终逐项退出码记入忽略文件 `target/parity/java230/phase2-final-validation.md`，避免把包含自身SHA的验证文件反复提交。

实现首轮严格Clippy通过；Release Workspace **573项通过，0失败**（client553、common7、server单元6、双客户端集成7），Release workspace build通过，真实EXE `--self-check`实际进程退出0；上述两套Java Oracle重跑、旧地图/噪声及新版8场景全格比较、9份230 fixture语义比较通过。局部流程曾因测试使用非实际产品名“Iron Ingot”而失败，改为已存在的菜单名“Iron”；没有为迎合测试修改游戏配方。

文档与最后补充提交后对最终HEAD再次执行以下必需门禁；实际结果、测试总数和对应日志以 `phase2-final-validation.md` 的HEAD为准，不把远程CI算作通过：

```powershell
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo test --release --workspace --locked
cargo build --release --workspace --locked
./target/release/minicraft-parity.exe java224-noise target/parity/java224/output
./target/release/minicraft-parity.exe java224-generation target/parity/java224/output
./target/release/minicraft-parity.exe java230-generation target/parity/java230/output
./target/release/minicraft-parity.exe java230-flora-smoke target/parity/java230/phase2-smoke
```

`--self-check`使用Start-Process Hidden/Wait/PassThru取得实际EXE退出码。测试渲染输出显式放根target，未改tracked渲染图片。最终测试在正常文件权限下执行，以便Windows原子替换测试真实运行。既有三平台CI定义保留；Windows本机已验证，远程CI、Linux/macOS运行仍未验证。没有push/tag/release/merge main，也未进行无限地图、载具、网络重写或依赖升级。

## Windows新增玩法验收

从仓库根启动 `./target/release/minicraft-rust.exe --savedir ./test-data/manual-java230-phase2`。此目录由玩家本次新建；原存档仅复制后操作。

1. 新建Survival、Normal/Island、128、Seed42；无需切换生成选项，新世界默认Java230Flora。探索草地花丛，徒手采Rose、Sunflower、Cornflower等，预期各自纹理/掉落，不会全变Daisy；天空/地下不应出现新增自然花簇。
2. 正常砍树采矿，工作台做Furnace/Anvil；Stone10+Iron5做Dye Vat，Iron3在Anvil做Watering Can。对水攻击补满，对草地反复浇水，花出现后采走再浇，偶见White Lily等初始地图不产的花（1/60且随机19种，不保证短时间必出）。不要用Creative或送材料命令。
3. 用Rose在Dye Vat做Red Dye；羊掉落/剪毛得到白Wool，Wood10+Wool5做Loom。Loom用Wool1+Red Dye1做红Wool，Wood5+Red Wool3做Red Bed；也可按表混色。预期材料准确扣除，不足时不能做。
4. 放床，夜间交互应睡到早晨并设出生点；拆下再放颜色保持。采掉一处自然花、记录位置并放一株已采新花，再Save & Quit关闭重进；前者仍是Grass，后者、床色和出生点保留。持一份染料丢弃拾回，再死亡/复活拾箱，预期数量守恒。
5. 用菜单复制一份旧世界再进入，检查原地标、楼梯、旧花/库存未变化，保存退出、重进、重命名后仍保持。旧副本仅加载不会长新花；主动浇草地才可能出现新版花。不要拿唯一原档做降级测试。

本轮不要求重复验收未改动的音频、按键、窗口或资源包行为。
