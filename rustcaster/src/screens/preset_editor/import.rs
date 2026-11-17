// Texture import: native file-open dialog (rfd, blocking) + validation and
// normalisation of the picked image for a preset slot.

use std::path::{Path, PathBuf};

use crate::assets::preset::TextureSlot;
use crate::assets::texture::Texture;
use crate::render::color::Color;

/// Extensions offered by the dialog (all decoded by the `image` crate).
pub const IMAGE_EXTENSIONS: [&str; 6] = ["png", "jpg", "jpeg", "bmp", "gif", "tga"];
/// Files bigger than this are refused before decoding.
const MAX_FILE_BYTES: u64 = 64 * 1024 * 1024;
/// Imported images are downscaled so no side exceeds this (sky: twice as wide).
pub const MAX_TEXTURE_SIDE: usize = 256;
const MAX_SKY_SIDE: usize = 512;

/// Opens the native "open file" dialog (blocks until closed). `None` if cancelled.
pub fn pick_image(slot: TextureSlot, start_dir: Option<&Path>) -> Option<PathBuf> {
    // The game window must not minimise while the dialog has the focus.
    sdl2::hint::set("SDL_VIDEO_MINIMIZE_ON_FOCUS_LOSS", "0");
    let mut dialog = rfd::FileDialog::new()
        .set_title(format!("Import {} texture", slot.label().to_lowercase()))
        .add_filter("Images", &IMAGE_EXTENSIONS)
        .add_filter("All files", &["*"]);
    if let Some(dir) = start_dir.filter(|d| d.is_dir()) {
        dialog = dialog.set_directory(dir);
    }
    // Owned by the game window: shown above it (even when it is topmost
    // fullscreen) and modal, so clicks cannot reach the game meanwhile.
    if let Some(owner) = owner::OwnerWindow::active() {
        dialog = dialog.set_parent(&owner);
    }
    dialog.pick_file()
}

/// Result of a successful import.
pub struct Imported {
    pub texture: Texture,
    /// Original size, when it was downscaled.
    pub downscaled_from: Option<(usize, usize)>,
}

/// Loads `path` for `slot`: size checks, downscale, and for the opaque slots
/// (wall, floor, ceiling, sky) transparency is flattened over `fallback`.
/// Grid, coin and end keep their alpha (see-through grids, sprites).
pub fn load_for_slot(path: &Path, slot: TextureSlot, fallback: Color) -> Result<Imported, String> {
    let name = path.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
    let meta = std::fs::metadata(path).map_err(|e| format!("{}: {}", name, e))?;
    if meta.len() > MAX_FILE_BYTES {
        return Err(format!("{} is too big ({} MB)", name, meta.len() / (1024 * 1024)));
    }
    let texture = Texture::load(path).map_err(|_| format!("{} is not a supported image", name))?;
    if texture.width == 0 || texture.height == 0 {
        return Err(format!("{} is empty", name));
    }
    let max_side = if slot == TextureSlot::Sky { MAX_SKY_SIDE } else { MAX_TEXTURE_SIDE };
    let original = (texture.width, texture.height);
    let mut texture = texture.fit_within(max_side);
    let downscaled_from = if (texture.width, texture.height) != original { Some(original) } else { None };
    if !keeps_alpha(slot) && texture.has_transparency() {
        texture = texture.flattened(fallback);
    }
    Ok(Imported { texture, downscaled_from })
}

/// Slots drawn with transparency by the renderer.
pub fn keeps_alpha(slot: TextureSlot) -> bool {
    matches!(slot, TextureSlot::Grid | TextureSlot::Coin | TextureSlot::End)
}

#[cfg(windows)]
mod owner {
    use std::num::NonZeroIsize;

    use raw_window_handle::{
        DisplayHandle, HandleError, HasDisplayHandle, HasWindowHandle, RawWindowHandle, Win32WindowHandle, WindowHandle,
    };

    #[link(name = "user32")]
    extern "system" {
        // HWND of the calling thread's active window (the SDL window), or null.
        fn GetActiveWindow() -> isize;
    }

    pub struct OwnerWindow(NonZeroIsize);

    impl OwnerWindow {
        pub fn active() -> Option<OwnerWindow> {
            // SAFETY: no arguments, only reads the thread's window state.
            NonZeroIsize::new(unsafe { GetActiveWindow() }).map(OwnerWindow)
        }
    }

    impl HasWindowHandle for OwnerWindow {
        fn window_handle(&self) -> Result<WindowHandle<'_>, HandleError> {
            let raw = RawWindowHandle::Win32(Win32WindowHandle::new(self.0));
            // SAFETY: the SDL window outlives the (blocking) dialog using it.
            Ok(unsafe { WindowHandle::borrow_raw(raw) })
        }
    }

    impl HasDisplayHandle for OwnerWindow {
        fn display_handle(&self) -> Result<DisplayHandle<'_>, HandleError> {
            Ok(DisplayHandle::windows())
        }
    }
}

#[cfg(not(windows))]
mod owner {
    use raw_window_handle::{DisplayHandle, HandleError, HasDisplayHandle, HasWindowHandle, WindowHandle};

    /// No parent window outside Windows (the game only ships for Windows).
    pub struct OwnerWindow;

    impl OwnerWindow {
        pub fn active() -> Option<OwnerWindow> {
            None
        }
    }

    impl HasWindowHandle for OwnerWindow {
        fn window_handle(&self) -> Result<WindowHandle<'_>, HandleError> {
            Err(HandleError::Unavailable)
        }
    }

    impl HasDisplayHandle for OwnerWindow {
        fn display_handle(&self) -> Result<DisplayHandle<'_>, HandleError> {
            Err(HandleError::Unavailable)
        }
    }
}
