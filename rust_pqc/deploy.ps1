# Deployment setup script for production environment
param(
    [string]$DeployDir = "C:\PQC_FileTransfer"
)

Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'

Write-Host "Setting up PQC File Transfer System in $DeployDir"

# Create directory structure
@(
    $DeployDir,
    "$DeployDir\bin",
    "$DeployDir\keys",
    "$DeployDir\staging",
    "$DeployDir\encrypted",
    "$DeployDir\decrypted",
    "$DeployDir\logs",
    "$DeployDir\metrics",
    "$DeployDir\config"
) | ForEach-Object {
    if (-not (Test-Path $_)) {
        New-Item -ItemType Directory -Path $_ -Force | Out-Null
        Write-Host "Created: $_"
    }
}

# Copy binary
$exe = ".\target\release\rust_pqc.exe"
if (Test-Path $exe) {
    Copy-Item -Path $exe -Destination "$DeployDir\bin\rust_pqc.exe" -Force
    Write-Host "Copied binary to $DeployDir\bin\"
} else {
    Write-Error "Binary not found: $exe. Run 'cargo build --release' first."
    exit 1
}

# Copy scripts
@("batch_encrypt.ps1", "batch_decrypt.ps1", "smoke_test.ps1") | ForEach-Object {
    $src = ".\$_"
    if (Test-Path $src) {
        Copy-Item -Path $src -Destination "$DeployDir\bin\$_" -Force
        Write-Host "Copied: $_"
    }
}

# Copy config template and guides
$repoRoot = Split-Path -Parent (Split-Path -Parent $PSScriptRoot)
@("config.json.example", "PRODUCTION_INTEGRATION.md", "QUICKSTART.md") | ForEach-Object {
    $src = Join-Path $repoRoot $_
    if (Test-Path $src) {
        Copy-Item -Path $src -Destination "$DeployDir\$_" -Force
        Write-Host "Copied: $_"
    }
}

# Set permissions (Windows)
Write-Host "Setting folder permissions..."
icacls "$DeployDir\keys" /grant:r "${env:USERNAME}:F" /inheritance:r 2>$null | Out-Null
icacls "$DeployDir\logs" /grant:r "${env:USERNAME}:F" /inheritance:r 2>$null | Out-Null

Write-Host "`nDeployment complete!"
Write-Host "Directory structure created in: $DeployDir"
Write-Host "`nNext steps:"
Write-Host "1. Review: $DeployDir\config\config.json.example"
Write-Host "2. Generate keypairs: cd $DeployDir\bin && .\rust_pqc.exe keygen --outdir ..\keys\your_name"
Write-Host "3. Test: .\smoke_test.ps1"
Write-Host "4. Use .\batch_encrypt.ps1 and .\batch_decrypt.ps1 for workflows"
