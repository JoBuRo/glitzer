
# ✨ Glitzer

> *"See your Git repositories shine — one object at a time."*

**Glitzer** is a Rust-based command-line tool for exploring Git repositories.
Its goal is to provide fast, insightful Git statistics and repository analysis — though it’s still early in development.

[![Build](https://github.com/JoBuRo/glitzer/actions/workflows/rust.yml/badge.svg?branch=main)](https://github.com/JoBuRo/glitzer/actions/workflows/rust.yml)

> ⚠️ **Note:** Most functionality is not yet implemented.
> Currently, Glitzer supports:
>
> * Listing and inspecting Git objects
> * Displaying basic commit history

---

## 🧰 Features (planned & in progress)

* ✅ **Object Retrieval** — Read and display raw Git objects
* ✅ **Commit History** — Show commits in a human-readable format
* 🚧 **Statistics** — Analyze commits, branches, and contributors
* 🚧 **Visualization** — View activity timelines and branch overviews

---

## 🛠️ Installation

You’ll need [Rust and Cargo](https://www.rust-lang.org/tools/install) installed.

```bash
git clone https://github.com/JoBuRo/glitzer.git
cd glitzer
cargo build --release
```

Run the built binary:

```bash
./target/release/glitzer <command> [options]
```

---

## 💡 Usage

### Commands

| Command       | Description                                  |
| ------------- | -------------------------------------------- |
| `object <id>` | Show information about a specific Git object |
| `history`     | Display the repository’s commit history      |

### Examples

```bash
# Show commit history
glitzer history

# Inspect a Git object
glitzer object <object-id>
```

---

## 🤝 Contributing

Glitzer is in its early stages — contributions, ideas, and feedback are welcome!
Feel free to open an issue or submit a pull request.

---

## 🪪 License

This project is licensed under the **Apache 2.0 License**.


