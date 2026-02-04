use std::fs;
use std::path::Path;

// Import the Format trait and Yaml formatter by re-using the binary's code
// Since the types module is pub(crate), we test via the public binary interface.
// Instead, we replicate the formatting function for testing.

fn format_yaml(input: &str) -> Result<String, String> {
    // We test by invoking the binary via stdin.
    use std::io::Write;
    use std::process::{Command, Stdio};

    let mut child = Command::new(env!("CARGO_BIN_EXE_metafmt"))
        .args(["--stdin-filetype", "yaml", "-"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("failed to spawn metafmt");

    child
        .stdin
        .as_mut()
        .unwrap()
        .write_all(input.as_bytes())
        .expect("failed to write to stdin");

    let output = child
        .wait_with_output()
        .expect("failed to wait for metafmt");

    if output.status.success() {
        Ok(String::from_utf8(output.stdout).expect("invalid utf8 output"))
    } else {
        Err(String::from_utf8(output.stderr).expect("invalid utf8 stderr"))
    }
}

fn run_fixture(name: &str) {
    let base = Path::new("tests/fixtures/yaml");
    let input_path = base.join(format!("{}.input.yaml", name));
    let expected_path = base.join(format!("{}.expected.yaml", name));

    let input = fs::read_to_string(&input_path)
        .unwrap_or_else(|e| panic!("Failed to read {}: {}", input_path.display(), e));
    let expected = fs::read_to_string(&expected_path)
        .unwrap_or_else(|e| panic!("Failed to read {}: {}", expected_path.display(), e));

    let result =
        format_yaml(&input).unwrap_or_else(|e| panic!("Format failed for {}: {}", name, e));

    assert_eq!(
        result, expected,
        "Formatting mismatch for fixture '{}'.\n\nExpected:\n{}\n\nGot:\n{}",
        name, expected, result
    );

    // Idempotency check: formatting the output again should produce the same result.
    let result2 = format_yaml(&result)
        .unwrap_or_else(|e| panic!("Idempotency format failed for {}: {}", name, e));
    assert_eq!(
        result2, result,
        "Idempotency check failed for fixture '{}'.\n\nFirst format:\n{}\n\nSecond format:\n{}",
        name, result, result2
    );
}

#[test]
fn fixture_simple_mapping() {
    run_fixture("simple_mapping");
}

#[test]
fn fixture_nested_mapping() {
    run_fixture("nested_mapping");
}

#[test]
fn fixture_sequences() {
    run_fixture("sequences");
}

#[test]
fn fixture_sequence_of_mappings() {
    run_fixture("sequence_of_mappings");
}

#[test]
fn fixture_comments() {
    run_fixture("comments");
}

#[test]
fn fixture_block_scalars() {
    run_fixture("block_scalars");
}

#[test]
fn fixture_multi_document() {
    run_fixture("multi_document");
}

#[test]
fn fixture_flow_content() {
    run_fixture("flow_content");
}

#[test]
fn fixture_anchors_aliases() {
    run_fixture("anchors_aliases");
}

#[test]
fn fixture_merge_tags() {
    run_fixture("merge_tags");
}

#[test]
fn fixture_blank_lines() {
    run_fixture("blank_lines");
}

#[test]
fn fixture_quoted_scalars() {
    run_fixture("quoted_scalars");
}

#[test]
fn fixture_complex_nesting() {
    run_fixture("complex_nesting");
}

#[test]
fn fixture_github_actions() {
    run_fixture("github_actions");
}
