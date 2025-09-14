//! Simple Jsonnet performance benchmark script

use std::time::{Duration, Instant};

// 直接Jsonnetクレートを使用
use kotoba_jsonnet::*;

/// 簡単なJsonnet式の評価ベンチマーク
fn benchmark_simple_expression(iterations: u32) {
    let mut total_time = Duration::new(0, 0);

    for _ in 0..iterations {
        let start = Instant::now();
        let result = evaluate(r#"42 + 24"#);
        let elapsed = start.elapsed();
        total_time += elapsed;

        // Verify result
        assert!(result.is_ok());
        if let JsonnetValue::Number(n) = result.unwrap() {
            assert_eq!(n, 66.0);
        } else {
            panic!("Expected number");
        }
    }

    let avg_time = total_time / iterations;
    println!("Simple expression (42 + 24): {} iterations, avg: {:.2} μs",
             iterations, avg_time.as_micros());
}

/// オブジェクト作成のベンチマーク
fn benchmark_object_creation(iterations: u32) {
    let mut total_time = Duration::new(0, 0);

    for _ in 0..iterations {
        let start = Instant::now();
        let result = evaluate(r#"{ name: "test", value: 42 }"#);
        let elapsed = start.elapsed();
        total_time += elapsed;

        assert!(result.is_ok());
    }

    let avg_time = total_time / iterations;
    println!("Object creation: {} iterations, avg: {:.2} μs",
             iterations, avg_time.as_micros());
}

/// 配列操作のベンチマーク
fn benchmark_array_operations(iterations: u32) {
    let mut total_time = Duration::new(0, 0);

    for _ in 0..iterations {
        let start = Instant::now();
        let result = evaluate(r#"[1, 2, 3, 4, 5].map(function(x) x * 2)"#);
        let elapsed = start.elapsed();
        total_time += elapsed;

        assert!(result.is_ok());
    }

    let avg_time = total_time / iterations;
    println!("Array operations: {} iterations, avg: {:.2} μs",
             iterations, avg_time.as_micros());
}

/// 関数定義と呼び出しのベンチマーク
fn benchmark_function_calls(iterations: u32) {
    let mut total_time = Duration::new(0, 0);

    for _ in 0..iterations {
        let start = Instant::now();
        let result = evaluate(r#"local add = function(x, y) x + y; add(10, 20)"#);
        let elapsed = start.elapsed();
        total_time += elapsed;

        assert!(result.is_ok());
    }

    let avg_time = total_time / iterations;
    println!("Function calls: {} iterations, avg: {:.2} μs",
             iterations, avg_time.as_micros());
}

/// 文字列補間のベンチマーク
fn benchmark_string_interpolation(iterations: u32) {
    let mut total_time = Duration::new(0, 0);

    for _ in 0..iterations {
        let start = Instant::now();
        let result = evaluate(r#"local name = "World"; "Hello, %(name)s!""#);
        let elapsed = start.elapsed();
        total_time += elapsed;

        assert!(result.is_ok());
    }

    let avg_time = total_time / iterations;
    println!("String interpolation: {} iterations, avg: {:.2} μs",
             iterations, avg_time.as_micros());
}

/// ローカル変数のベンチマーク
fn benchmark_local_variables(iterations: u32) {
    let mut total_time = Duration::new(0, 0);

    for _ in 0..iterations {
        let start = Instant::now();
        let result = evaluate(r#"local x = 10, y = 20; x + y * 2"#);
        let elapsed = start.elapsed();
        total_time += elapsed;

        assert!(result.is_ok());
    }

    let avg_time = total_time / iterations;
    println!("Local variables: {} iterations, avg: {:.2} μs",
             iterations, avg_time.as_micros());
}

/// 条件式のベンチマーク
fn benchmark_conditionals(iterations: u32) {
    let mut total_time = Duration::new(0, 0);

    for _ in 0..iterations {
        let start = Instant::now();
        let result = evaluate(r#"if 10 > 5 then "greater" else "smaller""#);
        let elapsed = start.elapsed();
        total_time += elapsed;

        assert!(result.is_ok());
    }

    let avg_time = total_time / iterations;
    println!("Conditionals: {} iterations, avg: {:.2} μs",
             iterations, avg_time.as_micros());
}

/// stdライブラリ関数のベンチマーク
fn benchmark_std_functions(iterations: u32) {
    let mut total_time = Duration::new(0, 0);

    for _ in 0..iterations {
        let start = Instant::now();
        let result = evaluate(r#"std.length([1, 2, 3, 4, 5])"#);
        let elapsed = start.elapsed();
        total_time += elapsed;

        assert!(result.is_ok());
    }

    let avg_time = total_time / iterations;
    println!("Std functions: {} iterations, avg: {:.2} μs",
             iterations, avg_time.as_micros());
}

/// 大きなJsonnetファイルの評価ベンチマーク
fn benchmark_large_evaluation(iterations: u32) {
    let large_code = r#"
    local data = [
      { id: 1, name: "Alice", age: 30 },
      { id: 2, name: "Bob", age: 25 },
      { id: 3, name: "Charlie", age: 35 },
    ];

    local process = function(person)
      person + { adult: person.age >= 18 };

    {
      processed: data.map(process),
      total_age: data.foldLeft(0, function(acc, p) acc + p.age),
      names: data.map(function(p) p.name),
      average_age: data.foldLeft(0, function(acc, p) acc + p.age) / data.length(),
    }
    "#;

    let mut total_time = Duration::new(0, 0);

    for _ in 0..iterations {
        let start = Instant::now();
        let result = evaluate(large_code);
        let elapsed = start.elapsed();
        total_time += elapsed;

        assert!(result.is_ok());
    }

    let avg_time = total_time / iterations;
    println!("Large evaluation: {} iterations, avg: {:.2} μs",
             iterations, avg_time.as_micros());
}

fn main() {
    println!("Kotoba Jsonnet Performance Benchmark");
    println!("=====================================");
    println!("Running on: {}", std::env::consts::OS);

    let iterations = 1000;

    benchmark_simple_expression(iterations);
    benchmark_object_creation(iterations);
    benchmark_array_operations(iterations);
    benchmark_function_calls(iterations);
    benchmark_string_interpolation(iterations);
    benchmark_local_variables(iterations);
    benchmark_conditionals(iterations);
    benchmark_std_functions(iterations);
    benchmark_large_evaluation(iterations / 10); // Large evaluation is slower

    println!("\nBenchmark completed successfully!");
    println!("\nPerformance Summary:");
    println!("- All benchmarks use 1000 iterations (except large evaluation: 100)");
    println!("- Times shown are averages in microseconds (μs)");
    println!("- Lower values indicate better performance");
}