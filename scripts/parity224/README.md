# 固定 Java 2.2.4 参考执行

源码 pin 为 `47227a89fe05f1e3fe962f070f25c53111254a1f`，许可证 GPL-3.0-only。参见 `../../crates/minicraft-client/tests/fixtures/java224/README.md` 的来源和范围说明。

从仓库根执行（PowerShell，JDK 17+）：

```powershell
./scripts/parity224/run.ps1
cargo run --locked -p minicraft-parity -- java224-noise target/parity/java224/output
cargo run --locked -p minicraft-parity -- java224-generation target/parity/java224/output
cargo test --locked -p minicraft-client --lib java224_
```

可通过 `-Source` 和 `-DependencyJar` 指定已有参考目录和 jar；仍必须满足固定 Git blob。输出只进入现有 target。逐格比较报告每层第一个不同坐标及差异格数，发现差异即非零退出；名字只规范已验证的稳定 ID 别名，不修改方块或 data。噪声比较检查 features 16/32 的 IEEE 754 位模式。存储到 Git 的两种 Seed 黄金哈希测试不依赖本机 Java，可在正常 Cargo/CI 测试中运行。

这是原始地图和选定对象行为参考，不等于 Java 完整游戏进程、所有随机调用或整帧渲染轨迹对等。完整结构放置、AI、攻击伤害/掉落轨迹和跨语言渲染仍为 UNKNOWN。
