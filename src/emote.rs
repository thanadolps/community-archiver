use std::{collections::HashMap, fs::File, io, path::Path};

#[derive(Debug)]
pub struct EmoteResolver {
    mapping: HashMap<String, String>,
}

impl EmoteResolver {
    pub fn with_emote_dir(emote_dir: &Path) -> Self {
        Self {
            mapping: load_emoji_mapping(emote_dir),
        }
    }

    pub fn with_mapping(mapping: HashMap<String, String>) -> Self {
        Self { mapping }
    }

    pub fn resolve_emoji(&self, src: &str, alt: Option<&str>) -> Option<String> {
        let alt = alt.filter(|a| !a.trim().is_empty());

        // unicode emoji
        if src.contains("emoji_u") {
            if let Some(c) = alt {
                return Some(c.to_string());
            }

            // backup
            let code = src
                .split('/')
                .last()
                .unwrap()
                .split('.')
                .next()
                .unwrap()
                .trim_start_matches("emoji_u");
            let result = code
                .split('_')
                .map(|c| std::char::from_u32(u32::from_str_radix(c, 16).unwrap()).unwrap())
                .collect::<String>();
            return Some(result);
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
        self.mapping.get(id).map(|name| format!(":_{}:", name))
    }
}

fn load_emoji_mapping(emote_dir: &Path) -> HashMap<String, String> {
    let default: HashMap<String, String> =
        match File::open(emote_dir.join("emote_mapping_default.json")) {
            Ok(fs) => serde_json::from_reader(fs).unwrap(),
            Err(err) if err.kind() == io::ErrorKind::NotFound => Default::default(),
            Err(err) => panic!("{}", err),
        };

    let custom: HashMap<String, String> = match File::open(emote_dir.join("emote_mapping.json")) {
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
        let resolver = EmoteResolver::with_mapping(
            [(
                "FrYgdeZPpvXs-6Mp305ZiimWJ0wV5bcVZctaUy80mnIdwe-P8HRGYAm0OyBtVx8EB9_Dxkc="
                    .to_string(),
                "eyes-purple-crying".to_string(),
            )]
            .into(),
        );

        assert_eq!(
            resolver.resolve_emoji(
                "https://www.youtube.com/s/gaming/emoji/7ff574f2/emoji_u1f499.png",
                Some("💙")
            ),
            Some("💙".to_string())
        );

        assert_eq!(
            resolver.resolve_emoji(
                "https://yt3.googleusercontent.com/9CkO5FMttx7Cx-6HUnNQZ6RhhddVL4oBzrCX_A3kUYDL0nKVWCfwYp49_w3mjgSn7oBey3dxerU=s16-w24-h24-c-k-nd",
                Some("Rawr")
            ),
            Some(":_Rawr:".to_string())
        );

        assert_eq!(
            resolver.resolve_emoji(
                "https://www.youtube.com/s/gaming/emoji/7ff574f2/emoji_u1f499.png",
                None
            ),
            Some("💙".to_string())
        );

        assert_eq!(
            resolver.resolve_emoji(
                "https://www.youtube.com/s/gaming/emoji/7ff574f2/emoji_u1f647_200d_2642.png",
                None
            ),
            Some("🙇‍♂".to_string())
        );

        assert_eq!(
            resolver.resolve_emoji(
                "https://www.youtube.com/s/gaming/emoji/7ff574f2/emoji_u1f64c_1f3fb.png",
                None
            ),
            Some("🙌🏻".to_string())
        );

        assert_eq!(
            resolver.resolve_emoji(
                "https://www.youtube.com/s/gaming/emoji/7ff574f2/emoji_u1f64f_1f3fc.png",
                None
            ),
            Some("🙏🏼".to_string())
        );

        assert_eq!(
            resolver.resolve_emoji(
                "https://lh3.googleusercontent.com/FrYgdeZPpvXs-6Mp305ZiimWJ0wV5bcVZctaUy80mnIdwe-P8HRGYAm0OyBtVx8EB9_Dxkc=s16-w24-h24-c-k-nd",
                None
            ),
            Some(":_eyes-purple-crying:".to_string())
        );
    }
}
