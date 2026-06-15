use std::io::{BufRead, Write};

use crate::args::{parse_u64, parse_usize};

pub(crate) fn run_chat0_once(mut args: impl Iterator<Item = String>) -> Result<(), String> {
    let prompt = args.next().ok_or_else(|| String::from("missing prompt"))?;
    let expected = args.next();
    let trace_path = args
        .next()
        .unwrap_or_else(|| String::from("target/chat0-traces/chat0-last.trace"));
    if args.next().is_some() {
        return Err(String::from("too many arguments"));
    }

    let trace = nando_eval::chat0_once(13, 128, prompt.as_bytes(), expected.as_deref());
    let trace_text = trace.to_text();
    write_text_file(&trace_path, &trace_text)?;

    print!("{trace_text}");
    println!("trace_saved: {trace_path}");

    if expected.is_some() {
        let feedback_path = "target/chat0-feedback/chat0-feedback.log";
        append_feedback_log(feedback_path, &trace)?;
        println!("feedback_logged: {feedback_path}");
    }

    Ok(())
}

pub(crate) fn run_chat0_once_promoted(
    mut args: impl Iterator<Item = String>,
) -> Result<(), String> {
    let state_path = args
        .next()
        .ok_or_else(|| String::from("missing promoted state path"))?;
    let prompt = args.next().ok_or_else(|| String::from("missing prompt"))?;
    let expected = args.next();
    let trace_path = args
        .next()
        .unwrap_or_else(|| String::from("target/chat0-traces/chat0-promoted-last.trace"));
    if args.next().is_some() {
        return Err(String::from("too many arguments"));
    }

    let state = read_chat0_promoted_state(&state_path)?;
    let trace = nando_eval::chat0_once_with_promoted_state(
        state.train_seed,
        state.cases_per_split,
        prompt.as_bytes(),
        expected.as_deref(),
        &state,
    );
    let trace_text = trace.to_text();
    write_text_file(&trace_path, &trace_text)?;

    print!("{trace_text}");
    println!("trace_saved: {trace_path}");
    println!("promoted_state: {state_path}");

    if expected.is_some() {
        let feedback_path = "target/chat0-feedback/chat0-feedback.log";
        append_feedback_log(feedback_path, &trace)?;
        println!("feedback_logged: {feedback_path}");
    }

    Ok(())
}

pub(crate) fn run_chat0_shell(mut args: impl Iterator<Item = String>) -> Result<(), String> {
    let trace_dir = args
        .next()
        .unwrap_or_else(|| String::from("target/chat0-traces/shell"));
    let feedback_path = args
        .next()
        .unwrap_or_else(|| String::from("target/chat0-feedback/chat0-shell.log"));
    if args.next().is_some() {
        return Err(String::from("too many arguments"));
    }

    std::fs::create_dir_all(&trace_dir)
        .map_err(|error| format!("failed to create '{trace_dir}': {error}"))?;
    if let Some(parent) = std::path::Path::new(&feedback_path).parent() {
        std::fs::create_dir_all(parent)
            .map_err(|error| format!("failed to create '{}': {error}", parent.display()))?;
    }

    println!("Nando Wave chat-0 shell");
    println!("input: <prompt> || <expected>");
    println!("commands: :quit");

    let stdin = std::io::stdin();
    let mut handled = 0usize;
    for line in stdin.lock().lines() {
        let line = line.map_err(|error| format!("failed to read stdin: {error}"))?;
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        if matches!(trimmed, ":quit" | ":exit") {
            break;
        }

        handled += 1;
        let (prompt, expected) = parse_shell_chat0_line(trimmed);
        let trace = nando_eval::chat0_once(13, 128, prompt.as_bytes(), expected);
        let trace_path = format!("{trace_dir}/chat0-{handled:04}.trace");
        write_text_file(&trace_path, &trace.to_text())?;
        if expected.is_some() {
            append_feedback_log(&feedback_path, &trace)?;
        }

        println!(
            "chat0[{handled}]: response={} route={} feedback_correct={:?} trace={}",
            trace.response, trace.route, trace.feedback_correct, trace_path
        );
    }

    println!("chat0_shell_turns: {handled}");
    println!("trace_dir: {trace_dir}");
    println!("feedback_log: {feedback_path}");
    Ok(())
}

fn parse_shell_chat0_line(line: &str) -> (&str, Option<&str>) {
    match line.split_once("||") {
        Some((prompt, expected)) => {
            let expected = expected.trim();
            let expected = (!expected.is_empty()).then_some(expected);
            (prompt.trim(), expected)
        }
        None => (line.trim(), None),
    }
}

pub(crate) fn run_eval_chat0_promote(mut args: impl Iterator<Item = String>) -> Result<(), String> {
    let feedback_path = args
        .next()
        .unwrap_or_else(|| String::from("target/chat0-feedback/chat0-feedback.log"));
    let train_seed = match args.next() {
        Some(value) => parse_u64(&value, "train-seed")?,
        None => 13,
    };
    let holdout_seed = match args.next() {
        Some(value) => parse_u64(&value, "holdout-seed")?,
        None => 97,
    };
    let cases = match args.next() {
        Some(value) => parse_usize(&value, "cases")?,
        None => 128,
    };
    if args.next().is_some() {
        return Err(String::from("too many arguments"));
    }
    if cases == 0 {
        return Err(String::from("cases must be greater than zero"));
    }

    let feedback = read_chat0_feedback_log(&feedback_path)?;
    print!(
        "{}",
        nando_eval::chat0_promote_eval(train_seed, holdout_seed, cases, &feedback).to_text()
    );
    println!("feedback_log: {feedback_path}");
    Ok(())
}

pub(crate) fn run_eval_chat0_promoted_holdout(
    mut args: impl Iterator<Item = String>,
) -> Result<(), String> {
    let feedback_path = args
        .next()
        .unwrap_or_else(|| String::from("target/chat0-feedback/chat0-feedback.log"));
    let train_seed = match args.next() {
        Some(value) => parse_u64(&value, "train-seed")?,
        None => 13,
    };
    let holdout_seed = match args.next() {
        Some(value) => parse_u64(&value, "holdout-seed")?,
        None => 97,
    };
    let cases = match args.next() {
        Some(value) => parse_usize(&value, "cases")?,
        None => 128,
    };
    if args.next().is_some() {
        return Err(String::from("too many arguments"));
    }
    if cases == 0 {
        return Err(String::from("cases must be greater than zero"));
    }

    let feedback = read_chat0_feedback_log(&feedback_path)?;
    print!(
        "{}",
        nando_eval::chat0_promoted_holdout_eval(train_seed, holdout_seed, cases, &feedback)
            .to_text()
    );
    println!("feedback_log: {feedback_path}");
    Ok(())
}

pub(crate) fn run_chat0_promote_save(mut args: impl Iterator<Item = String>) -> Result<(), String> {
    let feedback_path = args
        .next()
        .ok_or_else(|| String::from("missing feedback log path"))?;
    let state_path = args
        .next()
        .ok_or_else(|| String::from("missing promoted state path"))?;
    let train_seed = match args.next() {
        Some(value) => parse_u64(&value, "train-seed")?,
        None => 13,
    };
    let holdout_seed = match args.next() {
        Some(value) => parse_u64(&value, "holdout-seed")?,
        None => 97,
    };
    let cases = match args.next() {
        Some(value) => parse_usize(&value, "cases")?,
        None => 128,
    };
    if args.next().is_some() {
        return Err(String::from("too many arguments"));
    }
    if cases == 0 {
        return Err(String::from("cases must be greater than zero"));
    }

    let feedback = read_chat0_feedback_log(&feedback_path)?;
    let report = nando_eval::chat0_promote_eval(train_seed, holdout_seed, cases, &feedback);
    print!("{}", report.to_text());
    if report.mode_status != "chat0_feedback_replay_promote_candidate_passed" {
        return Err(format!("promote gate did not pass: {}", report.mode_status));
    }

    let state = nando_eval::Chat0PromotedState::from_feedback(train_seed, cases, &feedback);
    if state.entries.is_empty() {
        return Err(String::from("promote gate passed but state has no entries"));
    }
    write_text_file(&state_path, &state.to_text())?;
    println!("promoted_state_saved: {state_path}");
    println!("promoted_entries: {}", state.entries.len());
    Ok(())
}

fn read_chat0_feedback_log(path: &str) -> Result<Vec<nando_eval::Chat0FeedbackEntry>, String> {
    let text = std::fs::read_to_string(path)
        .map_err(|error| format!("failed to read '{path}': {error}"))?;
    let mut entries = Vec::new();
    for (line_index, line) in text.lines().enumerate() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        let entry = parse_chat0_feedback_log_line(trimmed)
            .map_err(|error| format!("{path}:{}: {error}", line_index + 1))?;
        entries.push(entry);
    }
    Ok(entries)
}

fn read_chat0_promoted_state(path: &str) -> Result<nando_eval::Chat0PromotedState, String> {
    let text = std::fs::read_to_string(path)
        .map_err(|error| format!("failed to read '{path}': {error}"))?;
    nando_eval::Chat0PromotedState::from_text(&text)
        .map_err(|error| format!("failed to parse '{path}': {error}"))
}

fn parse_chat0_feedback_log_line(line: &str) -> Result<nando_eval::Chat0FeedbackEntry, String> {
    let prompt = feedback_field(line, "prompt=", " response=")?;
    let response = feedback_field(line, " response=", " expected=")?;
    let expected = feedback_field(line, " expected=", " feedback_correct=")?;
    let feedback_correct = feedback_field(line, " feedback_correct=", " route=")?;
    let feedback_correct = match feedback_correct {
        "Some(true)" | "true" => true,
        "Some(false)" | "false" => false,
        value => return Err(format!("invalid feedback_correct '{value}'")),
    };

    Ok(nando_eval::Chat0FeedbackEntry {
        prompt: prompt.to_owned(),
        response: response.to_owned(),
        expected: expected.to_owned(),
        feedback_correct,
    })
}

fn feedback_field<'a>(line: &'a str, start: &str, end: &str) -> Result<&'a str, String> {
    let start_index = line
        .find(start)
        .ok_or_else(|| format!("missing marker '{start}'"))?
        + start.len();
    let rest = &line[start_index..];
    let end_index = rest
        .find(end)
        .ok_or_else(|| format!("missing marker '{end}'"))?;
    Ok(rest[..end_index].trim())
}

fn write_text_file(path: &str, text: &str) -> Result<(), String> {
    let path = std::path::Path::new(path);
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|error| format!("failed to create '{}': {error}", parent.display()))?;
    }
    std::fs::write(path, text)
        .map_err(|error| format!("failed to write '{}': {error}", path.display()))
}

fn append_feedback_log(path: &str, trace: &nando_eval::Chat0Trace) -> Result<(), String> {
    let path = std::path::Path::new(path);
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|error| format!("failed to create '{}': {error}", parent.display()))?;
    }

    let mut file = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)
        .map_err(|error| format!("failed to open '{}': {error}", path.display()))?;
    writeln!(
        file,
        "prompt={} response={} expected={} feedback_correct={:?} route={} predicted_task={} coherence={:.6} spectral_entropy={:.6}",
        log_field(&trace.prompt),
        trace.response,
        trace
            .expected_response
            .as_deref()
            .map(log_field)
            .unwrap_or_else(|| String::from("none")),
        trace.feedback_correct,
        trace.route,
        trace.predicted_task,
        trace.coherence,
        trace.spectral_entropy
    )
    .map_err(|error| format!("failed to write '{}': {error}", path.display()))
}

fn log_field(value: &str) -> String {
    value.replace(['\n', '\t'], " ")
}
