#![cfg(target_os = "windows")]

mod clipboard;
mod destination_list;
mod direct_manipulation;
#[cfg(not(feature = "wgpu"))]
mod direct_write;
#[cfg(not(feature = "wgpu"))]
mod directx_atlas;
#[cfg(not(feature = "wgpu"))]
mod directx_devices;
#[cfg(not(feature = "wgpu"))]
mod directx_renderer;
mod dispatcher;
mod display;
mod events;
mod keyboard;
mod platform;
mod system_settings;
mod util;
mod vsync;
mod window;
mod wrapper;

pub(crate) use clipboard::*;
pub(crate) use destination_list::*;
#[cfg(not(feature = "wgpu"))]
pub(crate) use direct_write::*;
#[cfg(not(feature = "wgpu"))]
pub(crate) use directx_atlas::*;
#[cfg(not(feature = "wgpu"))]
pub(crate) use directx_devices::*;
#[cfg(not(feature = "wgpu"))]
pub(crate) use directx_renderer::*;
pub(crate) use dispatcher::*;
pub(crate) use display::*;
pub(crate) use events::*;
pub(crate) use keyboard::*;
pub(crate) use platform::*;
pub(crate) use system_settings::*;
pub(crate) use util::*;
pub(crate) use vsync::*;
pub(crate) use window::*;
pub(crate) use wrapper::*;

pub use platform::WindowsPlatform;

pub(crate) use windows::Win32::Foundation::HWND;
