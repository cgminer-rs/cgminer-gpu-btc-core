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
        let mut supported_devices = vec!["gpu".to_string()];

        // 根据编译特性添加支持的设备类型
        #[cfg(feature = "mac-metal")]
        supported_devices.push("mac-metal".to_string());

        #[cfg(feature = "opencl")]
        supported_devices.push("opencl".to_string());

        #[cfg(feature = "cuda")]
        supported_devices.push("cuda".to_string());

        let info = CoreInfo::new(
            "GPU Mining Core Factory".to_string(),
            cgminer_core::CoreType::Custom("gpu".to_string()),
            crate::VERSION.to_string(),
            "GPU挖矿核心工厂，支持Mac M4 Metal、OpenCL、CUDA等多种GPU平台".to_string(),
            "CGMiner Rust Team".to_string(),
            supported_devices,
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
    /// 获取核心类型
    fn core_type(&self) -> cgminer_core::CoreType {
        cgminer_core::CoreType::Custom("gpu".to_string())
    }

    /// 获取核心信息
    fn core_info(&self) -> CoreInfo {
        self.info.clone()
    }

    /// 创建挖矿核心实例
    async fn create_core(&self, config: cgminer_core::CoreConfig) -> Result<Box<dyn MiningCore>, CoreError> {
        info!("🏭 创建GPU挖矿核心实例: {}", config.name);

        let mut core = GpuMiningCore::new(config.name.clone());
        core.initialize(config).await?;

        debug!("✅ GPU挖矿核心实例创建成功");
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
            serde_json::Value::Number(serde_json::Number::from(1)) // Mac 通常只有一个 GPU
        );
        config.custom_params.insert(
            "work_size".to_string(),
            serde_json::Value::Number(serde_json::Number::from(65536)) // Mac M4 优化的工作组大小
        );

        // Mac Metal 特定配置
        #[cfg(feature = "mac-metal")]
        {
            config.custom_params.insert(
                "backend".to_string(),
                serde_json::Value::String("metal".to_string())
            );
            config.custom_params.insert(
                "threads_per_threadgroup".to_string(),
                serde_json::Value::Number(serde_json::Number::from(1024))
            );
        }

        // OpenCL 配置
        #[cfg(feature = "opencl")]
        {
            config.custom_params.insert(
                "opencl_platform".to_string(),
                serde_json::Value::Number(serde_json::Number::from(0))
            );
        }

        // CUDA 配置
        #[cfg(feature = "cuda")]
        {
            config.custom_params.insert(
                "cuda_device".to_string(),
                serde_json::Value::Number(serde_json::Number::from(0))
            );
        }

        config
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
        let config = factory.default_config();
        let core = factory.create_core(config).await;
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


}
