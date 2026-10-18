#requires -Version 7.0
param([switch]$JavaOracle)
$ErrorActionPreference = 'Stop'
$repo = (Resolve-Path "$PSScriptRoot/..").Path
Set-Location $repo
$output = Join-Path $repo 'target/validation/current'
New-Item -ItemType Directory -Force $output | Out-Null
# Replace a previous PASS before work begins, so an interrupted run cannot
# accidentally inherit the previous run's result.
Set-Content -LiteralPath (Join-Path $output 'validation.txt') -Value 'status=running' -Encoding utf8
$env:RUST_TEST_THREADS = '4'
$env:MINICRAFT_PARITY_RENDER_OUTPUT = Join-Path $repo 'target/parity/render-frame/rust'

function Invoke-Check([string]$Name, [scriptblock]$Command) {
    Write-Output "Running $Name"
    & $Command *> (Join-Path $output "$Name.log")
    if ($LASTEXITCODE -ne 0) { throw "$Name failed: see $output/$Name.log" }
    Write-Output "PASS $Name"
}

# These are the existing gates and reference drivers, not a second test suite.
Invoke-Check 'fmt' { cargo fmt --all -- --check }
Invoke-Check 'clippy' { cargo clippy --workspace --all-targets --locked -- -D warnings }
Invoke-Check 'workspace-tests' { cargo test --release --workspace --locked }
Invoke-Check 'build' { cargo build --release --workspace --locked }
Invoke-Check 'selfcheck' { & ./target/release/minicraft-rust.exe --self-check --savedir target/validation/selfcheck }
if ($JavaOracle) {
    Invoke-Check 'oracle224' { & pwsh -NoProfile -NonInteractive -File scripts/parity224/run.ps1 }
    Invoke-Check 'oracle230' { & pwsh -NoProfile -NonInteractive -File scripts/parity230/run-world36.ps1 }
    Invoke-Check 'oracle-fixtures' { & python scripts/compare-java-oracles.py }
    foreach ($backend in @('java224-generation','java224-noise','java230-generation','java230-depths')) {
        $directory = if ($backend.StartsWith('java224')) { 'java224' } else { 'java230' }
        Invoke-Check $backend { & ./target/release/minicraft-parity.exe $backend "target/parity/$directory/output" }
    }
}
$revision = (& git rev-parse HEAD).Trim()
if ($LASTEXITCODE -ne 0) { throw 'Cannot identify validated revision' }
$dirty = [bool](& git status --porcelain)
@('status=pass', "revision=$revision", "dirty=$dirty", "java_oracle=$JavaOracle", "completed_utc=$([DateTime]::UtcNow.ToString('o'))") |
    Set-Content -LiteralPath (Join-Path $output 'validation.txt') -Encoding utf8
