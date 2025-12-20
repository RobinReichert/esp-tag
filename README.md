<table>
  <tr>
    <td>
      <img src="assets/logo.svg" width="80">
    </td>
    <td>
      <h1>ESP32 Laser Tag System</h1>
    </td>
  </tr>
</table>

An open-source laser tag system built on the ESP32c3

---

## Table of Contents
- [Overview](#overview)
- [Features](#features)
- [Roadmap](#roadmap)
- [Getting Started](#getting-started)
- [Contributing](#contributing)
- [License](#license)

---

## Overview
This project is a DIY laser tag system based on the ESP32c3 microcontroller. It is designed for hobbyists and makers who want a modular, low-cost, and customizable laser tag experience. The system supports IR-based hit detection, wireless syncing between players.

---

## Features
- Wireless communication via ESP-NOW  
- Self healing mesh network

---

## Roadmap
- Ir-based hit detection
- Multiple sensors communicating via i2c
- Web gateway for game configuration

---

## Getting Started

### Prerequisites
- ESP32c3 board and USB cable  
- Follow the guide on the [Rust Book](https://docs.espressif.com/projects/rust/book/) for toolchain setup

### CLI Cheatsheet
- build and flash app:

```sh
cargo run --release --no-default-features --features hardware --target riscv32imc-unknown-none-elf
```

- test hardware independent code:

```sh
cargo test --no-default-features --features std
```

---

## Contributing
We welcome contributions! To ensure smooth collaboration, please:

- Fork the repository and create your feature branch.
- Write clear, well-documented code.
- Add tests for any new functionality.
- Submit a pull request and describe your changes.

---

## License
This project is licensed under the MIT License. See the [LICENSE](./LICENSE) file for details.
