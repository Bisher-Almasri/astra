<p align="center">
	<img src="/banner.png" alt="Astra banner" />
</p>

<h1 align="center">Astra Browser Engine</h1>

<p align="center">
	An experimental browser engine written in Rust.
</p>

It currently includes a full mini pipeline:

- HTML parsing into a DOM
- CSS tokenization/parsing
- Style computation
- Layout calculation
- Software rendering to an RGBA pixel buffer
- A lightweight browser API with navigation history and resource caching
- Basic HTTP/HTTPS fetching
- JavaScript execution via V8 (with minimal browser-like bindings)

## Status

This project is an educational/experimental engine and is still in active development. APIs and behavior may change.

## Features

- Browser core (`src/browser.rs`)
	- Load local files, `file://` URLs, and HTTP(S) content
	- Keep page history and support back/forward navigation
	- Cache fetched HTML resources
	- Track loading state (`Idle`, `Loading`, `Complete`, `Error`)
- Parsing and rendering pipeline
	- HTML parser (`src/html/`)
	- CSS parser/tokenizer (`src/css/`)
	- Style computation (`src/style.rs`)
	- Layout engine (`src/layout.rs`)
	- Renderer (`src/render.rs`)
- Networking (`src/network.rs`)
	- Basic HTTP and HTTPS GET requests
- JavaScript runtime (`src/js.rs`)
	- Executes scripts using V8
	- Supports basic `console.log`
	- Includes a minimal mock `document.getElementById(...).setAttribute(...)`

## Project Structure

```
src/
	browser.rs      # High-level browser API and loading pipeline
	dom.rs          # DOM node model
	html/           # HTML parser and tests
	css/            # CSS tokenizer/parser
	style.rs        # Applies CSS to DOM
	layout.rs       # Computes layout tree
	render.rs       # Produces RGBA pixel buffer
	network.rs      # HTTP/HTTPS fetch implementation
	js.rs           # JavaScript runtime integration (V8)
	resources.rs    # Resource lifetime management
	error.rs        # Browser error types and error page helper
	main.rs         # Example executable entrypoint
```

## Requirements

- Rust toolchain (edition 2024, stable recommended)
- Cargo

Note: building with the `v8` crate may take additional time/resources on first compile.

## Build

```bash
cargo build
```

## Run

```bash
cargo run
```

The current `main` executable loads `test/index.html`, runs inline scripts, and prints pipeline/runtime info.

## Test

```bash
cargo test
```

## Minimal Usage Example

```rust
use astra::browser::Browser;

fn main() {
		let mut browser = Browser::new();

		match browser.load("test/index.html") {
				Ok(pixels) => {
						println!("Loaded page with {} bytes", pixels.len());
				}
				Err(err) => {
						eprintln!("Failed to load page: {}", err);
				}
		}
}
```

## Notes and Limitations

- JavaScript DOM support is intentionally minimal and not standards-compliant.
- Networking is basic and currently focused on simple GET flows.
- Rendering/layout behavior is not yet production-browser compatible.

## License

CC by NC 4.0
