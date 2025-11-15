# rust_pqc — Kyber-768 + XChaCha20-Poly1305 hybrid encryptor

This folder contains a Rust CLI implementing a hybrid post-quantum + symmetric file encryption flow.

Design summary
- Use Kyber-768 KEM to exchange a shared secret. (Encapsulate to recipient public key.)
- Derive a KEK from the Kyber shared secret with HKDF-SHA256.
- Generate a random 32-byte file key and wrap it with KEK using XChaCha20-Poly1305.
- Stream the file in 1 MiB chunks; each chunk is encrypted with XChaCha20-Poly1305 using the file key and a per-chunk nonce. Each chunk contains its own authentication tag so corrupted blocks can be detected and a transfer can resume at chunk boundaries.

Files added
- `Cargo.toml` — dependencies and crate metadata
- `src/main.rs` — CLI with subcommands `keygen`, `encrypt`, `decrypt`

Build notes (PowerShell)

```powershell
# Build (release)
cd rust_pqc
cargo build --release

# Generate keys
cargo run --release -- keygen --outdir keys

# Encrypt
cargo run --release -- encrypt --input ..\secret.bin --output ..\secret.bin.pqc --pubkey keys\kyber_public.key

# Decrypt
cargo run --release -- decrypt --input ..\secret.bin.pqc --output ..\secret_decrypted.bin --privkey keys\kyber_private.key
```

Caveats and platform notes
- The code targets crates from crates.io. The Kyber KEM API used in `src/main.rs` assumes a `pqcrypto_kem::kyber768` style API (functions like `keypair()`, `encapsulate()`, `decapsulate()` and types returning raw byte slices). Depending on the exact crate/version you pick you may need to adapt small API calls. Another option is to use `oqs` bindings (liboqs) if you prefer.
- Building may require linking to native libraries depending on the pqc crate. If you choose an `oqs` binding you'll need to install `liboqs` on your system first.
- This is a demonstration implementation. For production use:
  - get a cryptographic review
  - ensure constant-time operations for secret handling
  - use secure key storage (HSM or OS key store) for private keys
  - add authenticated metadata (file sizes, names)

Resilience for bad networks
- Per-chunk nonces and authentication tags allow corruption to be detected per-chunk and enable resumable transfers if you preserve chunk offsets.
# rust_pqc — Kyber-768 + XChaCha20-Poly1305 hybrid encryptor

This folder contains a Rust CLI implementing a hybrid post-quantum + symmetric file encryption flow.

Design summary
- Use Kyber-768 KEM to exchange a shared secret. (Encapsulate to recipient public key.)
- Derive a KEK from the Kyber shared secret with HKDF-SHA256.
- Generate a random 32-byte file key and wrap it with KEK using XChaCha20-Poly1305.
- Stream the file in 1 MiB chunks; each chunk is encrypted with XChaCha20-Poly1305 using the file key and a per-chunk nonce. Each chunk contains its own authentication tag so corrupted blocks can be detected and a transfer can resume at chunk boundaries.

Files added
- `Cargo.toml` — dependencies and crate metadata
- `src/main.rs` — CLI with subcommands `keygen`, `encrypt`, `decrypt`

Build notes (PowerShell)

```powershell
# Create and activate a venv (optional) and install Rust toolchain via rustup
# Build
cd rust_pqc
cargo build --release

# Run examples
# Generate keys
.
# Keygen
cargo run --release -- keygen --outdir keys

# Encrypt
cargo run --release -- encrypt --input ..\secret.bin --output ..\secret.bin.pqc --pubkey keys\kyber_public.key

# Decrypt
cargo run --release -- decrypt --input ..\secret.bin.pqc --output ..\secret_decrypted.bin --privkey keys\kyber_private.key
```

Caveats and platform notes
- The code targets crates from crates.io. The Kyber KEM API used in `src/main.rs` assumes a `pqcrypto_kem::kyber768` style API (functions like `keypair()`, `encapsulate()`, `decapsulate()` and types returning raw byte slices). Depending on the exact crate/version you pick you may need to adapt small API calls. Another option is to use `oqs` bindings (liboqs) if you prefer.
- Building may require linking to native libraries depending on the pqc crate. If you choose an `oqs` binding you'll need to install `liboqs` on your system first.
- This is a demonstration implementation. For production use:
  - get a cryptographic review
  - ensure constant-time operations for secret handling
  - use secure key storage (HSM or OS key store) for private keys
  - add authenticated metadata (file sizes, names)

Resilience for bad networks
- Per-chunk nonces and authentication tags allow corruption to be detected per-chunk and enable resumable transfers if you preserve chunk offsets.
