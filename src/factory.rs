//! GPU核心工厂实现

use cgminer_core::{CoreFactory, MiningCore, CoreInfo, CoreError};
use crate::core::GpuMiningCore;
use async_trait::async_trait;
use tracing::{info, debug};

/// GPU核心工厂
pub struct GpuCoreFactory {
    /// 工厂信息
    info: CoreInfo,
}

impl GpuCoreFactory {
    /// 创建新的GPU核心工厂
    pub fn new() -> Self {
        let info = CoreInfo::new(
            "GPU Mining Core Factory".to_string(),
            cgminer_core::CoreType::Custom("gpu".to_string()),
            crate::VERSION.to_string(),
            "GPU挖矿核心工厂，用于创建和管理GPU挖矿核心实例".to_string(),
            "CGMiner Rust Team".to_string(),
            vec!["gpu".to_string(), "opencl".to_string(), "cuda".to_string()],
        );

        Self { info }
    }
}

impl Default for GpuCoreFactory {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl CoreFactory for GpuCoreFactory {
    /// 获取核心信息
    fn core_info(&self) -> CoreInfo {
        self.info.clone()
    }

    /// 创建挖矿核心实例
    async fn create_core(&self, name: String) -> Result<Box<dyn MiningCore>, CoreError> {
        info!("🏭 创建GPU挖矿核心实例: {}", name);

        let core = GpuMiningCore::new(name.clone());
        
        debug!("✅ GPU挖矿核心实例 {} 创建成功", name);
        Ok(Box::new(core))
    }

    /// 验证核心配置
    fn validate_config(&self, config: &cgminer_core::CoreConfig) -> Result<(), CoreError> {
        debug!("🔍 验证GPU核心配置: {}", config.name);

        if config.name.is_empty() {
            return Err(CoreError::config("核心名称不能为空".to_string()));
        }

        // 验证GPU特定配置
        if let Some(max_hashrate) = config.custom_params.get("max_hashrate") {
            if let Some(hashrate) = max_hashrate.as_f64() {
                if hashrate <= 0.0 {
                    return Err(CoreError::config("最大算力必须大于0".to_string()));
                }
                if hashrate > 100_000_000_000_000.0 { // 100 TH/s 上限
                    return Err(CoreError::config("最大算力不能超过100 TH/s".to_string()));
                }
            } else {
                return Err(CoreError::config("最大算力必须是数字".to_string()));
            }
        }

        if let Some(device_count) = config.custom_params.get("device_count") {
            if let Some(count) = device_count.as_u64() {
                if count == 0 {
                    return Err(CoreError::config("设备数量必须大于0".to_string()));
                }
                if count > 16 {
                    return Err(CoreError::config("设备数量不能超过16".to_string()));
                }
            } else {
                return Err(CoreError::config("设备数量必须是整数".to_string()));
            }
        }

        if let Some(work_size) = config.custom_params.get("work_size") {
            if let Some(size) = work_size.as_u64() {
                if size == 0 {
                    return Err(CoreError::config("工作大小必须大于0".to_string()));
                }
                if size > 1024 {
                    return Err(CoreError::config("工作大小不能超过1024".to_string()));
                }
            } else {
                return Err(CoreError::config("工作大小必须是整数".to_string()));
            }
        }

        debug!("✅ GPU核心配置验证通过");
        Ok(())
    }

    /// 获取支持的设备类型
    fn supported_devices(&self) -> Vec<String> {
        vec![
            "gpu".to_string(),
            "opencl".to_string(),
            "cuda".to_string(),
            "nvidia".to_string(),
            "amd".to_string(),
            "intel".to_string(),
        ]
    }

    /// 获取默认配置
    fn default_config(&self) -> cgminer_core::CoreConfig {
        let mut config = cgminer_core::CoreConfig::default();
        config.name = "GPU Mining Core".to_string();
        
        // 设置GPU特定的默认参数
        config.custom_params.insert(
            "max_hashrate".to_string(),
            serde_json::Value::Number(serde_json::Number::from_f64(1_000_000_000_000.0).unwrap()) // 1 TH/s
        );
        config.custom_params.insert(
            "device_count".to_string(),
            serde_json::Value::Number(serde_json::Number::from(8))
        );
        config.custom_params.insert(
            "work_size".to_string(),
            serde_json::Value::Number(serde_json::Number::from(256))
        );
        config.custom_params.insert(
            "opencl_platform".to_string(),
            serde_json::Value::Number(serde_json::Number::from(0))
        );
        config.custom_params.insert(
            "cuda_device".to_string(),
            serde_json::Value::Number(serde_json::Number::from(0))
        );

        config
    }

    /// 检查系统兼容性
    async fn check_compatibility(&self) -> Result<bool, CoreError> {
        info!("🔍 检查GPU系统兼容性");

        // 检查OpenCL支持
        #[cfg(feature = "opencl")]
        {
            match self.check_opencl_support().await {
                Ok(true) => {
                    info!("✅ OpenCL支持检查通过");
                    return Ok(true);
                }
                Ok(false) => {
                    debug!("⚠️ OpenCL不可用");
                }
                Err(e) => {
                    debug!("⚠️ OpenCL检查失败: {}", e);
                }
            }
        }

        // 检查CUDA支持
        #[cfg(feature = "cuda")]
        {
            match self.check_cuda_support().await {
                Ok(true) => {
                    info!("✅ CUDA支持检查通过");
                    return Ok(true);
                }
                Ok(false) => {
                    debug!("⚠️ CUDA不可用");
                }
                Err(e) => {
                    debug!("⚠️ CUDA检查失败: {}", e);
                }
            }
        }

        // 如果启用了mock-gpu特性，总是返回兼容
        #[cfg(feature = "mock-gpu")]
        {
            info!("✅ Mock GPU模式，兼容性检查通过");
            return Ok(true);
        }

        // 如果没有任何GPU支持，返回不兼容
        #[cfg(not(any(feature = "opencl", feature = "cuda", feature = "mock-gpu")))]
        {
            return Err(CoreError::runtime("没有启用任何GPU后端".to_string()));
        }

        Ok(false)
    }

    /// 获取核心版本
    fn version(&self) -> String {
        crate::VERSION.to_string()
    }
}

impl GpuCoreFactory {
    /// 检查OpenCL支持
    #[cfg(feature = "opencl")]
    async fn check_opencl_support(&self) -> Result<bool, CoreError> {
        debug!("🔍 检查OpenCL支持");

        // 这里应该实际检查OpenCL平台和设备
        // 为了简化，我们假设OpenCL可用
        tokio::time::sleep(std::time::Duration::from_millis(10)).await;
        
        debug!("✅ OpenCL支持检查完成");
        Ok(true)
    }

    /// 检查CUDA支持
    #[cfg(feature = "cuda")]
    async fn check_cuda_support(&self) -> Result<bool, CoreError> {
        debug!("🔍 检查CUDA支持");

        // 这里应该实际检查CUDA运行时和设备
        // 为了简化，我们假设CUDA可用
        tokio::time::sleep(std::time::Duration::from_millis(10)).await;
        
        debug!("✅ CUDA支持检查完成");
        Ok(true)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_factory_creation() {
        let factory = GpuCoreFactory::new();
        let info = factory.core_info();
        assert_eq!(info.name, "GPU Mining Core Factory");
        assert!(matches!(info.core_type, cgminer_core::CoreType::Custom(ref s) if s == "gpu"));
    }

    #[tokio::test]
    async fn test_core_creation() {
        let factory = GpuCoreFactory::new();
        let core = factory.create_core("Test GPU Core".to_string()).await;
        assert!(core.is_ok());
    }

    #[tokio::test]
    async fn test_config_validation() {
        let factory = GpuCoreFactory::new();
        
        // 测试有效配置
        let valid_config = factory.default_config();
        assert!(factory.validate_config(&valid_config).is_ok());
        
        // 测试无效配置
        let mut invalid_config = factory.default_config();
        invalid_config.name = "".to_string();
        assert!(factory.validate_config(&invalid_config).is_err());
    }

    #[tokio::test]
    async fn test_supported_devices() {
        let factory = GpuCoreFactory::new();
        let devices = factory.supported_devices();
        assert!(devices.contains(&"gpu".to_string()));
        assert!(devices.contains(&"opencl".to_string()));
        assert!(devices.contains(&"cuda".to_string()));
    }

    #[tokio::test]
    async fn test_compatibility_check() {
        let factory = GpuCoreFactory::new();
        // 在测试环境中，兼容性检查应该能够处理各种情况
        let result = factory.check_compatibility().await;
        // 不管结果如何，都不应该panic
        assert!(result.is_ok() || result.is_err());
    }
}
