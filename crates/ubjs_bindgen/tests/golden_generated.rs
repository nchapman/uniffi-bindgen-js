use std::fs;
use std::path::PathBuf;

use ubjs_bindgen::cli::GenerateArgs;

fn run_golden(fixture_name: &str, udl_file: &str, ts_file: &str) {
    run_golden_impl(fixture_name, udl_file, ts_file, ts_file, None);
}

/// Like `run_golden` but passes an explicit config path instead of relying on
/// automatic `uniffi.toml` discovery.  Use this for fixtures whose whole point
/// is to exercise config behaviour so the test is self-documenting.
fn run_golden_with_config(fixture_name: &str, udl_file: &str, ts_file: &str, config_file: &str) {
    let repo = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    let config = repo.join(format!("fixtures/{fixture_name}/src/{config_file}"));
    run_golden_impl(fixture_name, udl_file, ts_file, ts_file, Some(config));
}

/// Like `run_golden_with_config` but the generated file name (from the UDL namespace)
/// differs from the expected golden file name.
fn run_golden_with_config_mapped(
    fixture_name: &str,
    udl_file: &str,
    generated_ts: &str,
    expected_ts: &str,
    config_file: &str,
) {
    let repo = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    let config = repo.join(format!("fixtures/{fixture_name}/src/{config_file}"));
    run_golden_impl(
        fixture_name,
        udl_file,
        generated_ts,
        expected_ts,
        Some(config),
    );
}

fn run_golden_impl(
    fixture_name: &str,
    udl_file: &str,
    generated_ts: &str,
    expected_ts: &str,
    config: Option<PathBuf>,
) {
    let repo = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    let fixture = repo.join(format!("fixtures/{fixture_name}/src/{udl_file}"));
    let expected = repo.join(format!("fixtures/{fixture_name}/expected/{expected_ts}"));
    // Use a subdir keyed by expected filename to avoid collisions when multiple
    // golden tests share the same fixture but use different configs.
    let out_dir = repo.join(format!(
        "target/test-generated-js/{fixture_name}/{}",
        expected_ts.replace('.', "_")
    ));

    let _ = fs::remove_dir_all(&out_dir);

    ubjs_bindgen::js::generate_bindings(&GenerateArgs {
        source: fixture,
        out_dir: out_dir.clone(),
        config,
        library: false,
        crate_name: None,
    })
    .expect("generation should succeed");

    let generated = fs::read_to_string(out_dir.join(generated_ts)).expect("generated file");

    // When UPDATE_GOLDEN is set, overwrite the expected file instead of asserting.
    if std::env::var("UPDATE_GOLDEN").is_ok() {
        fs::create_dir_all(expected.parent().unwrap()).expect("create expected dir");
        fs::write(&expected, &generated).expect("update golden file");
        return;
    }

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

#[test]
fn golden_arithmetic_fixture() {
    run_golden("arithmetic", "arithmetic.udl", "arithmetic.ts");
}

#[test]
fn golden_geometry_fixture() {
    run_golden("geometry", "geometry.udl", "geometry.ts");
}

#[test]
fn golden_counter_fixture() {
    run_golden("counter", "counter.udl", "counter.ts");
}

#[test]
fn golden_rich_errors_fixture() {
    run_golden("rich-errors", "rich_errors.udl", "rich_errors.ts");
}

#[test]
fn golden_rename_exclude_fixture() {
    run_golden_with_config(
        "rename-exclude",
        "rename_exclude.udl",
        "rename_exclude.ts",
        "uniffi.toml",
    );
}

#[test]
fn golden_custom_types_fixture() {
    run_golden("custom-types", "custom_types.udl", "custom_types.ts");
}

#[test]
fn golden_custom_types_lift_lower_fixture() {
    run_golden_with_config_mapped(
        "custom-types",
        "custom_types.udl",
        "custom_types.ts",
        "custom_types_lift.ts",
        "uniffi_lift.toml",
    );
}

#[test]
fn golden_traits_fixture() {
    run_golden("traits", "traits.udl", "traits.ts");
}

#[test]
fn golden_callbacks_fixture() {
    run_golden("callbacks", "callbacks.udl", "callbacks.ts");
}

#[test]
fn golden_docstrings_fixture() {
    run_golden("docstrings", "docstrings.udl", "docstrings.ts");
}

#[test]
fn golden_ext_types_demo_fixture() {
    run_golden("ext-types-demo", "ext_types_demo.udl", "ext_types_demo.ts");
}

#[test]
fn golden_regression_fixture() {
    run_golden("regression", "regression.udl", "regression.ts");
}

#[test]
fn golden_type_zoo_fixture() {
    run_golden("type-zoo", "type_zoo.udl", "type_zoo.ts");
}

#[test]
fn golden_keywords_demo_fixture() {
    run_golden("keywords-demo", "keywords_demo.udl", "keywords_demo.ts");
}

#[test]
fn golden_type_limits_demo_fixture() {
    run_golden(
        "type-limits-demo",
        "type_limits_demo.udl",
        "type_limits_demo.ts",
    );
}

#[test]
fn golden_coverall_demo_fixture() {
    run_golden("coverall-demo", "coverall_demo.udl", "coverall_demo.ts");
}

#[test]
fn golden_error_types_demo_fixture() {
    run_golden(
        "error-types-demo",
        "error_types_demo.udl",
        "error_types_demo.ts",
    );
}

#[test]
fn golden_non_exhaustive_demo_fixture() {
    run_golden(
        "non-exhaustive-demo",
        "non_exhaustive_demo.udl",
        "non_exhaustive_demo.ts",
    );
}
