use std::process::Command;

fn git(args: &[&str]) -> String {
    Command::new("git")
        .args(args)
        .output()
        .ok()
        .filter(|o| o.status.success())
        .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
        .unwrap_or_default()
}

fn main() {
    // Re-run if git HEAD changes (new commits, branch switches, tags)
    println!("cargo:rerun-if-changed=.git/HEAD");

    let commit = git(&["rev-parse", "--short", "HEAD"]);
    let date = git(&["log", "-1", "--format=%ci"]);

    // If HEAD is on a tag, use it as version; otherwise use branch name
    let tag = git(&["tag", "--points-at", "HEAD"]);
    let version = if tag.is_empty() {
        let branch = git(&["rev-parse", "--abbrev-ref", "HEAD"]);
        if branch.is_empty() {
            "dev".to_string()
        } else {
            branch
        }
    } else {
        // Take first tag if multiple
        tag.lines().next().unwrap_or("dev").to_string()
    };

    println!("cargo:rustc-env=GIT_VERSION={version}");
    println!("cargo:rustc-env=GIT_COMMIT={commit}");
    println!("cargo:rustc-env=GIT_DATE={date}");
}
