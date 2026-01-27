# json-poller

Data-agnostic JSON polling. Works with any struct that implements `Deserialize`. Reuses HTTP connections instead of creating new ones each time, eliminating TCP and TLS handshake overhead on every request.

## Installation

Add to your `Cargo.toml`:
```toml
[dependencies]
json-poller = "0.1"
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

## Usage
```rust
use json_poller::JsonPoller;
use serde::Deserialize;

#[derive(Deserialize)]
struct Weather {
    temperature: f64,
    condition: String,
    latitude: f64,
    longitude: f64
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let poller = JsonPoller::<Weather>::builder("https://api.weather.com/current")
        .poll_interval_ms(60000)
        .build()?;

    poller.start(|weather, duration| {
        println!("{:.1}°C - {} at ({:.2}, {:.2}) (fetched in {:?})",
         weather.temperature,
         weather.condition,
         weather.latitude,
         weather.longitude,
         duration
        );
    }).await;

    Ok(())
}
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

## Requirements

- Works with any type that implements `serde::Deserialize`
- Requires `tokio` async runtime
- Only supports JSON responses

## License

MIT OR Apache-2.0