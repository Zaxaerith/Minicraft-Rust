# Check every source/resource blob against the pinned Git tree in two Git calls.
# Batched hash-object avoids hundreds of process launches without normalizing bytes.
function Assert-ReferencePin($ReferenceGit, $Pin, $SourceRoot, $Files) {
    $tree = @(& git --git-dir=$ReferenceGit ls-tree -r $Pin)
    if ($LASTEXITCODE -ne 0) { throw 'Cannot read pinned reference tree' }
    $expected = [Collections.Generic.Dictionary[string,string]]::new([StringComparer]::Ordinal)
    foreach ($entry in $tree) {
        if ($entry -match '^[0-9]+ blob ([0-9a-f]+)\t(.+)$') { $expected.Add($Matches[2], $Matches[1]) }
    }
    $actual = @($Files | ForEach-Object { $_.FullName.Replace('\','/') } | & git hash-object --no-filters --stdin-paths)
    if ($LASTEXITCODE -ne 0 -or $actual.Count -ne $Files.Count) { throw 'Cannot hash all reference files' }
    for ($i = 0; $i -lt $Files.Count; $i++) {
        $relative = [IO.Path]::GetRelativePath($SourceRoot, $Files[$i].FullName).Replace('\','/')
        if (-not $expected.ContainsKey($relative) -or $actual[$i] -cne $expected[$relative]) {
            throw "Reference source differs from pin: $relative"
        }
    }
    Write-Output "Verified $($Files.Count) unfiltered source/resource blobs at $Pin"
}
