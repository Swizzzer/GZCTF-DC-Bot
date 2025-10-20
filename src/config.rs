use serde::Deserialize;

#[derive(Debug, Deserialize, Clone)]
pub struct DiscordConfig {
    pub token: String,
    pub channel_id: u64,
}

#[derive(Debug, Deserialize, Clone)]
pub struct GzctfConfig {
    pub url: String,
    pub poll_interval: u64,
    #[serde(default)]
    pub matches: Vec<MatchConfig>,
    #[serde(default)]
    pub match_id: Option<u32>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct MatchConfig {
    pub id: u32,
    pub name: Option<String>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct Config {
    pub discord: DiscordConfig,
    pub gzctf: GzctfConfig,
}

impl Config {
    pub fn from_file(path: &str) -> anyhow::Result<Self> {
        let config_str = std::fs::read_to_string(path)?;
        let config: Config = toml::from_str(&config_str)?;
        Ok(config)
    }

    pub fn get_matches(&self) -> Vec<MatchConfig> {
        if !self.gzctf.matches.is_empty() {
            self.gzctf.matches.clone()
        } else if let Some(match_id) = self.gzctf.match_id {
            vec![MatchConfig {
                id: match_id,
                name: None,
            }]
        } else {
            Vec::new()
        }
    }
}
