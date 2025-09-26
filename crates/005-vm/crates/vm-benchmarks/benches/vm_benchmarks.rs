use criterion::{black_box, criterion_group, criterion_main, Criterion};
use vm_types::{Dag, Task, TaskCharacteristics, ComputationType, Instruction, TaskId};
use vm_scheduler::{DataflowRuntimeImpl, MemoizationEngineImpl};
use vm_memory::{MemorySystemImpl};
use vm_cpu::{VonNeumannCoreImpl};

fn create_simple_dag(size: usize) -> Dag {
    let mut tasks = Vec::new();

    for i in 0..size {
        let task = Task {
            id: i as u64,
            operation: vec![Instruction::Halt],
            dependencies: vec![],
            estimated_execution_time: 10,
            characteristics: TaskCharacteristics {
                computation_type: ComputationType::GeneralPurpose,
                data_size: 1024,
                parallelism_factor: 1,
                memory_intensity: 0.1,
            },
        };
        tasks.push(task);
    }

    Dag { tasks }
}

fn bench_dag_creation(c: &mut Criterion) {
    let mut group = c.benchmark_group("dag_creation");

    group.bench_function("create_dag_100", |b| {
        b.iter(|| {
            let dag = create_simple_dag(100);
            black_box(dag);
        });
    });

    group.bench_function("create_dag_1000", |b| {
        b.iter(|| {
            let dag = create_simple_dag(1000);
            black_box(dag);
        });
    });

    group.finish();
}

fn bench_memory_allocation(c: &mut Criterion) {
    let mut group = c.benchmark_group("memory_allocation");

    group.bench_function("allocate_1kb", |b| {
        b.iter(|| {
            let memory = MemorySystemImpl::new(1024);
            black_box(memory);
        });
    });

    group.bench_function("allocate_1mb", |b| {
        b.iter(|| {
            let memory = MemorySystemImpl::new(1024 * 1024);
            black_box(memory);
        });
    });

    group.finish();
}

fn bench_cpu_operations(c: &mut Criterion) {
    let mut group = c.benchmark_group("cpu_operations");

    group.bench_function("create_cpu_core", |b| {
        b.iter(|| {
            let cpu = VonNeumannCoreImpl::new();
            black_box(cpu);
        });
    });

    group.finish();
}

criterion_group!(benches, bench_dag_creation, bench_memory_allocation, bench_cpu_operations);
criterion_main!(benches);