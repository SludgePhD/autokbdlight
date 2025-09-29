use std::{fs, num::NonZeroU32, path::Path, str::FromStr};

use anyhow::ensure;
use serde::Deserialize;

#[derive(Default, Deserialize)]
pub struct Config {
    #[serde(default)]
    pub general: General,
    #[serde(rename = "input", default)]
    pub inputs: Vec<Input>,
    #[serde(rename = "led", default)]
    pub leds: Vec<Led>,
}

impl Config {
    pub fn load<P: AsRef<Path>>(path: P) -> anyhow::Result<Self> {
        Self::parse(&fs::read_to_string(path)?)
    }

    fn parse(contents: &str) -> anyhow::Result<Self> {
        Ok(toml::from_str(contents)?)
    }
}

#[derive(Deserialize)]
pub struct General {
    #[serde(default = "default_timeout")]
    pub timeout: NonZeroU32,
    #[serde(default = "default_fade")]
    pub fade: f32,
    #[serde(default = "default_brightness")]
    pub brightness: Brightness,
}

impl Default for General {
    fn default() -> Self {
        Self {
            timeout: default_timeout(),
            fade: default_fade(),
            brightness: default_brightness(),
        }
    }
}

fn default_timeout() -> NonZeroU32 {
    const { NonZeroU32::new(10).unwrap() }
}
fn default_fade() -> f32 {
    0.10 // 100ms
}
fn default_brightness() -> Brightness {
    Brightness(100)
}

#[derive(Deserialize)]
pub struct Input {
    pub name: String,
}

#[derive(Deserialize)]
pub struct Led {
    pub name: String,
    pub brightness: Option<Brightness>,
}

#[derive(Debug, Clone, Copy, Deserialize)]
#[serde(try_from = "u8")]
pub struct Brightness(u8);

impl Brightness {
    pub fn raw(&self) -> u8 {
        self.0
    }
}

impl TryFrom<u8> for Brightness {
    type Error = anyhow::Error;

    fn try_from(raw: u8) -> Result<Self, Self::Error> {
        ensure!(raw <= 100, "brightness must be in range 0-100");
        Ok(Self(raw))
    }
}

impl FromStr for Brightness {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        s.parse::<u8>()?.try_into()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn example_config_parses() {
        Config::load("config.example.toml").unwrap();
    }

    #[test]
    fn empty_config_parses() {
        Config::parse("").unwrap();
    }
}
