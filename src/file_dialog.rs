use std::path::PathBuf;

#[cfg(windows)]
pub fn choose_image(title_text: &str, photos: bool) -> Result<Option<PathBuf>, String> {
    use std::os::windows::ffi::OsStringExt;
    use windows_sys::Win32::UI::Controls::Dialogs::*;
    let mut path = vec![0u16; 32768];
    let filter: Vec<u16> = if photos {
        "Photos (*.png;*.jpg;*.jpeg)\0*.png;*.jpg;*.jpeg\0\0"
    } else {
        "PNG images (*.png)\0*.png\0\0"
    }
    .encode_utf16()
    .collect();
    let title: Vec<u16> = title_text
        .encode_utf16()
        .chain(std::iter::once(0))
        .collect();
    let mut dialog: OPENFILENAMEW = unsafe { std::mem::zeroed() };
    dialog.lStructSize = std::mem::size_of::<OPENFILENAMEW>() as u32;
    dialog.hwndOwner =
        unsafe { windows_sys::Win32::UI::WindowsAndMessaging::GetForegroundWindow() };
    dialog.lpstrFilter = filter.as_ptr();
    dialog.lpstrTitle = title.as_ptr();
    dialog.lpstrFile = path.as_mut_ptr();
    dialog.nMaxFile = path.len() as u32;
    dialog.Flags = OFN_FILEMUSTEXIST | OFN_PATHMUSTEXIST | OFN_NOCHANGEDIR | OFN_EXPLORER;
    if unsafe { GetOpenFileNameW(&mut dialog) } != 0 {
        let length = path.iter().position(|c| *c == 0).unwrap_or(path.len());
        Ok(Some(std::ffi::OsString::from_wide(&path[..length]).into()))
    } else {
        let error = unsafe { CommDlgExtendedError() };
        if error == 0 {
            Ok(None)
        } else {
            Err(format!("Cannot open file chooser ({error})."))
        }
    }
}
#[cfg(not(windows))]
pub fn choose_image(title_text: &str, photos: bool) -> Result<Option<PathBuf>, String> {
    let extensions = if photos {
        vec!["png", "jpg", "jpeg"]
    } else {
        vec!["png"]
    };
    Ok(rfd::FileDialog::new()
        .set_title(title_text)
        .add_filter("Images", &extensions)
        .pick_file())
}

#[cfg(windows)]
pub fn choose_backup(save: bool) -> Result<Option<PathBuf>, String> {
    use std::os::windows::ffi::OsStringExt;
    use windows_sys::Win32::UI::Controls::Dialogs::*;
    let mut path = vec![0u16; 32768];
    if save {
        for (target, value) in path
            .iter_mut()
            .zip("lab-manager-backup.json".encode_utf16())
        {
            *target = value;
        }
    }
    let filter: Vec<u16> = "Lab Manager backup (*.json)\0*.json\0\0"
        .encode_utf16()
        .collect();
    let title: Vec<u16> = if save {
        "Export Lab Manager backup"
    } else {
        "Choose backup to restore"
    }
    .encode_utf16()
    .chain(std::iter::once(0))
    .collect();
    let extension: Vec<u16> = "json\0".encode_utf16().collect();
    let mut dialog: OPENFILENAMEW = unsafe { std::mem::zeroed() };
    dialog.lStructSize = std::mem::size_of::<OPENFILENAMEW>() as u32;
    dialog.hwndOwner =
        unsafe { windows_sys::Win32::UI::WindowsAndMessaging::GetForegroundWindow() };
    dialog.lpstrFilter = filter.as_ptr();
    dialog.lpstrTitle = title.as_ptr();
    dialog.lpstrFile = path.as_mut_ptr();
    dialog.nMaxFile = path.len() as u32;
    dialog.lpstrDefExt = extension.as_ptr();
    dialog.Flags = OFN_PATHMUSTEXIST
        | OFN_NOCHANGEDIR
        | OFN_EXPLORER
        | if save {
            OFN_OVERWRITEPROMPT
        } else {
            OFN_FILEMUSTEXIST
        };
    let accepted = unsafe {
        if save {
            GetSaveFileNameW(&mut dialog)
        } else {
            GetOpenFileNameW(&mut dialog)
        }
    };
    if accepted != 0 {
        let length = path.iter().position(|c| *c == 0).unwrap_or(path.len());
        Ok(Some(std::ffi::OsString::from_wide(&path[..length]).into()))
    } else {
        let error = unsafe { CommDlgExtendedError() };
        if error == 0 {
            Ok(None)
        } else {
            Err(format!("Cannot open backup file chooser ({error})."))
        }
    }
}
#[cfg(not(windows))]
pub fn choose_backup(save: bool) -> Result<Option<PathBuf>, String> {
    let dialog = rfd::FileDialog::new().add_filter("Lab Manager backup", &["json"]);
    Ok(if save {
        dialog
            .set_title("Export Lab Manager backup")
            .set_file_name("lab-manager-backup.json")
            .save_file()
    } else {
        dialog.set_title("Choose backup to restore").pick_file()
    })
}
