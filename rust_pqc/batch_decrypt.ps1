# Batch decrypt all encrypted files in a directory
param(
    [string]$InputDir = ".\encrypted",
    [string]$OutputDir = ".\decrypted",
    [string]$PrivateKeyPath = ".\keys\kyber_private.key",
    [string]$LogFile = ".\logs\batch_decrypt.log"
)

Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'

# Ensure directories exist
@($OutputDir, (Split-Path $LogFile)) | ForEach-Object {
    if (-not (Test-Path $_)) { New-Item -ItemType Directory -Path $_ | Out-Null }
}

$exe = ".\target\release\rust_pqc.exe"
if (-not (Test-Path $exe)) { Write-Error "Binary not found: $exe"; exit 1 }

$files = Get-ChildItem -Path $InputDir -Filter "*.rkpq" -File
$total = $files.Count
$success = 0
$failed = 0

Write-Host "Batch decrypting $total files from $InputDir -> $OutputDir"
"$(Get-Date) - Starting batch decryption of $total files" | Out-File -FilePath $LogFile -Append

foreach ($file in $files) {
    $outfile = Join-Path $OutputDir ($file.Name -replace '\.rkpq$', '')
    $start = Get-Date
    
    try {
        Write-Host "Decrypting: $($file.Name) ..."
        & $exe decrypt --input $file.FullName --output $outfile --privkey $PrivateKeyPath | Out-Null
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
