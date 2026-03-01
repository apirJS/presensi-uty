# Inject Presensi UTY

Tool CLI untuk inject presensi di SIA UTY secara otomatis dari terminal.

> [!WARNING]
> Gunakan dengan bijak!

---

## Instalasi

Tidak perlu install Rust atau compiler apapun. Cukup download file yang sesuai dengan sistem operasimu dari halaman [Releases](../../releases/latest).

| Sistem Operasi | File yang didownload |
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

> **macOS saja:** Jika muncul peringatan _"tidak dapat dibuka karena developer tidak diverifikasi"_, jalankan perintah ini sekali:
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

# Mengisi semua minggu sekaligus
presensi-uty --id-matkul 123456 -n 5220411272 -p passwordkamu -m 1,2,3,4,5,6,7,8,9,10,11,12,13,14
```

---

## Cara Mendapatkan ID Matkul

ID matkul adalah angka 6 digit yang bisa kamu temukan saat masa pengisian KRS.

1. Pastikan sedang dalam **masa pengisian KRS**
2. Buka halaman **Kartu Rencana Studi** di [sia.uty.ac.id](https://sia.uty.ac.id)
3. Klik **Tambah Mata Kuliah**
4. Klik kanan di nama mata kuliah → **Inspect** (atau tekan `F12`)
4. Cari atribut seperti `value="123456"` — angka itulah ID matkulnya

---

## Cara Mendapatkan Kode Presensi Lama

Jika kamu punya kode QR presensi dari sistem lama UTY, kode terenkripsinya bisa langsung digunakan sebagai nilai `--presensi-lama`. Salin teks lengkap yang dimulai dengan `U2FsdGVk...`.
