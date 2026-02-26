//! Benchmarks for parsing AgentScript recipes.
//!
//! Run with: cargo bench
//! Results are saved to target/criterion/

use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};
use glob::glob;
use std::fs;

/// Load all recipe files from the agent-script-recipes submodule.
fn load_recipes() -> Vec<(String, String)> {
    let mut recipes = Vec::new();

    // Find all .agent files in the submodule
    // Try multiple paths to handle different working directories
    let patterns = [
        "agent-script-recipes/**/*.agent",
        "../../agent-script-recipes/**/*.agent",
    ];

    for pattern in patterns {
        for path in glob(pattern).expect("Failed to read glob pattern").flatten() {
            if let Ok(content) = fs::read_to_string(&path) {
                let name = path
                    .file_stem()
                    .and_then(|s| s.to_str())
                    .unwrap_or("unknown")
                    .to_string();
                // Avoid duplicates
                if !recipes.iter().any(|(n, _)| n == &name) {
                    recipes.push((name, content));
                }
            }
        }
    }

    // Sort by name for consistent ordering
    recipes.sort_by(|a, b| a.0.cmp(&b.0));
    recipes
}

/// Load the ComprehensiveDemo.agent example file.
fn load_comprehensive_demo() -> Option<String> {
    let paths = [
        "examples/ComprehensiveDemo.agent",
        "../../examples/ComprehensiveDemo.agent",
    ];

    for path in paths {
        if let Ok(content) = fs::read_to_string(path) {
            return Some(content);
        }
    }
    None
}

/// Benchmark parsing all recipes.
fn bench_parse_all(c: &mut Criterion) {
    let recipes = load_recipes();

    if recipes.is_empty() {
        eprintln!("Warning: No recipe files found. Make sure git submodules are initialized.");
        return;
    }

    let total_bytes: usize = recipes.iter().map(|(_, content)| content.len()).sum();

    let mut group = c.benchmark_group("parse_all_recipes");
    group.throughput(Throughput::Bytes(total_bytes as u64));

    group.bench_function("parse", |b| {
        b.iter(|| {
            for (_, content) in &recipes {
                let _ = black_box(busbar_sf_agentscript_parser::parse(content));
            }
        });
    });

    group.bench_function("parse_and_serialize", |b| {
        b.iter(|| {
            for (_, content) in &recipes {
                if let Ok(ast) = busbar_sf_agentscript_parser::parse(content) {
                    let _ = black_box(serde_json::to_string(&ast));
                }
            }
        });
    });

    group.finish();
}

/// Benchmark individual recipes.
fn bench_individual_recipes(c: &mut Criterion) {
    let recipes = load_recipes();

    if recipes.is_empty() {
        return;
    }

    // Benchmark parsing individual recipes
    let mut parse_group = c.benchmark_group("parse_individual");
    for (name, content) in &recipes {
        parse_group.throughput(Throughput::Bytes(content.len() as u64));
        parse_group.bench_with_input(BenchmarkId::new("parse", name), content, |b, content| {
            b.iter(|| black_box(busbar_sf_agentscript_parser::parse(content)));
        });
    }
    parse_group.finish();

    // Benchmark parse + serialize for individual recipes
    let mut serialize_group = c.benchmark_group("serialize_individual");
    for (name, content) in &recipes {
        serialize_group.throughput(Throughput::Bytes(content.len() as u64));
        serialize_group.bench_with_input(
            BenchmarkId::new("parse_serialize", name),
            content,
            |b, content| {
                b.iter(|| {
                    if let Ok(ast) = busbar_sf_agentscript_parser::parse(content) {
                        black_box(serde_json::to_string(&ast))
                    } else {
                        Ok(String::new())
                    }
                });
            },
        );
    }
    serialize_group.finish();
}

/// Benchmark by recipe size categories.
fn bench_by_size(c: &mut Criterion) {
    let recipes = load_recipes();

    if recipes.is_empty() {
        return;
    }

    // Categorize by size
    let small: Vec<_> = recipes
        .iter()
        .filter(|(_, c)| c.len() < 1000)
        .cloned()
        .collect();
    let medium: Vec<_> = recipes
        .iter()
        .filter(|(_, c)| c.len() >= 1000 && c.len() < 3000)
        .cloned()
        .collect();
    let large: Vec<_> = recipes
        .iter()
        .filter(|(_, c)| c.len() >= 3000)
        .cloned()
        .collect();

    let mut group = c.benchmark_group("parse_by_size");

    if !small.is_empty() {
        let bytes: usize = small.iter().map(|(_, c)| c.len()).sum();
        group.throughput(Throughput::Bytes(bytes as u64));
        group.bench_function(format!("small_<1KB_{}_files", small.len()), |b| {
            b.iter(|| {
                for (_, content) in &small {
                    let _ = black_box(busbar_sf_agentscript_parser::parse(content));
                }
            });
        });
    }

    if !medium.is_empty() {
        let bytes: usize = medium.iter().map(|(_, c)| c.len()).sum();
        group.throughput(Throughput::Bytes(bytes as u64));
        group.bench_function(format!("medium_1-3KB_{}_files", medium.len()), |b| {
            b.iter(|| {
                for (_, content) in &medium {
                    let _ = black_box(busbar_sf_agentscript_parser::parse(content));
                }
            });
        });
    }

    if !large.is_empty() {
        let bytes: usize = large.iter().map(|(_, c)| c.len()).sum();
        group.throughput(Throughput::Bytes(bytes as u64));
        group.bench_function(format!("large_>3KB_{}_files", large.len()), |b| {
            b.iter(|| {
                for (_, content) in &large {
                    let _ = black_box(busbar_sf_agentscript_parser::parse(content));
                }
            });
        });
    }

    group.finish();
}

/// Benchmark the ComprehensiveDemo.agent file specifically.
/// This file contains all AgentScript language features and serves as a reference benchmark.
fn bench_comprehensive_demo(c: &mut Criterion) {
    let content = match load_comprehensive_demo() {
        Some(c) => c,
        None => {
            eprintln!("Warning: ComprehensiveDemo.agent not found");
            return;
        }
    };

    let mut group = c.benchmark_group("comprehensive_demo");
    group.throughput(Throughput::Bytes(content.len() as u64));

    group.bench_function("parse", |b| {
        b.iter(|| black_box(busbar_sf_agentscript_parser::parse(&content)));
    });

    group.bench_function("parse_and_serialize_json", |b| {
        b.iter(|| {
            if let Ok(ast) = busbar_sf_agentscript_parser::parse(&content) {
                black_box(serde_json::to_string(&ast))
            } else {
                Ok(String::new())
            }
        });
    });

    group.bench_function("parse_and_serialize_agentscript", |b| {
        b.iter(|| {
            if let Ok(ast) = busbar_sf_agentscript_parser::parse(&content) {
                black_box(busbar_sf_agentscript_parser::serialize(&ast))
            } else {
                String::new()
            }
        });
    });

    group.finish();
}

criterion_group!(
    benches,
    bench_parse_all,
    bench_individual_recipes,
    bench_by_size,
    bench_comprehensive_demo
);
criterion_main!(benches);
