# Quick start guide for using PQC encryption in your smart file transfer system

## 1. Deploy the system

```powershell
cd C:\Users\KUSHAGRA\Desktop\trs\PitlinkPQC
.\deploy.ps1  # or specify custom path: .\deploy.ps1 -DeployDir "D:\FileTransfer"
```

This creates the full directory structure with binaries and scripts ready to use.

## 2. Generate keypairs

```powershell
cd C:\PQC_FileTransfer\bin

# Generate a keypair for yourself
.\rust_pqc.exe keygen --outdir ..\keys\my_name

# Generate for other recipients
.\rust_pqc.exe keygen --outdir ..\keys\alice
.\rust_pqc.exe keygen --outdir ..\keys\bob
```

Keys are stored in `..\keys\<name>\`:
- `kyber_public.key` — share this with others or use for encryption
- `kyber_private.key` — keep this secret; use for decryption

## 3. Encrypt files before sending

```powershell
# Single file
.\rust_pqc.exe encrypt \
  --input "C:\Documents\secret.pdf" \
  --output "C:\PQC_FileTransfer\encrypted\secret.pdf.rkpq" \
  --pubkey "..\keys\alice\kyber_public.key"

# Batch (all files in a folder)
.\batch_encrypt.ps1 \
  -InputDir "..\staging" \
  -OutputDir "..\encrypted" \
  -PublicKeyPath "..\keys\alice\kyber_public.key"
```

The `.rkpq` file is now encrypted and safe to upload through your file transfer system.

## 4. Decrypt files after receiving

```powershell
# Single file
.\rust_pqc.exe decrypt \
  --input "C:\Downloads\secret.pdf.rkpq" \
  --output "C:\PQC_FileTransfer\decrypted\secret.pdf" \
  --privkey "..\keys\my_name\kyber_private.key"

# Batch (all encrypted files)
.\batch_decrypt.ps1 \
  -InputDir "..\encrypted" \
  -OutputDir "..\decrypted" \
  -PrivateKeyPath "..\keys\my_name\kyber_private.key"
```

The decrypted file is recovered and ready to use.

## 5. Verify everything works

```powershell
.\smoke_test.ps1  # Should show: SMOKE TEST: OK - files match
```

## Integration with your file transfer service

### Workflow 1: Encrypt before upload
```
My System
   ↓
[File] → [Encryption CLI] → [Encrypted .rkpq file] → [Upload to Transfer Service]
```

### Workflow 2: Decrypt after download
```
Transfer Service
   ↓
[Download .rkpq] → [Decryption CLI] → [Original File] → [Use]
```

### Workflow 3: Batch processing
```
Staging Folder (files ready to encrypt)
   ↓
[batch_encrypt.ps1] → [Encrypted Folder]
   ↓
[Upload all .rkpq files]
```

## Performance expectations

| File Size | Encrypt Time | Throughput |
|-----------|--------------|-----------|
| 1 KB | ~31 ms | 0.03 MB/s |
| 1 MB | ~33 ms | 30 MB/s |
| 10 MB | ~50 ms | 201 MB/s |
| 100 MB | ~500 ms | Expected 200+ MB/s |

For small files, setup overhead dominates. For large files, expect 175–200 MB/s.

## Security tips

1. **Keep private keys safe**
   - Store in a secure folder with restricted access
   - Back them up securely (encrypted, offline backup)
   - Never share private keys

2. **Share public keys securely**
   - Use secure channels (email, secure messaging, in-person)
   - Verify the key fingerprint with recipient

3. **Monitor transfers**
   - Check logs in `C:\PQC_FileTransfer\logs\`
   - Enable alerts for failed encryptions/decryptions

4. **Rotate keys periodically**
   - Every 12 months or after key compromise
   - Re-encrypt old files with new keys (keep old keys to decrypt old files)

## Troubleshooting

| Problem | Solution |
|---------|----------|
| "Binary not found" | Run `deploy.ps1` first to set up the system |
| "Permission denied" | Check file/folder permissions; ensure you have read/write access |
| "Decryption failed" | Verify you're using the correct private key for that encrypted file |
| "Files don't match after decrypt" | Check file wasn't corrupted during transfer; re-download and try again |
| Slow encryption | Check CPU usage; may be I/O-bound if using slow disk (use SSD for best performance) |

## Next steps

1. **Integrate into your transfer service**: Hook the CLI commands into your upload/download workflows
2. **Set up batch jobs**: Use Task Scheduler to run batch_encrypt.ps1/batch_decrypt.ps1 on schedules
3. **Enable monitoring**: Collect logs and metrics for SLA/compliance
4. **Test failover**: Ensure you can decrypt old files even after key rotation

See `PRODUCTION_INTEGRATION.md` for detailed integration architecture and advanced topics.
