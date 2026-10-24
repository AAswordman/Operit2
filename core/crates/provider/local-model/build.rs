/// Builds native inference engines with an isolated Windows C++ runtime boundary.
fn main() {
    println!("cargo:rerun-if-changed=native");
    println!("cargo:rerun-if-env-changed=OPERIT_LLAMA_SOURCE");
    println!("cargo:rerun-if-env-changed=OPERIT_MNN_SOURCE");
    for variable in [
        "ANDROID_NDK_HOME",
        "ANDROID_NDK_ROOT",
        "ANDROID_NDK",
        "OHOS_NATIVE_HOME",
    ] {
        println!("cargo:rerun-if-env-changed={variable}");
    }
    #[cfg(feature = "native-llm")]
    if std::env::var("CARGO_CFG_TARGET_ARCH").as_deref() != Ok("wasm32") {
        let mut config = cmake::Config::new("native");
        config.profile("Release").build_target("operit_local_llm");
        // Prevent nested native builds from exhausting memory on large CI workers.
        if std::env::var_os("CMAKE_BUILD_PARALLEL_LEVEL").is_none() {
            let threads = std::thread::available_parallelism()
                .map(|n| n.get().min(8))
                .unwrap_or(2);
            config.env("CMAKE_BUILD_PARALLEL_LEVEL", threads.to_string());
            // cmake-rs passes NUM_JOBS as --parallel, overriding the CMake env var.
            let jobs = std::env::var("NUM_JOBS")
                .ok()
                .and_then(|value| value.parse::<usize>().ok())
                .unwrap_or(threads);
            std::env::set_var("NUM_JOBS", jobs.min(threads).to_string());
        }
        for (env, option) in [
            ("OPERIT_LLAMA_SOURCE", "FETCHCONTENT_SOURCE_DIR_LLAMA"),
            ("OPERIT_MNN_SOURCE", "FETCHCONTENT_SOURCE_DIR_MNN"),
        ] {
            if let Some(path) = std::env::var_os(env) {
                config.define(option, path);
            }
        }
        let os = std::env::var("CARGO_CFG_TARGET_OS").unwrap();
        let target_env = std::env::var("CARGO_CFG_TARGET_ENV").unwrap_or_default();
        let arch = std::env::var("CARGO_CFG_TARGET_ARCH").unwrap();
        if os == "android" {
            let ndk = ["ANDROID_NDK_HOME", "ANDROID_NDK_ROOT", "ANDROID_NDK"]
                .into_iter()
                .find_map(std::env::var_os)
                .expect("Set ANDROID_NDK_HOME to build embedded local LLM engines for Android");
            let abi = match arch.as_str() {
                "aarch64" => "arm64-v8a",
                "arm" => "armeabi-v7a",
                "x86_64" => "x86_64",
                "x86" => "x86",
                _ => panic!("Unsupported Android architecture: {arch}"),
            };
            config
                .generator("Ninja")
                .define(
                    "CMAKE_TOOLCHAIN_FILE",
                    std::path::PathBuf::from(ndk).join("build/cmake/android.toolchain.cmake"),
                )
                .define("ANDROID_ABI", abi)
                .define("ANDROID_PLATFORM", "android-23")
                .define("ANDROID_STL", "c++_static");
        } else if target_env == "ohos" {
            let sdk = std::env::var_os("OHOS_NATIVE_HOME")
                .expect("Set OHOS_NATIVE_HOME to the native OpenHarmony SDK directory");
            let abi = match arch.as_str() {
                "aarch64" => "arm64-v8a",
                "x86_64" => "x86_64",
                _ => panic!("Unsupported OHOS architecture: {arch}"),
            };
            config
                .generator("Ninja")
                .define(
                    "CMAKE_TOOLCHAIN_FILE",
                    std::path::PathBuf::from(sdk).join("build/cmake/ohos.toolchain.cmake"),
                )
                .define("OHOS_ARCH", abi)
                .define("OHOS_STL", "c++_static");
        }
        let windows = os == "windows";
        if windows {
            let out_dir = std::path::PathBuf::from(std::env::var_os("OUT_DIR").unwrap());
            let profile_dir = out_dir
                .ancestors()
                .nth(3)
                .expect("Cargo OUT_DIR must be under the profile build directory");
            // Place the runtime DLL beside Cargo's executable and bridge artifacts.
            config.define("OPERIT_LOCAL_LLM_RUNTIME_DIR", profile_dir);
        }
        let output = config.build();
        let build = output.join("build");
        let names: &[&str] = if windows {
            &["operit_local_llm"]
        } else {
            &[
                "operit_local_llm",
                "llama",
                "ggml",
                "ggml-cpu",
                "ggml-base",
                "MNN",
            ]
        };
        /// Locates one exact native library filename in CMake's target output.
        fn find(dir: &std::path::Path, name: &str) -> Option<std::path::PathBuf> {
            let entries = std::fs::read_dir(dir).ok()?;
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_dir() {
                    if let Some(found) = find(&path, name) {
                        return Some(found);
                    }
                } else if path.file_name().and_then(|n| n.to_str()) == Some(name) {
                    return Some(path);
                }
            }
            None
        }
        for name in names {
            let file = if target_env == "msvc" {
                format!("{name}.lib")
            } else if windows {
                format!("lib{name}.dll.a")
            } else {
                format!("lib{name}.a")
            };
            let path = find(&build, &file)
                .unwrap_or_else(|| panic!("native inference library missing: {file}"));
            println!(
                "cargo:rustc-link-search=native={}",
                path.parent().unwrap().display()
            );
            let kind = if windows { "dylib" } else { "static" };
            println!("cargo:rustc-link-lib={kind}={name}");
        }
        if windows {
            return;
        }
        if target_env == "ohos" {
            println!("cargo:rustc-link-lib=c++_static");
            println!("cargo:rustc-link-lib=c++abi");
            return;
        }
        match os.as_str() {
            "macos" | "ios" => println!("cargo:rustc-link-lib=c++"),
            "android" => {
                println!("cargo:rustc-link-lib=c++_static");
                println!("cargo:rustc-link-lib=log");
            }
            _ => {
                println!("cargo:rustc-link-lib=stdc++");
                println!("cargo:rustc-link-lib=pthread");
                println!("cargo:rustc-link-lib=dl");
            }
        }
    }
}
