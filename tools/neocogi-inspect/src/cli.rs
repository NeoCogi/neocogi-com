//! Command-line grammar and usage diagnostics.

use std::ffi::{OsStr, OsString};
use std::path::PathBuf;

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

fn parse_plan(args: impl Iterator<Item = OsString>) -> Result<Command, String> {
    let mut libraries = Vec::new();
    let mut format = OutputFormat::Json;
    let mut output = None;
    let mut args = args.peekable();

    while let Some(arg) = args.next() {
        if arg == OsStr::new("--format") || arg == OsStr::new("-f") {
            let value = args.next().ok_or_else(|| "missing value for --format".to_string())?;
            format = parse_format(&value)?;
        } else if arg == OsStr::new("--output") || arg == OsStr::new("-o") {
            let value = args.next().ok_or_else(|| "missing value for --output".to_string())?;
            if output.replace(PathBuf::from(value)).is_some() {
                return Err("duplicate --output".to_string());
            }
        } else if let Some(value) = option_value(&arg, "--format=") {
            format = parse_format(&value)?;
        } else if let Some(value) = option_value(&arg, "--output=") {
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

fn parse_dump(args: impl Iterator<Item = OsString>) -> Result<Command, String> {
    let mut library = None;
    let mut format = OutputFormat::Json;
    let mut output = None;
    let mut args = args.peekable();

    while let Some(arg) = args.next() {
        if arg == OsStr::new("--format") || arg == OsStr::new("-f") {
            let value = args.next().ok_or_else(|| "missing value for --format".to_string())?;
            format = parse_format(&value)?;
        } else if arg == OsStr::new("--output") || arg == OsStr::new("-o") {
            let value = args.next().ok_or_else(|| "missing value for --output".to_string())?;
            if output.replace(PathBuf::from(value)).is_some() {
                return Err("duplicate --output".to_string());
            }
        } else if let Some(value) = option_value(&arg, "--format=") {
            format = parse_format(&value)?;
        } else if let Some(value) = option_value(&arg, "--output=") {
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

fn option_value(arg: &OsString, prefix: &str) -> Option<OsString> {
    let value = arg.to_string_lossy();
    value.strip_prefix(prefix).map(|value| OsString::from(value.to_owned()))
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
