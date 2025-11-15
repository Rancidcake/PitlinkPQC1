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
$exe = ".\rust_pqc\target\release\rust_pqc.exe"
if (Test-Path $exe) {
    Copy-Item -Path $exe -Destination "$DeployDir\bin\rust_pqc.exe" -Force
    Write-Host "Copied binary to $DeployDir\bin\"
} else {
    Write-Error "Binary not found: $exe. Run 'cargo build --release' first."
    exit 1
}

# Copy scripts
@("batch_encrypt.ps1", "batch_decrypt.ps1", "smoke_test.ps1") | ForEach-Object {
    $src = ".\rust_pqc\$_"
    if (Test-Path $src) {
        Copy-Item -Path $src -Destination "$DeployDir\bin\$_" -Force
        Write-Host "Copied: $_"
    }
}

# Copy config template
if (Test-Path ".\config.json.example") {
    Copy-Item -Path ".\config.json.example" -Destination "$DeployDir\config\config.json.example" -Force
    Write-Host "Copied config template"
}

# Copy integration guide
if (Test-Path ".\PRODUCTION_INTEGRATION.md") {
    Copy-Item -Path ".\PRODUCTION_INTEGRATION.md" -Destination "$DeployDir\PRODUCTION_INTEGRATION.md" -Force
    Write-Host "Copied production guide"
}

# Set permissions (Windows)
Write-Host "Setting folder permissions..."
icacls "$DeployDir\keys" /grant:r "${env:USERNAME}:F" /inheritance:r | Out-Null
icacls "$DeployDir\logs" /grant:r "${env:USERNAME}:F" /inheritance:r | Out-Null

Write-Host "`nDeployment complete!"
Write-Host "Next steps:"
Write-Host "1. Review $DeployDir\config\config.json.example"
Write-Host "2. Generate keypairs: cd $DeployDir\bin && .\rust_pqc.exe keygen --outdir ..\keys\your_name"
Write-Host "3. Test: .\smoke_test.ps1"
Write-Host "4. Use .\batch_encrypt.ps1 and .\batch_decrypt.ps1 for production workflows"
