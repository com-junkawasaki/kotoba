//! # Digital Computing System VM CLI
//!
//! Command-line interface for the Digital Computing System VM.
//! Provides tools for running simulations, benchmarks, and analysis.

use clap::{Parser, Subcommand};
use vm_core::Vm;
use vm_types::{Dag, Task, TaskId, Instruction, TaskCharacteristics, ComputationType};
use vm_memory::{MemorySystem, MemorySystemImpl};
use vm_cpu::{VonNeumannCore, VonNeumannCoreImpl};
use vm_scheduler::{DataflowRuntime, DataflowRuntimeImpl};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::fs;
use std::time::Instant;
use anyhow::{Result, Context};

/// DAG file format for task graph definitions
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DagFile {
    pub tasks: Vec<TaskFile>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskFile {
    pub id: TaskId,
    pub name: Option<String>,
    #[serde(default)]
    pub operation: Vec<String>,
    #[serde(default)]
    pub dependencies: Vec<TaskId>,
    pub estimated_execution_time: u64,
    pub characteristics: TaskCharacteristicsFile,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskCharacteristicsFile {
    pub computation_type: String,
    pub data_size: usize,
    pub parallelism_factor: u32,
    pub memory_intensity: f32,
}

/// Digital Computing System VM - Command Line Interface
#[derive(Parser)]
#[command(name = "vm-cli")]
#[command(about = "Digital Computing System VM CLI")]
#[command(version = "0.1.0")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Run the VM with optional DAG file
    Run {
        /// Enable verbose output
        #[arg(short, long)]
        verbose: bool,
        /// Path to DAG file (JSON format)
        #[arg(short, long)]
        dag_file: Option<PathBuf>,
        /// Output format (table, json, csv)
        #[arg(short, long, default_value = "table")]
        output: String,
    },
    /// Run performance benchmarks
    Bench {
        /// Benchmark type (memory, cpu, scheduler, hardware)
        #[arg(short, long, default_value = "all")]
        bench_type: String,
    },
    /// Analyze VM performance
    Analyze {
        /// Analysis type (throughput, latency, utilization)
        #[arg(short, long, default_value = "throughput")]
        analysis_type: String,
    },
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Run { verbose, dag_file, output: _ } => {
            let mut vm = Vm::new();

            if let Some(dag_path) = dag_file {
                if verbose {
                    println!("📄 Loading DAG from file: {}", dag_path.display());
                }
                let custom_dag = load_dag_from_file(&dag_path)?;
                vm.run_with_dag(custom_dag, verbose)
                    .map_err(|e| anyhow::anyhow!("VM execution failed: {}", e))?;
            } else {
                vm.run();
            }

            if verbose {
                println!("✅ Execution completed successfully");
            }
        }
        Commands::Bench { bench_type } => {
            println!("🏃 Running benchmarks: {}", bench_type);

            match bench_type.as_str() {
                "memory" => run_memory_benchmarks().await,
                "cpu" => run_cpu_benchmarks().await,
                "scheduler" => run_scheduler_benchmarks().await,
                "hardware" => run_hardware_benchmarks().await,
                "all" => {
                    run_memory_benchmarks().await;
                    run_cpu_benchmarks().await;
                    run_scheduler_benchmarks().await;
                    run_hardware_benchmarks().await;
                }
                _ => {
                    eprintln!("❌ Unknown benchmark type: {}", bench_type);
                    std::process::exit(1);
                }
            }
        }
        Commands::Analyze { analysis_type } => {
            run_analysis(analysis_type).await?;
        }
    }
    Ok(())
}

/// Load DAG from JSON file
fn load_dag_from_file(path: &PathBuf) -> Result<Dag> {
    let content = fs::read_to_string(path)
        .with_context(|| format!("Failed to read DAG file: {}", path.display()))?;

    let dag_file: DagFile = serde_json::from_str(&content)
        .with_context(|| format!("Failed to parse DAG file as JSON: {}", path.display()))?;

    let tasks = dag_file.tasks.into_iter()
        .map(|task_file| {
            // Convert string instructions to Instructions enum
            let operation = task_file.operation.into_iter()
                .map(|instr| match instr.as_str() {
                    "Load" => Instruction::Load { dest_reg: 0, addr: 0 }, // Simplified
                    "Store" => Instruction::Store { src_reg: 0, addr: 0 }, // Simplified
                    "Add" => Instruction::Add { dest_reg: 0, src1_reg: 0, src2_reg: 1 }, // Simplified
                    "Sub" => Instruction::Sub { dest_reg: 0, src1_reg: 0, src2_reg: 1 }, // Simplified
                    "Jz" => Instruction::Jz { reg: 0, new_ip: 0 }, // Simplified
                    "Halt" => Instruction::Halt,
                    _ => Instruction::Halt, // Default to Halt for unknown instructions
                })
                .collect();

            // Convert computation type string to enum
            let computation_type = match task_file.characteristics.computation_type.as_str() {
                "GeneralPurpose" => ComputationType::GeneralPurpose,
                "HighlyParallel" => ComputationType::HighlyParallel,
                "Reconfigurable" => ComputationType::Reconfigurable,
                "MemoryBound" => ComputationType::MemoryBound,
                _ => ComputationType::GeneralPurpose, // Default
            };

            Task {
                id: task_file.id,
                operation,
                dependencies: task_file.dependencies,
                estimated_execution_time: task_file.estimated_execution_time,
                characteristics: TaskCharacteristics {
                    computation_type,
                    data_size: task_file.characteristics.data_size,
                    parallelism_factor: task_file.characteristics.parallelism_factor,
                    memory_intensity: task_file.characteristics.memory_intensity,
                },
            }
        })
        .collect();

    Ok(Dag { tasks })
}

async fn run_memory_benchmarks() {
    println!("🧠 Memory System Benchmarks");
    println!("  Running performance tests...");

    // Create memory system
    let mut memory = MemorySystemImpl::new(1024 * 1024); // 1MB

    // Benchmark sequential writes
    let start = Instant::now();
    for i in 0..10000 {
        memory.write(i % 1024, (i % 256) as u8);
    }
    let sequential_write_time = start.elapsed();

    // Benchmark sequential reads
    let start = Instant::now();
    let mut sum = 0u64;
    for i in 0..10000 {
        sum += memory.read(i % 1024) as u64;
    }
    let sequential_read_time = start.elapsed();

    // Benchmark random access
    let start = Instant::now();
    let mut rng = 12345u64;
    for _ in 0..10000 {
        rng = rng.wrapping_mul(1103515245).wrapping_add(12345);
        let addr = (rng % 1024) as u64;
        memory.write(addr, (rng % 256) as u8);
    }
    let random_access_time = start.elapsed();

    println!("  📊 Results:");
    println!("    Sequential writes (10K ops): {:.2} μs/op", sequential_write_time.as_micros() as f64 / 10000.0);
    println!("    Sequential reads (10K ops):  {:.2} μs/op", sequential_read_time.as_micros() as f64 / 10000.0);
    println!("    Random access (10K ops):     {:.2} μs/op", random_access_time.as_micros() as f64 / 10000.0);
    println!("    (Sum of reads to prevent optimization: {})", sum);
}

async fn run_cpu_benchmarks() {
    println!("⚡ CPU Core Benchmarks");
    println!("  Running performance tests...");

    // Create CPU core and memory
    let mut memory = MemorySystemImpl::new(1024);

    // Test simple program execution
    let start = Instant::now();
    for _ in 0..1000 {
        let mut cpu = VonNeumannCoreImpl::new(); // Reset CPU
        cpu.run(&mut memory);
    }
    let program_execution_time = start.elapsed();

    println!("  📊 Results:");
    println!("    Program execution (1K runs): {:.2} μs/run", program_execution_time.as_micros() as f64 / 1000.0);
}

async fn run_scheduler_benchmarks() {
    println!("🎯 Scheduler Benchmarks");
    println!("  Running performance tests...");

    let scheduler = DataflowRuntimeImpl::new();

    // Create test DAGs of different sizes
    let small_dag = create_benchmark_dag(10);
    let medium_dag = create_benchmark_dag(50);
    let large_dag = create_benchmark_dag(100);

    // Benchmark topological sorting
    let start = Instant::now();
    for _ in 0..100 {
        let _ = scheduler.schedule_dag(&small_dag);
    }
    let small_sort_time = start.elapsed();

    let start = Instant::now();
    for _ in 0..10 {
        let _ = scheduler.schedule_dag(&medium_dag);
    }
    let medium_sort_time = start.elapsed();

    let start = Instant::now();
    for _ in 0..5 {
        let _ = scheduler.schedule_dag(&large_dag);
    }
    let large_sort_time = start.elapsed();

    // Benchmark critical path analysis
    let start = Instant::now();
    for _ in 0..100 {
        let _ = scheduler.schedule_with_critical_path(&small_dag);
    }
    let small_critical_time = start.elapsed();

    println!("  📊 Results:");
    println!("    DAG sorting (10 tasks):  {:.2} μs/op", small_sort_time.as_micros() as f64 / 100.0);
    println!("    DAG sorting (50 tasks):  {:.2} μs/op", medium_sort_time.as_micros() as f64 / 10.0);
    println!("    DAG sorting (100 tasks): {:.2} μs/op", large_sort_time.as_micros() as f64 / 5.0);
    println!("    Critical path (10 tasks): {:.2} μs/op", small_critical_time.as_micros() as f64 / 100.0);
}

/// Create a benchmark DAG with specified number of tasks
fn create_benchmark_dag(size: usize) -> Dag {
    let mut tasks = Vec::new();

    for i in 0..size {
        let deps = if i > 0 {
            vec![(i - 1) as TaskId]
        } else {
            vec![]
        };

        let task = Task {
            id: i as TaskId,
            operation: vec![Instruction::Halt],
            dependencies: deps,
            estimated_execution_time: (i as u64 * 10) + 50,
            characteristics: TaskCharacteristics {
                computation_type: ComputationType::GeneralPurpose,
                data_size: 1024,
                parallelism_factor: 1,
                memory_intensity: 0.5,
            },
        };
        tasks.push(task);
    }

    Dag { tasks }
}

async fn run_hardware_benchmarks() {
    println!("🔧 Hardware Dispatch Benchmarks");
    println!("  Running performance tests...");

    let scheduler = DataflowRuntimeImpl::new();

    // Create hardware tiles
    let tiles = create_hardware_tiles();

    // Create test tasks with different characteristics
    let tasks = create_test_tasks();

    // Benchmark hardware dispatch
    let start = Instant::now();
    for _ in 0..1000 {
        for task in &tasks {
            let _ = scheduler.dispatch_to_hardware(task, &tiles);
        }
    }
    let dispatch_time = start.elapsed();

    println!("  📊 Results:");
    println!("    Hardware dispatch (4K ops): {:.2} μs/op", dispatch_time.as_micros() as f64 / 4000.0);
}

/// Create test hardware tiles for benchmarking
fn create_hardware_tiles() -> Vec<vm_types::HardwareTile> {
    vec![
        vm_types::HardwareTile {
            id: 0,
            characteristics: vm_types::HardwareCharacteristics {
                tile_type: vm_types::HardwareTileType::CPU,
                compute_units: 8,
                memory_bandwidth: 50_000,
                power_efficiency: 0.8,
                current_load: 0.0,
            },
            is_available: true,
        },
        vm_types::HardwareTile {
            id: 1,
            characteristics: vm_types::HardwareCharacteristics {
                tile_type: vm_types::HardwareTileType::GPU,
                compute_units: 1024,
                memory_bandwidth: 1_000_000,
                power_efficiency: 0.6,
                current_load: 0.0,
            },
            is_available: true,
        },
        vm_types::HardwareTile {
            id: 2,
            characteristics: vm_types::HardwareCharacteristics {
                tile_type: vm_types::HardwareTileType::CgraFpga,
                compute_units: 64,
                memory_bandwidth: 100_000,
                power_efficiency: 0.9,
                current_load: 0.0,
            },
            is_available: true,
        },
        vm_types::HardwareTile {
            id: 3,
            characteristics: vm_types::HardwareCharacteristics {
                tile_type: vm_types::HardwareTileType::PIM,
                compute_units: 16,
                memory_bandwidth: 10_000_000,
                power_efficiency: 0.95,
                current_load: 0.0,
            },
            is_available: true,
        },
    ]
}

/// Create test tasks with different characteristics
fn create_test_tasks() -> Vec<Task> {
    vec![
        Task {
            id: 0,
            operation: vec![Instruction::Halt],
            dependencies: vec![],
            estimated_execution_time: 100,
            characteristics: TaskCharacteristics {
                computation_type: ComputationType::GeneralPurpose,
                data_size: 1024,
                parallelism_factor: 1,
                memory_intensity: 0.5,
            },
        },
        Task {
            id: 1,
            operation: vec![Instruction::Halt],
            dependencies: vec![],
            estimated_execution_time: 200,
            characteristics: TaskCharacteristics {
                computation_type: ComputationType::HighlyParallel,
                data_size: 65536,
                parallelism_factor: 64,
                memory_intensity: 0.3,
            },
        },
        Task {
            id: 2,
            operation: vec![Instruction::Halt],
            dependencies: vec![],
            estimated_execution_time: 50,
            characteristics: TaskCharacteristics {
                computation_type: ComputationType::MemoryBound,
                data_size: 1024,
                parallelism_factor: 1,
                memory_intensity: 0.9,
            },
        },
        Task {
            id: 3,
            operation: vec![Instruction::Halt],
            dependencies: vec![],
            estimated_execution_time: 150,
            characteristics: TaskCharacteristics {
                computation_type: ComputationType::Reconfigurable,
                data_size: 4096,
                parallelism_factor: 8,
                memory_intensity: 0.4,
            },
        },
    ]
}

async fn run_analysis(analysis_type: String) -> Result<()> {
    println!("📊 Running {} analysis...", analysis_type);

    // Create a sample DAG for analysis
    let test_dag = create_analysis_dag();
    let scheduler = DataflowRuntimeImpl::new();

    match analysis_type.as_str() {
        "throughput" => {
            println!("📈 Throughput Analysis");
            run_throughput_analysis(&scheduler, &test_dag).await?;
        }
        "latency" => {
            println!("⏱️  Latency Analysis");
            run_latency_analysis(&scheduler, &test_dag).await?;
        }
        "utilization" => {
            println!("📊 Utilization Analysis");
            run_utilization_analysis(&scheduler, &test_dag).await?;
        }
        _ => {
            eprintln!("❌ Unknown analysis type: {}", analysis_type);
            std::process::exit(1);
        }
    }

    Ok(())
}

async fn run_throughput_analysis(scheduler: &DataflowRuntimeImpl, dag: &Dag) -> Result<()> {
    println!("  Measuring task scheduling throughput...");

    let start = Instant::now();
    let iterations = 1000;

    for _ in 0..iterations {
        let _ = scheduler.schedule_dag(dag);
    }

    let total_time = start.elapsed();
    let tasks_per_second = (iterations * dag.tasks.len()) as f64 / total_time.as_secs_f64();

    println!("  📊 Results:");
    println!("    Tasks scheduled per second: {:.0}", tasks_per_second);
    println!("    Average latency per task: {:.2} μs", (total_time.as_micros() as f64) / (iterations * dag.tasks.len()) as f64);

    Ok(())
}

async fn run_latency_analysis(scheduler: &DataflowRuntimeImpl, dag: &Dag) -> Result<()> {
    println!("  Measuring scheduling and dispatch latency...");

    // Measure topological sorting latency
    let start = Instant::now();
    for _ in 0..1000 {
        let _ = scheduler.schedule_dag(dag);
    }
    let sort_time = start.elapsed();

    // Measure critical path analysis latency
    let start = Instant::now();
    for _ in 0..1000 {
        let _ = scheduler.schedule_with_critical_path(dag);
    }
    let critical_time = start.elapsed();

    // Create hardware tiles for dispatch testing
    let tiles = create_hardware_tiles();

    // Measure hardware dispatch latency
    let start = Instant::now();
    for _ in 0..10000 {
        for task in &dag.tasks {
            let _ = scheduler.dispatch_to_hardware(task, &tiles);
        }
    }
    let dispatch_time = start.elapsed();

    println!("  📊 Results:");
    println!("    Topological sort latency: {:.2} μs/op", sort_time.as_micros() as f64 / 1000.0);
    println!("    Critical path latency: {:.2} μs/op", critical_time.as_micros() as f64 / 1000.0);
    println!("    Hardware dispatch latency: {:.2} μs/op", dispatch_time.as_micros() as f64 / (10000.0 * dag.tasks.len() as f64));

    Ok(())
}

async fn run_utilization_analysis(scheduler: &DataflowRuntimeImpl, dag: &Dag) -> Result<()> {
    println!("  Analyzing hardware utilization patterns...");

    let tiles = create_hardware_tiles();

    // Simulate multiple task dispatches to analyze utilization
    let mut tile_usage = vec![0; tiles.len()];

    for _ in 0..1000 {
        for (_i, task) in dag.tasks.iter().enumerate() {
            if let Some(selected_tile) = scheduler.dispatch_to_hardware(task, &tiles) {
                if let Some(tile_idx) = tiles.iter().position(|t| t.id == selected_tile.id) {
                    tile_usage[tile_idx] += 1;
                }
            }
        }
    }

    println!("  📊 Hardware Utilization:");
    for (i, &usage) in tile_usage.iter().enumerate() {
        let utilization = usage as f64 / 1000.0;
        let tile_name = match tiles[i].characteristics.tile_type {
            vm_types::HardwareTileType::CPU => "CPU",
            vm_types::HardwareTileType::GPU => "GPU",
            vm_types::HardwareTileType::CgraFpga => "CGRA/FPGA",
            vm_types::HardwareTileType::PIM => "PIM",
        };
        println!("    {} Tile: {:.1}% utilization", tile_name, utilization * 100.0);
    }

    Ok(())
}

fn create_analysis_dag() -> Dag {
    Dag {
        tasks: vec![
            Task {
                id: 0,
                operation: vec![Instruction::Halt],
                dependencies: vec![],
                estimated_execution_time: 100,
                characteristics: TaskCharacteristics {
                    computation_type: ComputationType::GeneralPurpose,
                    data_size: 1024,
                    parallelism_factor: 1,
                    memory_intensity: 0.5,
                },
            },
            Task {
                id: 1,
                operation: vec![Instruction::Halt],
                dependencies: vec![0],
                estimated_execution_time: 200,
                characteristics: TaskCharacteristics {
                    computation_type: ComputationType::HighlyParallel,
                    data_size: 65536,
                    parallelism_factor: 64,
                    memory_intensity: 0.3,
                },
            },
            Task {
                id: 2,
                operation: vec![Instruction::Halt],
                dependencies: vec![0],
                estimated_execution_time: 50,
                characteristics: TaskCharacteristics {
                    computation_type: ComputationType::MemoryBound,
                    data_size: 1024,
                    parallelism_factor: 1,
                    memory_intensity: 0.9,
                },
            },
        ]
    }
}
