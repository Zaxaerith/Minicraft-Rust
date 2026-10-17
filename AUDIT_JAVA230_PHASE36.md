# Java 2.3.0-dev2 Phase 3.6：完整固定世界组装

日期：2026-10-17。平台：Windows x64 MSVC，Rust/Cargo1.99，Microsoft OpenJDK17.0.20.1。此次重点是 Rust **自己生成**世界（B路径）；已有 Java 存档迁移（A路径）是单独回归，不能替代生成对照。下述 PASS 均限于实际矩阵和被比较字段，不是所有主题、Seed、AI或GUI的全对等声明。

## 一、基线与真实 Java 调用链

开始工作区干净、HEAD确为 `6facdba9f31e02b24ceaa0672374eab2fae4f65f`，从该点创建 `feat/java230-world-assembly`。完整继承224、Phase1/2/3/3.5，没有从旧main开始。已阅读Phase2/3/3.5审计、Phase3.5最终验证及现有Oracle/保存与世界代码。重新读取远程引用：2.2.X=`47227a89fe05f1e3fe962f070f25c53111254a1f`，2.3.0-dev=`4dc00332e24a727b96be7b885d5cbbdd9c85263c`，main=`42ba9192fcc229bcf615453a6504d415c777feaf`；固定主要目标仍为dev2，未移植Infdev。

Java路径相对于固定dev2的 `src/client/java/minicraft/`：

- `core/World.java:107–157`：reset→Random(worldSeed)→按depth1,0,-1,-2,-3,-4逐层 `new Level(random.nextLong(),parent,makeWorld=true)`→玩家findStartPos。不是先创建六张同Seed地图再集中放结构。
- `level/Level.java:177–247`：原始 `LevelGen.createAndValidateMap`（含原版校验重试）→负深度Spawner结构/地表村庄/-4地牢模板→父层Stairs Down对应连接（-4同时画入口与Boss房）或天空房→checkChestCount(false)→checkAirWizard。
- `level/Level.java:297–346,969–1190`：补箱、Spawner、村庄、地牢模板。`level/Structure.java`：ASCII模板及转置坐标、家具。`entity/furniture/Chest.java`及原始 `resources/data/chestloot/*`：战利品抽样。
- `entity/mob/Player.java` findStartPos：独立Random(worldSeed)，按实际grass候选顺序选出生位置及原版后备路径。

### 分阶段 Oracle

新增 `scripts/parity230/WorldProbe36.java` 与 `WorldStages36.java`，复用原 `run.ps1` 编译全部固定源码与资源，jar仅提供第三方依赖。只在原Level构造器实际方法边界插入只读观察回调，用JDK17 ASM；读取原Random内部48位state、Tile/data、官方Save.writeEntity的全部持久实体和库存。**不改参考源码、不替换RNG、不增减原算法随机抽样，也不使用Rust输出作期望。**

每世界30个实际观察点：sky4、surface5、-1/-2/-3各5、-4为6。raw是原始LevelGen返回结果；其内部每次重试的RNG并未单独探针化，不能将构造器Random的匹配说成观测了全部内部随机调用。层构造器Random和LevelGen Random分别初始化；Spawner瞬时timer也不能消耗层组装随机流。

新增六个压缩fixture共约1.9MiB：每包30张原始二进制Tile/data快照、阶段JSON和4份官方初始Save字段文件。完整源码、地图、官方存档和类文件仍在忽略目录。重新运行后六包**210个条目内容逐字节复现**。Seed42观察版官方Save每个文件还与新运行的**无探针**WorldAssembly35及原Phase3.5 normal-survival-42 fixture逐字节相同，验证探针透明性。正常runtime fixture中未持久化AI随机源不可跨JVM固定，没有更新旧快照伪称一致。

执行 `scripts/parity230/run-world36.ps1` 可重现全部既有230 Oracle与新六世界；224脚本也重新执行。两个源校验器共享 `scripts/parity224/verify-source.ps1`：两次Git调用批量读取固定树和原始blob哈希，仍逐个检查722/614份源码及资源；修改字节的独立负例确实拒绝。解决逐文件启动上千进程的无输出耗时，未减弱固定源码校验。

### 层Seed与量化结果

Seed42的创建顺序及完整有符号Seed：

| Depth | Layer Seed |
|---|---:|
|1|-5025562857975149833|
|0|-5843495416241995736|
|-1|5694868678511409995|
|-2|5111195811822994797|
|-3|-6169532649852302182|
|-4|-1782466964123969572|

`level/noise/mod.rs::next_i64` 按Java有符号两次next(32)实现，包含低32位符号扩展；负世界Seed和0也有实际对照。原始六层LevelGen旧路径不改，新增版本只复用已验证raw规则，然后按层Seed构造。

|128 Seed42 depth|历史Java230Flora与Java完整World不同格数|新Java230World不同格数|
|---|---:|---:|
|-3|3178|0|
|-2|3439|0|
|-1|4425|0|
|0|8937|0|
|1|4908|0|
|-4|9470|0|

历史差异测试 `assembly35_tests` 继续保留历史版本及原样本；没有把其非零差异改写成新规则或替换黄金值。新 `assembly36_tests` 比较180个阶段的**每格Tile/data、原构造器RNG48、构造器箱计数、全部持久实体/库存**，均0差异；差异时报告depth/stage/第一个坐标或对象，非只比hash。

以Seed42举例，raw后的实际Tile/data修改数（各阶段相对前阶段）：sky房35；地表村庄92/楼梯18；-1 Spawner350/楼梯26；-2楼梯34；-3楼梯28；-4 Spawner143/地牢模板285/连接及Boss房101/补箱1。首次历史偏离源于各层Seed；后续则是上述真实组装步骤，而非随机改几株花。所有180观察点和RNG值明细在 `target/parity/java230/phase36-stage-report.md`。

## 二、版本与兼容规则

- 新serde名 **`Java230World`**：`level/spec.rs` 定义，`world/assembly36.rs` 独立实现完整固定dev2组装。
- **Legacy224、Java230Flora不改名、不重解释**；缺标记的历史存档仍默认Legacy224，历史Seed-only仍使用对应旧构造器。
- `worlds::create_named` 在最低六世界矩阵通过后明确改为新建默认Java230World；`app/mod.rs`原入口已将record版本传给构造器，不需要新增内部字段UI。内部无版本参数的历史构造器仍保留Legacy224。
- state.json存在且可加载时优先已有地图、实体与版本；未知/损坏版本报错或按原bak恢复，绝不降成最新版本重生成。复制、改名、原子备份路径继续保留标记。
- 新字段仅新增枚举选项、Mob/SpawnIntent的serde默认羊色，未改原生格式2及历史标记、ItemId、Tile0–69/OrnateWood70或库存实例语义。**新存档不保证旧EXE降级读取**（未知枚举应拒绝）。
- 新矩阵真实磁盘writer/loader验证128/256/512全部地图、实体、进度、库存；实际采花后保存不会长回。Seed42实际调度楼梯从天空到-4往返，并执行世界复制/改名/未知版本主档+bak恢复/无bak拒绝。旧版本兼容测试、Seed-only和旧完整state仍全部通过。

## 三、实际实现与有意差异

### 楼梯、结构、宝箱与Boss

`world/assembly36.rs` 从源算法复现天空转置非方形房、GoldLantern/Enchanter、AirWizard中心位置；Spawner种类、等级及扫描顺序；洞穴侧室；村庄房型/覆盖层/箱位置；-4模板选择与ceil(sqrt(size))尝试次数；所有父子楼梯、入口IronLantern与Boss房/Statue，以及新增补箱。没有复制某Seed的最后结构坐标，也没有删掉不一致实体。

原始loot表只读复用已有资源，抽样顺序和工具各次独立roll、实际耐久、普通stack数量/槽序全部比较。真实Java dev2 `WoolItem.copy()` 把表中的Wool12变成实际count1，首次箱内容差异定位到该源调用；只在**新Java世界初始loot**复现官方实际1，绝不改已有库存/转移/导入数量。工具并未误合并；原142物品和旧工具耐久门禁继续通过。

**原版越界箱缺陷没有掩盖**：`Level.checkChestCount` 把tile坐标乘20/23与21/24，128 Seed42产生(1224,2194)箱，256 Seed42有5个越界箱，512 Seed42有4个且含重复坐标。新版本保留实际位置、独立实体和全部loot，不把它们移入地图冒充原样对等；各样本分别为1/1/1/1/5/4个越界箱。它们对玩家可能不可达，属于真实上游玩法缺陷，尚未设计兼顾精确生成与可达性的独立修复版本。没有无界放宽导入坐标；迁移器已有限定包络的既有保护不改。

Java构造器chestCount还可能漏计模板箱（Seed42计14而实际17）。Oracle诊断原计数完全匹配；Rust运行时仍按实际未解锁箱计数，保留Phase3.5钥匙门禁和**最后锁箱5 Gold Apple一次性奖励**，不恢复旧漏计Bug。真实奖励Oracle与原回归继续通过；不能为不可达的原版箱承诺正常步行可完成全部解锁。

初始Boss集合匹配：AirWizard1、KnightStatue1；**没有把尚未触发的ObsidianKnight假造为初始实体**。`behavior36_tests` 通过World真实三次交互（第二次后保存重进），以Java实际81格房间重建、1731 Boss HP和玩家移除后的Statue1731为对照。房内箱Wood7及实体不删除、离层调度与返回不重建、原生恢复不重复Boss。死亡时不再把已移除玩家坐标传给生成/实体调度，防止Boss继续把死玩家当目标、死亡箱立刻自动拾回。

**有意的房间保护差异**：仅新Java230World显式激活时修复原BossWall/Floor/Door及data；玩家放的非竞技场方块和全部实体/库存保留。Java会无条件覆盖整9×9区域；Rust不毁玩家作品。已有世界加载、换层、普通保存不触发房间重建；旧生成版本/Java导入世界保留既有激活路径。此保护不是“玩家改造后房间逐格与Java相同”的声明。Boss完整战斗AI与每tick攻击轨迹仍不全对等。

### 彩色羊与资源

固定源码 `level/Level.java:669–682` 实际自然色：白80%、黑/灰/浅灰各5%、棕3%、粉2%；`Sheep.java` 实际染料交互、剪毛1..3、再生、按难度死亡羊毛/牛肉均接通真实运行。任何16色染料可染，实际扣1（含Creative，按Java）；剪刀最后耐久被移除，已剪羊死亡不重复出羊毛，再生保留颜色。

Mob/待生成实体新增 `sheep_color`；旧字段缺失默认白15，非法颜色/非羊携带色明确拒绝。独立原生保存颜色与cut；Java官方Save不保存色，导入按其官方Load构造白羊，绝不捏造历史颜色，也不声称导出Java后能够保色。活羊存读、掉落保存、真实待生成提交都测试。

`Behavior36.java` 实际调用16色×3难度共48组Java方法，以固定实体Random1234比较染料扣除、剪毛、未剪/已剪死亡掉落；另观察原Level.trySpawn实际颜色roll（只读RNG逆变换，不替换原随机源）。真实Rust渲染器16色像素检查发现旧羊图羊毛为灰，不可只凭状态字段宣称完成；新增**455字节原始dev2 sheep.png**到expansion230，manifest记源版本/路径/SHA256，GPL-3.0和原署名保留。新增图逐字节等于源，旧403资源无改动；扩展图总108，原资源包标准sheep键及覆盖优先级保持。

高级Spawner家具物品搬运**未完成**：已有实例stack仅承载水壶，家具物品无species+level配置；将等级接到所有背包/容器/掉落/PowerGlove/place/export仍需要通用家具实例协议及完整迁移。保留高等级拾起拒绝和原实体，不能降成level1。多行标牌、新Boss AI、其它GUI收尾本轮未扩展。

## 四、实际验收矩阵与证据边界

|Size/Seed|六层/30阶段Tile/data与RNG|全部初始持久实体|总宝箱/Spawner|相连楼梯对|磁盘恢复|
|---|---|---:|---|---:|---|
|128/42|PASS零差异|33|21/7|17|PASS|
|128/256|PASS零差异|40|27/8|17|PASS|
|128/0|PASS零差异|49|31/13|17|PASS|
|128/-1|PASS零差异|43|28/10|17|PASS|
|256/42|PASS零差异|83|54/24|33|PASS|
|512/42|PASS零差异|152|107/40|65|PASS|

总宝箱包含普通Chest和DungeonChest。对象按语义集合比较，保留位置、层、锁定、每个槽/工具/实例；内部EntityId无需相同。独立Java/Rust Tile映射处理Flower2→native69、Ornate59→native70、新Wool60..69→native59..68、旧0..58；不直接拿Java整数当Rust ID。

Player15字段、Game除明确保留的旧Java导出版本stamp以外的初始字段、出生点、空库存/配方、初始完成计数和官方advancements仅Version都已比较。不是教程第一次GUI帧的所有通知时序证明。Java没有保存的临时AI/RNG、Spawner倒计时/粒子、逐帧渲染和非持久treeTypes显示未逐项纳入本矩阵，保持UNKNOWN；不能由持久集合一致推导其相同。原raw八地表/24全层及224Seed42/256和噪声位模式继续独立验证。完整World目前只测Normal/Island/Survival/Normal难度这六场景，未穷举其它主题/模式/Seed。

性能只做有限样本：六世界完整180阶段比较+磁盘测试曾总2.91s；最终Workspace包含全部新大图回归、15秒两日模拟原断言继续通过。没有重复目标、长压力测试或进程RSS测量声明；512最大固定数组边界与存读已实际执行。Rust新代码仅用现有std/serde/JavaRandom，无新平台依赖或Cargo依赖升级。

## 五、工程验证与失败记录

局部执行：behavior36_tests（4）、worlds::tests（5）、snapshot兼容（1）、assembly36_tests（6）；fmt和严格Clippy通过。功能HEAD `74458f6b1c5e10d1b792f330d0c92dc10306289d` 的完整四线程Release Workspace：**633 PASS = client613 + common7 + server6 +双客户端e2e7**，0失败、0忽略；client用时14.89s，编译25.45s。最终文档提交后再按用户要求对最终HEAD执行门禁，**以忽略的 `target/parity/java230/phase36-final-validation.md` 中完整SHA、退出码、时长、日志为准**，避免为了自引用SHA循环提交。

实际Java：`scripts/parity224/run.ps1`；`scripts/parity230/run-world36.ps1`（含run.ps1）。后者所有六个官方World、所有旧物品/配方/花卉/迁移源例通过。19个确定性JSON、5组官方Save的每文件字节、真实normal-survival世界及5苹果奖励重新核对通过；正常runtime随机快照不更新、不做跨JVM轨迹断言。新六世界210条目重放通过；扩展108图的所有字节/SHA256通过。旧Cargo.lock、403资源、原ItemId/Tile映射及迁移核心均未改。

中间失败如实保留：

- 新比较器曾将合法bare Book/家具getData和Java Tnt别名误当带_count名称；按源码补充语义解析，不过滤对象或改golden。
- 新磁盘比较曾因ProgressState的HashSet顺序失败；只规范原8个真正无序set，其余地图、库存/实体数组和字段保留严格值比较，简短错误改报首差异。
- 彩羊实际渲染未出现目标色，定位到旧羊图灰羊毛；接入原始dev2扩展图后16色像素均通过。
- 曾把保存前校验扩大到全部地图尺寸，破坏已有“序列化损坏快照→读取拒绝”的负例接口；收窄为新增待生成羊色的读写校验，原测试断言和原接口保留，完整回归重新通过。
- Clippy items_after_test_module失败：移动新函数到测试模块前，无lint豁免。
- 初次四线程全量613客户端有612通过/1旧快照失败（如上）；修复后633全Workspace通过。两日模拟未删除、未调15秒预算；**本轮默认并行额外全量NOT RUN**，四线程是既有CI设置，不能谎称默认并行也验证。

测试曾写入两张已跟踪旧预览，已仅恢复本轮生成的这两个文件，最终设置MINICRAFT_PARITY_RENDER_OUTPUT到现有忽略target目录。必要失败证据留target；未大范围格式化或修改旧黄金文件。

最终命令清单（全部以最终日志实际exit为准）：

```powershell
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --locked -- -D warnings
$env:RUST_TEST_THREADS = '4'
$env:MINICRAFT_PARITY_RENDER_OUTPUT = "$PWD/target/parity/render-frame/rust"
cargo test --release --workspace --locked
cargo build --release --workspace --locked
./target/release/minicraft-rust.exe --self-check --savedir target/parity/java230/phase36-selfcheck
./target/release/minicraft-parity.exe java224-generation target/parity/java224/output
./target/release/minicraft-parity.exe java224-noise target/parity/java224/output
./target/release/minicraft-parity.exe java230-generation target/parity/java230/output
./target/release/minicraft-parity.exe java230-depths target/parity/java230/output
cargo test --release --locked -p minicraft-client --lib assembly36_tests
```

GitHub Actions三平台定义已检查，四线程和Windows SDK rc.exe/SDL条件配置保留。**Linux/macOS实际构建、远程CI、Phase3.6真人GUI：NOT RUN**。不把工作流存在/用户上一阶段试玩当成本轮通过。GUI只需下面新增场景。

## 六、Git交付

分支：`feat/java230-world-assembly`。本轮功能提交：

1. `402bb47c152ae9e04e83401ba7957f001308ae09` — 独立Java230World算法、源阶段Oracle、六世界/磁盘/版本矩阵和新建默认。
2. `74458f6b1c5e10d1b792f330d0c92dc10306289d` — 彩色羊/原版羊图、Boss显式激活与死亡边界、真实行为Oracle及批量固定源码校验。
3. 本审计记录的文档提交；最终完整HEAD和全部提交列表写入忽略的最终验证记录。

最终工作区状态以门禁日志和最终答复复核。没有Push、Tag、Release、合并main或重写历史，没有修改玩家实际存档及Java参考源码。只提交必要源码、压缩源样本、三份小行为JSON和455字节羊图。

## 七、Windows真人验收（只检查新增行为）

1. 运行 `target/release/minicraft-rust.exe`，新建独立Survival，普通主题/Island、128、Seed42；不需要修改JSON，正常创建入口默认Java230World。出生地开始采花、正常升级装备，探索楼梯，向下到-4并返回；天空入口、天空房/灯/Enchanter及AirWizard应存在。数值坐标已由Oracle验证，无需逐格人工核对。
2. 地下Spawner应有不同怪物/等级，房间和地牢入口不消失。用钥匙开可达锁箱，数量不足不应解锁；物品可正常取出。最后锁箱5苹果及只奖励一次已自动测试，无需人工找遍地图；上述原版越界箱可能不可达，不能以“找不到最后一箱”推断保存丢失。
3. 在副本触碰KnightStatue两次→Save & Quit→重进→第三次触发；Boss血条应出现，房间只在显式激活修复，房内自放非Boss方块/箱内容仍在。离层返回、死亡、保存重进不应重复Boss或清空箱子。
4. 对自然羊用两种染料分别染色，确认实际颜色和各扣1；剪毛应掉对应颜色羊毛，羊吃草再生仍同色。将一把剪刀用尽，不能继续免费剪。保存重进，羊色/已剪状态、所得羊毛保持。
5. 新世界随意采一片花、改造地牢外方块、开一箱，Save & Quit重进；地形不回滚、loot不刷新、结构/实体不重叠新增。另用Phase2/3/3.5**副本**加载保存，原地形和既有羊/水壶/Spawner状态保持；不要手动修改其生成版本。
6. 高级Spawner搬运未实现，PowerGlove应保持拒绝并保留原实体/等级。原有音频、窗口、按键和资源包全流程本轮不重复要求人工验收。
