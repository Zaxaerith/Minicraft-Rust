# Java 2.2.4 实际执行样本

GPL-3.0-only，来源 MinicraftPlus/minicraft-plus-revived。
固定源码提交：`47227a89fe05f1e3fe962f070f25c53111254a1f`（真正的 `2.2.X`）；不用错误指向 Infdev 的 `v2.2.4` Tag。

生成入口：仓库根 `scripts/parity224/run.ps1` 和 `Oracle.java`。
所有固定版 Java 客户端/common 源码重新编译，源码和资源逐文件校验 Git blob；新编译 classes 和固定版资源在官方 jar 前，jar 只供应第三方依赖。需要 JDK 17+ 和已存在的固定版源码、官方依赖 jar、`.reference/upstream.git`。不会下载/复制整个 Java 仓库。

Player/Inventory/Entities 文件由真实 Java `Save` writer 生成。Advancements 的部分完成状态经真实 Java load/save 输出，保留原始时间，不要求通过实际 GUI 玩到该状态。reference.json 的期限、耐久与合成结果来自真实 Java 对象调用。tiles.json 来自 59 个注册方块和黑曜石上火把的 Level 数据。generation.json 是实际 Java 六层原始 LevelGen 数组的 FNV-1a 哈希（每格 ID 一字节、data 小端两字节），Seed 42/256、128×128、默认 Island/Normal；不是 Rust 自己生成的预期值。

原始逐格数组及噪声位模式仅保存在忽略的 `target/parity/java224/output`。JSON 样本可按对象比较重新生成结果（JSON 键顺序不属于语义）。仅七个小样本进入 Git。
