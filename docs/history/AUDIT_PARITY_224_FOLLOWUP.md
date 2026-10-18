# Java 2.2.4 后续功能审计（2026-10-10）

本记录接续 `AUDIT_PARITY_224.md`；上一轮提交 `86abb26bf00e481a56973e6f9fac1687fe9cdd6a` 的安全加固保留。起始 Git 工作区干净，继续本地 `fix/parity-hardening-224`，没有覆盖用户修改。
本次再次查询上游远程：真正的稳定分支 `2.2.X` 仍为 `47227a89fe05f1e3fe962f070f25c53111254a1f`。错误指向 Infdev 的 Tag 没有作为参考。没有迁移 2.3.0。

## 已修复的有证据差异

| 问题 / 修复后行为 | Rust 源码 | 固定 Java 参考 |
| --- | --- | --- |
| advancements.json 原来没有导入：现在恢复完成教程、显式教程游标、任务/任务组解锁、部分 criteria 和 unlockingCriteria、配方 advancement；已完成奖励不重复发放，原始时间和未知项保留 | `saveload/load.rs`、`progression/progress.rs`、`advancement.rs` | `saveload/Load.java`、`Save.java`、`util/AdvancementElement.java`、`screen/TutorialDisplayHandler.java`、`QuestsDisplay.java` |
| 配方解锁原来未接实际合成：现在保存完整产物/成本定义，菜单、数量和制作操作共用过滤；获得材料后按真实 recipes.json advancement 解锁，教程/任务关闭仍运行配方推进 | `item/crafting.rs`、`world/player_actions.rs`、`inventory_ui.rs` | `CraftingDisplay.java`、`AdvancementElement.java`、`resources/recipes.json` |
| Java 导出占位出生坐标、颜色、配方、Score 初始分钟数和壶内容：现在保留真实状态；修正重复末尾逗号、Java 层索引和 signs 层号；服装颜色也接入玩家 whiteTint 渲染 | `saveload/save.rs`、`load.rs`、`world/mod.rs`、`render.rs` | `Save.writePlayer/writeInventory/writeGame`、`Player.java`、`gfx/Sprite.java`、`World.lvlIdx` |
| Java 导入层数组与原生 Rust 顺序不同，旧上下楼/复活按固定索引错层：现在按 depth 查找，保留两种已有存档布局 | `world/death_transition.rs`、`mod.rs` | `core/World.java`、`Player.respawn` |
| 死亡箱原来固定 300 秒、拾回识别错状态、满包可能未正确扣余量、到期删除工具/物品：现在创建时捕获 450/300/150 秒，保存期限，只转移成功数量/工具实例，余量留箱，到期生成可拾取的物品与保留耐久工具；死亡箱不能拿成家具，Creative 接触也可拾回 | `entity/furniture/chest.rs`、`arena.rs`、`types.rs`、`tick.rs`、`world/level_update.rs`、`death_transition.rs`、`entity_render.rs` | `DeathChest.java`、`Chest.die`、`Player.die`、`Level.tickEntity` |
| Java DeathChest 实体导入未正确恢复内容和剩余期限：现在专门解析，无限容量接收物品和工具，保留实际剩余 ticks；不支持的箱内物品报错而不是静默丢弃 | `saveload/load.rs` | `Save.writeEntity`、`Load.loadEntities` |
| Java 独立 held item 在满 Rust 包时无法插入工具：现在保持其数量/耐久并作为附近可拾取实体；active 引用在死亡/导出时不会额外复制 | `load.rs`、`entity/arena.rs`、`world/death_transition.rs` | `Player.activeItem`、`Inventory.java` |
| 实际注册名称 Cloud Cactus、Obsidian、Stone、Obsidian Boss Wall/Floor/Door 缺少映射；现代 Torch 的 base data 被覆盖成 Grass：现在 59 个稳定方块 ID 全部通过真实 Java fixture，火把保留底层 ID，旧带名火把仍兼容 | `saveload/load.rs` | `level/tile/Tiles.java`、`TorchTile.java`、`Load.loadWorld` |
| Seed 相同却产生不同地表/天空/地牢：边缘距离错误为八次方，Java 连续两次四次方；天空仙人掌还错误排斥邻近仙人掌。按原 Java 运算和判定修复，洞穴的八次方保持原值 | `level/generation.rs` | `LevelGen.java` 地表约 246–249、地牢 499–503、天空 699–735；洞穴 588–590 |

所有新增原生字段都有 serde 默认值，旧原生存档的已有背包/进度保持；旧存档没有配方解锁数据时继续保留原有可制作集合，不凭空猜测历史。新世界和 Java 导入使用真实配方解锁。既有地图不会重新生成；修复会改变新建世界的地表、天空、地牢，使已检查样本与 Java 一致。

## 实际 Java 证据与验证范围

新增 `scripts/parity224/Oracle.java` / `run.ps1`，编译未经修改的全部固定源码，源码/资源逐个检查 Git blob。官方 jar 仅提供依赖。执行和输出在原有忽略目录 `target/parity/java224`，不创建另一套 Cargo target。七个小型真实 fixture 提交到 `crates/minicraft-client/tests/fixtures/java224`；生成方法和哈希编码见该目录 README。

- 实际 Java writer 的 Player/Inventory 字节字段、教程游标、已完成/部分完成/解锁时间、任务状态在导入→原生保存→恢复→字段导出中验证；没有重置玩家。JSON 对象比较保留原始时间。对未知进度及未知教程游标也做保留测试。
- 固定 Seed 42、128×128，六层原始地图每层 16384 格的方块和 data 实际逐格比较全部通过；features 16/32 噪声共 32768 个 double 位模式相等。
- 增加 Seed 256 的实际 Java 六层黄金哈希，两种 Seed 共十二层的方块/data 测试通过。旧 Rust 自身快照断言被更强的实际 Java 黄金值替换，预期不是从 Rust 导出。
- 修复前首次逐格差异：Seed42 天空 (86,11)、地表 (71,8)、地牢 (4,4)；地下 -1/-2 本来一致，-3 的 LAPIS/OBSIDIAN 名称别名经稳定 ID 校对后亦一致。指数修复后天空只剩 (30,24) 起共4格，进一步定位到仙人掌邻域判定后归零。明确的公式/判定差异有源码依据；没有伪称测得完整 RNG 首次调用分歧。
- 真实 Java 死亡箱构造期限、最后耐久/耗尽调用、拒绝材料不足的木门制作、正常木板产量，与 Rust 实际流程比对；死亡→满包部分拾回→保存恢复→到期→地上拾取检查堆叠、工具耐久、护甲守恒。
- 八项 `java224_` 回归通过；局部 `saveload` 26 项和 `progression` 26 项通过（各自的执行检查点），严格 workspace Clippy 通过。开发过程发现并修复了缺失 Java `STONE` 映射的真实失败；测试构造/编译错误已纠正，不作为功能一致证据。

完整的最终 Git HEAD 验证在本地提交后执行，命令、退出码、计数和完整 SHA 写到忽略目录 `target/parity/final-validation.md`，最终交付同时给出该记录。这样不在检查后改动 HEAD，也不把计划执行的检查提前写成通过。最终使用一次优化版全 Workspace 测试，覆盖 Release 扣材料问题；不反复全量构建。

## 保留差异与 UNKNOWN

| 项目 | 状态 / 下一步 |
| --- | --- |
| 所有 Seed、256/512 尺寸、其他地形/主题的 Java 逐格对照 | UNKNOWN：本轮只对固定默认设置的 128 两种 Seed 建立黄金样本，已有尺寸/主题 Rust 测试不等于跨语言一致 |
| 完整 Level 结构、联层楼梯、Boss/实体布置 | UNKNOWN：LevelGen 原始数组一致不能替代完整世界组装对照 |
| 完整攻击伤害轨迹、怪物 AI、死亡/掉落 RNG 序列、逐帧更新时序 | UNKNOWN：本轮实际 Java oracle 仅验证选定工具/配方/死亡箱构造行为；Rust 现有测试不冒称 Java 轨迹通过 |
| 典型场景整帧渲染、死亡箱倒计时文字和临近过期红闪 | UNKNOWN / 明确界面差异：未生成真实 Java golden frame；服装 whiteTint 已接线但还需窗口观察；死亡箱计时文本和红闪仍未接入 |
| 死亡箱触碰范围和满包提示 | Rust 使用中心距离阈值，Java 通过 bounds touchedBy；精确接触时序及提示文字仍存在差异，物品余量守恒已验证 |
| 独立 held item 与 active 引用 | 保留架构差异：Java 27 格包外另持有一项，Rust 引用包内；溢出保留附近物品，数量守恒但布局不是 28 独立格完全对等；不在本轮重构全库存 |
| WateringCan 实例状态 | 单个玩家持有壶的导入/原生保存/导出内容已验证；Rust 仍是玩家级 watering_content，多个壶不同内容、箱内/死亡/地上独立内容未完整支持，不能宣称所有壶实例无损迁移。需要专门物品实例存储设计 |
| Advancement slots.empty/full/occupied | 现有聚合 inventory-name map 不等于 Java 实际格布局，相关触发条件仍不完全对等；本轮不假造格数 |
| 完成时间、旧原生解锁历史 | 新事件记录实际 UTC ISO LocalDateTime，原 Java 本地时间原样保留；旧存档没有的过去时间/解锁历史不虚构 |
| 完整 Java 存档导出 | 修复现有字段 helpers，不代表已提供全世界双向 exporter；未逐项穷尽所有实体/物品导入 |
| Java 联机协议 | 仍是 Rust JSON-lines 中继，没有 Java 联机协议兼容或权威服务器的声明 |

因此本轮没有宣布 Java 2.2.4 全玩法完成；以上未知或需要实例架构设计的部分继续专项处理。依赖、Cargo.lock、版本、GPL 许可证、403 原始资源未变，上一轮保存/资源/网络保护保留。

## 格式与 CI 复核

比较上一轮 164c1c7→86abb26 的 113 个 Rust 文件：45 个通过在内存运行当前 rustfmt并忽略局部 allow 属性证明仅格式/豁免差异；其余 68 个包含语义修复或需审查，结合差分检查嵌套 if→let-chain、默认结构体初始化、布尔条件、Java浮点常量保留等，没有发现无关的行为变更。LevelGen 上一轮只有格式/import 整理，本次实际地图差异来自既存公式，不能归因于格式化。

检查三平台 CI 的 SDK rc.exe、Linux bundled SDL 构建依赖、无设备 self-check。新增 `MINICRAFT_PARITY_RENDER_OUTPUT` 指向根 target，避免测试写 tracked 的 crate 下 PNG。没有执行远程 CI，Linux/macOS 也没有在本机编译，保持 NOT RUN；不以工作流存在代替通过。

## Git 与 Windows 人工验收

继续 `fix/parity-hardening-224`；本轮允许的本地提交和最终完整 SHA 见最终验证记录。未 push、未创建 Tag/Release、未合并 main。根目录旧审计记录为上一轮历史，本记录覆盖其进度/死亡箱/地图的后续结论。

仅需进入真实 Windows 窗口的验收如下，使用测试世界或已有世界副本；从仓库根运行：

```powershell
./target/release/minicraft-rust.exe --savedir ./test-data/manual-224-followup
```

1. 在导入的 Java 副本查看当前教程、任务组已解锁状态、合成菜单与衣服颜色；完成当前教程的剩余条件，Save & Quit，关闭程序再进入。预期游标继续推进、已完成任务不重发奖励、配方保持解锁，服装颜色可见。
2. 在该副本上下楼、死亡后复活，再保存重进。预期上下楼层正确，复活回地表出生点，生命/背包/耐久和已保存进度保留；检查真实层切换动画和碰撞位置。
3. 在可丢弃 Survival 世界携带一把已损耗工具、护甲、堆叠物，填满包后碰触死亡箱，再空出容量拾回。预期只转移装得下的数量，余量仍在，工具耐久和护甲各一件。可分别 Easy/Normal/Hard 保持该层有效运行约450/300/150秒观察到期掉落；暂停/菜单时间不是有效运行时间。倒计时文字/红闪目前仍缺失，不作为已修复验收条件。
4. 检查新世界的初始合成列表，收集木材后制作木板/工作台；预期对应配方解锁、消耗正确。用工具最后一点耐久攻击，预期最后一次有效后工具移除；画面没有残留装备精灵。

自动化已验证的格式、编译、字段和原始地图不要求玩家再次手工检查。此轮没有改音频/资源包/按键系统，不新增它们的人工重复验收。
