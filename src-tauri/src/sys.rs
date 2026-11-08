//! Monitor do sistema: CPU, RAM, rede e GPU NVIDIA (se houver).

use nvml_wrapper::{enum_wrappers::device::TemperatureSensor, Nvml};
use serde::Serialize;
use sysinfo::{MemoryRefreshKind, Networks, System};

#[derive(Serialize, Clone, Debug, Default)]
pub struct Gpu {
    pub usage: u32,
    pub temp: u32,
    pub vram_used: u64,
    pub vram_total: u64,
}

#[derive(Serialize, Clone, Debug, Default)]
pub struct Stats {
    pub cpu: f32,
    pub ram_used: u64,
    pub ram_total: u64,
    /// bytes por segundo
    pub down: u64,
    pub up: u64,
    pub gpu: Option<Gpu>,
}

pub struct Monitor {
    sys: System,
    nets: Networks,
    nvml: Option<Nvml>,
}

impl Monitor {
    pub fn new() -> Self {
        let mut sys = System::new();
        sys.refresh_cpu_usage();
        Self { sys, nets: Networks::new_with_refreshed_list(), nvml: Nvml::init().ok() }
    }

    /// Chamar a cada ~1 s (taxas de rede são por intervalo).
    pub fn sample(&mut self, interval_s: f64) -> Stats {
        self.sys.refresh_cpu_usage();
        self.sys.refresh_memory_specifics(MemoryRefreshKind::nothing().with_ram());
        self.nets.refresh(true);

        let (mut rx, mut tx) = (0u64, 0u64);
        for (_name, data) in self.nets.iter() {
            rx += data.received();
            tx += data.transmitted();
        }
        let k = 1.0 / interval_s.max(0.1);

        Stats {
            cpu: self.sys.global_cpu_usage(),
            ram_used: self.sys.used_memory(),
            ram_total: self.sys.total_memory(),
            down: (rx as f64 * k) as u64,
            up: (tx as f64 * k) as u64,
            gpu: self.gpu(),
        }
    }

    fn gpu(&self) -> Option<Gpu> {
        let dev = self.nvml.as_ref()?.device_by_index(0).ok()?;
        let util = dev.utilization_rates().ok()?;
        let mem = dev.memory_info().ok()?;
        Some(Gpu {
            usage: util.gpu,
            temp: dev.temperature(TemperatureSensor::Gpu).unwrap_or(0),
            vram_used: mem.used,
            vram_total: mem.total,
        })
    }
}
