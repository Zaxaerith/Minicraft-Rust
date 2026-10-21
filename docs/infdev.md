# Infdev 实验引擎（Windows x64）

当前提供独立图形化飞行探索，以及单独的**基础 Survival 实验检查点**。
支持地表0、铁矿层-1、金矿层-2、宝石层-3、黑曜石地牢-4；已接通采集、工具、冶炼、苹果/饥饿循环、牛/猪/羊的狩猎、剪毛/染色及区块死亡箱；尚无完整Survival、天空、船只或多人；新规则已接通昼夜、火把照明、地下Slime/Zombie及首日后的地表夜间刷怪。普通游戏仍使用固定世界，
`Legacy224`、`Java230Flora`、`Java230World`及既有存档不切换规则。
Phase 4A历史说明见[归档记录](history/infdev4a.md)。

## 整合准备与验证状态

Phase 4A～4L功能冻结点为`fc87f12a4f671ef9a6cbf88a21e55aa723d76996`；当前只准备审查和整合，
没有将Experimental提升为正式模式。现有Java dev2 Windows RC仍是固定世界包，不包含这些新实验功能。

| 检查 | 实际结论与边界 |
|---|---|
| Release Workspace / Java与Infdev Oracle | 生产基线`4168194`的728项及Oracle PASS；后续`fc87f12`只改诊断，整合准备不重跑全量 |
| Phase 4L原生窗口短流程 | PASS：空包采集/制火把、地下自然Slime/Zombie、Esc保存、独立进程恢复全部6个实体页/库存/时钟，菜单暂停及GC |
| 完整昼夜原生窗口长测 | 收到WM_CLOSE中断，未到首夜，返回码1；不是控制脚本超时或主动kill；发送者UNKNOWN |
| 夜间保存及独立窗口恢复 | NOT RUN：前置长测中断后停止，不自动重试 |
| Phase 4真人键盘/鼠标试玩 | 尚未完成；程序Input窗口测试不等同真人试玩 |
| 历史偶发MoveFileW发布失败 | 根因UNKNOWN；已增加现场诊断，受控锁冲突回归不能证明历史原因 |

详细证据保留在`target/validation/current/infdev-final-validation.md`、`validation.txt`、
`infdev-environment-acceptance.log`、`environment-processes.json`及关闭/发布失败现场；不清理这些日志或旧世界。
源码与Oracle逐项对照仍有下文列出的非确定性和玩法边界，不能把以上各行合称“全部验收PASS”。

## 存档与规则隔离速查

默认实验根为`SAVEDIR/experiments`；显式`--infdev-root`可另指定根，只改变目录，不自动进入实验模式。

| 目录 | 模式与契约 |
|---|---|
| `infdev-v2` | 已有单层飞行探索；保留原地表生成器/区块编码，GUI可以打开，不直接重解释成多层 |
| `infdev-v3` | 五层飞行探索，`JavaLayersV1`；新建入口为New Layered World (v3) |
| `infdev-survival-v4` | 独立Survival；新建入口为New Survival World (v4)，各代规则由保存字段区分 |

普通固定世界仍在`SAVEDIR/saves`，保持`Legacy224`、`Java230Flora`、`Java230World`原契约。
没有v2/v3→v4自动转换，也不承诺新存档可被旧EXE降级读取。旧v4不是统一的一套规则：
按已有`food_rules`、`death_rule`、`entity_rules`、`enemy_rules`、`station_rules`及`environment`继续运行。
缺失字段分别保留Legacy食物、KeepInventory、Legacy实体、无敌怪、Session工作站和无environment；
已有PigV1/PassiveV1、死亡箱或Zombie标记不被回退或补成最新规则。新建v4才默认启用当前组合。

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
   没有改写原配方的材料或数量；新敌怪规则开放原Sword配方，装备等未接线配方仍不开放。木斧可攻击自然出现的牛/猪/羊并消耗实际耐久。
5. 站上楼梯按V/P下矿，使用镐采集铁/金/宝石/青金石。返回对应楼梯可回地表；负坐标和区块边界仍适用。
6. 采集至少30 Iron Ore及10 Coal：熔炉每次把3 Iron Ore + 1 Coal变为1 Iron（铁锭），金矿对应Gold；
   在工作台用5 Iron制作Anvil，放置后以5 Wood + 5 Iron制作Iron Pickaxe。
   熔炉/铁砧均用背包装备后C放置、Enter打开菜单、V/P拆回；它们使用原游戏的即时配方交易，没有独立燃烧计时或内部库存。
7. 自然树每次受击有1/100概率掉1 Apple，空手也可获得；没有保底或凭空赠送。选择Apple后按C食用，
   消耗一个苹果与2点耐力，恢复1点饥饿；饱食或耐力为0时不消耗。森林中的大量树木是当前可探索获取的食物来源，
   下矿前建议准备苹果。新规则世界还可狩猎猪获得Raw Pork、牛羊获得Raw Beef；农业、钓鱼及烹饪尚未接入。
8. 按Q丢一个所持物品、Shift+Q丢整堆；工具按实例丢弃并保留耐久，1秒后可自动拾回。
9. Esc先关闭背包/合成面板，再按Esc执行Save & Quit；重进恢复位置、生命、耐力、库存、工具、掉落物及地图修改。

装备铲子可把草变成泥土、挖泥土/沙子得到物品并留下洞。有限放置沿用已有TileItem规则：
Dirt填洞/流体、Plank铺水面、Stone或StoneBrick铺桥，以及放回已采集的花。未接线的树苗等放置会拒绝并保留物品。
V/P面向工作台、熔炉或铁砧可拆回相应真实物品掉落；楼梯上优先执行跨层。

新建v4使用显式`food_rules: JavaNormal`，按Java Normal难度消耗饥饿（每阶段400预算、7阶段/饥饿点；
静止且满耐力时每30 tick扣1预算，移动、缺耐力、回血有额外消耗）；饥饿>5可缓慢回血，饥饿为0每120 tick扣1生命、最低保留3生命。
菜单暂停计时。存档保留全部饥饿/回血计数器；新规则重生会恢复饥饿及计时。
旧v4缺少此字段时使用`Legacy`，保持原来不消耗饥饿、不自动回血的行为，没有自动升级。
边界：目前没有装备或农业；普通动物为Cow/Pig/Sheep；旧敌怪规则仅地下Zombie，新环境规则支持Slime/Zombie。
水中不回复耐力且持续消耗，耗尽后溺水伤害；熔岩造成伤害。死亡后按Enter回初始出生点，
出生点被阻挡会拒绝重生。**新建v4使用死亡箱；旧v4保留各自death_rule，缺失时才保留库存**，具体规则见下节。
下矿前请准备足够工具，尚无Boss/地下家具生成。
旧规则session最多保留512个掉落实例；新规则的普通掉落进入区块实体页，不再共用全世界512上限。
新建世界的工作台/熔炉/铁砧及染缸进入区块实体页，旧世界仍用session的64个工作站上限。达到目的页上限时拒绝新增操作，物品不会被静默删除。掉落暂不自然过期。
尚未推进随机方块tick或树/岩石的伤害衰减；部分采集伤害会持久化，不代表这些时序已与Java完整对等。
耐力与伤害由后台游戏tick推进，菜单暂停生存计时；GUI不执行磁盘访问或地形采样。

v4格式为`MinicraftRust.Experimental.Infdev.Survival.v4`，使用原`JavaLayersV1`地形、`MCIFCH04`区块头。
游戏RNG及原有Player/Inventory数据位于有校验的不可变`sessions/`对象中（单对象256KiB上限）；旧规则的工作站/掉落仍位于session，新规则对应实体位于`entities/`对象。
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
不会为了生成箱子清地形或吞物品。当前未接入的装备和完整怪物系统不因此宣称完成；地下Zombie另见战斗章节。

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

新规则分页实体已接通牛/猪/羊、Slime/Zombie、火把、普通掉落和工作站；旧世界的512掉落及工作站64上限仍保留原session契约。
目前支持有限的Slime/Zombie追击/战斗，没有离线模拟或通用无限容器更新器。水壶运行玩法尚未接入Infdev，
但共享物品模型中的已有实例经过死亡箱/磁盘测试保持完整。

## 动态实体与狩猎（仅新建v4）

新建Survival写入`entity_rules: PassiveV1`，仍使用原v4协议和独立实验目录。
历史`PigV1`世界保留仅猪、原碰撞/活动规则和原序列化；缺少字段的旧v4按`Legacy`读取，
默认字段再次保存仍省略。已有世界不补动物、不搬运旧session掉落，也不升级死亡规则；v2/v3不变。
新字段/实体类型不保证被旧EXE读取，没有自动原地升级。

三种动物复用原`Mob`、物品、工具和贴图，不新增ItemId/Tile ID。固定Infdev实际只有Cow/Pig/Sheep，没有Chicken。
当前采用Normal难度：牛生命10，猪羊生命8，hurtTime10；空手基础伤害1–2，工具使用原近战/耐久规则。
牛死亡掉1–2 Leather和相同数量Raw Beef；猪掉1–2 Raw Pork；未剪毛羊先掉1–2对应颜色羊毛，
再独立抽取1–2 Raw Beef，剪毛羊不再掉羊毛。Raw Beef/Raw Pork均可通过原食物操作食用；饱食不消耗。

羊自然颜色按原版白80%、黑/灰/浅灰各5%、棕3%、粉2%。铁砧以原4 Iron配方制作Shears，
背包选中后朝羊按C剪毛，产生1–3对应颜色羊毛，消耗1耐久；已经剪毛的羊不能重复产毛。
新规则工作台开放原Dye Vat配方（10 Stone、5 Iron），放置后Enter打开原35种染料配方。
选中真实染料对羊按C染色，消耗1份染料（即使颜色相同，剪毛羊也可染色）。
牛、羊站在Grass时每tick有原1/1000机会吃草变Dirt；羊同时恢复羊毛。两者与动物状态一起原子保存。
颜色、剪毛状态复用既有彩色羊系统；固定dev2与本Infdev的这些交互规则一致，Java Save未保存羊色不代表可恢复历史颜色。

生成依据原`Level.trySpawn`/`PassiveMob.checkStartPos`：地表Grass/Flower，每tick最多30候选，
相对玩家每轴[-64,63]Tile，白天roll<=33牛、>=68猪、其余羊；最少80像素距离，
Normal白天矩形排斥半径320像素，沿用`spawn_skip_chance`与200活动种群上限；首个合法候选只生成一只。
历史无environment世界固定使用这个白天分支；新环境规则的昼夜抽签见下节。
**无限世界适配：**只更新当前深度、玩家相邻且地形已在LRU中的最多9区块，不为刷动物生成远处地形；
种群/排斥检查作用于这些活动页。距离采用安全i64全局Tile/格内像素，出生像素继承玩家位置，没有每帧全球扫描。
Java每只动物用独立时间RNG；Rust复用已持久化的session JavaRandom与稳定页/ID顺序，
不宣称自然出现坐标、多动物随机轨迹或全帧更新与Java逐次一致。

动物保存类型、生命、格内像素、游走/受击、年龄、毛色/剪毛、击退与空闲计时；使用原4×3像素身体检查地形和玩家/动物碰撞。
奇数tick移动，每40机会重选方向、45游走计数；不穿入实心地形、水、熔岩、空洞或未加载区块。
跨页在同一事务移出源页、插入目标页，保留`birth=(初始depth/区块X/Y,id)`身份；页内ID不复用。
目的页满或越过i64边界则保留原拥有者/位置，不能丢失动物，也不能只提交吃草而丢失对应再生状态。
卸载/菜单/离线暂停活动；重新载入读取原实体页，不重新投放初始动物。

PassiveV1按原Mob阈值处理活动动物：超过1280像素移除；空闲1800tick且远于640时每800机会移除；
年龄超过18000tick且不在玩家128×76近邻矩形则移除。近于640重置空闲计时。
它限制反复经过区域的活动种群；未加载实体不扫描、不离线变老，不能保证全球动物总数恒定。
当前没有Lantern放置，尚不实现原灯光抑制消失、水中活动或动物自动上下楼梯。
明确修正两处上游Bug，仅用于新规则：`Level.distanceOfClosestPlayer`以hypot(w,h)初始化会截断真实距离，
这里使用真实无限坐标距离；原Mob的受伤计时会抑制全部击退，这里让6→0击退按原整数/2位移并遵守身体碰撞。
旧PigV1不增加这些新行为。不将此称作完整Java动物引擎或精确AI轨迹对等。

新规则的采集、Q丢弃和拆工作站继续由原有Survival动作产生物品，session掉落数组只作本次动作的临时outbox，
随后移动到对应depth/区块的实体页。沿用原ItemStack/ToolItem，保留堆叠、耐久、水壶731/23/UNKNOWN状态。
取物允许部分成功，满包保留剩余物品；新掉落、猎杀、拾取和跨页迁移会请求原子检查点，
玩家、源页、目标页及地图一起提交。读取/容量预检在地图或库存变更前完成，错误不清除所有权。
每页仍最多64实体/256KiB；普通掉落暂为静态可拾取实体，没有抛物线/离线过期。
新建世界的工作站已分页；打开校验和GC仍遍历已保存根，不承诺超大世界无打开成本。

固定Java Oracle复用`AnimalOracle.java`，原`animals.json`黄金不变；新增`passive-animals.json`记录原牛/羊执行：
三难度生命、五Seed Normal死亡掉落、全部16色实际染色/剪毛、各120无障碍Dirt移动tick、Seed329吃草再生，
以及远距查询和受伤移动的原版问题。输出/小型Fixture仍在原位置；Rust逐项比较真实状态、数量和每tick位置/方向/计数。
另反射执行原版`Level.trySpawn`：32 Seed、受控白天草地、同一玩家位置的首次动物类型、像素坐标和毛色逐项相等；
这是单次生成的受控Oracle，不等于真实多实体世界/时间RNG全序列一致。

窗口诊断继续用`python -X utf8 scripts/infdev4a/window-smoke.py --hunt-only`，
从空背包采集制作木斧，自然寻找三种动物，正常狩猎取得猪肉/皮革/羊毛、拾取与食用、跨区块，
两个独立进程在载入tick前核对完整实体页及库存，并运行GC。没有注入库存、动物、地形或饥饿。
这属于程序Input驱动的真实Windows窗口，不是玩家真人试玩；剪毛/染色有Java及Rust回归，仍建议真人检查其手感和颜色显示。

## 地下敌怪与基础战斗（仅新建v4）

新建Survival在原v4协议中显式保存`enemy_rules: UndergroundZombieV1`。
缺少字段的所有历史Legacy/PigV1/PassiveV1世界按`None`读取，再保存仍省略字段，不自动增加敌怪或武器菜单。
已保存的地图、生成器版本、动物/物品ID和死亡箱规则不转换，不建立v5目录；新存档不保证被旧EXE读取。

无environment的历史敌怪规则只接通原版Zombie；新环境规则另有Slime，见下节。
Zombie在-1/-2/-3层分别为level1/2/3，Normal生命10/40/90，接触伤害1/2/3；
玩家30tick受伤保护、Zombie10tick受击保护，复用原工具/武器近战与耐久、分页移动及4×3身体碰撞。
新规则工作台/铁砧开放原Sword配方（Wood Sword为5Wood），不是赠送武器或修改材料成本。
Zombie死亡按原Normal表掉1–3 Cloth，再独立抽取稀有Iron、彩色Clothes和Potato；使用原ItemStack，不丢弃稀有项。
衣服穿戴/种植尚未接入，不宣称全部稀有物品用途完成。

生成依据固定Infdev `Level.trySpawn`、`EnemyMob.checkStartPos`：当前深度、每tick最多30候选、
相对玩家每轴[-64,63]Tile、最小60像素、普通地下矩形排斥半径13×16像素与实体身体、Normal活动种群上限300及原跳过概率。
原Slime<=40、Zombie41–75、Creeper76–84、Skeleton>=85选择不改；首个合法候选抽到尚未支持的类型则结束本轮，
不替换成Zombie或继续重抽。当前只观察已加载附近页，所有已保存实体不会每帧全球扫描。

**历史亮度/范围边界：**没有`environment`字段的旧Zombie世界继续采用无光域模型；不自动开放火把或地表夜间刷怪。
新环境规则接入昼夜及火把，见下节；Lantern、-4地牢Snake/Knight和Spawner仍未接通。

AI沿用原EnemyMob/MobAi顺序：移动后1/200机会重新游走60tick，计数为0且100像素内才追击，
保留原`sig0=1`的对齐轴不对称规则。地下EnemyMob原距离处理不调用普通远距消失分支；
按MobAi寿命3600活动tick且不在玩家128×76近邻时移除，卸载/菜单/离线暂停。
受击击退沿用PassiveV1已明确的修正，不能越过实心地形、未加载页或满目标页；保留birth身份与等级/生命/AI状态。
上下楼梯若抵达被怪物占据的位置，新敌怪规则允许从已有身体重叠中走出，沿用原Entity.move的`wasInside`处理思路；
没有删除、移动怪物或重置楼梯，旧None规则的碰撞阈值和行为不改。真实Java重叠移动观察与自然楼梯回归覆盖此场景。
玩家接触伤害已接通，玩家自身的击退、护甲、药水、得分及完整AI/游泳尚未对等；新环境火把照明单独说明。
近战仍使用既有相邻Tile单目标适配器，尚未移植原Player20像素矩形的群体挥击；同格或斜角需移动到正面再攻击。
Java每实体时间RNG与Rust持久化会话RNG不同，不宣称多怪物逐帧轨迹完全一致。

怪物攻击扣血触发检查点；致死使用原死亡箱，把原库存移动到死亡位置，与玩家清空、怪物页及地图在同一个检查点提交。
满页/受阻继续使用原库存保留保护，不吞物品；成功拾回也与库存一起发布。读取失败不重新生成怪物页或删除掉落。

原`AnimalOracle.java`继续扩展，新增小型`enemies.json`：实际原版三难度/三级生命、接触伤害及保护计时，
128Seed Normal死亡掉落，Seed42追击/远距游走各80tick，以及三地下层×16Seed的原私有`trySpawn`。
完整Java观察保留Slime/Skeleton/Creeper数据；Rust明确只支持Zombie，并验证其他抽签不会偷换或重抽。
所有旧动物/地图黄金样本不变，输出继续复用原目录。

真实窗口诊断：`python -X utf8 scripts/infdev4a/window-smoke.py --combat-only`。
使用独占测试目录，从空背包采集、合成工作台/木剑/木镐、走自然楼梯下矿，遭自然Zombie攻击后击杀取得Cloth，
跨区块保存退出；新进程在tick前比较完整实体页/库存。随后自然怪物致死（不调用DiagnosticDeath），
在死亡检查点后终止自己的进程，重启、复活、沿楼梯返回拾回原库存并再次保存/GC。
诊断持续记录最后存活时的真实库存，允许途中提前被怪物击杀；只有死亡箱与该库存完全相同、
且现场为地下干燥地面和相邻自然Zombie时，才接受致死检查点，不把路线失败或环境伤害计作战斗通过。
这是程序Input驱动的真实Windows窗口，不是玩家真人试玩；没有注入库存、摆怪或伪造掉落。
死亡箱转移逐项比较完整ItemStack/ToolItem序列及容量；`slot_revision`是新背包用于失效旧引用的内部计数，
转移后可正常改变，不能把它当成玩家进度恢复。转移后的Save/Load仍比较完整背包（含该计数），并检查旧活动引用已清空。

## 昼夜、火把与Slime（仅新环境规则）

新建v4显式保存`environment.version: JavaDayTorchSlimeV1`；包含`day_tick`、有符号`game_time`和`past_day_one`。
旧v4缺失整个`environment`时为None，再保存仍不写入：Legacy、PigV1、PassiveV1、原Zombie和Phase 4K工作站世界
不自动启用时间、火把或Slime。原地形与v2/v3含义不变，不创建v5，不保证旧EXE降级读取。
未知环境版本/字段和越界日内时间拒绝；损坏继续整体回退或拒绝，绝不重新生成地图。

时钟调用现有`core::clock::Clock`推进：一天64,800个60Hz活动tick，四阶段各16,200tick，初始Morning。
首日地表夜间不生成敌怪；完整跨过一天后才开放地表Night。地下-1/-2/-3不受昼夜限制，-4仍无敌怪。
新规则使用Java `Level.trySpawn`的同一候选流，先敌怪、再地表动物回退；保留Slime0–40、Zombie41–75、
Creeper76–84、Skeleton85–99抽签，后两者未实现且消耗本次合法候选，不替换/重抽。
动物夜间Cow上限22、白天33；Pig从68起，其余Sheep；夜间排斥半径15格、白天20格，均来自原PassiveMob。
旧世界继续用原独立动物/Zombie路径。NPC仅更新附近活动页，不扫描全世界。

游戏暂停面板、死亡界面与离线均不推进环境时钟；Save & Quit/重启保存当前计数，不按墙钟补时。
这是实验host的明确暂停规则：原Java聚焦菜单时gameTime可继续，而这里连同日历一起暂停；没有额外的GUI时间系统。
HUD显示阶段，地表渐暗及地下黑暗复用现有Screen.overlay/LightOverlay，玩家自身半径5×8像素，
火把半径同为5×8像素，不额外构造光影引擎或声称有墙体遮光。保留旧无环境世界的原明亮探索画面。

### 真实可获得的火把

使用已有原配方：手工或工作台`1 Wood + 1 Coal → 2 Torch`。材料必须正常采集，不赠送。
背包装备Torch后C放在合法草地、泥地、沙地或已有地板；V/P拆回真实掉落，自动拾取遵循原满包规则。
楼梯上V/P仍优先跨层，拆火把时站到普通地面。火把不阻挡移动，工作站/不可放置地形拒绝。
原Java要求PowerGlove拆回，V/P是沿用实验模式现有家具交互的明确扩展，不声称工具前提完全相同。

存储用非实心Torch实体覆盖底层Tile/data，避免原Torch data只存底层Tile ID而丢失原data。
底层数据不被替换；挖掘前需拆回，动物不能吃掉其底层草地。光源、库存、掉落、死亡箱和地图同一检查点提交。
火把与其他实体共享每页64实例/256KiB限制，满页预先拒绝；跨层、卸载、备份及GC保护不变。

刷怪照明采用原Torch的±3格正方形查询，与绘制的径向光晕是两个规则；不会把玩家自带视觉光晕当作刷怪保护。
最多查询候选周围四个实体页；绘制额外读取五格光源边界，仍是有界快照，不全球搜索光源。
Java `getAreaTilePositions`保留有限w/h裁剪，负坐标火把在原版isLight中失效；Rust有意移除这个裁剪，
在所有合法i64坐标保持同样的3格规则。Oracle保留原版负坐标false值，此处是无限世界必要扩展。
原版Level.trySpawn还会以像素调用isLight；Torch的Tile坐标检查已由EnemyMob.checkStartPos执行。

Lantern尚未开放制作/放置。原Infdev Type灯半径9/12/15，但isLight混用实体像素矩形、Tile位置及像素参数，
实际Oracle在多个坐标的tileCall都为false；现有固定版普通Lantern半径还是8。这里保留证据，不把猜测的修正当原样移植。

### Slime运行边界与Oracle

Slime使用现有Mob/等级/物品模型，Normal三级生命2/8/18，接触伤害1/2/3、受伤保护及击退复用现有战斗。
死亡使用原1–3 Slime掉落（Normal），不是Zombie的Cloth表。跳跃计时独立持久化`slime_jump`，旧实体缺失为0。
真实Slime子类：50像素追击、1/40游走，跳跃中不重新随机方向；计时≤-10且有方向时跳10tick，减至0时停止移动。
绘制固定朝下，跳跃时使用跳跃帧并上移4像素；普通Zombie仍保留100像素/1⁄200规则。
跨页满目标保留原拥有者，计时、生命、等级、birth、掉落和死亡箱随同事务保存；不假称完整逐帧多实体RNG一致。
地表白天的原版远距离清除分支尚未补齐；当前继续沿用已验证的活动寿命/附近玩家保留规则，没有伪造日光灼伤。

原`AnimalOracle.java`扩展，新增必要小型`environment.json`，不修改上游源码：
- 原setTime边界、首日标记、243个Torch查询和9个Lantern观察。
- 64个真实地表昼夜首个刷怪，所有原版类型都保留；遇未支持类型验证不替换。
- Slime三难度/三级HP、接触保护、32种子实际掉落、近/远两条各160tick的真实跳跃轨迹。
新流最初在Night/首日/Seed6首次不同：误沿用白天20格排斥得到Pig(568,536)，Java为Cow(-264,168)；
按PassiveMob原夜间15格规则修复后64例全部通过，没有修改Java预期来掩盖差异。

`python -X utf8 scripts/infdev4a/window-smoke.py --environment-only`运行独占新世界的短流程：
空包采集、制作工具/木剑与火把，地下自然Slime/Zombie战斗并拾取掉落、放置光源、返回地表，
经普通Esc输入Save & Quit，再启动独立进程。在worker启动前准确比较完整时钟、库存、位置及全部已保存实体页，
随后验证菜单暂停、再次保存与GC；短流程不等待夜晚。

显式追加`--environment-night`，在短流程独立通过后，用第三进程原速等待首夜，记录白天/傍晚/夜晚画面和墙钟耗时，
再用第四进程检查夜间保存恢复。同一天仍为原64,800 tick，首夜48,600 tick；不注入时间、不加速或缩短一天。
只尝试一次长等待，关闭或超时后直接失败并保留短流程结果，不自动重跑。
窗口标题显示等待进度，`environment-processes.json`记录各PID、起止时间、返回码、超时/强制终止标记及EXE哈希。
失败时保留实际窗口状态、游戏快照和渲染图；旧`Window smoke interrupted by window close`只证明窗口收到关闭消息，
没有来源证据时保留UNKNOWN，不将其归因于用户或游戏自行关闭。
这是程序Input驱动的原生窗口，不是人类键盘试玩。实际结果以`target/validation/current/infdev-final-validation.md`和同目录日志为准。

## Windows发布与工作站长期存储

新建v4增加独立的`station_rules: PagedV1`，工作台、熔炉、铁砧及已有染缸共用区块实体协议。
所有历史v4缺少该字段时按`Session`处理，再保存仍省略字段，原64工作站及既有敌怪规则不变；没有自动搬迁或v5。
PagedV1的持久session中`stations`必须为空，重复拥有数据明确拒绝；未知规则/实体类型也拒绝。
新存档不保证旧EXE能够降级读取。

放置、Enter合成、V/P拆回仍调用原有StationKind/配方/Inventory操作；原材料、燃料、工具成本不变。
一次动作只临时投影目标工作站，发布前移回目标实体页，不复制一份持久库存或工作站系统。
每个实体页仍共享64实体/256KiB上限，与动物、Zombie、掉落和死亡箱共同占用容量；
全世界工作站总量不再受session的64个限制。ID/birth、depth与i64坐标跨卸载和重启保留。
合成菜单、玩家/动物碰撞、渲染、拆回掉落都读取分页状态；满包拆回保留原掉落，不静默吞物品。
满页操作预先拒绝，工作站、材料和掉落与原死亡箱共用同一检查点、备份恢复及GC保护。

### Windows os error 5 的证据边界

历史日志只能确认独占目录发布失败；当时没有源临时目录和句柄现场，根因保持**UNKNOWN**。
已核实当前调用链在`MoveFileW`前`drop(w)`，关闭自身`share_mode(0)`的session.lock；
未发现可证实的生产句柄泄漏，也没有证据将错误归因于杀毒软件或ACL。
受控隔离回归实际观察：故意持有同一排他写锁时MoveFileW返回5，持有不允许删除共享的源目录句柄时返回32，
释放后发布成功；目标已存在（包括空目录）仍拒绝覆盖。它证明可触发的条件，不证明历史原因。
另有4线程×2次独立新建/重开/重复目标拒绝测试，临时路径独占，无无限重试。

现在Infdev发布失败会报告具体操作、原Win32码、源/目标路径、文件属性、最多16个子项状态、PID和World关闭前后进程句柄总数。
总数受并发影响，不能当作某个文件的句柄归属；外部持有者仍标UNKNOWN。
不可变对象写入/同步和checkpoint原子保存错误也附带路径上下文。没有修改全局ACL、共享模式或安全检查，没有覆盖发布或自动重试。
失败证据继续放在`target/validation/current/`；玩家遇到此错误时请保留具体日志和世界副本，不删除恢复hold来强行通过。

### 混合存储测量

`./target/release/minicraft-parity.exe infdev-survival-measure target/validation/current`
复用现有诊断模块，在独占、自动清理的目录建立64/256个合成测试区块，含多层/正负坐标、
工作站、牛猪羊、Zombie、独立水量掉落、工具及死亡箱。它是**构造的存储夹具，不是自然生存试玩**。
LRU仅8页，保存重启和GC前后逐项比较完整页，另执行真实附近实体更新后再次重开。
输出复用`infdev-mixed-storage.csv`（chunks列为测试构造的64/256区块，另保留初始出生区块）；记录打开完整校验、最后常驻批次保存、GC毫秒与实际文件字节数，
不清OS缓存、不使用墙钟硬阈值，不把操作系统缓存差异当优化收益。每次运行均完整验证，未为性能跳过数据。

工作站分页阶段的Windows Release测量（合成夹具；区块数不含初始出生区块，字节数含当前/备份对象；非本次重测）：

| 测试区块 | 实体 / 工作站 | 打开校验 | 最后常驻批次保存 | GC | GC前 / 后文件字节数 |
|---|---:|---:|---:|---:|---:|
| 64 | 398 / 192 | 40.26 ms | 1.61 ms | 176.80 ms | 1,002,137 / 879,883 |
| 256 | 1,589 / 768 | 138.84 ms | 1.42 ms | 635.97 ms | 3,933,837 / 3,547,034 |

GC后分别270/1,044个文件；未放宽当前/备份引用保护。此规模未发现需要局部优化的明确瓶颈，
不据此承诺任意大世界的打开时间；完整校验与GC仍为O(N)。

`python -X utf8 scripts/infdev4a/window-smoke.py --survival-only`已从空背包执行真实窗口流程：
采木石、制作/放置工作台和熔炉、下矿取得30 Iron Ore与10 Coal、真实扣料冶炼、制作铁砧和使用铁镐、
拆回熔炉并自动拾回、食用自然苹果、Save & Quit及独立进程重开。分页工作台/铁砧的ID、birth、
类型、局部坐标与页键逐项恢复，熔炉只保留一份物品，session工作站为空，随后GC成功。
使用程序Input驱动现有窗口/菜单/worker，不是人类试玩，也没有注入材料或预设工作站。

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
`python scripts/infdev4a/window-smoke.py`运行飞行、Survival各一对、死亡恢复三个及狩猎两个独立窗口进程，用同一菜单/探索器和编程Input验证
自然生成、正负区块、楼梯至-4并返回、各层编辑、地下Save & Quit及重开。它不是人类键盘试玩；PNG为真实软件渲染输出。
窗口脚本还通过独立v4菜单从空背包执行采集、工作台、木/石工具、熔炉、铁砧和铁镐制作，真实矿/煤扣除，苹果自然掉落与食用，正常碰撞移动、下矿返回及重启检查；
`--survival-only`仅运行这一组窗口流程。它不会注入库存、传送角色或预置资源。
`python scripts/infdev4a/window-smoke.py --death-only`单独运行死亡链路：从空背包真实采集，
下矿后受控致死；在死亡检查点成功后强制终止进程，重开、复活并沿真实楼梯返回取物，
再次Save & Quit、第三进程恢复和GC校验。受控条件只改变生命值，不注入库存、摆放资源或传送。
自动输入经过实际菜单/窗口/worker/容器；这是原生窗口自动化，不是玩家真人验收。
该死亡箱历史窗口实测位于地下-1层`(-42,-24)`，工具与Acorn/Apple/Stone/Coal/Iron Ore均来自正常采集。

新增回归覆盖部分拾回、满包、工具/不同水量实例、重复死亡、受阻/满页、过期保留、128实体页LRU、
正负i64边界、五层、磁盘恢复、死亡/拾回发布前中断、写入失败、损坏整体回退及GC备份保护。
完整门禁继续用`scripts/validate-windows.ps1 -JavaOracle`，现有728项Release测试基线保留；本次文档整合准备未重复运行。
最终结果复用`target/validation/current/infdev-final-validation.md`及同目录日志，不另建阶段日志树。
冶炼/食物配方依据固定世界`item/recipe.rs`及固定Infdev的`item/Recipes.java`；苹果与食物规则依据
`level/tile/TreeTile.java:149-168`、`item/FoodItem.java:47-54`和`entity/mob/Player.java:398-450`。
复用原ItemId/Inventory/ToolItem，无新增物品ID、Tile ID或世界命名空间；Java的树随机源与Rust会话RNG不同，
这里对齐规则和数量，不宣称每次掉落的随机序列或全帧玩家更新顺序逐次相同。
下次优先处理Lantern的明确坐标语义、活动规则和掉落生命周期，并继续收集Windows发布失败现场；同时评估更大世界的分批校验/GC；
不重解释已有生成器版本，不将当前检查点当作完整无限世界游戏。
