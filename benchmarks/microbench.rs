//! Hermes microbenchmarks using criterion
//!
//! Run with: cargo bench

use criterion::{black_box, criterion_group, criterion_main, Criterion};
use hermes_common::Provider;

fn bench_provider_from_str(c: &mut Criterion) {
    let mut group = c.benchmark_group("provider_parsing");
    group.bench_function("parse_openai", |b| {
        b.iter(|| "openai".parse::<Provider>().unwrap())
    });
    group.bench_function("parse_anthropic", |b| {
        b.iter(|| "anthropic".parse::<Provider>().unwrap())
    });
    group.bench_function("parse_deepseek", |b| {
        b.iter(|| "deepseek".parse::<Provider>().unwrap())
    });
    group.bench_function("parse_case_insensitive", |b| {
        b.iter(|| "OpenAI".parse::<Provider>().unwrap())
    });
    group.bench_function("parse_invalid", |b| {
        b.iter(|| black_box("notaprovider").parse::<Provider>())
    });
    group.finish();
}

fn bench_provider_detection(c: &mut Criterion) {
    let mut group = c.benchmark_group("provider_detection");
    group.bench_function("detect_openai_url", |b| {
        b.iter(|| hermes_common::detect_provider_from_url("https://api.openai.com/v1/chat/completions"))
    });
    group.bench_function("detect_anthropic_url", |b| {
        b.iter(|| hermes_common::detect_provider_from_url("https://api.anthropic.com/v1/messages"))
    });
    group.bench_function("detect_unknown_url", |b| {
        b.iter(|| hermes_common::detect_provider_from_url("https://example.com/api"))
    });
    group.finish();
}

fn bench_token_estimation(c: &mut Criterion) {
    let mut group = c.benchmark_group("token_estimation");
    group.bench_function("estimate_short", |b| {
        b.iter(|| hermes_runtime::context::token_est::estimate_tokens("Hello, world!"))
    });
    let long_text = "This is a longer piece of text. ".repeat(100);
    group.bench_function("estimate_long", |b| {
        b.iter(|| hermes_runtime::context::token_est::estimate_tokens(&long_text))
    });
    group.finish();
}

fn bench_model_metadata(c: &mut Criterion) {
    let mut group = c.benchmark_group("model_metadata");
    group.bench_function("lookup_gpt4o", |b| {
        b.iter(|| hermes_common::model_metadata::get_model_metadata("gpt-4o"))
    });
    group.bench_function("cost_estimate", |b| {
        b.iter(|| hermes_common::model_metadata::estimate_cost("gpt-4o", 1000, 500))
    });
    group.bench_function("all_models", |b| {
        b.iter(|| hermes_common::model_metadata::all_models().len())
    });
    group.finish();
}

criterion_group!(
    benches,
    bench_provider_from_str,
    bench_provider_detection,
    bench_token_estimation,
    bench_model_metadata,
);
criterion_main!(benches);
