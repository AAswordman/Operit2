/// Builds native inference engines and links the configured C++ runtime.
fn main() {
    println!("cargo:rerun-if-changed=native");
    println!("cargo:rerun-if-env-changed=OPERIT_LLAMA_SOURCE");
    println!("cargo:rerun-if-env-changed=OPERIT_MNN_SOURCE");
    println!("cargo:rerun-if-env-changed=OPERIT_KLEIDIAI_SOURCE");
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
            ("OPERIT_KLEIDIAI_SOURCE", "KLEIDIAI_SRC_DIR"),
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
            let sdk = std::path::PathBuf::from(
                std::env::var_os("OHOS_NATIVE_HOME")
                    .expect("Set OHOS_NATIVE_HOME to the native OpenHarmony SDK directory"),
            );
            let abi = match arch.as_str() {
                "aarch64" => "arm64-v8a",
                "x86_64" => "x86_64",
                _ => panic!("Unsupported OHOS architecture: {arch}"),
            };
            config
                .generator("Ninja")
                .define(
                    "CMAKE_TOOLCHAIN_FILE",
                    sdk.join("build/cmake/ohos.toolchain.cmake"),
                )
                .define(
                    "CMAKE_CXX_STANDARD_INCLUDE_DIRECTORIES",
                    sdk.join("llvm/include/c++/v1"),
                )
                .define("OHOS_ARCH", abi)
                .define("OHOS_STL", "c++_static");
        }
        let windows = os == "windows";
        let shared = windows || os == "android" || target_env == "ohos";
        let out_dir = std::path::PathBuf::from(std::env::var_os("OUT_DIR").unwrap());
        let profile_dir = out_dir
            .ancestors()
            .nth(3)
            .expect("Cargo OUT_DIR must be under the profile build directory");
        if shared {
            // Place the shared engine beside Cargo's executable and bridge artifacts.
            config.define("OPERIT_LOCAL_LLM_RUNTIME_DIR", profile_dir);
        }
        let output = config.build();
        let build = output.join("build");
        let names: &[&str] = if shared {
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
            } else if shared {
                format!("lib{name}.so")
            } else {
                format!("lib{name}.a")
            };
            let path = if shared && !windows {
                let path = profile_dir.join(&file);
                assert!(path.is_file(), "native inference library missing: {file}");
                path
            } else {
                find(&build, &file)
                    .unwrap_or_else(|| panic!("native inference library missing: {file}"))
            };
            println!(
                "cargo:rustc-link-search=native={}",
                path.parent().unwrap().display()
            );
            let kind = if shared { "dylib" } else { "static" };
            println!("cargo:rustc-link-lib={kind}={name}");
        }
        if shared {
            return;
        }
        match os.as_str() {
            "macos" | "ios" => {
                println!("cargo:rustc-link-lib=c++");
                println!("cargo:rustc-link-lib=framework=Accelerate");
            }
            _ => {
                let libraries = std::fs::read_to_string(build.join("operit_cxx_runtime.txt"))
                    .expect("CMake must report the configured C++ runtime");
                for library in libraries.lines() {
                    println!("cargo:rustc-link-lib={library}");
                }
                println!("cargo:rustc-link-lib=pthread");
                println!("cargo:rustc-link-lib=dl");
            }
        }
    }
}
