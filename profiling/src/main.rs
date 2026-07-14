//! Host-side benchmark and profiling harness for the vendored dsntk engine.
//!
//! Measures the same work pgdmn performs per SQL row, minus PostgreSQL itself:
//! JSON -> FeelContext conversion, FEEL parsing/evaluation, DMN invocable
//! evaluation, and FEEL -> JSON conversion. Runs natively on the host in
//! release mode so it can be profiled with a sampling profiler.
//!
//! Usage:
//!   pgdmn-profiling [--filter SUBSTR] [--json PATH] [--samples N]
//!   pgdmn-profiling --profile SCENARIO --seconds N   # hot loop for profilers
//!   pgdmn-profiling --list

use std::hint::black_box;
use std::sync::Arc;
use std::time::{Duration, Instant};

use dsntk_feel::context::FeelContext;
use dsntk_feel::values::Value;
use dsntk_feel::{FeelScope, Name};
use dsntk_feel_evaluator::evaluate;
use dsntk_feel_parser::parse_expression;
use dsntk_model_evaluator::ModelEvaluator;

const CONCAT_DMN: &str = include_str!("../models/concat.dmn");
const RISK_DMN: &str = include_str!("../models/risk.dmn");
const LENDING_DMN: &str = include_str!("../../examples/lending.dmn");
const LOANCOMP_DMN: &str = include_str!("../../examples/loan-comparison.dmn");

// --- Conversion functions mirroring src/convert.rs (the pgrx-free parts). ---
// Duplicated because the extension crate only compiles against pg_config
// inside Docker; keep in sync with src/convert.rs when it changes.

fn json_to_feel(json: &serde_json::Value) -> Value {
    match json {
        serde_json::Value::Null => Value::Null(None),
        serde_json::Value::Bool(b) => Value::Boolean(*b),
        serde_json::Value::Number(n) => {
            if let Some(i) = n.as_i64() {
                Value::Number(dsntk_feel::FeelNumber::from(i))
            } else if let Some(u) = n.as_u64() {
                Value::Number(dsntk_feel::FeelNumber::from(u))
            } else {
                let s = n.to_string();
                match s.parse::<dsntk_feel::FeelNumber>() {
                    Ok(num) => Value::Number(num),
                    Err(_) => Value::Null(Some(format!("cannot convert number: {s}"))),
                }
            }
        }
        serde_json::Value::String(s) => Value::String(s.clone()),
        serde_json::Value::Array(arr) => Value::List(arr.iter().map(json_to_feel).collect()),
        serde_json::Value::Object(map) => {
            let mut ctx = FeelContext::new();
            for (key, val) in map {
                ctx.insert(Name::from(key.as_str()), json_to_feel(val));
            }
            Value::Context(ctx)
        }
    }
}

fn feel_number_to_json(n: &dsntk_feel::FeelNumber) -> serde_json::Value {
    if n.is_integer() {
        if let Ok(i) = i64::try_from(n) {
            return serde_json::Value::Number(serde_json::Number::from(i));
        }
    }
    let s = n.to_string();
    if let Ok(i) = s.parse::<i64>() {
        serde_json::Value::Number(serde_json::Number::from(i))
    } else if let Ok(f) = s.parse::<f64>() {
        serde_json::json!(f)
    } else {
        serde_json::Value::String(s)
    }
}

fn feel_to_json(value: &Value) -> serde_json::Value {
    match value {
        Value::Null(_) => serde_json::Value::Null,
        Value::Boolean(b) => serde_json::Value::Bool(*b),
        Value::Number(n) => feel_number_to_json(n),
        Value::String(s) => serde_json::Value::String(s.clone()),
        Value::List(items) => serde_json::Value::Array(items.iter().map(feel_to_json).collect()),
        Value::Context(ctx) => {
            let mut map = serde_json::Map::new();
            for (name, val) in ctx.iter() {
                map.insert(name.to_string(), feel_to_json(val));
            }
            serde_json::Value::Object(map)
        }
        Value::Date(d) => serde_json::Value::String(d.to_string()),
        Value::Time(t) => serde_json::Value::String(t.to_string()),
        Value::DateTime(dt) => serde_json::Value::String(dt.to_string()),
        Value::DaysAndTimeDuration(d) => serde_json::Value::String(d.to_string()),
        Value::YearsAndMonthsDuration(d) => serde_json::Value::String(d.to_string()),
        _ => serde_json::Value::String(value.to_string()),
    }
}

fn json_to_context(json: &serde_json::Value) -> FeelContext {
    if let serde_json::Value::Object(map) = json {
        let mut ctx = FeelContext::new();
        for (key, val) in map {
            ctx.insert(Name::from(key.as_str()), json_to_feel(val));
        }
        ctx
    } else {
        FeelContext::new()
    }
}

// --- Input generation ------------------------------------------------------

const EMPLOYMENT: [&str; 4] = ["employed", "self-employed", "unemployed", "retired"];

/// Deterministic pseudo-varied risk inputs (mirrors the SQL bench generator).
fn risk_inputs_json(n: usize) -> Vec<serde_json::Value> {
    (0..n)
        .map(|i| {
            let f = i % 100;
            let l = (i * 7) % 100;
            serde_json::json!({
                "age": 18 + ((f * 7 + l * 3) % 50),
                "income": 25000 + ((f * 13 + l * 17) % 100) * 1000,
                "credit_score": 500 + ((f * 11 + l * 7) % 350),
                "employment_status": EMPLOYMENT[(f + l) % 4],
                "years_employed": (f * 3 + l * 5) % 20,
            })
        })
        .collect()
}

fn concat_inputs_json(n: usize) -> Vec<serde_json::Value> {
    (0..n)
        .map(|i| {
            serde_json::json!({
                "first_name": format!("First_{:03}", i % 100),
                "last_name": format!("Last_{:03}", (i * 7) % 100),
            })
        })
        .collect()
}

fn lending_input_json() -> serde_json::Value {
    serde_json::json!({
        "ApplicantData": {
            "Age": 51, "MaritalStatus": "M", "EmploymentStatus": "EMPLOYED",
            "ExistingCustomer": false,
            "Monthly": {"Income": 10000, "Expenses": 3000, "Repayments": 0}
        },
        "RequestedProduct": {
            "ProductType": "STANDARD LOAN", "Amount": 100000, "Rate": 0.06, "Term": 36
        },
        "BureauData": {"CreditScore": 700, "Bankrupt": false}
    })
}

fn build_evaluator(xml: &str) -> Arc<ModelEvaluator> {
    let definitions = dsntk_model::parse(xml).expect("model parse failed");
    ModelEvaluator::new(&[definitions]).expect("evaluator build failed")
}

// --- Measurement core -------------------------------------------------------

struct BenchResult {
    name: &'static str,
    median_ns: f64,
    mean_ns: f64,
    p10_ns: f64,
    p90_ns: f64,
    iters_per_sample: u64,
    samples: usize,
}

const TARGET_SAMPLE: Duration = Duration::from_millis(2);
const WARMUP: Duration = Duration::from_millis(200);

fn bench(name: &'static str, samples: usize, mut f: impl FnMut()) -> BenchResult {
    // Calibrate iterations per sample
    let start = Instant::now();
    f();
    let once = start.elapsed().max(Duration::from_nanos(20));
    let iters = (TARGET_SAMPLE.as_nanos() / once.as_nanos()).clamp(1, 10_000_000) as u64;

    // Warmup
    let warm_start = Instant::now();
    while warm_start.elapsed() < WARMUP {
        for _ in 0..iters.min(1000) {
            f();
        }
    }

    // Measure
    let mut per_op: Vec<f64> = (0..samples)
        .map(|_| {
            let t = Instant::now();
            for _ in 0..iters {
                f();
            }
            t.elapsed().as_nanos() as f64 / iters as f64
        })
        .collect();
    per_op.sort_by(|a, b| a.total_cmp(b));

    let n = per_op.len();
    BenchResult {
        name,
        median_ns: per_op[n / 2],
        mean_ns: per_op.iter().sum::<f64>() / n as f64,
        p10_ns: per_op[n / 10],
        p90_ns: per_op[(n * 9) / 10],
        iters_per_sample: iters,
        samples: n,
    }
}

// --- Scenarios ---------------------------------------------------------------

struct Scenario {
    name: &'static str,
    desc: &'static str,
    run: Box<dyn Fn(usize) -> BenchResult>,
    hot: Box<dyn Fn(Duration)>,
}

#[expect(clippy::too_many_lines)]
fn scenarios() -> Vec<Scenario> {
    let mut list: Vec<Scenario> = Vec::new();

    // Helper to register a scenario from a closure factory. Each factory call
    // builds fresh state; the closure runs one logical operation.
    macro_rules! scenario {
        ($name:literal, $desc:literal, $setup:expr) => {{
            list.push(Scenario {
                name: $name,
                desc: $desc,
                run: Box::new(move |samples| {
                    let mut op = $setup();
                    bench($name, samples, move || op())
                }),
                hot: Box::new(move |dur| {
                    let mut op = $setup();
                    let start = Instant::now();
                    while start.elapsed() < dur {
                        for _ in 0..1000 {
                            op();
                        }
                    }
                }),
            });
        }};
    }

    // -- Model evaluation: engine only (prebuilt contexts) --
    scenario!(
        "model_concat_eval",
        "ConcatDecision/FullName: engine-only eval, prebuilt FeelContext",
        || {
            let evaluator = build_evaluator(CONCAT_DMN);
            let ctxs: Vec<FeelContext> = concat_inputs_json(256)
                .iter()
                .map(json_to_context)
                .collect();
            let result = evaluator.evaluate_invocable(
                "https://example.org/concat",
                "ConcatDecision",
                "FullName",
                &ctxs[0],
            );
            assert!(matches!(result, Value::String(_)), "unexpected: {result:?}");
            let mut i = 0usize;
            move || {
                let ctx = &ctxs[i % ctxs.len()];
                i += 1;
                black_box(evaluator.evaluate_invocable(
                    "https://example.org/concat",
                    "ConcatDecision",
                    "FullName",
                    ctx,
                ));
            }
        }
    );

    scenario!(
        "model_risk_eval",
        "RiskAssessment/RiskScore: engine-only eval (decision table + literal chain)",
        || {
            let evaluator = build_evaluator(RISK_DMN);
            let ctxs: Vec<FeelContext> =
                risk_inputs_json(256).iter().map(json_to_context).collect();
            let result = evaluator.evaluate_invocable(
                "https://example.org/risk",
                "RiskAssessment",
                "RiskScore",
                &ctxs[0],
            );
            assert!(matches!(result, Value::Number(_)), "unexpected: {result:?}");
            let mut i = 0usize;
            move || {
                let ctx = &ctxs[i % ctxs.len()];
                i += 1;
                black_box(evaluator.evaluate_invocable(
                    "https://example.org/risk",
                    "RiskAssessment",
                    "RiskScore",
                    ctx,
                ));
            }
        }
    );

    scenario!(
        "model_lending_eval",
        "Lending/Strategy: deep DRG with BKMs and nested contexts",
        || {
            let evaluator = build_evaluator(LENDING_DMN);
            let ctx = json_to_context(&lending_input_json());
            let result = evaluator.evaluate_invocable(
                "http://www.trisotech.com/definitions/_4e0f0b70-d31c-471c-bd52-5ca709ed362b",
                "0004-lending",
                "Strategy",
                &ctx,
            );
            assert!(matches!(result, Value::String(_)), "unexpected: {result:?}");
            move || {
                black_box(evaluator.evaluate_invocable(
                    "http://www.trisotech.com/definitions/_4e0f0b70-d31c-471c-bd52-5ca709ed362b",
                    "0004-lending",
                    "Strategy",
                    &ctx,
                ));
            }
        }
    );

    // -- Full pgdmn-equivalent path (minus PostgreSQL) --
    scenario!(
        "e2e_risk_json",
        "risk model: serde_json parse -> json_to_context -> eval -> feel_to_json -> to_string",
        || {
            let evaluator = build_evaluator(RISK_DMN);
            let inputs: Vec<String> = risk_inputs_json(256)
                .iter()
                .map(|v| serde_json::to_string(v).expect("serialize"))
                .collect();
            let mut i = 0usize;
            move || {
                let raw = &inputs[i % inputs.len()];
                i += 1;
                let json: serde_json::Value = serde_json::from_str(raw).expect("parse");
                let ctx = json_to_context(&json);
                let result = evaluator.evaluate_invocable(
                    "https://example.org/risk",
                    "RiskAssessment",
                    "RiskScore",
                    &ctx,
                );
                let out = serde_json::to_string(&feel_to_json(&result)).expect("serialize");
                black_box(out);
            }
        }
    );

    // -- Conversion layers in isolation --
    scenario!(
        "convert_json_to_context",
        "risk inputs: serde_json::Value -> FeelContext",
        || {
            let inputs = risk_inputs_json(256);
            let mut i = 0usize;
            move || {
                let json = &inputs[i % inputs.len()];
                i += 1;
                black_box(json_to_context(json));
            }
        }
    );

    scenario!(
        "convert_feel_to_json",
        "risk result Value -> serde_json::Value",
        || {
            let evaluator = build_evaluator(RISK_DMN);
            let results: Vec<Value> = risk_inputs_json(64)
                .iter()
                .map(|j| {
                    evaluator.evaluate_invocable(
                        "https://example.org/risk",
                        "RiskAssessment",
                        "RiskScore",
                        &json_to_context(j),
                    )
                })
                .collect();
            let mut i = 0usize;
            move || {
                let v = &results[i % results.len()];
                i += 1;
                black_box(feel_to_json(v));
            }
        }
    );

    // -- FEEL parsing and evaluation --
    scenario!(
        "feel_parse_short",
        "parse 'x + y' against a 2-name scope",
        || {
            let scope = FeelScope::default();
            let mut ctx = FeelContext::new();
            ctx.set_entry(&Name::from("x"), Value::Number(21.into()));
            ctx.set_entry(&Name::from("y"), Value::Number(2.into()));
            scope.push(ctx);
            move || {
                black_box(parse_expression(&scope, "x + y", false).expect("parse"));
            }
        }
    );

    scenario!(
        "feel_eval_short",
        "evaluate prebuilt AST for 'x + y'",
        || {
            let scope = FeelScope::default();
            let mut ctx = FeelContext::new();
            ctx.set_entry(&Name::from("x"), Value::Number(21.into()));
            ctx.set_entry(&Name::from("y"), Value::Number(2.into()));
            scope.push(ctx);
            let node = parse_expression(&scope, "x + y", false).expect("parse");
            move || {
                black_box(evaluate(&scope, &node));
            }
        }
    );

    scenario!(
        "feel_parse_eval_short",
        "parse + evaluate 'x + y' (pgdmn feel_eval per-call behavior)",
        || {
            let scope = FeelScope::default();
            let mut ctx = FeelContext::new();
            ctx.set_entry(&Name::from("x"), Value::Number(21.into()));
            ctx.set_entry(&Name::from("y"), Value::Number(2.into()));
            scope.push(ctx);
            move || {
                let node = parse_expression(&scope, "x + y", false).expect("parse");
                black_box(evaluate(&scope, &node));
            }
        }
    );

    // -- Construct-specific scenarios: iteration, filters, regex BIFs --
    scenario!(
        "model_loancomp_eval",
        "LoanComparison/RankedProducts: iteration + sorting over 10 products, BKMs",
        || {
            let evaluator = build_evaluator(LOANCOMP_DMN);
            let ctx = json_to_context(&serde_json::json!({"RequestedAmt": 330000}));
            let result = evaluator.evaluate_invocable(
                "http://www.trisotech.com/definitions/_56c7d4a5-e6db-4bba-ac5f-dc082a16f719",
                "0014-loan-comparison",
                "RankedProducts",
                &ctx,
            );
            assert!(
                matches!(result, Value::Context(_)),
                "unexpected: {result:?}"
            );
            move || {
                black_box(evaluator.evaluate_invocable(
                    "http://www.trisotech.com/definitions/_56c7d4a5-e6db-4bba-ac5f-dc082a16f719",
                    "0014-loan-comparison",
                    "RankedProducts",
                    &ctx,
                ));
            }
        }
    );

    scenario!(
        "feel_for_100",
        "prepared eval: for-expression over a 100-element list",
        || {
            let scope = FeelScope::default();
            let mut ctx = FeelContext::new();
            let items: Vec<Value> = (0..100).map(|i| Value::Number(i.into())).collect();
            ctx.set_entry(&Name::from("items"), Value::List(items));
            scope.push(ctx);
            let node =
                parse_expression(&scope, "for i in items return i * i", false).expect("parse");
            let result = evaluate(&scope, &node);
            assert!(matches!(&result, Value::List(l) if l.len() == 100));
            move || {
                black_box(evaluate(&scope, &node));
            }
        }
    );

    scenario!(
        "feel_filter_100",
        "prepared eval: filter expression over a 100-element list",
        || {
            let scope = FeelScope::default();
            let mut ctx = FeelContext::new();
            let items: Vec<Value> = (0..100).map(|i| Value::Number(i.into())).collect();
            ctx.set_entry(&Name::from("items"), Value::List(items));
            scope.push(ctx);
            let node = parse_expression(&scope, "items[item > 50]", false).expect("parse");
            let result = evaluate(&scope, &node);
            assert!(matches!(&result, Value::List(l) if l.len() == 49));
            move || {
                black_box(evaluate(&scope, &node));
            }
        }
    );

    scenario!(
        "feel_replace_regex",
        "prepared eval: replace() regex BIF on a 40-char string",
        || {
            let scope = FeelScope::default();
            let mut ctx = FeelContext::new();
            ctx.set_entry(
                &Name::from("text"),
                Value::String("the quick brown fox jumps over lazy dogs".to_string()),
            );
            scope.push(ctx);
            let node = parse_expression(&scope, r#"replace(text, "[aeiou]", "_")"#, false)
                .expect("parse");
            let result = evaluate(&scope, &node);
            assert!(matches!(&result, Value::String(s) if s.contains("q__ck")));
            move || {
                black_box(evaluate(&scope, &node));
            }
        }
    );

    // -- Micro: FeelNumber --
    scenario!("num_parse", "parse '123.45' into FeelNumber", || {
        move || {
            black_box("123.45".parse::<dsntk_feel::FeelNumber>().expect("parse"));
        }
    });

    scenario!("num_add", "FeelNumber addition", || {
        let a: dsntk_feel::FeelNumber = "123.45".parse().expect("parse");
        let b: dsntk_feel::FeelNumber = "67.89".parse().expect("parse");
        move || {
            black_box(black_box(a) + black_box(b));
        }
    });

    scenario!("num_mul", "FeelNumber multiplication", || {
        let a: dsntk_feel::FeelNumber = "123.45".parse().expect("parse");
        let b: dsntk_feel::FeelNumber = "67.89".parse().expect("parse");
        move || {
            black_box(black_box(a) * black_box(b));
        }
    });

    scenario!("num_to_string", "FeelNumber -> String", || {
        let a: dsntk_feel::FeelNumber = "123.45".parse().expect("parse");
        move || {
            black_box(black_box(a).to_string());
        }
    });

    // -- Micro: names and contexts --
    scenario!("name_from", "Name::from(\"employment_status\")", || {
        move || {
            black_box(Name::from(black_box("employment_status")));
        }
    });

    scenario!(
        "ctx_build_5",
        "build 5-entry FeelContext (prebuilt Names)",
        || {
            let names: Vec<Name> = [
                "age",
                "income",
                "credit_score",
                "employment_status",
                "years_employed",
            ]
            .iter()
            .map(|s| Name::from(*s))
            .collect();
            move || {
                let mut ctx = FeelContext::new();
                for (i, name) in names.iter().enumerate() {
                    ctx.set_entry(name, Value::Number((i as i64).into()));
                }
                black_box(ctx);
            }
        }
    );

    list
}

// --- CLI ---------------------------------------------------------------------

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let mut filter: Option<String> = None;
    let mut json_path: Option<String> = None;
    let mut samples: usize = 30;
    let mut profile: Option<String> = None;
    let mut seconds: u64 = 10;
    let mut list_only = false;

    let mut it = args.iter();
    while let Some(arg) = it.next() {
        match arg.as_str() {
            "--filter" => filter = it.next().cloned(),
            "--json" => json_path = it.next().cloned(),
            "--samples" => {
                samples = it
                    .next()
                    .and_then(|s| s.parse().ok())
                    .expect("--samples requires a number");
            }
            "--profile" => profile = it.next().cloned(),
            "--seconds" => {
                seconds = it
                    .next()
                    .and_then(|s| s.parse().ok())
                    .expect("--seconds requires a number");
            }
            "--list" => list_only = true,
            other => {
                eprintln!("unknown argument: {other}");
                std::process::exit(2);
            }
        }
    }

    let all = scenarios();

    if list_only {
        for s in &all {
            println!("{:28} {}", s.name, s.desc);
        }
        return;
    }

    if let Some(name) = profile {
        let s = all
            .iter()
            .find(|s| s.name == name)
            .unwrap_or_else(|| panic!("no scenario named {name}"));
        eprintln!(
            "profiling '{}' hot loop for {seconds}s (pid {})",
            s.name,
            std::process::id()
        );
        (s.hot)(Duration::from_secs(seconds));
        return;
    }

    let selected: Vec<&Scenario> = all
        .iter()
        .filter(|s| filter.as_ref().is_none_or(|f| s.name.contains(f.as_str())))
        .collect();

    println!(
        "{:28} {:>12} {:>12} {:>12} {:>12} {:>10}",
        "scenario", "median ns", "mean ns", "p10 ns", "p90 ns", "iters"
    );
    let mut results = Vec::new();
    for s in selected {
        let r = (s.run)(samples);
        println!(
            "{:28} {:>12.1} {:>12.1} {:>12.1} {:>12.1} {:>10}",
            r.name, r.median_ns, r.mean_ns, r.p10_ns, r.p90_ns, r.iters_per_sample
        );
        results.push(r);
    }

    if let Some(path) = json_path {
        let json = serde_json::json!({
            "results": results.iter().map(|r| serde_json::json!({
                "name": r.name,
                "median_ns": r.median_ns,
                "mean_ns": r.mean_ns,
                "p10_ns": r.p10_ns,
                "p90_ns": r.p90_ns,
                "iters_per_sample": r.iters_per_sample,
                "samples": r.samples,
            })).collect::<Vec<_>>(),
        });
        std::fs::write(
            &path,
            serde_json::to_string_pretty(&json).expect("serialize"),
        )
        .expect("write json");
        eprintln!("wrote {path}");
    }
}
