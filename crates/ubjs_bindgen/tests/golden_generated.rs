use std::fs;
use std::path::PathBuf;

use ubjs_bindgen::cli::GenerateArgs;

fn run_golden(fixture_name: &str, udl_file: &str, ts_file: &str) {
    let repo = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    let fixture = repo.join(format!("fixtures/{fixture_name}/src/{udl_file}"));
    let expected = repo.join(format!("fixtures/{fixture_name}/expected/{ts_file}"));
    let out_dir = repo.join(format!("target/test-generated-js/{fixture_name}"));

    let _ = fs::remove_dir_all(&out_dir);

    ubjs_bindgen::js::generate_bindings(&GenerateArgs {
        source: fixture,
        out_dir: out_dir.clone(),
        config: None,
    })
    .expect("generation should succeed");

    let generated = fs::read_to_string(out_dir.join(ts_file)).expect("generated file");
    let expected = fs::read_to_string(expected).expect("expected file");
    assert_eq!(generated, expected);
}

#[test]
fn golden_simple_fixture() {
    run_golden("simple", "simple.udl", "simple.ts");
}

#[test]
fn golden_simple_fns_fixture() {
    run_golden("simple-fns", "simple_fns.udl", "simple_fns.ts");
}
