// Learn more about Tauri commands at https://tauri.app/develop/calling-rust/
use rfont::{FaceSummary, Font, FontCollection};
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use tauri::State;
use tracing::{error, info};

/// 应用状态
struct AppState {
    /// 当前选中的字面
    font: Option<Arc<Font>>,
    /// 字体集合（TTC/OTC），非集合文件为 None
    collection: Option<Arc<FontCollection>>,
    current_font_path: Option<PathBuf>,
    /// 当前字面在集合中的索引
    selected_face: usize,
}

impl AppState {
    fn new() -> Self {
        Self {
            font: None,
            collection: None,
            current_font_path: None,
            selected_face: 0,
        }
    }
}

/// 加载字体文件
#[tauri::command]
async fn load_font(path: String, state: State<'_, Mutex<AppState>>) -> Result<String, String> {
    info!("Loading font from path: {}", path);

    let font_path = PathBuf::from(&path);

    let data = std::fs::read(&font_path).map_err(|e| format!("Failed to read font file: {}", e))?;

    let (collection, font) = if FontCollection::is_collection(&data) {
        let collection =
            FontCollection::from_bytes(data).map_err(|e| format!("Failed to load font: {}", e))?;
        let font = collection
            .face(0)
            .map_err(|e| format!("Failed to load font: {}", e))?;
        (Some(Arc::new(collection)), Arc::new(font))
    } else {
        let font = Font::from_bytes(&data).map_err(|e| format!("Failed to load font: {}", e))?;
        (None, Arc::new(font))
    };

    let mut app_state = state.lock().unwrap();
    app_state.current_font_path = Some(font_path);
    app_state.collection = collection;
    app_state.font = Some(font);
    app_state.selected_face = 0;

    info!("Font loaded successfully");
    Ok("Font loaded successfully".to_string())
}

/// 列出字体集合中的字面；非集合文件返回空列表
#[tauri::command]
async fn list_faces(state: State<'_, Mutex<AppState>>) -> Result<Vec<FaceSummary>, String> {
    let app_state = state.lock().unwrap();

    match app_state.collection {
        Some(ref collection) => collection
            .faces()
            .map_err(|e| format!("Failed to list faces: {}", e)),
        None => Ok(Vec::new()),
    }
}

/// 切换当前字面
#[tauri::command]
async fn select_face(index: usize, state: State<'_, Mutex<AppState>>) -> Result<(), String> {
    let mut app_state = state.lock().unwrap();

    let collection = app_state
        .collection
        .as_ref()
        .ok_or_else(|| "Current font is not a collection".to_string())?;

    let font = collection
        .face(index)
        .map_err(|e| format!("Failed to select face: {}", e))?;

    app_state.font = Some(Arc::new(font));
    app_state.selected_face = index;

    info!("Selected face index: {}", index);
    Ok(())
}

/// 读取字体文件为 base64
///
/// 字体集合按 `face` 提取对应字面的独立字体字节，否则返回整个文件内容。
#[tauri::command]
async fn read_font_as_base64(path: String, face: usize) -> Result<String, String> {
    use base64::{engine::general_purpose, Engine as _};

    let data = std::fs::read(&path).map_err(|e| format!("Failed to read font file: {}", e))?;

    let bytes = if FontCollection::is_collection(&data) {
        let collection = FontCollection::from_bytes(data)
            .map_err(|e| format!("Failed to read font file: {}", e))?;
        collection
            .face_bytes(face)
            .map_err(|e| format!("Failed to extract face: {}", e))?
    } else if face == 0 {
        data
    } else {
        return Err(format!("Face index {} out of range", face));
    };

    Ok(general_purpose::STANDARD.encode(&bytes))
}

/// 获取字体信息
#[tauri::command]
async fn get_font_info(state: State<'_, Mutex<AppState>>) -> Result<rfont::FontInfo, String> {
    let app_state = state.lock().unwrap();

    if let Some(ref font) = app_state.font {
        font.get_font_info()
            .map_err(|e| format!("Failed to get font info: {}", e))
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
    info!(
        "Subsetting/converting font with text: '{}', format: {}",
        text, output_format
    );

    let app_state = state.lock().unwrap();

    // 使用已加载的字体，避免重复加载
    let font = if let Some(ref font) = app_state.font {
        Arc::clone(font)
    } else if let Some(ref path) = app_state.current_font_path {
        // 如果没有加载，才重新加载
        Arc::new(
            Font::load_with_face(path, app_state.selected_face)
                .map_err(|e| format!("Failed to load font: {}", e))?,
        )
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

        let format: rfont::OutputFormat = rfont::OutputFormat::parse(&output_format)
            .ok_or_else(|| format!("Unsupported output format: {}", output_format))?;

        let mut options = rfont::SubsetOptions {
            output_format: format,
            ..Default::default()
        };

        match options.output_format {
            rfont::OutputFormat::Woff => options.compression_level = 9,
            rfont::OutputFormat::Woff2 => options.compression_level = 11,
            rfont::OutputFormat::Ttf => options.compression_level = 0,
        }

        font.subset_with_options(&glyph_ids, &options)
    };
    match result {
        Ok(font_data) => {
            info!(
                "Output font size: {} bytes, format: {}",
                font_data.len(),
                output_format
            );

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
            read_font_as_base64,
            list_faces,
            select_face
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
