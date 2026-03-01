# CryptoJS-Compatible AES-256-CBC Decryption in Rust

This document explains every concept involved in the `extract_subject_id` function and `evp_bytes_to_key` helper — from first principles to implementation details.

---

## The Problem

The attendance code passed via `--presensi-lama` was originally encrypted in **JavaScript using CryptoJS**:

```js
// JavaScript (the producer side)
const encrypted = CryptoJS.AES.encrypt("120184,extra,data", "utyjombor123").toString();
// produces: "U2FsdGVkX19xJlSDxwXghX/OIKvlFrU/..."
```

We need to **decrypt** this in Rust. The challenge: CryptoJS does not use a standard documented format — it uses OpenSSL's legacy `EVP_BytesToKey` key derivation and bundles the salt into the ciphertext in a specific way. If you don't replicate all of this exactly, decryption fails or produces garbage.

---

## Step 0: What is Encryption?

Encryption transforms readable data (plaintext) into unreadable data (ciphertext) using a key. Only someone with the key can reverse it back to plaintext.

```
plaintext + key → [encrypt] → ciphertext
ciphertext + key → [decrypt] → plaintext
```

We're using **AES-256-CBC** — the same algorithm CryptoJS uses by default.

---

## Step 1: What is AES?

**AES (Advanced Encryption Standard)** is a symmetric block cipher. "Symmetric" means the same key is used to both encrypt and decrypt.

Key facts:
- Operates on **fixed-size blocks of 16 bytes** at a time
- "256" in AES-256 = the key is **256 bits = 32 bytes** long
- AES itself only handles one 16-byte block — a **mode** (like CBC) is needed to handle longer data

---

## Step 2: What is CBC Mode?

**CBC (Cipher Block Chaining)** is a way to apply AES to multiple blocks of data.

The problem without CBC: if two plaintext blocks are identical, their ciphertext blocks would also be identical — revealing patterns in the data.

CBC fixes this by XOR-ing each plaintext block with the **previous ciphertext block** before encrypting:

```
block_1_ciphertext = AES(block_1_plaintext XOR IV)
block_2_ciphertext = AES(block_2_plaintext XOR block_1_ciphertext)
block_3_ciphertext = AES(block_3_plaintext XOR block_2_ciphertext)
...
```

The first block has no previous ciphertext, so it uses the **IV (Initialization Vector)** instead.

---

## Step 3: What is an IV?

The **IV (Initialization Vector)** is a random 16-byte value used as the "previous ciphertext" for the first block in CBC mode.

Why random? If the IV were always the same, two messages with the same key and same first block would produce identical first ciphertext blocks — again leaking patterns.

In CryptoJS, the IV is **derived from the password and salt** (via `EVP_BytesToKey`), not randomly generated per message. This is a known weakness of this approach, but it's what CryptoJS does.

---

## Step 4: What is PKCS7 Padding?

AES works on exactly 16-byte blocks. What if the plaintext is not a multiple of 16 bytes?

**PKCS7 padding** adds extra bytes at the end to fill the last block. The value of each padding byte equals the number of bytes added:

```
plaintext:  [H][e][l][l][o]           (5 bytes)
padded:     [H][e][l][l][o][11][11][11][11][11][11][11][11][11][11][11]
                              ↑ 11 bytes of padding, each with value 0x0B (= 11)
```

On decryption, PKCS7 removes these bytes. If the padding is invalid (corrupted data or wrong key), decryption returns an error.

In our code: `.decrypt_padded_mut::<Pkcs7>` — this is the `Pkcs7` padding scheme being applied during decryption.

---

## Step 5: The OpenSSL Format (What CryptoJS Produces)

When CryptoJS encrypts a string, the output is a **Base64-encoded binary blob** with this structure:

```
[0..8]   = "Salted__"  (ASCII literal, 8 bytes)
[8..16]  = salt        (8 random bytes)
[16..]   = ciphertext  (AES-256-CBC encrypted data)
```

This is the **OpenSSL "Salted" format** — a legacy format from OpenSSL's `enc` tool. CryptoJS adopted it for compatibility.

So the full encoded string:
```
Base64("Salted__" + salt + ciphertext)
```

---

## Step 6: What is Base64?

**Base64** is an encoding (not encryption) that represents binary data as printable ASCII characters.

Binary data (bytes 0–255) can't always be safely transmitted in text contexts (URLs, JSON, headers). Base64 maps every 3 bytes to 4 printable characters from the set `A-Z a-z 0-9 + /`.

```
binary: [0xFF][0x00][0xAB]
base64: "/wCr"
```

In our code:
```rust
base64::engine::general_purpose::STANDARD.decode(attendance_code)
```

`STANDARD` = standard Base64 alphabet (`+` and `/`), with `=` padding. This is what CryptoJS uses.

---

## Step 7: What is a Salt?

A **salt** is random bytes added to the password before hashing, to ensure that:
- Two encryptions of the same plaintext with the same password produce **different ciphertexts**
- Precomputed attack tables (rainbow tables) don't work

The salt is **not secret** — it's stored in the ciphertext (bytes 8–15 of the OpenSSL format). Its only purpose is to introduce randomness into the key derivation.

---

## Step 8: What is Key Derivation? (`EVP_BytesToKey`)

We have a human-readable password (`"utyjombor123"`) but AES needs:
- A **256-bit key** (32 bytes)
- An **IV** (16 bytes)

**Key derivation** is a function that stretches a short password into the right number of bytes.

OpenSSL's `EVP_BytesToKey` is the one CryptoJS uses. It works by repeatedly hashing:

```
D0 = MD5(password + salt)
D1 = MD5(D0 + password + salt)
D2 = MD5(D1 + password + salt)
...concatenate until you have 48 bytes...

key = first 32 bytes
IV  = next 16 bytes
```

Each round produces 16 bytes (MD5 output size). You need 48 bytes total (32 + 16), so 3 rounds.

---

## Step 9: What is MD5?

**MD5 (Message Digest 5)** is a hash function. It takes any amount of input and produces a fixed 16-byte (128-bit) output.

Key properties:
- Deterministic: same input always produces same output
- One-way: you can't reverse it
- Fixed output: always 16 bytes regardless of input

MD5 is **cryptographically broken** (collisions can be found) — but that's not relevant here. We're not using it for security; we're using it because that's exactly what `EVP_BytesToKey` specifies and what CryptoJS implements.

---

## Step 10: The Full Code Walkthrough

```rust
fn extract_subject_id(attendance_code: &str) -> Result<String, ValidationError> {
    let secret = b"utyjombor123";
```
`b"..."` is a **byte string literal** — a `&[u8]` (slice of raw bytes), not a `&str`. We need bytes because cryptographic functions work on raw bytes, not Unicode strings.

---

```rust
    let encrypted = base64::engine::general_purpose::STANDARD
        .decode(attendance_code)
        .map_err(|_| ValidationError::InvalidAttendanceCode)?;
```
Decode Base64 → `Vec<u8>` (raw bytes). If the input isn't valid Base64, return `InvalidAttendanceCode`.

---

```rust
    if encrypted.len() < 16 || &encrypted[..8] != b"Salted__" {
        return Err(ValidationError::InvalidAttendanceCode);
    }
```
Validate the OpenSSL format:
- Must be at least 16 bytes (`Salted__` header + salt)
- First 8 bytes must literally be `"Salted__"` in ASCII

`&encrypted[..8]` = slice from index 0 up to (not including) index 8.

---

```rust
    let salt = &encrypted[8..16];
    let ciphertext = &encrypted[16..];
```
Split the blob:
- `[8..16]` = bytes 8, 9, 10, 11, 12, 13, 14, 15 → the 8-byte salt
- `[16..]` = everything from byte 16 onward → the actual ciphertext

---

```rust
    let (key, iv) = evp_bytes_to_key(secret, salt);
```
Derive the 32-byte key and 16-byte IV from the password + salt. See `evp_bytes_to_key` walkthrough below.

---

```rust
    use aes::cipher::{BlockDecryptMut, KeyIvInit, block_padding::Pkcs7};
    type Aes256CbcDec = cbc::Decryptor<aes::Aes256>;
```
- `BlockDecryptMut` — trait that provides `.decrypt_padded_mut()`
- `KeyIvInit` — trait that provides `.new(key, iv)`
- `Pkcs7` — the padding scheme
- `Aes256CbcDec` — a type alias combining `cbc::Decryptor` with `aes::Aes256` as the underlying block cipher. This is the **RustCrypto** pattern for composing cipher + mode.

---

```rust
    let mut buf = ciphertext.to_vec();
    let decrypted = Aes256CbcDec::new(&key.into(), &iv.into())
        .decrypt_padded_mut::<Pkcs7>(&mut buf)
        .map_err(|_| ValidationError::InvalidAttendanceCode)?;
```
- `ciphertext.to_vec()` — copy into owned mutable buffer (decryption happens **in-place**)
- `&key.into()` — convert `[u8; 32]` into the `GenericArray<u8, U32>` type the crate expects (`.into()` does this automatically via `From` trait)
- `.decrypt_padded_mut::<Pkcs7>(&mut buf)` — decrypt in place, strip PKCS7 padding, return a `&[u8]` slice of the plaintext within `buf`
- If wrong key/IV or corrupted data → padding error → `InvalidAttendanceCode`

---

```rust
    let plaintext =
        std::str::from_utf8(decrypted).map_err(|_| ValidationError::InvalidAttendanceCode)?;
```
Convert raw bytes to a `&str`. AES produces bytes; we need to verify they're valid UTF-8 before treating them as a string. If decryption succeeded but produced garbage bytes, this catches it.

---

```rust
    let id = plaintext
        .split(',')
        .next()
        .ok_or(ValidationError::InvalidAttendanceCode)?;

    Ok(id.to_string())
```
The plaintext is formatted as `"120184,other,data"`. We only need the first segment (the subject ID). `.split(',').next()` gives `Some("120184")`. `.ok_or(...)` converts `Option` → `Result`.

---

## Step 11: `evp_bytes_to_key` Walkthrough

```rust
fn evp_bytes_to_key(password: &[u8], salt: &[u8]) -> ([u8; 32], [u8; 16]) {
    use md5::Digest;
    let mut d: Vec<u8> = Vec::new();
    let mut prev: Vec<u8> = Vec::new();
```
- `d` accumulates the derived bytes until we have 48
- `prev` holds the output of the previous MD5 round (starts empty for the first round)

---

```rust
    while d.len() < 48 {
        let mut hasher = md5::Md5::new();
        hasher.update(&prev);
        hasher.update(password);
        hasher.update(salt);
        prev = hasher.finalize().to_vec();
        d.extend_from_slice(&prev);
    }
```
Each iteration:
1. Create a fresh MD5 hasher
2. Feed in `prev` (empty on first round, previous 16-byte hash after that)
3. Feed in `password`
4. Feed in `salt`
5. Finalize → 16 bytes
6. Store as `prev` for next round
7. Append to `d`

After 3 rounds: `d` = 48 bytes.

---

```rust
    let mut key = [0u8; 32];
    let mut iv = [0u8; 16];
    key.copy_from_slice(&d[..32]);
    iv.copy_from_slice(&d[32..48]);
    (key, iv)
}
```
Split `d` into key (first 32 bytes) and IV (next 16 bytes). Return as fixed-size arrays — not `Vec`, because the AES crate expects `[u8; 32]` and `[u8; 16]`.

`[0u8; 32]` = array of 32 bytes, all initialized to zero. `u8` = unsigned 8-bit integer = one byte (0–255).

---

## Summary: Full Decryption Pipeline

```
Input:  "U2FsdGVkX19xJlSDxwXgh..."   ← Base64 string from CryptoJS

         ↓ Base64 decode

        [53 61 6c 74 65 64 5f 5f | 71 26 54 83 ... | <ciphertext bytes>]
         ↑ "Salted__" (8 bytes)    ↑ salt (8 bytes)   ↑ ciphertext

         ↓ EVP_BytesToKey(password="utyjombor123", salt)
           → Round 1: MD5("" + password + salt) = D0 (16 bytes)
           → Round 2: MD5(D0 + password + salt) = D1 (16 bytes)
           → Round 3: MD5(D1 + password + salt) = D2 (16 bytes)
           → key = D0 + D1 (32 bytes)
           → IV  = D2       (16 bytes)

         ↓ AES-256-CBC decrypt(ciphertext, key, IV) + remove PKCS7 padding

        "120184,extra,data"          ← plaintext

         ↓ split(',').next()

Output: "120184"                     ← subject ID
```
