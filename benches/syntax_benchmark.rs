use criterion::{black_box, criterion_group, criterion_main, Criterion};
use std::collections::HashSet;
use vim_editor::config::Theme;
use vim_editor::syntax::{count_leading_spaces, create_indent_spans, highlight_syntax_with_state, tokenize_with_state, BracketState};

fn benchmark_highlight_syntax(c: &mut Criterion) {
    let test_lines = vec![
        "fn main() {",
        "    let mut map = HashMap::new();",
        "    for (index, &value) in data.iter().enumerate() {",
        "        if value > 0 {",
        "            map.insert(format!(\"item_{}\", index), value);",
        "        } else {",
        "            eprintln!(\"Warning: negative value at index {}\", index);",
        "        }",
        "    }",
        "    Ok(map)",
        "}",
    ];

    c.bench_function("highlight_syntax_simple", |b| {
        let theme = Theme::default();
        let unmatched_brackets = HashSet::new();
        b.iter(|| {
            for (i, line) in test_lines.iter().enumerate() {
                black_box(highlight_syntax_with_state(black_box(line), i, 4, &mut BracketState::new(), black_box(&theme), &unmatched_brackets));
            }
        })
    });

    // 長い行のテスト
    let long_line = "    ".repeat(10) + &"let very_long_variable_name_that_should_test_performance = some_function_call_with_many_parameters(param1, param2, param3, param4, param5);";
    
    c.bench_function("highlight_syntax_long_line", |b| {
        let theme = Theme::default();
        let unmatched_brackets = HashSet::new();
        b.iter(|| {
            black_box(highlight_syntax_with_state(black_box(&long_line), 0, 4, &mut BracketState::new(), black_box(&theme), &unmatched_brackets));
        })
    });

    // 深いインデントのテスト
    let deep_indent_line = "    ".repeat(20) + "println!(\"deeply nested\");";
    
    c.bench_function("highlight_syntax_deep_indent", |b| {
        let theme = Theme::default();
        let unmatched_brackets = HashSet::new();
        b.iter(|| {
            black_box(highlight_syntax_with_state(black_box(&deep_indent_line), 0, 4, &mut BracketState::new(), black_box(&theme), &unmatched_brackets));
        })
    });
}

fn benchmark_tokenize(c: &mut Criterion) {
    let complex_code = "fn process_data(data: &[i32]) -> Result<HashMap<String, i32>, Box<dyn std::error::Error>> {";
    
    c.bench_function("tokenize_complex", |b| {
        let unmatched_brackets = HashSet::new();
        b.iter(|| {
            black_box(tokenize_with_state(black_box(complex_code), 0, 0, &mut BracketState::new(), &unmatched_brackets));
        })
    });

    let simple_code = "let x = 42;";
    
    c.bench_function("tokenize_simple", |b| {
        let unmatched_brackets = HashSet::new();
        b.iter(|| {
            black_box(tokenize_with_state(black_box(simple_code), 0, 0, &mut BracketState::new(), &unmatched_brackets));
        })
    });

    // 文字列が多い行
    let string_heavy = "println!(\"Hello, {}! Welcome to {} version {}\", name, app_name, version);";
    
    c.bench_function("tokenize_string_heavy", |b| {
        let unmatched_brackets = HashSet::new();
        b.iter(|| {
            black_box(tokenize_with_state(black_box(string_heavy), 0, 0, &mut BracketState::new(), &unmatched_brackets));
        })
    });
}

fn benchmark_indent_operations(c: &mut Criterion) {
    let test_lines = vec![
        "",
        "hello",
        "    hello",
        "        hello",
        "            hello",
        "                hello",
        "                    hello",
        "                        hello",
    ];

    c.bench_function("count_leading_spaces", |b| {
        b.iter(|| {
            for line in &test_lines {
                black_box(count_leading_spaces(black_box(line)));
            }
        })
    });

    c.bench_function("create_indent_spans", |b| {
        let theme = Theme::default();
        b.iter(|| {
            for line in &test_lines {
                black_box(create_indent_spans(black_box(line), black_box(4), black_box(&theme)));
            }
        })
    });
}

fn benchmark_large_file_simulation(c: &mut Criterion) {
    // 大きなファイルをシミュレート（1000行）
    let large_file_lines: Vec<String> = (0..1000).map(|i| {
        let indent = "    ".repeat(i % 8);
        format!("{}fn function_{}() -> Result<(), Error> {{", indent, i)
    }).collect();

    c.bench_function("highlight_large_file", |b| {
        let theme = Theme::default();
        let unmatched_brackets = HashSet::new();
        b.iter(|| {
            for (i, line) in large_file_lines.iter().enumerate() {
                black_box(highlight_syntax_with_state(black_box(line), i, 4, &mut BracketState::new(), black_box(&theme), &unmatched_brackets));
            }
        })
    });
}

fn benchmark_memory_intensive(c: &mut Criterion) {
    // メモリ集約的な操作のテスト
    let lines_with_many_tokens: Vec<String> = (0..100).map(|i| {
        format!("let var_{} = function_{}(param1, param2, param3) + another_function_{}();", i, i, i)
    }).collect();

    c.bench_function("highlight_many_tokens", |b| {
        let theme = Theme::default();
        let unmatched_brackets = HashSet::new();
        b.iter(|| {
            for (i, line) in lines_with_many_tokens.iter().enumerate() {
                black_box(highlight_syntax_with_state(black_box(line), i, 4, &mut BracketState::new(), black_box(&theme), &unmatched_brackets));
            }
        })
    });
}

criterion_group!(
    benches,
    benchmark_highlight_syntax,
    benchmark_tokenize,
    benchmark_indent_operations,
    benchmark_large_file_simulation,
    benchmark_memory_intensive
);
criterion_main!(benches);