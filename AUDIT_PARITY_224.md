# 2.2.4 可靠性修复与上游差异复核（2026-10-09）

## 基线、方法和范围

开始时 Rust `main` 与远程 `main` 均为 `164c1c7aab0b76766fdf85bd5d986951d1fceae4`，工作区干净。修复分支为 `fix/parity-hardening-224`。本轮未更新依赖、版本、资源或许可，也未执行 push、创建 Tag 或 Release。

重新执行 `git ls-remote` 并实际获取引用后确认：

| Java 引用 | 本轮获取的提交 | 源码版本 |
| --- | --- | --- |
| `2.2.X` | `47227a89fe05f1e3fe962f070f25c53111254a1f` | 真正的 2.2.4；本轮稳定版参考 |
| `2.3.0-dev` | `4dc00332e24a727b96be7b885d5cbbdd9c85263c` | 2.3.0-dev2；固定尺寸世界的开发参考 |
| `main` / `v2.2.4` | `42ba9192fcc229bcf615453a6504d415c777feaf` | 2.3.0-infdev3；实验参考，不能充当 2.2.4 |

本地 Java 路径实际是没有 `.git` 的源码目录。为避免信任该目录的版本标记，本轮通过忽略目录 `.reference/upstream.git` 的浅层 Git 对象直接读取上述提交，没有复制 Java 工程或替换 Rust 源码。稳定版 `src/client/resources` 的 **403 个文件** 与 Rust `assets` 逐字节比较，差异 **0**。

本文的 Java 路径均相对对应提交的 `src/client/java/minicraft/`；客户端 Rust 路径相对 `crates/minicraft-client/src/`，服务器路径相对 `crates/minicraft-server/src/`，Common 路径相对 `crates/minicraft-common/src/`。源码比较、Rust 回归测试、Java fixture 导入和实际 Java 游戏运行属于不同证据强度。本轮没有启动 Java oracle，因此不声称所有 Seed、帧像素、AI 或玩家轨迹已经一对一验证。

## 已实施的修复和证据

| 问题与原行为 | 修复后的行为 | Rust 位置 | 证据 |
| --- | --- | --- | --- |
| 主存档超限/非法 UTF-8/打不开时提前返回，掩盖有效备份 | 各候选独立检查；继续读取 `.bak`；全部坏时明确报错，均不存在才允许新建 | `saveload/storage.rs`、`worlds.rs` | 超限、无效编码、主文件缺失、备份损坏/缺失、Windows 独占文件测试 |
| 先轮换旧主文件、再安装新文件，失败时存在保护空窗；固定 `.new` 可能覆盖已有文件 | 独占创建唯一临时文件，写入并同步；同步旧主文件副本，先替换主文件，再替换备份；错误和退出清理只属于本次写入的临时文件 | `saveload/storage.rs` | 两代存档、残留 `.new`、强制备份轮换失败、Windows 共享权限错误测试 |
| 成功从备份加载后，下一次保存可能把坏主文件轮换为备份 | 用已验证备份修复主文件且保留备份；修复权限不足时仍允许读取，报告修复失败 | `worlds.rs` | 恢复后再次保存，验证主文件及备份都可解析 |
| 隐藏/损坏的世界目录不在菜单列表中，新建世界可能覆盖；菜单元数据不使用备份 | 拒绝任何已有目标路径；元数据限长并回退；改名/复制/删除检查真实目录归属 | `worlds.rs` | 元数据损坏后恢复并改名，备份仍可解析；null 候选不会引起改名 panic；已有隐藏目录内容保持不变 |
| 资源元数据的图像名称经字符串拼接到读取路径 | ZIP/文件夹统一拒绝绝对路径、点组件、反斜杠、驱动器/ADS、设备名和非法组件；文件夹检查真实路径归属，ZIP 拒绝链接条目 | `resource/pack.rs`、`assets.rs` | 真实资源元数据加载链中的越界 connector 图像测试；便携路径输入测试；Unix 链接测试已加入但本机未执行 |
| 仅看条目声明大小，实际读取可超限；ZIP 库先按中央目录计数分配 | 实际读取限 16 MiB；ZIP 容器限 64 MiB，打开中央目录前限制 4096 条目/4 MiB，并检查 ZIP64 计数 | `resource/pack.rs` | 无 metadata 的超限 reader、正常 ZIP/文件夹、伪造 ZIP64 极大计数；已核实依赖库 `read.rs` 的 `Vec::with_capacity` 调用链 |
| PNG 解码前可按恶意尺寸分配大量内存；RGB 尺寸乘法可溢出 | 输入限 16 MiB；边长 ≤4096、像素 ≤4194304、解码缓冲/库限额 32 MiB；尺寸乘法检查 | `gfx/image.rs`、`resource/pack.rs` | 大尺寸小文件 header、空/坏 PNG、RGB 极端尺寸测试；正常 pack logo 保持可用 |
| 无连接上限、无界消息队列、空闲读线程和关闭时遗留连接 | 默认最多 64 连接；登录 5s、空闲 30s、写入 2s；每连接队列 256；每秒 2048 消息；超限断开；关闭全部 socket 并 join 所有线程 | `server/server.rs`、`server/session.rs` | 7 个双客户端/连接集成测试，覆盖洪泛、空闲、容量恢复、分片登录、关闭和健康客户端不受影响；慢队列单测 |
| 不检查重复登录、名字冲突、消息方向及移动域；握手失败泄漏客户端线程 | 校验会话身份、大小写不敏感的重名、合法状态/方向；INIT 先于广播；就绪前在同一锁内更新存在列表和状态重放，避免同时登录的缺口；客户端同步握手后才创建 reader，Drop 关闭并 join | `server/session.rs`、`network/transport.rs`、`common/protocol.rs` | 非法状态不改变 replay；重复名字/非法消息；有界输入输出协议测试；确定性构造初次接受后另一个玩家加入/移动的重放回归 |
| 合成、工具材料扣除、使用物品放在 `debug_assert!` 中，Release 完全不执行 | 扣除作为实际操作执行；合成失败恢复完整背包；断言仅检查结果 | `item/recipe.rs`、`world/player_actions.rs` | **修复前 Release 回归失败（木材仍为 10，应为 0），修复后通过**；实际制作/放置工作台、Claymore 消耗工具材料、材料不足/满包回滚 |
| 死亡仅取走堆叠物品，工具仍留背包；active 又增加一件；27 格死亡箱装不下满背包加护甲 | 工具与堆叠物品一起转移；active 是背包引用，清除引用而不重复复制；死亡箱使用 Java 的无限存储容量 | `world/death_transition.rs`、`item/inventory.rs`、`entity/arena.rs` | **修复前回归失败（死亡背包仍占 1 格），修复后通过**；满包+工具+护甲逐项数量及死亡存档恢复 |
| Java 导出再次枚举 active，重复工具/物品；游戏 tick 导出的是模拟 tick | active 编码一次且保持每个堆叠数量；导出实际昼夜 tick | `saveload/save.rs` | 多堆叠 1000 木材总量不变、工具一次、日历 tick 与模拟 tick 不同的回归 |
| Java 导入背包超容量时忽略 `add` 的余量 | 先恢复玩家位置和层数，将多出的堆叠物品放在该层玩家附近；过量工具明确报错 | `saveload/load.rs` | 实际调用顺序/返回值证据；现有 Java core fixture 测试通过，28 工具布局仍需专项支持 |
| 旧/恶意 JSON 中极端背包总数、饥饿计数、天数和坐标距离导致整数溢出 | 数量累加/余量/计数使用饱和边界；距离扩大整数中间值后封顶；正常范围计算保持相同 | `item/inventory.rs`、`world/player_actions.rs`、`world/core_update.rs`、`entity/helpers.rs` | 保留超大堆叠的回归；存档恢复后执行真实生存/日历 tick；极值距离测试 |

Java 行为参考：[Recipe.craft](https://github.com/MinicraftPlus/minicraft-plus-revived/blob/47227a89fe05f1e3fe962f070f25c53111254a1f/src/client/java/minicraft/item/Recipe.java)、[Player.die / attack](https://github.com/MinicraftPlus/minicraft-plus-revived/blob/47227a89fe05f1e3fe962f070f25c53111254a1f/src/client/java/minicraft/entity/mob/Player.java)、[DeathChest 的 unlimited inventory](https://github.com/MinicraftPlus/minicraft-plus-revived/blob/47227a89fe05f1e3fe962f070f25c53111254a1f/src/client/java/minicraft/entity/furniture/DeathChest.java)、[Save.writeGame / writeInventory](https://github.com/MinicraftPlus/minicraft-plus-revived/blob/47227a89fe05f1e3fe962f070f25c53111254a1f/src/client/java/minicraft/saveload/Save.java)。

原子写入测试覆盖程序级失败和中断后候选布局，并未模拟真实突然断电。Unix 实施目录同步；Windows 的标准库没有相同的目录同步支持，不承诺断电后目录元数据的绝对持久性。备份轮换失败可能意味着新主文件已经保存，错误会明确说明。多文件世界元数据与状态不是整目录事务。

真实路径校验针对静态链接逃逸；其他本地进程并发替换目录产生的 TOCTOU 仍需平台专用句柄设计。没有把这种尚未验证的竞态报告成已闭合漏洞。PNG/ZIP 上限是新增的资源预算，超大自定义包会被拒绝，并输出原因。

多人实现仍是 **Rust 自定义 JSON-lines 状态中继**。坐标 0 ≤ x/y < 8192、层数 0..6（不含 6）、生命值 ≤20 是固定地图数据域检查；没有服务端权威世界、移动速度/碰撞验证、账号认证或完整反作弊，也不兼容 Java 联机线协议。名字只在当前服务器会话内唯一。客户端 DNS 查询仍依赖系统解析器，心跳由调用方发送。

## Java 2.2.4 功能对等性复核

| 模块 | Java 参考源码 | Rust 实际位置 | 本轮结论与剩余范围 |
| --- | --- | --- | --- |
| 世界生成 | `level/LevelGen.java`、`Level.java` | `level/generation.rs`、`level/noise/mod.rs`、`world/structure.rs` | 六层/尺寸/Seed 重复性和主题测试通过；尚未逐 Seed、128/256/512 全层与真实 Java 数组逐格比较，Rust 自身 hash 不构成 Java 相等证据 |
| 玩家与死亡 | `entity/mob/Player.java`、`screen/PlayerDeathDisplay.java` | `world/player_actions.rs`、`death_transition.rs` | 消耗与死亡重复/丢失已修复；实际攻击、饥饿/耐力/回血/死亡延迟、跨层流程有测试；仍需 GUI 与 Java 对照轨迹 |
| 实体、Boss、家具 | `entity/mob/*`、`entity/furniture/*` | `entity/*`、`world/level_update.rs`、`world/spawn.rs` | AI/掉落/家具/生成/碰撞的已有回归通过；不能据同名模块宣称全部 AI 一致。死亡箱期限仍固定 300 秒，Java Easy/Normal/Hard 为 450/300/150 秒，拾回/到期掉落需专项对照 |
| 物品、合成与容量 | `item/Items.java`、`Recipe.java`、`Inventory.java` | `item/mod.rs`、`recipe.rs`、`inventory.rs`、`world/inventory_ui.rs` | Release 扣材料和正常操作路径已修复；材料不足/满包回滚、新增最后耐久攻击用例通过。Java 独立 held item 与 Rust active 引用模型仍不同，满包+held 的所有布局尚未完全对等 |
| 世界交互 | `level/tile/*`、`Player.java`、`core/Updater.java` | `world/tile_*`、`player_actions.rs`、`core_update.rs` | 农作物、钓鱼、挖掘、楼梯与昼夜有现有测试；最后工具损坏、跨层中点/返回位置和保存恢复有实际 World 流程覆盖。未穷举所有玩法序列 |
| 游戏模式 | `Game.java`、`PlayerDeathDisplay.java`、`EndGameDisplay.java` | `world/death_transition.rs`、`end_game.rs`、`core/updater.rs` | Survival/Creative/Hardcore/Score 现有测试通过；Hardcore 死亡只有 Save & Quit 与稳定版一致，Java 此处没有自动删除世界。死后重启实际 GUI 仍需验收 |
| 界面、输入与进度 | `screen/*`、`core/io/InputHandler.java`、`util/AdvancementElement.java` | `app/*`、`screen/*`、`io/input.rs`、`progression/*` | 菜单栈、容器双栏、改键、资源包优先级等现有流程通过；本轮没有启动可交互窗口。Advancement 的 slots.empty/full 匹配在当前 inventory-name map 适配中仍未完整实现 |
| 持久化 | `saveload/Save.java`、`Load.java` | `worlds.rs`、`saveload/*`、`world/mod.rs` | 原生 envelope 与旧裸 JSON 的恢复/状态保留、Java core fixture、复制/改名/删除通过；**Java advancements.json、教程步骤/配方解锁尚未完整导入**，不称无损迁移 |
| 画面与音频 | `gfx/*`、`core/io/Sound.java` | `gfx/*`、`presenter/*`、`audio/wasapi.rs` | 403 文件完全一致；RGB 帧重复性、10 音效 WAVE/PCM 路径通过；没有新 Java golden-frame 比对或实际声卡听音。设备初始化失败的 WASAPI/SDL 静音回退已有实现，物理设备行为待人工确认 |

Java 格式导出 helpers 还有已知限制：出生点用当前位置填充；衣服颜色导出 0；配方解锁导出 `{}`；Score 初始分钟数默认 20；WateringCan 的 Java 数值并非完整状态导出。原生 JSON 存档不使用这些占位字段。新/旧原生存档及 Java 导入必须分开验收，不能把这些占位误称为原生数据已丢失。

上述剩余项保留现状并明确登记；本轮没有删除字段、重置玩家、迁移整个 LevelGen、改变地图尺寸模型或套用 Infdev 存档。

## Java 2.3.0 与 Infdev 功能差异矩阵

以下开发版路径来自 `4dc00332e24a727b96be7b885d5cbbdd9c85263c`；标注 Infdev 的来自 `42ba9192fcc229bcf615453a6504d415c777feaf`。开发版不是已发布稳定版本。

| 模块 | Java 参考源码 | Rust 实现位置 | 当前状态 | 缺失行为 | 移植难度 | 建议处理 |
| --- | --- | --- | --- | --- | --- | --- |
| 2.2.4 任务导入 | 稳定 `saveload/Load.java`、`util/AdvancementElement.java` | `saveload/load.rs`、`progression/*` | **稳定版缺失** | Java 任务/教程/配方进度恢复 | 中 | 比新增物品更优先；先建立有真实进度的 Java fixture，保证不覆盖原生进度 |
| 死亡箱期限与到期行为 | 稳定 `entity/furniture/DeathChest.java` | `entity/tick.rs`、`furniture/chest.rs` | **稳定版行为差异** | 按难度 450/300/150 秒、逐项拾回/到期语义 | 中 | 单独完善存档兼容与实际拾回流程测试 |
| 16 色染料 | 开发 `item/DyeItem.java`、`Items.java` | `item/dye.rs`、`item/mod.rs` | **已有枚举，未完整接线；2.3 新玩法** | 独立染料 ItemId、名称解析、背包/掉落/合成/纹理 | 低至中 | 优先小步实现；新增 ID 保留既有存档名称和索引 |
| 16 色羊毛 | 开发 `item/WoolItem.java`、`level/tile/WoolTile.java`、`Tiles.java` | `item/wool.rs`、`level/tile/wool.rs`、`item/mod.rs` | **已有 6 色；2.3 新玩法不全** | 其余 10 色物品/方块、染色配方/资源 | 中 | 与染料一起完整接线，避免只增加枚举和孤立单测 |
| 16 色床 | 开发 `entity/furniture/Bed.java`、`item/Items.java` | `entity/furniture/bed.rs`、`furniture/mod.rs` | **基础床可用；2.3 新玩法缺失** | 颜色状态、家具/物品转换、保存、配方和资源 | 中 | 在染料/羊毛之后；先设计旧 Bed 字段默认值 |
| 新花卉 | 开发 `level/tile/FlowerTile.java`、`Items.java`、`Tiles.java` | `level/tile/flower.rs`、`world/tile_behavior.rs`、`item/mod.rs` | **19 变体数量声明存在，完整玩法未接线** | 各独立花卉物品、19 变体注册/掉落/生成/纹理；稳定版仍用两个 flower_shape | 中 | 禁止把 `VARIANT_COUNT=19` 当作完成；先接注册/放置/采摘和存档，再处理生成 |
| 染缸与配方 | 开发 `entity/furniture/Crafter.java` 的 DyeVat、`item/Recipes.java` | `item/crafting.rs`、`furniture/mod.rs`、`world/inventory_ui.rs` | **2.3 新玩法缺失** | 染缸家具、入口、染料配方列表和动画/资源 | 中 | 在真实物品/颜色保存完成后实现；测试制作→放置→使用→再次加载 |
| 其他装饰方块/配方 | 开发 `item/Items.java`、`Recipes.java`、`level/tile/Tiles.java` | `level/tile/decor.rs`、`item/mod.rs`、`crafting.rs` | **有部分基础装饰，逐项运行结果待判定** | 需要注册表逐项差集以及实际获得/放置/破坏流程 | 中 | 与染料分批处理，不能仅按类名认定缺失或完成 |
| 有限区块管理 | 开发 `level/ChunkManager.java`、`Level.java` | `world/mod.rs` 中有限 Vec 层；没有同等 ChunkManager | **2.3 内部结构变化** | 64×64 chunk 分组及访问接口 | 高 | 独立设计；开发分支仍是固定尺寸数组，并非无限地图，现阶段无必要改存档 |
| 世界生成变化 | 开发 `level/LevelGen.java`、`Level.java` | `level/generation.rs`、`noise/mod.rs` | **现有 2.2.4 生成，未逐种子审核新算法** | 花卉/地形生成变化及生成规则兼容 | 高 | 先保存跨语言样本、生成版本标记；不直接整体移植 |
| 船和载具 | Infdev `entity/vehicle/Boat.java`、Vehicle 及 Player 调用链 | 家具/实体系统，无相同驾驶接线 | **Infdev 实验功能** | 上下船、载具运动/碰撞、玩家绑定、存档 | 高 | 独立开发阶段；不伪装为 2.2.4 缺失 |
| 无限地图 | Infdev `level/ChunkManager.java`、`Level.java`、存储调用链 | 固定宽高/六层 Vec、`worlds.rs` | **Infdev 实验架构** | 负区块坐标、按需生成、卸载、区块持久化/迁移 | 很高 | 专门设计与完整兼容测试后实施；本轮保持固定地图 |
| 生物群系 | Infdev `level/biome/Biome.java`、`Biomes.java`、各 Biome | `level/biome/mod.rs` 仅模块说明 | **薄模块，无运行机制；Infdev 实验功能** | 分布、边界、生成、群系资源/刷怪、保存版本 | 很高 | 依赖生成/区块设计；同名目录不表示实现 |

建议次序：稳定版进度迁移与死亡箱细节 → 染料/羊毛/床/染缸的完整操作闭环 → 花卉与装饰 → 有限区块设计 → 单独评估 Infdev。此次没有新增 2.3 玩法，以免未经版本化的物品/方块修改污染 2.2.4 世界。

## 工程检查与验证边界

新增 `.github/workflows/ci.yml`：Windows、Ubuntu、macOS，格式、编译、Workspace 测试、严格 Clippy、Release 消耗回归及无窗口 self-check；没有发布/打包任务。CI 尚未在远程执行，本轮禁止 push。Windows 使用已有 SDK `rc.exe`，非 Windows 使用现有 bundled SDL/OpenGL 路径，不更换依赖。

格式与 Clippy 的原始基线并不干净。必要的全仓格式化与 lint 整理涉及多个模块，未重写引擎。Java Simplex 浮点字面量保留原值，只在该模块明确允许精度/近似常量提示，防止改变舍入和确定性；已有多参数渲染接口仅使用局部豁免。测试断言和合法内部不变量保留。Windows 图标的 `MAKEINTRESOURCEW(1)` 保留数字资源 ID 1，不能用按 u16 对齐的 dangling pointer 替代。

实际平台：Windows x64 MSVC，rustc 1.99.0（b940084d7，2026-09-28）、Cargo 1.99.0，未实际编译 Linux/macOS，未做断电实验或真实 Java oracle。Release 曾运行客户端全套 509 项测试；后续新增资源目录/极值回归采用局部验证，然后统一执行最终 Workspace 检查。仅现有 `target` 被使用，没有第二套构建目录。

最终检查均通过（Windows x64）：

| 检查 | 实际结果 |
| --- | --- |
| 最终 `cargo test --workspace --locked` | 客户端 512、Common 7、服务器单元 5、集成 7，共 531 项通过；0 失败/忽略 |
| 最后元数据与登录保护补丁 | 全套后补充备份保留/null 候选断言并单独通过；登录重放竞态补丁后重新执行服务器 6 单元+7 集成测试通过 |
| Release 客户端全套 | 核心修复检查点 509 项通过；后续新增极值/ZIP 测试采用局部回归 |
| 最终 Release 消耗回归 | 通过；此前修复前版本明确失败 |
| `cargo fmt --all -- --check` | 通过 |
| `cargo clippy --workspace --all-targets -- -D warnings` | 通过 |
| `cargo build --release --locked` | 通过；必要的后续源码补丁完成后更新了验收 EXE，没有反复无变化全量构建 |
| `--self-check` | 原始资源与 PCM 检查通过，使用隔离 `test-data/audit-smoke` |
| 最终 EXE 的客户端/中继 smoke | 临时本机监听 `127.0.0.1:3409`，真实 `--multiplayer-probe` 完成 LOGIN/INIT/PING/PONG 并退出；通过，测试进程已清理 |
| `--audio-probe` | Windows WASAPI 静音诊断通过；30 播放命令、293 次写入、正常期空缓冲 0，MMCSS 已启用；没有完成主观听音 |
| Java 稳定资源 | 403 文件逐字节相等，0 差异 |
| Linux/macOS / 真实 Java oracle / 突然断电 | 未在本轮执行，不能宣称通过 |

额外执行了最终 EXE 的 `--server 127.0.0.1:0` 与 `--multiplayer-probe 127.0.0.1:3409 AUDIT_PROBE`，均使用隔离 savedir；3409 是本次临时监听分配的端口。

实际执行的主要命令如下：

```powershell
cargo test -p minicraft-client --lib saveload --locked
cargo test -p minicraft-client --lib resource::pack --locked
cargo test -p minicraft-client --lib gfx::image --locked
cargo test -p minicraft-client --lib release_regression --locked
cargo test -p minicraft-server --locked
cargo test --workspace --locked
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --release -p minicraft-client --lib --locked
cargo test --release -p minicraft-client --lib release_regression --locked
cargo build --release --locked
.\target\release\minicraft-rust.exe --self-check --savedir .\test-data\audit-smoke
.\target\release\minicraft-rust.exe --audio-probe --savedir .\test-data\audit-smoke
```

失败证据包括修复前的 Release 消耗和死亡回归、原始格式/lint 基线，以及并行新增 Debug 测试时 Windows 对已运行测试 EXE 的 LNK1104 文件锁；后者待旧测试退出后重新运行通过，与源码编译错误不同。新增 Unix 符号链接测试没有在 Windows 上冒称执行。

未穷尽所有合法 JSON 的运行语义（尤其异常坐标/实体速度/过渡字段）、第三方资源包竞态、音频队列长期资源预算或所有 AI 数值字段；本轮边界修复不能替代后续 fuzz 和跨语言玩法比较。

## Windows 人工验收（隔离测试世界）

从仓库根运行 ` .\target\release\minicraft-rust.exe --savedir .\test-data\manual-audit `。使用新建的测试世界或旧世界副本，不破坏实际世界。默认键为 C 使用/装备、X 背包、Enter 确认/使用工作台、Esc 返回。

1. 新建 Survival（128、Seed 42），收集 10 木材，制作工作台，装备并放置。预期木材按配方消耗，工作台在放置后从背包扣除；材料不足无法再制作。满包时失败的合成保持材料不变。
2. 使用木斧直到最后一次耐久。预期有效攻击仍生效，斧头消失，背包释放一格，后续攻击不会使用残留斧头；吃食物/使用消耗品后数量实际减少。
3. 建造、携带物品并经楼梯换层；记录位置、生命/饥饿、工具耐久、任务/成就。Save & Quit 后退出程序再启动，重新加载，预期原生存档状态保留。旧原生存档和 Java 导入副本分别检查，Java 进度缺失按上述限制验收。
4. 在可丢弃 Survival 世界装入工具和护甲、尽量填满背包后死亡、复活、走向死亡箱。预期工具/装备/堆叠各出现一次，无复制或消失；实际碰触拾回行为与箱内自动测试一起验收。另建 Hardcore 世界死亡，预期死亡菜单只有 Save & Quit，重新进入不会给出正常复活选项；不要要求自动删除世界。
5. 启用已有正常 ZIP 与文件夹资源包，调整优先级，查看水动画/纹理覆盖；在 Options → Key Bindings 改使用/背包键，再检查菜单与游戏实际动作一致。退出重开检查设置保留。
6. 实际听选择、砍伐、受伤/死亡等音效；切换/禁用默认音频设备后重新启动，预期初始化失败时游戏仍可运行。检查标题栏/任务栏图标、OpenGL 画面与窗口切换。这些涉及真实设备/窗口，self-check 没有覆盖。
