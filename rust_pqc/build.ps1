# Build release binary and copy artifact for CI (optimized RUSTFLAGS)
param(
    [string]$OutDir = "artifacts"
)

Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'

# Set RUSTFLAGS to enable CPU tuning and optimizations on the builder machine
$env:RUSTFLAGS = "-C target-cpu=native"
Write-Host "RUSTFLAGS=$env:RUSTFLAGS"

Write-Host "Building release..."
cargo build --release

if (-Not (Test-Path $OutDir)) { New-Item -ItemType Directory -Path $OutDir | Out-Null }

$exe = Join-Path -Path "target\release" -ChildPath "rust_pqc.exe"
if (-Not (Test-Path $exe)) { Write-Error "Release binary not found: $exe"; exit 1 }

Copy-Item -Path $exe -Destination (Join-Path $OutDir "rust_pqc.exe") -Force
Write-Host "Copied release binary to $OutDir\rust_pqc.exe"
Write-Host "Done."