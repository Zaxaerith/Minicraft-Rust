param(
    [string]$Source = '.reference/release-2.2.x/minicraft-plus-revived-47227a89fe05f1e3fe962f070f25c53111254a1f',
    [string]$DependencyJar = '.reference/official/minicraft-plus-2.2.4.jar'
)
$ErrorActionPreference = 'Stop'
$repo = (Resolve-Path "$PSScriptRoot/../..").Path
Set-Location $repo
$sourceRoot = (Resolve-Path $Source).Path
$jar = (Resolve-Path $DependencyJar).Path
$referenceGit = Join-Path $repo '.reference/upstream.git'
$pin = '47227a89fe05f1e3fe962f070f25c53111254a1f'
# The jar supplies third-party dependencies only: compile ALL pinned sources,
# and put fresh classes + pinned resources before the jar on the classpath.
$sources = @(Get-ChildItem "$sourceRoot/src/client/java", "$sourceRoot/src/common/java" -Recurse -Filter '*.java')
$resources = @(Get-ChildItem "$sourceRoot/src/client/resources" -Recurse -File)
foreach ($file in @($sources) + @($resources)) {
    $relative = [IO.Path]::GetRelativePath($sourceRoot, $file.FullName).Replace('\','/')
    $expected = & git --git-dir=$referenceGit rev-parse "${pin}:$relative"
    $actual = & git hash-object --no-filters -- $file.FullName
    if ($LASTEXITCODE -ne 0 -or $actual -ne $expected) { throw "Reference source differs from pin: $relative" }
}
$outputRoot = Join-Path $repo 'target/parity/java224'
$classes = Join-Path $outputRoot 'classes'
$output = Join-Path $outputRoot 'output'
New-Item -ItemType Directory -Force -Path $classes,$output | Out-Null
$argsFile = Join-Path $outputRoot 'sources.args'
$sourceArgs = @($sources | ForEach-Object { '"' + $_.FullName.Replace('\','/') + '"' })
$sourceArgs += '"' + "$PSScriptRoot/Oracle.java".Replace('\','/') + '"'
[IO.File]::WriteAllLines($argsFile, $sourceArgs, [Text.UTF8Encoding]::new($false))
& javac -encoding UTF-8 -cp $jar -d $classes "@$argsFile"
if ($LASTEXITCODE -ne 0) { throw 'Pinned Java compilation failed' }
$separator = [IO.Path]::PathSeparator
$classpath = "$classes${separator}$sourceRoot/src/client/resources${separator}$jar"
Push-Location $outputRoot
try {
& java '-Djava.awt.headless=true' -cp $classpath minicraft.level.Oracle $output
$javaExit = $LASTEXITCODE
} finally { Pop-Location }
if ($javaExit -ne 0) { throw 'Java reference execution failed' }
Write-Output "Java $pin reference written to $output"
