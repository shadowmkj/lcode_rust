# 🚀 leetrs (LeetCode TUI)

[![Rust](https://img.shields.io/badge/rust-1.70%2B-blue.svg)](https://www.rust-lang.org)
[![Status](https://img.shields.io/badge/status-beta-orange.svg)]()
[![Neovim](https://img.shields.io/badge/Neovim-0.9%2B-green.svg)](https://neovim.io/)

**leetrs** is a blazing-fast, Rust-powered CLI and engine designed to make solving LeetCode problems from the terminal a first-class developer experience.

Built specifically for developers who live in the terminal and rely on **Neovim**, `leetrs` strips away the distraction of the browser. It handles intelligent authentication, Markdown problem generation, native editor window splitting, and asynchronous code submission without ever leaving your workflow.

---

## ✨ Features

* **Intelligent Authentication (`leetrs auth`)**
  * Automatically extracts `LEETCODE_SESSION` and `csrftoken` cookies from your active Chrome or Firefox sessions.
  * Secure, hidden manual fallback for containerized browser profiles.
* **Frictionless Problem Fetching (`leetrs pick`)**
  * Fetch problems using their URL slug (e.g., `two-sum`) or standard numerical ID (e.g., `1`).
  * Automatically parses LeetCode's raw HTML into clean, readable **Markdown**.
  * Generates idiomatic `snake_case.rs` files containing the exact boilerplate required.
* **Native Neovim Integration**
  * Instantly hijacks the terminal process to launch Neovim.
  * Forces a clean vertical split (`vsplit`) to place your problem description and code side-by-side, bypassing layout quirks from custom dashboards.
* **Async Submission Engine (`leetrs submit`)**
  * Submit your local file directly to LeetCode's execution servers.
  * Automatic ID resolution and CSRF token bypass.
  * Color-coded terminal output for execution results, including Runtime/Memory statistics and detailed compiler error logs.

---

## 🛠️ Installation

Currently, `leetrs` is in beta and must be built from source.

### Prerequisites
* [Rust & Cargo](https://rustup.rs/)
* [Neovim](https://neovim.io/) (Accessible via the `nvim` command in your PATH)
* A valid [LeetCode](https://leetcode.com) account

### Build & Install

```bash
# 1. Clone the repository
git clone https://github.com/shadowmkj/leetrs_rust.git
cd leetcode_engine

# 2. Build the release binary
cargo build --release

# 3. Move the binary to your PATH (e.g., /usr/local/bin or ~/.cargo/bin)
cp target/release/leetrs ~/.cargo/bin/
