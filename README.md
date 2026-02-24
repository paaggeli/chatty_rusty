# Chatty Rusty

A minimal, beginner-friendly TCP chat server and client written in Rust. Perfect for learning Rust concepts.

Connect multiple clients to a server, send messages, and watch them appear in real time on every other connected terminal.

## Features

- 🦀 Built with async Rust and Tokio
- 💬 Broadcast messages to all connected clients
- 🔌 Handles multiple concurrent connections
- 📡 Graceful connection lifecycle (connect & disconnect detection)
- 🪶 Lightweight — no threads per connection, Tokio tasks instead

## Prerequisites

Before you begin, make sure you have Rust installed on your system.

### Installing Rust

**Linux/macOS:**
```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
```

**Windows:**
Download and run the installer from [rustup.rs](https://rustup.rs/)

Verify installation:
```bash
rustc --version
cargo --version
```

## Installation

### Clone
```bash
# Clone the repository (if you have it on GitHub)
git clone https://github.com/yourusername/chatty_rusty.git

# Navigate to the project directory
cd chatty_rusty
```

### Option 1: Build the project
```bash
# Build the project
cargo build --release

# The executable will be in target/release/
```

### Option 2: Install directly with Cargo
```bash
cargo install --path .
```

## Usage

Chatty Rusty has two binaries. A server and a client. You will need at least **three terminal windows** open in the project directory.

### Start the Server

**Terminal 1:**
```bash
cargo run --bin server
```

You should see:
```
Chatty Rusty server listening on 127.0.0.1:8080
```

### Connect Clients

**Terminal 2:**
```bash
cargo run --bin client
```

**Terminal 3:**
```bash
cargo run --bin client
```

You should see in each client terminal:
```
Connected to Chatty Rusty server!
```

And in the server terminal:
```
New connection from: 127.0.0.1:54321
127.0.0.1:54321 has been added to the client registry
```

### Send Messages

Type a message in Terminal 2 and press **Enter**. It will appear in Terminal 3 prefixed with the sender's address:

**Terminal 2 (sender):**
```
hello!
```

**Terminal 3 (receiver):**
```
127.0.0.1:54321: hello!
```

### Disconnect

Press **Ctrl+C** in a client terminal to disconnect. The server will log the disconnection and remove the client from the registry:
```
127.0.0.1:54321 disconnected
127.0.0.1:54321 has been removed from the client registry
```

## How It Works

### Server

The server listens for incoming TCP connections on `127.0.0.1:8080`. When a client connects, it spawns a new **Tokio task** to handle that client independently. Each task reads incoming messages from its client and broadcasts them to all other connected clients.

A shared `Arc<Mutex<HashMap>>` stores the write half of every connected client's TCP stream. This allows any task to send a message to any other client safely across concurrent tasks.

### Client

The client connects to the server and splits its TCP stream into two halves:
- A **read half** watched by a dedicated task that prints incoming messages from the server
- A **write half** used by a second task that reads the user's terminal input and sends it to the server

`tokio::select!` races both tasks. When either one finishes (server disconnects or user quits), the client exits cleanly.

### Key Concepts

| Concept | What it does in this project |
|---|---|
| `TcpListener` | Accepts incoming client connections on the server |
| `TcpStream` | The active connection between server and client |
| `into_split()` | Splits a stream into independent read and write halves |
| `Arc<Mutex<...>>` | Shares the client registry safely across concurrent tasks |
| `tokio::spawn` | Launches a lightweight task per connected client |
| `BufReader` | Reads complete lines efficiently from a TCP stream |
| `tokio::select!` | Races two async tasks and reacts to whichever finishes first |

## Project Structure
```
chatty_rusty/
├── src/
│   └── bin/
│       ├── server.rs    # Server — accepts connections, broadcasts messages
│       └── client.rs    # Client — sends user input, prints incoming messages
├── Cargo.toml           # Project dependencies
├── README.md            # This file
├── LICENSE              # License information
└── CONTRIBUTING.md      # Contribution guidelines
```

## Dependencies

This project uses the following crates:
- **tokio** - Async runtime
This project uses only one dependency, [Tokio](https://tokio.rs/), the async runtime for Rust. The `"full"` feature flag enables TCP networking, async I/O, task spawning, and everything else needed to run the app.

## Extra Resources
**📖 Blog Post**: Read about how I built this project and learned Rust along the way using AI:
- [Building Chatty Rusty: Learning Rust with AI and Small Projects](https://opensourceodyssey.com/building-chatty-rusty-learning-rust-with-ai-and-small-projects/)

## Contributing

Contributions are welcome! Please read [CONTRIBUTING.md](CONTRIBUTING.md) for details on our code of conduct and the process for submitting pull requests.

### Ways to Contribute

Contributions are welcome! Feel free to:
- 🐛 Report bugs
- 💡 Suggest new features
- 📝 Improve documentation
- 🎨 Enhance the UI/UX

## License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.

## Author

- GitHub: [@paaggeli](https://github.com/paaggeli)

---

**Happy chatting! 🦀💬**