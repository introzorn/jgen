#![cfg_attr(target_os = "windows", windows_subsystem = "windows")]

use clap::Parser;
use eframe::NativeOptions;
use egui::{IconData, ViewportBuilder};
use env_logger::Env;
use image::{DynamicImage, ImageFormat};
use jgenesis_gui::app::{App, ConfigInfo, LauncherOverrides, LoadAtStartup};
use jgenesis_native_config::AppConfig;
use jgenesis_native_config::paths::{ConfigDirs, ConfigWithPath};
use std::path::PathBuf;

#[derive(Debug, Parser)]
struct Args {
    /// Use a specific config file path instead of the default path of 'jgenesis-config.toml'
    #[arg(long = "config")]
    config_path: Option<PathBuf>,

    /// If set, the GUI will open this file immediately after starting up, and the GUI will exit
    /// when the emulator window is closed
    #[arg(long = "file-path", short = 'f')]
    startup_file_path: Option<PathBuf>,

    /// In combination with -f, attempt to load the specified save state when launching the game.
    /// This arg has no effect if -f/--file-path is not set
    #[arg(long, value_name = "SLOT")]
    load_save_state: Option<usize>,

    /// Hide the settings GUI window. Requires -f/--file-path. The application exits when the
    /// emulator window is closed.
    #[arg(long, requires = "startup_file_path")]
    hide_gui: bool,

    /// Emulator window X position in pixels; negative values are allowed (e.g. --x=-1920 for a left monitor)
    #[arg(long, allow_hyphen_values = true, value_name = "PIXELS")]
    x: Option<i32>,

    /// Emulator window Y position in pixels; negative values are allowed
    #[arg(long, allow_hyphen_values = true, value_name = "PIXELS")]
    y: Option<i32>,

    /// Emulator window width in pixels
    #[arg(long)]
    width: Option<u32>,

    /// Emulator window height in pixels
    #[arg(long)]
    height: Option<u32>,

    /// Audio output device SDL instance id or name substring
    #[arg(long, value_name = "DEVICE")]
    audio_device: Option<String>,

    /// Launch the emulator in fullscreen after applying --x/--y window placement (overrides config)
    #[arg(long, default_value_t = false, action = clap::ArgAction::SetTrue)]
    fullscreen: bool,

    /// Print version string and immediately exit
    #[arg(short = 'v', long, default_value_t = false, action = clap::ArgAction::SetTrue)]
    version: bool,
}

impl Args {
    fn fix_appimage_relative_paths(mut self) -> Self {
        if let Some(config_path) = self.config_path {
            self.config_path = Some(jgenesis_common::fix_appimage_relative_path(config_path));
        }

        if let Some(startup_file_path) = self.startup_file_path {
            self.startup_file_path =
                Some(jgenesis_common::fix_appimage_relative_path(startup_file_path));
        }

        self
    }

    fn load_at_startup(&self) -> Option<LoadAtStartup> {
        self.startup_file_path.as_ref().map(|file_path| LoadAtStartup {
            file_path: file_path.clone(),
            load_state_slot: self.load_save_state,
        })
    }

    fn launcher_overrides(&self) -> LauncherOverrides {
        LauncherOverrides {
            window_x: self.x,
            window_y: self.y,
            window_width: self.width,
            window_height: self.height,
            audio_output_device: self.audio_device.clone(),
        }
    }
}

// Attempt to detect if the application is running on a Steam Deck, and if it is then override
// the winit scale factor to 1. It defaults to 4.5 on the Steam Deck which results in the GUI
// being completely unusable.
#[cfg(all(unix, not(target_os = "macos")))]
fn steam_deck_dpi_hack() {
    let Ok(mut xhandle) = xrandr::XHandle::open() else {
        return;
    };
    let Ok(monitors) = xhandle.monitors() else {
        return;
    };

    if monitors.len() != 1 {
        return;
    }

    let monitor = &monitors[0];

    if monitor.width_px != 1280 || monitor.height_px != 800 || monitor.outputs.len() != 1 {
        return;
    }

    let output = &monitor.outputs[0];

    let Some(edid) = output.properties.iter().find_map(|(_, property)| match &property.value {
        xrandr::Value::Edid(edid) => Some(edid),
        _ => None,
    }) else {
        return;
    };

    // Display name part of the EDID is always here on the Steam Deck: 'ANX7530 U<LF>'
    if edid[75..87] == [0xFC, 0x00, 0x41, 0x4E, 0x58, 0x37, 0x35, 0x33, 0x30, 0x20, 0x55, 0x0A] {
        log::info!(
            "It looks like this is a Steam Deck; overriding winit scale factor to 1 as otherwise it will default to 4.5"
        );

        // SAFETY: This function is only called during initialization, before spawning any threads
        unsafe {
            std::env::set_var("WINIT_X11_SCALE_FACTOR", "1");
        }
    }
}

fn initial_gui_size(config: &AppConfig) -> (f32, f32) {
    (
        f32_max(jgenesis_native_config::DEFAULT_GUI_WIDTH, config.gui_window_width),
        f32_max(jgenesis_native_config::DEFAULT_GUI_HEIGHT, config.gui_window_height),
    )
}

fn f32_max(value: f32, max: f32) -> f32 {
    if value < max { max } else { value }
}

fn load_icon() -> DynamicImage {
    const ICON: &[u8] = include_bytes!("../../256x256.png");

    image::load_from_memory_with_format(ICON, ImageFormat::Png).expect("Failed to load GUI icon")
}

fn main() -> eframe::Result<()> {
    env_logger::Builder::from_env(
        Env::default().default_filter_or(jgenesis_common::DEFAULT_LOG_FILTER),
    )
    .init();

    let args = Args::parse().fix_appimage_relative_paths();

    if args.version {
        println!("{}", env!("CARGO_PKG_VERSION"));
        return Ok(());
    }

    #[cfg(all(unix, not(target_os = "macos")))]
    steam_deck_dpi_hack();

    let config_dirs = ConfigDirs::new();
    let config_dir_type = config_dirs.default_dir_type(args.config_path.clone());
    let config_with_path = ConfigWithPath::load_from_dir_or_default(
        &config_dirs,
        &config_dir_type,
        AppConfig::default,
    );

    if let Some(file_path) = &args.startup_file_path {
        log::info!("Will open file '{}' after starting", file_path.display());
    }

    if args.hide_gui {
        log::info!("Settings GUI will be hidden; application will exit when emulator closes");
    }

    if args.x.is_some() ^ args.y.is_some() {
        log::warn!("Both --x and --y should be set for custom emulator window placement");
    }

    let (gui_width, gui_height) = initial_gui_size(&config_with_path.config);

    let icon = load_icon();
    let icon_width = icon.width();
    let icon_height = icon.height();

    let mut viewport = ViewportBuilder::default()
        .with_inner_size([gui_width, gui_height])
        .with_icon(IconData { rgba: icon.into_bytes(), width: icon_width, height: icon_height });

    if args.hide_gui {
        viewport = viewport
            .with_visible(false)
            .with_taskbar(false)
            .with_decorations(false)
            .with_inner_size([1.0, 1.0])
            .with_resizable(false)
            .with_active(false);
    }

    let options = NativeOptions { viewport, ..NativeOptions::default() };

    let config_info = ConfigInfo {
        initial_config: config_with_path.config,
        config_path: config_with_path.path,
        config_dirs,
        config_dir_type,
    };
    let load_at_startup = args.load_at_startup();
    let launcher_overrides = args.launcher_overrides();
    let hide_gui = args.hide_gui;

    eframe::run_native(
        "jgenesis",
        options,
        Box::new(|cc| {
            Ok(Box::new(App::new(
                config_info,
                load_at_startup,
                launcher_overrides,
                hide_gui,
                cc.egui_ctx.clone(),
            )))
        }),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn load_icon_does_not_panic() {
        let _ = load_icon();
    }
}
