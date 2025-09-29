mod config;
mod input;
mod leds;
mod logger;

use std::{
    path::PathBuf,
    time::{Duration, Instant},
};

use anyhow::bail;
use clap::Parser;

use crate::{
    config::{Brightness, Config},
    input::{DeviceFilter, InputHandler},
    leds::Leds,
};

#[derive(Parser)]
#[command(version, about)]
struct Args {
    /// Path to the (optional) configuration file.
    #[arg(short, long)]
    config: Option<PathBuf>,

    /// Enables more verbose logging.
    #[arg(short, long)]
    verbose: bool,

    /// Sets the default LED brightness when no config file is used (0-100).
    #[arg(long)]
    brightness: Option<Brightness>,
}

fn main() -> anyhow::Result<()> {
    let args = Args::parse();
    logger::init(args.verbose);

    if args.config.is_some() && args.brightness.is_some() {
        bail!(
            "`--config` and `--brightness` are mutually exclusive; specify the default brightness in the [general] section instead"
        );
    }

    let mut config = match &args.config {
        Some(path) => match Config::load(path) {
            Ok(config) => config,
            Err(e) => bail!(
                "failed to load configuration from '{}': {e}",
                path.display()
            ),
        },
        None => Config::default(),
    };

    if let Some(b) = args.brightness {
        config.general.brightness = b;
    }

    drop(args);

    let mut service = Service::new(config)?;
    service.run();
}

struct Service {
    leds: Leds,
    input: InputHandler,
    state: bool,
    last_change: Instant,
    timeout: Duration,
}

impl Service {
    fn new(config: Config) -> anyhow::Result<Self> {
        let leds = Leds::from_config(&config)?;
        let filter = if config.inputs.is_empty() {
            DeviceFilter::Auto
        } else {
            DeviceFilter::from_names(config.inputs.into_iter().map(|inp| inp.name))
        };

        let timeout = Duration::from_secs(config.general.timeout.get().into());
        let input = InputHandler::spawn(filter)?;

        Ok(Self {
            leds,
            input,
            state: false,
            last_change: Instant::now(),
            timeout,
        })
    }

    fn run(&mut self) -> ! {
        loop {
            let new_state = match self.input.wait_deadline(self.last_change + self.timeout) {
                Ok(_) => true,
                Err(_) => false,
            };
            self.last_change = Instant::now();

            if self.state != new_state {
                self.state = new_state;
                log::info!("{}", if new_state { "ON" } else { "OFF" });
                if let Err(e) = self.leds.set_state(new_state) {
                    log::error!("failed to set LED brightness: {e}");
                }
            }
        }
    }
}
