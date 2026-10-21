# Shared stable-Rust configuration for local validation, CI and packaging.
# Dot-source in a dedicated PowerShell process; never print private prefixes.
$releaseRepo = [IO.Path]::GetFullPath((Join-Path $PSScriptRoot '..'))
$releaseSysroot = (& rustc --print sysroot).Trim()
if ($LASTEXITCODE -ne 0) { throw 'Cannot locate Rust toolchain for path remapping.' }
$releaseMappings = [ordered]@{
    $HOME = 'build/home'
    $releaseRepo = 'build/project'
    $releaseSysroot = 'build/toolchain'
}
$releaseCargoHome = if ($env:CARGO_HOME) { $env:CARGO_HOME } else { Join-Path $HOME '.cargo' }
$releaseRustupHome = if ($env:RUSTUP_HOME) { $env:RUSTUP_HOME } else { Join-Path $HOME '.rustup' }
$releaseMappings[$releaseCargoHome] = 'build/cargo'
$releaseMappings[$releaseRustupHome] = 'build/rustup'
# Cargo's plain RUSTFLAGS uses whitespace splitting; encoded flags preserve
# spaces within individual arguments. Respect Cargo's environment precedence.
$releaseCallerFlags = @()
if ($env:CARGO_ENCODED_RUSTFLAGS) { $releaseCallerFlags = @($env:CARGO_ENCODED_RUSTFLAGS.Split([char]31)) }
elseif ($env:RUSTFLAGS) { $releaseCallerFlags = @($env:RUSTFLAGS -split '\s+' | Where-Object { $_ }) }
elseif ($env:CARGO_BUILD_RUSTFLAGS) { $releaseCallerFlags = @($env:CARGO_BUILD_RUSTFLAGS -split '\s+' | Where-Object { $_ }) }
$releaseRemaps = @($releaseMappings.GetEnumerator() | Sort-Object { $_.Key.Length } | ForEach-Object {
    $prefix = [IO.Path]::GetFullPath($_.Key).TrimEnd('\','/')
    "--remap-path-prefix=$prefix=$($_.Value)"
    "--remap-path-prefix=$($prefix.Replace('\','/'))=$($_.Value)"
})
$env:CARGO_ENCODED_RUSTFLAGS = ($releaseCallerFlags + $releaseRemaps) -join [char]31
$releaseConfigHash = [Convert]::ToHexString([Security.Cryptography.SHA256]::HashData(
    [Text.Encoding]::UTF8.GetBytes($env:CARGO_ENCODED_RUSTFLAGS))).ToLowerInvariant()
