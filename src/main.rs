use druid::widget::{Button, Flex, Label, TextBox, ProgressBar};
use druid::{AppLauncher, Data, Env, Lens, PlatformError, Widget, WidgetExt, WindowDesc};
use indicatif::{ProgressBar as IndicatifProgressBar, ProgressStyle};
use rayon::prelude::*;
use std::fs;
use std::path::Path;
use std::process::Command;
use std::sync::{Arc, Mutex};
use tinyfiledialogs::select_folder_dialog;

#[derive(Clone, Data, Lens)]
struct AppState {
    source_folder: String,
    dest_folder: String,
    status: String,
    progress: f64, // 0.0 to 1.0
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
        convert_images(&state.source_folder, &state.dest_folder, state);
    });

    let status_label = Label::new(|data: &AppState, _env: &Env| data.status.clone());
    let progress_bar = ProgressBar::new().lens(AppState::progress);

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

fn convert_images(source: &str, dest: &str, state: &mut AppState) {
    let source_path = Path::new(source);
    let dest_path = Path::new(dest);

    if !source_path.is_dir() || !dest_path.is_dir() {
        state.status = "Invalid folder path(s)".to_string();
        return;
    }

    let entries: Vec<_> = fs::read_dir(source_path)
        .unwrap()
        .filter_map(Result::ok)
        .filter(|e| e.path().is_file())
        .collect();

    if entries.is_empty() {
        state.status = "No files found".to_string();
        return;
    }

    let total_files = entries.len() as f64;
    let processed_count = Arc::new(Mutex::new(0));

    let pb = IndicatifProgressBar::new(total_files as u64);
    pb.set_style(
        ProgressStyle::default_bar()
            .template("[{elapsed_precise}] {bar:40.cyan/blue} {pos:>7}/{len:7} {msg}")
            .progress_chars("##-"),
    );

    entries.par_iter().for_each(|entry| {
        let path = entry.path();
        let dest_file = dest_path.join(format!(
            "{}.jpg",
            path.file_stem().unwrap().to_str().unwrap()
        ));

        let output = Command::new("dcraw")
            .arg("-c")
            .arg("-e")
            .arg(path.to_str().unwrap())
            .output();

        if let Ok(output) = output {
            if output.status.success() {
                let jpeg_data = output.stdout;
                if fs::write(&dest_file, jpeg_data).is_ok() {
                    let mut count = processed_count.lock().unwrap();
                    *count += 1;
                    pb.inc(1);
                    // Update progress for the GUI
                    let progress = *count as f64 / total_files;
                    let mut state_progress = state.progress; // Access outside closure
                    state_progress = progress; // Update progress
                    state.progress = progress; // Reflect in GUI
                }
            }
        }
    });

    pb.finish_with_message("Conversion complete");

    let final_count = *processed_count.lock().unwrap();
    state.status = format!("Converted {} images", final_count);
    state.progress = 1.0;
}