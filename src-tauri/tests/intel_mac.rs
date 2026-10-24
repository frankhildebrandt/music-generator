use music_generator::{engine::device_plan, hardware::cpu_only_target, types::Settings};

#[test]
fn intel_mac_uses_one_cpu_server_even_with_saved_gpu_fallback_or_cpu_preference() {
    assert!(cpu_only_target("macos", "x86_64"));
    assert!(!cpu_only_target("macos", "aarch64"));
    assert!(!cpu_only_target("windows", "x86_64"));
    assert!(!cpu_only_target("linux", "x86_64"));
    for device in ["auto", "cpu"] {
        for mode in ["single", "split"] {
            let settings = Settings {
                lm_device: device.into(),
                mode: mode.into(),
                ..Default::default()
            };
            assert_eq!(device_plan(&settings, true), (true, false));
            assert_eq!(
                device_plan(&settings, false),
                (false, device == "cpu" || mode == "split")
            );
        }
    }
}
