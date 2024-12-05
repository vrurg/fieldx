use cargo_toolchain::get_active_toolchain;
use rustc_version::Version;
use std::{
    env,
    fs::{self, DirEntry},
    io,
    path::PathBuf,
};
use trybuild;

struct UncompEnv {
    // .0 is a path of .stderr under the version subdir, .1 is the one used for testing
    stderrs:     Vec<(PathBuf, PathBuf)>,
    base_dir:    PathBuf,
    outputs_dir: PathBuf,
}

impl UncompEnv {
    fn new(subdir: &str) -> Self {
        let manifest_dir = env!("CARGO_MANIFEST_DIR").to_string();
        let base_dir = PathBuf::from(format!("{}/tests/{}", manifest_dir, subdir));
        let outputs_dir = Self::outputs_dir(&base_dir).unwrap();
        // stderrs.append(&mut Self::collect_stderrs(&outputs_dir));
        let mut me = Self {
            stderrs: Vec::new(),
            base_dir,
            outputs_dir,
        };
        me.collect_stderrs().unwrap();
        me
    }

    fn stringify_fname(entry: Result<DirEntry, io::Error>, from_dir: &PathBuf) -> String {
        entry
            .expect(&format!(
                "Failed to fetch an entry from directory '{}'",
                from_dir.display()
            ))
            .file_name()
            .into_string()
            .unwrap_or_else(|e| format!("Badly formed file name '{}'", e.to_string_lossy()))
    }

    fn copy_ok(from: &PathBuf, to: &PathBuf) -> bool {
        fs::copy(&from, &to).map_or_else(
            |err| {
                eprintln!("!!! Failed to copy '{}' to '{}': {}", from.display(), to.display(), err);
                false
            },
            |_| true,
        )
    }

    fn remove_ok(file: &PathBuf) -> bool {
        fs::remove_file(file).map_or_else(
            |err| {
                eprintln!("!!! Failed to remove '{}': {}", file.display(), err);
                false
            },
            |_| true,
        )
    }

    fn collect_stderrs(&mut self) -> Result<(), io::Error> {
        let dest_dir = &self.base_dir;
        let from_dir = &self.outputs_dir;

        if !from_dir.exists() {
            panic!("Outputs directory '{}' doesn't exists.", from_dir.display());
        }

        if !from_dir.is_dir() {
            panic!("'{}' is not a directory", from_dir.display());
        }

        for entry in std::fs::read_dir(&from_dir).expect(&format!("Failed to read '{}' directory", from_dir.display()))
        {
            let fname = Self::stringify_fname(entry, &from_dir);

            if fname.ends_with(".stderr") {
                let dest_stderr = dest_dir.join(&fname);
                let src_stderr = from_dir.join(&fname);
                eprintln!("+ {} -> {}", src_stderr.display(), dest_stderr.display());
                let _ = Self::copy_ok(&src_stderr, &dest_stderr);
                self.stderrs.push((src_stderr, dest_stderr));
            }
        }

        Ok(())
    }

    fn version_group() -> String {
        if let Ok(toolchain) = get_active_toolchain() {
            // If toolchain is <version>-<arch> then strip the arch part off
            let toolchain = if let Some((t, _)) = toolchain.split_once("-") {
                t.to_string()
            }
            else {
                toolchain
            };

            if Version::parse(&toolchain).is_err() {
                // This is a case of named toolchain, use it as the group name
                return toolchain.into();
            }
        }

        // If it's not the case of a named toolchain then use the compiler version as such
        let full_version = rustc_version::version().expect("Rust Compiler Version");
        // Still, if the version has any pre-release then remove it
        let version = Version::new(full_version.major, full_version.minor, full_version.patch);
        (if version < Version::new(1, 78, 0) {
            "1.77"
        }
        else if version == Version::new(1, 78, 0) {
            "1.78"
        }
        else if version <= Version::new(1, 80, 0) {
            "1.79"
        }
        else {
            panic!(
                "Unknown version {} ({})",
                full_version,
                get_active_toolchain().unwrap_or("<unknown toolchain>".into())
            );
        })
        .to_string()
    }

    fn outputs_subdir() -> String {
        #[allow(unused_mut)]
        let mut groups = Vec::<String>::new();

        #[cfg(feature = "serde")]
        groups.push("serde".into());

        #[cfg(feature = "sync")]
        groups.push("sync".into());

        #[cfg(feature = "async")]
        groups.push("async".into());

        #[cfg(feature = "diagnostics")]
        groups.push("diagnostics".into());

        if groups.len() > 0 {
            format!("{}+{}", Self::version_group(), groups.join(","))
        }
        else {
            Self::version_group()
        }
    }

    fn outputs_dir(base_dir: &PathBuf) -> Result<PathBuf, io::Error> {
        let outputs_dir = base_dir.join(Self::outputs_subdir());

        if !outputs_dir.exists() {
            std::fs::create_dir_all(&outputs_dir)?;
        }

        Ok(outputs_dir)
    }

    fn check_for_new(&self) -> Result<(), io::Error> {
        let test_dir = PathBuf::from(&self.base_dir);
        for entry in std::fs::read_dir(&test_dir).expect(&format!("Failed to read '{}' directory", test_dir.display()))
        {
            let fname = Self::stringify_fname(entry, &test_dir);

            if fname.ends_with(".stderr") {
                let src_stderr = test_dir.join(&fname);
                let dest_stderr = self.outputs_dir.join(&fname);
                eprintln!("> {} -> {}", fname, dest_stderr.display());
                let _ = Self::copy_ok(&src_stderr, &dest_stderr);
                let _ = Self::remove_ok(&src_stderr);
            }
        }

        Ok(())
    }
}

impl Drop for UncompEnv {
    fn drop(&mut self) {
        let mut with_failures = false;
        let try_build = env::var("TRYBUILD").map_or(false, |v| v == "overwrite");
        for (ref ver_stderr, ref stderr) in self.stderrs.iter() {
            if try_build {
                if try_build {
                    eprintln!("* Updating {}", ver_stderr.display());
                    with_failures = with_failures || !Self::copy_ok(stderr, ver_stderr);
                }
            }

            eprintln!("- Removing {}", stderr.display());
            with_failures = with_failures || !Self::remove_ok(stderr);
        }

        if try_build {
            // Pick up any new .stderrs
            self.check_for_new().unwrap();
        }

        if with_failures {
            panic!("Failed to update or remove some stderr files");
        }
    }
}

#[test]
fn failures() {
    if std::env::var("__FIELDX_DEFAULT_TOOLCHAIN__").map_or(true, |v| v != "nightly") {
        let test_env = UncompEnv::new("uncompilable");
        let t = trybuild::TestCases::new();
        t.compile_fail(format!("{}/*.rs", test_env.base_dir.display()));
    }
}

#[test]
fn success() {
    let t = trybuild::TestCases::new();
    t.pass("tests/compilable/*.rs");
}
