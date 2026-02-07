<p align="center">
  <img src="assets/logo.png" alt="Blink Logo" width="120" />
</p>

<h1 align="center">Blink</h1>

<p align="center">
  <b>⚡ A lightning-fast, modern file manager for Linux</b>
</p>

<p align="center">
  <img src="https://img.shields.io/badge/rust-%23000000.svg?style=for-the-badge&logo=rust&logoColor=white" alt="Rust" />
  <img src="https://img.shields.io/badge/GTK4-4A90D9?style=for-the-badge&logo=gnome&logoColor=white" alt="GTK4" />
  <img src="https://img.shields.io/badge/Libadwaita-1A5FB4?style=for-the-badge&logo=gnome&logoColor=white" alt="Libadwaita" />
  <img src="https://img.shields.io/badge/License-MIT-green.svg?style=for-the-badge" alt="MIT License" />
</p>

<p align="center">
  <img src="assets/screenshot.png" alt="Blink Screenshot" width="800" />
</p>

---

## ✨ Features

<table>
<tr>
<td width="50%">

### 🚀 Performance
- **Blazing fast** file browsing powered by Rust
- **Async scanning** for large directories
- **Smart caching** for instant navigation

</td>
<td width="50%">

### 🎨 Design
- **Native GTK4** with Libadwaita styling
- **Dark and light** theme support
- **Clean, minimal** interface

</td>
</tr>
<tr>
<td width="50%">

### ⌨️ Keyboard-First
- **Fully customizable** keybindings
- **Vim-inspired** navigation
- **Quick access** to all actions

</td>
<td width="50%">

### 🔧 Power Features
- **Terminal integration** – open terminal in current directory
- **Mouse button support** – back/forward navigation
- **Micro editor** integration

</td>
</tr>
</table>

---

## 📦 Installation

### From Source

```bash
mkdir ~/.config/alloy
git clone https://github.com/artwik22/blink ~/.config/alloy/blink
cd ~/.config/alloy/blink
./install.sh
```

### Dependencies

| Package | Version |
|---------|---------|
| `gtk4` | ≥ 4.12 |
| `libadwaita` | ≥ 1.5 |
| `rust` | ≥ 1.70 |

---

## ⌨️ Keyboard Shortcuts

| Action | Default Keybind |
|--------|-----------------|
| **Toggle Hidden Files** | `Ctrl` + `H` |
| **Open Terminal** | `H` |
| **Select All** | `Ctrl` + `A` |
| **Refresh** | `F5` |
| **Open with Micro** | `M` |
| **Back** | `Mouse8` |
| **Forward** | `Mouse9` |
| **Go Up** | `↑` |
| **Go Home** | `Home` |
| **Copy** | `Ctrl` + `C` |
| **Cut** | `Ctrl` + `X` |
| **Paste** | `Ctrl` + `V` |
| **Delete** | `Delete` |
| **Rename** | `F2` |

> 💡 **Tip:** All keybindings are fully customizable through **Fuse Settings → Index**

---

## ⚙️ Configuration

Configuration files are stored in:

```
~/.config/index/
├── keybinds.conf    # Keyboard shortcuts
└── settings.json    # General preferences
```

### Keybinds Format

```ini
# Format: action=key:modifier1,modifier2
toggle_hidden=h:Control
open_terminal=h
select_all=a:Control
refresh=F5
back=Mouse8
forward=Mouse9
```

**Available modifiers:** `Control`, `Shift`, `Alt`, `Super`

---

## 🎯 Part of Alloy

<p align="center">
  <b>Blink is part of the <a href="https://github.com/artwik22/alloy">Alloy</a> desktop suite</b>
</p>

| Component | Description |
|-----------|-------------|
| **Dart** | Quickshell-powered panel & widgets |
| **Fuse** | GTK4 settings center |
| **Blink** | Modern file manager |
| **Core** | System monitor |

---

## 📄 License

This project is licensed under the **MIT License** – see the [LICENSE](LICENSE) file for details.

---

<p align="center">
  Made with ❤️ by <a href="https://github.com/artwik22">artwik22</a>
</p>