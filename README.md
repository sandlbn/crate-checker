# crate-checker 

[![Crates.io](https://img.shields.io/crates/v/crate-checker.svg)](https://crates.io/crates/crate-checker)
[![Documentation](https://docs.rs/crate-checker/badge.svg)](https://docs.rs/crate-checker)
[![License](https://img.shields.io/crates/l/crate-checker.svg)](LICENSE)

A Rust crate information retrieval tool that provides both a powerful CLI and HTTP API for querying crates.io.

## Features

- **Crate Existence Checking** - Quickly verify if a crate exists on crates.io
- **Version Information** - Get detailed version history and metadata
- **Dependency Analysis** - Explore crate dependencies and their relationships
- **Download Statistics** - Access download metrics and trends
- **Batch Processing** - Process multiple crates efficiently in parallel
- **REST API Server** - Run as an HTTP server for integration with other tools
- **Multiple Output Formats** - JSON, YAML, Table, CSV, and compact formats

## Installation

### From crates.io

```bash
cargo install crate-checker
```

### From source

```bash
git clone https://github.com/sandlbn/crate-checker.git
cd crate-checker
cargo install --path .
```

## Quick Start

### CLI Usage

Check if a crate exists:
```bash
crate-checker check serde
```

Get detailed information about a crate:
```bash
crate-checker info tokio --deps --stats
```

Search for crates:
```bash
crate-checker search "http client" --limit 10
```

Check multiple crates at once:
```bash
crate-checker check-multiple serde tokio reqwest clap
```

Process a batch from a JSON file:
```bash
crate-checker batch --file crates.json
```

Start the HTTP API server:
```bash
crate-checker server --port 8080
```

### API Usage

```rust
use crate_checker::{CrateClient, Result};

#[tokio::main]
async fn main() -> Result<()> {
    let client = CrateClient::new();
    
    // Check if a crate exists
    let exists = client.crate_exists("serde").await?;
    println!("Serde exists: {}", exists);
    
    // Get crate information
    let info = client.get_crate_info("tokio").await?;
    println!("Tokio version: {}", info.newest_version);
    
    // Search for crates
    let results = client.search_crates("async", Some(5)).await?;
    for crate_info in results {
        println!("Found: {} v{}", crate_info.name, crate_info.newest_version);
    }
    
    Ok(())
}
```

## Command Reference

### Global Options

- `-f, --format <FORMAT>` - Output format: table (default), json, yaml, csv, compact
- `--verbose` - Enable verbose output
- `-q, --quiet` - Only show errors
- `--config <FILE>` - Path to configuration file
- `--timeout <DURATION>` - Request timeout (e.g., 30s, 2m, 1h)
- `--api-url <URL>` - Custom crates.io API URL

### Commands

#### `check` - Check if a crate exists

```bash
crate-checker check <CRATE_NAME> [--version <VERSION>]
```

#### `check-multiple` - Check multiple crates

```bash
crate-checker check-multiple <CRATE_NAMES...> [OPTIONS]
```

Options:
- `-s, --summary-only` - Show only summary
- `--fail-on-missing` - Exit with error if any crate doesn't exist

#### `info` - Get detailed crate information

```bash
crate-checker info <CRATE_NAME> [OPTIONS]
```

Options:
- `-d, --deps` - Include dependency information
- `-s, --stats` - Include download statistics

#### `versions` - List all versions

```bash
crate-checker versions <CRATE_NAME> [OPTIONS]
```

Options:
- `--no-yanked` - Hide yanked versions
- `-l, --limit <N>` - Limit number of versions

#### `search` - Search for crates

```bash
crate-checker search <QUERY> [OPTIONS]
```

Options:
- `-l, --limit <N>` - Maximum results (default: 10)
- `-e, --exact` - Show only exact matches

#### `deps` - Show dependencies

```bash
crate-checker deps <CRATE_NAME> [OPTIONS]
```

Options:
- `-v, --version <VERSION>` - Specific version (default: latest)
- `--runtime-only` - Show only runtime dependencies

#### `stats` - Show download statistics

```bash
crate-checker stats <CRATE_NAME> [OPTIONS]
```

Options:
- `-v, --versions` - Show version-specific stats

#### `batch` - Process multiple crates

```bash
crate-checker batch [OPTIONS]
```

Options:
- `--json <JSON>` - JSON string with batch input
- `--file <FILE>` - JSON file with batch input
- `-p, --parallel` - Process in parallel

#### `server` - Start HTTP API server

```bash
crate-checker server [OPTIONS]
```

Options:
- `-p, --port <PORT>` - Port to bind (default: 3000)
- `--host <HOST>` - Host to bind (default: 0.0.0.0)
- `--cors` - Enable CORS
- `-c, --config <FILE>` - Server configuration file

#### `config` - Generate configuration file

```bash
crate-checker config [--output <FILE>]
```

#### `examples` - Show batch input examples

```bash
crate-checker examples
```

## Batch Input Formats

### Crate Version Map

```json
{
  "serde": "1.0.0",
  "tokio": "latest",
  "reqwest": "0.11.0"
}
```

### Crate List

```json
{
  "crates": ["serde", "tokio", "reqwest", "clap"]
}
```

### Advanced Operations

```json
{
  "operations": [
    {
      "crate": "serde",
      "version": "1.0.0",
      "operation": "check_version"
    },
    {
      "crates": ["tokio", "reqwest"],
      "operation": "batch_check"
    }
  ]
}
```

## HTTP API Endpoints

When running as a server, the following endpoints are available:

- `GET /` - API documentation
- `GET /health` - Health check
- `GET /metrics` - Server metrics
- `GET /api/crates/{name}` - Get crate information
- `GET /api/crates/{name}/{version}` - Check specific version
- `GET /api/crates/{name}/{version}/deps` - Get dependencies
- `GET /api/crates/{name}/stats` - Get download statistics
- `GET /api/search?q={query}&limit={n}` - Search crates
- `POST /api/batch` - Batch processing

## Configuration

### Configuration File

Create a `crate-checker.toml` file:

```toml
[server]
port = 8080
host = "0.0.0.0"
workers = 4
enable_cors = true

[cache]
enabled = true
ttl_seconds = 300
max_entries = 1000

[logging]
level = "info"
format = "pretty"

[crates_io]
api_url = "https://crates.io/api/v1"
timeout_seconds = 30
```

Generate a sample configuration:
```bash
crate-checker config --output config.toml
```

### Environment Variables

All configuration options can be overridden using environment variables:

```bash
export CRATE_CHECKER__SERVER__PORT=8080
export CRATE_CHECKER__LOGGING__LEVEL=debug
export CRATE_CHECKER__CACHE__ENABLED=true
```

## Examples

### Monitor Crate Updates

```rust
use crate_checker::{CrateClient, Result};
use std::collections::HashMap;

async fn monitor_crate_updates(crates: Vec<&str>) -> Result<HashMap<String, String>> {
    let client = CrateClient::new();
    let mut versions = HashMap::new();
    
    for crate_name in crates {
        let version = client.get_latest_version(crate_name).await?;
        versions.insert(crate_name.to_string(), version);
    }
    
    Ok(versions)
}
```

### Integration with CI/CD

```yaml
# .github/workflows/check-deps.yml
name: Check Dependencies

on:
  schedule:
    - cron: '0 0 * * MON'

jobs:
  check:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v3
      - run: cargo install crate-checker
      - run: crate-checker check-multiple $(grep -E "^\w+ = " Cargo.toml | cut -d' ' -f1)
```


## Contributing

Contributions are welcome! Please feel free to submit a Pull Request.

1. Fork the repository
2. Create your feature branch (`git checkout -b feature/amazing-feature`)
3. Commit your changes (`git commit -m 'Add amazing feature'`)
4. Push to the branch (`git push origin feature/amazing-feature`)
5. Open a Pull Request

## License

This project is licensed under 

- Apache License, Version 2.0 http://www.apache.org/licenses/LICENSE-2.0)


## Support

If you encounter any issues or have questions, please file an issue on [GitHub](https://github.com/sandlbn/crate-checker/issues).

---

Made with ❤️ by the Rust community