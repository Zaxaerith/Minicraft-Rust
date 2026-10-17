#requires -Version 7.0
param(
    [string]$RuntimeDll,
    [switch]$Replace
)
# Build separately. This script only packages an existing, clean Windows build.
Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'
if (-not $IsWindows) { throw 'This RC supports Windows x64 only.' }
$repo = [IO.Path]::GetFullPath((Join-Path $PSScriptRoot '..'))
$output = Join-Path $repo 'target/rc/java230-dev2'
$stem = 'Minicraft-Rust-java230-dev2-windows-x64-rc'
$archivePath = Join-Path $output "$stem.zip"
$binary = Join-Path $repo 'target/release/minicraft-rust.exe'
$revision = (& git -C $repo rev-parse HEAD).Trim()
if ($LASTEXITCODE -ne 0) { throw 'Cannot identify source revision.' }
$changes = @(& git -C $repo status --porcelain)
if ($LASTEXITCODE -ne 0 -or $changes.Count) { throw 'Commit or preserve working changes before packaging a clean RC.' }
if (Test-Path -LiteralPath $archivePath) {
    if (-not $Replace) { throw "RC already exists: $archivePath (use -Replace explicitly)." }
}

function Assert-X64Pe([string]$Path) {
    $stream = [IO.File]::OpenRead($Path)
    $reader = [IO.BinaryReader]::new($stream)
    try {
        if ($reader.ReadUInt16() -ne 0x5a4d) { throw "Not a PE file: $Path" }
        $stream.Position = 0x3c
        $offset = $reader.ReadInt32()
        if ($offset -lt 0 -or $offset -gt $stream.Length - 6) { throw 'Invalid PE offset.' }
        $stream.Position = $offset
        if ($reader.ReadUInt32() -ne 0x4550 -or $reader.ReadUInt16() -ne 0x8664) {
            throw "Expected x64 PE: $Path"
        }
    } finally { $reader.Dispose(); $stream.Dispose() }
}
function Get-Digest([byte[]]$Bytes) {
    $sha = [Security.Cryptography.SHA256]::Create()
    try { [BitConverter]::ToString($sha.ComputeHash($Bytes)).Replace('-','').ToLowerInvariant() }
    finally { $sha.Dispose() }
}
Assert-X64Pe $binary

# Select only release x64 redist files, never a DLL copied from System32.
if (-not $RuntimeDll) {
    $compiler = (Get-Command cl.exe -ErrorAction Stop).Source
    $marker = $compiler.IndexOf('\VC\Tools\MSVC\', [StringComparison]::OrdinalIgnoreCase)
    if ($marker -lt 0) { throw 'Pass -RuntimeDll pointing to the licensed VS x64 release redist.' }
    $redist = Join-Path $compiler.Substring(0, $marker) 'VC/Redist/MSVC'
    $versions = @(Get-ChildItem -LiteralPath $redist -Directory |
        Where-Object Name -Match '^\d+(\.\d+)+$' |
        Sort-Object { [version]$_.Name } -Descending)
    foreach ($version in $versions) {
        $candidate = @(Get-ChildItem -Path (Join-Path $version.FullName 'x64/Microsoft.VC*.CRT/vcruntime140.dll') -File)
        if ($candidate.Count -eq 1) { $RuntimeDll = $candidate[0].FullName; break }
    }
}
if (-not $RuntimeDll) { throw 'No licensed x64 VCRUNTIME140 redist found.' }
$runtime = (Get-Item -LiteralPath $RuntimeDll).FullName
if ($runtime -notmatch '[\\/]VC[\\/]Redist[\\/]MSVC[\\/][\d.]+[\\/]x64[\\/]Microsoft\.VC\d+\.CRT[\\/]vcruntime140\.dll$') {
    throw 'RuntimeDll must come from the VS release x64 CRT redist directory.'
}
Assert-X64Pe $runtime
$runtimeInfo = (Get-Item -LiteralPath $runtime).VersionInfo
$signature = Get-AuthenticodeSignature -LiteralPath $runtime
if ($runtimeInfo.IsDebug -or $runtimeInfo.IsPreRelease -or $signature.Status -ne 'Valid' -or
    $signature.SignerCertificate.Subject -notmatch 'O=Microsoft Corporation') {
    throw 'Runtime must be an unmodified, signed Microsoft release DLL.'
}

# Fail closed when a future build introduces another non-system DLL dependency.
$dumpbin = (Get-Command dumpbin.exe -ErrorAction Stop).Source
$dependencyReport = @(& $dumpbin /nologo /dependents $binary)
if ($LASTEXITCODE -ne 0) { throw 'Cannot inspect executable imports.' }
$imports = @($dependencyReport | ForEach-Object {
    if ($_ -match '^\s*([\w.-]+\.dll)\s*$') { $Matches[1].ToLowerInvariant() }
} | Sort-Object -Unique)
if (-not $imports.Count -or $imports -notcontains 'vcruntime140.dll') { throw 'Unexpected dependency report.' }
$systemDlls = @('kernel32.dll','ntdll.dll','oleaut32.dll','ole32.dll','avrt.dll',
    'bcryptprimitives.dll','user32.dll','gdi32.dll','opengl32.dll','ws2_32.dll')
foreach ($name in $imports) {
    if ($name -ne 'vcruntime140.dll' -and $name -notin $systemDlls -and $name -notlike 'api-ms-win-*.dll') {
        throw "Unpackaged runtime dependency: $name"
    }
}
$runtimeReport = @(& $dumpbin /nologo /dependents $runtime)
if ($LASTEXITCODE -ne 0) { throw 'Cannot inspect CRT imports.' }
foreach ($line in $runtimeReport) {
    if ($line -match '^\s*([\w.-]+\.dll)\s*$') {
        $name = $Matches[1].ToLowerInvariant()
        if ($name -notin $systemDlls -and $name -notlike 'api-ms-win-*.dll') {
            throw "Additional CRT dependency needs review: $name"
        }
    }
}

$sysroot = (& rustc --print sysroot).Trim()
if ($LASTEXITCODE -ne 0) { throw 'Cannot locate the installed Rust runtime notices.' }
$files = [Collections.Generic.SortedDictionary[string,byte[]]]::new([StringComparer]::Ordinal)
function Add-File([string]$Name, [string]$Source) {
    $files.Add($Name, [IO.File]::ReadAllBytes($Source))
}
function Add-Text([string]$Name, [string]$Text) {
    $files.Add($Name, [Text.UTF8Encoding]::new($false).GetBytes($Text.Replace("`r`n","`n")))
}
Add-File 'minicraft-rust.exe' $binary
Add-File 'vcruntime140.dll' $runtime
foreach ($name in @('LICENSE','README.md','THIRD_PARTY_NOTICES.md')) { Add-File $name (Join-Path $repo $name) }
Add-File 'CREDITS.txt' (Join-Path $repo 'assets/assets/books/credits.txt')
Add-File 'licenses/MICROSOFT-VC-RUNTIME-LICENSE.txt' (Join-Path $repo 'licenses/MICROSOFT-VC-RUNTIME-LICENSE.txt')
Add-File 'licenses/MICROSOFT-VC-RUNTIME-LICENSE.docx' (Join-Path $repo 'licenses/MICROSOFT-VC-RUNTIME-LICENSE.docx')
Add-File 'licenses/RUST-COPYRIGHT-library.html' (Join-Path $sysroot 'share/doc/rust/COPYRIGHT-library.html')
Add-File 'licenses/UPSTREAM-RESOURCES.md' (Join-Path $repo 'assets/expansion230/README.md')
Add-File 'licenses/EXPANSION230-MANIFEST.json' (Join-Path $repo 'assets/expansion230/manifest.json')
$lockHash = Get-Digest ([IO.File]::ReadAllBytes((Join-Path $repo 'Cargo.lock')))
$build = @(
    'label=java230-dev2-local-rc', 'platform=windows-x64-msvc', "source_revision=$revision",
    "cargo_lock_sha256=$lockHash", "binary_sha256=$(Get-Digest $files['minicraft-rust.exe'])",
    "vcruntime_file_version=$($runtimeInfo.FileVersion)", "vcruntime_sha256=$(Get-Digest $files['vcruntime140.dll'])",
    "rustc=$(& rustc -V)", "imports=$($imports -join ',')",
    'assets=embedded 2.2.4 originals and selected fixed dev2 expansion resources',
    'cargo_version=2.2.4 (unchanged; content label does not alter save formats)',
    'private_local_rc=true; no automatic publication or source-availability claim'
) -join "`n"
Add-Text 'BUILD-INFO.txt' ($build + "`n")
Add-Text 'RC-README.txt' @'
Minicraft-Rust — Java 2.3.0-dev2 content, Windows x64 local RC

Extract the entire folder. Double-click minicraft-rust.exe to play.
Requires a maintained Windows10/11 x64 installation and working graphics driver.
No Java, Rust, external assets, SDL2 or repository directory is needed to play.
Keep vcruntime140.dll beside the EXE; Windows supplies UCRT and system DLLs.

Default data: %APPDATA%\playminicraft\mods\Minicraft_Rust
For an isolated trial, open PowerShell here and run:
  .\minicraft-rust.exe --savedir ".\portable-data"
Self-check (no game window):
  .\minicraft-rust.exe --self-check --savedir ".\selfcheck-data"
See README.md for controls, Java224 copy-loading and dev2 transactional import.
Back up your data before upgrading; older EXEs may not read new saves.

Known limits: approximate AI, some original Java dungeon chests outside the map,
advanced Spawner pickup refused, and no complete multiplayer world synchronization.
This is a private local RC, not an upstream release or a universal parity claim.
Game/source/assets: GPL-3.0-only. Preserve LICENSE, credits and third-party notices.
Before redistribution supply exact corresponding source; see SOURCE.txt.
'@
Add-Text 'SOURCE.txt' @"
This local RC corresponds to source revision $revision in the local
Minicraft-Rust Git repository. No Push, Tag or online Release was performed.
The project URL is https://github.com/Zaxaerith/Minicraft-Rust; this does NOT
claim that this unpushed revision is available online. Before giving binaries
to others, provide the matching source, embedded resources, Cargo.lock and
build scripts under GPLv3 section6, with licenses and third-party terms.
This runtime ZIP intentionally contains no development tree or test saves.
Microsoft runtime terms are in licenses/MICROSOFT-VC-RUNTIME-LICENSE.txt.
Official redist guidance: https://learn.microsoft.com/en-us/visualstudio/releases/2026/redistribution
Runtime license source: https://aka.ms/VCRedistLicense
"@
$manifest = @($files.GetEnumerator() | ForEach-Object { "$(Get-Digest $_.Value)  $($_.Key)" }) -join "`n"
Add-Text 'MANIFEST.sha256' ($manifest + "`n")

# Explicit allowlist only; never recurse into a build, saves or reference tree.
[IO.Directory]::CreateDirectory($output) | Out-Null
$temporary = Join-Path $output ('.rc-' + [guid]::NewGuid().ToString('N') + '.tmp')
try {
    $stream = [IO.File]::Open($temporary, [IO.FileMode]::CreateNew, [IO.FileAccess]::Write)
    $zip = [IO.Compression.ZipArchive]::new($stream, [IO.Compression.ZipArchiveMode]::Create, $false)
    try {
        foreach ($entry in $files.GetEnumerator()) {
            $packed = $zip.CreateEntry("$stem/$($entry.Key)", [IO.Compression.CompressionLevel]::Optimal)
            $packed.LastWriteTime = [DateTimeOffset]::new(2000,1,1,0,0,0,[TimeSpan]::Zero)
            $content = $packed.Open()
            try { $content.Write($entry.Value, 0, $entry.Value.Length) } finally { $content.Dispose() }
        }
    } finally { $zip.Dispose(); $stream.Dispose() }
    [IO.File]::Move($temporary, $archivePath, $Replace.IsPresent)
} finally {
    # Delete only this exact exclusively-created temporary file, never a tree.
    if ([IO.File]::Exists($temporary)) { [IO.File]::Delete($temporary) }
}
$listing = @($files.GetEnumerator() | ForEach-Object {
    [ordered]@{ path="$stem/$($_.Key)"; bytes=$_.Value.Length; sha256=(Get-Digest $_.Value) }
})
$zipHash = Get-Digest ([IO.File]::ReadAllBytes($archivePath))
[IO.File]::WriteAllText((Join-Path $output "$stem.files.json"), ($listing | ConvertTo-Json -Depth 4))
[IO.File]::WriteAllText((Join-Path $output "$stem.zip.sha256"), "$zipHash  $stem.zip`n")
Write-Output "Created $archivePath"
Write-Output "Files=$($files.Count); bytes=$((Get-Item -LiteralPath $archivePath).Length); SHA256=$zipHash"
