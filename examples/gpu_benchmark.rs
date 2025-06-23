//! GPU 挖矿性能基准测试
//!
//! 测试不同 GPU 后端的性能表现

use cgminer_core::{CoreConfig, Work, DeviceConfig, DeviceInfo, DeviceType};
use cgminer_gpu_btc_core::{GpuCoreFactory, GpuMiningCore};
use std::collections::HashMap;
use tokio::time::{sleep, Duration, Instant};
use tracing::{info, warn, error};
use uuid::Uuid;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 初始化日志
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .init();

    info!("🏁 启动 GPU 挖矿性能基准测试");

    // 测试不同的配置
    let test_configs = vec![
        ("单设备低算力", 1, 100_000_000_000u64),      // 100 GH/s
        ("单设备高算力", 1, 1_000_000_000_000u64),     // 1 TH/s
        ("双设备中算力", 2, 500_000_000_000u64),       // 500 GH/s each
        ("四设备高算力", 4, 1_000_000_000_000u64),     // 1 TH/s each
    ];

    for (test_name, device_count, hashrate_per_device) in test_configs {
        info!("\n🧪 开始测试: {}", test_name);
        info!("   设备数量: {}", device_count);
        info!("   单设备算力: {:.2} GH/s", hashrate_per_device as f64 / 1_000_000_000.0);

        let result = run_benchmark_test(device_count, hashrate_per_device).await;

        match result {
            Ok(stats) => {
                info!("✅ 测试 '{}' 完成:", test_name);
                info!("   总算力: {:.2} GH/s", stats.total_hashrate / 1_000_000_000.0);
                info!("   找到解数: {}", stats.solutions_found);
                info!("   平均延迟: {:.2} ms", stats.avg_latency_ms);
                info!("   设备利用率: {:.1}%", stats.device_utilization * 100.0);
            }
            Err(e) => {
                error!("❌ 测试 '{}' 失败: {}", test_name, e);
            }
        }

        // 测试间隔
        sleep(Duration::from_secs(2)).await;
    }

    info!("\n🏆 所有基准测试完成");
    Ok(())
}

/// 基准测试统计信息
#[derive(Debug)]
struct BenchmarkStats {
    total_hashrate: f64,
    solutions_found: u32,
    avg_latency_ms: f64,
    device_utilization: f64,
}

/// 运行单个基准测试
async fn run_benchmark_test(device_count: u32, hashrate_per_device: u64) -> Result<BenchmarkStats, Box<dyn std::error::Error>> {
    // 创建 GPU 核心工厂
    let factory = GpuCoreFactory::new();

    // 创建核心配置
    let mut custom_params = HashMap::new();
    custom_params.insert("max_hashrate".to_string(), serde_json::Value::Number(serde_json::Number::from(hashrate_per_device)));
    custom_params.insert("device_count".to_string(), serde_json::Value::Number(serde_json::Number::from(device_count)));

    let core_config = CoreConfig {
        name: format!("GPU-Benchmark-{}-devices", device_count),
        device_count,
        custom_params,
    };

    // 创建并初始化挖矿核心
    let mut core = factory.create_core().await?;
    core.initialize(core_config).await?;

    // 启动核心
    core.start().await?;

    // 创建测试工作
    let work = create_benchmark_work();

    // 提交工作到所有设备
    for device_id in 0..device_count {
        if let Ok(mut device) = core.get_device(device_id).await {
            device.submit_work(work.clone()).await?;
        }
    }

    // 运行基准测试
    let test_duration = Duration::from_secs(30); // 30秒测试
    let start_time = Instant::now();
    let mut solutions_found = 0;
    let mut latency_samples = Vec::new();

    info!("⏱️  运行 30 秒基准测试...");

    while start_time.elapsed() < test_duration {
        let round_start = Instant::now();

        // 收集结果
        for device_id in 0..device_count {
            if let Ok(mut device) = core.get_device(device_id).await {
                while let Ok(Some(_result)) = device.get_result().await {
                    solutions_found += 1;
                    latency_samples.push(round_start.elapsed().as_millis() as f64);
                }
            }
        }

        // 重新提交工作
        let new_work = create_benchmark_work();
        for device_id in 0..device_count {
            if let Ok(mut device) = core.get_device(device_id).await {
                let _ = device.submit_work(new_work.clone()).await;
            }
        }

        sleep(Duration::from_millis(100)).await;
    }

    // 计算统计信息
    let total_hashrate = (device_count as u64 * hashrate_per_device) as f64;
    let avg_latency_ms = if latency_samples.is_empty() {
        0.0
    } else {
        latency_samples.iter().sum::<f64>() / latency_samples.len() as f64
    };

    // 计算设备利用率（基于找到的解数量）
    let expected_solutions = (total_hashrate / 1_000_000_000_000.0 * 30.0 * 0.001).max(1.0); // 粗略估算
    let device_utilization = (solutions_found as f64 / expected_solutions).min(1.0);

    // 停止核心
    core.stop().await?;

    Ok(BenchmarkStats {
        total_hashrate,
        solutions_found,
        avg_latency_ms,
        device_utilization,
    })
}

/// 创建基准测试工作
fn create_benchmark_work() -> Work {
    // 创建一个标准的测试区块头
    let mut block_header = [0u8; 80];

    // 设置测试数据
    block_header[0..4].copy_from_slice(&1u32.to_le_bytes()); // version

    // 随机化一些字段以确保每次工作都不同
    let random_data: [u8; 32] = std::array::from_fn(|_| fastrand::u8(..));
    block_header[4..36].copy_from_slice(&random_data); // prev_hash

    block_header[68..72].copy_from_slice(&0x1d00ffffu32.to_le_bytes()); // bits
    block_header[72..76].copy_from_slice(&(std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs() as u32).to_le_bytes()); // timestamp

    // 设置适中的目标难度
    let mut target = [0xffu8; 32];
    target[28] = 0x00;
    target[29] = 0x00;
    target[30] = 0x00;
    target[31] = 0x1f; // 稍微容易一点，便于测试

    Work::new("benchmark_job".to_string(), target, block_header, 1.0)
}
