# Java 2.3.0 第一阶段玩法移植（2026-10-16）

## 基线与范围

开始时 `fix/parity-hardening-224` HEAD 确认为 `eac5b77001ca065a86c0750be47090b43386f395`，Git工作区干净；已阅读两份2.2.4审计记录。新分支 `feat/java230-colored-items` 直接从该已验证提交创建，因此依赖未合并main的2.2.4修复，不能直接移到旧main而省略父链。

重新查询上游远程：`2.3.0-dev` 仍为用户指定 `4dc00332e24a727b96be7b885d5cbbdd9c85263c`（2.3.0-dev2）；稳定2.2.X仍47227a89，main仍42ba9192（Infdev3）。本轮只使用前者新增玩法，保留独立的2.2.4 Oracle，不迁移无限世界/船/群系或升级依赖。

实现提交：`d43b0b408c5bf590c7c8f9a46b95756b4edec4cf`。后续文档提交后的最终完整SHA、实际最终检查结果见本地 `target/parity/java230/final-validation.md`；该记录在最终HEAD验证后写入忽略目录，不在测试后改动HEAD。没有push、Tag、Release、合并main或改写已有历史。Cargo包版本保持原值，本分支不冒称已发布2.3稳定版。

## 已接通的玩法

| 模块 | 实际操作链路 | Java固定参考 | Rust主要位置 |
| --- | --- | --- | --- |
| 16色染料 | 独立物品/名称/贴图；染缸制作、组合染料、堆叠、Q/ShiftQ丢弃拾回、死亡箱、原生保存恢复 | `item/DyeItem.java`、`Items.java`、`Recipes.java` | `item/dye.rs`、`mod.rs`、`recipe.rs`、`crafting.rs`、`world/player_actions.rs`、`assets.rs` |
| 16色羊毛 | 颜色与物品/方块双向映射；织机染色；在Hole/Water放置；剪刀开采、最后一点耐久、同色掉落和拾回、地图保存 | `WoolItem.java`、`level/tile/WoolTile.java`、`Tiles.java` | `item/wool.rs`、`tile_item.rs`、`level/tile/mod.rs`、`wool.rs`、`world/tile_attack.rs` |
| 16色床 | 织机用同色羊毛制作或白床染色；放置/使用睡眠/出生点/拆除、物品↔家具状态传递；满包留在原地；保存后颜色和行为保留 | `entity/furniture/Bed.java`、`FurnitureItem.java`、`Save.java`/`Load.java` | `entity/furniture/bed.rs`、`item/furniture_item.rs`、`world/furniture_action.rs`、`entity_render.rs`、`render.rs` |
| 染缸 | 工作台Stone10+Iron5制作；家具放置、真实合成入口、35配方、材料扣除/不足拒绝/满包回滚；拾回/保存 | `Crafter.Type.DyeVat`、`Recipes.dyeVatRecipes` | `entity/furniture/mod.rs`、`item/crafting.rs`、`recipe.rs`、`world/mod.rs` |
| 19花卉 | Rose/Oxeye Daisy保留，再加17物品；草地放置、独立variant、徒手/铁锹采摘、原花掉落、染料原料、保存与显示 | `FlowerTile.FlowerVariant`、`TileItem.java`、`Recipes.java` | `level/tile/flower.rs`、`item/mod.rs`、`tile_item.rs`、`world/tile_attack.rs`、`assets.rs` |

织机使用固定Java2.3的完整57条配方，染缸35条，另加工作台染缸1条；旧工作台的43项及工具穿插索引保持，染缸追加末尾。织机染色成本改用独立染料，普通床生产改为彩床；旧普通床、旧衣服/羊毛物品仍可读取使用。没有为了保留旧菜单而混入无Java依据的重复配方。

新配方采用真实2.3 recipes.json的93个相关解锁定义，追加独立owner命名空间，防止旧同名已完成记录挡住新配方推进。多个原料得到同一产物时保留全部已解锁成本；旧解锁成本也保留在原生variants中。教程/任务关闭不影响配方解锁。重导入解锁集合会替换而非遗留旧variants。

## 存档、ID与资源兼容

- **旧142个物品的顺序、serde名称、asset key、显示名逐项保留**；新追加60项，总202。旧白羊毛保持 `ItemId::Wool`/显示Wool，Java `White Wool`通过别名匹配，避免改坏旧序列化名称。
- 旧方块0..58不变；十种羊毛追加59..68，新花卉`Flower230`为69。没有采用Java2.3重排后的数值ID。火把/栅栏/牌子底层ID继续使用Rust既有稳定编码。
- 旧`Tile::Flower`的两种纹理及旧data语义不变；新`Flower230`保存0..18的Java花卉ordinal，两者不会混读。旧生成仍是2.2.4，未改LevelGen或相同Seed结果。
- 床复用原FurnitureKind：旧state0仍是旧普通Bed、默认红色，保留旧`bed.png`包覆盖；新state1..16为DyeColor ordinal+1。修正裸state2的死亡箱判断对红床的冲突，只对容器执行死亡箱限制。
- 原生新增配方字段均serde default；旧catalog只更新静态定义，不重置库存、进度或时间。未知物品名称反序列化报错，不静默删除。
- Rust新版本读取旧原生及Java2.2.4存档仍受原回归保护；含新枚举的原生存档不能交给旧EXE读取，不宣称双向降级兼容。建议用旧世界副本验收。
- **本轮不是完整Java2.3世界导入器**：其花卉data、Torch底层ID与2.2.4不同。直接完整导入Java>=2.3明确拒绝且不写state.json，不会假装兼容后跳过彩床/误读地图；原Java文件保持不变。2.3部分advancement的跨语言导入和完整世界布局另行设计。

原始403资源逐字节复核：0差异。新增106个原始PNG共24,026字节，仅位于`assets/expansion230`；60 item、17 entity、29 tile，manifest记录固定源路径/提交和SHA256。另含上述93个原值解锁定义的筛选JSON。来源为Minicraft+，沿用GPL-3.0，根LICENSE及上游说明保留，无AI替代贴图或额外素材包。旧ZIP/目录资源包按原优先级加载，缺少新key时使用嵌入资源，可按Java标准新key覆盖。详见该目录README。

## 实际Java对照与测试证据

`scripts/parity230/run.ps1`校验固定源码/资源blob，重新编译全部Java客户端/common源码，classes和固定资源优先于依赖jar。实际执行成功；仅复用现有官方jar的第三方依赖，没有把jar内游戏类假作固定源。输出放现有忽略target，七个小JSON fixture约29KB进入测试。

`world/java230_oracle_tests.rs`比较真实Java结果：

- 16个DyeColor的顺序/packed颜色/名称/资源名；彩床碰撞、物品/实体颜色和保存恢复。
- 35/57/1全配方的产物、数量和完整材料多重集合，保留替代配方。
- 16羊毛实际Java放置、剪取、耐久17→16、掉落/底层Hole；Rust自身ID仅做显式颜色映射，不伪称与Java重排后的ID相等。
- 19花实际放置ordinal、徒手采摘掉落、恢复草地。

`world/java230_*tests.rs`和输入/资源回归覆盖真实World的制作/攻击/放置/使用、剪刀最后耐久、满包、睡眠和出生点、Q/ShiftQ逻辑tick、全部60新增物品死亡箱/拾回/原生保存数量守恒、Save & Quit所用磁盘writer/loader、旧catalog迁移及未知物品拒绝。局部22项java230测试、7项drop筛选测试通过，后加Java2.3拒绝导入回归1项通过；最终完整测试包含它们，不以enum存在作为玩法完成证据。

一次掉落回归失败来自测试把玩家移到随机散布的角落；改为留在原中心并增加每项地面数量2/背包0和散布坐标断言后全部拾回2。没有降低守恒断言。调试构建有文件系统incremental硬链接失败后复制的环境警告，未改依赖/缓存目录规避。严格Clippy的代码lint已通过，最终结果以最终记录为准。

有意修正两处上游错误：Java铲花先reset tile再读取data，实测Violet错误掉Oxeye Daisy；Rust保存采前variant再掉同花。Java Player的drop-one/stack条件与InputHandler键名/注释相反；Rust按Q一件、ShiftQ整堆，工具作为独立实例扣除。均不声称该错误分支逐行相同。Rust active仍引用包内ItemId，多个同种999堆时ShiftQ丢第一匹配堆，不借本轮重写整个库存。

## 最终验证及剩余范围

所有改动及文档提交后，在最终HEAD统一执行：

```powershell
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo test --release --workspace --locked
cargo build --release --workspace --locked
```

随后实际Release EXE `--self-check`，Release parity的2.2.4噪声位模式/六层逐格比较，以及 `java230-preview target/parity/java230/preview.png`。后者通过真实World/方块/家具渲染器绘制单张颜色总览并人工查看；是Rust离屏Smoke，不是Java整帧golden，也不是Windows交互窗口试玩。测试用 `MINICRAFT_PARITY_RENDER_OUTPUT` 指向根target，不改已有tracked PNG。最终逐项退出码/计数见final-validation.md；此处不把计划执行的检查预写成通过。

仍未完成：新增花卉自然分布/生存来源（需独立生成版本设计，当前通过Creative获得）；完整Java2.3世界导入和部分进度迁移；控制器专用丢弃键映射；已有Controls页逐行重绑定UI差异（配置/逻辑输入重绑定已测试）；完整Java整帧/AI轨迹比较。CI定义保留，远程三平台CI及Linux/macOS本机编译未执行，不能标为通过。未实现本轮明确排除的Infdev/多人/引擎重构等内容。

## Windows试玩（仅窗口交互仍需玩家确认）

从仓库根启动隔离目录，旧世界只复制后验收：

```powershell
./target/release/minicraft-rust.exe --savedir ./test-data/manual-java230
```

1. 新建Creative世界，在物品目录取得染缸、织机、白羊毛、白床及几种花/染料。逐项选中、放置；检查16色羊毛/床/染料及19花没有missing贴图，旧资源包缺少新图时仍可显示。
2. 使用工作台检查染缸配方Stone10+Iron5；打开染缸，把Rose转Red Dye，再混合染料。织机用白羊毛+染料制作彩羊毛，用Wood5+同色Wool3制作床，并试WhiteBed+染料。Creative不扣材料是原有模式规则；实际扣材料/不足/满包请在Survival副本中检查。
3. 羊毛放在孔/水上，用剪刀剪回同色；花放草地后徒手/铁锹采回原花；夜间使用彩床，预期入睡、出生点正确。拆床再放置应保持颜色，满包拿床时家具留在原地。
4. 手持新增物品按Q丢一件、ShiftQ丢当前一堆，走近拾回。Survival副本携带新染料/羊毛/床/花死亡、复活拾回死亡箱，预期数量/颜色不变；使用Save & Quit关闭程序再启动，确认物品、已放方块、床色及配方进度保持。
