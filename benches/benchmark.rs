fn dfa_1(c: &mut criterion::Criterion) {
    let target_regex = "(p(erl|ython|hp)|ruby)";
    let targets = vec!["perl", "python", "ruby", "rust"];

    let regex = rustegex::Engine::new(target_regex, "dfa").unwrap();

    c.bench_function("DFA 1", |b| {
        b.iter(|| {
            for target in &targets {
                regex.is_match(target);
            }
        })
    });
}

fn dfa_2(c: &mut criterion::Criterion) {
    let target_regex = "ab(cd|)ef|g*|h+";
    let targets = vec!["abcdef", "abef", "abefg", "abefgh", "", "ggggg", "hhhh"];

    let regex = rustegex::Engine::new(target_regex, "dfa").unwrap();

    c.bench_function("DFA 2", |b| {
        b.iter(|| {
            for target in &targets {
                regex.is_match(target);
            }
        })
    });
}

fn dfa_long(c: &mut criterion::Criterion) {
    let target_regex = "a+b";

    let regex = rustegex::Engine::new(target_regex, "dfa").unwrap();

    c.bench_function("DFA long", |b| {
        b.iter(|| {
            regex.is_match("a".repeat(1000000).as_str());
        })
    });
}

fn vm_1(c: &mut criterion::Criterion) {
    let target_regex = "(p(erl|ython|hp)|ruby)";
    let targets = vec!["perl", "python", "ruby", "rust"];

    let regex = rustegex::Engine::new(target_regex, "vm").unwrap();

    c.bench_function("VM 1", |b| {
        b.iter(|| {
            for target in &targets {
                regex.is_match(target);
            }
        })
    });
}

fn vm_2(c: &mut criterion::Criterion) {
    let target_regex = "ab(cd|)ef|g*|h+";
    let targets = vec!["abcdef", "abef", "abefg", "abefgh", "", "ggggg", "hhhh"];

    let regex = rustegex::Engine::new(target_regex, "vm").unwrap();

    c.bench_function("VM 2", |b| {
        b.iter(|| {
            for target in &targets {
                regex.is_match(target);
            }
        })
    });
}

fn vm_long(c: &mut criterion::Criterion) {
    let target_regex = "a+b";

    let regex = rustegex::Engine::new(target_regex, "vm").unwrap();

    c.bench_function("VM long", |b| {
        b.iter(|| {
            regex.is_match("a".repeat(1000000).as_str());
        })
    });
}

fn derivative_1(c: &mut criterion::Criterion) {
    let target_regex = "(p(erl|ython|hp)|ruby)";
    let targets = vec!["perl", "python", "ruby", "rust"];

    let regex = rustegex::Engine::new(target_regex, "derivative").unwrap();

    c.bench_function("Derivative 1", |b| {
        b.iter(|| {
            for target in &targets {
                regex.is_match(target);
            }
        })
    });
}

fn derivative_2(c: &mut criterion::Criterion) {
    let target_regex = "ab(cd|)ef|g*|h+";
    let targets = vec!["abcdef", "abef", "abefg", "abefgh", "", "ggggg", "hhhh"];

    let regex = rustegex::Engine::new(target_regex, "derivative").unwrap();

    c.bench_function("Derivative 2", |b| {
        b.iter(|| {
            for target in &targets {
                regex.is_match(target);
            }
        })
    });
}

fn derivative_long(c: &mut criterion::Criterion) {
    let target_regex = "a+b";

    let regex = rustegex::Engine::new(target_regex, "derivative").unwrap();

    c.bench_function("Derivative long", |b| {
        b.iter(|| {
            regex.is_match("a".repeat(1000000).as_str());
        })
    });
}

criterion::criterion_group!(
    benches,
    dfa_1,
    dfa_2,
    dfa_long,
    vm_1,
    vm_2,
    vm_long,
    derivative_1,
    derivative_2,
    derivative_long
);
criterion::criterion_main!(benches);
