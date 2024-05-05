use rustc_version::Version;
use trybuild;

fn version_group() -> String {
    let version = rustc_version::version().expect("Rust Compiler Version");
    eprintln!("VER: {}", version);
    (if version < Version::new(1, 78, 0) {
        "1.77"
    } else {
        "1.78"
    })
    .to_string()
}

#[test]
fn failures() {
    let t = trybuild::TestCases::new();
    eprintln!("tests/uncompilable/{}/*.rs", version_group());
    t.compile_fail(format!("tests/uncompilable/{}/*.rs", version_group()));
}

#[test]
fn success() {
    let t = trybuild::TestCases::new();
    t.pass("tests/compilable/*.rs");
}

#[cfg(feature = "serde")]
#[test]
fn serde_failures() {
    let t = trybuild::TestCases::new();
    t.compile_fail("tests/uncompilable_serde/*.rs");
}
