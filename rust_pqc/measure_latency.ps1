# Measure latency (ms) for keygen, encrypt and decrypt operations
Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'

$exe = Join-Path -Path "target\release" -ChildPath "rust_pqc.exe"
if (-Not (Test-Path $exe)) {
    Write-Host "Release binary missing, building..."
    cargo build --release
}

# ensure keys exist
$keysDir = "keys_latency"
if (Test-Path $keysDir) { Remove-Item -Recurse -Force $keysDir }
New-Item -ItemType Directory -Path $keysDir | Out-Null

Write-Host "Measuring keygen..."
$tg = Measure-Command { & $exe keygen --outdir $keysDir }
$keygen_ms = [math]::Round($tg.TotalMilliseconds,2)
Write-Host "Keygen time: $keygen_ms ms"

# sample sizes: 1KB, 1MB, 10MB
$sizes = @{ '1KB' = 1024; '1MB' = 1MB; '10MB' = 10MB }
$results = @()

foreach ($label in $sizes.Keys) {
    $size = $sizes[$label]
    $sample = "sample_${label}.bin"
    $enc = "${sample}.enc"
    $dec = "${sample}.dec"

    # create random sample
    Write-Host "Creating $label file ($size bytes) -> $sample"
    $b = New-Object byte[] $size
    [System.Security.Cryptography.RandomNumberGenerator]::Create().GetBytes($b)
    [IO.File]::WriteAllBytes($sample, $b)

    # encrypt
    Write-Host "Encrypting $sample -> $enc"
    $te = Measure-Command { & $exe encrypt --input $sample --output $enc --pubkey (Join-Path $keysDir 'kyber_public.key') }
    $enc_ms = [math]::Round($te.TotalMilliseconds,2)

    # decrypt
    Write-Host "Decrypting $enc -> $dec"
    $td = Measure-Command { & $exe decrypt --input $enc --output $dec --privkey (Join-Path $keysDir 'kyber_private.key') }
    $dec_ms = [math]::Round($td.TotalMilliseconds,2)

    # verify
    $ok = $false
    try {
        $h1 = (Get-FileHash $sample -Algorithm SHA256).Hash
        $h2 = (Get-FileHash $dec -Algorithm SHA256).Hash
        $ok = ($h1 -eq $h2)
    } catch { $ok = $false }

    $mb = [math]::Round($size / 1MB, 3)
    $enc_mbps = if ($enc_ms -gt 0) { [math]::Round($mb / ($enc_ms/1000.0), 3) } else { 0 }
    $dec_mbps = if ($dec_ms -gt 0) { [math]::Round($mb / ($dec_ms/1000.0), 3) } else { 0 }

    $results += [pscustomobject]@{
        Size = $label
        Bytes = $size
        Encrypt_ms = $enc_ms
        Encrypt_MBps = $enc_mbps
        Decrypt_ms = $dec_ms
        Decrypt_MBps = $dec_mbps
        Verified = $ok
    }

    # cleanup per-case intermediate files
    Remove-Item -Force $sample, $enc, $dec -ErrorAction SilentlyContinue
}

Write-Host "\nSummary (latencies in ms, throughput MB/s):"
$results | Format-Table -AutoSize

# final: cleanup keys
# Remove-Item -Recurse -Force $keysDir

# return results as JSON for programmatic consumption
$results | ConvertTo-Json -Compress > latency_results.json
Write-Host "Results saved to latency_results.json"
