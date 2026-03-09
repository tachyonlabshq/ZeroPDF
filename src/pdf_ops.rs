use crate::errors::ZeroPdfError;
use anyhow::{Context, Result};
use pdfplumber::{BBox, Pdf, SearchOptions, TextOptions, WordOptions};
use serde::Serialize;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Serialize)]
pub struct PdfMetadataView {
    pub title: Option<String>,
    pub author: Option<String>,
    pub subject: Option<String>,
    pub keywords: Option<String>,
    pub creator: Option<String>,
    pub producer: Option<String>,
    pub creation_date: Option<String>,
    pub modification_date: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct PageSummary {
    pub page_number: usize,
    pub width: f64,
    pub height: f64,
    pub word_count: usize,
    pub annotation_count: usize,
    pub preview: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct InspectPdfResponse {
    pub status: String,
    pub pdf_path: String,
    pub page_count: usize,
    pub metadata: PdfMetadataView,
    pub pages: Vec<PageSummary>,
}

#[derive(Debug, Clone, Serialize)]
pub struct WordBox {
    pub text: String,
    pub left: f64,
    pub top: f64,
    pub right: f64,
    pub bottom: f64,
}

#[derive(Debug, Clone, Serialize)]
pub struct PageTextResponse {
    pub page_number: usize,
    pub text: String,
    pub clipped: bool,
    pub word_count: usize,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub words: Option<Vec<WordBox>>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ExtractTextResponse {
    pub status: String,
    pub pdf_path: String,
    pub page_count: usize,
    pub pages: Vec<PageTextResponse>,
}

#[derive(Debug, Clone, Serialize)]
pub struct TextSearchMatch {
    pub page_number: usize,
    pub text: String,
    pub left: f64,
    pub top: f64,
    pub right: f64,
    pub bottom: f64,
    pub context: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct SearchTextResponse {
    pub status: String,
    pub pdf_path: String,
    pub query: String,
    pub match_count: usize,
    pub matches: Vec<TextSearchMatch>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ExtractRegionResponse {
    pub status: String,
    pub pdf_path: String,
    pub page_number: usize,
    pub bbox: RegionRequest,
    pub text: String,
    pub word_count: usize,
    pub words: Vec<WordBox>,
}

#[derive(Debug, Clone, Copy, Serialize)]
pub struct RegionRequest {
    pub left: f64,
    pub top: f64,
    pub right: f64,
    pub bottom: f64,
}

#[derive(Debug, Clone)]
pub struct ExtractTextRequest {
    pub page: Option<usize>,
    pub page_start: Option<usize>,
    pub page_end: Option<usize>,
    pub max_chars_per_page: usize,
    pub include_words: bool,
}

impl Default for ExtractTextRequest {
    fn default() -> Self {
        Self {
            page: None,
            page_start: None,
            page_end: None,
            max_chars_per_page: 4000,
            include_words: false,
        }
    }
}

fn open_pdf(path: &Path) -> Result<Pdf> {
    Pdf::open_file(path, None).map_err(|e| ZeroPdfError::Pdf(e.to_string()).into())
}

pub fn inspect_pdf(path: impl AsRef<Path>) -> Result<InspectPdfResponse> {
    let path = path.as_ref();
    let pdf = open_pdf(path)?;
    let mut pages = Vec::new();

    for page_idx in 0..pdf.page_count() {
        let page = pdf
            .page(page_idx)
            .map_err(|e| ZeroPdfError::Pdf(e.to_string()))?;
        let words = page.extract_words(&WordOptions::default());
        let preview = clip_text(&page.extract_text(&TextOptions::default()), 180).0;
        pages.push(PageSummary {
            page_number: page_idx + 1,
            width: page.width(),
            height: page.height(),
            word_count: words.len(),
            annotation_count: page.annots().len(),
            preview,
        });
    }

    let metadata = pdf.metadata();
    Ok(InspectPdfResponse {
        status: "success".to_string(),
        pdf_path: path.to_string_lossy().to_string(),
        page_count: pdf.page_count(),
        metadata: PdfMetadataView {
            title: metadata.title.clone(),
            author: metadata.author.clone(),
            subject: metadata.subject.clone(),
            keywords: metadata.keywords.clone(),
            creator: metadata.creator.clone(),
            producer: metadata.producer.clone(),
            creation_date: metadata.creation_date.clone(),
            modification_date: metadata.mod_date.clone(),
        },
        pages,
    })
}

pub fn extract_text(
    path: impl AsRef<Path>,
    request: &ExtractTextRequest,
) -> Result<ExtractTextResponse> {
    let path = path.as_ref();
    let pdf = open_pdf(path)?;
    let pages = collect_page_indices(
        pdf.page_count(),
        request.page,
        request.page_start,
        request.page_end,
    )?;
    let mut extracted = Vec::new();

    for page_idx in pages {
        let page = pdf
            .page(page_idx)
            .map_err(|e| ZeroPdfError::Pdf(e.to_string()))?;
        let words = page.extract_words(&WordOptions::default());
        let (text, clipped) = clip_text(
            &page.extract_text(&TextOptions::default()),
            request.max_chars_per_page,
        );
        let word_boxes = request
            .include_words
            .then(|| words.iter().map(word_to_box).collect::<Vec<_>>());
        extracted.push(PageTextResponse {
            page_number: page_idx + 1,
            text,
            clipped,
            word_count: words.len(),
            words: word_boxes,
        });
    }

    Ok(ExtractTextResponse {
        status: "success".to_string(),
        pdf_path: path.to_string_lossy().to_string(),
        page_count: pdf.page_count(),
        pages: extracted,
    })
}

pub fn search_text(
    path: impl AsRef<Path>,
    query: &str,
    page: Option<usize>,
    limit: usize,
) -> Result<SearchTextResponse> {
    let path = path.as_ref();
    let pdf = open_pdf(path)?;
    let page_indices = collect_page_indices(pdf.page_count(), page, None, None)?;
    let mut matches = Vec::new();

    for page_idx in page_indices {
        let current_page = pdf
            .page(page_idx)
            .map_err(|e| ZeroPdfError::Pdf(e.to_string()))?;
        for item in current_page.search(
            query,
            &SearchOptions {
                regex: false,
                case_sensitive: false,
            },
        ) {
            let context_bbox =
                expand_bbox(item.bbox, 18.0, current_page.width(), current_page.height());
            let context = current_page
                .crop(context_bbox)
                .extract_text(&TextOptions::default());
            matches.push(TextSearchMatch {
                page_number: item.page_number + 1,
                text: item.text,
                left: item.bbox.x0,
                top: item.bbox.top,
                right: item.bbox.x1,
                bottom: item.bbox.bottom,
                context: clip_text(&context, 240).0,
            });
            if matches.len() >= limit {
                break;
            }
        }
        if matches.len() >= limit {
            break;
        }
    }

    if matches.is_empty() {
        return Err(ZeroPdfError::QueryNotFound {
            query: query.to_string(),
        }
        .into());
    }

    Ok(SearchTextResponse {
        status: "success".to_string(),
        pdf_path: path.to_string_lossy().to_string(),
        query: query.to_string(),
        match_count: matches.len(),
        matches,
    })
}

pub fn extract_region(
    path: impl AsRef<Path>,
    page_number: usize,
    region: RegionRequest,
    margin: f64,
) -> Result<ExtractRegionResponse> {
    validate_region(region)?;
    let path = path.as_ref();
    let pdf = open_pdf(path)?;
    let page_idx = validate_page_number(page_number, pdf.page_count())?;
    let page = pdf
        .page(page_idx)
        .map_err(|e| ZeroPdfError::Pdf(e.to_string()))?;
    let bbox = expand_bbox(
        BBox::new(region.left, region.top, region.right, region.bottom),
        margin,
        page.width(),
        page.height(),
    );
    let cropped = page.crop(bbox);
    let words = cropped.extract_words(&WordOptions::default());

    Ok(ExtractRegionResponse {
        status: "success".to_string(),
        pdf_path: path.to_string_lossy().to_string(),
        page_number,
        bbox: region,
        text: cropped.extract_text(&TextOptions::default()),
        word_count: words.len(),
        words: words.iter().map(word_to_box).collect(),
    })
}

pub fn validate_page_number(page_number: usize, page_count: usize) -> Result<usize> {
    if page_number == 0 || page_number > page_count {
        return Err(ZeroPdfError::InvalidPageNumber {
            page_number,
            page_count,
        }
        .into());
    }
    Ok(page_number - 1)
}

pub fn validate_region(region: RegionRequest) -> Result<()> {
    if region.right <= region.left || region.bottom <= region.top {
        return Err(ZeroPdfError::InvalidBounds(format!(
            "[{:.2}, {:.2}, {:.2}, {:.2}]",
            region.left, region.top, region.right, region.bottom
        ))
        .into());
    }
    Ok(())
}

pub fn extract_near_bbox(
    path: impl AsRef<Path>,
    page_number: usize,
    bbox: BBox,
    padding: f64,
) -> Result<String> {
    let path = path.as_ref();
    let pdf = open_pdf(path)?;
    let page_idx = validate_page_number(page_number, pdf.page_count())?;
    let page = pdf
        .page(page_idx)
        .map_err(|e| ZeroPdfError::Pdf(e.to_string()))?;
    let expanded = expand_bbox(bbox, padding, page.width(), page.height());
    let text = page.crop(expanded).extract_text(&TextOptions::default());
    if !text.trim().is_empty() {
        return Ok(text);
    }

    let words = page.extract_words(&WordOptions::default());
    let mut nearest = words
        .iter()
        .map(|word| {
            let center_x = (word.bbox.x0 + word.bbox.x1) / 2.0;
            let center_y = (word.bbox.top + word.bbox.bottom) / 2.0;
            let bbox_center_x = (bbox.x0 + bbox.x1) / 2.0;
            let bbox_center_y = (bbox.top + bbox.bottom) / 2.0;
            let dist =
                ((center_x - bbox_center_x).powi(2) + (center_y - bbox_center_y).powi(2)).sqrt();
            (dist, word.text.clone())
        })
        .collect::<Vec<_>>();
    nearest.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap_or(std::cmp::Ordering::Equal));
    Ok(nearest
        .into_iter()
        .take(18)
        .map(|(_, text)| text)
        .collect::<Vec<_>>()
        .join(" "))
}

pub fn expand_bbox(bbox: BBox, padding: f64, page_width: f64, page_height: f64) -> BBox {
    BBox::new(
        (bbox.x0 - padding).max(0.0),
        (bbox.top - padding).max(0.0),
        (bbox.x1 + padding).min(page_width),
        (bbox.bottom + padding).min(page_height),
    )
}

pub fn clip_text(text: &str, max_chars: usize) -> (String, bool) {
    if text.chars().count() <= max_chars {
        return (text.to_string(), false);
    }
    let clipped = text.chars().take(max_chars).collect::<String>();
    (format!("{}...", clipped.trim_end()), true)
}

pub fn pdf_page_height(path: impl AsRef<Path>, page_number: usize) -> Result<f64> {
    let path = path.as_ref();
    let pdf = open_pdf(path)?;
    let page_idx = validate_page_number(page_number, pdf.page_count())?;
    let page = pdf
        .page(page_idx)
        .map_err(|e| ZeroPdfError::Pdf(e.to_string()))?;
    Ok(page.height())
}

pub fn page_dimensions(path: impl AsRef<Path>, page_number: usize) -> Result<(f64, f64)> {
    let path = path.as_ref();
    let pdf = open_pdf(path)?;
    let page_idx = validate_page_number(page_number, pdf.page_count())?;
    let page = pdf
        .page(page_idx)
        .map_err(|e| ZeroPdfError::Pdf(e.to_string()))?;
    Ok((page.width(), page.height()))
}

pub fn canonical_pdf_path(path: impl AsRef<Path>) -> Result<PathBuf> {
    path.as_ref()
        .canonicalize()
        .with_context(|| format!("failed to resolve {}", path.as_ref().display()))
}

fn collect_page_indices(
    page_count: usize,
    page: Option<usize>,
    page_start: Option<usize>,
    page_end: Option<usize>,
) -> Result<Vec<usize>> {
    if let Some(single) = page {
        return Ok(vec![validate_page_number(single, page_count)?]);
    }

    let start = page_start.unwrap_or(1);
    let end = page_end.unwrap_or(page_count);
    if start > end {
        return Err(ZeroPdfError::InvalidArguments(format!(
            "page-start ({start}) must be <= page-end ({end})"
        ))
        .into());
    }
    let mut pages = Vec::new();
    for page_number in start..=end {
        pages.push(validate_page_number(page_number, page_count)?);
    }
    Ok(pages)
}

fn word_to_box(word: &pdfplumber::Word) -> WordBox {
    WordBox {
        text: word.text.clone(),
        left: word.bbox.x0,
        top: word.bbox.top,
        right: word.bbox.x1,
        bottom: word.bbox.bottom,
    }
}
