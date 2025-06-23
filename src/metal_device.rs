//! Mac Metal GPU 设备实现
//!
//! 专门为 Mac M4 GPU 优化的设备实现，使用 Metal 计算着色器
//! 进行高性能的 SHA256d 并行计算。

use cgminer_core::{
    MiningDevice, DeviceInfo, DeviceConfig, DeviceStats, DeviceError,
    Work, MiningResult, HashRate
};
use crate::metal_backend::{MetalBackend, MetalDeviceInfo};
use async_trait::async_trait;
use std::sync::{Arc, RwLock};
use std::time::{Duration, SystemTime};
use tokio::sync::Mutex;
use tracing::{info, warn, error, debug};
use uuid::Uuid;

/// Mac Metal GPU 设备
pub struct MetalDevice {
    /// 设备信息
    device_info: DeviceInfo,
    /// 设备配置
    config: DeviceConfig,
    /// 设备统计信息
    stats: Arc<RwLock<DeviceStats>>,
    /// Metal 后端
    metal_backend: Arc<Mutex<MetalBackend>>,
    /// 是否正在运行
    running: Arc<RwLock<bool>>,
    /// 当前工作
    current_work: Arc<Mutex<Option<Work>>>,
    /// 启动时间
    start_time: Option<SystemTime>,
    /// 结果队列
    result_queue: Arc<Mutex<Vec<MiningResult>>>,
    /// 工作计数器
    work_counter: Arc<RwLock<u64>>,
}

impl MetalDevice {
    /// 创建新的 Metal GPU 设备
    pub async fn new(
        device_info: DeviceInfo,
        config: DeviceConfig,
    ) -> Result<Self, DeviceError> {
        info!("🍎 创建 Mac Metal GPU 设备: {}", device_info.name);

        // 创建 Metal 后端
        let mut metal_backend = MetalBackend::new()?;
        metal_backend.initialize().await?;

        let stats = DeviceStats::new(device_info.id);

        Ok(Self {
            device_info,
            config,
            stats: Arc::new(RwLock::new(stats)),
            metal_backend: Arc::new(Mutex::new(metal_backend)),
            running: Arc::new(RwLock::new(false)),
            current_work: Arc::new(Mutex::new(None)),
            start_time: None,
            result_queue: Arc::new(Mutex::new(Vec::new())),
            work_counter: Arc::new(RwLock::new(0)),
        })
    }

    /// 启动挖矿循环
    async fn start_mining_loop(&self) -> Result<(), DeviceError> {
        let device_name = self.device_info.name.clone();
        let _device_id = self.device_info.id;
        let running = self.running.clone();
        let current_work = self.current_work.clone();
        let result_queue = self.result_queue.clone();
        let metal_backend = self.metal_backend.clone();
        let stats = self.stats.clone();

        tokio::spawn(async move {
            info!("🔄 Mac Metal GPU 设备 {} 挖矿循环启动", device_name);

            while running.read().map(|r| *r).unwrap_or_else(|_| {
                error!("获取运行状态失败");
                false
            }) {
                // 获取当前工作
                let work = {
                    let work_guard = current_work.lock().await;
                    work_guard.clone()
                };

                if let Some(work) = work {
                    let compute_start = SystemTime::now();

                    // 使用 Metal 后端进行挖矿
                    let backend = metal_backend.lock().await;
                    let optimal_work_size = backend.get_optimal_work_size();

                    match backend.mine(&work, 0, optimal_work_size).await {
                        Ok(results) => {
                            if !results.is_empty() {
                                let mut queue = result_queue.lock().await;
                                queue.extend(results.clone());

                                info!("🎯 Mac Metal GPU 设备 {} 找到 {} 个有效解!",
                                     device_name, results.len());
                            }

                            // 更新统计信息
                            if let Ok(mut stats) = stats.write() {
                                let compute_time = SystemTime::now()
                                    .duration_since(compute_start)
                                    .unwrap_or_default()
                                    .as_secs_f64();

                                if compute_time > 0.0 {
                                    stats.update_hashrate(optimal_work_size as u64, compute_time);
                                }
                            }
                        }
                        Err(e) => {
                            error!("Mac Metal GPU 设备 {} 挖矿失败: {}", device_name, e);
                            tokio::time::sleep(Duration::from_millis(1000)).await;
                        }
                    }

                    // 控制挖矿频率，避免过度消耗资源
                    let compute_time = SystemTime::now()
                        .duration_since(compute_start)
                        .unwrap_or_default();

                    let target_interval = Duration::from_millis(50); // 目标50ms间隔
                    if compute_time < target_interval {
                        tokio::time::sleep(target_interval - compute_time).await;
                    }
                } else {
                    // 没有工作，等待
                    tokio::time::sleep(Duration::from_millis(100)).await;
                }
            }

            info!("🛑 Mac Metal GPU 设备 {} 挖矿循环停止", device_name);
        });

        Ok(())
    }

    /// 更新设备统计信息
    fn update_stats(&self, hashes_computed: u64) -> Result<(), DeviceError> {
        let mut stats = self.stats.write().map_err(|e| {
            DeviceError::hardware_error(format!("获取统计信息锁失败: {}", e))
        })?;

        let now = SystemTime::now();

        if let Some(start_time) = self.start_time {
            let elapsed = now
                .duration_since(start_time)
                .unwrap_or_default()
                .as_secs_f64();

            if elapsed > 0.0 {
                stats.update_hashrate(hashes_computed, elapsed);
            }
        }

        stats.last_updated = now;
        Ok(())
    }

    /// 获取 Metal 设备信息
    pub async fn get_metal_info(&self) -> Result<MetalDeviceInfo, DeviceError> {
        let backend = self.metal_backend.lock().await;
        Ok(backend.get_device_info().clone())
    }
}

#[async_trait]
impl MiningDevice for MetalDevice {
    /// 获取设备ID
    fn device_id(&self) -> u32 {
        self.device_info.id
    }

    /// 获取设备信息
    async fn get_info(&self) -> Result<DeviceInfo, DeviceError> {
        Ok(self.device_info.clone())
    }

    /// 初始化设备
    async fn initialize(&mut self, config: DeviceConfig) -> Result<(), DeviceError> {
        info!("🚀 初始化 Mac Metal GPU 设备: {}", self.device_info.name);

        self.config = config;

        // 确保 Metal 后端已初始化
        let mut backend = self.metal_backend.lock().await;
        if !backend.is_initialized() {
            backend.initialize().await?;
        }

        info!("✅ Mac Metal GPU 设备 {} 初始化完成", self.device_info.name);
        Ok(())
    }

    /// 启动设备
    async fn start(&mut self) -> Result<(), DeviceError> {
        info!("🔥 启动 Mac Metal GPU 设备: {}", self.device_info.name);

        // 检查运行状态
        {
            let running = self.running.read().map_err(|e| {
                DeviceError::hardware_error(format!("获取运行状态锁失败: {}", e))
            })?;

            if *running {
                warn!("Mac Metal GPU 设备 {} 已经在运行", self.device_info.name);
                return Ok(());
            }
        }

        // 启动挖矿循环
        self.start_mining_loop().await?;

        // 设置运行状态
        {
            let mut running = self.running.write().map_err(|e| {
                DeviceError::hardware_error(format!("获取运行状态锁失败: {}", e))
            })?;
            *running = true;
        }

        self.start_time = Some(SystemTime::now());

        info!("✅ Mac Metal GPU 设备 {} 启动完成", self.device_info.name);
        Ok(())
    }

    /// 停止设备
    async fn stop(&mut self) -> Result<(), DeviceError> {
        info!("🛑 停止 Mac Metal GPU 设备: {}", self.device_info.name);

        // 检查运行状态
        {
            let running = self.running.read().map_err(|e| {
                DeviceError::hardware_error(format!("获取运行状态锁失败: {}", e))
            })?;

            if !*running {
                warn!("Mac Metal GPU 设备 {} 已经停止", self.device_info.name);
                return Ok(());
            }
        }

        // 等待挖矿循环停止
        tokio::time::sleep(Duration::from_millis(200)).await;

        // 设置停止状态
        {
            let mut running = self.running.write().map_err(|e| {
                DeviceError::hardware_error(format!("获取运行状态锁失败: {}", e))
            })?;
            *running = false;
        }

        info!("✅ Mac Metal GPU 设备 {} 停止完成", self.device_info.name);
        Ok(())
    }

    /// 重启设备
    async fn restart(&mut self) -> Result<(), DeviceError> {
        info!("🔄 重启 Mac Metal GPU 设备: {}", self.device_info.name);
        self.stop().await?;
        tokio::time::sleep(Duration::from_millis(500)).await;
        self.start().await?;
        info!("✅ Mac Metal GPU 设备 {} 重启完成", self.device_info.name);
        Ok(())
    }

    /// 提交工作
    async fn submit_work(&mut self, work: Work) -> Result<(), DeviceError> {
        debug!("📤 向 Mac Metal GPU 设备 {} 提交工作", self.device_info.name);

        let mut current_work = self.current_work.lock().await;
        *current_work = Some(work);

        // 增加工作计数器
        let mut counter = self.work_counter.write().map_err(|e| {
            DeviceError::hardware_error(format!("获取工作计数器锁失败: {}", e))
        })?;
        *counter += 1;

        debug!("✅ 工作提交到 Mac Metal GPU 设备 {} 成功", self.device_info.name);
        Ok(())
    }

    /// 获取挖矿结果
    async fn get_result(&mut self) -> Result<Option<MiningResult>, DeviceError> {
        let mut queue = self.result_queue.lock().await;

        if let Some(mut result) = queue.pop() {
            // 设置正确的设备ID
            result.device_id = self.device_info.id;

            debug!("📥 从 Mac Metal GPU 设备 {} 获取到挖矿结果", self.device_info.name);

            // 更新统计信息
            self.update_stats(1000000)?; // 假设每个结果代表100万次哈希计算

            Ok(Some(result))
        } else {
            Ok(None)
        }
    }

    /// 获取设备状态
    async fn get_status(&self) -> Result<cgminer_core::DeviceStatus, DeviceError> {
        let running = self.running.read().map_err(|e| {
            DeviceError::hardware_error(format!("获取运行状态锁失败: {}", e))
        })?;

        if *running {
            Ok(cgminer_core::DeviceStatus::Running)
        } else {
            Ok(cgminer_core::DeviceStatus::Idle)
        }
    }

    /// 获取设备统计信息
    async fn get_stats(&self) -> Result<DeviceStats, DeviceError> {
        let stats = self.stats.read().map_err(|e| {
            DeviceError::hardware_error(format!("获取统计信息锁失败: {}", e))
        })?;
        Ok(stats.clone())
    }

    /// 设置频率 (Metal GPU 不支持)
    async fn set_frequency(&mut self, _frequency: u32) -> Result<(), DeviceError> {
        Err(DeviceError::unsupported_operation("Mac Metal GPU 不支持频率设置".to_string()))
    }

    /// 设置电压 (Metal GPU 不支持)
    async fn set_voltage(&mut self, _voltage: u32) -> Result<(), DeviceError> {
        Err(DeviceError::unsupported_operation("Mac Metal GPU 不支持电压设置".to_string()))
    }

    /// 设置风扇速度 (Metal GPU 不支持)
    async fn set_fan_speed(&mut self, _speed: u32) -> Result<(), DeviceError> {
        Err(DeviceError::unsupported_operation("Mac Metal GPU 不支持风扇控制".to_string()))
    }

    /// 重置设备
    async fn reset(&mut self) -> Result<(), DeviceError> {
        info!("🔄 重置 Mac Metal GPU 设备: {}", self.device_info.name);

        // 停止设备
        self.stop().await?;

        // 清理状态
        let mut queue = self.result_queue.lock().await;
        queue.clear();

        let mut current_work = self.current_work.lock().await;
        *current_work = None;

        // 重新初始化 Metal 后端
        let mut backend = self.metal_backend.lock().await;
        *backend = MetalBackend::new()?;
        backend.initialize().await?;

        info!("✅ Mac Metal GPU 设备 {} 重置完成", self.device_info.name);
        Ok(())
    }

    /// 健康检查
    async fn health_check(&self) -> Result<bool, DeviceError> {
        debug!("🏥 Mac Metal GPU 设备 {} 健康检查", self.device_info.name);

        // 检查运行状态
        let is_running = {
            let running = self.running.read().map_err(|e| {
                DeviceError::hardware_error(format!("获取运行状态锁失败: {}", e))
            })?;
            *running
        };

        if !is_running {
            return Ok(false);
        }

        // 检查 Metal 后端状态
        let backend = self.metal_backend.lock().await;
        if !backend.is_initialized() {
            return Ok(false);
        }

        debug!("✅ Mac Metal GPU 设备 {} 健康检查通过", self.device_info.name);
        Ok(true)
    }
}
