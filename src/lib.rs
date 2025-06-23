//! CGMiner GPU Core - GPU挖矿核心
//!
//! 这个库提供基于GPU的挖矿实现，支持多种GPU平台：
//! - Mac M4 GPU (Metal 计算着色器)
//! - NVIDIA GPU (CUDA) - 预留
//! - AMD GPU (OpenCL) - 预留
//! - Intel GPU (OpenCL) - 预留
//!
//! GPU核心专注于高性能SHA256d算法计算，提供高算力的比特币挖矿能力。

pub mod core;
pub mod device;
pub mod factory;
pub mod gpu_manager;

// 平台特定的 GPU 后端
pub mod opencl_backend;

#[cfg(feature = "mac-metal")]
pub mod metal_backend;

#[cfg(feature = "mac-metal")]
pub mod metal_device;

#[cfg(feature = "cuda")]
pub mod cuda_backend;

#[cfg(feature = "mock-gpu")]
pub mod mock_gpu;

// 重新导出主要类型
pub use core::GpuMiningCore;
pub use device::GpuDevice;
pub use factory::GpuCoreFactory;

#[cfg(feature = "mac-metal")]
pub use metal_device::MetalDevice;

use cgminer_core::{CoreType, CoreInfo};

/// 库版本
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// 获取GPU核心信息
pub fn get_core_info() -> CoreInfo {
    let mut supported_devices = vec!["gpu".to_string()];

    // 根据编译特性添加支持的设备类型
    #[cfg(feature = "mac-metal")]
    supported_devices.push("mac-metal".to_string());

    #[cfg(feature = "opencl")]
    supported_devices.push("opencl".to_string());

    #[cfg(feature = "cuda")]
    supported_devices.push("cuda".to_string());

    CoreInfo::new(
        "GPU Mining Core".to_string(),
        CoreType::Custom("gpu".to_string()),
        VERSION.to_string(),
        "GPU挖矿核心，支持Mac M4 Metal、OpenCL、CUDA等多种GPU平台".to_string(),
        "CGMiner Rust Team".to_string(),
        supported_devices,
    )
}

/// 创建GPU核心工厂
pub fn create_factory() -> Box<dyn cgminer_core::CoreFactory> {
    Box::new(GpuCoreFactory::new())
}

// C FFI 导出函数，用于动态加载
#[no_mangle]
pub extern "C" fn cgminer_gpu_btc_core_info() -> *const std::os::raw::c_char {
    use std::ffi::CString;

    let info = get_core_info();
    let json = serde_json::to_string(&info).unwrap_or_default();
    let c_string = CString::new(json).unwrap_or_default();

    // 注意：这里返回的指针需要调用者负责释放
    c_string.into_raw()
}

#[no_mangle]
pub extern "C" fn cgminer_gpu_btc_create_factory() -> *mut std::os::raw::c_void {
    let factory = create_factory();
    Box::into_raw(Box::new(factory)) as *mut std::os::raw::c_void
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_core_info() {
        let info = get_core_info();
        assert_eq!(info.name, "GPU Mining Core");
        assert!(matches!(info.core_type, CoreType::Custom(ref s) if s == "gpu"));
        assert!(info.supported_devices.contains(&"gpu".to_string()));
    }

    #[test]
    fn test_factory_creation() {
        let factory = create_factory();
        let info = factory.core_info();
        assert_eq!(info.name, "GPU Mining Core");
    }
}
