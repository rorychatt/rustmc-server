use plugin_bridge::JvmManager;

#[test]
fn test_jvm_get_returns_none_or_some() {
    // JvmManager::get() returns None if JVM hasn't been initialized,
    // or Some if another test in this process already initialized it.
    // Either is valid — we just verify it doesn't panic.
    let _ = JvmManager::get();
}

#[test]
fn test_jvm_initialize_fails_without_java() {
    // On systems without Java installed, initialization should return
    // a clear error rather than panicking
    let result = JvmManager::initialize(&[]);
    match result {
        Ok(_jvm) => {
            // Java is installed — JVM initialized successfully
            // Verify we can get it back
            assert!(JvmManager::get().is_some());
        }
        Err(e) => {
            let msg = e.to_string();
            // Should give a meaningful error about missing JVM
            assert!(
                msg.contains("JVM") || msg.contains("Java") || msg.contains("jvm") || msg.contains("java"),
                "Error should mention JVM/Java: {msg}"
            );
        }
    }
}

#[test]
fn test_jvm_shutdown_is_safe_without_init() {
    // shutdown() should be safe to call even if JVM was never initialized
    JvmManager::shutdown();
}
