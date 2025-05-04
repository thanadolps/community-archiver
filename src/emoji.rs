use std::{collections::HashMap, fs::File, io, sync::LazyLock};

pub fn resolve_emoji(src: &str, alt: Option<&str>) -> Option<String> {
    // unicode emoji
    if src.contains("emoji_u") {
        if let Some(c) = alt {
            return Some(c.to_string());
        }

        // backup
        let code = src.split('_').next().unwrap();
        let code = u32::from_str_radix(code, 16).unwrap();
        return Some(std::char::from_u32(code).unwrap().to_string());
    }

    // custom emoji
    if let Some(c) = alt {
        return Some(format!(":_{}:", c));
    }

    // backup
    let id = src
        .split("/")
        .last()
        .unwrap()
        .split(".")
        .next()
        .unwrap()
        .split_inclusive("=")
        .next()
        .unwrap();
    custom_emoji_name(id).map(|name| format!(":_{}:", name))
}

fn custom_emoji_name(id: &str) -> Option<&str> {
    MAPPING.get(id).map(|s| s.as_str())
}

static MAPPING: LazyLock<HashMap<String, String>> = LazyLock::new(load_emoji_mapping);

fn load_emoji_mapping() -> HashMap<String, String> {
    let default: HashMap<String, String> = match File::open("data/emoji_mapping_default.json") {
        Ok(fs) => serde_json::from_reader(fs).unwrap(),
        Err(err) if err.kind() == io::ErrorKind::NotFound => Default::default(),
        Err(err) => panic!("{}", err),
    };

    let custom: HashMap<String, String> = match File::open("data/emoji_mapping.json") {
        Ok(fs) => serde_json::from_reader(fs).unwrap(),
        Err(err) if err.kind() == io::ErrorKind::NotFound => Default::default(),
        Err(err) => panic!("{}", err),
    };

    default.into_iter().chain(custom).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_emoji() {
        assert_eq!(
            resolve_emoji(
                "https://www.youtube.com/s/gaming/emoji/7ff574f2/emoji_u1f499.png",
                Some("💙")
            ),
            Some("💙".to_string())
        );

        assert_eq!(
            resolve_emoji(
                "https://yt3.googleusercontent.com/9CkO5FMttx7Cx-6HUnNQZ6RhhddVL4oBzrCX_A3kUYDL0nKVWCfwYp49_w3mjgSn7oBey3dxerU=s16-w24-h24-c-k-nd",
                Some("Rawr")
            ),
            Some(":_Rawr:".to_string())
        );
    }
}
