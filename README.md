# Filament Tray 🧵

[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](LICENSE)

A tiny cross-platform (Windows / macOS / Linux) **system tray app** that shows how much filament is left in each **AMS slot**, straight from [Spoolman](https://github.com/Donkie/Spoolman).

Companion to [bambu-spoolman-bridge](https://github.com/Yarasarda/bambu-spoolman-bridge), which automatically deducts used filament from Spoolman after every Bambu Lab print. The bridge keeps the numbers accurate, **Filament Tray** puts them one hover away.

> 🌍 [Türkçe açıklama aşağıda](#-türkçe)

## Features

- Tray icon popup: colour, name, remaining grams and percentage bar for **AMS 1–4**
- **Hover** (Windows, macOS) or click to open it
- Desktop notification when a spool drops below a threshold (default 100 g)
- Settings in the popup: Spoolman URL, threshold, refresh interval
- Built with Tauri 2 (Rust + plain HTML). No Node.js needed, small binary

## How slots are detected

A spool is shown in a slot when its Spoolman **Location** is `AMS 1`, `AMS 2`, `AMS 3` or `AMS 4` (`Slot 2`, `Yuva 3` or just `4` also work). This is the same convention the bridge uses, so both always agree.

## Build & run

```bash
# Linux deps (Arch): webkit2gtk-4.1 gtk3 libappindicator-gtk3 base-devel
cargo run            # dev
cargo build --release
```

On some Linux/Wayland setups use `WEBKIT_DISABLE_DMABUF_RENDERER=1 cargo run`.

Default Spoolman URL is `http://192.168.1.123:7912`; change it in the ⚙ settings.

## Platform notes

- **Windows / macOS:** hover and click both work.
- **Linux:** tray protocols (AppIndicator) emit no hover events, so use left-click or the right-click menu → *Show*.
- Only tested on Linux so far. Windows/macOS builds and testing are welcome (PRs/issues).

## 🇹🇷 Türkçe

Spoolman'daki **AMS yuvalarının** kalan filament miktarını sistem tepsisi ikonunda gösteren küçük, çok platformlu (Windows / macOS / Linux) bir uygulama.

Her Bambu baskısından sonra kullanılan filamenti Spoolman'dan otomatik düşen [bambu-spoolman-bridge](https://github.com/Yarasarda/bambu-spoolman-bridge) projesinin eşlikçisidir: köprü rakamları doğru tutar, bu uygulama onları ikonun üstüne getirir.

- İkonun üstüne gelince (Windows/macOS) ya da tıklayınca popup: renk, ad, kalan gram ve yüzde çubuğu
- Makara eşiğin altına düşünce masaüstü bildirimi
- Ayarlar popup'tan: Spoolman adresi, eşik, yenileme aralığı
- Yuva eşlemesi Spoolman'daki **Location** alanından (`AMS 1..4`) okunur. Köprüyle aynı yöntem.
- Linux'ta tepsi protokolü hover olayı vermez, sol tık ya da sağ tık menüsü → *Göster* kullanılır.

## License

MIT
