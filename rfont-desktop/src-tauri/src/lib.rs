// Learn more about Tauri commands at https://tauri.app/develop/calling-rust/
use rfont::Font;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use tauri::State;
use tracing::{error, info, warn};


/// 应用状态
struct AppState {
    font: Option<Arc<Font>>,
    current_font_path: Option<PathBuf>,
}

impl AppState {
    fn new() -> Self {
        Self {
            font: None,
            current_font_path: None,
        }
    }
}

/// 加载字体文件
#[tauri::command]
async fn load_font(path: String, state: State<'_, Mutex<AppState>>) -> Result<String, String> {
    info!("Loading font from path: {}", path);

    let font_path = PathBuf::from(&path);

    // 验证字体是否可以加载
    match Font::load(path.as_str()) {
        Ok(font) => {
            let mut app_state = state.lock().unwrap();
            app_state.current_font_path = Some(font_path);
            app_state.font = Some(Arc::new(font));
            info!("Font loaded successfully");
            Ok("Font loaded successfully".to_string())
        }
        Err(e) => {
            error!("Failed to load font: {}", e);
            Err(format!("Failed to load font: {}", e))
        }
    }
}

/// 读取字体文件为 base64
#[tauri::command]
async fn read_font_as_base64(path: String) -> Result<String, String> {
    use base64::{engine::general_purpose, Engine as _};
    use std::fs;

    match fs::read(&path) {
        Ok(data) => {
            let base64_data = general_purpose::STANDARD.encode(&data);
            Ok(base64_data)
        }
        Err(e) => Err(format!("Failed to read font file: {}", e)),
    }
}

/// 获取字体信息
#[tauri::command]
async fn get_font_info(state: State<'_, Mutex<AppState>>) -> Result<serde_json::Value, String> {
    let app_state = state.lock().unwrap();

    if let Some(ref font) = app_state.font {
        match font.get_font_info() {
            Ok(info) => {
                let json = serde_json::json!({
                    "family": info.family_name.unwrap_or_default(),
                    "style": info.style_name.unwrap_or_default(),
                    "version": info.version.unwrap_or_default(),
                    "units_per_em": info.units_per_em,
                    "glyph_count": info.glyph_count,
                });
                Ok(json)
            }
            Err(e) => Err(format!("Failed to get font info: {}", e)),
        }
    } else {
        Err("No font loaded".to_string())
    }
}

/// 执行字体子集化或格式转换
#[tauri::command]
async fn subset_font(
    text: String,
    output_path: String,
    output_format: String,
    state: State<'_, Mutex<AppState>>,
) -> Result<String, String> {
    info!("Subsetting/converting font with text: '{}', format: {}", text, output_format);

    let app_state = state.lock().unwrap();

    // 使用已加载的字体，避免重复加载
    let font = if let Some(ref font) = app_state.font {
        Arc::clone(font)
    } else if let Some(ref path) = app_state.current_font_path {
        // 如果没有加载，才重新加载
        let font_path = path.to_str().ok_or("Invalid font path")?;
        Arc::new(Font::load(font_path).map_err(|e| format!("Failed to load font: {}", e))?)
    } else {
        return Err("No font loaded".to_string());
    };

    let result = if text.is_empty() {
        info!("Text is empty, performing direct format conversion");
        // 直接格式转换，不提取字形
        let compression_level = match output_format.as_str() {
            "woff" => 9,
            "woff2" => 11,
            _ => 0,
        };
        font.convert_format(&output_format, compression_level)
    } else {
        info!("Subsetting font for text: {}", text);
        let glyph_ids = font.get_glyph_ids_for_text(&text);
        
        if glyph_ids.is_empty() {
            return Err("No glyphs found for the specified text".to_string());
        }
        
        let mut options = rfont::SubsetOptions::default();
        options.output_format = output_format.clone();
        
        match output_format.as_str() {
            "woff" => options.compression_level = 9,
            "woff2" => options.compression_level = 11,
            _ => options.compression_level = 0,
        }
        
        font.subset_with_options(&glyph_ids, &options)
    };
    match result {
        Ok(font_data) => {
            info!("Output font size: {} bytes, format: {}", font_data.len(), output_format);

            if font_data.len() < 12 {
                error!("Font data too small: {} bytes", font_data.len());
                return Err("Generated font data is too small to be valid".to_string());
            }

            if font_data.len() >= 12 {
                let num_tables = u16::from_be_bytes([font_data[4], font_data[5]]);
                info!("Number of tables in output font: {}", num_tables);

                let expected_min_size = 12 + (num_tables as usize * 16);
                if font_data.len() < expected_min_size {
                    error!(
                        "Font data too small: have {} bytes, need at least {} bytes for {} tables",
                        font_data.len(),
                        expected_min_size,
                        num_tables
                    );
                    return Err(format!(
                        "Generated font data is incomplete: {} bytes (expected at least {})",
                        font_data.len(),
                        expected_min_size
                    ));
                }
            }

            let output_path_buf = PathBuf::from(&output_path);

            match std::fs::write(&output_path_buf, &font_data) {
                Ok(_) => {
                    info!("Font saved to: {:?}", output_path_buf);
                    Ok(format!("Font saved successfully to {}", output_path))
                }
                Err(e) => {
                    error!("Failed to save font: {}", e);
                    Err(format!("Failed to save font: {}", e))
                }
            }
        }
        Err(e) => {
            error!("Failed to process font: {}", e);
            Err(format!("Failed to process font: {}", e))
        }
    }
}

pub fn run() {
    tracing_subscriber::fmt::init();

    info!("Starting rfont desktop application");

    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .manage(Mutex::new(AppState::new()))
        .invoke_handler(tauri::generate_handler![
            load_font,
            get_font_info,
            subset_font,
            read_font_as_base64
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
