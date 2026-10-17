# xerj-tui

> **Disclaimer:** xerj-tui adalah tool komunitas yang **tidak resmi** dan tidak berafiliasi dengan [xerj-org](https://github.com/xerj-org/xerj). XERJ berlisensi [Apache-2.0](https://github.com/xerj-org/xerj/blob/main/LICENSE).

Menu terminal untuk memakai [XERJ](https://github.com/xerj-org/xerj) (mesin pencari lokal untuk AI coding agent) tanpa menghafal perintah.

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

## Install

Download binary dari [Releases](https://github.com/latifismail53/xerj-tui/releases), lalu jalankan:

```bash
./xerj-tui
```

Atau build sendiri (butuh [Rust](https://sh.rustup.rs)):

```bash
cargo build --release
./target/release/xerj-tui   # satu binary ±3 MB, tanpa runtime dependency
```

Butuh: macOS atau Linux, terminal minimal 80x24.

## Cara pakai

1. **Install XERJ** (menu 1) — menjalankan installer resmi.
2. **Start server** (menu 2) — jalan di `http://localhost:9200`.
3. **Index project** (menu 3) — paste path folder, Enter.
4. **Init project** (menu 4) — menulis `.mcp.json` + skill di folder project agar Claude Code/Cursor bisa memakai XERJ.

## Fitur

- Deteksi otomatis: kalau `xerj` sudah terinstall, menu install nonaktif dan versinya ditampilkan.
- Panel status: versi, URL server/console, lokasi API key, data-dir, dan daftar korpus ter-index dalam satu kartu.
- Start/stop server sekali klik, plus auto-start saat login (macOS).
- Cari kode, go-to-definition, dan statistik pemakaian dari dalam menu.
- Semua aksi yang mengubah sesuatu meminta konfirmasi dulu.
- Menu opsional: kirim field report ke komunitas XERJ.

## Struktur kode

```
src/
├── main.rs   # entry point
├── core.rs   # logika: deteksi, parsing, manajemen server, perintah xerj
├── app.rs    # state machine + layar konfirmasi
└── ui.rs     # rendering ratatui
```

## Keamanan

- Tidak memodifikasi binary, data, atau key XERJ. Tidak ada menu hapus/reset.
- File milik TUI hanya di `~/.xerj-tui/` (PID server, state, log).
- Server jalan dengan `--insecure` (tanpa TLS/auth), hanya bind `127.0.0.1`, sesuai rekomendasi XERJ untuk pemakaian pribadi di laptop sendiri.
- Data-dir default `~/xerj-data`, selalu di luar folder yang di-index.

## FAQ

**Apakah dataku dikirim ke mana-mana?**
Tidak. XERJ jalan 100% lokal.

**Server tidak mau start?**
Pastikan port 9200 tidak dipakai aplikasi lain. Cek log di `~/.xerj-tui/xerj-server.log`.

**Cara update XERJ ke versi baru?**
Jalankan ulang menu Install.

## Lisensi

MIT — lihat [LICENSE](LICENSE). Repo ini bukan fork XERJ dan tidak membundel binary XERJ; instalasi selalu via installer resmi.
