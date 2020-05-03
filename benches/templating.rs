use criterion::{black_box, criterion_group, criterion_main, Criterion};
use rustrest::keyvalue::KeyValue;
use rustrest::templating;

fn templating_benchmark(c: &mut Criterion) {
    let base = std::fs::read_to_string("benches/template_base.txt").unwrap();
    let vars = vec![
        {
            KeyValue {
                name: "var1".to_string(),
                value: "value1".to_string(),
            }
        },
        {
            KeyValue {
                name: "var2".to_string(),
                value: "value2".to_string(),
            }
        },
        {
            KeyValue {
                name: "var3".to_string(),
                value: "value3".to_string(),
            }
        },
        {
            KeyValue {
                name: "var4".to_string(),
                value: "value4".to_string(),
            }
        },
        {
            KeyValue {
                name: "var5".to_string(),
                value: "value5".to_string(),
            }
        },
        {
            KeyValue {
                name: "var6".to_string(),
                value: "value6".to_string(),
            }
        },
        {
            KeyValue {
                name: "var7".to_string(),
                value: "value7".to_string(),
            }
        },
    ];

    c.bench_function("template", move |b| {
        b.iter_batched(
            || base.clone(),
            |data| templating::substitute(black_box(data), black_box(&vars)),
            criterion::BatchSize::SmallInput,
        );
    });
}

criterion_group!(benches, templating_benchmark);
criterion_main!(benches);
