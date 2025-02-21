use druid::widget::{Button, Flex, Label, ProgressBar, TextBox};
use druid::{AppLauncher, Data, Env, Lens, PlatformError, Widget, WidgetExt, WindowDesc};
use image::{ImageBuffer, Rgb, ImageFormat};
use rawloader::decode_file;
use rayon::prelude::*;
use std::fs;
use std::path::Path;
use std::sync::{Arc, Mutex};
use tinyfiledialogs::select_folder_dialog;

#[derive(Clone, Data, Lens)]
struct AppState {
    source_folder: String,
    dest_folder: String,
    status: String,
    progress: f64, // 0.0 to 1.0 for the progress bar
}

fn main() -> Result<(), PlatformError> {
    let main_window = WindowDesc::new(build_ui())
        .title("RawRiser")
        .window_size((400.0, 250.0));

    let initial_state = AppState {
        source_folder: String::new(),
        dest_folder: String::new(),
        status: "Ready".to_string(),
        progress: 0.0,
    };

    AppLauncher::with_window(main_window)
        .launch(initial_state)?;
    Ok(())
}

fn build_ui() -> impl Widget<AppState> {
    let source_label = Label::new("Source Folder:");
    let source_input = TextBox::new()
        .with_placeholder("Select source folder")
        .lens(AppState::source_folder)
        .fix_width(250.0);
    let source_button = Button::<AppState>::new("Browse").on_click(|_, state, _| {
        if let Some(folder) = select_folder_dialog("Select Source Folder", "") {
            state.source_folder = folder;
        }
    });

    let dest_label = Label::new("Destination Folder:");
    let dest_input = TextBox::new()
        .with_placeholder("Select destination folder")
        .lens(AppState::dest_folder)
        .fix_width(250.0);
    let dest_button = Button::<AppState>::new("Browse").on_click(|_, state, _| {
        if let Some(folder) = select_folder_dialog("Select Destination Folder", "") {
            state.dest_folder = folder;
        }
    });

    let convert_button = Button::<AppState>::new("Convert").on_click(|_, state, _| {
        if state.source_folder.is_empty() || state.dest_folder.is_empty() {
            state.status = "Please select both folders".to_string();
            return;
        }
        match convert_images(&state.source_folder, &state.dest_folder) {
            Ok((count, progress)) => {
                state.status = format!("Converted {} images", count);
                state.progress = progress;
            }
            Err(msg) => state.status = msg,
        }
    });

    let status_label = Label::new(|data: &AppState, _env: &Env| data.status.clone());
    let progress_bar = ProgressBar::new().lens(AppState::progress).fix_width(360.0);

    Flex::column()
        .with_child(
            Flex::row()
                .with_child(source_label)
                .with_spacer(10.0)
                .with_child(source_input)
                .with_spacer(10.0)
                .with_child(source_button),
        )
        .with_spacer(20.0)
        .with_child(
            Flex::row()
                .with_child(dest_label)
                .with_spacer(10.0)
                .with_child(dest_input)
                .with_spacer(10.0)
                .with_child(dest_button),
        )
        .with_spacer(20.0)
        .with_child(convert_button)
        .with_spacer(20.0)
        .with_child(status_label)
        .with_spacer(10.0)
        .with_child(progress_bar)
        .padding(20.0)
}

fn convert_images(source: &str, dest: &str) -> Result<(usize, f64), String> {
    let source_path = Path::new(source);
    let dest_path = Path::new(dest);

    if !source_path.is_dir() || !dest_path.is_dir() {
        return Err("Invalid folder path(s)".to_string());
    }

    let entries: Vec<_> = fs::read_dir(source_path)
        .map_err(|e| e.to_string())?
        .filter_map(Result::ok)
        .filter(|e| {
            if let Some(ext) = e.path().extension() {
                let ext = ext.to_string_lossy().to_lowercase();
                ["arw", "nef", "crw"].contains(&ext.as_str())
            } else {
                false
            }
        })
        .collect();

    if entries.is_empty() {
        return Err("No supported RAW files found (ARW, NEF, CRW)".to_string());
    }

    let total_files = entries.len() as f64;
    let processed_count = Arc::new(Mutex::new(0));

    entries.par_iter().for_each(|entry| {
        let path = entry.path();
        let output_file = dest_path.join(format!(
            "{}.jpg",
            path.file_stem().unwrap().to_str().unwrap()
        ));

        if let Ok(raw_image) = decode_file(&path) {
            if let Some(img) = raw_to_image_buffer(&raw_image) {
                if img.save_with_format(&output_file, ImageFormat::Jpeg).is_ok() {
                    let mut count = processed_count.lock().unwrap();
                    *count += 1;
                }
            }
        }
    });

    let final_count = *processed_count.lock().unwrap();
    let final_progress = if total_files > 0.0 { final_count as f64 / total_files } else { 1.0 };
    Ok((final_count, final_progress))
}

fn raw_to_image_buffer(raw_image: &rawloader::RawImage) -> Option<ImageBuffer<Rgb<u8>, Vec<u8>>> {
    match &raw_image.data {
        rawloader::RawImageData::Integer(data) => {
            let mut rgb_data = Vec::with_capacity(raw_image.width as usize * raw_image.height as usize * 3);
            for pixel in data {
                let value = (*pixel as f32 / 65535.0 * 255.0) as u8; // Scale 16-bit to 8-bit
                rgb_data.push(value); // R (simplified, no demosaicing)
                rgb_data.push(value); // G
                rgb_data.push(value); // B
            }
            ImageBuffer::from_raw(raw_image.width as u32, raw_image.height as u32, rgb_data)
        }
        rawloader::RawImageData::Float(_) => None, // Skip floating-point data for simplicity
    }
}