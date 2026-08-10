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

fn bench_partial(
    group: &mut criterion::BenchmarkGroup<criterion::measurement::WallTime>,
    pattern: &str,
    input: &str,
) {
    let dfa = rustegex::Engine::new(pattern, "dfa").unwrap();
    group.bench_function("rustegex/dfa", |b| {
        b.iter(|| {
            dfa.is_partial_match(input);
        });
    });

    let vm = rustegex::Engine::new(pattern, "vm").unwrap();
    group.bench_function("rustegex/vm", |b| {
        b.iter(|| {
            vm.is_partial_match(input);
        });
    });

    let re = regex::Regex::new(pattern).unwrap();
    group.bench_function("regex", |b| {
        b.iter(|| {
            re.is_match(input);
        });
    });
}

fn case_partial_literal(c: &mut criterion::Criterion) {
    let pattern = "Sherlock";
    let input = format!(
        "{}Sherlock{}",
        "abcde ".repeat(50_000),
        "xyzzy ".repeat(50_000)
    );

    let mut group = c.benchmark_group("partial literal");
    bench_partial(&mut group, pattern, &input);
    group.finish();
}

fn case_partial_alt(c: &mut criterion::Criterion) {
    let pattern = "(p(erl|ython|hp)|ruby)";
    let input = format!("{}python", "lorem ipsum ".repeat(20_000));

    let mut group = c.benchmark_group("partial alt");
    bench_partial(&mut group, pattern, &input);
    group.finish();
}

fn case_partial_long(c: &mut criterion::Criterion) {
    let pattern = "a+b";
    let input = "a".repeat(1_000_000);

    let mut group = c.benchmark_group("partial long");
    bench_partial(&mut group, pattern, &input);
    group.finish();
}

fn case_partial_required_miss(c: &mut criterion::Criterion) {
    let pattern = "a*bcd";
    let input = "xyz ".repeat(50_000);

    let mut group = c.benchmark_group("partial required miss");
    bench_partial(&mut group, pattern, &input);
    group.finish();
}

fn case_partial_required_hit(c: &mut criterion::Criterion) {
    let pattern = "a*bcd";
    let input = format!("{}aaabcd", "xyz ".repeat(50_000));

    let mut group = c.benchmark_group("partial required hit");
    bench_partial(&mut group, pattern, &input);
    group.finish();
}

fn case_partial_start_byte_miss(c: &mut criterion::Criterion) {
    let pattern = "a+b";
    let input = "x".repeat(500_000);

    let mut group = c.benchmark_group("partial start-byte miss");
    bench_partial(&mut group, pattern, &input);
    group.finish();
}

fn case_partial_start_byte_hit(c: &mut criterion::Criterion) {
    let pattern = "a+b";
    let input = format!("{}aaab", "x".repeat(500_000));

    let mut group = c.benchmark_group("partial start-byte hit");
    bench_partial(&mut group, pattern, &input);
    group.finish();
}

fn case_partial_short_alt(c: &mut criterion::Criterion) {
    let pattern = "ab|cd";
    let input = format!("{}cd", "lorem ipsum ".repeat(20_000));

    let mut group = c.benchmark_group("partial short alt");
    bench_partial(&mut group, pattern, &input);
    group.finish();
}

fn case_partial_digit(c: &mut criterion::Criterion) {
    let pattern = r"\d+";
    let input = format!("{}42", "word ".repeat(40_000));

    let mut group = c.benchmark_group("partial digit");
    bench_partial(&mut group, pattern, &input);
    group.finish();
}

fn case_partial_prefix_fp(c: &mut criterion::Criterion) {
    let pattern = "abc+d";
    // Many "abc" runs that are not followed by more c's and d, then a real match.
    let mut input = "abcX".repeat(5_000);
    input.push_str("abcd");

    let mut group = c.benchmark_group("partial prefix fp");
    bench_partial(&mut group, pattern, &input);
    group.finish();
}

criterion::criterion_group!(
    benches,
    case_1,
    case_2,
    case_long,
    case_meta,
    case_meta_long,
    case_partial_literal,
    case_partial_alt,
    case_partial_long,
    case_partial_required_miss,
    case_partial_required_hit,
    case_partial_start_byte_miss,
    case_partial_start_byte_hit,
    case_partial_short_alt,
    case_partial_digit,
    case_partial_prefix_fp,
);
criterion::criterion_main!(benches);
