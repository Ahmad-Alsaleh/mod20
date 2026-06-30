use base64::Engine;
use clap::{CommandFactory, Parser, Subcommand};
use clap_complete::{generate, Shell};
use owo_colors::{AnsiColors, OwoColorize};
use percent_encoding::percent_decode_str;
use std::io::{IsTerminal, Read};
use std::process::ExitCode;
use url::{form_urlencoded, Url};

#[derive(Parser)]
#[command(name = "mod20", about = "Percent-decode a URL/text")]
struct Args {
    /// The URL or text to process. Use "-" or omit to read from stdin
    input: Option<String>,
    /// Pretty-print the URL's structure with colors
    #[arg(short, long)]
    pretty: bool,
    /// Disable colored output (also honored via the NO_COLOR env var)
    #[arg(long)]
    no_color: bool,
    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand)]
enum Commands {
    /// Generate a shell completion script
    Completions { shell: Shell },
}

/// Read all of stdin, trimming a single trailing newline (CRLF-aware).
fn read_stdin() -> std::io::Result<String> {
    let mut buf = String::new();
    std::io::stdin().read_to_string(&mut buf)?;
    if buf.ends_with('\n') {
        buf.pop();
        if buf.ends_with('\r') {
            buf.pop();
        }
    }
    Ok(buf)
}

fn main() -> ExitCode {
    let args = Args::parse();

    if let Some(Commands::Completions { shell }) = args.command {
        generate(shell, &mut Args::command(), "mod20", &mut std::io::stdout());
        return ExitCode::SUCCESS;
    }

    let input = match args.input.as_deref() {
        // An explicit "-" always reads from stdin, by convention.
        Some("-") => match read_stdin() {
            Ok(buf) => buf,
            Err(err) => {
                eprintln!("error: failed to read stdin: {err}");
                return ExitCode::FAILURE;
            }
        },
        Some(_) => args.input.unwrap(),
        None => {
            if std::io::stdin().is_terminal() {
                eprintln!("error: an input URL/text is required (pass an argument, \"-\", or pipe via stdin)");
                return ExitCode::FAILURE;
            }
            match read_stdin() {
                Ok(buf) => buf,
                Err(err) => {
                    eprintln!("error: failed to read stdin: {err}");
                    return ExitCode::FAILURE;
                }
            }
        }
    };

    if args.pretty {
        return pretty_print(&input, args.no_color);
    }

    match percent_decode_str(&input).decode_utf8() {
        Ok(decoded) => {
            println!("{decoded}");
            ExitCode::SUCCESS
        }
        Err(err) => {
            eprintln!("error: decoded bytes are not valid UTF-8: {err}");
            ExitCode::FAILURE
        }
    }
}

/// Maximum recursion depth for nested URLs, to avoid pathological inputs.
const MAX_DEPTH: usize = 3;

/// Apply an ANSI color to `s` only when coloring is enabled.
fn paint(s: &str, color: AnsiColors, use_color: bool) -> String {
    if use_color {
        s.color(color).to_string()
    } else {
        s.to_string()
    }
}

/// Render a dim `label:` followed by `value` at the given indentation.
fn line(indent: &str, label: &str, value: &str, use_color: bool) {
    let lbl = paint(&format!("{label}:"), AnsiColors::BrightBlack, use_color);
    println!("{indent}{lbl} {value}");
}

struct Decoded {
    text: String,
    ok: bool,
}

/// Percent-decode `s` as UTF-8, falling back to the raw text on failure.
fn decode(s: &str) -> Decoded {
    match percent_decode_str(s).decode_utf8() {
        Ok(c) => Decoded {
            text: c.into_owned(),
            ok: true,
        },
        Err(_) => Decoded {
            text: s.to_string(),
            ok: false,
        },
    }
}

/// Entry point for `--pretty`: parse the URL and render a colored breakdown.
fn pretty_print(input: &str, no_color: bool) -> ExitCode {
    let use_color = !no_color
        && std::io::stdout().is_terminal()
        && std::env::var_os("NO_COLOR").is_none();

    match Url::parse(input) {
        Ok(url) => {
            render_url(&url, use_color, 0);
            ExitCode::SUCCESS
        }
        Err(_) => {
            // Fallback: treat the input as a bare query string (relative URLs,
            // `?a=b&c=d`, or `a=b` aren't absolute URLs and won't parse above).
            let query = input.strip_prefix('?').unwrap_or(input);
            let pairs = group_pairs(form_urlencoded::parse(query.as_bytes()));
            if pairs.is_empty() {
                eprintln!("error: could not parse input as a URL");
                return ExitCode::FAILURE;
            }
            println!(
                "{}",
                paint(
                    "(not an absolute URL \u{2014} showing query only)",
                    AnsiColors::BrightBlack,
                    use_color
                )
            );
            render_query(&pairs, use_color, 0);
            ExitCode::SUCCESS
        }
    }
}

/// Group decoded key/value pairs, preserving order and collapsing repeated keys.
fn group_pairs<'a, I>(pairs: I) -> Vec<(String, Vec<String>)>
where
    I: Iterator<Item = (std::borrow::Cow<'a, str>, std::borrow::Cow<'a, str>)>,
{
    let mut out: Vec<(String, Vec<String>)> = Vec::new();
    for (k, v) in pairs {
        if let Some(entry) = out.iter_mut().find(|(ek, _)| *ek == k) {
            entry.1.push(v.into_owned());
        } else {
            out.push((k.into_owned(), vec![v.into_owned()]));
        }
    }
    out
}

/// Render a full URL's components as an indented, color-coded tree.
fn render_url(url: &Url, use_color: bool, depth: usize) {
    let indent = "  ".repeat(depth);

    line(
        &indent,
        "scheme",
        &paint(url.scheme(), AnsiColors::Cyan, use_color),
        use_color,
    );

    if !url.username().is_empty() || url.password().is_some() {
        let mut creds = url.username().to_string();
        if let Some(pw) = url.password() {
            creds.push(':');
            creds.push_str(pw);
        }
        line(
            &indent,
            "userinfo",
            &paint(&creds, AnsiColors::Red, use_color),
            use_color,
        );
    }

    if let Some(host) = url.host_str() {
        line(
            &indent,
            "host",
            &paint(host, AnsiColors::Green, use_color),
            use_color,
        );
    }

    if let Some(port) = url.port() {
        line(
            &indent,
            "port",
            &paint(&port.to_string(), AnsiColors::Magenta, use_color),
            use_color,
        );
    }

    let raw_path = url.path();
    if raw_path == "/" {
        line(
            &indent,
            "path",
            &paint("/", AnsiColors::Blue, use_color),
            use_color,
        );
    } else if !raw_path.is_empty() {
        println!("{indent}{}", paint("path:", AnsiColors::BrightBlack, use_color));
        for seg in raw_path.split('/').filter(|s| !s.is_empty()) {
            let d = decode(seg);
            println!(
                "{indent}  {}{}",
                paint("/", AnsiColors::BrightBlack, use_color),
                paint(&d.text, AnsiColors::Blue, use_color)
            );
        }
    }

    if url.query().is_some() {
        let pairs = group_pairs(url.query_pairs());
        if !pairs.is_empty() {
            render_query(&pairs, use_color, depth);
        }
    }

    if let Some(frag) = url.fragment() {
        let d = decode(frag);
        line(
            &indent,
            "fragment",
            &paint(&d.text, AnsiColors::White, use_color),
            use_color,
        );
    }

    if depth == 0 {
        print_warnings(url, use_color);
    }
}

/// Render grouped query pairs as aligned `key = value` rows with enrichment.
fn render_query(pairs: &[(String, Vec<String>)], use_color: bool, depth: usize) {
    let indent = "  ".repeat(depth);
    println!("{indent}{}", paint("query:", AnsiColors::BrightBlack, use_color));
    for (k, vals) in pairs {
        let key_str = paint(k, AnsiColors::Yellow, use_color);
        if vals.len() == 1 {
            println!(
                "{indent}  {key_str} = {}",
                paint(&vals[0], AnsiColors::White, use_color)
            );
            render_value(&vals[0], use_color, depth + 2);
        } else {
            println!("{indent}  {key_str} (x{})", vals.len());
            for v in vals {
                println!("{indent}    - {}", paint(v, AnsiColors::White, use_color));
                render_value(v, use_color, depth + 3);
            }
        }
    }
}

/// Detect and render enriched forms of a value: nested URL, JWT, or base64.
fn render_value(value: &str, use_color: bool, depth: usize) {
    if depth > MAX_DEPTH {
        return;
    }
    let indent = "  ".repeat(depth);

    if let Ok(nested) = Url::parse(value) {
        if nested.has_host() {
            println!(
                "{indent}{}",
                paint("\u{21b3} nested url:", AnsiColors::BrightBlack, use_color)
            );
            render_url(&nested, use_color, depth + 1);
            return;
        }
    }

    if let Some((header, payload)) = try_jwt(value) {
        println!("{indent}{}", paint("\u{21b3} jwt:", AnsiColors::BrightBlack, use_color));
        print_json(&header, "header", &indent, use_color);
        print_json(&payload, "payload", &indent, use_color);
        return;
    }

    if let Some(text) = try_base64(value) {
        println!(
            "{indent}{} {}",
            paint("\u{21b3} base64:", AnsiColors::BrightBlack, use_color),
            paint(&text, AnsiColors::White, use_color)
        );
    }
}

/// Print a pretty JSON blob under a dim label, indenting each line.
fn print_json(json: &str, label: &str, indent: &str, use_color: bool) {
    let lbl = paint(&format!("  {label}:"), AnsiColors::BrightBlack, use_color);
    println!("{indent}{lbl}");
    for ln in json.lines() {
        println!("{indent}    {}", paint(ln, AnsiColors::White, use_color));
    }
}

/// If `value` looks like a JWT, return pretty-printed header and payload JSON.
fn try_jwt(value: &str) -> Option<(String, String)> {
    let parts: Vec<&str> = value.split('.').collect();
    if parts.len() != 3 {
        return None;
    }
    let is_b64url = |s: &str| {
        !s.is_empty()
            && s.chars()
                .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
    };
    if !parts.iter().all(|p| is_b64url(p)) {
        return None;
    }
    let header = b64url_json(parts[0])?;
    let payload = b64url_json(parts[1])?;
    Some((header, payload))
}

/// Base64url-decode `part` and pretty-print it as JSON.
fn b64url_json(part: &str) -> Option<String> {
    let bytes = base64::engine::general_purpose::URL_SAFE_NO_PAD.decode(part).ok()?;
    let val: serde_json::Value = serde_json::from_slice(&bytes).ok()?;
    serde_json::to_string_pretty(&val).ok()
}

/// If `value` is standard base64 that decodes to printable UTF-8, return it.
fn try_base64(value: &str) -> Option<String> {
    if value.len() < 8 {
        return None;
    }
    if !value
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || c == '+' || c == '/' || c == '=')
    {
        return None;
    }
    let bytes = base64::engine::general_purpose::STANDARD.decode(value).ok()?;
    let text = String::from_utf8(bytes).ok()?;
    if text.is_empty() || text.chars().any(|c| c.is_control() && c != '\n' && c != '\t') {
        return None;
    }
    Some(text)
}

/// Emit safety warnings about the URL (insecure scheme, credentials, bad escapes).
fn print_warnings(url: &Url, use_color: bool) {
    let mut warns: Vec<String> = Vec::new();

    if url.scheme() == "http" {
        warns.push("insecure scheme (http)".to_string());
    }
    if !url.username().is_empty() || url.password().is_some() {
        warns.push("embedded credentials in userinfo".to_string());
    }

    let mut bad_escape = !decode(url.path()).ok;
    if let Some(q) = url.query() {
        bad_escape |= !decode(q).ok;
    }
    if let Some(f) = url.fragment() {
        bad_escape |= !decode(f).ok;
    }
    if bad_escape {
        warns.push("malformed percent-escape (not valid UTF-8)".to_string());
    }

    for w in warns {
        println!(
            "{} {}",
            paint("warning:", AnsiColors::Red, use_color),
            paint(&w, AnsiColors::Red, use_color)
        );
    }
}
