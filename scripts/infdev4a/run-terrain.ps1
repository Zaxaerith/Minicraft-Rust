#requires -Version 7.0
$ErrorActionPreference='Stop'
$repo=(Resolve-Path "$PSScriptRoot/../..").Path
Set-Location $repo
& "$PSScriptRoot/run.ps1"
& python "$PSScriptRoot/extract_sources.py"
if ($LASTEXITCODE -ne 0) { throw 'Infdev source extraction failed' }
$source=Join-Path $repo 'target/parity/infdev4a/source'
$output=Join-Path $repo 'target/parity/infdev4a'
$classes=Join-Path $output 'classes'
$jar=(Resolve-Path '.reference/official/minicraft-plus-2.2.4.jar').Path
$sources=@(Get-ChildItem "$source/src/client/java","$source/src/common/java" -Recurse -File -Filter '*.java')
$resources=@(Get-ChildItem "$source/src/client/resources" -Recurse -File)
. "$repo/scripts/parity224/verify-source.ps1"
Assert-ReferencePin (Join-Path $repo '.reference/upstream.git') '42ba9192fcc229bcf615453a6504d415c777feaf' $source (@($sources)+@($resources))
$arguments=@($sources | ForEach-Object {'"'+$_.FullName.Replace('\','/')+'"'})
$arguments+='"'+"$PSScriptRoot/TerrainOracle.java".Replace('\','/')+'"'
$arguments+='"'+"$PSScriptRoot/UndergroundOracle.java".Replace('\','/')+'"'
$arguments+='"'+"$PSScriptRoot/DeathChestOracle.java".Replace('\','/')+'"'
$arguments+='"'+"$PSScriptRoot/AnimalOracle.java".Replace('\','/')+'"'
$argsFile=Join-Path $output 'terrain-sources.args'
[IO.File]::WriteAllLines($argsFile,$arguments,[Text.UTF8Encoding]::new($false))
& javac -encoding UTF-8 -cp $jar -d $classes "@$argsFile"
if ($LASTEXITCODE -ne 0) { throw 'Original Infdev source compilation failed' }
$cp="$classes$([IO.Path]::PathSeparator)$source/src/client/resources$([IO.Path]::PathSeparator)$jar"
Push-Location $output
try { & java '-Djava.awt.headless=true' -cp $cp minicraft.level.TerrainOracle $output; $code=$LASTEXITCODE } finally { Pop-Location }
if ($code -ne 0) { throw 'Actual Infdev terrain execution failed' }
Push-Location $output
try { & java '-Djava.awt.headless=true' -cp $cp minicraft.level.UndergroundOracle $output; $code=$LASTEXITCODE } finally { Pop-Location }
if ($code -ne 0) { throw 'Actual Infdev underground execution failed' }

Push-Location $output
try { & java '-Djava.awt.headless=true' -cp $cp minicraft.entity.furniture.DeathChestOracle $output; $code=$LASTEXITCODE } finally { Pop-Location }
if ($code -ne 0) { throw 'Actual Infdev DeathChest execution failed' }

Push-Location $output
try { & java '-Djava.awt.headless=true' -cp $cp minicraft.entity.mob.AnimalOracle $output; $code=$LASTEXITCODE } finally { Pop-Location }
if ($code -ne 0) { throw 'Actual Infdev animal execution failed' }

& python -c 'import json,sys; assert json.load(open(sys.argv[1],encoding="utf-8")) == json.load(open(sys.argv[2],encoding="utf-8")), "original animal Oracle fixture changed"' (Join-Path $output 'animals.json') (Join-Path $repo 'crates/minicraft-client/tests/fixtures/infdev4a/animals.json')
if ($LASTEXITCODE -ne 0) { throw 'Original animal fixture comparison failed' }

& python -c 'import json,sys; assert json.load(open(sys.argv[1],encoding="utf-8")) == json.load(open(sys.argv[2],encoding="utf-8")), "original passive Oracle fixture changed"' (Join-Path $output 'passive-animals.json') (Join-Path $repo 'crates/minicraft-client/tests/fixtures/infdev4a/passive-animals.json')
if ($LASTEXITCODE -ne 0) { throw 'Original passive fixture comparison failed' }

& python -c 'import json,sys; assert json.load(open(sys.argv[1],encoding="utf-8")) == json.load(open(sys.argv[2],encoding="utf-8")), "original enemy Oracle fixture changed"' (Join-Path $output 'enemies.json') (Join-Path $repo 'crates/minicraft-client/tests/fixtures/infdev4a/enemies.json')
if ($LASTEXITCODE -ne 0) { throw 'Original enemy fixture comparison failed' }
