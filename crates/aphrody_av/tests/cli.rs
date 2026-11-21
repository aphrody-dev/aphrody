#![allow(
    clippy::disallowed_methods,
    reason = "the test drives a separate process and no GPUI executor is involved"
)]
mod common;
use aphrody_av::{Codec, Format};
use common::*;
use serde_json::{Value, json};
use std::io::{BufRead, BufReader, Write};
use std::process::{Command, Stdio};

const BIN: &str = env!("CARGO_BIN_EXE_aphrody-av");

fn run(args: &[&str]) -> (i32, String, String) {
    let output = Command::new(BIN)
        .args(args)
        .output()
        .expect("spawn aphrody-av");
    (
        output.status.code().unwrap_or(-1),
        String::from_utf8_lossy(&output.stdout).into_owned(),
        String::from_utf8_lossy(&output.stderr).into_owned(),
    )
}

#[test]
fn ffmpeg_style_transcode_and_probe() {
    let directory = tempfile::tempdir().expect("tempdir");
    let input = directory.path().join("in.mp4");
    std::fs::write(
        &input,
        av_file(Format::Mp4, Codec::H264, Codec::Aac, 20, (64, 48)),
    )
    .expect("write");
    let (input, out_mkv, out_webm) = (
        input.to_str().expect("utf8").to_owned(),
        directory.path().join("out.mkv"),
        directory.path().join("out.webm"),
    );

    let (code, _, stderr) = run(&[
        "-i",
        &input,
        "-c:v",
        "vp9",
        "-c:a",
        "opus",
        "-b:v",
        "200k",
        "-vf",
        "scale=32:-1",
        "-ss",
        "0.1",
        "-t",
        "0.4",
        out_webm.to_str().expect("utf8"),
    ]);
    assert_eq!(code, 0, "{stderr}");
    let (code, stdout, _) = run(&["probe", out_webm.to_str().expect("utf8"), "--json"]);
    assert_eq!(code, 0);
    let info: Value = serde_json::from_str(&stdout).expect("json");
    assert_eq!(info["format"], "webm");
    assert_eq!(info["streams"][0]["codec"], "vp9");
    assert_eq!(
        (
            info["streams"][0]["width"].as_u64(),
            info["streams"][0]["height"].as_u64()
        ),
        (Some(32), Some(24))
    );
    assert_eq!(info["streams"][0]["frames"], 12);
    assert_eq!(info["streams"][1]["codec"], "opus");

    let (code, _, stderr) = run(&["-i", &input, "-c", "copy", out_mkv.to_str().expect("utf8")]);
    assert_eq!(code, 0, "{stderr}");
    let (_, stdout, _) = run(&["probe", out_mkv.to_str().expect("utf8")]);
    assert!(
        stdout.contains("matroska") && stdout.contains("h264") && stdout.contains("aac"),
        "{stdout}"
    );

    // refuses to overwrite without -y
    let (code, _, stderr) = run(&["-i", &input, "-c", "copy", out_mkv.to_str().expect("utf8")]);
    assert_eq!(code, 1);
    assert!(stderr.contains("already exists"), "{stderr}");
    let (code, _, _) = run(&[
        "-y",
        "-i",
        &input,
        "-c",
        "copy",
        out_mkv.to_str().expect("utf8"),
    ]);
    assert_eq!(code, 0);
}

#[test]
fn listings_and_precise_failures() {
    let (code, stdout, _) = run(&["-codecs"]);
    assert_eq!(code, 0);
    assert!(
        stdout.contains("h264") && stdout.contains("DEV"),
        "{stdout}"
    );
    let (code, stdout, _) = run(&["-formats"]);
    assert_eq!(code, 0);
    assert!(stdout.contains("matroska"));
    let (_, markdown, _) = run(&["-codecs", "--markdown"]);
    assert_eq!(markdown.trim_end(), aphrody_av::caps::markdown().trim_end());

    let directory = tempfile::tempdir().expect("tempdir");
    let input = directory.path().join("in.mp4");
    std::fs::write(
        &input,
        av_file(Format::Mp4, Codec::H264, Codec::Aac, 6, (64, 48)),
    )
    .expect("write");
    let input = input.to_str().expect("utf8");
    let output = |name: &str| {
        directory
            .path()
            .join(name)
            .to_str()
            .expect("utf8")
            .to_owned()
    };

    let (code, _, stderr) = run(&["-i", input, "-c:v", "h264", &output("o.webm")]);
    assert_eq!(
        (code, stderr.contains("webm cannot hold h264")),
        (1, true),
        "{stderr}"
    );
    let (code, _, stderr) = run(&["-i", input, "-c:v", "nonsense", &output("o.mp4")]);
    assert_eq!(
        (code, stderr.contains("unknown codec \"nonsense\"")),
        (1, true),
        "{stderr}"
    );
    let (code, _, stderr) = run(&["-i", input, "-vf", "hue=s=0", &output("o.mp4")]);
    assert_eq!(
        (code, stderr.contains("only a single `scale=W:H` filter")),
        (2, true),
        "{stderr}"
    );
    let (code, _, stderr) = run(&["-i", input, "-bogus", "1", &output("o.mp4")]);
    assert_eq!(
        (code, stderr.contains("unsupported option -bogus")),
        (2, true),
        "{stderr}"
    );
    let (code, _, stderr) = run(&["-i", input, "-c", "copy", "-ss", "1", &output("o.mp4")]);
    assert_eq!(
        (code, stderr.contains("need a re-encode")),
        (1, true),
        "{stderr}"
    );
    let (code, _, stderr) = run(&["-i", input, &output("o.xyz")]);
    assert_eq!(
        (code, stderr.contains("cannot infer the output format")),
        (1, true),
        "{stderr}"
    );
    let (code, _, stderr) = run(&["probe", &output("missing.mp4")]);
    assert_eq!(code, 1);
    assert!(stderr.contains("i/o error"), "{stderr}");
    let garbage = directory.path().join("garbage.bin");
    std::fs::write(&garbage, vec![0x5Au8; 5000]).expect("write");
    let (code, _, stderr) = run(&["probe", garbage.to_str().expect("utf8")]);
    assert_eq!(
        (code, stderr.contains("unrecognised container")),
        (1, true),
        "{stderr}"
    );
}

#[test]
fn convert_subcommand_writes_documents() {
    let directory = tempfile::tempdir().expect("tempdir");
    let input = directory.path().join("clip.mp4");
    std::fs::write(
        &input,
        av_file(Format::Mp4, Codec::H264, Codec::Aac, 12, (64, 48)),
    )
    .expect("write");
    let out = directory.path().join("doc");
    let (code, _, stderr) = run(&[
        "convert",
        input.to_str().expect("utf8"),
        out.to_str().expect("utf8"),
        "--formats",
        "svg,md",
        "--fps",
        "5",
        "--width",
        "32",
    ]);
    assert_eq!(code, 0, "{stderr}");
    assert!(
        out.join("animated.svg").exists()
            && out.join("video.md").exists()
            && out.join("frames/frame-00001.svg").exists()
    );
    let (code, _, stderr) = run(&[
        "convert",
        input.to_str().expect("utf8"),
        out.to_str().expect("utf8"),
        "--formats",
        "pdf",
    ]);
    assert_eq!(
        (code, stderr.contains("unknown convert format")),
        (1, true),
        "{stderr}"
    );
}

struct Mcp {
    child: std::process::Child,
    reader: BufReader<std::process::ChildStdout>,
    next_id: u64,
}

impl Mcp {
    fn start(read: &std::path::Path, write: &std::path::Path) -> Mcp {
        let mut child = Command::new(BIN)
            .args([
                "mcp",
                "--read-root",
                read.to_str().expect("utf8"),
                "--write-root",
                write.to_str().expect("utf8"),
            ])
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
            .expect("spawn mcp");
        let reader = BufReader::new(child.stdout.take().expect("stdout"));
        Mcp {
            child,
            reader,
            next_id: 1,
        }
    }

    fn send(&mut self, message: &Value) {
        let stdin = self.child.stdin.as_mut().expect("stdin");
        writeln!(stdin, "{message}").expect("write");
        stdin.flush().expect("flush");
    }

    fn request(&mut self, method: &str, params: Value) -> Value {
        let id = self.next_id;
        self.next_id += 1;
        self.send(&json!({ "jsonrpc": "2.0", "id": id, "method": method, "params": params }));
        loop {
            let mut line = String::new();
            assert!(
                self.reader.read_line(&mut line).expect("read") > 0,
                "server closed the pipe"
            );
            let value: Value = serde_json::from_str(line.trim()).expect("json-rpc line");
            if value.get("id") == Some(&json!(id)) {
                return value;
            }
        }
    }

    fn call(&mut self, tool: &str, arguments: Value) -> (bool, String) {
        let response = self.request(
            "tools/call",
            json!({ "name": tool, "arguments": arguments }),
        );
        let result = &response["result"];
        let text = result["content"][0]["text"]
            .as_str()
            .unwrap_or_default()
            .to_owned();
        (result["isError"].as_bool().unwrap_or(false), text)
    }
}

impl Drop for Mcp {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

#[test]
fn mcp_session_over_stdio_with_root_confinement() {
    let read = tempfile::tempdir().expect("read root");
    let write = tempfile::tempdir().expect("write root");
    let outside = tempfile::tempdir().expect("outside");
    std::fs::write(
        read.path().join("in.mp4"),
        av_file(Format::Mp4, Codec::H264, Codec::Aac, 12, (64, 48)),
    )
    .expect("write");
    std::fs::write(
        outside.path().join("secret.mp4"),
        av_file(Format::Mp4, Codec::H264, Codec::Aac, 3, (64, 48)),
    )
    .expect("write");

    let mut mcp = Mcp::start(read.path(), write.path());
    let init = mcp.request("initialize", json!({ "protocolVersion": "2025-06-18", "capabilities": {}, "clientInfo": { "name": "aphrody-av-test", "version": "0" } }));
    assert!(
        init["result"]["capabilities"]["tools"].is_object(),
        "{init}"
    );
    mcp.send(&json!({ "jsonrpc": "2.0", "method": "notifications/initialized" }));

    let tools = mcp.request("tools/list", json!({}));
    let names: Vec<&str> = tools["result"]["tools"]
        .as_array()
        .expect("tools")
        .iter()
        .filter_map(|t| t["name"].as_str())
        .collect();
    assert_eq!(
        names
            .iter()
            .copied()
            .filter(|n| ["probe", "transcode", "convert", "capabilities"].contains(n))
            .count(),
        4,
        "{names:?}"
    );

    let input = read.path().join("in.mp4");
    let (error, text) = mcp.call("probe", json!({ "path": input }));
    assert!(!error, "{text}");
    let info: Value = serde_json::from_str(&text).expect("probe json");
    assert_eq!(info["streams"][0]["codec"], "h264");

    let output = write.path().join("out.mkv");
    let (error, text) = mcp.call(
        "transcode",
        json!({ "input": input, "output": output, "video_codec": "av1", "audio_codec": "opus" }),
    );
    assert!(!error, "{text}");
    let report: Value = serde_json::from_str(&text).expect("report json");
    assert_eq!(report["output_format"], "matroska");
    assert!(output.exists());
    let (error, text) = mcp.call("probe", json!({ "path": output }));
    assert!(
        error,
        "the output lives in the write root, which is not readable: {text}"
    );
    assert!(text.contains("outside the read roots"), "{text}");

    let documents = write.path().join("docs");
    let (error, text) = mcp.call("convert", json!({ "input": input, "output_dir": documents, "formats": ["svg"], "fps": 4, "width": 32 }));
    assert!(!error, "{text}");
    assert!(documents.join("animated.svg").exists());

    // confinement
    let (error, text) = mcp.call(
        "probe",
        json!({ "path": outside.path().join("secret.mp4") }),
    );
    assert!(error && text.contains("outside the read roots"), "{text}");
    let (error, text) = mcp.call("probe", json!({ "path": read.path().join("../").join(outside.path().file_name().expect("name")).join("secret.mp4") }));
    assert!(error && text.contains("outside the read roots"), "{text}");
    let (error, text) = mcp.call(
        "transcode",
        json!({ "input": input, "output": outside.path().join("stolen.mp4") }),
    );
    assert!(error && text.contains("outside the write roots"), "{text}");
    assert!(!outside.path().join("stolen.mp4").exists());
    let (error, text) = mcp.call(
        "transcode",
        json!({ "input": input, "output": read.path().join("injected.mp4") }),
    );
    assert!(error && text.contains("outside the write roots"), "{text}");
    let (error, text) = mcp.call(
        "transcode",
        json!({ "input": input, "output": write.path().join("..").join("x.mp4") }),
    );
    assert!(error && text.contains("`..`"), "{text}");
    let (error, text) = mcp.call(
        "convert",
        json!({ "input": input, "output_dir": outside.path().join("docs") }),
    );
    assert!(error && text.contains("outside the write roots"), "{text}");
    assert!(!outside.path().join("docs").exists());
    // overwrite refusal and unsupported combination messages reach the client
    let (error, text) = mcp.call("transcode", json!({ "input": input, "output": output }));
    assert!(error && text.contains("already exists"), "{text}");
    let (error, text) = mcp.call(
        "transcode",
        json!({ "input": input, "output": write.path().join("w.webm"), "video_codec": "h264" }),
    );
    assert!(error && text.contains("webm cannot hold h264"), "{text}");
    let (error, text) = mcp.call("capabilities", json!({}));
    assert!(!error && text.contains("\"h264\""), "{text}");
}

#[test]
fn mcp_refuses_to_start_without_roots() {
    let (code, _, stderr) = run(&["mcp"]);
    assert_eq!(code, 2);
    assert!(
        stderr.contains("at least one --read-root or --write-root"),
        "{stderr}"
    );
}
