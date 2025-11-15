# Batch encrypt all files in a directory
param(
    [string]$InputDir = ".\staging",
    [string]$OutputDir = ".\encrypted",
    [string]$PublicKeyPath = ".\keys\kyber_public.key",
    [string]$LogFile = ".\logs\batch_encrypt.log"
)

Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'

# Ensure directories exist
@($OutputDir, (Split-Path $LogFile)) | ForEach-Object {
    if (-not (Test-Path $_)) { New-Item -ItemType Directory -Path $_ | Out-Null }
}

$exe = ".\target\release\rust_pqc.exe"
if (-not (Test-Path $exe)) { Write-Error "Binary not found: $exe"; exit 1 }

$files = Get-ChildItem -Path $InputDir -File
$total = $files.Count
$success = 0
$failed = 0

Write-Host "Batch encrypting $total files from $InputDir -> $OutputDir"
"$(Get-Date) - Starting batch encryption of $total files" | Out-File -FilePath $LogFile -Append

foreach ($file in $files) {
    $outfile = Join-Path $OutputDir "$($file.Name).rkpq"
    $start = Get-Date
    
    try {
        Write-Host "Encrypting: $($file.Name) ..."
        & $exe encrypt --input $file.FullName --output $outfile --pubkey $PublicKeyPath | Out-Null
        $end = Get-Date
        $elapsed = ($end - $start).TotalMilliseconds
        "$(Get-Date) OK [$($file.Name)] $elapsed ms" | Out-File -FilePath $LogFile -Append
        $success++
    } catch {
        $end = Get-Date
        $elapsed = ($end - $start).TotalMilliseconds
        "$(Get-Date) FAIL [$($file.Name)] $_" | Out-File -FilePath $LogFile -Append
        $failed++
        Write-Host "  ERROR: $_" -ForegroundColor Red
    }
}

Write-Host "`nBatch complete: $success succeeded, $failed failed"
"$(Get-Date) - Batch complete: $success succeeded, $failed failed" | Out-File -FilePath $LogFile -Append
