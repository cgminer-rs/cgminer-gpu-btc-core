# CGMiner GPU BTC Core

GPU Bitcoin mining core for CGMiner-RS, providing high-performance GPU-based Bitcoin mining capabilities.

## Features

- **Multi-Platform GPU Support**: OpenCL, CUDA, and Metal backends
- **Mac M4 GPU Optimization**: Specialized Metal compute shaders for Apple Silicon
- **High Performance**: Optimized SHA256d parallel computation
- **Flexible Architecture**: Modular design supporting multiple GPU vendors

## Supported Platforms

### Mac (Apple Silicon)
- **Metal Backend**: Optimized for Mac M4 GPU
- **Features**: `mac-metal`
- **Performance**: Up to 2 TH/s on M4 GPU

### NVIDIA GPUs
- **CUDA Backend**: High-performance NVIDIA GPU support
- **Features**: `cuda`
- **Status**: Planned

### AMD GPUs
- **OpenCL Backend**: AMD GPU support via OpenCL
- **Features**: `amd-opencl`
- **Status**: Planned

### Intel GPUs
- **OpenCL Backend**: Intel GPU support via OpenCL
- **Features**: `intel-opencl`
- **Status**: Planned

## Installation

Add to your `Cargo.toml`:

```toml
[dependencies]
cgminer-gpu-btc-core = { path = "../cgminer-gpu-btc-core", features = ["mac-metal"] }
```

## Usage

```rust
use cgminer_gpu_btc_core::{GpuCoreFactory, GpuMiningCore};
use cgminer_core::{CoreConfig, Work};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Create GPU core factory
    let factory = GpuCoreFactory::new();
    let mut core = factory.create_core().await?;

    // Initialize and start
    let config = CoreConfig::default();
    core.initialize(config).await?;
    core.start().await?;

    // Submit work and get results
    let work = create_work();
    core.submit_work(work).await?;

    Ok(())
}
```

## Examples

### Basic GPU Mining
```bash
cargo run --example basic_gpu_mining
```

### GPU Performance Benchmark
```bash
cargo run --example gpu_benchmark
```

### Mac Metal GPU Demo (Mac only)
```bash
cargo run --example metal_gpu_demo --features mac-metal
```

## Testing

### Run Unit Tests
```bash
cargo test
```

### Run Integration Tests
```bash
cargo test --test integration_tests
```

### Run Performance Benchmarks
```bash
cargo bench
```

### Generate Benchmark Reports
```bash
cargo bench -- --output-format html
```

## Performance Benchmarks

The project includes comprehensive benchmarks for:

- **Core Creation**: GPU core factory performance
- **Initialization**: Core initialization overhead
- **Device Scaling**: Performance with multiple devices
- **Work Submission**: Work submission throughput
- **Result Retrieval**: Result processing performance
- **Hashrate Scaling**: Performance at different hashrates

## Configuration

### GPU Core Configuration

```rust
use std::collections::HashMap;

let mut custom_params = HashMap::new();
custom_params.insert("max_hashrate".to_string(),
    serde_json::Value::Number(serde_json::Number::from(1_000_000_000_000u64))); // 1 TH/s
custom_params.insert("device_count".to_string(),
    serde_json::Value::Number(serde_json::Number::from(2)));

let config = CoreConfig {
    name: "GPU-BTC-Core".to_string(),
    device_count: 2,
    custom_params,
};
```

### Feature Flags

- `mac-metal`: Enable Mac Metal GPU support
- `opencl`: Enable OpenCL GPU support
- `cuda`: Enable NVIDIA CUDA support (planned)
- `mock-gpu`: Enable mock GPU for testing

## Architecture

The GPU core follows the cgminer-core architecture:

- **GpuCoreFactory**: Creates GPU mining cores
- **GpuMiningCore**: Main GPU mining implementation
- **GpuDevice**: Individual GPU device abstraction
- **Backend Implementations**: Platform-specific GPU backends
  - **MetalBackend**: Mac Metal compute shaders
  - **OpenCLBackend**: OpenCL kernel execution
  - **CudaBackend**: NVIDIA CUDA kernels (planned)

## GPU Algorithms

### SHA256d Implementation

The core implements optimized SHA256d (double SHA256) algorithms for each platform:

- **Metal Shaders** (`src/shaders/sha256d.metal`): Optimized for Mac M4 GPU
- **OpenCL Kernels** (`src/shaders/sha256d.cl`): Cross-platform OpenCL implementation
- **CUDA Kernels**: Planned for NVIDIA GPUs

### Performance Optimizations

- **Parallel Processing**: Batch processing of multiple nonces
- **Memory Optimization**: Efficient GPU memory usage
- **Threadgroup Optimization**: Platform-specific threading strategies
- **Constant Memory**: Cached SHA256 constants for better performance

## License

GPL-3.0
