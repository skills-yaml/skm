use std::error::Error;
use std::io::{self, BufRead, IsTerminal, Write};

type Result<T> = std::result::Result<T, Box<dyn Error>>;

pub fn confirm(question: &str) -> Result<bool> {
    if !io::stdin().is_terminal() || !io::stderr().is_terminal() {
        return Err(
            "Interactive add needs a terminal; pass --yes to apply without a prompt".into(),
        );
    }
    let stdin = io::stdin();
    let stderr = io::stderr();
    confirm_with_io(question, &mut stdin.lock(), &mut stderr.lock())
}

fn confirm_with_io<R: BufRead, W: Write>(
    question: &str,
    input: &mut R,
    output: &mut W,
) -> Result<bool> {
    write!(output, "{question} [y/N] ")?;
    output.flush()?;
    let mut answer = String::new();
    if input.read_line(&mut answer)? == 0 {
        return Ok(false);
    }
    Ok(matches!(
        answer.trim().to_ascii_lowercase().as_str(),
        "y" | "yes"
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn explicit_yes_only_and_eof_declines() {
        for (reply, expected) in [
            ("y\n", true),
            ("YES\n", true),
            ("\n", false),
            ("n\n", false),
            ("maybe\n", false),
            ("", false),
        ] {
            let mut output = Vec::new();
            assert_eq!(
                confirm_with_io("Install this skill?", &mut reply.as_bytes(), &mut output).unwrap(),
                expected
            );
            assert_eq!(output, b"Install this skill? [y/N] ");
        }
    }
}
