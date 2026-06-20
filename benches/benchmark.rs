fn bench_short(
    group: &mut criterion::BenchmarkGroup<criterion::measurement::WallTime>,
    pattern: &str,
    targets: &[&str],
) {
    let dfa = rustegex::Engine::new(pattern, "dfa").unwrap();
    group.bench_function("rustegex/dfa", |b| {
        b.iter(|| {
            for target in targets {
                dfa.is_match(target);
            }
        });
    });

    let vm = rustegex::Engine::new(pattern, "vm").unwrap();
    group.bench_function("rustegex/vm", |b| {
        b.iter(|| {
            for target in targets {
                vm.is_match(target);
            }
        });
    });

    let derivative = rustegex::Engine::new(pattern, "derivative").unwrap();
    group.bench_function("rustegex/derivative", |b| {
        b.iter(|| {
            for target in targets {
                derivative.is_match(target);
            }
        });
    });

    let re = regex::Regex::new(pattern).unwrap();
    group.bench_function("regex", |b| {
        b.iter(|| {
            for target in targets {
                re.is_match(target);
            }
        });
    });
}

fn bench_long(
    group: &mut criterion::BenchmarkGroup<criterion::measurement::WallTime>,
    pattern: &str,
    input: &str,
) {
    let dfa = rustegex::Engine::new(pattern, "dfa").unwrap();
    group.bench_function("rustegex/dfa", |b| {
        b.iter(|| {
            dfa.is_match(input);
        });
    });

    let vm = rustegex::Engine::new(pattern, "vm").unwrap();
    group.bench_function("rustegex/vm", |b| {
        b.iter(|| {
            vm.is_match(input);
        });
    });

    let derivative = rustegex::Engine::new(pattern, "derivative").unwrap();
    group.bench_function("rustegex/derivative", |b| {
        b.iter(|| {
            derivative.is_match(input);
        });
    });

    let re = regex::Regex::new(pattern).unwrap();
    group.bench_function("regex", |b| {
        b.iter(|| {
            re.is_match(input);
        });
    });
}

fn case_1(c: &mut criterion::Criterion) {
    let pattern = "(p(erl|ython|hp)|ruby)";
    let targets = ["perl", "python", "ruby", "rust"];

    let mut group = c.benchmark_group("case 1");
    bench_short(&mut group, pattern, &targets);
    group.finish();
}

fn case_2(c: &mut criterion::Criterion) {
    let pattern = "ab(cd|)ef|g*|h+";
    let targets = ["abcdef", "abef", "abefg", "abefgh", "", "ggggg", "hhhh"];

    let mut group = c.benchmark_group("case 2");
    bench_short(&mut group, pattern, &targets);
    group.finish();
}

fn case_long(c: &mut criterion::Criterion) {
    let pattern = "a+b";
    let input = "a".repeat(1_000_000);

    let mut group = c.benchmark_group("case long");
    bench_long(&mut group, pattern, &input);
    group.finish();
}

fn case_meta(c: &mut criterion::Criterion) {
    let pattern = r"a\db|\s\w+|.\d";
    let targets = ["a0b", "a9b", "axb", " foo", "\tBar", "x5", "nope", "a\nb"];

    let mut group = c.benchmark_group("case meta");
    bench_short(&mut group, pattern, &targets);
    group.finish();
}

fn case_meta_long(c: &mut criterion::Criterion) {
    let pattern = r"\d+";
    let input = "0123456789".repeat(100_000);

    let mut group = c.benchmark_group("case meta long");
    bench_long(&mut group, pattern, &input);
    group.finish();
}

criterion::criterion_group!(
    benches,
    case_1,
    case_2,
    case_long,
    case_meta,
    case_meta_long,
);
criterion::criterion_main!(benches);
