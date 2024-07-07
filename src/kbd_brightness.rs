use anyhow::Result;
use log::{debug, info};
use logind_zbus::session::SessionProxyBlocking;

use crate::read_value;

pub(crate) struct KBDBrightness<'a> {
    proxy: &'a SessionProxyBlocking<'a>,
    subsystem: &'a str,
    name: &'a str,
}

impl<'a> KBDBrightness<'a> {
    pub(crate) fn new(
        proxy: &'a SessionProxyBlocking<'a>,
        subsystem: &'a str,
        name: &'a str,
    ) -> Self {
        Self {
            proxy,
            subsystem,
            name,
        }
    }

    fn read(&self) -> Result<u32> {
        read_value(&format!(
            "/sys/class/{}/{}/brightness",
            self.subsystem, self.name
        ))
    }

    pub(crate) fn adjust(&self, new_val: u32) -> Result<()> {
        let new_level = match new_val {
            v if v < 55 => 1,
            v if v < 65 => 2,
            v if v < 70 => 3,
            v if v < 75 => 2,
            v if v < 80 => 1,
            _ => 0,
        };

        let cur_brightness = self.read()?;

        debug!(
            "KBD: value:{:?}, level:{:?}, current:{:?}",
            new_val, new_level, cur_brightness
        );
        if cur_brightness != new_level {
            info!(
                "Adjusting KBD Backlight: val:{:?} old:{:?} new:{:?}",
                new_val, cur_brightness, new_level
            );
            self.proxy
                .set_brightness(self.subsystem, self.name, new_level)?;
        }

        Ok(())
    }
}
