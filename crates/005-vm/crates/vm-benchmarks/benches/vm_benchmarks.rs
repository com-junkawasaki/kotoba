use criterion::{black_box, criterion_group, criterion_main, Criterion, BatchSize};
use vm_types::{Dag, Task, TaskCharacteristics, ComputationType, Instruction, TaskId, HardwareTile, HardwareTileType, HardwareCharacteristics};
use vm_scheduler::{DataflowRuntime, DataflowRuntimeImpl};
use vm_memory::{MemorySystem, MemorySystemImpl};
use vm_cpu::{VonNeumannCore, VonNeumannCoreImpl};
use std::collections::{HashMap, VecDeque};
use std::hash::{Hash, Hasher};

fn create_large_dag(size: usize) -> Dag {
    let mut tasks = Vec::new();

    // Create tasks with dependencies forming a diamond pattern
    for i in 0..size {
        let deps: Vec<TaskId> = if i < size / 2 {
            vec![]
        } else if i < size * 3 / 4 {
            vec![(i - size / 2) as TaskId]
        } else {
            vec![(i - size / 4) as TaskId, (i - size / 4 + 1) as TaskId]
        };

        let comp_type = match i % 4 {
            0 => ComputationType::GeneralPurpose,
            1 => ComputationType::HighlyParallel,
            2 => ComputationType::MemoryBound,
            _ => ComputationType::Reconfigurable,
        };

        let task = Task {
            id: i as u64,
            operation: vec![Instruction::Halt],
            dependencies: deps,
            estimated_execution_time: (i % 10 + 1) as u64,
            characteristics: TaskCharacteristics {
                computation_type: comp_type,
                data_size: (i * 1024) as usize,
                parallelism_factor: ((i % 8) + 1) as u32,
                memory_intensity: (i % 100) as f32 / 100.0,
            },
        };
        tasks.push(task);
    }

    Dag { tasks }
}

fn bench_dag_scheduling(c: &mut Criterion) {
    let mut group = c.benchmark_group("dag_scheduling");

    for size in [10, 50, 100].iter() {
        let dag = create_large_dag(*size);
        let runtime = DataflowRuntimeImpl::new();

        group.bench_with_input(format!("size_{}", size), &dag, |b, dag| {
            b.iter(|| {
                let _result = runtime.schedule_dag(black_box(dag));
            });
        });
    }
    group.finish();
}

fn bench_critical_path_calculation(c: &mut Criterion) {
    let mut group = c.benchmark_group("critical_path");

    for size in [10, 50, 100].iter() {
        let dag = create_large_dag(*size);
        let runtime = DataflowRuntimeImpl::new();

        group.bench_with_input(format!("size_{}", size), &dag, |b, dag| {
            b.iter(|| {
                let _result = runtime.schedule_with_critical_path(black_box(dag));
            });
        });
    }
    group.finish();
}

fn bench_memoization(c: &mut Criterion) {
    let mut group = c.benchmark_group("memoization");

    let dag = create_large_dag(100);
    let mut runtime = DataflowRuntimeImpl::new();

    // Pre-populate cache with some results
    for task in &dag.tasks[0..10] {
        runtime.cache_task_result(task, vec![42; 8]);
    }

    group.bench_function("schedule_with_memoization", |b| {
        b.iter(|| {
            let _result = runtime.schedule_dag(black_box(&dag));
        });
    });

    group.finish();
}

fn bench_memory_operations(c: &mut Criterion) {
    let mut group = c.benchmark_group("memory_operations");

    let mut memory = MemorySystemImpl::new(1024 * 1024); // 1MB

    group.bench_function("sequential_writes", |b| {
        b.iter(|| {
            for addr in 0..1000 {
                memory.write(addr, (addr % 256) as u8);
            }
        });
    });

    group.bench_function("sequential_reads", |b| {
        b.iter(|| {
            for addr in 0..1000 {
                black_box(memory.read(addr));
            }
        });
    });

    group.bench_function("random_access", |b| {
        b.iter(|| {
            for _ in 0..1000 {
                let addr = (rand::random::<u64>() % 1000000) as u64;
                memory.write(addr, (addr % 256) as u8);
                black_box(memory.read(addr));
            }
        });
    });

    group.finish();
}

fn bench_von_neumann_execution(c: &mut Criterion) {
    let mut group = c.benchmark_group("von_neumann_core");

    let mut core = VonNeumannCoreImpl::new();
    let mut memory = MemorySystemImpl::new(1024);

    group.bench_function("program_execution", |b| {
        b.iter(|| {
            core.run(black_box(&mut memory));
        });
    });

    group.finish();
}

fn bench_hardware_dispatch(c: &mut Criterion) {
    let mut group = c.benchmark_group("hardware_dispatch");

    let runtime = DataflowRuntimeImpl::new();
    let task = Task {
        id: 1,
        operation: vec![Instruction::Halt],
        dependencies: vec![],
        estimated_execution_time: 100,
        characteristics: TaskCharacteristics {
            computation_type: ComputationType::HighlyParallel,
            data_size: 1024 * 1024,
            parallelism_factor: 8,
            memory_intensity: 0.8,
        },
    };

    // Create available hardware tiles for dispatch testing
    let tiles = vec![
        HardwareTile {
            id: 0,
            characteristics: HardwareCharacteristics {
                tile_type: HardwareTileType::CPU,
                compute_units: 8,
                memory_bandwidth: 50_000_000_000,
                power_efficiency: 0.8,
                current_load: 0.2,
            },
            is_available: true,
        },
        HardwareTile {
            id: 1,
            characteristics: HardwareCharacteristics {
                tile_type: HardwareTileType::GPU,
                compute_units: 1024,
                memory_bandwidth: 1_000_000_000_000,
                power_efficiency: 0.6,
                current_load: 0.3,
            },
            is_available: true,
        },
    ];

    group.bench_function("task_hardware_dispatch", |b| {
        b.iter(|| {
            let _tile = runtime.dispatch_to_hardware(
                black_box(&task),
                black_box(&tiles)
            );
        });
    });

    group.finish();
}

// --- Comparison Benchmarks ---

/// Simple topological sort implementation for comparison
fn simple_topological_sort(dag: &Dag) -> Vec<TaskId> {
    let mut result = Vec::new();
    let mut in_degree = HashMap::new();
    let mut queue = VecDeque::new();

    // Initialize in-degrees
    for task in &dag.tasks {
        in_degree.insert(task.id, task.dependencies.len());
        if task.dependencies.is_empty() {
            queue.push_back(task.id);
        }
    }

    // Process queue
    while let Some(task_id) = queue.pop_front() {
        result.push(task_id);

        // Find tasks that depend on this task
        for task in &dag.tasks {
            if task.dependencies.contains(&task_id) {
                let degree = in_degree.get_mut(&task.id).unwrap();
                *degree -= 1;
                if *degree == 0 {
                    queue.push_back(task.id);
                }
            }
        }
    }

    result
}

/// Round-robin hardware dispatch for comparison
fn round_robin_dispatch<'a>(_task: &Task, tiles: &'a [HardwareTile]) -> Option<&'a HardwareTile> {
    static mut COUNTER: usize = 0;
    unsafe {
        let tile = &tiles[COUNTER % tiles.len()];
        COUNTER += 1;
        Some(tile)
    }
}

/// Random hardware dispatch for comparison
fn random_dispatch<'a>(_task: &Task, tiles: &'a [HardwareTile]) -> Option<&'a HardwareTile> {
    use rand::Rng;
    let mut rng = rand::thread_rng();
    let index = rng.gen_range(0..tiles.len());
    Some(&tiles[index])
}

/// Simple HashMap-based cache for comparison
struct SimpleCache {
    cache: HashMap<u64, Vec<u8>>,
}

impl SimpleCache {
    fn new() -> Self {
        Self {
            cache: HashMap::new(),
        }
    }

    fn get(&self, key: u64) -> Option<&Vec<u8>> {
        self.cache.get(&key)
    }

    fn insert(&mut self, key: u64, value: Vec<u8>) {
        self.cache.insert(key, value);
    }
}

fn bench_dag_scheduler_comparison(c: &mut Criterion) {
    let mut group = c.benchmark_group("dag_scheduler_comparison");

    for size in [10, 50, 100].iter() {
        let dag = create_large_dag(*size);

        group.bench_with_input(format!("vm_scheduler_size_{}", size), &dag, |b, dag| {
            let runtime = DataflowRuntimeImpl::new();
            b.iter(|| {
                let _result = runtime.schedule_dag(black_box(dag));
            });
        });

        group.bench_with_input(format!("simple_topo_sort_size_{}", size), &dag, |b, dag| {
            b.iter(|| {
                let _result = simple_topological_sort(black_box(dag));
            });
        });
    }

    group.finish();
}

fn bench_hardware_dispatch_comparison(c: &mut Criterion) {
    let mut group = c.benchmark_group("hardware_dispatch_comparison");

    let runtime = DataflowRuntimeImpl::new();
    let task = Task {
        id: 1,
        operation: vec![Instruction::Halt],
        dependencies: vec![],
        estimated_execution_time: 100,
        characteristics: TaskCharacteristics {
            computation_type: ComputationType::HighlyParallel,
            data_size: 1024 * 1024,
            parallelism_factor: 8,
            memory_intensity: 0.8,
        },
    };

    let tiles = vec![
        HardwareTile {
            id: 0,
            characteristics: HardwareCharacteristics {
                tile_type: HardwareTileType::CPU,
                compute_units: 8,
                memory_bandwidth: 50_000_000_000,
                power_efficiency: 0.8,
                current_load: 0.2,
            },
            is_available: true,
        },
        HardwareTile {
            id: 1,
            characteristics: HardwareCharacteristics {
                tile_type: HardwareTileType::GPU,
                compute_units: 1024,
                memory_bandwidth: 1_000_000_000_000,
                power_efficiency: 0.6,
                current_load: 0.3,
            },
            is_available: true,
        },
        HardwareTile {
            id: 2,
            characteristics: HardwareCharacteristics {
                tile_type: HardwareTileType::CgraFpga,
                compute_units: 64,
                memory_bandwidth: 100_000_000_000,
                power_efficiency: 0.9,
                current_load: 0.1,
            },
            is_available: true,
        },
    ];

    group.bench_function("vm_intelligent_dispatch", |b| {
        b.iter(|| {
            let _tile = runtime.dispatch_to_hardware(
                black_box(&task),
                black_box(&tiles)
            );
        });
    });

    group.bench_function("round_robin_dispatch", |b| {
        b.iter(|| {
            let _tile = round_robin_dispatch(black_box(&task), black_box(&tiles));
        });
    });

    group.bench_function("random_dispatch", |b| {
        b.iter(|| {
            let _tile = random_dispatch(black_box(&task), black_box(&tiles));
        });
    });

    group.finish();
}

fn bench_memoization_comparison(c: &mut Criterion) {
    let mut group = c.benchmark_group("memoization_comparison");

    let dag = create_large_dag(50);
    let mut vm_runtime = DataflowRuntimeImpl::new();
    let mut simple_cache = SimpleCache::new();

    // Pre-populate caches
    for task in &dag.tasks[0..10] {
        let result_data = vec![42; 64];
        vm_runtime.cache_task_result(task, result_data.clone());

        // Use task ID as hash for simple cache
        let mut hasher = std::collections::hash_map::DefaultHasher::new();
        task.id.hash(&mut hasher);
        let hash = hasher.finish();
        simple_cache.insert(hash, result_data);
    }

    group.bench_function("vm_memoization_xxhash", |b| {
        b.iter(|| {
            let _result = vm_runtime.schedule_dag(black_box(&dag));
        });
    });

    // For simple cache, we'll just do lookups
    group.bench_function("hashmap_cache_lookups", |b| {
        b.iter(|| {
            for task in &dag.tasks[0..10] {
                let mut hasher = std::collections::hash_map::DefaultHasher::new();
                task.id.hash(&mut hasher);
                let hash = hasher.finish();
                let _result = simple_cache.get(hash);
            }
        });
    });

    group.finish();
}

fn bench_memory_comparison(c: &mut Criterion) {
    let mut group = c.benchmark_group("memory_comparison");

    let mut vm_memory = MemorySystemImpl::new(1024 * 1024);
    let mut std_vec = vec![0u8; 1024 * 1024];
    let mut std_hashmap = HashMap::new();

    group.bench_function("vm_memory_sequential_write", |b| {
        b.iter(|| {
            for addr in 0..10000 {
                vm_memory.write(addr, (addr % 256) as u8);
            }
        });
    });

    group.bench_function("std_vec_sequential_write", |b| {
        b.iter(|| {
            for addr in 0..10000 {
                std_vec[addr as usize] = (addr % 256) as u8;
            }
        });
    });

    group.bench_function("vm_memory_random_access", |b| {
        b.iter(|| {
            for _ in 0..5000 {
                let addr = (rand::random::<u64>() % 100000) as u64;
                vm_memory.write(addr, (addr % 256) as u8);
                black_box(vm_memory.read(addr));
            }
        });
    });

    group.bench_function("std_hashmap_random_access", |b| {
        b.iter(|| {
            for _ in 0..5000 {
                let key = rand::random::<u64>() % 100000;
                std_hashmap.insert(key, (key % 256) as u8);
                let _val = std_hashmap.get(&key);
            }
        });
    });

    group.finish();
}

fn bench_memory_slice_comparison(c: &mut Criterion) {
    let mut group = c.benchmark_group("memory_slice_comparison");
    const SLICE_SIZE: usize = 1024;
    let mut vm_memory = MemorySystemImpl::new(1024 * 1024);
    let data_to_write = vec![42; SLICE_SIZE];

    group.bench_function("byte_by_byte_write", |b| {
        b.iter(|| {
            for i in 0..SLICE_SIZE {
                vm_memory.write(i as u64, data_to_write[i]);
            }
        })
    });

    group.bench_function("slice_write", |b| {
        b.iter(|| {
            vm_memory.write_slice(0, &data_to_write);
        })
    });

    group.bench_function("byte_by_byte_read", |b| {
        b.iter(|| {
            let mut read_data = Vec::with_capacity(SLICE_SIZE);
            for i in 0..SLICE_SIZE {
                read_data.push(vm_memory.read(i as u64));
            }
            black_box(read_data);
        })
    });

    group.bench_function("slice_read", |b| {
        b.iter(|| {
            let read_slice = vm_memory.read_slice(0, SLICE_SIZE).unwrap();
            black_box(read_slice);
        })
    });

    group.finish();
}

fn bench_memory_allocation_comparison(c: &mut Criterion) {
    let mut group = c.benchmark_group("memory_allocation_comparison");

    group.bench_function("vm_memory_allocate", |b| {
        b.iter_batched_ref(
            || vm_memory::MemorySystemImpl::new(1024 * 1024),
            |mem| {
                let _handle = mem.allocate(64);
            },
            BatchSize::SmallInput
        );
    });

    group.bench_function("std_vec_allocate", |b| {
        b.iter_batched_ref(
            || Vec::with_capacity(1024 * 1024),
            |vec| {
                // Simulate allocation by extending vec
                vec.extend_from_slice(&[0u8; 64]);
                vec.truncate(vec.len() - 64); // Deallocate
            },
            BatchSize::SmallInput
        );
    });

    group.finish();
}

criterion_group!(
    benches,
    bench_dag_scheduling,
    bench_critical_path_calculation,
    bench_memoization,
    bench_memory_operations,
    bench_von_neumann_execution,
    bench_hardware_dispatch,
    // Comparison benchmarks
    bench_dag_scheduler_comparison,
    bench_hardware_dispatch_comparison,
    bench_memoization_comparison,
    bench_memory_comparison,
    bench_memory_slice_comparison,
    bench_memory_allocation_comparison
);
criterion_main!(benches);
