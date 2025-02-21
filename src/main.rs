use druid::widget::{Button, Flex, Label, ProgressBar, TextBox};
use druid::{AppLauncher, Data, Env, Lens, PlatformError, Widget, WidgetExt, WindowDesc};
use image::{ImageBuffer, Rgb};
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
    progress: f64,
}

fn main() -> Result<(), PlatformError> {
    let main_window = WindowDesc::new(|| build_ui()) // Fixed: Added closure
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
    let source_button = Button::new("Browse")
        .on_click(|_ctx, data: &mut AppState, _env| {
            if let Some(folder) = select_folder_dialog("Select Source Folder", "") {
                data.source_folder = folder;
            }
        });

    let dest_label = Label::new("Destination Folder:");
    let dest_input = TextBox::new()
        .with_placeholder("Select destination folder")
        .lens(AppState::dest_folder)
        .fix_width(250.0);
    let dest_button = Button::new("Browse")
        .on_click(|_ctx, data: &mut AppState, _env| {
            if let Some(folder) = select_folder_dialog("Select Destination Folder", "") {
                data.dest_folder = folder;
            }
        });

    let convert_button = Button::new("Convert")
        .on_click(|_ctx, data: &mut AppState, _env| {
            if data.source_folder.is_empty() || data.dest_folder.is_empty() {
                data.status = "Please select both folders".to_string();
                return;
            }
            data.status = "Converting...".to_string();
            data.progress = 0.0;

            match convert_images(&data.source_folder, &data.dest_folder) {
                Ok((count, final_progress)) => {
                    data.status = format!("Converted {} images", count);
                    data.progress = final_progress;
                }
                Err(msg) => {
                    data.status = msg;
                }
            }
        });

    let status_label = Label::new(|data: &AppState, _env: &_| data.status.clone());
    let progress_bar = ProgressBar::new()
        .lens(AppState::progress);

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
                ["arw", "nef", "crw", "cr2", "orf", "rw2"].contains(&ext.as_str())
            } else {
                false
            }
        })
        .collect();

    if entries.is_empty() {
        return Err("No supported RAW files found".to_string());
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
            if let Some(rgb_image) = raw_to_image_buffer(&raw_image) {
                if rgb_image.save(&output_file).is_ok() {
                    let mut count = processed_count.lock().unwrap();
                    *count += 1;
                }
            }
        }
    });

    let final_count = *processed_count.lock().unwrap();
    let final_progress = if total_files > 0.0 {
        final_count as f64 / total_files
    } else {
        1.0
    };

    Ok((final_count, final_progress))
}

fn raw_to_image_buffer(raw_image: &rawloader::RawImage) -> Option<ImageBuffer<Rgb<u8>, Vec<u8>>> {
    let width = raw_image.width as u32;
    let height = raw_image.height as u32;
    
    match &raw_image.data {
        rawloader::RawImageData::Integer(data) => {
            let mut rgb_data = Vec::with_capacity((width * height * 3) as usize);
            
            for y in 0..height {
                for x in 0..width {
                    let idx = (y * width + x) as usize;
                    if idx >= data.len() {
                        continue;
                    }
                    
                    // Convert 16-bit raw data to 8-bit RGB with basic processing
                    let max_value = raw_image.whitelevels[0] as f32;
                    let mut value = (data[idx] as f32 / max_value).min(1.0);
                    
                    // Apply basic gamma correction
                    value = value.powf(1.0/2.2);
                    
                    // Convert to 8-bit
                    let pixel = (value * 255.0) as u8;
                    
                    // Extend with RGB values (grayscale for now)
                    rgb_data.extend_from_slice(&[pixel, pixel, pixel]);
                }
            }
            
            ImageBuffer::from_raw(width, height, rgb_data)
        }
        _ => None,
    }
}