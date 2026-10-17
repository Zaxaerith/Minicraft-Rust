param(
    [string]$Source = '.reference/release-2.3.0-dev',
    [string]$DependencyJar = '.reference/official/minicraft-plus-2.2.4.jar'
)
$ErrorActionPreference = 'Stop'
$repo = (Resolve-Path "$PSScriptRoot/../..").Path
Set-Location $repo
# Reuse the existing complete pin verification, compiler and original oracles.
& "$PSScriptRoot/run.ps1" -Source $Source -DependencyJar $DependencyJar
$classes = (Resolve-Path 'target/parity/java230/classes').Path
$jar = (Resolve-Path $DependencyJar).Path
$resources = (Resolve-Path "$Source/src/client/resources").Path
$separator = [IO.Path]::PathSeparator
$dependencyClasspath = "$classes${separator}$jar"
$gameClasspath = "$classes${separator}$resources${separator}$jar"
$output = Join-Path $repo 'target/parity/java230/world36'
New-Item -ItemType Directory -Force $output | Out-Null
& javac --add-exports java.base/jdk.internal.org.objectweb.asm=ALL-UNNAMED -encoding UTF-8 -cp $dependencyClasspath -d $classes "$PSScriptRoot/WorldProbe36.java" "$PSScriptRoot/WorldStages36.java"
if ($LASTEXITCODE -ne 0) { throw 'World observation driver compilation failed' }
$manifest = Join-Path $output 'manifest.txt'
Set-Content -LiteralPath $manifest "Premain-Class: minicraft.level.WorldProbe36`n" -Encoding ascii
$agent = Join-Path $output 'probe36.jar'
& jar cfm $agent $manifest -C $classes minicraft/level/WorldProbe36.class -C $classes 'minicraft/level/WorldProbe36$1.class' -C $classes 'minicraft/level/WorldProbe36$1$1.class' -C $classes 'minicraft/level/WorldProbe36$1$1$1.class'
if ($LASTEXITCODE -ne 0) { throw 'Observer agent packaging failed' }
foreach ($case in @(@(128,42),@(128,256),@(128,0),@(128,-1),@(256,42),@(512,42))) {
    $size=$case[0]; $seed=$case[1]
    Push-Location $output
    try {
        & java --add-exports java.base/jdk.internal.org.objectweb.asm=ALL-UNNAMED --add-opens java.base/java.util=ALL-UNNAMED "-javaagent:$agent" '-Djava.awt.headless=true' -cp $gameClasspath minicraft.level.WorldStages36 (Join-Path $output "$size-$seed") $size $seed
        $code=$LASTEXITCODE
    } finally { Pop-Location }
    if ($code -ne 0) { throw "Actual World.initWorld observation failed for $size/$seed" }
}
