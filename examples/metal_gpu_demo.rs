//! Mac Metal GPU 挖矿演示
//!
//! 专门演示 Mac M4 GPU 的 Metal 计算着色器挖矿能力

#[cfg(feature = "mac-metal")]
use cgminer_gpu_btc_core::MetalDevice;
use cgminer_core::{DeviceInfo, DeviceConfig, DeviceType, Work};
use tokio::time::{sleep, Duration};
use tracing::{info, warn, error};
use uuid::Uuid;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 初始化日志
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .init();

    info!("🍎 启动 Mac Metal GPU 挖矿演示");

    #[cfg(feature = "mac-metal")]
    {
        run_metal_demo().await?;
    }

    #[cfg(not(feature = "mac-metal"))]
    {
        warn!("⚠️  Metal 功能未启用，无法运行 Metal GPU 演示");
        info!("请使用 --features mac-metal 编译以启用 Metal 支持");
    }

    Ok(())
}

#[cfg(feature = "mac-metal")]
async fn run_metal_demo() -> Result<(), Box<dyn std::error::Error>> {
    info!("🔧 初始化 Metal GPU 设备");

    // 创建 Metal 设备信息
    let device_info = DeviceInfo {
        id: 0,
        name: "Mac M4 GPU".to_string(),
        device_type: DeviceType::Gpu,
        vendor: "Apple".to_string(),
        driver_version: "Metal 3.0".to_string(),
    };

    // 创建设备配置
    let device_config = DeviceConfig {
        enabled: true,
        max_hashrate: Some(2_000_000_000_000.0), // 2 TH/s for M4 GPU
        target_temperature: Some(75.0),
        power_limit: Some(50.0), // 50W
        custom_params: std::collections::HashMap::new(),
    };

    // 创建 Metal 设备
    let mut metal_device = MetalDevice::new(device_info, device_config).await?;

    info!("✅ Metal GPU 设备创建成功");

    // 初始化设备
    metal_device.initialize(device_config.clone()).await?;
    info!("🚀 Metal GPU 设备初始化完成");

    // 启动设备
    metal_device.start().await?;
    info!("🔥 Metal GPU 设备启动完成");

    // 创建高难度测试工作
    let work = create_metal_test_work();
    info!("📋 创建 Metal 测试工作: {}", work.id);

    // 提交工作
    metal_device.submit_work(work.clone()).await?;
    info!("📤 工作已提交到 Metal GPU");

    // 运行挖矿测试
    let mut total_solutions = 0;
    let start_time = std::time::Instant::now();

    info!("⛏️  开始 Metal GPU 挖矿测试 (60秒)...");

    for round in 1..=12 { // 12轮，每轮5秒
        info!("🔄 Metal 挖矿轮次 {}/12", round);

        // 等待 Metal GPU 计算
        sleep(Duration::from_secs(5)).await;

        // 收集结果
        let mut round_solutions = 0;
        while let Ok(Some(result)) = metal_device.get_result().await {
            total_solutions += 1;
            round_solutions += 1;
            info!("💎 Metal GPU 找到解: nonce=0x{:08x}, 轮次解数={}",
                  result.nonce, round_solutions);
        }

        // 获取设备统计信息
        if let Ok(stats) = metal_device.get_stats().await {
            info!("📊 Metal GPU 统计: 算力={:.2} GH/s, 总哈希={}",
                  stats.hashrate / 1_000_000_000.0, stats.total_hashes);
        }

        // 健康检查
        if let Ok(healthy) = metal_device.health_check().await {
            if !healthy {
                warn!("⚠️  Metal GPU 设备健康检查失败");
            }
        }

        // 提交新工作
        let new_work = create_metal_test_work();
        metal_device.submit_work(new_work).await?;
    }

    let elapsed = start_time.elapsed();
    info!("⏱️  Metal GPU 挖矿测试完成!");
    info!("   总时间: {:.2}s", elapsed.as_secs_f64());
    info!("   总解数: {}", total_solutions);
    info!("   平均: {:.2} 解/秒", total_solutions as f64 / elapsed.as_secs_f64());

    // 最终统计
    if let Ok(final_stats) = metal_device.get_stats().await {
        info!("📈 Metal GPU 最终统计:");
        info!("   总算力: {:.2} GH/s", final_stats.hashrate / 1_000_000_000.0);
        info!("   总哈希: {}", final_stats.total_hashes);
        info!("   运行时间: {:.2}s", elapsed.as_secs_f64());

        if final_stats.total_hashes > 0 {
            let efficiency = total_solutions as f64 / final_stats.total_hashes as f64 * 1_000_000.0;
            info!("   挖矿效率: {:.2} 解/MH", efficiency);
        }
    }

    // 停止设备
    metal_device.stop().await?;
    info!("🛑 Metal GPU 设备已停止");

    // 关闭设备
    metal_device.shutdown().await?;
    info!("🔌 Metal GPU 设备已关闭");

    info!("✅ Mac Metal GPU 挖矿演示完成");
    Ok(())
}

/// 创建 Metal 测试工作
fn create_metal_test_work() -> Work {
    // 创建一个适合 Metal GPU 的测试区块头
    let mut block_header = [0u8; 80];

    // 设置版本
    block_header[0..4].copy_from_slice(&0x20000000u32.to_le_bytes()); // version 2

    // 设置随机的前一个区块哈希
    let prev_hash: [u8; 32] = std::array::from_fn(|_| fastrand::u8(..));
    block_header[4..36].copy_from_slice(&prev_hash);

    // 设置随机的 Merkle 根
    let merkle_root: [u8; 32] = std::array::from_fn(|_| fastrand::u8(..));
    block_header[36..68].copy_from_slice(&merkle_root);

    // 设置时间戳
    let timestamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs() as u32;
    block_header[68..72].copy_from_slice(&timestamp.to_le_bytes());

    // 设置难度位
    block_header[72..76].copy_from_slice(&0x1d00ffffu32.to_le_bytes()); // bits

    // nonce 将由 Metal 着色器设置
    block_header[76..80].copy_from_slice(&0u32.to_le_bytes()); // nonce

    // 设置适合 Metal GPU 的目标难度
    let mut target = [0xffu8; 32];
    // 设置一个中等难度，让 Metal GPU 能够找到解但不会太容易
    target[28] = 0x00;
    target[29] = 0x00;
    target[30] = 0x0f;
    target[31] = 0xff;

    Work::new("metal_job".to_string(), target, block_header, 16.0)
}

#[cfg(not(feature = "mac-metal"))]
async fn run_metal_demo() -> Result<(), Box<dyn std::error::Error>> {
    // 空实现，用于非 Metal 平台
    Ok(())
}
