use rustc_version::Version;
use std::{fs, path::PathBuf};
use trybuild;

struct UncompEnv {
    stderrs: Vec<PathBuf>,
}

impl UncompEnv {
    fn new() -> Self {
        let mut stderrs: Vec<PathBuf> = vec![];
        // let test_path = PathBuf::from(format!("{}/", env!("CARGO_MANIFEST_DIR")));
        let base_dir = env!("CARGO_MANIFEST_DIR");
        stderrs.append(&mut Self::collect_stderrs(format!("{}/tests/uncompilable", base_dir)));
        #[cfg(feature = "serde")]
        stderrs.append(&mut Self::collect_stderrs(format!(
            "{}/tests/uncompilable_serde",
            base_dir
        )));

        Self { stderrs }
    }

    fn collect_stderrs(from: String) -> Vec<PathBuf> {
        let dest_dir = PathBuf::from(&from);
        let from_dir = PathBuf::from(format!("{}/{}", from, Self::version_group()));

        if !from_dir.exists() {
            return vec![];
        }

        if !from_dir.is_dir() {
            panic!("'{}' is not a directory", from_dir.display());
        }

        let mut stderrs: Vec<PathBuf> = vec![];
        for entry in std::fs::read_dir(&from_dir).expect(&format!("Failed to read '{}' directory", from_dir.display()))
        {
            let fname = entry
                .expect(&format!(
                    "Failed to fetch an entry from directory '{}'",
                    from_dir.display()
                ))
                .file_name();
            let Ok(fname) = fname.clone().into_string()
            else {
                panic!("Badly formed file name '{}'", fname.to_string_lossy())
            };

            if fname.ends_with(".stderr") {
                let dest_stderr = dest_dir.join(&fname);
                let src_stderr = from_dir.join(&fname);
                eprintln!("+ {} -> {}", src_stderr.display(), dest_stderr.display());
                fs::copy(&src_stderr, &dest_stderr).expect(&format!(
                    "Failed to copy '{}' to '{}'",
                    src_stderr.display(),
                    dest_stderr.display()
                ));
                stderrs.push(dest_stderr);
            }
        }

        stderrs
    }

    fn version_group() -> String {
        let version = rustc_version::version().expect("Rust Compiler Version");
        let version = Version::new(version.major, version.minor, version.patch);
        eprintln!("Rust Compiler Version: {}", version);
        (if version < Version::new(1, 78, 0) {
            "1.77"
        }
        else if version == Version::new(1, 78, 0) {
            "1.78"
        }
        else if version == Version::new(1, 79, 0) {
            "1.79"
        }
        else {
            "1.80"
        })
        .to_string()
    }
}

impl Drop for UncompEnv {
    fn drop(&mut self) {
        let mut with_failures = false;
        for stderr in self.stderrs.iter() {
            eprintln!("- {}", stderr.display());
            if fs::remove_file(stderr).is_err() {
                eprintln!("!!! Failed to remove '{}'", stderr.display());
                with_failures = true;
            }
        }
        if with_failures {
            panic!("Failed to remove some stderr files");
        }
    }
}

#[test]
fn failures() {
    let _test_env = UncompEnv::new();
    let t = trybuild::TestCases::new();
    t.compile_fail(format!("tests/uncompilable/*.rs"));
    #[cfg(feature = "serde")]
    t.compile_fail("tests/uncompilable_serde/*.rs");
}

#[test]
fn success() {
    let t = trybuild::TestCases::new();
    t.pass("tests/compilable/*.rs");
}
