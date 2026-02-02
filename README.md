# json-poller

A lightweight, flexible, high-performance JSON polling library for Rust.

## Features

- Automatic connection reuse (no TCP/TLS handshake overhead)
- High-performance polling with configurable intervals
- Works with any struct that implements `serde::Deserialize`
- Async callbacks for flexible data processing

## Installation

Add to your `Cargo.toml`:
```toml
[dependencies]
json-poller = "0.2.2"
tokio = { version = "1", features = ["full"] }
serde = { version = "1.0", features = ["derive"] }
```

Or install directly from GitHub:
```toml
[dependencies]
json-poller = { git = "https://github.com/erik404/json-poller" }
tokio = { version = "1", features = ["full"] }
serde = { version = "1.0", features = ["derive"] }
```

## Configuration
```rust
let poller = JsonPoller::<MyType>::builder(url)
    .poll_interval_ms(500)        // How often to poll (default: 500ms)
    .request_timeout_ms(1000)     // Request timeout (default: 1000ms)
    .pool_max_idle_per_host(1)    // Connections to keep alive (default: 1)
    .pool_idle_timeout_secs(90)   // How long to keep connections (default: 90s)
    .tcp_keepalive_secs(60)       // TCP keepalive interval (default: 60s)
    .build()?;
```

## Usage
```rust
use json_poller::JsonPoller;
use serde::Deserialize;

#[derive(Deserialize)]
struct PriceResponse {
    price: f64,
    timestamp: i64,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let poller = JsonPoller::<PriceResponse>::builder("https://api.example.com/price")
        .poll_interval_ms(250)
        .build()?;

    poller.start(|resp, duration| async move {
        save_to_database(&resp).await.ok();
        println!("Price: €{:.2} (fetched in {:?})", resp.price, duration);
    }).await;

    Ok(())
}
```

## Requirements

- Works with any type that implements `serde::Deserialize`
- Requires `tokio` async runtime
- Only supports JSON responses

## License

MIT OR Apache-2.0