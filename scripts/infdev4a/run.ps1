#requires -Version 7.0
$ErrorActionPreference = 'Stop'
$repo = (Resolve-Path "$PSScriptRoot/../..").Path
Set-Location $repo
$reference = Join-Path $repo '.reference/upstream.git'
$pin = '42ba9192fcc229bcf615453a6504d415c777feaf'
$output = Join-Path $repo 'target/parity/infdev4a'
$source = Join-Path $output 'source'
$classes = Join-Path $output 'classes'
New-Item -ItemType Directory -Force $source,$classes | Out-Null
$paths = @('src/client/java/minicraft/util/Simplex.java', 'src/client/java/minicraft/level/noise/LevelNoise.java', 'src/client/java/minicraft/level/ChunkManager.java')
# Read the original blobs. git archive applies upstream eol attributes even
# with autocrlf=false; copying stdout bytes avoids altering the pinned sources.
foreach ($relative in $paths) {
    $destination = Join-Path $source $relative
    New-Item -ItemType Directory -Force ([IO.Path]::GetDirectoryName($destination)) | Out-Null
    $info = [Diagnostics.ProcessStartInfo]::new('git')
    $info.UseShellExecute = $false
    $info.RedirectStandardOutput = $true
    foreach ($arg in @("--git-dir=$reference",'cat-file','blob',"${pin}:$relative")) { $info.ArgumentList.Add($arg) }
    $process = [Diagnostics.Process]::Start($info)
    try {
        $file = [IO.File]::Create($destination)
        try { $process.StandardOutput.BaseStream.CopyTo($file) } finally { $file.Dispose() }
        $process.WaitForExit()
        if ($process.ExitCode -ne 0) { throw "Cannot read pinned blob $relative" }
    } finally { $process.Dispose() }
}
. "$repo/scripts/parity224/verify-source.ps1"
Assert-ReferencePin $reference $pin $source @($paths | ForEach-Object { Get-Item (Join-Path $source $_) })
$manager = Get-Content -LiteralPath (Join-Path $source $paths[2]) -Raw
$size = [regex]::Match($manager,'public static final int CHUNK_SIZE = (\d+);').Groups[1].Value
if ($size -ne '64') { throw 'Unexpected source chunk size' }
& javac -encoding UTF-8 -d $classes (Join-Path $source $paths[0]) (Join-Path $source $paths[1]) "$PSScriptRoot/Oracle.java"
if ($LASTEXITCODE -ne 0) { throw 'Original Infdev noise compilation failed' }
& java -cp $classes infdev4a.Oracle (Join-Path $output 'noise-coordinates.tsv') $size
if ($LASTEXITCODE -ne 0) { throw 'Original Infdev noise execution failed' }
