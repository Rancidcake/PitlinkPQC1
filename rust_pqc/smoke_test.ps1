# Smoke test: keygen -> encrypt -> decrypt -> verify
Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'

# paths
$exe = Join-Path -Path "target\release" -ChildPath "rust_pqc.exe"
$keysDir = "keys"
$sample = "sample.txt"
$enc = "sample.enc"
$dec = "sample.dec"

Write-Host "Ensure release binary exists (will build if missing)..."
if (-Not (Test-Path $exe)) { cargo build --release }

Write-Host "Generating keys into $keysDir"
& $exe keygen --outdir $keysDir

Write-Host "Writing sample plaintext"
"hello pqc - $(Get-Date)" | Out-File -FilePath $sample -Encoding utf8

Write-Host "Encrypting $sample -> $enc"
& $exe encrypt --input $sample --output $enc --pubkey (Join-Path $keysDir "kyber_public.key")

Write-Host "Decrypting $enc -> $dec"
& $exe decrypt --input $enc --output $dec --privkey (Join-Path $keysDir "kyber_private.key")

Write-Host "Verifying files"
$h1 = (Get-FileHash $sample -Algorithm SHA256).Hash
$h2 = (Get-FileHash $dec -Algorithm SHA256).Hash
if ($h1 -eq $h2) { Write-Host "SMOKE TEST: OK - files match"; exit 0 } else { Write-Error "SMOKE TEST: FAIL - files differ"; exit 2 }