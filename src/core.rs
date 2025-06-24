//! GPU挖矿核心实现

use cgminer_core::{
    MiningCore, CoreInfo, CoreCapabilities, CoreConfig, CoreStats, CoreError,
    DeviceInfo, MiningDevice, Work, MiningResult
};
use crate::device::GpuDevice;
use crate::gpu_manager::GpuManager;

#[cfg(feature = "mac-metal")]
use crate::metal_device::MetalDevice;
use async_trait::async_trait;
use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use std::time::{Duration, SystemTime};
use tokio::sync::Mutex;
use tracing::{info, warn, error, debug};

/// GPU挖矿核心
pub struct GpuMiningCore {
    /// 核心信息
    core_info: CoreInfo,
    /// 核心能力
    capabilities: CoreCapabilities,
    /// 核心配置
    config: Option<CoreConfig>,
    /// 设备列表
    devices: Arc<Mutex<HashMap<u32, Box<dyn MiningDevice>>>>,
    /// 核心统计信息
    stats: Arc<RwLock<CoreStats>>,
    /// 是否正在运行
    running: Arc<RwLock<bool>>,
    /// 启动时间
    start_time: Option<SystemTime>,
    /// GPU管理器
    gpu_manager: Option<Arc<GpuManager>>,
}

impl GpuMiningCore {
    /// 创建新的GPU挖矿核心
    pub fn new(name: String) -> Self {
        let mut supported_devices = vec!["gpu".to_string()];

        // 根据编译特性添加支持的设备类型
        #[cfg(feature = "mac-metal")]
        supported_devices.push("mac-metal".to_string());

        #[cfg(feature = "opencl")]
        supported_devices.push("opencl".to_string());

        #[cfg(feature = "cuda")]
        supported_devices.push("cuda".to_string());

        let core_info = CoreInfo::new(
            name.clone(),
            cgminer_core::CoreType::Custom("gpu".to_string()),
            crate::VERSION.to_string(),
            "GPU挖矿核心，支持Mac M4 Metal、OpenCL、CUDA等多种GPU平台".to_string(),
            "CGMiner Rust Team".to_string(),
            supported_devices,
        );

        let capabilities = CoreCapabilities {
            supports_auto_tuning: true,
            temperature_capabilities: cgminer_core::TemperatureCapabilities {
                supports_monitoring: true,
                supports_control: true,
                supports_threshold_alerts: true,
                monitoring_precision: Some(1.0),
            },
            voltage_capabilities: cgminer_core::VoltageCapabilities {
                supports_monitoring: true,
                supports_control: true,
                control_range: Some((800, 1200)), // GPU电压范围
            },
            frequency_capabilities: cgminer_core::FrequencyCapabilities {
                supports_monitoring: true,
                supports_control: true,
                control_range: Some((500, 2000)), // GPU频率范围 (MHz)
            },
            fan_capabilities: cgminer_core::FanCapabilities {
                supports_monitoring: true,
                supports_control: true,
                fan_count: Some(2), // 大多数GPU有2个风扇
            },
            supports_multiple_chains: false, // GPU通常不支持多链
            max_devices: Some(16), // GPU核心支持最多16个GPU设备
            supported_algorithms: vec!["SHA256".to_string(), "SHA256d".to_string()],
            cpu_capabilities: None, // GPU核心不使用CPU特有能力
            core_type: cgminer_core::CoreType::Custom("gpu".to_string()),
        };

        let stats = CoreStats::new(name);

        Self {
            core_info,
            capabilities,
            config: None,
            devices: Arc::new(Mutex::new(HashMap::new())),
            stats: Arc::new(RwLock::new(stats)),
            running: Arc::new(RwLock::new(false)),
            start_time: None,
            gpu_manager: None,
        }
    }

    /// 获取GPU管理器
    pub fn gpu_manager(&self) -> Option<Arc<GpuManager>> {
        self.gpu_manager.clone()
    }

    /// 设置GPU管理器
    pub fn set_gpu_manager(&mut self, manager: Arc<GpuManager>) {
        self.gpu_manager = Some(manager);
    }
}

#[async_trait]
impl MiningCore for GpuMiningCore {
    /// 获取核心信息
    fn get_info(&self) -> &CoreInfo {
        &self.core_info
    }

    /// 获取核心能力
    fn get_capabilities(&self) -> &CoreCapabilities {
        &self.capabilities
    }

    /// 初始化核心
    async fn initialize(&mut self, config: CoreConfig) -> Result<(), CoreError> {
        info!("🚀 初始化GPU挖矿核心: {}", config.name);

        // 验证配置
        self.validate_config(&config)?;

        // 初始化GPU管理器
        let gpu_manager = Arc::new(GpuManager::new()?);
        gpu_manager.initialize().await?;
        self.gpu_manager = Some(gpu_manager);

        // 保存配置
        self.config = Some(config);

        info!("✅ GPU挖矿核心初始化完成");
        Ok(())
    }

    /// 启动核心
    async fn start(&mut self) -> Result<(), CoreError> {
        info!("🔥 启动GPU挖矿核心");

        // 检查运行状态
        {
            let running = self.running.read().map_err(|e| {
                CoreError::runtime(format!("获取运行状态锁失败: {}", e))
            })?;

            if *running {
                warn!("GPU挖矿核心已经在运行");
                return Ok(());
            }
        }

        // 扫描并创建设备
        let device_infos = self.scan_devices().await?;
        info!("🔍 发现 {} 个GPU设备", device_infos.len());

        let mut devices = self.devices.lock().await;
        for device_info in device_infos {
            match self.create_device(device_info.clone()).await {
                Ok(mut device) => {
                    // 初始化设备
                    let device_config = cgminer_core::DeviceConfig::default();
                    if let Err(e) = device.initialize(device_config).await {
                        error!("初始化GPU设备 {} 失败: {}", device_info.id, e);
                        continue;
                    }

                    // 启动设备
                    if let Err(e) = device.start().await {
                        error!("启动GPU设备 {} 失败: {}", device_info.id, e);
                        continue;
                    }

                    devices.insert(device_info.id, device);
                    info!("✅ GPU设备 {} 启动成功", device_info.id);
                }
                Err(e) => {
                    error!("创建GPU设备 {} 失败: {}", device_info.id, e);
                }
            }
        }

        // 设置运行状态
        {
            let mut running = self.running.write().map_err(|e| {
                CoreError::runtime(format!("获取运行状态锁失败: {}", e))
            })?;
            *running = true;
        }

        self.start_time = Some(SystemTime::now());

        info!("🎉 GPU挖矿核心启动完成，共启动 {} 个设备", devices.len());
        Ok(())
    }

    /// 停止核心
    async fn stop(&mut self) -> Result<(), CoreError> {
        info!("🛑 停止GPU挖矿核心");

        // 检查运行状态
        {
            let running = self.running.read().map_err(|e| {
                CoreError::runtime(format!("获取运行状态锁失败: {}", e))
            })?;

            if !*running {
                warn!("GPU挖矿核心已经停止");
                return Ok(());
            }
        }

        // 停止所有设备
        let mut devices = self.devices.lock().await;
        for (device_id, device) in devices.iter_mut() {
            if let Err(e) = device.stop().await {
                error!("停止GPU设备 {} 失败: {}", device_id, e);
            } else {
                info!("✅ GPU设备 {} 停止成功", device_id);
            }
        }

        // 设置停止状态
        {
            let mut running = self.running.write().map_err(|e| {
                CoreError::runtime(format!("获取运行状态锁失败: {}", e))
            })?;
            *running = false;
        }

        info!("✅ GPU挖矿核心停止完成");
        Ok(())
    }

    /// 重启核心
    async fn restart(&mut self) -> Result<(), CoreError> {
        info!("🔄 重启GPU挖矿核心");
        self.stop().await?;
        tokio::time::sleep(Duration::from_secs(1)).await;
        self.start().await?;
        info!("✅ GPU挖矿核心重启完成");
        Ok(())
    }

    /// 扫描设备
    async fn scan_devices(&self) -> Result<Vec<DeviceInfo>, CoreError> {
        debug!("🔍 扫描GPU设备");

        let mut device_infos = Vec::new();

        // 优先扫描 Mac Metal 设备
        #[cfg(all(feature = "mac-metal", target_os = "macos"))]
        {
            use crate::metal_backend::MetalBackend;

            match MetalBackend::new() {
                Ok(backend) => {
                    let metal_info = backend.get_device_info();
                    let device_info = DeviceInfo::new(
                        0,
                        format!("Mac Metal GPU: {}", metal_info.name),
                        "mac-metal".to_string(),
                        0,
                    );
                    device_infos.push(device_info);
                    info!("🍎 发现 Mac Metal GPU: {}", metal_info.name);
                }
                Err(e) => {
                    debug!("Mac Metal GPU 不可用: {}", e);
                }
            }
        }

        // 如果没有找到 Metal 设备，尝试其他 GPU
        if device_infos.is_empty() {
            if let Some(gpu_manager) = &self.gpu_manager {
                let gpu_infos = gpu_manager.scan_gpus().await?;

                for (index, _gpu_info) in gpu_infos.iter().enumerate() {
                    let device_info = DeviceInfo::new(
                        index as u32,
                        format!("GPU-{}", index),
                        "gpu".to_string(),
                        0, // GPU通常没有链的概念
                    );
                    device_infos.push(device_info);
                }
            }
        }

        debug!("✅ 扫描到 {} 个GPU设备", device_infos.len());
        Ok(device_infos)
    }

    /// 创建设备
    async fn create_device(&self, device_info: DeviceInfo) -> Result<Box<dyn MiningDevice>, CoreError> {
        info!("🏭 创建GPU设备: {}", device_info.name);

        let device_config = cgminer_core::DeviceConfig::default();

        // 根据平台和特性选择设备类型
        #[cfg(all(feature = "mac-metal", target_os = "macos"))]
        {
            info!("🍎 创建 Mac Metal GPU 设备");
            let device = MetalDevice::new(device_info, device_config).await?;
            info!("✅ Mac Metal GPU 设备创建成功");
            return Ok(Box::new(device));
        }

        // 回退到通用 GPU 设备
        #[cfg(not(all(feature = "mac-metal", target_os = "macos")))]
        {
            let gpu_manager = self.gpu_manager.as_ref()
                .ok_or_else(|| CoreError::runtime("GPU管理器未初始化".to_string()))?;

        // 从配置中获取参数
        let default_config = CoreConfig::default();
        let config = self.config.as_ref().unwrap_or(&default_config);

        let target_hashrate = config.custom_params
            .get("max_hashrate")
            .and_then(|v| v.as_f64())
            .unwrap_or(1_000_000_000_000.0); // 1 TH/s 默认算力

        let device = GpuDevice::new(
            device_info,
            device_config,
            target_hashrate,
            gpu_manager.clone(),
        ).await?;

            info!("✅ 通用 GPU 设备创建成功");
            Ok(Box::new(device))
        }
    }

    /// 获取所有设备
    async fn get_devices(&self) -> Result<Vec<Box<dyn MiningDevice>>, CoreError> {
        // 由于 MiningDevice trait 没有 Clone，我们返回设备信息而不是设备本身
        Err(CoreError::runtime("获取设备列表功能需要重新设计接口".to_string()))
    }

    /// 获取设备数量
    async fn device_count(&self) -> Result<u32, CoreError> {
        let devices = self.devices.lock().await;
        Ok(devices.len() as u32)
    }

    /// 提交工作到所有设备
    async fn submit_work(&mut self, work: Arc<Work>) -> Result<(), CoreError> {
        debug!("📤 提交工作到所有GPU设备");

        let mut devices = self.devices.lock().await;
        let mut submitted_count = 0;

        for (device_id, device) in devices.iter_mut() {
            match device.submit_work(work.clone()).await {
                Ok(()) => {
                    submitted_count += 1;
                    debug!("✅ 工作提交到GPU设备 {} 成功", device_id);
                }
                Err(e) => {
                    error!("❌ 工作提交到GPU设备 {} 失败: {}", device_id, e);
                }
            }
        }

        if submitted_count > 0 {
            debug!("📤 工作提交完成，成功提交到 {} 个设备", submitted_count);
            Ok(())
        } else {
            Err(CoreError::runtime("没有设备成功接收工作".to_string()))
        }
    }

    /// 收集所有设备的挖矿结果
    async fn collect_results(&mut self) -> Result<Vec<MiningResult>, CoreError> {
        debug!("📥 收集所有GPU设备的挖矿结果");

        let mut devices = self.devices.lock().await;
        let mut results = Vec::new();

        for (device_id, device) in devices.iter_mut() {
            match device.get_result().await {
                Ok(Some(result)) => {
                    debug!("✅ 从GPU设备 {} 收集到挖矿结果", device_id);
                    results.push(result);
                }
                Ok(None) => {
                    // 设备暂无结果，这是正常的
                }
                Err(e) => {
                    error!("❌ 从GPU设备 {} 收集结果失败: {}", device_id, e);
                }
            }
        }

        debug!("📥 结果收集完成，共收集到 {} 个结果", results.len());
        Ok(results)
    }

    /// 获取核心统计信息
    async fn get_stats(&self) -> Result<CoreStats, CoreError> {
        let stats = self.stats.read().map_err(|e| {
            CoreError::runtime(format!("获取统计信息锁失败: {}", e))
        })?;
        Ok(stats.clone())
    }

    /// 健康检查
    async fn health_check(&self) -> Result<bool, CoreError> {
        debug!("🏥 GPU挖矿核心健康检查");

        // 检查运行状态
        let is_running = {
            let running = self.running.read().map_err(|e| {
                CoreError::runtime(format!("获取运行状态锁失败: {}", e))
            })?;
            *running
        };

        if !is_running {
            return Ok(false);
        }

        // 检查GPU管理器
        if let Some(gpu_manager) = &self.gpu_manager {
            if !gpu_manager.is_healthy().await {
                return Ok(false);
            }
        } else {
            return Ok(false);
        }

        // 检查设备健康状态
        let devices = self.devices.lock().await;
        for (device_id, device) in devices.iter() {
            match device.health_check().await {
                Ok(healthy) => {
                    if !healthy {
                        warn!("GPU设备 {} 健康检查失败", device_id);
                        return Ok(false);
                    }
                }
                Err(e) => {
                    error!("GPU设备 {} 健康检查出错: {}", device_id, e);
                    return Ok(false);
                }
            }
        }

        debug!("✅ GPU挖矿核心健康检查通过");
        Ok(true)
    }

    /// 验证配置
    fn validate_config(&self, config: &CoreConfig) -> Result<(), CoreError> {
        debug!("🔍 验证GPU挖矿核心配置");

        if config.name.is_empty() {
            return Err(CoreError::config("核心名称不能为空".to_string()));
        }

        // 验证GPU特定配置
        if let Some(max_hashrate) = config.custom_params.get("max_hashrate") {
            if let Some(hashrate) = max_hashrate.as_f64() {
                if hashrate <= 0.0 {
                    return Err(CoreError::config("最大算力必须大于0".to_string()));
                }
            }
        }

        debug!("✅ GPU挖矿核心配置验证通过");
        Ok(())
    }

    /// 获取默认配置
    fn default_config(&self) -> CoreConfig {
        let mut config = CoreConfig::default();
        config.name = "GPU Mining Core".to_string();

        // 设置GPU特定的默认参数
        config.custom_params.insert(
            "max_hashrate".to_string(),
            serde_json::Value::Number(serde_json::Number::from_f64(1_000_000_000_000.0).unwrap())
        );
        config.custom_params.insert(
            "device_count".to_string(),
            serde_json::Value::Number(serde_json::Number::from(8))
        );
        config.custom_params.insert(
            "work_size".to_string(),
            serde_json::Value::Number(serde_json::Number::from(256))
        );

        config
    }

    /// 关闭核心
    async fn shutdown(&mut self) -> Result<(), CoreError> {
        info!("🔌 关闭GPU挖矿核心");

        // 停止核心
        self.stop().await?;

        // 清理资源
        let mut devices = self.devices.lock().await;
        devices.clear();

        // 关闭GPU管理器
        if let Some(gpu_manager) = &self.gpu_manager {
            gpu_manager.shutdown().await?;
        }

        info!("✅ GPU挖矿核心关闭完成");
        Ok(())
    }
}
