fn main() {
    {
        // build info
        use std::env;

        let target = env::var("TARGET").unwrap();
        let profile = env::var("PROFILE").unwrap();

        println!("cargo:rustc-env=BUILD_TARGET={}", target);
        println!("cargo:rustc-env=BUILD_PROFILE={}", profile);
    }

    if cfg!(feature = "webui") {
        let status = std::process::Command::new("npm")
            .args(&["run", "build"])
            .current_dir("web")
            .status()
            .expect("Unable to execute npm. Seems node packages manager does not installed. It required because 'webui' feature is enabled.");
        if !status.success() {
            panic!("Error when executing command 'npm run build'");
        }
    }
}
