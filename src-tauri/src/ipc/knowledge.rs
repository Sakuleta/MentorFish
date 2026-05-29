// ─── Knowledge Commands ───

use super::*;
use std::sync::Arc;
use tauri::State;


#[tauri::command]
pub async fn cmd_run_ingestion(
    state: State<'_, Arc<AppState>>,
) -> Result<IngestionReportResponse, String> {
    let knowledge_dir = crate::knowledge::knowledge_dir().to_path_buf();
    crate::knowledge::ingestion::run_ingestion(state.inference.as_ref(), &knowledge_dir)
        .await
        .map(|report| IngestionReportResponse {
            books_processed: report.books_processed,
            chunks_created: report.chunks_created,
            chunks_embedded: report.chunks_embedded,
            message: report.message,
        })
        .map_err(|e| format!("Ingestion error: {}", e))
}

#[tauri::command]
pub async fn cmd_get_knowledge_summary() -> Result<KnowledgeSummary, String> {
    let knowledge_dir = crate::knowledge::knowledge_dir();

    // Try chunks_indexed.json first (has embeddings), then chunks_all.json
    let chunks_path = if knowledge_dir.join("chunks_indexed.json").exists() {
        knowledge_dir.join("chunks_indexed.json")
    } else if knowledge_dir.join("chunks_all.json").exists() {
        knowledge_dir.join("chunks_all.json")
    } else {
        return Ok(KnowledgeSummary {
            total_books: 0,
            total_chunks: 0,
            total_embedded: 0,
            books: vec![],
        });
    };

    // Move blocking file I/O to a dedicated thread to avoid stalling the Tokio runtime
    let path = chunks_path.clone();
    let json = tokio::task::spawn_blocking(move || std::fs::read_to_string(&path))
        .await
        .map_err(|e| format!("Task join error: {}", e))?
        .map_err(|e| format!("Failed to read chunks: {}", e))?;
    let root: serde_json::Value =
        serde_json::from_str(&json).map_err(|e| format!("Invalid JSON: {}", e))?;

    let chunks = root["chunks"]
        .as_array()
        .ok_or_else(|| "Missing 'chunks' array in chunks file".to_string())?;

    // Group chunks by source
    let mut source_map: std::collections::HashMap<String, (u64, String, bool)> =
        std::collections::HashMap::new();
    let mut total_embedded = 0u64;

    for chunk in chunks {
        let source = chunk["source"].as_str().unwrap_or("Unknown").to_string();
        let chunk_type = chunk["chunk_type"]
            .as_str()
            .unwrap_or("unknown")
            .to_string();

        let has_embedding = chunk["embedding"].is_array()
            && !chunk["embedding"]
                .as_array()
                .map(|a| a.is_empty())
                .unwrap_or(true);

        if has_embedding {
            total_embedded += 1;
        }

        let entry = source_map
            .entry(source.clone())
            .or_insert((0, chunk_type, false));
        entry.0 += 1;
        entry.2 = entry.2 || has_embedding;
    }

    let books: Vec<BookSummary> = source_map
        .into_iter()
        .map(|(title, (count, chunk_type, has_embeddings))| BookSummary {
            title,
            chunk_count: count,
            chunk_type,
            has_embeddings,
        })
        .collect();

    Ok(KnowledgeSummary {
        total_books: books.len() as u64,
        total_chunks: chunks.len() as u64,
        total_embedded,
        books,
    })
}

#[tauri::command]
pub async fn cmd_copy_to_knowledge(request: CopyToKnowledgeRequest) -> Result<String, String> {
    let knowledge_dir = crate::knowledge::knowledge_dir();
    let dest_dir = match request.file_type.as_str() {
        "pdf" => knowledge_dir.join("books"),
        "pgn" => knowledge_dir.join("pgn"),
        other => {
            return Err(format!(
                "Invalid file type '{}'. Use 'pdf' or 'pgn'.",
                other
            ))
        }
    };

    std::fs::create_dir_all(&dest_dir).map_err(|e| format!("Failed to create directory: {}", e))?;

    // Sanitize filename: only keep the base name, strip any path components
    let filename = std::path::Path::new(&request.file_name)
        .file_name()
        .ok_or_else(|| "Invalid file name".to_string())?;
    let dest = dest_dir.join(filename);

    // Validate file size (max 50MB) and content type by magic bytes
    const MAX_FILE_SIZE: usize = 50 * 1024 * 1024;
    if request.file_content.len() > MAX_FILE_SIZE {
        return Err(format!(
            "File too large: {} bytes (max {} bytes)",
            request.file_content.len(),
            MAX_FILE_SIZE
        ));
    }
    // Basic magic-byte validation for allowed types
    match request.file_type.as_str() {
        "pgn" if request.file_content.iter().filter(|&&b| b == 0).count() > 10 => {
            return Err("PGN files should not contain binary data".to_string());
        }
        "pdf" if !request.file_content.starts_with(b"%PDF") => {
            return Err("PDF files must start with %PDF header".to_string());
        }
        _ => {}
    }

    let content = request.file_content.clone();
    let dest_display = dest.to_string_lossy().to_string();
    tokio::task::spawn_blocking(move || std::fs::write(&dest, &content))
        .await
        .map_err(|e| format!("Task join error: {}", e))?
        .map_err(|e| format!("Failed to write file: {}", e))?;

    log::info!(
        "Imported {} file: {} → {}",
        request.file_type,
        request.file_name,
        dest_display
    );

    Ok(dest_display)
}

// ─── Book Reader Commands (Section 3.7) ───

#[tauri::command]
pub async fn cmd_get_book_chunks(
    request: GetBookChunksRequest,
) -> Result<GetBookChunksResponse, String> {
    let all = crate::knowledge::retrieve_by_source(&request.source, 100_000)
        .await
        .map_err(|e| format!("Failed to retrieve book chunks: {}", e))?;

    Ok(GetBookChunksResponse { chunks: all.chunks })
}
