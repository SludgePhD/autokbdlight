use std::{
    fs::{self, File},
    io::{Read, Seek, SeekFrom, Write},
    path::PathBuf,
    str,
    time::{Duration, Instant},
};

use anyhow::{Context, anyhow, bail, ensure};

use crate::config::Config;

const BASE_PATH: &str = "/sys/class/leds/";
const BACKLIGHT: &str = "kbd_backlight";

/// A set of LEDs in `/sys/class/leds`.
pub struct Leds {
    fade_duration: f32,
    leds: Vec<Led>,
}

struct Led {
    name: String,
    file: File,
    /// Abs. target brightness
    target_brightness: u32,
    /// Start brightness for fading.
    start_brightness: u32,
}

impl Led {
    fn open(name: String, target_brightness: u8) -> anyhow::Result<Self> {
        let mut base_path = PathBuf::from(BASE_PATH);
        base_path.push(&name);

        log::info!("opening '{}'", base_path.display());
        let max_brightness = fs::read_to_string(base_path.join("max_brightness"))?;
        log::debug!("max brightness: '{}'", max_brightness.trim());
        let max_brightness: u32 = max_brightness.trim().parse()?;
        let file = File::options()
            .read(true)
            .write(true)
            .open(base_path.join("brightness"))
            .with_context(|| base_path.display().to_string())?;
        let target_brightness =
            (target_brightness as f32 / 100.0 * max_brightness as f32).round() as u32;
        log::debug!("abs. target brightness: {target_brightness}");
        Ok(Self {
            name,
            file,
            target_brightness,
            start_brightness: 0,
        })
    }

    fn read_brightness(&self) -> anyhow::Result<u32> {
        (&self.file).seek(SeekFrom::Start(0))?;
        let mut buf = [0; 32];
        let n = (&self.file).read(&mut buf)?;
        let s = str::from_utf8(&buf[..n])?;
        s.trim().parse().map_err(Into::into)
    }

    fn set_brightness(&self, val: u32) -> anyhow::Result<()> {
        let mut buf = [0; 32];
        let mut writer = &mut buf[..];
        writeln!(writer, "{val}").ok();
        let remaining = writer.len();
        let n = buf.len() - remaining;
        (&self.file).write(&buf[..n])?;
        Ok(())
    }
}

impl Leds {
    pub fn from_config(conf: &Config) -> anyhow::Result<Self> {
        if conf.leds.is_empty() {
            return Ok(Self {
                leds: Self::auto(conf)?,
                fade_duration: conf.general.fade,
            });
        }

        let leds: Vec<Led> = conf
            .leds
            .iter()
            .map(|led| {
                Led::open(
                    led.name.clone(),
                    led.brightness.unwrap_or(conf.general.brightness).raw(),
                )
            })
            .collect::<anyhow::Result<_>>()?;

        Ok(Self {
            leds,
            fade_duration: conf.general.fade,
        })
    }

    fn auto(conf: &Config) -> anyhow::Result<Vec<Led>> {
        let mut leds = Vec::new();
        for res in fs::read_dir(BASE_PATH)? {
            let entry = res?;
            let name = entry.file_name();
            let Some(name) = name.to_str() else {
                continue;
            };

            if !name.contains(BACKLIGHT) {
                continue;
            }

            leds.push(Led::open(name.into(), conf.general.brightness.raw())?);
        }

        if leds.is_empty() {
            bail!("didn't find any LEDs named `{BACKLIGHT}` in {BASE_PATH}");
        }

        Ok(leds)
    }

    pub fn set_state(&mut self, state: bool) -> anyhow::Result<()> {
        self.each_led(|led| {
            led.start_brightness = led
                .read_brightness()
                .map_err(|e| anyhow!("failed to read LED `{}`: {e}", led.name))?;
            Ok(())
        })?;

        let start = Instant::now() - Duration::from_millis(10);
        loop {
            let t = start.elapsed().as_secs_f32() / self.fade_duration;
            self.each_led(|led| {
                let target = match state {
                    true => led.target_brightness as f32,
                    false => 0.0,
                };
                let val =
                    lerp(led.start_brightness as f32, target, t.clamp(0.0, 1.0)).round() as u32;
                led.set_brightness(val)
                    .map_err(|e| anyhow!("failed to set LED `{}`: {e}", led.name))?;
                Ok(())
            })?;

            if t >= 1.0 {
                return Ok(());
            }
        }
    }

    fn each_led(
        &mut self,
        mut f: impl FnMut(&mut Led) -> anyhow::Result<()>,
    ) -> anyhow::Result<()> {
        // When an error occurs while trying to read/write to an LED, log the error and drop that
        // LED from the list.
        self.leds.retain_mut(|led| match f(led) {
            Ok(()) => true,
            Err(e) => {
                log::error!("LED '{}': {e}", led.name);
                false
            }
        });

        ensure!(
            !self.leds.is_empty(),
            "all controlled LEDs encountered errors"
        );
        Ok(())
    }
}

fn lerp(start: f32, end: f32, t: f32) -> f32 {
    start + t * (end - start)
}
