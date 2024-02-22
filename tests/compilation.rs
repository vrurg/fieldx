use trybuild;

#[test]
fn failures() {
    let t = trybuild::TestCases::new();
    t.compile_fail("tests/uncompilable/*.rs");
}

// #[test]
// fn successes() {
//     let t = trybuild::TestCases::new();
//     t.pass("tests/nonsync.rs");
//     t.pass("tests/sync.rs");
// }
