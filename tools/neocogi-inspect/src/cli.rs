//! Command-line grammar and usage diagnostics.

use std::ffi::{OsStr, OsString};
use std::path::PathBuf;

/// Documents split value arguments; output paths remain literal native values.
pub(crate) const USAGE: &str = "\
Usage:
  neocogi-inspect dump <LIBRARY> [--format json|text] [--output <PATH>]
  neocogi-inspect check <LIBRARY>
  neocogi-inspect plan <LIBRARY>... [--format json|text] [--output <PATH>]
  neocogi-inspect diff <OLD_JSON> <NEW_JSON> [--exact|--compat]

Commands:
  dump    Extract plugin descriptor metadata and print it
  check   Validate that descriptor metadata can be extracted
  plan    Inspect artifacts and compute a dependency-first load order
  diff    Compare artifacts exactly or check old-to-new compatibility

Option values use separate arguments: --output PATH, --format FORMAT.
";

pub(crate) enum Command {
    Dump {
        library: PathBuf,
        format: OutputFormat,
        output: Option<PathBuf>,
    },
    Check {
        library: PathBuf,
    },
    Plan {
        libraries: Vec<PathBuf>,
        format: OutputFormat,
        output: Option<PathBuf>,
    },
    Diff {
        old: PathBuf,
        new: PathBuf,
        mode: DiffMode,
    },
}

#[derive(Clone, Copy, Eq, PartialEq)]
pub(crate) enum DiffMode {
    Exact,
    Compatible,
}

#[derive(Clone, Copy, Eq, PartialEq)]
pub(crate) enum OutputFormat {
    Json,
    Text,
}

pub(crate) fn parse_args(args: impl IntoIterator<Item = OsString>) -> Result<Command, String> {
    let mut args = args.into_iter();
    let _program = args.next();
    let command = args.next().ok_or_else(|| "missing command".to_string())?;

    if command == OsStr::new("--help") || command == OsStr::new("-h") {
        println!("{USAGE}");
        std::process::exit(0);
    }

    match command.to_string_lossy().as_ref() {
        "dump" => parse_dump(args),
        "check" => parse_check(args),
        "plan" => parse_plan(args),
        "diff" => parse_diff(args),
        other => Err(format!("unknown command `{other}`")),
    }
}

/// Parses multiple libraries while split output values retain native path identity.
fn parse_plan(mut args: impl Iterator<Item = OsString>) -> Result<Command, String> {
    let mut libraries = Vec::new();
    let mut format = OutputFormat::Json;
    let mut output = None;

    while let Some(arg) = args.next() {
        if arg == OsStr::new("--format") || arg == OsStr::new("-f") {
            let value = args.next().ok_or_else(|| "missing value for --format".to_string())?;
            format = parse_format(&value)?;
        } else if arg == OsStr::new("--output") || arg == OsStr::new("-o") {
            let value = args.next().ok_or_else(|| "missing value for --output".to_string())?;
            if output.replace(PathBuf::from(value)).is_some() {
                return Err("duplicate --output".to_string());
            }
        } else if arg.to_string_lossy().starts_with('-') {
            return Err(format!("unknown plan option `{}`", arg.to_string_lossy()));
        } else {
            libraries.push(PathBuf::from(arg));
        }
    }

    if libraries.is_empty() {
        return Err("plan requires at least one library path".to_string());
    }
    Ok(Command::Plan { libraries, format, output })
}

/// Parses one library; option recognition never reinterprets a consumed output path.
fn parse_dump(mut args: impl Iterator<Item = OsString>) -> Result<Command, String> {
    let mut library = None;
    let mut format = OutputFormat::Json;
    let mut output = None;

    while let Some(arg) = args.next() {
        if arg == OsStr::new("--format") || arg == OsStr::new("-f") {
            let value = args.next().ok_or_else(|| "missing value for --format".to_string())?;
            format = parse_format(&value)?;
        } else if arg == OsStr::new("--output") || arg == OsStr::new("-o") {
            let value = args.next().ok_or_else(|| "missing value for --output".to_string())?;
            if output.replace(PathBuf::from(value)).is_some() {
                return Err("duplicate --output".to_string());
            }
        } else if arg.to_string_lossy().starts_with('-') {
            return Err(format!("unknown dump option `{}`", arg.to_string_lossy()));
        } else if library.replace(PathBuf::from(arg)).is_some() {
            return Err("dump accepts exactly one library path".to_string());
        }
    }

    Ok(Command::Dump {
        library: library.ok_or_else(|| "missing library path".to_string())?,
        format,
        output,
    })
}

fn parse_check(args: impl Iterator<Item = OsString>) -> Result<Command, String> {
    let mut library = None;
    for arg in args {
        if arg.to_string_lossy().starts_with('-') {
            return Err(format!("unknown check option `{}`", arg.to_string_lossy()));
        }
        if library.replace(PathBuf::from(arg)).is_some() {
            return Err("check accepts exactly one library path".to_string());
        }
    }

    Ok(Command::Check {
        library: library.ok_or_else(|| "missing library path".to_string())?,
    })
}

fn parse_diff(args: impl Iterator<Item = OsString>) -> Result<Command, String> {
    let mut old = None;
    let mut new = None;
    let mut mode = None;
    for arg in args {
        if arg == OsStr::new("--exact") {
            if mode.replace(DiffMode::Exact).is_some() {
                return Err("duplicate or conflicting diff mode".to_string());
            }
        } else if arg == OsStr::new("--compat") {
            if mode.replace(DiffMode::Compatible).is_some() {
                return Err("duplicate or conflicting diff mode".to_string());
            }
        } else if arg.to_string_lossy().starts_with('-') {
            return Err(format!("unknown diff option `{}`", arg.to_string_lossy()));
        } else if old.is_none() {
            old = Some(PathBuf::from(arg));
        } else if new.is_none() {
            new = Some(PathBuf::from(arg));
        } else {
            return Err("diff accepts exactly two JSON artifact paths".to_string());
        }
    }

    Ok(Command::Diff {
        old: old.ok_or_else(|| "missing old JSON artifact path".to_string())?,
        new: new.ok_or_else(|| "missing new JSON artifact path".to_string())?,
        mode: mode.unwrap_or(DiffMode::Exact),
    })
}

fn parse_format(value: &OsString) -> Result<OutputFormat, String> {
    match value.to_string_lossy().as_ref() {
        "json" => Ok(OutputFormat::Json),
        "text" => Ok(OutputFormat::Text),
        other => Err(format!("unknown output format `{other}`; expected json or text")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Parses the same opaque split value through both commands and spellings.
    fn split_output(command: &str, option: &str, path: &OsStr) -> PathBuf {
        let args = [
            OsString::from("neocogi-inspect"),
            OsString::from(command),
            OsString::from("plugin.so"),
            OsString::from(option),
            path.to_owned(),
        ];
        match parse_args(args).expect("split native output") {
            Command::Dump { library, output, .. } => {
                assert_eq!(library, PathBuf::from("plugin.so"));
                output.unwrap()
            }
            Command::Plan { libraries, output, .. } => {
                assert_eq!(libraries, [PathBuf::from("plugin.so")]);
                output.unwrap()
            }
            _ => panic!("expected output command"),
        }
    }

    /// ASCII, Unicode and option-looking filenames remain literal split values.
    #[test]
    fn split_output_keeps_literal_native_values() {
        for command in ["dump", "plan"] {
            for option in ["--output", "-o"] {
                for value in ["report.json", "report-é.json", "--format=json"] {
                    assert_eq!(split_output(command, option, OsStr::new(value)), PathBuf::from(value));
                }
            }
        }
    }

    /// Only documented split values are accepted in an option position.
    #[test]
    fn equals_options_are_usage_errors() {
        for command in ["dump", "plan"] {
            for option in ["--output=report.json", "--output=", "--format=json", "--format="] {
                assert!(parse_args(["neocogi-inspect", command, "plugin.so", option].map(OsString::from)).is_err());
            }
        }
    }

    /// Missing values, repeated output and command cardinality retain their owners.
    #[test]
    fn output_options_preserve_errors_and_library_cardinality() {
        for command in ["dump", "plan"] {
            for option in ["--output", "-o", "--format", "-f"] {
                assert!(parse_args(["neocogi-inspect", command, "plugin.so", option].map(OsString::from)).is_err());
            }
            assert!(parse_args(["neocogi-inspect", command, "plugin.so", "--output", "one", "-o", "two"].map(OsString::from)).is_err());
            assert!(parse_args(["neocogi-inspect", command].map(OsString::from)).is_err());
            for option in ["--format", "-f"] {
                let parsed = parse_args(["neocogi-inspect", command, "plugin.so", option, "text"].map(OsString::from)).unwrap();
                match parsed {
                    Command::Dump { format, .. } | Command::Plan { format, .. } => assert!(matches!(format, OutputFormat::Text)),
                    _ => panic!("expected output command"),
                }
            }
        }
        assert!(parse_args(["neocogi-inspect", "dump", "one.so", "two.so"].map(OsString::from)).is_err());
        let parsed = parse_args(["neocogi-inspect", "plan", "one.so", "two.so", "-o", "report"].map(OsString::from)).unwrap();
        match parsed {
            Command::Plan { libraries, output, .. } => {
                assert_eq!(libraries, [PathBuf::from("one.so"), PathBuf::from("two.so")]);
                assert_eq!(output, Some(PathBuf::from("report")));
            }
            _ => panic!("expected plan"),
        }
    }

    /// Native filename bytes survive parsing; removed inline aliases never decode them.
    #[cfg(unix)]
    #[test]
    fn unix_output_preserves_non_utf8_bytes() {
        use std::os::unix::ffi::{OsStrExt, OsStringExt};
        let path = OsString::from_vec(b"report-\xff.json".to_vec());
        for command in ["dump", "plan"] {
            for option in ["--output", "-o"] {
                assert_eq!(split_output(command, option, &path).as_os_str().as_bytes(), b"report-\xff.json");
            }
            for prefix in ["--output=", "--format="] {
                let mut inline = OsString::from(prefix);
                inline.push(&path);
                assert!(parse_args([OsString::from("neocogi-inspect"), OsString::from(command), OsString::from("plugin.so"), inline]).is_err());
            }
        }
    }

    /// Windows paths retain code units, including parser-only unpaired surrogates.
    #[cfg(windows)]
    #[test]
    fn windows_output_preserves_native_code_units() {
        use std::os::windows::ffi::{OsStrExt, OsStringExt};
        let units = [0x72, 0x65, 0x70, 0x6f, 0x72, 0x74, 0x2d, 0xd800, 0x2e, 0x6a, 0x73, 0x6f, 0x6e];
        let path = OsString::from_wide(&units);
        for command in ["dump", "plan"] {
            for option in ["--output", "-o"] {
                assert_eq!(split_output(command, option, &path).as_os_str().encode_wide().collect::<Vec<_>>(), units);
            }
            for prefix in ["--output=", "--format="] {
                let mut inline = OsString::from(prefix);
                inline.push(&path);
                assert!(parse_args([OsString::from("neocogi-inspect"), OsString::from(command), OsString::from("plugin.so"), inline]).is_err());
            }
        }
    }

    #[test]
    fn dump_defaults_to_json() {
        let command = parse_args(["neocogi-inspect", "dump", "plugin.so"].map(OsString::from)).expect("parsed command");

        match command {
            Command::Dump { library, format, output } => {
                assert_eq!(library, PathBuf::from("plugin.so"));
                assert!(matches!(format, OutputFormat::Json));
                assert!(output.is_none());
            }
            Command::Check { .. } | Command::Plan { .. } | Command::Diff { .. } => {
                panic!("expected dump command")
            }
        }
    }

    #[test]
    fn dump_accepts_text_output_file() {
        let command =
            parse_args(["neocogi-inspect", "dump", "plugin.so", "--format", "text", "--output", "plugin.txt"].map(OsString::from)).expect("parsed command");

        match command {
            Command::Dump { library, format, output } => {
                assert_eq!(library, PathBuf::from("plugin.so"));
                assert!(matches!(format, OutputFormat::Text));
                assert_eq!(output, Some(PathBuf::from("plugin.txt")));
            }
            Command::Check { .. } | Command::Plan { .. } | Command::Diff { .. } => {
                panic!("expected dump command")
            }
        }
    }

    #[test]
    fn diff_accepts_two_json_paths() {
        let command = parse_args(["neocogi-inspect", "diff", "old.json", "new.json"].map(OsString::from)).expect("parsed command");

        match command {
            Command::Diff { old, new, mode } => {
                assert_eq!(old, PathBuf::from("old.json"));
                assert_eq!(new, PathBuf::from("new.json"));
                assert!(matches!(mode, DiffMode::Exact));
            }
            Command::Dump { .. } | Command::Check { .. } | Command::Plan { .. } => {
                panic!("expected diff command")
            }
        }
    }

    #[test]
    fn diff_accepts_directional_compatibility_mode() {
        let command = parse_args(["neocogi-inspect", "diff", "--compat", "old.json", "new.json"].map(OsString::from)).expect("parsed command");

        match command {
            Command::Diff { old, new, mode } => {
                assert_eq!(old, PathBuf::from("old.json"));
                assert_eq!(new, PathBuf::from("new.json"));
                assert!(matches!(mode, DiffMode::Compatible));
            }
            Command::Dump { .. } | Command::Check { .. } | Command::Plan { .. } => {
                panic!("expected diff command")
            }
        }
    }

    #[test]
    fn plan_accepts_multiple_libraries() {
        let command = parse_args(["neocogi-inspect", "plan", "b.so", "a.so", "--format", "text"].map(OsString::from)).expect("parsed command");
        match command {
            Command::Plan { libraries, format, output } => {
                assert_eq!(libraries, [PathBuf::from("b.so"), PathBuf::from("a.so")]);
                assert!(matches!(format, OutputFormat::Text));
                assert!(output.is_none());
            }
            _ => panic!("expected plan command"),
        }
    }
}
