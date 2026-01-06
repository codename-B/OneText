//! PDF export functionality using krilla.

use krilla::color::rgb;
use krilla::geom::{Point, PathBuilder};
use krilla::num::NormalizedF32;
use krilla::page::PageSettings;
use krilla::paint::Fill;
use krilla::text::{Font, TextDirection};
use krilla::Document;
use std::path::Path;
use tracing::info;

/// PDF export configuration.
pub struct PdfConfig {
    /// Font size in points.
    pub font_size: f32,
    /// Page margins in points.
    pub margin: f32,
    /// Header text (filename + date).
    pub header: Option<String>,
    /// Background color as RGB (0-255).
    pub background_rgb: (u8, u8, u8),
    /// Text color as RGB (0-255).
    pub text_rgb: (u8, u8, u8),
}

impl Default for PdfConfig {
    fn default() -> Self {
        Self {
            font_size: 12.0,
            margin: 72.0, // 1 inch in points
            header: None,
            background_rgb: (255, 255, 255), // white
            text_rgb: (0, 0, 0),             // black
        }
    }
}

// Embedded font data - using a simple built-in approach
// In production, you'd embed a TTF file
const FONT_DATA: &[u8] = include_bytes!("../../assets/fonts/NotoSans-Regular.ttf");

/// Exports text content to a PDF file.
pub fn export_to_pdf(content: &str, path: &Path, config: &PdfConfig) -> anyhow::Result<()> {
    // A4 dimensions in points (1 point = 1/72 inch)
    const A4_WIDTH: f32 = 595.0;
    const A4_HEIGHT: f32 = 842.0;
    const LINE_HEIGHT_FACTOR: f32 = 1.4;
    const RESERVED_FOOTER_SPACE: f32 = 30.0;
    const AVG_CHAR_WIDTH_FACTOR: f32 = 0.5;
    
    let mut document = Document::new();
    
    // Load font
    let font = Font::new(FONT_DATA.to_vec().into(), 0)
        .ok_or_else(|| anyhow::anyhow!("Failed to load font"))?;
    
    let usable_width = A4_WIDTH - (2.0 * config.margin);
    let line_height = config.font_size * LINE_HEIGHT_FACTOR;
    let lines_per_page = ((A4_HEIGHT - 2.0 * config.margin - RESERVED_FOOTER_SPACE) / line_height) as usize;
    
    // Approximate characters per line
    let chars_per_line = (usable_width / (config.font_size * AVG_CHAR_WIDTH_FACTOR)) as usize;
    
    // Wrap text into lines
    let wrapped_lines = wrap_text(content, chars_per_line);
    // Calculate pages needed, ensuring at least 1 page even for empty content
    let total_pages = ((wrapped_lines.len() + lines_per_page - 1) / lines_per_page.max(1)).max(1);
    
    info!(
        lines = wrapped_lines.len(),
        pages = total_pages,
        chars_per_line,
        "Exporting to PDF"
    );
    
    let mut line_idx = 0;
    
    for page_num in 1..=total_pages {
        let mut page = document.start_page_with(
            PageSettings::from_wh(A4_WIDTH, A4_HEIGHT)
                .ok_or_else(|| anyhow::anyhow!("Invalid page dimensions"))?
        );
        let mut surface = page.surface();
        
        // Draw background if not white
        if config.background_rgb != (255, 255, 255) {
            let mut pb = PathBuilder::new();
            pb.move_to(0.0, 0.0);
            pb.line_to(A4_WIDTH, 0.0);
            pb.line_to(A4_WIDTH, A4_HEIGHT);
            pb.line_to(0.0, A4_HEIGHT);
            pb.close();
            let rect = pb.finish().unwrap();
            
            surface.set_fill(Some(Fill {
                paint: rgb::Color::new(
                    config.background_rgb.0,
                    config.background_rgb.1,
                    config.background_rgb.2,
                ).into(),
                opacity: NormalizedF32::ONE,
                rule: Default::default(),
            }));
            surface.draw_path(&rect);
        }
        
        // Set text color
        surface.set_fill(Some(Fill {
            paint: rgb::Color::new(
                config.text_rgb.0,
                config.text_rgb.1,
                config.text_rgb.2,
            ).into(),
            opacity: NormalizedF32::ONE,
            rule: Default::default(),
        }));
        
        let mut y_pos = config.margin;
        
        // Draw header
        if let Some(ref header) = config.header {
            surface.draw_text(
                Point::from_xy(config.margin, y_pos),
                font.clone(),
                config.font_size * 0.9,
                &format!("{} - Page {} of {}", header, page_num, total_pages),
                false,
                TextDirection::Auto,
            );
            y_pos += line_height * 1.5;
        }
        
        // Draw content lines
        let start_line = (page_num - 1) * lines_per_page;
        let end_line = (start_line + lines_per_page).min(wrapped_lines.len());
        
        for _ in start_line..end_line {
            if line_idx >= wrapped_lines.len() {
                break;
            }
            
            surface.draw_text(
                Point::from_xy(config.margin, y_pos),
                font.clone(),
                config.font_size,
                &wrapped_lines[line_idx],
                false,
                TextDirection::Auto,
            );
            
            y_pos += line_height;
            line_idx += 1;
        }
        
        surface.finish();
        page.finish();
    }
    
    // Save to file
    let pdf_data = document.finish()
        .map_err(|e| anyhow::anyhow!("Failed to generate PDF: {:?}", e))?;
    std::fs::write(path, &pdf_data)?;
    
    info!(path = ?path, "PDF exported successfully");
    Ok(())
}

/// Wraps text into lines of approximately the given width.
/// Preserves leading whitespace (indentation) from the original lines.
fn wrap_text(content: &str, max_chars: usize) -> Vec<String> {
    let mut lines = Vec::new();
    
    for paragraph in content.lines() {
        if paragraph.is_empty() {
            lines.push(String::new());
            continue;
        }
        
        // Preserve leading whitespace (indentation)
        let trimmed = paragraph.trim_start();
        let indent = &paragraph[..paragraph.len() - trimmed.len()];
        
        let words: Vec<&str> = trimmed.split_whitespace().collect();
        if words.is_empty() {
            // Line with only whitespace - preserve as empty
            lines.push(String::new());
            continue;
        }
        
        let mut current_line = String::new();
        let mut is_first_line = true;
        
        for word in words {
            if current_line.is_empty() {
                // Start new line with indent (only first line of paragraph gets original indent)
                if is_first_line {
                    current_line = format!("{}{}", indent, word);
                } else {
                    // Continuation lines get same indent for visual consistency
                    current_line = format!("{}{}", indent, word);
                }
            } else if current_line.len() + 1 + word.len() <= max_chars {
                current_line.push(' ');
                current_line.push_str(word);
            } else {
                lines.push(current_line);
                is_first_line = false;
                current_line = format!("{}{}", indent, word);
            }
        }
        
        if !current_line.is_empty() {
            lines.push(current_line);
        }
    }
    
    lines
}

#[cfg(test)]
mod tests {
    use super::wrap_text;

    #[test]
    fn test_wrap_preserves_indentation() {
        let input = "    indented line";
        let result = wrap_text(input, 80);
        assert_eq!(result, vec!["    indented line"]);
    }

    #[test]
    fn test_wrap_preserves_different_indent_levels() {
        let input = "no indent\n  two spaces\n    four spaces";
        let result = wrap_text(input, 80);
        assert_eq!(result, vec!["no indent", "  two spaces", "    four spaces"]);
    }

    #[test]
    fn test_wrap_long_indented_line_preserves_indent_on_continuation() {
        let input = "    word1 word2 word3 word4";
        let result = wrap_text(input, 20);
        // Each continuation line should also be indented
        assert!(result.len() >= 2);
        assert!(result[0].starts_with("    "));
        assert!(result[1].starts_with("    "));
    }

    #[test]
    fn test_wrap_empty_lines() {
        let input = "line1\n\nline2";
        let result = wrap_text(input, 80);
        assert_eq!(result, vec!["line1", "", "line2"]);
    }
}
