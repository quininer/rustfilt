use std::{ fs, mem };
use std::io::{ self, BufRead, Write };
use std::path::PathBuf;
use bstr::ByteSlice;
use bstr::io::BufReadExt;
use argh::FromArgs;
use regex::bytes::Regex;
use rustc_demangle::demangle;
use v_htmlescape::HTMLEscape;


/// Rust demangle tool
#[derive(FromArgs)]
struct Options {
    /// input file
    #[argh(option, short = 'i')]
    input: Option<PathBuf>,

    /// output file
    #[argh(option, short = 'o')]
    output: Option<PathBuf>,

    /// include hash
    #[argh(switch)]
    include_hash: bool,

    /// html escape
    #[argh(switch, short = 'e')]
    escape: bool,
}

impl Options {
    fn stream(&self, input: &mut dyn BufRead, output: &mut dyn Write) -> anyhow::Result<()> {
        // from https://github.com/luser/rustfilt/blob/master/src/main.rs#L36
        let pattern = Regex::new(r"_(ZN|R)[\$\._[:alnum:]]*")?;
        let mut buf = Vec::new();

        input.for_byte_line_with_terminator(|line| {
            let mut pos = 0;

            for mat in pattern.find_iter(line) {
                debug_assert!(pos <= mat.end());

                let start = mem::replace(&mut pos, mat.end());
                if start < mat.start() {
                    let end = mat.start();
                    output.write_all(&line[start..end])?;
                }

                let name = match mat.as_bytes().to_str() {
                    Ok(name) => demangle(name),
                    Err(_) => {
                        output.write_all(mat.as_bytes())?;
                        continue
                    }
                };

                macro_rules! fmt {
                    ( $output:expr ) => {
                        if self.include_hash {
                            write!($output, "{}", name)?;
                        } else {
                            write!($output, "{:#?}", name)?;
                        }
                    }
                }

                if !self.escape {
                    fmt!(output);
                } else {
                    buf.clear();
                    fmt!(&mut buf);
                    write!(output, "{}", HTMLEscape::new(&buf[..]))?;
                }
            }

            if let Some(buf) = line.get(pos..) {
                if !buf.is_empty() {
                    output.write_all(buf)?;
                }
            }

            output.flush()?;

            Ok(true)
        })?;

        Ok(())
    }
}

fn main() -> anyhow::Result<()> {
    let options: Options = argh::from_env();

    let stdin = io::stdin();
    let stdout = io::stdout();

    let mut input: Box<dyn BufRead> = if let Some(path) = options.input.as_ref() {
        Box::new(io::BufReader::new(fs::File::open(path)?))
    } else {
        Box::new(stdin.lock())
    };

    let mut output: Box<dyn io::Write> = if let Some(path) = options.output.as_ref() {
        Box::new(fs::File::create(path)?)
    } else {
        Box::new(stdout.lock())
    };

    options.stream(&mut input, &mut output)?;

    Ok(())
}
