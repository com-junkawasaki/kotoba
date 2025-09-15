//! Jsonnet評価のパフォーマンスベンチマーク

use criterion::{black_box, criterion_group, criterion_main, Criterion};
use kotoba_jsonnet::*;

/// 簡単なJsonnet式の評価ベンチマーク
fn bench_simple_expression(c: &mut Criterion) {
    c.bench_function("jsonnet_simple_expr", |b| {
        b.iter(|| {
            let result = evaluate(r#"42 + 24"#);
            black_box(result.unwrap());
        });
    });
}

/// オブジェクト作成のベンチマーク
fn bench_object_creation(c: &mut Criterion) {
    c.bench_function("jsonnet_object_creation", |b| {
        b.iter(|| {
            let result = evaluate(r#"{ name: "test", value: 42 }"#);
            black_box(result.unwrap());
        });
    });
}

/// 配列操作のベンチマーク
fn bench_array_operations(c: &mut Criterion) {
    c.bench_function("jsonnet_array_ops", |b| {
        b.iter(|| {
            let result = evaluate(r#"[x * 2 for x in [1, 2, 3, 4, 5]]"#);
            black_box(result.unwrap());
        });
    });
}

/// 関数定義と呼び出しのベンチマーク
fn bench_function_call(c: &mut Criterion) {
    c.bench_function("jsonnet_function_call", |b| {
        b.iter(|| {
            let result = evaluate(r#"local add = function(x, y) x + y; add(10, 20)"#);
            black_box(result.unwrap());
        });
    });
}

/// 文字列補間のベンチマーク
fn bench_string_interpolation(c: &mut Criterion) {
    c.bench_function("jsonnet_string_interp", |b| {
        b.iter(|| {
            let result = evaluate(r#"local name = "World"; "Hello, %(name)s!""#);
            black_box(result.unwrap());
        });
    });
}

/// ローカル変数のベンチマーク
fn bench_local_variables(c: &mut Criterion) {
    c.bench_function("jsonnet_local_vars", |b| {
        b.iter(|| {
            let result = evaluate(r#"local x = 10, y = 20; x + y * 2"#);
            black_box(result.unwrap());
        });
    });
}

/// 条件式のベンチマーク
fn bench_conditionals(c: &mut Criterion) {
    c.bench_function("jsonnet_conditionals", |b| {
        b.iter(|| {
            let result = evaluate(r#"if 10 > 5 then "greater" else "smaller""#);
            black_box(result.unwrap());
        });
    });
}

/// stdライブラリ関数のベンチマーク
fn bench_std_functions(c: &mut Criterion) {
    c.bench_function("jsonnet_std_length", |b| {
        b.iter(|| {
            let result = evaluate(r#"std.length([1, 2, 3, 4, 5])"#);
            black_box(result.unwrap());
        });
    });
}

/// 大きなJsonnetファイルの評価ベンチマーク
fn bench_large_evaluation(c: &mut Criterion) {
    let large_code = r#"
    local data = [
      { id: 1, name: "Alice", age: 30 },
      { id: 2, name: "Bob", age: 25 },
    ];

    {
      count: std.length(data),
      first_name: data[0].name,
    }
    "#;

    c.bench_function("jsonnet_large_eval", |b| {
        b.iter(|| {
            let result = evaluate(large_code);
            black_box(result.unwrap());
        });
    });
}

criterion_group! {
    name = jsonnet_benches;
    config = Criterion::default()
        .sample_size(10)
        .measurement_time(std::time::Duration::from_secs(1))
        .warm_up_time(std::time::Duration::from_millis(100));
    targets = bench_simple_expression,
             bench_object_creation,
             bench_array_operations,
             bench_function_call,
             bench_string_interpolation,
             bench_local_variables,
             bench_conditionals,
             bench_std_functions,
             bench_large_evaluation
}

criterion_main!(jsonnet_benches);
