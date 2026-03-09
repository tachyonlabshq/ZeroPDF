use lopdf::content::{Content, Operation};
use lopdf::{Document, Object, Stream, dictionary};
use tempfile::TempDir;
use zeropdf::{
    AddNoteRequest, ExtractTextRequest, HighlightTextRequest, extract_text, inspect_pdf,
    resolve_agent_task_context, scan_agent_comments, search_text, set_agent_task_status,
    sync_agent_tasks,
};

fn make_sample_pdf(output: &std::path::Path) {
    let mut doc = Document::with_version("1.5");
    let pages_id = doc.new_object_id();
    let font_id = doc.add_object(dictionary! {
        "Type" => "Font",
        "Subtype" => "Type1",
        "BaseFont" => "Helvetica",
    });
    let resources_id = doc.add_object(dictionary! {
        "Font" => dictionary! {
            "F1" => font_id,
        },
    });

    let content = Content {
        operations: vec![
            Operation::new("BT", vec![]),
            Operation::new("Tf", vec!["F1".into(), 14.into()]),
            Operation::new("Td", vec![72.into(), 720.into()]),
            Operation::new(
                "Tj",
                vec![Object::string_literal(
                    "Revenue increased in Q4 and margin expanded.",
                )],
            ),
            Operation::new("Td", vec![0.into(), (-24).into()]),
            Operation::new(
                "Tj",
                vec![Object::string_literal(
                    "Target this sentence for @agent follow-up.",
                )],
            ),
            Operation::new("ET", vec![]),
        ],
    };
    let content_id = doc.add_object(Stream::new(dictionary! {}, content.encode().unwrap()));
    let page_id = doc.add_object(dictionary! {
        "Type" => "Page",
        "Parent" => pages_id,
        "Contents" => content_id,
    });
    let pages = dictionary! {
        "Type" => "Pages",
        "Kids" => vec![page_id.into()],
        "Count" => 1,
        "Resources" => resources_id,
        "MediaBox" => vec![0.into(), 0.into(), 612.into(), 792.into()],
    };
    doc.objects.insert(pages_id, pages.into());
    let catalog_id = doc.add_object(dictionary! {
        "Type" => "Catalog",
        "Pages" => pages_id,
    });
    doc.trailer.set("Root", catalog_id);
    doc.compress();
    doc.save(output).unwrap();
}

#[test]
fn inspect_extract_and_search_pdf() {
    let tmp = TempDir::new().unwrap();
    let pdf_path = tmp.path().join("sample.pdf");
    make_sample_pdf(&pdf_path);

    let inspect = inspect_pdf(&pdf_path).unwrap();
    assert_eq!(inspect.page_count, 1);
    assert_eq!(inspect.pages[0].annotation_count, 0);

    let extract = extract_text(
        &pdf_path,
        &ExtractTextRequest {
            include_words: true,
            ..Default::default()
        },
    )
    .unwrap();
    assert!(extract.pages[0].text.contains("Revenue increased in Q4"));
    assert!(!extract.pages[0].words.as_ref().unwrap().is_empty());

    let search = search_text(&pdf_path, "margin expanded", None, 5).unwrap();
    assert_eq!(search.match_count, 1);
    assert_eq!(search.matches[0].page_number, 1);
}

#[test]
fn note_annotations_drive_agent_tasks() {
    let tmp = TempDir::new().unwrap();
    let input_pdf = tmp.path().join("input.pdf");
    let noted_pdf = tmp.path().join("noted.pdf");
    make_sample_pdf(&input_pdf);

    zeropdf::add_note(
        &input_pdf,
        &noted_pdf,
        &AddNoteRequest {
            page_number: 1,
            x: 80.0,
            y: 70.0,
            width: 18.0,
            height: 18.0,
            comment: "@Agent summarize the nearby paragraph".to_string(),
            author: Some("QA".to_string()),
            color: None,
            icon: None,
        },
    )
    .unwrap();

    let scan = scan_agent_comments(&noted_pdf).unwrap();
    assert_eq!(scan.task_count, 1);
    assert!(scan.tasks[0].instruction.contains("summarize"));

    let sync = sync_agent_tasks(&noted_pdf, None).unwrap();
    assert_eq!(sync.task_count, 1);

    let updated = set_agent_task_status(
        &noted_pdf,
        &scan.tasks[0].task_id,
        "running",
        Some("picked up".to_string()),
        None,
    )
    .unwrap();
    assert_eq!(updated.task_status, "running");

    let resolved = resolve_agent_task_context(&noted_pdf, &scan.tasks[0].task_id, None).unwrap();
    assert_eq!(resolved.task_status, "running");
    assert!(resolved.context_text.contains("Revenue") || resolved.context_text.contains("Target"));
}

#[test]
fn highlight_annotations_drive_agent_tasks() {
    let tmp = TempDir::new().unwrap();
    let input_pdf = tmp.path().join("input.pdf");
    let highlighted_pdf = tmp.path().join("highlighted.pdf");
    make_sample_pdf(&input_pdf);

    zeropdf::highlight_text(
        &input_pdf,
        &highlighted_pdf,
        &HighlightTextRequest {
            page_number: Some(1),
            query: "Target this sentence".to_string(),
            match_index: 0,
            comment: "@Agent rewrite this sentence with a sharper tone".to_string(),
            author: Some("QA".to_string()),
            color: None,
        },
    )
    .unwrap();

    let scan = scan_agent_comments(&highlighted_pdf).unwrap();
    assert_eq!(scan.task_count, 1);
    assert!(scan.tasks[0].context_text.contains("Target"));
    assert!(scan.tasks[0].context_text.contains("sentence"));
}
