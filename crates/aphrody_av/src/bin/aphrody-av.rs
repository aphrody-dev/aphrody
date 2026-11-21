//! `aphrody-av`: an ffmpeg-like command line over the pure-Rust codecs and containers.

use aphrody_av::caps::{self, can_encode, decode_status};
use aphrody_av::service::{
    ConvertSpec, Sandbox, TranscodeSpec, convert_file, probe_file, transcode_file,
};
use aphrody_av::{AvError, Codec, Format};
use std::path::PathBuf;
use std::process::ExitCode;

const USAGE: &str = "\
aphrody-av: pure-Rust probe, transcode and convert

usage:
  aphrody-av [options] -i INPUT [options] OUTPUT     transcode or remux (ffmpeg-style)
  aphrody-av probe INPUT [--json]                    container, streams, codecs, duration
  aphrody-av convert INPUT OUTDIR [--formats svg,md,html,jsx,gpui] [--fps N] [--width N] [--max-frames N] [-y]
  aphrody-av mcp --read-root DIR [--read-root DIR]... --write-root DIR [--write-root DIR]...
  aphrody-av -codecs [--json|--markdown]             codecs and what they can do
  aphrody-av -formats [--json]                       containers

transcode options:
  -i FILE            input
  -c copy | -c:v CODEC | -c:a CODEC     codec or `copy` (aliases -vcodec, -acodec)
  -b:v RATE  -b:a RATE                  bit rate, e.g. 800k or 2M
  -vf scale=W:H      resize (-1 keeps the aspect ratio); -s WxH is the same
  -ss TIME  -t TIME  start offset and duration (seconds, MM:SS or HH:MM:SS.ms); need a re-encode
  -r FPS             output frame rate (needs a re-encode)
  -ac N  -ar HZ      audio channels (1 or 2) and sample rate
  -vn  -an           drop video / audio
  -f FORMAT          output container (default: from the output extension)
  -y | -n            overwrite / never overwrite (default: refuse to overwrite)

Unsupported codec, container or option combinations fail with the exact reason.";

enum Command {
    Help,
    Version,
    Probe {
        input: String,
        json: bool,
    },
    Codecs {
        mode: ListMode,
    },
    Formats {
        json: bool,
    },
    Convert {
        input: String,
        output_dir: String,
        spec: ConvertSpec,
        overwrite: bool,
    },
    Mcp {
        read: Vec<PathBuf>,
        write: Vec<PathBuf>,
    },
    Transcode {
        input: String,
        output: String,
        spec: TranscodeSpec,
        overwrite: bool,
    },
}

#[derive(Clone, Copy)]
enum ListMode {
    Text,
    Json,
    Markdown,
}

struct Usage(String);

fn usage<T>(message: impl Into<String>) -> Result<T, Usage> {
    Err(Usage(message.into()))
}

fn value<'a>(args: &'a [String], at: &mut usize, flag: &str) -> Result<&'a str, Usage> {
    *at += 1;
    args.get(*at)
        .map(String::as_str)
        .ok_or_else(|| Usage(format!("{flag} needs a value")))
}

fn parse(args: &[String]) -> Result<Command, Usage> {
    let first = args.first().map(String::as_str);
    match first {
        None | Some("-h") | Some("-help") | Some("--help") | Some("help") => {
            return Ok(Command::Help);
        }
        Some("-version") | Some("--version") | Some("version") => return Ok(Command::Version),
        Some("probe") => {
            let mut input = None;
            let mut json = false;
            for arg in &args[1..] {
                match arg.as_str() {
                    "--json" | "-json" => json = true,
                    other if other.starts_with('-') && other.len() > 1 => {
                        return usage(format!("probe: unknown option {other}"));
                    }
                    other => {
                        if input.replace(other.to_owned()).is_some() {
                            return usage("probe takes one input file");
                        }
                    }
                }
            }
            return Ok(Command::Probe {
                input: input.ok_or_else(|| Usage("probe needs an input file".into()))?,
                json,
            });
        }
        Some("-codecs") | Some("--codecs") | Some("codecs") => {
            let mode = match args.get(1).map(String::as_str) {
                None => ListMode::Text,
                Some("--json") => ListMode::Json,
                Some("--markdown") => ListMode::Markdown,
                Some(other) => return usage(format!("-codecs: unknown option {other}")),
            };
            return Ok(Command::Codecs { mode });
        }
        Some("-formats") | Some("--formats") | Some("formats") => {
            return Ok(Command::Formats {
                json: args.get(1).map(String::as_str) == Some("--json"),
            });
        }
        Some("convert") => {
            let mut positional = Vec::new();
            let mut spec = ConvertSpec::default();
            let mut overwrite = false;
            let mut at = 1;
            while at < args.len() {
                match args[at].as_str() {
                    "--formats" => {
                        spec.formats = value(args, &mut at, "--formats")?
                            .split(',')
                            .map(|s| s.trim().to_owned())
                            .filter(|s| !s.is_empty())
                            .collect()
                    }
                    "--fps" => {
                        spec.fps = Some(
                            value(args, &mut at, "--fps")?
                                .parse()
                                .map_err(|_| Usage("--fps needs a number".into()))?,
                        )
                    }
                    "--width" => {
                        spec.width = Some(
                            value(args, &mut at, "--width")?
                                .parse()
                                .map_err(|_| Usage("--width needs an integer".into()))?,
                        )
                    }
                    "--max-frames" => {
                        spec.max_frames = Some(
                            value(args, &mut at, "--max-frames")?
                                .parse()
                                .map_err(|_| Usage("--max-frames needs an integer".into()))?,
                        )
                    }
                    "-y" => overwrite = true,
                    other if other.starts_with('-') && other.len() > 1 => {
                        return usage(format!("convert: unknown option {other}"));
                    }
                    other => positional.push(other.to_owned()),
                }
                at += 1;
            }
            let [input, output_dir] = <[String; 2]>::try_from(positional)
                .map_err(|_| Usage("convert needs INPUT and OUTDIR".into()))?;
            return Ok(Command::Convert {
                input,
                output_dir,
                spec,
                overwrite,
            });
        }
        Some("mcp") => {
            let (mut read, mut write) = (Vec::new(), Vec::new());
            let mut at = 1;
            while at < args.len() {
                match args[at].as_str() {
                    "--read-root" => read.push(PathBuf::from(value(args, &mut at, "--read-root")?)),
                    "--write-root" => {
                        write.push(PathBuf::from(value(args, &mut at, "--write-root")?))
                    }
                    other => return usage(format!("mcp: unknown option {other}")),
                }
                at += 1;
            }
            if read.is_empty() && write.is_empty() {
                return usage(
                    "mcp needs at least one --read-root or --write-root; it never serves the whole file system",
                );
            }
            return Ok(Command::Mcp { read, write });
        }
        _ => {}
    }
    parse_transcode(args)
}

fn parse_transcode(args: &[String]) -> Result<Command, Usage> {
    let mut input = None;
    let mut output = None;
    let mut spec = TranscodeSpec::default();
    let mut overwrite = false;
    let mut refuse = false;
    let mut at = 0;
    while at < args.len() {
        let arg = args[at].as_str();
        match arg {
            "-i" => {
                if input
                    .replace(value(args, &mut at, "-i")?.to_owned())
                    .is_some()
                {
                    return usage("only one -i input is supported");
                }
            }
            "-y" => overwrite = true,
            "-n" => refuse = true,
            "-hide_banner" | "-nostdin" | "-stats" | "-nostats" => {}
            "-loglevel" | "-v" => {
                value(args, &mut at, arg)?;
            }
            "-f" => spec.format = Some(value(args, &mut at, "-f")?.to_owned()),
            "-c" | "-codec" => {
                let codec = value(args, &mut at, arg)?.to_owned();
                spec.video_codec = Some(codec.clone());
                if codec == "copy" {
                    spec.audio_codec = Some(codec);
                } else {
                    return usage(
                        "-c needs `copy`; name a codec with -c:v or -c:a so it cannot be applied to the wrong stream",
                    );
                }
            }
            "-c:v" | "-vcodec" | "-codec:v" => {
                spec.video_codec = Some(value(args, &mut at, arg)?.to_owned())
            }
            "-c:a" | "-acodec" | "-codec:a" => {
                spec.audio_codec = Some(value(args, &mut at, arg)?.to_owned())
            }
            "-b:v" | "-vb" => spec.video_bitrate = Some(value(args, &mut at, arg)?.to_owned()),
            "-b:a" | "-ab" => spec.audio_bitrate = Some(value(args, &mut at, arg)?.to_owned()),
            "-vf" | "-filter:v" => {
                let filter = value(args, &mut at, arg)?;
                if filter.contains(',') || !filter.starts_with("scale=") {
                    return usage(format!(
                        "-vf {filter}: only a single `scale=W:H` filter is supported"
                    ));
                }
                spec.scale = Some(filter.to_owned());
            }
            "-s" => spec.scale = Some(value(args, &mut at, arg)?.to_owned()),
            "-ss" => spec.start = Some(value(args, &mut at, arg)?.to_owned()),
            "-t" => spec.duration = Some(value(args, &mut at, arg)?.to_owned()),
            "-r" => spec.fps = Some(value(args, &mut at, arg)?.to_owned()),
            "-ac" => {
                spec.channels = Some(
                    value(args, &mut at, arg)?
                        .parse()
                        .map_err(|_| Usage("-ac needs 1 or 2".into()))?,
                )
            }
            "-ar" => {
                spec.sample_rate = Some(
                    value(args, &mut at, arg)?
                        .parse()
                        .map_err(|_| Usage("-ar needs a sample rate in Hz".into()))?,
                )
            }
            "-vn" => spec.video_codec = Some("none".into()),
            "-an" => spec.audio_codec = Some("none".into()),
            other if other.starts_with('-') && other.len() > 1 => {
                return usage(format!(
                    "unsupported option {other}; run aphrody-av -h for the supported ffmpeg-style options"
                ));
            }
            other => {
                if output.replace(other.to_owned()).is_some() {
                    return usage(format!(
                        "more than one output file ({other}); one output per run"
                    ));
                }
            }
        }
        at += 1;
    }
    let input =
        input.ok_or_else(|| Usage("no input: give -i INPUT (or a subcommand; see -h)".into()))?;
    let output = output.ok_or_else(|| Usage("no output file".into()))?;
    Ok(Command::Transcode {
        input,
        output,
        spec,
        overwrite: overwrite && !refuse,
    })
}

fn describe_probe(info: &aphrody_av::ProbeInfo) -> String {
    let mut out = format!(
        "Input: {} ({}), {} bytes, duration {}, {}\n",
        info.format,
        info.format_long,
        info.size_bytes,
        info.duration_seconds
            .map_or("unknown".into(), |d| format!("{d:.3} s")),
        info.bit_rate
            .map_or("bit rate unknown".into(), |b| format!("{} kb/s", b / 1000))
    );
    for stream in &info.streams {
        let mut line = format!(
            "  Stream #{} {} {}",
            stream.index, stream.kind, stream.codec
        );
        if let (Some(w), Some(h)) = (stream.width, stream.height) {
            line.push_str(&format!(" {w}x{h}"));
        }
        if let Some(rate) = &stream.frame_rate_ratio {
            line.push_str(&format!(" {rate} fps"));
        }
        if let Some(frames) = stream.frames {
            line.push_str(&format!(", {frames} frames"));
        }
        if let Some(rate) = stream.sample_rate {
            line.push_str(&format!(" {rate} Hz"));
        }
        if let Some(channels) = stream.channels {
            line.push_str(&format!(" {channels} ch"));
        }
        if let Some(bit_rate) = stream.bit_rate {
            line.push_str(&format!(", {} kb/s", bit_rate / 1000));
        }
        out.push_str(&line);
        out.push('\n');
    }
    out
}

fn list_codecs(mode: ListMode) -> String {
    match mode {
        ListMode::Json => serde_json::to_string_pretty(&caps::codec_caps()).unwrap_or_default(),
        ListMode::Markdown => caps::markdown(),
        ListMode::Text => {
            let mut out = String::from(
                "Codecs: D = decode, E = encode (L = decode linked but unverified); V video, A audio\n",
            );
            for codec in Codec::ALL {
                let decode = match decode_status(codec) {
                    "yes" => 'D',
                    "linked" => 'L',
                    _ => '.',
                };
                let kind = if codec.kind() == aphrody_av::TrackKind::Video {
                    'V'
                } else {
                    'A'
                };
                let containers: Vec<&str> = Format::ALL
                    .into_iter()
                    .filter(|f| can_encode(codec) && caps::muxes(*f, codec))
                    .map(Format::name)
                    .collect();
                out.push_str(&format!(
                    " {}{}{} {:<11} {} [{}]\n",
                    decode,
                    if can_encode(codec) { 'E' } else { '.' },
                    kind,
                    codec.name(),
                    codec.long_name(),
                    containers.join(",")
                ));
            }
            out
        }
    }
}

fn list_formats(json: bool) -> String {
    if json {
        return serde_json::to_string_pretty(&caps::format_caps()).unwrap_or_default();
    }
    let mut out =
        String::from("Formats: D = read, E = write (? = read path not verified by a test)\n");
    for caps in caps::format_caps() {
        out.push_str(&format!(
            " {}{}{} {:<9} {} ({})\n",
            'D',
            if caps.write { 'E' } else { '.' },
            if caps.read == "tested" { ' ' } else { '?' },
            caps.name,
            caps.long_name,
            caps.extensions.join(",")
        ));
    }
    out
}

fn fail(error: &AvError) -> ExitCode {
    eprintln!("aphrody-av: {error}");
    ExitCode::from(1)
}

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let command = match parse(&args) {
        Ok(command) => command,
        Err(Usage(message)) => {
            eprintln!("aphrody-av: {message}\nrun `aphrody-av -h` for usage");
            return ExitCode::from(2);
        }
    };
    match command {
        Command::Help => {
            println!("{USAGE}");
            ExitCode::SUCCESS
        }
        Command::Version => {
            println!("aphrody-av {}", env!("CARGO_PKG_VERSION"));
            ExitCode::SUCCESS
        }
        Command::Codecs { mode } => {
            println!("{}", list_codecs(mode));
            ExitCode::SUCCESS
        }
        Command::Formats { json } => {
            println!("{}", list_formats(json));
            ExitCode::SUCCESS
        }
        Command::Probe { input, json } => match probe_file(&Sandbox::open(), &input) {
            Ok(info) => {
                if json {
                    println!(
                        "{}",
                        serde_json::to_string_pretty(&info).unwrap_or_default()
                    );
                } else {
                    print!("{}", describe_probe(&info));
                }
                ExitCode::SUCCESS
            }
            Err(error) => fail(&error),
        },
        Command::Transcode {
            input,
            output,
            spec,
            overwrite,
        } => match transcode_file(&Sandbox::open(), &input, &output, &spec, overwrite) {
            Ok(report) => {
                let part = |label: &str, stream: &Option<aphrody_av::transcode::StreamReport>| {
                    stream.as_ref().map(|s| {
                        let size = s
                            .width
                            .zip(s.height)
                            .map_or(String::new(), |(w, h)| format!(" {w}x{h}"));
                        format!(
                            "{label} {} ({}, {} {}{size})",
                            s.codec,
                            s.mode,
                            s.frames,
                            if label == "video" {
                                "frames"
                            } else {
                                "sample frames"
                            }
                        )
                    })
                };
                let streams: Vec<String> =
                    [part("video", &report.video), part("audio", &report.audio)]
                        .into_iter()
                        .flatten()
                        .collect();
                eprintln!(
                    "wrote {output}: {} -> {}, {} bytes; {}",
                    report.input_format,
                    report.output_format,
                    report.output_bytes,
                    streams.join("; ")
                );
                ExitCode::SUCCESS
            }
            Err(error) => fail(&error),
        },
        Command::Convert {
            input,
            output_dir,
            spec,
            overwrite,
        } => match convert_file(&Sandbox::open(), &input, &output_dir, &spec, overwrite) {
            Ok(summary) => {
                eprintln!(
                    "wrote {} files to {output_dir}: {} frames from {} decoded",
                    summary.files.len(),
                    summary.frames,
                    summary.decoded
                );
                ExitCode::SUCCESS
            }
            Err(error) => fail(&error),
        },
        Command::Mcp { read, write } => {
            let sandbox = match Sandbox::restricted(&read, &write) {
                Ok(sandbox) => sandbox,
                Err(error) => return fail(&error),
            };
            let runtime = match tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
            {
                Ok(runtime) => runtime,
                Err(error) => {
                    eprintln!("aphrody-av: cannot start the async runtime: {error}");
                    return ExitCode::from(1);
                }
            };
            match runtime.block_on(aphrody_av::mcp::serve_stdio(sandbox)) {
                Ok(()) => ExitCode::SUCCESS,
                Err(message) => {
                    eprintln!("aphrody-av: {message}");
                    ExitCode::from(1)
                }
            }
        }
    }
}
