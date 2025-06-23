//! GPU 核心性能基准测试

use criterion::{black_box, criterion_group, criterion_main, Criterion, BenchmarkId};
use cgminer_core::{CoreConfig, Work, DeviceConfig};
use cgminer_gpu_btc_core::{GpuCoreFactory, GpuMiningCore};
use std::collections::HashMap;
use tokio::runtime::Runtime;
use uuid::Uuid;

/// 基准测试：GPU 核心创建
fn bench_gpu_core_creation(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();

    c.bench_function("gpu_core_creation", |b| {
        b.iter(|| {
            rt.block_on(async {
                let factory = GpuCoreFactory::new();
                let core = factory.create_core().await.unwrap();
                black_box(core);
            });
        });
    });
}

/// 基准测试：GPU 核心初始化
fn bench_gpu_core_initialization(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();

    c.bench_function("gpu_core_initialization", |b| {
        b.iter(|| {
            rt.block_on(async {
                let factory = GpuCoreFactory::new();
                let mut core = factory.create_core().await.unwrap();

                let config = create_benchmark_config(1);
                core.initialize(config).await.unwrap();
                black_box(core);
            });
        });
    });
}

/// 基准测试：不同设备数量的性能
fn bench_device_scaling(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let mut group = c.benchmark_group("device_scaling");

    for device_count in [1, 2, 4, 8].iter() {
        group.bench_with_input(
            BenchmarkId::new("devices", device_count),
            device_count,
            |b, &device_count| {
                b.iter(|| {
                    rt.block_on(async {
                        let factory = GpuCoreFactory::new();
                        let mut core = factory.create_core().await.unwrap();

                        let config = create_benchmark_config(device_count);
                        core.initialize(config).await.unwrap();
                        core.start().await.unwrap();

                        // 模拟短时间运行
                        tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;

                        core.stop().await.unwrap();
                        black_box(core);
                    });
                });
            },
        );
    }
    group.finish();
}

/// 基准测试：工作提交性能
fn bench_work_submission(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();

    // 预先创建和初始化核心
    let (mut core, _) = rt.block_on(async {
        let factory = GpuCoreFactory::new();
        let mut core = factory.create_core().await.unwrap();
        let config = create_benchmark_config(1);
        core.initialize(config).await.unwrap();
        core.start().await.unwrap();
        let device = core.get_device(0).await.unwrap();
        (core, device)
    });

    c.bench_function("work_submission", |b| {
        b.iter(|| {
            rt.block_on(async {
                let work = create_benchmark_work();
                let mut device = core.get_device(0).await.unwrap();
                device.submit_work(work).await.unwrap();
                black_box(());
            });
        });
    });

    rt.block_on(async {
        core.stop().await.unwrap();
    });
}

/// 基准测试：结果获取性能
fn bench_result_retrieval(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();

    c.bench_function("result_retrieval", |b| {
        b.iter(|| {
            rt.block_on(async {
                let factory = GpuCoreFactory::new();
                let mut core = factory.create_core().await.unwrap();
                let config = create_benchmark_config(1);
                core.initialize(config).await.unwrap();
                core.start().await.unwrap();

                // 提交工作
                let work = create_benchmark_work();
                let mut device = core.get_device(0).await.unwrap();
                device.submit_work(work).await.unwrap();

                // 等待并获取结果
                tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;
                let result = device.get_result().await;

                core.stop().await.unwrap();
                black_box(result);
            });
        });
    });
}

/// 基准测试：不同算力配置的性能
fn bench_hashrate_scaling(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let mut group = c.benchmark_group("hashrate_scaling");

    let hashrates = [
        100_000_000u64,      // 100 MH/s
        1_000_000_000u64,    // 1 GH/s
        10_000_000_000u64,   // 10 GH/s
        100_000_000_000u64,  // 100 GH/s
    ];

    for hashrate in hashrates.iter() {
        group.bench_with_input(
            BenchmarkId::new("hashrate_mhs", hashrate / 1_000_000),
            hashrate,
            |b, &hashrate| {
                b.iter(|| {
                    rt.block_on(async {
                        let factory = GpuCoreFactory::new();
                        let mut core = factory.create_core().await.unwrap();

                        let config = create_hashrate_config(hashrate);
                        core.initialize(config).await.unwrap();
                        core.start().await.unwrap();

                        // 提交工作并运行短时间
                        let work = create_benchmark_work();
                        let mut device = core.get_device(0).await.unwrap();
                        device.submit_work(work).await.unwrap();

                        tokio::time::sleep(tokio::time::Duration::from_millis(20)).await;

                        core.stop().await.unwrap();
                        black_box(core);
                    });
                });
            },
        );
    }
    group.finish();
}

/// 基准测试：统计信息获取
fn bench_statistics_retrieval(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();

    c.bench_function("statistics_retrieval", |b| {
        b.iter(|| {
            rt.block_on(async {
                let factory = GpuCoreFactory::new();
                let mut core = factory.create_core().await.unwrap();
                let config = create_benchmark_config(1);
                core.initialize(config).await.unwrap();
                core.start().await.unwrap();

                let device = core.get_device(0).await.unwrap();
                let stats = device.get_stats().await.unwrap();

                core.stop().await.unwrap();
                black_box(stats);
            });
        });
    });
}

/// 创建基准测试配置
fn create_benchmark_config(device_count: u32) -> CoreConfig {
    let mut custom_params = HashMap::new();
    custom_params.insert("max_hashrate".to_string(), serde_json::Value::Number(serde_json::Number::from(1_000_000_000u64)));
    custom_params.insert("device_count".to_string(), serde_json::Value::Number(serde_json::Number::from(device_count)));

    CoreConfig {
        name: "Benchmark-GPU-Core".to_string(),
        device_count,
        custom_params,
    }
}

/// 创建指定算力的配置
fn create_hashrate_config(hashrate: u64) -> CoreConfig {
    let mut custom_params = HashMap::new();
    custom_params.insert("max_hashrate".to_string(), serde_json::Value::Number(serde_json::Number::from(hashrate)));
    custom_params.insert("device_count".to_string(), serde_json::Value::Number(serde_json::Number::from(1)));

    CoreConfig {
        name: "Hashrate-Benchmark-GPU-Core".to_string(),
        device_count: 1,
        custom_params,
    }
}

/// 创建基准测试工作
fn create_benchmark_work() -> Work {
    let mut block_header = [0u8; 80];

    // 设置随机数据以确保每次都不同
    for i in 0..80 {
        block_header[i] = fastrand::u8(..);
    }

    // 设置标准字段
    block_header[0..4].copy_from_slice(&1u32.to_le_bytes()); // version
    block_header[68..72].copy_from_slice(&0x1d00ffffu32.to_le_bytes()); // bits

    let mut target = [0xffu8; 32];
    target[31] = 0x0f; // 简单目标

    Work::new("benchmark_job".to_string(), target, block_header, 1.0)
}

criterion_group!(
    benches,
    bench_gpu_core_creation,
    bench_gpu_core_initialization,
    bench_device_scaling,
    bench_work_submission,
    bench_result_retrieval,
    bench_hashrate_scaling,
    bench_statistics_retrieval
);

criterion_main!(benches);
