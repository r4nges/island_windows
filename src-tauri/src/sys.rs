//! Monitor do sistema: CPU, RAM, rede e GPU NVIDIA (se houver),
//! mais os detalhes por componente (processos, núcleos, interfaces).

use nvml_wrapper::enum_wrappers::device::{Clock, TemperatureSensor};
use nvml_wrapper::enums::device::UsedGpuMemory;
use nvml_wrapper::Nvml;
use serde::Serialize;
use serde_json::{json, Value};
use std::collections::HashMap;
use std::net::IpAddr;
use sysinfo::{MemoryRefreshKind, Networks, ProcessRefreshKind, ProcessesToUpdate, System};

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
    interval: f64,
}

#[derive(Default)]
struct Group {
    cpu: f32,
    mem: u64,
    count: u32,
}

fn clean_name(n: &str) -> String {
    let n = n.strip_suffix(".exe").or_else(|| n.strip_suffix(".EXE")).unwrap_or(n);
    n.to_string()
}

impl Monitor {
    pub fn new() -> Self {
        let mut sys = System::new();
        sys.refresh_cpu_usage();
        Self { sys, nets: Networks::new_with_refreshed_list(), nvml: Nvml::init().ok(), interval: 1.0 }
    }

    /// Chamar a cada ~1 s (taxas de rede são por intervalo).
    pub fn sample(&mut self, interval_s: f64) -> Stats {
        self.interval = interval_s.max(0.1);
        self.sys.refresh_cpu_usage();
        self.sys.refresh_memory_specifics(MemoryRefreshKind::nothing().with_ram().with_swap());
        self.nets.refresh(true);

        let (mut rx, mut tx) = (0u64, 0u64);
        for (_name, data) in self.nets.iter() {
            rx += data.received();
            tx += data.transmitted();
        }
        let k = 1.0 / self.interval;

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

    fn refresh_procs(&mut self) {
        self.sys.refresh_processes_specifics(
            ProcessesToUpdate::All,
            true,
            ProcessRefreshKind::nothing().with_cpu().with_memory(),
        );
    }

    /// Processos agrupados pelo nome (Chrome com 30 processos vira uma linha).
    fn groups(&self) -> Vec<(String, Group)> {
        let ncpu = self.sys.cpus().len().max(1) as f32;
        let mut map: HashMap<String, Group> = HashMap::new();
        for p in self.sys.processes().values() {
            let name = clean_name(&p.name().to_string_lossy());
            if name.is_empty() || name == "System Idle Process" || name == "Idle" {
                continue;
            }
            let g = map.entry(name).or_default();
            g.cpu += p.cpu_usage() / ncpu;
            g.mem += p.memory();
            g.count += 1;
        }
        map.into_iter().collect()
    }

    fn top(&self, by_cpu: bool, n: usize) -> Value {
        let mut g = self.groups();
        if by_cpu {
            g.sort_by(|a, b| b.1.cpu.partial_cmp(&a.1.cpu).unwrap_or(std::cmp::Ordering::Equal));
        } else {
            g.sort_by(|a, b| b.1.mem.cmp(&a.1.mem));
        }
        Value::Array(
            g.into_iter()
                .take(n)
                .map(|(name, g)| json!({ "name": name, "cpu": g.cpu, "mem": g.mem, "count": g.count }))
                .collect(),
        )
    }

    /// Detalhe de um componente: "cpu" | "ram" | "gpu" | "net"
    pub fn detail(&mut self, kind: &str) -> Value {
        match kind {
            "cpu" => {
                self.refresh_procs();
                self.sys.refresh_cpu_frequency();
                let cpus = self.sys.cpus();
                let freq = cpus.iter().map(|c| c.frequency()).max().unwrap_or(0);
                json!({
                    "kind": "cpu",
                    "brand": cpus.first().map(|c| c.brand().trim().to_string()).unwrap_or_default(),
                    "usage": self.sys.global_cpu_usage(),
                    "freq_mhz": freq,
                    "physical": System::physical_core_count(),
                    "threads": cpus.len(),
                    "cores": cpus.iter().map(|c| c.cpu_usage()).collect::<Vec<_>>(),
                    "procs": self.top(true, 7),
                })
            }
            "ram" => {
                self.refresh_procs();
                json!({
                    "kind": "ram",
                    "used": self.sys.used_memory(),
                    "total": self.sys.total_memory(),
                    "available": self.sys.available_memory(),
                    "swap_used": self.sys.used_swap(),
                    "swap_total": self.sys.total_swap(),
                    "procs": self.top(false, 7),
                })
            }
            "gpu" => self.gpu_detail(),
            "net" => {
                let k = 1.0 / self.interval;
                let mut ifs: Vec<Value> = self
                    .nets
                    .iter()
                    .filter(|(_, d)| d.total_received() + d.total_transmitted() > 0)
                    .map(|(name, d)| {
                        let ips: Vec<String> = d
                            .ip_networks()
                            .iter()
                            .filter(|n| matches!(n.addr, IpAddr::V4(_)))
                            .map(|n| n.addr.to_string())
                            .collect();
                        json!({
                            "name": name,
                            "down": (d.received() as f64 * k) as u64,
                            "up": (d.transmitted() as f64 * k) as u64,
                            "total_down": d.total_received(),
                            "total_up": d.total_transmitted(),
                            "ips": ips,
                        })
                    })
                    .collect();
                ifs.sort_by_key(|v| std::cmp::Reverse(v["total_down"].as_u64().unwrap_or(0)));
                ifs.truncate(5);
                json!({ "kind": "net", "ifaces": ifs })
            }
            _ => Value::Null,
        }
    }

    fn gpu_detail(&mut self) -> Value {
        let Some(nvml) = self.nvml.as_ref() else {
            return json!({ "kind": "gpu", "available": false });
        };
        let Ok(dev) = nvml.device_by_index(0) else {
            return json!({ "kind": "gpu", "available": false });
        };
        let util = dev.utilization_rates().ok();
        let mem = dev.memory_info().ok();
        let mut pids: Vec<(u32, Option<u64>)> = Vec::new();
        for list in [dev.running_graphics_processes(), dev.running_compute_processes()] {
            for p in list.unwrap_or_default() {
                let used = match p.used_gpu_memory {
                    UsedGpuMemory::Used(b) => Some(b),
                    UsedGpuMemory::Unavailable => None,
                };
                if !pids.iter().any(|(x, _)| *x == p.pid) {
                    pids.push((p.pid, used));
                }
            }
        }
        let name = dev.name().unwrap_or_default();
        let temp = dev.temperature(TemperatureSensor::Gpu).unwrap_or(0);
        let power = dev.power_usage().map(|mw| mw as f64 / 1000.0).ok();
        let clock = dev.clock_info(Clock::Graphics).ok();
        let mclock = dev.clock_info(Clock::Memory).ok();
        let fan = dev.fan_speed(0).ok();

        // nomes dos processos pelo PID
        self.sys.refresh_processes_specifics(ProcessesToUpdate::All, true, ProcessRefreshKind::nothing());
        let mut seen: HashMap<String, (u32, Option<u64>)> = HashMap::new();
        for (pid, used) in pids {
            let pname = self
                .sys
                .process(sysinfo::Pid::from_u32(pid))
                .map(|p| clean_name(&p.name().to_string_lossy()))
                .unwrap_or_else(|| format!("PID {pid}"));
            let e = seen.entry(pname).or_insert((0, None));
            e.0 += 1;
            if let Some(u) = used {
                e.1 = Some(e.1.unwrap_or(0) + u);
            }
        }
        let mut procs: Vec<Value> = seen
            .into_iter()
            .map(|(n, (c, m))| json!({ "name": n, "count": c, "vram": m }))
            .collect();
        procs.sort_by_key(|v| std::cmp::Reverse(v["vram"].as_u64().unwrap_or(0)));
        procs.truncate(7);

        json!({
            "kind": "gpu",
            "available": true,
            "name": name,
            "usage": util.as_ref().map(|u| u.gpu),
            "mem_util": util.as_ref().map(|u| u.memory),
            "temp": temp,
            "power_w": power,
            "clock_mhz": clock,
            "mem_clock_mhz": mclock,
            "fan": fan,
            "vram_used": mem.as_ref().map(|m| m.used),
            "vram_total": mem.as_ref().map(|m| m.total),
            "procs": procs,
        })
    }
}
