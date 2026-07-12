//! PDF text extraction via pdfium.
//!
//! pdfium is loaded dynamically from a bundled `libpdfium` shared library, so
//! there is no link-time dependency. The Tauri shell binds once at startup and
//! reuses the [`Pdfium`] instance.

use pdfium_render::prelude::*;

/// Bind to the `libpdfium` shared library in `lib_dir` (the folder containing
/// `libpdfium.dylib` / `.so` / `.dll`).
pub fn bind_pdfium(lib_dir: &str) -> Result<Pdfium, PdfiumError> {
    let bindings =
        Pdfium::bind_to_library(Pdfium::pdfium_platform_library_name_at_path(lib_dir))?;
    Ok(Pdfium::new(bindings))
}

/// Extract all page text from a PDF byte slice, pages joined by `\n`.
pub fn extract_text(pdfium: &Pdfium, bytes: &[u8]) -> Result<String, PdfiumError> {
    let doc = pdfium.load_pdf_from_byte_slice(bytes, None)?;
    let mut out = String::new();
    for (i, page) in doc.pages().iter().enumerate() {
        if i > 0 {
            out.push('\n');
        }
        out.push_str(&page.text()?.all());
    }
    Ok(out)
}
