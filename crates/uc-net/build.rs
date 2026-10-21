//! The build ID the players compare at hello: the git commit, with `-dirty` for uncommitted
//! changes. Two builds of the same commit play the same game; any other pair may not.

use std::process::Command;

fn main() {
    let git = |args: &[&str]| {
        Command::new("git").args(args).output().ok().filter(|o| o.status.success())
            .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
    };
    let id = git(&["describe", "--always", "--dirty", "--abbrev=12"]).unwrap_or_else(|| "unknown".into());
    println!("cargo:rustc-env=UC_BUILD_ID={id}");
    // Again when the commit changes (HEAD, or the branch it points at) or the index does.
    if let Some(dir) = git(&["rev-parse", "--git-dir"]) {
        let dir = std::path::Path::new(&dir);
        println!("cargo:rerun-if-changed={}", dir.join("HEAD").display());
        println!("cargo:rerun-if-changed={}", dir.join("index").display());
        if let Some(head) = git(&["symbolic-ref", "-q", "HEAD"]) {
            println!("cargo:rerun-if-changed={}", dir.join(head).display());
        }
    }
    println!("cargo:rerun-if-changed=build.rs");
}
