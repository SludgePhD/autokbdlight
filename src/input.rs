//! Opens matching `evdev` devices and receives input events.

use std::{
    io, process,
    sync::{
        Arc,
        mpsc::{self, RecvTimeoutError, SyncSender, sync_channel},
    },
    thread,
    time::{Duration, Instant},
};

use evdevil::{
    Evdev, InputProp,
    enumerate::EnumerateHotplug,
    event::{InputEvent, Key},
};

fn is_keyboard_or_trackpad(dev: &Evdev) -> io::Result<bool> {
    if dev.key_repeat()?.is_some() {
        // Keyboards support automatic key repeat.
        return Ok(true);
    }

    // Trackpads are detected by looking for BTN_TOUCH, and
    // differentiated from touchscreens and drawing tablets by
    // checking for PROP_DIRECT.
    let keys = dev.supported_keys()?;
    if keys.contains(Key::BTN_TOUCH) && !dev.props()?.contains(InputProp::DIRECT) {
        return Ok(true);
    }

    Ok(false)
}

#[derive(Clone)]
pub enum DeviceFilter {
    Names(Arc<[String]>),
    Auto,
}

impl DeviceFilter {
    pub fn from_names(names: impl IntoIterator<Item = String>) -> Self {
        Self::Names(names.into_iter().collect())
    }
}

pub struct InputHandler {
    recv: mpsc::Receiver<()>,
}

impl InputHandler {
    pub fn spawn(filter: DeviceFilter) -> io::Result<Self> {
        // Exit early if we don't have permission to open evdevs.
        if let Some(Err(e)) = evdevil::enumerate()?.next() {
            return Err(e);
        }

        let enumerate = evdevil::enumerate_hotplug()?;
        let (sender, recv) = sync_channel(1);
        thread::Builder::new()
            .name("hotplug".into())
            .spawn(move || {
                hotplug_thread(enumerate, &filter, &sender);
                log::error!("hotplug thread exited unexpectedly; exiting");
                process::exit(1);
            })?;

        Ok(Self { recv })
    }

    pub fn wait_deadline(&self, deadline: Instant) -> Result<(), RecvTimeoutError> {
        let dur = deadline.saturating_duration_since(Instant::now());
        self.recv.recv_timeout(dur)
    }
}

fn hotplug_thread(enumerate: EnumerateHotplug, filter: &DeviceFilter, sender: &SyncSender<()>) {
    for res in enumerate {
        let dev = match res {
            Ok(dev) => dev,
            Err(e) => {
                log::error!("failed to open device: {e}");
                continue;
            }
        };

        match maybe_open_dev(dev, &filter, sender) {
            Ok(()) => {}
            Err(e) => log::error!("failed to query device: {e}"),
        }
    }
}

fn maybe_open_dev(dev: Evdev, filter: &DeviceFilter, sender: &SyncSender<()>) -> io::Result<()> {
    let interest = match filter {
        DeviceFilter::Names(items) => items.contains(&dev.name()?),
        DeviceFilter::Auto => is_keyboard_or_trackpad(&dev)?,
    };

    if !interest {
        return Ok(());
    }

    dev.set_nonblocking(true)?;

    let name = dev.name()?;
    let sender = sender.clone();
    thread::Builder::new()
        .name(name.clone())
        .spawn(move || -> anyhow::Result<()> {
            log::info!("opened '{}' ({})", name, dev.path().display());
            let mut buf = [InputEvent::zeroed(); 32];
            loop {
                dev.block_until_readable()?;

                if sender.send(()).is_err() {
                    return Ok(());
                }

                // Drain the kernel buffer so that we don't immediately loop again.
                for _ in 0..16 {
                    match dev.read_events(&mut buf) {
                        Ok(0) => break,
                        Ok(_) => {}
                        Err(e) if e.kind() == io::ErrorKind::WouldBlock => break,
                        Err(e) => return Err(e.into()),
                    }
                }

                // Devices like trackpads send a lot of events, which would keep
                // this loop unnecessarily busy, so we sleep a bit.
                // The backlight timeout is at least 1 second, so we have to
                // notify the main thread once every second at minimum.
                // This may overflow the kernel buffer, but that doesn't really
                // matter, since we only care about the presence of events,
                // not their content.
                thread::sleep(Duration::from_millis(350));
            }
        })?;

    Ok(())
}
