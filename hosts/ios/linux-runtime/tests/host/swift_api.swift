import OperitArm64

// Compile-only ABI check for an eventual Swift session adapter.
public func validateRuntimeAPI(root: UnsafePointer<CChar>) -> Int32 {
    var config = ort_arm64_config()
    config.root_directory = root
    config.resident_memory_limit = 32 * 1024 * 1024
    config.enable_fixed_acceleration = 0
    guard let vm = ort_arm64_create(&config) else { return -1 }
    defer { ort_arm64_destroy(vm) }
    let result = ort_arm64_run(vm, 1000)
    _ = ort_arm64_last_error(vm)
    return result.exit_code
}

// Ensure the new bind API is exported and importable without a Swift wrapper.
public func validateRuntimeBindingAPI(vm: OpaquePointer, source: UnsafePointer<CChar>,
                                     target: UnsafePointer<CChar>) -> Int32 {
    return ort_arm64_bind(vm, source, target, UInt32(ORT_ARM64_BIND_READ_ONLY))
}
