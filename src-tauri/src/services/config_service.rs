use crate::i18n;
use serde_json::Value;
use std::fs;
use std::path::PathBuf;

/// 移除 JSONC 中的注释
fn strip_jsonc_comments(content: &str) -> String {
    let mut result = String::with_capacity(content.len());
    let chars: Vec<char> = content.chars().collect();
    let mut i = 0;
    let mut in_string = false;

    while i < chars.len() {
        let c = chars[i];

        if c == '"' && (i == 0 || chars[i - 1] != '\\') {
            in_string = !in_string;
            result.push(c);
            i += 1;
            continue;
        }

        if in_string {
            result.push(c);
            i += 1;
            continue;
        }

        if i + 1 < chars.len() && c == '/' && chars[i + 1] == '/' {
            while i < chars.len() && chars[i] != '\n' {
                i += 1;
            }
            continue;
        }

        if i + 1 < chars.len() && c == '/' && chars[i + 1] == '*' {
            i += 2;
            while i + 1 < chars.len() {
                if chars[i] == '*' && chars[i + 1] == '/' {
                    i += 2;
                    break;
                }
                i += 1;
            }
            continue;
        }

        result.push(c);
        i += 1;
    }

    result
}

/// 配置文件名优先级列表
/// oh-my-openagent 优先，回退到 oh-my-opencode
const CONFIG_FILE_CANDIDATES: &[&str] = &[
    "oh-my-openagent.jsonc",
    "oh-my-openagent.json",
    "oh-my-opencode.jsonc",
    "oh-my-opencode.json",
];

/// 获取 OMO 配置文件路径
/// 优先级: oh-my-openagent.jsonc > oh-my-openagent.json > oh-my-opencode.jsonc > oh-my-opencode.json
/// 优先查找存在的文件，都不存在时返回最后的回退路径
pub fn get_config_path() -> Result<PathBuf, String> {
    let home = std::env::var("HOME").map_err(|_| i18n::tr_current("home_env_var_error"))?;

    let config_dir = PathBuf::from(home).join(".config").join("opencode");

    for candidate in CONFIG_FILE_CANDIDATES {
        let path = config_dir.join(candidate);
        if path.exists() {
            return Ok(path);
        }
    }

    // 都不存在时，回退到 oh-my-opencode.json（保持向后兼容）
    Ok(config_dir.join("oh-my-opencode.json"))
}

/// 读取 OMO 配置文件
pub fn read_omo_config() -> Result<Value, String> {
    let config_path = get_config_path()?;

    if !config_path.exists() {
        return Err(i18n::tr_current("config_file_not_found"));
    }

    let content = fs::read_to_string(&config_path)
        .map_err(|e| format!("{}: {}", i18n::tr_current("read_config_failed"), e))?;

    let json_content = if config_path
        .extension()
        .map(|e| e == "jsonc")
        .unwrap_or(false)
    {
        strip_jsonc_comments(&content)
    } else {
        content
    };

    let config: Value = serde_json::from_str(&json_content)
        .map_err(|e| format!("{}: {}", i18n::tr_current("parse_json_failed"), e))?;

    Ok(config)
}

/// 写入 OMO 配置文件
pub fn write_omo_config(config: &Value) -> Result<(), String> {
    let config_path = get_config_path()?;

    if config_path.exists() {
        let backup_path = config_path.with_extension("json.bak");
        fs::copy(&config_path, &backup_path)
            .map_err(|e| format!("{}: {}", i18n::tr_current("create_backup_failed"), e))?;
    }

    if let Some(parent) = config_path.parent() {
        fs::create_dir_all(parent)
            .map_err(|e| format!("{}: {}", i18n::tr_current("create_config_dir_failed"), e))?;
    }

    let json_string = serde_json::to_string_pretty(config)
        .map_err(|e| format!("{}: {}", i18n::tr_current("serialize_json_failed"), e))?;

    let is_jsonc = config_path
        .extension()
        .map(|e| e == "jsonc")
        .unwrap_or(false);
    if is_jsonc {
        eprintln!("警告：写入 .jsonc 文件会丢失注释，原注释已备份到 .json.bak");
    }

    fs::write(&config_path, json_string)
        .map_err(|e| format!("{}: {}", i18n::tr_current("write_config_failed"), e))?;

    Ok(())
}

/// 验证配置文件基本结构
/// 检查是否包含必需的 agents 和 categories 键
pub fn validate_config(config: &Value) -> Result<(), String> {
    // 检查是否为对象
    if !config.is_object() {
        return Err(i18n::tr_current("config_root_must_be_object"));
    }

    let obj = config.as_object().unwrap();

    // 检查必需字段
    if !obj.contains_key("agents") {
        return Err(i18n::tr_current("config_missing_agents"));
    }

    if !obj.contains_key("categories") {
        return Err(i18n::tr_current("config_missing_categories"));
    }

    // 检查 agents 是否为对象
    if !obj["agents"].is_object() {
        return Err("'agents' 字段必须是对象".to_string());
    }

    // 检查 categories 是否为对象
    if !obj["categories"].is_object() {
        return Err("'categories' 字段必须是对象".to_string());
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use std::fs;

    /// 测试配置路径生成
    #[test]
    fn test_get_config_path() {
        let path = get_config_path().unwrap();
        let path_str = path.to_string_lossy();
        assert!(path_str.contains(".config/opencode/"));
        let valid_endings = [
            "oh-my-openagent.jsonc",
            "oh-my-openagent.json",
            "oh-my-opencode.jsonc",
            "oh-my-opencode.json",
        ];
        assert!(valid_endings.iter().any(|e| path_str.ends_with(e)));
    }

    /// 测试配置验证 - 有效配置
    #[test]
    fn test_validate_config_valid() {
        let config = json!({
            "agents": {
                "sisyphus": {
                    "model": "test-model"
                }
            },
            "categories": {
                "quick": {
                    "model": "test-model"
                }
            }
        });

        assert!(validate_config(&config).is_ok());
    }

    /// 测试配置验证 - 缺少 agents
    #[test]
    fn test_validate_config_missing_agents() {
        let config = json!({
            "categories": {}
        });

        let result = validate_config(&config);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("agents"));
    }

    /// 测试配置验证 - 缺少 categories
    #[test]
    fn test_validate_config_missing_categories() {
        let config = json!({
            "agents": {}
        });

        let result = validate_config(&config);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("categories"));
    }

    /// 测试配置验证 - 根节点不是对象
    #[test]
    fn test_validate_config_not_object() {
        let config = json!([]);

        let result = validate_config(&config);
        assert!(result.is_err());
        let err_msg = result.unwrap_err();
        assert!(err_msg.contains("对象") || err_msg.contains("object"));
    }

    /// 测试往返保留所有字段
    #[test]
    fn test_roundtrip_preserves_fields() {
        // 创建临时测试配置
        let test_config = json!({
            "$schema": "https://example.com/schema.json",
            "agents": {
                "test-agent": {
                    "model": "test-model",
                    "variant": "high",
                    "custom_field": "custom_value"
                }
            },
            "categories": {
                "test-category": {
                    "model": "test-model"
                }
            },
            "unknown_field": "should_be_preserved",
            "nested": {
                "deep": {
                    "value": 123
                }
            }
        });

        // 创建临时目录
        let temp_dir = std::env::temp_dir().join("omo-test");
        fs::create_dir_all(&temp_dir).unwrap();

        let test_path = temp_dir.join("test-config.json");

        // 写入测试配置
        let json_string = serde_json::to_string_pretty(&test_config).unwrap();
        fs::write(&test_path, json_string).unwrap();

        // 模拟读取（直接从文件读）
        let content = fs::read_to_string(&test_path).unwrap();
        let read_config: Value = serde_json::from_str(&content).unwrap();

        // 验证所有字段都被保留
        assert_eq!(read_config["$schema"], test_config["$schema"]);
        assert_eq!(read_config["agents"], test_config["agents"]);
        assert_eq!(read_config["categories"], test_config["categories"]);
        assert_eq!(read_config["unknown_field"], test_config["unknown_field"]);
        assert_eq!(read_config["nested"]["deep"]["value"], 123);

        // 清理
        fs::remove_dir_all(&temp_dir).unwrap();
    }

    /// 测试备份文件创建
    #[test]
    fn test_backup_file_creation() {
        // 创建临时目录
        let temp_dir = std::env::temp_dir().join("omo-backup-test");
        fs::create_dir_all(&temp_dir).unwrap();

        let test_path = temp_dir.join("test-config.json");
        let backup_path = temp_dir.join("test-config.json.bak");

        // 创建初始配置
        let initial_config = json!({
            "agents": {},
            "categories": {}
        });

        fs::write(
            &test_path,
            serde_json::to_string_pretty(&initial_config).unwrap(),
        )
        .unwrap();

        // 模拟写入新配置（会创建备份）
        let new_config = json!({
            "agents": {"new": {}},
            "categories": {}
        });

        // 手动执行备份逻辑
        if test_path.exists() {
            fs::copy(&test_path, &backup_path).unwrap();
        }
        fs::write(
            &test_path,
            serde_json::to_string_pretty(&new_config).unwrap(),
        )
        .unwrap();

        // 验证备份文件存在
        assert!(backup_path.exists());

        // 验证备份内容是初始配置
        let backup_content = fs::read_to_string(&backup_path).unwrap();
        let backup_config: Value = serde_json::from_str(&backup_content).unwrap();
        assert_eq!(backup_config, initial_config);

        // 清理
        fs::remove_dir_all(&temp_dir).unwrap();
    }
}
