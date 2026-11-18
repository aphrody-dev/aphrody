# hub

A Hugging Face Hub compatible repository service: the official clients work against it unchanged by setting `HF_ENDPOINT`. Built by the replica workflow (`tools/replica`, recon in `replica/huggingface`) from the Hub's published API description.

| Mode | What it serves |
|---|---|
| Hosted | repositories created and pushed here: create, preupload, NDJSON commits, LFS basic transfer, revisions, refs, commits, tree, paths-info, resolve with the headers clients read (`X-Repo-Commit`, `ETag`, `X-Linked-Etag`, `X-Linked-Size`, ranges), listing and search, `whoami-v2`, private repositories |
| Mirror | anything not hosted, fetched from `HUB_UPSTREAM` (for example `https://huggingface.co`), cached by content on disk and served from the cache afterwards; upstream redirects (`/api/resolve-cache/...`) are mirrored too |
| Web | the same paths in a browser (`Accept: text/html`): home, model, dataset and space lists with search, repository pages |

Large files never sit in memory: uploads stream to disk while their SHA-256 is checked, downloads and mirror answers stream from disk, one thread per request. LFS upload links are signed for the object and the user and expire after an hour, so no token travels in a URL.

## Settings

| Variable | Default |
|---|---|
| `HUB_ADDR` | `127.0.0.1:8090` |
| `HUB_DATA` | `hub-data` |
| `HUB_UPSTREAM`, `HUB_UPSTREAM_TOKEN` | none: hosted only |
| `HUB_TOKENS` | `token=user;...`; empty means an open, single-user hub |
| `HUB_PUBLIC_URL` | taken from the request's `Host` |
| `HUB_LFS_THRESHOLD` | 10 MiB |
| `HUB_MAX_BODY` | 64 MiB (JSON and commit bodies, read whole) |
| `HUB_MAX_OBJECT` | 100 GiB (one LFS object, streamed to disk) |
| `HUB_MAX_CONNECTIONS` | 256 requests at once; more are answered 503 |
| `HUB_SECRET` | random per process; set it when several processes serve one data directory |
| `HUB_OPEN` | `1` lets a hub without `HUB_TOKENS` serve beyond loopback; otherwise it refuses to start |

Beyond loopback the hub needs `HUB_TOKENS` and `HUB_PUBLIC_URL` (the https address of the proxy in front of it).

## Verified

- `cargo test -p hub`: 8 tests (read and write paths, ranges, signed upload links, HMAC test vector, path segments and revision names).
- `tests/hf_client.py` with the latest `huggingface_hub` against a hosted hub: whoami, create_repo, upload_file, upload_folder with an LFS file, model_info, list_repo_files, hf_hub_download, LFS download, snapshot_download, list_models, list_repo_commits, list_repo_refs, file_exists, delete_file.
- Mirror of huggingface.co with `huggingface_hub`: model_info, hf_hub_download twice (second from the cache), snapshot_download with patterns; bytes identical to a direct download.

## Not yet

Xet storage (clients run with `HF_HUB_DISABLE_XET=1`), git over HTTP, users and organisations beyond static tokens, discussions, collections, buckets, jobs, the Rust `hf-hub` and JavaScript `@huggingface/hub` clients, a Material 3 web interface. The feature matrix and parity are in `replica/huggingface`.
