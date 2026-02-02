//! Concurrency benchmarks for aria-runtime
//!
//! These benchmarks measure the performance of Aria's concurrency primitives
//! against the design targets from ARIA-PD-006:
//! - Context switch: < 300ns
//! - Task spawn (with pool): < 1μs
//! - Channel operations: competitive with Go channels

use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::thread;

use aria_runtime::{
    buffered, unbuffered,
    pool_spawn,
    with_scope, with_scope_result,
    CancelToken,
};

// ============================================================================
// Task Spawn Benchmarks
// ============================================================================

fn bench_spawn_latency(c: &mut Criterion) {
    let mut group = c.benchmark_group("spawn_latency");

    // Benchmark thread pool spawn (target: < 1μs)
    group.bench_function("pool_spawn_noop", |b| {
        b.iter(|| {
            let handle = pool_spawn(|| black_box(42));
            handle.join()
        })
    });

    // Benchmark raw thread spawn for comparison
    group.bench_function("std_thread_spawn_noop", |b| {
        b.iter(|| {
            let handle = thread::spawn(|| black_box(42));
            handle.join().unwrap()
        })
    });

    // Benchmark spawn with actual work
    group.bench_function("pool_spawn_light_work", |b| {
        b.iter(|| {
            let handle = pool_spawn(|| {
                let mut sum = 0u64;
                for i in 0..100 {
                    sum += black_box(i);
                }
                sum
            });
            handle.join()
        })
    });

    group.finish();
}

fn bench_spawn_throughput(c: &mut Criterion) {
    let mut group = c.benchmark_group("spawn_throughput");

    for num_tasks in [10, 100, 1000, 10000].iter() {
        group.throughput(Throughput::Elements(*num_tasks as u64));

        group.bench_with_input(
            BenchmarkId::new("pool_spawn", num_tasks),
            num_tasks,
            |b, &n| {
                b.iter(|| {
                    let handles: Vec<_> = (0..n)
                        .map(|i| pool_spawn(move || black_box(i * 2)))
                        .collect();

                    for h in handles {
                        let _ = h.join();
                    }
                })
            },
        );
    }

    group.finish();
}

// ============================================================================
// Channel Benchmarks
// ============================================================================

fn bench_channel_latency(c: &mut Criterion) {
    let mut group = c.benchmark_group("channel_latency");

    // Unbuffered channel single send/recv
    group.bench_function("unbuffered_single", |b| {
        let (tx, rx) = unbuffered::<i32>();
        b.iter(|| {
            let tx = tx.clone();
            let h = thread::spawn(move || tx.send(black_box(42)));
            let val = rx.recv().unwrap();
            let _ = h.join();
            val
        })
    });

    // Buffered channel single send/recv
    group.bench_function("buffered_single", |b| {
        let (tx, rx) = buffered::<i32>(16);
        b.iter(|| {
            tx.send(black_box(42)).unwrap();
            rx.recv().unwrap()
        })
    });

    group.finish();
}

fn bench_channel_throughput(c: &mut Criterion) {
    let mut group = c.benchmark_group("channel_throughput");

    for msg_count in [100, 1000, 10000].iter() {
        group.throughput(Throughput::Elements(*msg_count as u64));

        // Buffered channel SPSC throughput
        group.bench_with_input(
            BenchmarkId::new("buffered_spsc", msg_count),
            msg_count,
            |b, &n| {
                b.iter(|| {
                    let (tx, rx) = buffered::<i32>(64);

                    let producer = thread::spawn(move || {
                        for i in 0..n {
                            tx.send(i as i32).unwrap();
                        }
                    });

                    let mut sum = 0i64;
                    for _ in 0..n {
                        sum += rx.recv().unwrap() as i64;
                    }

                    producer.join().unwrap();
                    black_box(sum)
                })
            },
        );

        // Buffered channel MPSC throughput
        group.bench_with_input(
            BenchmarkId::new("buffered_mpsc_4producers", msg_count),
            msg_count,
            |b, &n| {
                b.iter(|| {
                    let (tx, rx) = buffered::<i32>(64);
                    let per_producer = n / 4;

                    let producers: Vec<_> = (0..4)
                        .map(|p| {
                            let tx = tx.clone();
                            thread::spawn(move || {
                                for i in 0..per_producer {
                                    tx.send((p * per_producer + i) as i32).unwrap();
                                }
                            })
                        })
                        .collect();

                    drop(tx); // Drop original sender

                    let mut sum = 0i64;
                    for _ in 0..(per_producer * 4) {
                        sum += rx.recv().unwrap() as i64;
                    }

                    for p in producers {
                        p.join().unwrap();
                    }
                    black_box(sum)
                })
            },
        );
    }

    group.finish();
}

// ============================================================================
// Scope Benchmarks
// ============================================================================

fn bench_scope_overhead(c: &mut Criterion) {
    let mut group = c.benchmark_group("scope_overhead");

    // Empty scope overhead
    group.bench_function("empty_scope", |b| {
        b.iter(|| {
            with_scope(|_scope| {
                black_box(42)
            })
        })
    });

    // Scope with single task
    group.bench_function("scope_single_task", |b| {
        b.iter(|| {
            with_scope(|scope| {
                let h = scope.spawn(|| black_box(42));
                h.join().unwrap()
            })
        })
    });

    // Scope with multiple tasks
    group.bench_function("scope_10_tasks", |b| {
        b.iter(|| {
            with_scope(|scope| {
                let handles: Vec<_> = (0..10)
                    .map(|i| scope.spawn(move || black_box(i * 2)))
                    .collect();

                let mut sum = 0;
                for h in handles {
                    sum += h.join().unwrap();
                }
                sum
            })
        })
    });

    group.finish();
}

fn bench_scope_cancellation(c: &mut Criterion) {
    let mut group = c.benchmark_group("scope_cancellation");

    // Cancel token creation
    group.bench_function("cancel_token_new", |b| {
        b.iter(|| {
            black_box(CancelToken::new())
        })
    });

    // Cancel token check (not cancelled)
    group.bench_function("cancel_check_not_cancelled", |b| {
        let token = CancelToken::new();
        b.iter(|| {
            black_box(token.is_cancelled())
        })
    });

    // Cancel token check (cancelled)
    group.bench_function("cancel_check_cancelled", |b| {
        let token = CancelToken::new();
        token.cancel();
        b.iter(|| {
            black_box(token.is_cancelled())
        })
    });

    // Cancellation propagation latency
    group.bench_function("cancel_propagation", |b| {
        b.iter(|| {
            let token = CancelToken::new();
            let child = token.clone();

            let h = thread::spawn(move || {
                while !child.is_cancelled() {
                    // Spin
                }
            });

            token.cancel();
            h.join().unwrap();
        })
    });

    group.finish();
}

// ============================================================================
// Thread Pool Benchmarks
// ============================================================================

fn bench_pool_scalability(c: &mut Criterion) {
    let mut group = c.benchmark_group("pool_scalability");
    group.sample_size(50); // Reduce sample size for longer benchmarks

    // Work-stealing efficiency test
    for num_tasks in [100, 1000].iter() {
        group.bench_with_input(
            BenchmarkId::new("work_stealing", num_tasks),
            num_tasks,
            |b, &n| {
                b.iter(|| {
                    let counter = Arc::new(AtomicUsize::new(0));

                    let handles: Vec<_> = (0..n)
                        .map(|_| {
                            let counter = Arc::clone(&counter);
                            pool_spawn(move || {
                                // Simulate variable work
                                let work = black_box(100);
                                let mut sum = 0u64;
                                for i in 0..work {
                                    sum = sum.wrapping_add(i);
                                }
                                counter.fetch_add(1, Ordering::Relaxed);
                                sum
                            })
                        })
                        .collect();

                    for h in handles {
                        let _ = h.join();
                    }

                    assert_eq!(counter.load(Ordering::Relaxed), n);
                })
            },
        );
    }

    group.finish();
}

fn bench_pool_contention(c: &mut Criterion) {
    let mut group = c.benchmark_group("pool_contention");
    group.sample_size(30);

    // Test pool behavior under high contention
    group.bench_function("high_contention_shared_counter", |b| {
        b.iter(|| {
            let counter = Arc::new(AtomicUsize::new(0));
            let num_tasks = 100;

            let handles: Vec<_> = (0..num_tasks)
                .map(|_| {
                    let counter = Arc::clone(&counter);
                    pool_spawn(move || {
                        for _ in 0..100 {
                            counter.fetch_add(1, Ordering::Relaxed);
                        }
                    })
                })
                .collect();

            for h in handles {
                let _ = h.join();
            }

            assert_eq!(counter.load(Ordering::Relaxed), num_tasks * 100);
        })
    });

    group.finish();
}

// ============================================================================
// Structured Concurrency Patterns
// ============================================================================

fn bench_structured_patterns(c: &mut Criterion) {
    let mut group = c.benchmark_group("structured_patterns");
    group.sample_size(50);

    // Fork-join pattern
    group.bench_function("fork_join_10", |b| {
        b.iter(|| {
            with_scope_result(|scope| {
                let handles: Vec<_> = (0..10)
                    .map(|i| {
                        scope.spawn(move || {
                            // Simulate work
                            let mut sum = 0u64;
                            for j in 0..1000 {
                                sum = sum.wrapping_add(black_box(i as u64 * j));
                            }
                            sum
                        })
                    })
                    .collect();

                let mut total = 0u64;
                for h in handles {
                    total = total.wrapping_add(h.join().unwrap());
                }
                total
            })
        })
    });

    // Pipeline pattern with channels
    group.bench_function("pipeline_3_stages", |b| {
        b.iter(|| {
            let (tx1, rx1) = buffered::<i32>(16);
            let (tx2, rx2) = buffered::<i32>(16);

            // Stage 1: Producer
            let h1 = thread::spawn(move || {
                for i in 0..100 {
                    tx1.send(i).unwrap();
                }
            });

            // Stage 2: Transformer
            let h2 = thread::spawn(move || {
                while let Ok(v) = rx1.recv() {
                    tx2.send(v * 2).unwrap();
                }
            });

            // Stage 3: Consumer
            let mut sum = 0i64;
            for _ in 0..100 {
                sum += rx2.recv().unwrap() as i64;
            }

            h1.join().unwrap();
            drop(h2); // h2 will exit when rx1 closes

            black_box(sum)
        })
    });

    group.finish();
}

// ============================================================================
// Criterion Configuration
// ============================================================================

criterion_group!(
    spawn_benches,
    bench_spawn_latency,
    bench_spawn_throughput,
);

criterion_group!(
    channel_benches,
    bench_channel_latency,
    bench_channel_throughput,
);

criterion_group!(
    scope_benches,
    bench_scope_overhead,
    bench_scope_cancellation,
);

criterion_group!(
    pool_benches,
    bench_pool_scalability,
    bench_pool_contention,
);

criterion_group!(
    pattern_benches,
    bench_structured_patterns,
);

criterion_main!(
    spawn_benches,
    channel_benches,
    scope_benches,
    pool_benches,
    pattern_benches,
);
