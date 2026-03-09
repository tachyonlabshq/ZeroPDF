use crate::errors::ZeroPdfError;
use crate::pdf_ops::{RegionRequest, page_dimensions, search_text, validate_page_number};
use anyhow::{Context, Result};
use lopdf::{Dictionary, Document, Object, ObjectId, StringFormat, dictionary};
use serde::Serialize;
use std::path::Path;

#[derive(Debug, Clone, Serialize)]
pub struct AnnotationWriteResponse {
    pub status: String,
    pub input_pdf_path: String,
    pub output_pdf_path: String,
    pub page_number: usize,
    pub annotation_subtype: String,
    pub comment: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub author: Option<String>,
    pub rect: RegionRequest,
}

#[derive(Debug, Clone, Copy)]
struct AnnotationGeometry {
    page_id: ObjectId,
    rect: RegionRequest,
    page_height: f64,
}

#[derive(Debug, Clone)]
pub struct AddNoteRequest {
    pub page_number: usize,
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
    pub comment: String,
    pub author: Option<String>,
    pub color: Option<[f32; 3]>,
    pub icon: Option<String>,
}

#[derive(Debug, Clone)]
pub struct HighlightTextRequest {
    pub page_number: Option<usize>,
    pub query: String,
    pub match_index: usize,
    pub comment: String,
    pub author: Option<String>,
    pub color: Option<[f32; 3]>,
}

pub fn add_note(
    input_pdf: impl AsRef<Path>,
    output_pdf: impl AsRef<Path>,
    request: &AddNoteRequest,
) -> Result<AnnotationWriteResponse> {
    if request.width <= 0.0 || request.height <= 0.0 {
        return Err(ZeroPdfError::InvalidBounds(format!(
            "width={} height={}",
            request.width, request.height
        ))
        .into());
    }

    let input_pdf = input_pdf.as_ref();
    let output_pdf = output_pdf.as_ref();
    let mut doc = Document::load(input_pdf)
        .with_context(|| format!("failed to load {}", input_pdf.display()))?;
    let pages = doc.get_pages();
    let page_idx = validate_page_number(request.page_number, pages.len())?;
    let (_, page_id) = nth_page(&pages, page_idx).ok_or(ZeroPdfError::InvalidPageNumber {
        page_number: request.page_number,
        page_count: pages.len(),
    })?;
    let (_, page_height) = page_dimensions(input_pdf, request.page_number)?;

    let left = request.x;
    let top = request.y;
    let right = request.x + request.width;
    let bottom = request.y + request.height;
    let geometry = AnnotationGeometry {
        page_id,
        rect: RegionRequest {
            left,
            top,
            right,
            bottom,
        },
        page_height,
    };

    let annot_id = doc.add_object(build_text_annotation(
        geometry,
        &request.comment,
        request.author.as_deref(),
        request.color,
        request.icon.as_deref(),
    ));
    attach_annotation_to_page(&mut doc, page_id, annot_id)?;
    doc.save(output_pdf)
        .with_context(|| format!("failed to save {}", output_pdf.display()))?;

    Ok(AnnotationWriteResponse {
        status: "success".to_string(),
        input_pdf_path: input_pdf.to_string_lossy().to_string(),
        output_pdf_path: output_pdf.to_string_lossy().to_string(),
        page_number: request.page_number,
        annotation_subtype: "Text".to_string(),
        comment: request.comment.clone(),
        author: request.author.clone(),
        rect: geometry.rect,
    })
}

pub fn highlight_text(
    input_pdf: impl AsRef<Path>,
    output_pdf: impl AsRef<Path>,
    request: &HighlightTextRequest,
) -> Result<AnnotationWriteResponse> {
    let input_pdf = input_pdf.as_ref();
    let output_pdf = output_pdf.as_ref();
    let search = search_text(
        input_pdf,
        &request.query,
        request.page_number,
        request.match_index + 1,
    )?;
    let selected = search
        .matches
        .get(request.match_index)
        .ok_or_else(|| ZeroPdfError::QueryNotFound {
            query: request.query.clone(),
        })?
        .clone();

    let mut doc = Document::load(input_pdf)
        .with_context(|| format!("failed to load {}", input_pdf.display()))?;
    let pages = doc.get_pages();
    let page_idx = validate_page_number(selected.page_number, pages.len())?;
    let (_, page_id) = nth_page(&pages, page_idx).ok_or(ZeroPdfError::InvalidPageNumber {
        page_number: selected.page_number,
        page_count: pages.len(),
    })?;
    let (_, page_height) = page_dimensions(input_pdf, selected.page_number)?;
    let geometry = AnnotationGeometry {
        page_id,
        rect: RegionRequest {
            left: selected.left,
            top: selected.top,
            right: selected.right,
            bottom: selected.bottom,
        },
        page_height,
    };

    let annot_id = doc.add_object(build_highlight_annotation(
        geometry,
        &request.comment,
        request.author.as_deref(),
        request.color,
    ));
    attach_annotation_to_page(&mut doc, page_id, annot_id)?;
    doc.save(output_pdf)
        .with_context(|| format!("failed to save {}", output_pdf.display()))?;

    Ok(AnnotationWriteResponse {
        status: "success".to_string(),
        input_pdf_path: input_pdf.to_string_lossy().to_string(),
        output_pdf_path: output_pdf.to_string_lossy().to_string(),
        page_number: selected.page_number,
        annotation_subtype: "Highlight".to_string(),
        comment: request.comment.clone(),
        author: request.author.clone(),
        rect: geometry.rect,
    })
}

fn attach_annotation_to_page(
    doc: &mut Document,
    page_id: ObjectId,
    annot_id: ObjectId,
) -> Result<()> {
    let annots_ref = {
        let page = doc
            .get_object(page_id)
            .context("missing page object")?
            .as_dict()
            .context("page object is not a dictionary")?;
        match page.get(b"Annots") {
            Ok(Object::Reference(array_id)) => Some(*array_id),
            _ => None,
        }
    };

    if let Some(array_id) = annots_ref {
        let array_obj = doc
            .get_object_mut(array_id)
            .context("missing Annots array")?;
        let array = array_obj
            .as_array_mut()
            .context("Annots reference does not point to an array")?;
        array.push(annot_id.into());
        return Ok(());
    }

    let page = doc
        .get_object_mut(page_id)
        .context("missing page object")?
        .as_dict_mut()
        .context("page object is not a dictionary")?;

    match page.get_mut(b"Annots") {
        Ok(Object::Array(items)) => items.push(annot_id.into()),
        _ => {
            page.set("Annots", vec![annot_id.into()]);
        }
    }

    Ok(())
}

fn build_text_annotation(
    geometry: AnnotationGeometry,
    comment: &str,
    author: Option<&str>,
    color: Option<[f32; 3]>,
    icon: Option<&str>,
) -> Dictionary {
    let rect = top_origin_rect_to_pdf(geometry.rect, geometry.page_height);
    let mut dict = lopdf::dictionary! {
        "Type" => "Annot",
        "Subtype" => "Text",
        "Rect" => vec![rect[0].into(), rect[1].into(), rect[2].into(), rect[3].into()],
        "Contents" => Object::String(comment.as_bytes().to_vec(), StringFormat::Literal),
        "Open" => false,
        "Name" => icon.unwrap_or("Comment"),
        "P" => geometry.page_id,
    };
    if let Some(author) = author {
        dict.set(
            "T",
            Object::String(author.as_bytes().to_vec(), StringFormat::Literal),
        );
    }
    if let Some(color) = color {
        dict.set("C", vec![color[0].into(), color[1].into(), color[2].into()]);
    }
    dict
}

fn build_highlight_annotation(
    geometry: AnnotationGeometry,
    comment: &str,
    author: Option<&str>,
    color: Option<[f32; 3]>,
) -> Dictionary {
    let rect = top_origin_rect_to_pdf(geometry.rect, geometry.page_height);
    let quad_points = vec![
        geometry.rect.left.into(),
        (geometry.page_height - geometry.rect.top).into(),
        geometry.rect.right.into(),
        (geometry.page_height - geometry.rect.top).into(),
        geometry.rect.left.into(),
        (geometry.page_height - geometry.rect.bottom).into(),
        geometry.rect.right.into(),
        (geometry.page_height - geometry.rect.bottom).into(),
    ];
    let mut dict = lopdf::dictionary! {
        "Type" => "Annot",
        "Subtype" => "Highlight",
        "Rect" => vec![rect[0].into(), rect[1].into(), rect[2].into(), rect[3].into()],
        "QuadPoints" => quad_points,
        "Contents" => Object::String(comment.as_bytes().to_vec(), StringFormat::Literal),
        "P" => geometry.page_id,
        "F" => 4,
    };
    if let Some(author) = author {
        dict.set(
            "T",
            Object::String(author.as_bytes().to_vec(), StringFormat::Literal),
        );
    }
    dict.set(
        "C",
        color
            .unwrap_or([1.0, 0.92, 0.2])
            .iter()
            .map(|value| Object::Real(*value))
            .collect::<Vec<_>>(),
    );
    dict
}

fn top_origin_rect_to_pdf(rect: RegionRequest, page_height: f64) -> [f64; 4] {
    [
        rect.left,
        page_height - rect.bottom,
        rect.right,
        page_height - rect.top,
    ]
}

fn nth_page(
    pages: &std::collections::BTreeMap<u32, ObjectId>,
    index: usize,
) -> Option<(u32, ObjectId)> {
    pages
        .iter()
        .nth(index)
        .map(|(page_number, object_id)| (*page_number, *object_id))
}
