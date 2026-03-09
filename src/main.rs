use anyhow::{Context, Result};
use std::env;
use std::path::Path;
use zeropdf::{
    AddNoteRequest, Diagnostic, ErrorCode, ExtractTextRequest, HighlightTextRequest, RegionRequest,
    classify_error, doctor_environment, extract_region, extract_text, init_project, inspect_pdf,
    resolve_agent_task_context, run_mcp_stdio, scan_agent_comments, schema_info, search_text,
    set_agent_task_status, skill_api_contract, sync_agent_tasks,
};

fn main() {
    let args: Vec<String> = env::args().collect();
    let pretty = args.iter().any(|arg| arg == "--pretty");
    let diagnostic_json = args.iter().any(|arg| arg == "--diagnostic-json");

    match run(&args, pretty) {
        Ok(()) => {}
        Err(err) => {
            let operation = args.get(1).map_or("cli", String::as_str);
            let diagnostic = classify_error(operation, &err);
            emit_error(&diagnostic, diagnostic_json || pretty);
            std::process::exit(1);
        }
    }
}

fn run(args: &[String], pretty: bool) -> Result<()> {
    if args.len() < 2 {
        print_usage();
        return Ok(());
    }

    match args[1].as_str() {
        "inspect" => {
            let pdf_path = required_arg(args, 2, "inspect requires <pdf_path>")?;
            print_json(&inspect_pdf(pdf_path)?, pretty)
        }
        "extract-text" => {
            let pdf_path = required_arg(args, 2, "extract-text requires <pdf_path>")?;
            let request = ExtractTextRequest {
                page: parse_usize_flag(args, "--page")?,
                page_start: parse_usize_flag(args, "--page-start")?,
                page_end: parse_usize_flag(args, "--page-end")?,
                max_chars_per_page: parse_usize_flag(args, "--max-chars")?.unwrap_or(4000),
                include_words: has_flag(args, "--include-words"),
            };
            print_json(&extract_text(pdf_path, &request)?, pretty)
        }
        "search-text" => {
            let pdf_path = required_arg(args, 2, "search-text requires <pdf_path> <query>")?;
            let query = required_arg(args, 3, "search-text requires <pdf_path> <query>")?;
            print_json(
                &search_text(
                    pdf_path,
                    query,
                    parse_usize_flag(args, "--page")?,
                    parse_usize_flag(args, "--limit")?.unwrap_or(10),
                )?,
                pretty,
            )
        }
        "extract-region" => {
            let pdf_path = required_arg(
                args,
                2,
                "extract-region requires <pdf_path> <page_number> <left> <top> <right> <bottom>",
            )?;
            let page_number = required_arg(args, 3, "missing page_number")?.parse::<usize>()?;
            let region = RegionRequest {
                left: required_arg(args, 4, "missing left")?.parse::<f64>()?,
                top: required_arg(args, 5, "missing top")?.parse::<f64>()?,
                right: required_arg(args, 6, "missing right")?.parse::<f64>()?,
                bottom: required_arg(args, 7, "missing bottom")?.parse::<f64>()?,
            };
            let margin = parse_f64_flag(args, "--margin")?.unwrap_or(0.0);
            print_json(
                &extract_region(pdf_path, page_number, region, margin)?,
                pretty,
            )
        }
        "add-note" => {
            let input_pdf = required_arg(args, 2, "add-note requires <input_pdf> <output_pdf>")?;
            let output_pdf = required_arg(args, 3, "add-note requires <input_pdf> <output_pdf>")?;
            let page_number = required_flag(args, "--page")?.parse::<usize>()?;
            let x = required_flag(args, "--x")?.parse::<f64>()?;
            let y = required_flag(args, "--y")?.parse::<f64>()?;
            let width = required_flag(args, "--width")?.parse::<f64>()?;
            let height = required_flag(args, "--height")?.parse::<f64>()?;
            let comment = required_flag(args, "--comment")?.to_string();
            let author = optional_flag(args, "--author").map(str::to_string);
            let response = zeropdf::add_note(
                input_pdf,
                output_pdf,
                &AddNoteRequest {
                    page_number,
                    x,
                    y,
                    width,
                    height,
                    comment,
                    author,
                    color: None,
                    icon: None,
                },
            )?;
            print_json(&response, pretty)
        }
        "highlight-text" => {
            let input_pdf =
                required_arg(args, 2, "highlight-text requires <input_pdf> <output_pdf>")?;
            let output_pdf =
                required_arg(args, 3, "highlight-text requires <input_pdf> <output_pdf>")?;
            let query = required_flag(args, "--query")?.to_string();
            let comment = required_flag(args, "--comment")?.to_string();
            let author = optional_flag(args, "--author").map(str::to_string);
            let response = zeropdf::highlight_text(
                input_pdf,
                output_pdf,
                &HighlightTextRequest {
                    page_number: parse_usize_flag(args, "--page")?,
                    query,
                    match_index: parse_usize_flag(args, "--match-index")?.unwrap_or(0),
                    comment,
                    author,
                    color: None,
                },
            )?;
            print_json(&response, pretty)
        }
        "scan-agent-comments" => {
            let pdf_path = required_arg(args, 2, "scan-agent-comments requires <pdf_path>")?;
            print_json(&scan_agent_comments(pdf_path)?, pretty)
        }
        "sync-agent-tasks" => {
            let pdf_path = required_arg(args, 2, "sync-agent-tasks requires <pdf_path>")?;
            let state_path = optional_flag(args, "--state-path").map(Path::new);
            print_json(&sync_agent_tasks(pdf_path, state_path)?, pretty)
        }
        "set-agent-task-status" => {
            let pdf_path = required_arg(
                args,
                2,
                "set-agent-task-status requires <pdf_path> <task_id> <status>",
            )?;
            let task_id = required_arg(
                args,
                3,
                "set-agent-task-status requires <pdf_path> <task_id> <status>",
            )?;
            let status = required_arg(
                args,
                4,
                "set-agent-task-status requires <pdf_path> <task_id> <status>",
            )?;
            let note = optional_flag(args, "--note").map(str::to_string);
            let state_path = optional_flag(args, "--state-path").map(Path::new);
            print_json(
                &set_agent_task_status(pdf_path, task_id, status, note, state_path)?,
                pretty,
            )
        }
        "resolve-agent-task-context" => {
            let pdf_path = required_arg(
                args,
                2,
                "resolve-agent-task-context requires <pdf_path> <task_id>",
            )?;
            let task_id = required_arg(
                args,
                3,
                "resolve-agent-task-context requires <pdf_path> <task_id>",
            )?;
            let state_path = optional_flag(args, "--state-path").map(Path::new);
            print_json(
                &resolve_agent_task_context(pdf_path, task_id, state_path)?,
                pretty,
            )
        }
        "doctor" => print_json(&doctor_environment()?, pretty),
        "init" => {
            let project_root =
                required_arg(args, 2, "init requires <project_root> --binary <path>")?;
            let binary = required_flag(args, "--binary")?;
            print_json(&init_project(project_root, Path::new(binary))?, pretty)
        }
        "schema-info" => print_json(&schema_info(), pretty),
        "skill-api-contract" => print_json(&skill_api_contract(), pretty),
        "mcp-stdio" => run_mcp_stdio(pretty),
        _ => {
            print_usage();
            Ok(())
        }
    }
}

fn print_usage() {
    eprintln!(
        "ZeroPDF\n\nCommands:\n  inspect <pdf_path>\n  extract-text <pdf_path> [--page N] [--page-start N --page-end N] [--max-chars N] [--include-words]\n  search-text <pdf_path> <query> [--page N] [--limit N]\n  extract-region <pdf_path> <page_number> <left> <top> <right> <bottom> [--margin N]\n  add-note <input_pdf> <output_pdf> --page N --x N --y N --width N --height N --comment TEXT [--author NAME]\n  highlight-text <input_pdf> <output_pdf> --query TEXT --comment TEXT [--page N] [--match-index N] [--author NAME]\n  scan-agent-comments <pdf_path>\n  sync-agent-tasks <pdf_path> [--state-path PATH]\n  set-agent-task-status <pdf_path> <task_id> <status> [--note TEXT] [--state-path PATH]\n  resolve-agent-task-context <pdf_path> <task_id> [--state-path PATH]\n  doctor\n  init <project_root> --binary <path_to_zeropdf_binary>\n  schema-info\n  skill-api-contract\n  mcp-stdio\n\nGlobal flags:\n  --pretty\n  --diagnostic-json"
    );
}

fn required_arg<'a>(args: &'a [String], index: usize, message: &str) -> Result<&'a str> {
    args.get(index)
        .map(String::as_str)
        .with_context(|| message.to_string())
}

fn required_flag<'a>(args: &'a [String], flag: &str) -> Result<&'a str> {
    optional_flag(args, flag).with_context(|| format!("missing required flag {flag}"))
}

fn optional_flag<'a>(args: &'a [String], flag: &str) -> Option<&'a str> {
    args.iter()
        .position(|arg| arg == flag)
        .and_then(|index| args.get(index + 1))
        .map(String::as_str)
}

fn has_flag(args: &[String], flag: &str) -> bool {
    args.iter().any(|arg| arg == flag)
}

fn parse_usize_flag(args: &[String], flag: &str) -> Result<Option<usize>> {
    optional_flag(args, flag)
        .map(|value| {
            value
                .parse::<usize>()
                .with_context(|| format!("invalid value for {flag}"))
        })
        .transpose()
}

fn parse_f64_flag(args: &[String], flag: &str) -> Result<Option<f64>> {
    optional_flag(args, flag)
        .map(|value| {
            value
                .parse::<f64>()
                .with_context(|| format!("invalid value for {flag}"))
        })
        .transpose()
}

fn print_json<T: serde::Serialize>(value: &T, pretty: bool) -> Result<()> {
    if pretty {
        println!("{}", serde_json::to_string_pretty(value)?);
    } else {
        println!("{}", serde_json::to_string(value)?);
    }
    Ok(())
}

fn emit_error(diagnostic: &Diagnostic, structured: bool) {
    if structured {
        let payload = serde_json::json!({
            "status": "error",
            "operation": diagnostic.operation,
            "code": diagnostic.code,
            "message": diagnostic.message,
            "hint": diagnostic.hint,
            "details": diagnostic.details,
        });
        eprintln!("{}", serde_json::to_string_pretty(&payload).unwrap());
    } else {
        eprintln!(
            "error [{}]: {}",
            format_error_code(diagnostic.code),
            diagnostic.message
        );
        if let Some(hint) = &diagnostic.hint {
            eprintln!("hint: {hint}");
        }
    }
}

fn format_error_code(code: ErrorCode) -> &'static str {
    match code {
        ErrorCode::InvalidArguments => "invalid_arguments",
        ErrorCode::InvalidPageNumber => "invalid_page_number",
        ErrorCode::InvalidBounds => "invalid_bounds",
        ErrorCode::InvalidPdf => "invalid_pdf",
        ErrorCode::AnnotationWrite => "annotation_write",
        ErrorCode::QueryNotFound => "query_not_found",
        ErrorCode::TaskNotFound => "task_not_found",
        ErrorCode::StateError => "state_error",
        ErrorCode::Io => "io",
        ErrorCode::Internal => "internal",
    }
}
