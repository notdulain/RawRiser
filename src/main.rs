use druid::widget::{Button, Flex, Label, TextBox};
use druid::{AppLauncher, Data, Env, Lens, PlatformError, Widget, WidgetExt, WindowDesc};
use image::{DynamicImage, ImageBuffer, Rgb, ImageOutputFormat};
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
}

fn main() -> Result<(), PlatformError> {
    let main_window = WindowDesc::new(build_ui())
        .title("RawRiser")
        .window_size((400.0, 200.0));

    let initial_state = AppState {
        source_folder: String::new(),
        dest_folder: String::new(),
        status: "Ready".to_string(),
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
            Ok(count) => state.status = format!("Converted {} images", count),
            Err(msg) => state.status = msg,
        }
    });

    let status_label = Label::new(|data: &AppState, _env: &Env| data.status.clone());

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
        .padding(20.0)
}

fn convert_images(source: &str, dest: &str) -> Result<usize, String> {
    let source_path = Path::new(source);
    let dest_path = Path::new(dest);

    // Check if paths are valid directories
    if !source_path.is_dir() || !dest_path.is_dir() {
        return Err("Invalid folder path(s)".to_string());
    }

    // Collect all files in the source directory
    let entries: Vec<_> = fs::read_dir(source_path)
        .map_err(|e| e.to_string())?
        .filter_map(Result::ok)
        .filter(|e| e.path().is_file())
        .collect();

    if entries.is_empty() {
        return Err("No files found".to_string());
    }

    // Thread-safe counter for successful conversions
    let processed_count = Arc::new(Mutex::new(0));

    // Process files in parallel
    entries.par_iter().for_each(|entry| {
        let path = entry.path();
        if let Ok(raw_img) = decode_file(&path) {
            // Attempt to convert raw data to RGB ImageBuffer
            if let Ok(img_buffer) = convert_raw_to_rgb(&raw_img) {
                let dynamic_img = DynamicImage::ImageRgb8(img_buffer);
                let dest_file = dest_path.join(format!(
                    "{}.jpg",
                    path.file_stem().unwrap().to_str().unwrap()
                ));
                if let Ok(mut file) = fs::File::create(&dest_file) {
                    if dynamic_img
                        .write_to(&mut file, ImageOutputFormat::Jpeg(85))
                        .is_ok()
                    {
                        let mut count = processed_count.lock().unwrap();
                        *count += 1;
                    }
                }
            }
        }
    });

    let final_count = *processed_count.lock().unwrap();
    Ok(final_count)
}

// Helper function to convert RawImage data to an RGB ImageBuffer
fn convert_raw_to_rgb(raw_img: &rawloader::RawImage) -> Result<ImageBuffer<Rgb<u8>, Vec<u8>>, String> {
    match &raw_img.data {
        rawloader::RawImageData::Integer(data) => {
            // Assume 16-bit data, scale to 8-bit RGB (simplified, no demosaicing)
            let mut rgb_data = Vec::with_capacity(raw_img.width as usize * raw_img.height as usize * 3);
            for pixel in data {
                let value = (pixel / 256) as u8; // Scale 16-bit to 8-bit
                rgb_data.push(value); // R
                rgb_data.push(value); // G
                rgb_data.push(value); // B
            }
            ImageBuffer::from_vec(raw_img.width as u32, raw_img.height as u32, rgb_data)
                .ok_or("Failed to create RGB buffer".to_string())
        }
        rawloader::RawImageData::Float(_) => Err("Floating-point RAW data not supported".to_string()),
    }
}