# Inject Presensi UTY

Tool CLI untuk inject presensi di SIA UTY (ril no fek).
Script ini harom untuk dibisniskan!

> [!WARNING]
> Use at your own risk!

---

## Instalasi

Tidak perlu install Rust atau compiler apapun. Download file yang sesuai dengan OS-mu dari halaman [Releases](../../releases/latest).

| OS | File |
|---|---|
| Windows 64-bit | `presensi-uty-windows-x86_64.exe` |
| Linux 64-bit | `presensi-uty-linux-x86_64` |
| macOS (semua, termasuk Intel via Rosetta 2) | `presensi-uty-macos-aarch64` |

### Windows

Download filenya, lalu jalankan lewat Command Prompt atau PowerShell:

```powershell
.\presensi-uty-windows-x86_64.exe --help
```

### Linux & macOS

```bash
# Beri izin eksekusi terlebih dahulu
chmod +x presensi-uty-linux-x86_64

# Jalankan
./presensi-uty-linux-x86_64 --help
```

> **macOS only:** Jika muncul peringatan _"tidak dapat dibuka karena developer tidak diverifikasi"_, jalankan perintah ini sekali:
> ```bash
> xattr -dr com.apple.quarantine ./presensi-uty-macos-*
> ```

---

## Cara Penggunaan

```
presensi-uty [OPTIONS] -m <MINGGU> -n <NIM> -p <PASSWORD>
```

### Opsi

| Opsi | Keterangan |
|---|---|
| `--id-matkul <ID>` | ID mata kuliah (6 digit angka). Gunakan ini **atau** `--presensi-lama` |
| `--presensi-lama <KODE>` | Kode presensi dari QR lama (format terenkripsi). Gunakan ini **atau** `--id-matkul` |
| `-m`, `--minggu <ANGKA>` | Minggu yang akan diisi, pisahkan dengan koma. Contoh: `1,2,3`. Rentang valid: 1–14 |
| `-n`, `--nim <NIM>` | NIM kamu |
| `-p`, `--password <PASSWORD>` | Password SIA kamu |

### Contoh

```bash
# Mengisi presensi minggu 1, 2, dan 3 menggunakan ID matkul
presensi-uty --id-matkul 123456 -n 5220411272 -p passwordkamu -m 1,2,3

# Mengisi hanya minggu 5 menggunakan kode presensi lama
presensi-uty --presensi-lama "U2FsdGVkX1+..." -n 5220411272 -p passwordkamu -m 5
```

---

## Cara Mendapatkan ID Matkul

ID matkul adalah angka 6 digit yang bisa kamu temukan saat masa pengisian KRS.

1. Pastikan sedang dalam **masa pengisian KRS**
2. Buka halaman **Kartu Rencana Studi** di [sia.uty.ac.id](https://sia.uty.ac.id)
3. Klik **Tambah Mata Kuliah**
4. Klik kanan di nama mata kuliah → **Inspect** (atau tekan `F12`)
4. Cari atribut seperti `value="123456"` — angka itulah ID matkulnya
