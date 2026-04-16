use super::*;
use serial_test::serial;
use std::fs;
use std::path::PathBuf;

fn create_temp_home(test_name: &str) -> PathBuf {
    let temp_dir = std::env::temp_dir().join(test_name);
    let _ = fs::remove_dir_all(&temp_dir);
    fs::create_dir_all(&temp_dir).expect("创建临时目录失败");
    temp_dir
}

fn with_temp_home<F>(test_name: &str, test_fn: F)
where
    F: FnOnce(&PathBuf),
{
    let temp_dir = create_temp_home(test_name);
    let original_home = std::env::var("HOME").ok();

    unsafe {
        std::env::set_var("HOME", &temp_dir);
    }

    test_fn(&temp_dir);

    unsafe {
        if let Some(home) = original_home {
            std::env::set_var("HOME", home);
        } else {
            std::env::remove_var("HOME");
        }
    }

    let _ = fs::remove_dir_all(&temp_dir);
}

#[test]
fn test_has_newer_version() {
    assert!(has_newer_version("3.5.2", "3.5.3"));
    assert!(!has_newer_version("3.5.3", "3.5.3"));
    assert!(!has_newer_version("3.5.3", "3.5.2"));
    assert!(has_newer_version("3.4.0", "3.5.0"));
}

#[test]
#[serial]
fn test_is_omo_installed_with_platform_specific_package() {
    with_temp_home("omo_version_service_platform_install_test", |temp_dir| {
        let package_dir = temp_dir
            .join(".config")
            .join("opencode")
            .join("node_modules")
            .join("oh-my-opencode-darwin-arm64");
        fs::create_dir_all(&package_dir).expect("创建平台包目录失败");
        fs::write(
            package_dir.join("package.json"),
            r#"{"name":"oh-my-opencode-darwin-arm64","version":"3.17.2"}"#,
        )
        .expect("写入平台包 package.json 失败");

        assert!(is_omo_installed(), "存在平台专属包时应识别为已安装");
    });
}

#[test]
#[serial]
fn test_detect_omo_install_reads_platform_specific_package_version() {
    with_temp_home("omo_version_service_platform_version_test", |temp_dir| {
        let package_dir = temp_dir
            .join(".config")
            .join("opencode")
            .join("node_modules")
            .join("oh-my-openagent-darwin-arm64");
        fs::create_dir_all(&package_dir).expect("创建平台包目录失败");
        fs::write(
            package_dir.join("package.json"),
            r#"{"name":"oh-my-openagent-darwin-arm64","version":"3.17.2"}"#,
        )
        .expect("写入平台包 package.json 失败");

        let detection = detect_omo_install();

        assert!(detection.is_some(), "应检测到平台专属包安装");
        let detection = detection.expect("平台专属包检测结果不能为空");
        assert_eq!(detection.version, Some("3.17.2".to_string()));
        assert_eq!(detection.install_source, "config_local");
        assert_eq!(
            detection.install_path,
            temp_dir.join(".config").join("opencode").to_string_lossy()
        );
        assert!(
            detection
                .detected_from
                .ends_with(".config/opencode/node_modules/oh-my-openagent-darwin-arm64/package.json"),
            "detected_from 应指向平台专属包 package.json"
        );
    });
}

#[test]
#[serial]
fn test_read_dependency_version_supports_optional_platform_packages() {
    with_temp_home("omo_version_service_optional_dep_test", |temp_dir| {
        let runtime_dir = temp_dir.join(".opencode");
        fs::create_dir_all(&runtime_dir).expect("创建运行目录失败");
        fs::write(
            runtime_dir.join("package.json"),
            r#"{
                "optionalDependencies": {
                    "oh-my-openagent-darwin-arm64": "3.17.2"
                }
            }"#,
        )
        .expect("写入 package.json 失败");

        let version = read_dependency_version(
            &runtime_dir.join("package.json").to_string_lossy(),
            &OMO_PACKAGE_NAMES,
        );

        assert_eq!(version, Some("3.17.2".to_string()));
    });
}
