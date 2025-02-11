# Unified Chat Actor

A self-contained WebAssembly actor that provides a complete LLM chat interface using Claude. This actor combines a chat interface, state management, and LLM integration into a single component.

## Features

- 🌐 Complete web interface for chat interactions
- 💾 Persistent chat history and state management
- 🤖 Integration with Anthropic's Claude API
- 🔄 Real-time updates via WebSocket
- 📱 Responsive design that works on all devices

## Quick Start

1. Clone the repository
2. Create an `api-key.txt` file in the root directory with your Anthropic API key
3. Build the actor:
```bash
cargo build --release
```

4. Run using the Theater runtime:
```bash
theater run actor.toml
```

5. Open `http://localhost:8080` in your browser

## Project Structure

```
unified-chat/
├── Cargo.toml              # Project configuration
├── actor.toml             # Actor manifest
├── src/
│   ├── lib.rs            # Actor implementation
│   └── bindings.rs       # Generated bindings
├── assets/               # Web assets
│   ├── index.html       # Main HTML file
│   ├── styles.css       # CSS styles
│   └── chat.js         # Frontend JavaScript
└── README.md            # This file
```

## Features

### Frontend
- Clean, modern interface
- Real-time message updates
- Code block formatting
- Responsive design
- Auto-expanding text input
- Connection status indicator

### Backend
- Persistent chat history
- WebSocket for real-time updates
- RESTful API endpoints
- Claude integration
- State management with verification

## API Endpoints

- `GET /api/chats` - List all chats
- `POST /api/chats` - Create a new chat
- `GET /api/chats/:id` - Get chat details and messages

## WebSocket Events

- `get_all` - Get all chats and messages
- `new_chat` - Create a new chat
- `send_message` - Send a message
- `message_update` - Receive message updates

## Configuration

The actor can be configured via `actor.toml`:

```toml
name = "unified-chat"
version = "0.1.0"
description = "Unified LLM chat actor"
component_path = "target/wasm32-wasi/release/unified_chat.wasm"

[interface]
implements = "ntwk:theater/actor"
requires = []

[[handlers]]
type = "http-server"
config = { port = 8080 }

[[handlers]]
type = "websocket-server"
config = { port = 8081 }
```

## Development

### Prerequisites

- Rust (latest stable)
- wasm32-wasi target: `rustup target add wasm32-wasi`
- Theater runtime

### Building

```bash
# Build the actor
cargo build --release

# Run the actor
theater run actor.toml
```

### Testing

```bash
# Run tests
cargo test
```

## Architecture

This actor combines several components into a single WebAssembly module:

1. **HTTP Server**: Serves the web interface and handles API requests
2. **WebSocket Server**: Provides real-time updates
3. **State Management**: Handles chat history and persistence
4. **LLM Integration**: Communicates with Claude for message generation

### Data Flow

1. User interacts with web interface
2. WebSocket sends message to actor
3. Actor processes message and updates state
4. Actor sends message to Claude
5. Response is received and sent back to client
6. State is updated and persisted

## Contributing

Contributions are welcome! Please feel free to submit a Pull Request.

## License

MIT License - feel free to use this code in your own projects.

## Credits

This actor is part of the Theater actor system and uses several great technologies:

- [Theater](https://github.com/ntwk/theater) - Actor system
- [Claude](https://anthropic.com) - Language model
- Rust, WebAssembly, and various web technologies

## Security

The actor includes several security features:

- Input validation
- State verification
- Secure WebSocket connections
- API key protection

Please note that you should keep your `api-key.txt` secure and never commit it to version control.
