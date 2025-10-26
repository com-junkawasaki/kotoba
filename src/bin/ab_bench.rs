use std::time::{Duration, Instant};
use std::sync::Arc;
use std::thread;
use serde::Serialize;
use eaf_ipg_runtime::engidb::adapter::GraphAdapter;
use eaf_ipg_runtime::engidb::adapter::SledAdapter;
use eaf_ipg_runtime::engidb::EngiDB;
#[cfg(feature = "fcdb")] use eaf_ipg_runtime::engidb::adapter::fcdb_adapter::FcdbAdapter;
use kotoba_types::{Graph, Node};

#[derive(Serialize)]
struct Stat { p50_ms: f64, p95_ms: f64 }

#[derive(Serialize)]
struct Report {
    scenario: String,
    threads: usize,
    cold: bool,
    iters: usize,
    stat: Stat,
}

fn percentile(mut v: Vec<f64>, p: f64) -> f64 {
    if v.is_empty() { return 0.0; }
    v.sort_by(|a,b| a.partial_cmp(b).unwrap());
    let idx = ((v.len() as f64 - 1.0) * p).round() as usize;
    v[idx]
}

fn bench_parallel<F>(threads: usize, iters: usize, f: F) -> Stat
where F: Fn() + Send + Sync + 'static {
    let f = Arc::new(f);
    let mut handles = Vec::new();
    let mut all = Vec::new();
    for _ in 0..threads {
        let f2 = f.clone();
        handles.push(thread::spawn(move || {
            let mut times = Vec::with_capacity(iters);
            for _ in 0..iters { let t0 = Instant::now(); f2(); times.push(t0.elapsed().as_secs_f64()*1000.0); }
            times
        }));
    }
    for h in handles { let mut v = h.join().unwrap(); all.append(&mut v); }
    Stat { p50_ms: percentile(all.clone(), 0.50), p95_ms: percentile(all, 0.95) }
}

fn make_line_graph(n: u64) -> Graph {
    let mut nodes = Vec::new();
    for i in 0..n { nodes.push(Node { id: format!("n{i}"), kind: "Vertex".into(), properties: Default::default() }); }
    // Minimal edges/incidence for adapter.import_graph to connect; leave empty here and use add_edge in benchmark
    Graph { node: nodes, edge: Vec::new(), incidence: Vec::new() }
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let threads: usize = args.iter().position(|a| a=="--threads").and_then(|i| args.get(i+1)).and_then(|s| s.parse().ok()).unwrap_or(8);
    let cold = args.iter().any(|a| a=="--cold");
    let iters: usize = args.iter().position(|a| a=="--iters").and_then(|i| args.get(i+1)).and_then(|s| s.parse().ok()).unwrap_or(200);
    let db_path = args.iter().position(|a| a=="--db").and_then(|i| args.get(i+1)).map(|s| s.as_str()).unwrap_or("./bench_db");

    if cold { std::fs::remove_dir_all(db_path).ok(); }

    #[cfg(feature = "fcdb")]
    let adapter: Arc<dyn GraphAdapter + Send + Sync> = Arc::new(
        FcdbAdapter::new_sync(std::path::PathBuf::from(db_path)).expect("create fcdb adapter")
    );
    #[cfg(not(feature = "fcdb"))]
    let adapter: Arc<dyn GraphAdapter + Send + Sync> = {
        let sled = EngiDB::open(db_path).expect("open db");
        Arc::new(SledAdapter::new(sled))
    };

    // Point lookup benchmark: add N nodes, then repeatedly add/get edges around one node
    let line = make_line_graph(1_000);
    adapter.import_graph(&line).expect("import graph");
    let start_id = 1u64;

    let rep1 = {
        let a = Arc::clone(&adapter);
        let stat = bench_parallel(threads, iters, move || {
            // simulate adjacency fanout from start_id via a synthetic edge label
            let _ = a.get_edges_from(start_id, "next").ok();
        });
        Report { scenario: "point_lookup".into(), threads, cold, iters, stat }
    };

    // Fanout benchmark: create star edges from center
    for t in 2..(2+1_000u64) { let _ = adapter.add_edge(start_id, "next", t); }
    let rep2 = {
        let a = Arc::clone(&adapter);
        let stat = bench_parallel(threads, iters, move || { let _ = a.get_edges_from(start_id, "next").ok(); });
        Report { scenario: "fanout_adjacent".into(), threads, cold, iters, stat }
    };

    // N-hop traversal approximation: do chained get_edges_from on synthetic chain
    for i in 1..5_000u64 { let _ = adapter.add_edge(i, "chain", i+1); }
    let hop = |a: &Arc<dyn GraphAdapter + Send + Sync>, mut v: u64, k: usize| {
        for _ in 0..k { if let Ok(ns) = a.get_edges_from(v, "chain") { if let Some(n1) = ns.first() { v = *n1; } } }
    };
    let rep3 = {
        let a = Arc::clone(&adapter);
        let stat = bench_parallel(threads, iters, move || { hop(&a, 1, 2); });
        Report { scenario: "hop_2".into(), threads, cold, iters, stat }
    };
    let rep4 = {
        let a = Arc::clone(&adapter);
        let stat = bench_parallel(threads, iters, move || { hop(&a, 1, 3); });
        Report { scenario: "hop_3".into(), threads, cold, iters, stat }
    };
    let rep5 = {
        let a = Arc::clone(&adapter);
        let stat = bench_parallel(threads, iters, move || { hop(&a, 1, 4); });
        Report { scenario: "hop_4".into(), threads, cold, iters, stat }
    };

    // Write burst with and without batch (sled implicit). Simulate by inserting edges.
    let write_burst = |a: &Arc<dyn GraphAdapter + Send + Sync>, n: u64| {
        for i in 0..n { let _ = a.add_edge(10_000+i, "wb", 20_000+i); }
    };
    let rep6 = {
        let a = Arc::clone(&adapter);
        let stat = bench_parallel(threads, 1, move || { write_burst(&a, 10_000); });
        Report { scenario: "write_burst_10k".into(), threads, cold, iters: 1, stat }
    };
    let rep7 = {
        let a = Arc::clone(&adapter);
        let stat = bench_parallel(threads, 1, move || { write_burst(&a, 100_000); });
        Report { scenario: "write_burst_100k".into(), threads, cold, iters: 1, stat }
    };

    let out = serde_json::to_string_pretty(&vec![rep1, rep2, rep3, rep4, rep5, rep6, rep7]).unwrap();
    println!("{}", out);
}


