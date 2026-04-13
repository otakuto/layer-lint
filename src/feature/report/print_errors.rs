use std::io::Write;

use crate::LintError;
use codespan_reporting::diagnostic::Diagnostic;
use codespan_reporting::files::SimpleFile;
use codespan_reporting::term;
use codespan_reporting::term::termcolor::{Color, ColorChoice, ColorSpec, StandardStream, WriteColor};

pub fn print_errors(errors: &[LintError]) -> anyhow::Result<()> {
    let writer = StandardStream::stderr(ColorChoice::Auto);
    let config = term::Config::default();
    let empty_file = SimpleFile::new("layer-lint", "");

    for e in errors {
        match e {
            LintError::Denied { from, to, rule_target, policy_target } => {
                let mut w = writer.lock();
                // error[deny-policy]: from -> to
                w.set_color(ColorSpec::new().set_fg(Some(Color::Red)).set_bold(true))?;
                write!(w, "error[deny-policy]")?;
                w.set_color(ColorSpec::new().set_bold(true))?;
                writeln!(w, ": {} -> {}", from, to)?;
                w.reset()?;

                // context lines (dim)
                w.set_color(ColorSpec::new().set_fg(Some(Color::Blue)))?;
                write!(w, " = ")?;
                w.reset()?;
                writeln!(w, "- {}", rule_target)?;

                w.set_color(ColorSpec::new().set_fg(Some(Color::Blue)))?;
                write!(w, " = ")?;
                w.reset()?;
                writeln!(w, "  rules:")?;

                match policy_target {
                    Some(target) => {
                        w.set_color(ColorSpec::new().set_fg(Some(Color::Blue)))?;
                        write!(w, " = ")?;
                        w.reset()?;
                        writeln!(w, "    - deny:")?;

                        // highlighted line
                        w.set_color(ColorSpec::new().set_fg(Some(Color::Blue)))?;
                        write!(w, " = ")?;
                        w.set_color(ColorSpec::new().set_fg(Some(Color::Red)).set_bold(true))?;
                        writeln!(w, "      > {}", target)?;
                        w.reset()?;
                    }
                    None => {
                        w.set_color(ColorSpec::new().set_fg(Some(Color::Blue)))?;
                        write!(w, " = ")?;
                        w.reset()?;
                        writeln!(w, "    - allow: [...]")?;

                        // highlighted line
                        w.set_color(ColorSpec::new().set_fg(Some(Color::Blue)))?;
                        write!(w, " = ")?;
                        w.set_color(ColorSpec::new().set_fg(Some(Color::Red)).set_bold(true))?;
                        writeln!(w, "      (not matched — denied by default)")?;
                        w.reset()?;
                    }
                }
                writeln!(w)?;
            }
            _ => {
                let (code, msg, notes) = match e {
                    LintError::UnusedIgnore { from, to } => (
                        "unused-ignore",
                        format!("{} -> {}", from, to),
                        vec!["unused ignore entry".to_string()],
                    ),
                    LintError::NoMatchTarget { from } => (
                        "unused-ignore",
                        from.to_string(),
                        vec!["ignore target matches no workspace crate".to_string()],
                    ),
                    LintError::UnusedAllow { from, policy, to } => (
                        "unused-allow",
                        format!("{} -> {} [{}]", from, to, policy.as_str()),
                        vec!["unused allow/deny entry".to_string()],
                    ),
                    LintError::UndefinedLayer { layer, context } => (
                        "undefined-layer",
                        format!("undefined layer '{}' in {}", layer, context),
                        vec![],
                    ),
                    LintError::LayerCycle { cycle } => (
                        "layer-cycle",
                        format!("cycle detected: {}", cycle.join(" → ")),
                        vec![],
                    ),
                    LintError::UncoveredCrate { name } => (
                        "uncovered-crate",
                        name.to_string(),
                        vec!["no rule covers this crate".to_string()],
                    ),
                    LintError::Denied { .. } => unreachable!(),
                };
                let diagnostic = Diagnostic::<()>::error()
                    .with_code(code)
                    .with_message(msg)
                    .with_notes(notes);
                term::emit(&mut writer.lock(), &config, &empty_file, &diagnostic)?;
            }
        }
    }

    Ok(())
}
