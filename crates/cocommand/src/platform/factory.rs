use std::sync::Arc;

use super::adapters::SharedPlatform;

pub fn default_platform() -> SharedPlatform {
    #[cfg(target_os = "macos")]
    {
        Arc::new(super::adapters::macos::MacosPlatform::new())
    }

    #[cfg(not(target_os = "macos"))]
    {
        Arc::new(super::adapters::portable::PortablePlatform::new())
    }
}
