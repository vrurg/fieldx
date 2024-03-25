use trybuild;

#[test]
fn failures() {
    let t = trybuild::TestCases::new();
    t.compile_fail("tests/uncompilable/*.rs");
}
