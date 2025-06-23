//! 基础 GPU 挖矿示例
//!
//! 演示如何使用 cgminer-gpu-btc-core 进行基础的 GPU 挖矿操作

use cgminer_core::{CoreConfig, Work, CoreFactory};
use cgminer_gpu_btc_core::GpuCoreFactory;
use std::collections::HashMap;
use tokio::time::{sleep, Duration};
use tracing::info;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 初始化日志
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .init();

    info!("🚀 启动基础 GPU 挖矿示例");

    // 创建 GPU 核心工厂
    let factory = GpuCoreFactory::new();

    // 创建核心配置
    let mut custom_params = HashMap::new();
    custom_params.insert("max_hashrate".to_string(), serde_json::Value::Number(serde_json::Number::from(1_000_000_000_000u64))); // 1 TH/s
    custom_params.insert("device_count".to_string(), serde_json::Value::Number(serde_json::Number::from(2)));

    let core_config = CoreConfig {
        name: "GPU-BTC-Core".to_string(),
        enabled: true,
        devices: vec![],
        custom_params,
    };

    // 创建并初始化挖矿核心
    let mut core = factory.create_core(core_config).await?;
    // 核心在创建时已经初始化

    info!("✅ GPU 挖矿核心初始化完成");

    // 启动核心
    core.start().await?;
    info!("🔥 GPU 挖矿核心启动完成");

    // 创建测试工作
    let work = create_test_work();
    info!("📋 创建测试工作: {}", work.id);

    // 提交工作到核心
    core.submit_work(work.clone()).await?;
    info!("📤 工作已提交到 GPU 核心");

    // 运行挖矿循环
    let mut total_results = 0;
    let start_time = std::time::Instant::now();

    info!("⛏️  开始 GPU 挖矿...");

    for round in 1..=20 {
        info!("🔄 挖矿轮次 {}/20", round);

        // 等待一段时间让设备工作
        sleep(Duration::from_secs(3)).await;

        // 收集核心的结果
        let results = core.collect_results().await?;
        let round_results = results.len();
        total_results += round_results;

        for result in results {
            info!("💎 GPU 核心找到解: nonce={}, 总计={}",
                  result.nonce, total_results);
        }

        if round_results > 0 {
            info!("✨ 本轮找到 {} 个有效解", round_results);
        }

        // 重新提交工作
        let new_work = create_test_work();
        core.submit_work(new_work).await?;
    }

    let elapsed = start_time.elapsed();
    info!("⏱️  挖矿完成! 总时间: {:.2}s, 总结果: {}, 平均: {:.2} 结果/秒",
          elapsed.as_secs_f64(), total_results, total_results as f64 / elapsed.as_secs_f64());

    // 获取核心统计信息
    if let Ok(core_stats) = core.get_stats().await {
        info!("📈 核心统计:");
        info!("   - 总算力: {:.2} GH/s", core_stats.total_hashrate / 1_000_000_000.0);
        info!("   - 活跃设备: {}", core_stats.active_devices);
        info!("   - 接受工作: {}", core_stats.accepted_work);
        info!("   - 拒绝工作: {}", core_stats.rejected_work);
    }

    // 停止核心
    core.stop().await?;
    info!("🛑 GPU 挖矿核心已停止");

    info!("✅ 基础 GPU 挖矿示例完成");
    Ok(())
}

/// 创建测试工作
fn create_test_work() -> Work {
    // 创建一个简单的测试区块头
    let mut block_header = [0u8; 80];

    // 设置一些测试数据
    block_header[0..4].copy_from_slice(&1u32.to_le_bytes()); // version
    block_header[68..72].copy_from_slice(&0x1d00ffffu32.to_le_bytes()); // bits (难度)
    block_header[72..76].copy_from_slice(&(std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs() as u32).to_le_bytes()); // timestamp

    // 设置一个相对容易的目标
    let mut target = [0xffu8; 32];
    target[28] = 0x00; // 使目标稍微困难一点
    target[29] = 0x00;
    target[30] = 0x00;
    target[31] = 0x0f;

    Work::new("test_job".to_string(), target, block_header, 1.0)
}
