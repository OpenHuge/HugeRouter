use tracing::info;

#[derive(Debug, Clone, Copy)]
pub struct ServiceRuntime {
    pub service_name: &'static str,
    pub role: &'static str,
}

pub fn announce_startup(runtime: ServiceRuntime) {
    info!(
        service = runtime.service_name,
        role = runtime.role,
        "HugeRouter service bootstrap placeholder is online"
    );
}

