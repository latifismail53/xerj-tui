# xerj-tui

> **Disclaimer:** xerj-tui adalah companion tool komunitas yang **TIDAK RESMI**. Tool ini tidak berafiliasi dan tidak di-endorse oleh xerj-org. XERJ adalah karya [xerj-org](https://github.com/xerj-org/xerj) berlisensi [Apache License 2.0](https://github.com/xerj-org/xerj/blob/main/LICENSE).

TUI (terminal UI) yang ramah untuk [XERJ](https://github.com/xerj-org/xerj), mesin pencari lokal untuk AI coding agent. Tidak perlu menghafal perintah, semua yang kamu butuhkan ada di menu.

```
 xerj-tui, pendamping ramah untuk XERJ
────────────────────────────────────────────────────────────────────────────────
┌ Status XERJ ─────────────────────────────────────────────────────────────────┐
│ versi   v1.0.0-rc.96      status ○ MATI                                      │
│  api     http://localhost:9200                                               │
│  console http://localhost:9200/_xerj-console/                                │
│  api key tidak diketahui                                                     │
│  data    -                                                                   │
│ korpus  -                                                                    │
└──────────────────────────────────────────────────────────────────────────────┘
┌ Menu ────────────────────────────────────────────────────────────────────────┐
│  1  Install XERJ   [v1.0.0-rc.96 terinstall]  ✓                              │
│› 2  Start server                                                             │
│  3  Index project   (butuh XERJ + server jalan)                              │
│  4  Init project untuk AI agent                                              │
│  5  Cari kode   (butuh XERJ + server jalan)                                  │
│  6  Go-to-definition   (butuh XERJ + server jalan)                           │
│  7  Statistik (xerj gain)   (butuh XERJ + server jalan)                      │
│  8  Auto-start saat login   (hanya macOS)                                    │
│  0  Kirim field report ke komunitas XERJ                                     │
│  9  Keluar                                                                   │
└──────────────────────────────────────────────────────────────────────────────┘
────────────────────────────────────────────────────────────────────────────────
       ↑↓ pilih · Enter jalankan · 0-9 jalan pintas · r refresh · q keluar
```

*(Screenshot asli dari render TUI. Server XERJ-nya benar-benar jalan di VM saat screenshot ini diambil.)*

## Kenapa ada tool ini?

Output mentah `xerj` itu berantakan buat pemula: log startup puluhan baris (info dashboard, lokasi key, dsb tercampur), perintah harus dihafal, dan server harus dinyalakan manual tiap boot. `xerj-tui` merapikan semuanya:

- **Panel Status XERJ**: satu kartu rapi berisi versi, URL server, URL console web, lokasi API key, data-dir, status server, dan daftar korpus yang sudah di-index. Semua field di-parse dari perilaku XERJ asli yang kami amati langsung, bukan tebakan.
- **Deteksi otomatis**: saat dibuka, TUI mengecek apakah `xerj` sudah terinstall. Kalau sudah, menu install otomatis disabled dan ada indikator ✓ plus versinya.
- **Manajemen server sekali klik**: Start/Stop tanpa buka terminal baru. PID dicatat di `~/.xerj-tui/server.pid`, status dicek via health endpoint `/_cluster/health`.
- **Auto-start (macOS)**: server jalan sendiri tiap login via LaunchAgent. Di Linux, menu ini terus terang bilang "belum didukung", tidak pura-pura jalan.

## Cara pakai

Butuh: macOS atau Linux, terminal 80x24 atau lebih besar.

```bash
# 1. Download binary (atau build sendiri, lihat bawah)
# 2. Jalankan:
/xerj-tui
```

Alur pertama kali:
1. Pilih **1 Install XERJ** untuk menjalankan installer resmi (`curl -fsSL https://xerj.org/get | sh`).
2. Pilih **2 Start server**. Server jalan di `http://localhost:9200`.
3. Pilih **3 Index project**, paste path folder (mis. `~/Development/erpv2`), Enter.
4. Pilih **4 Init project untuk AI agent** untuk menulis `.mcp.json` + skill di folder project, supaya Claude Code/Cursor bisa memakai XERJ.
5. Pilih **5 Cari kode** / **6 Go-to-definition** untuk mencoba.

Navigasi: `↑` `↓` / `Enter` / angka `1-9` / `r` refresh / `q` keluar.

## Cara build dari source

Butuh Rust toolchain ([rustup](https://sh.rustup.rs)):

```bash
cargo build --release
./target/release/xerj-tui   # satu binary ±3 MB, tanpa runtime dependency
```

Test:

```bash
cargo test
# 14 unit test (core) + 1 integration test (fake xerj shim + fake health
# server, tanpa install XERJ asli) + 4 UI smoke test (render via TestBackend)
```

## Struktur repo

```
xerj-tui/
├── Cargo.toml
├── src/
│   ├── main.rs   # entry point: setup terminal crossterm
│   ├── core.rs   # logika murni: deteksi, parsing, server, perintah xerj
│   ├── app.rs    # state machine: screen, menu, background task
│   └── ui.rs     # rendering ratatui (tema gelap, aksen cyan)
├── tests/
│   ├── integration_test.rs  # alur penuh dengan fake xerj shim
│   ├── ui_smoke_test.rs     # render tanpa terminal (TestBackend)
│   └── shim/xerj            # fake binary xerj untuk test
├── examples/
│   └── screenshot.rs        # render layar ke teks (untuk dokumentasi)
├── README.md
└── LICENSE
```

## Catatan teknis

- **Mode server: `--insecure`** (tanpa TLS & tanpa API key), sesuai rekomendasi XERJ untuk satu user di laptop sendiri. Server hanya bind `127.0.0.1`.
- **Data-dir default `~/xerj-data`**: selalu di luar folder project. TUI *menolak* meng-index kalau data-dir ada di dalam folder target, karena XERJ akan meng-index databasenya sendiri.
- **API key**: kalau server dijalankan *tanpa* `--insecure`, XERJ menulis `<data-dir>/admin.key` (mode 600). Kartu status menampilkan lokasinya kalau file-nya ada.
- **Daftar korpus** dibaca dari index `autoindex-catalog` (`doc_kind="dataset"`): nama index + jumlah rekaman + jumlah file.
- TUI tidak mengubah file project user selain yang diperintah menu (`xerj init` memang menulis `.mcp.json`, itu fungsinya).
- Repo ini **bukan fork** XERJ. Ini companion tool mandiri yang memanggil binary `xerj` resmi.

## Prinsip keamanan

TUI ini pembungkus pendamping. Prinsipnya satu: **dilarang merusak/mengubah XERJ**.

1. **Tidak pernah mem-patch, mengganti, atau memodifikasi binary `xerj`.** Hanya memanggil CLI resmi yang didokumentasikan: installer resmi, `--version`, `--insecure --data-dir`, `autoindex`, `init`, `search`, `def`, `gain`.
2. **Tidak pernah menghapus data-dir / index / key XERJ.** Tidak ada menu "reset/wipe" destruktif di TUI ini.
3. **File milik TUI** (PID server, state, log) disimpan di `~/.xerj-tui/`. File internal XERJ tidak diutak-atik.
4. **`xerj init`** menulis `.mcp.json` + `AGENTS.md` ke folder project: sebelum dijalankan, TUI menampilkan dulu apa yang akan ditulis dan meminta konfirmasi eksplisit (Enter = lanjut, Esc = batal). File `.mcp.json` yang sudah ada di-backup menjadi `.mcp.json.bak` oleh xerj sendiri. Perilaku ini sudah kami verifikasi langsung.
5. **Flag `--insecure` dijelaskan jujur di UI** (layar konfirmasi Start server): tanpa TLS dan tanpa API key. Itu hanya untuk satu pengguna di mesin sendiri (server bind `127.0.0.1`, sesuai security model resmi XERJ). Tidak disembunyikan.
6. **Semua aksi yang mengubah sesuatu di luar TUI** (install, start/stop server, autoindex, init, auto-start) menampilkan ringkasan "Yang akan dilakukan" dan meminta konfirmasi dulu. Aksi read-only (cek versi, status, search, def, gain) boleh langsung jalan.

## Palet warna

Teks putih/abu di atas background gelap terminal, border panel abu gelap. Aksen cyan dipakai di satu tempat saja: marker `›` pada menu yang sedang dipilih dan border popup modal. Warna lain hanya muncul sebagai penanda state yang nyata: hijau untuk OK/jalan, merah untuk error, kuning untuk peringatan. Tidak ada emoji dekoratif; glyph status (`✓` `●` `○`) hanya dipakai untuk state nyata. Semua angka yang tampil itu angka asli: versi dari `xerj --version`, jumlah korpus dari data katalog. Kalau belum ada data, empty state-nya bilang terus terang kenapa kosong dan apa yang harus dilakukan.

## FAQ

**Q: Perlu install apa saja?**
A: Cukup binary `xerj-tui` ini. XERJ-nya sendiri bisa di-install dari menu nomor 1 (installer resmi).

**Q: Apakah dataku dikirim ke mana-mana?**
A: Tidak. XERJ jalan 100% lokal di mesinmu. Tidak ada yang di-upload.

**Q: Autoindex gagal dengan pesan "verification failed"?**
A: Itu pesan dari XERJ sendiri; coba lagi, atau ulangi dengan fresh index. Kalau berulang, cek log server di `~/.xerj-tui/xerj-server.log`.

**Q: Server tidak mau start / health check gagal?**
A: Pastikan port 9200 tidak dipakai aplikasi lain. Cek log di `~/.xerj-tui/xerj-server.log`.

**Q: Bisa dipakai di Windows?**
A: Belum diuji. Secara kode tidak ada yang spesifik Unix kecuali manajemen PID/auto-start. PR welcome.

**Q: Bagaimana cara update XERJ ke versi baru?**
A: Jalankan ulang menu Install (installer resmi akan meng-overwrite binary lama).

## Lisensi

MIT, lihat [LICENSE](LICENSE). Lisensi ini hanya mencakup kode xerj-tui.

## Legal / Atribusi

Ringkasan kepatuhan (prinsip non-negotiable dari pemilik project):

1. **Kode kita**: [LICENSE](LICENSE) MIT, hanya untuk kode xerj-tui sendiri.
2. **XERJ bukan milik kita**: XERJ adalah karya [xerj-org](https://github.com/xerj-org/xerj) berlisensi [Apache License 2.0](https://github.com/xerj-org/xerj/blob/main/LICENSE). Lisensinya permisif, jadi companion tool independen seperti ini diperbolehkan. xerj-tui **tidak resmi**, tidak berafiliasi, tidak di-endorse oleh xerj-org.
3. **Tidak me-redistribute XERJ**: repo ini tidak membundel binary, source, dokumentasi, maupun aset brand (logo) XERJ. Instalasi XERJ selalu via installer resmi (`https://xerj.org/get`) saat runtime oleh pengguna. Semua referensi ke XERJ berupa link.
4. **Nama "xerj-tui"** adalah pemakaian nominatif/deskriptif. Artinya nama ini menjelaskan bahwa tool ini kompatibel dengan XERJ. Disclaimer di atas yang menjaganya tetap aman.
5. **Field report komunitas** (menu `0`): XERJ meminta setiap pengguna/agent mengirim satu field report sebagai "tiket masuk" komunitas. Ini **permintaan komunitas, BUKAN kewajiban hukum**. TUI menjelaskannya dengan jujur di layar konfirmasi, dan yang dijalankan hanya `xerj feedback --dry-run` (membuat draf, tidak mengirim apa-apa).
