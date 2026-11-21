"""UniFFI Python smoke test: AV_BINDINGS=<dir with aphrody_av.py and the cdylib> python3 -I python_smoke.py <clip.mp4>"""
import os
import sys

sys.path.insert(0, os.environ["AV_BINDINGS"])
import aphrody_av as av  # noqa: E402

data = open(sys.argv[1], "rb").read()
print("aphrody-av", av.version())

info = av.probe_bytes(data, "clip.mp4")
assert info.format == "mp4", info.format
assert [s.codec for s in info.streams] == ["h264", "aac"], info.streams
assert (info.streams[0].width, info.streams[0].height, info.streams[0].frames) == (64, 48, 12)
print("probe:", info.format, [(s.kind, s.codec) for s in info.streams], round(info.duration_seconds, 3), "s")

webm = av.transcode_bytes(data, "clip.mp4", "webm", av.TranscodeOptionsRecord(video_codec="vp9", audio_codec="opus", scale="32:-1"))
assert webm[:4] == b"\x1a\x45\xdf\xa3", webm[:4]
again = av.probe_bytes(bytes(webm), None)
assert again.format == "webm" and again.streams[0].codec == "vp9" and again.streams[0].width == 32, again
print("transcode: mp4 -> webm", len(webm), "bytes,", again.streams[0].frames, "frames at 32x24")

files = av.convert_bytes(data, "clip.mp4", ["svg", "md"], 5.0, 32)
paths = [f.path for f in files]
assert "animated.svg" in paths and "video.md" in paths, paths
assert next(f for f in files if f.path == "animated.svg").data.startswith(b"<svg")
print("convert:", len(files), "files")

try:
    av.probe_bytes(b"this is not a media file at all", None)
    raise SystemExit("garbage was accepted")
except av.AvFailure.Malformed as error:
    print("garbage rejected:", error)

try:
    av.transcode_bytes(data, "clip.mp4", "webm", av.TranscodeOptionsRecord(video_codec="h264"))
    raise SystemExit("unsupported combination was accepted")
except av.AvFailure.Unsupported as error:
    assert "webm cannot hold h264" in str(error), error
    print("unsupported rejected:", error)

caps = av.capabilities_json()
assert '"h264"' in caps
print("python smoke: ok")
