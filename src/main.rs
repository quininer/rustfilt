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

    /// enable html escape
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
                assert!(pos <= mat.end());

                if pos >= mat.start() {
                    pos = mat.end();
                } else {
                    let start = mem::replace(&mut pos, mat.end());
                    let end = mat.start();
                    output.write_all(&line[start..end])?;
                }

                let name = mat.as_bytes()
                    .to_str()
                    .map_err(|err| io::Error::new(io::ErrorKind::Other, err))?;
                let name = demangle(name);

                if !self.escape {
                    if !self.include_hash {
                        write!(output, "{}", name)?;
                    } else {
                        write!(output, "{:#?}", name)?;
                    }
                } else {
                    buf.clear();
                    if !self.include_hash {
                        write!(&mut buf, "{}", name)?;
                    } else {
                        write!(&mut buf, "{:#?}", name)?;
                    }
                    write!(output, "{}", HTMLEscape::new(&buf[..]))?;
                }
            }

            if pos < line.len() {
                output.write_all(&line[pos..])?;
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
