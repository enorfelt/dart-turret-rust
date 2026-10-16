use std::ffi::CStr;
use std::thread;

use esp_idf_svc::hal::task::thread::{MallocCap, ThreadSpawnConfiguration};

pub fn spawn_with_psram_stack<F>(name: &'static CStr, stack_size: usize, f: F) -> anyhow::Result<()>
where
    F: FnOnce() + Send + 'static,
{
    ThreadSpawnConfiguration {
        name: Some(name),
        stack_size,
        stack_alloc_caps: MallocCap::Spiram | MallocCap::Cap8bit,
        ..Default::default()
    }
    .set()?;
    let spawned = thread::Builder::new().stack_size(stack_size).spawn(f);
    ThreadSpawnConfiguration::default().set()?;
    spawned?;
    Ok(())
}
