# file-uploader

A small, stateless file storage service written in Rust (Axum + Tokio). Access is granted through S3-style presigned URLs (per-file, per-operation JWTs); uploads are written atomically (`.tmp` + `rename`) and both uploads and downloads are streamed, so memory use doesn't depend on file size.

Used by [personal_photos](https://github.com/artiimor/personal_photos) as a custom Active Storage service.

---

HTTP service in Rust ([Axum](https://github.com/tokio-rs/axum) 0.8 + Tokio) to upload, download, delete and inspect files (built with photos in mind). It has no users and no database: **another backend signs a JWT** that authorizes one specific operation on one specific file, and this service only verifies it and carries it out.

```
Client ──(1) asks for permission──▶ Your backend ──(2) signs JWT──▶ Client
Client ──(3) PUT /files/{id}/{file}?token=JWT──▶ file-uploader ──▶ disk
```

## Quick start

```bash
cp .env.examples .env    # then fill in the values
docker compose up -d --build
curl localhost:3050/health
```

Without Docker:

```bash
cargo run --release
```

### Environment variables

| Variable          | Example                  | What it does |
|-------------------|--------------------------|--------------|
| `JWT_SECRET`      | `a-long-secret`          | HMAC secret shared with the backend that signs the tokens. |
| `ALLOWED_ORIGINS` | `http://localhost:6767`  | Origin allowed by CORS. **It is a single origin**, not a list. |
| `MAX_BODY_LIMIT`  | `10`                     | Maximum request body size, **in MB**. |
| `FILES_ROOT_PATH` | `files`                  | Root folder where files are stored (`{FILES_ROOT_PATH}/{id}/{file_name}`). Relative to the working directory, or absolute. |

They are loaded with `dotenvy` from `.env` if it exists; otherwise from the process environment (which is what `docker compose` does with `env_file`). Variables already set in the environment take precedence over `.env`.

If any of them is missing, the server panics: `ALLOWED_ORIGINS`, `MAX_BODY_LIMIT` and `FILES_ROOT_PATH` are read when the router is built (at startup), and `JWT_SECRET` on every request that validates a token. Failing early beats starting up misconfigured.

> **With Docker**, `FILES_ROOT_PATH` must point to the folder mounted as a volume: `files` (the `WORKDIR` is `/app`) or `/app/files`. With any other path, files are stored inside the container and are lost when it is recreated.

## API

Every file route takes the token in the query string: `?token=<JWT>`.

| Method   | Route                                 | JWT scope     | Success response |
|----------|---------------------------------------|---------------|------------------|
| `GET`    | `/health`                             | —             | `200` |
| `GET`    | `/files/{id}/{file_name}`             | `download`    | `200` + streamed file |
| `PUT`    | `/files/{id}/{file_name}`             | `upload`      | `200 File uploaded!` |
| `DELETE` | `/files/{id}/{file_name}`             | `delete`      | `200 file {path} deleted successfully!` |
| `GET`    | `/files/{id}/{file_name}/metadata`    | `metadata`    | `200 The file size is {n} bytes!` |

Downloads are served with `Content-Type: image/{extension}` and `Content-Disposition: inline`, so the browser can display them directly in an `<img>`.

### JWT format

Signed with HS256 using `JWT_SECRET`:

```json
{
  "sub": "user_123",          // must match {id} in the URL
  "scope": "upload",          // must match the operation
  "file_name": "photo.jpg",   // must match {file_name} in the URL
  "exp": 1767225600           // required; expiration is validated (with 60 s of leeway)
}
```

### Uploading

`PUT` accepts two formats, depending on the `Content-Type`:

- **Direct**: the request body is the file as-is.
  ```bash
  curl -X PUT --data-binary @photo.jpg "localhost:3050/files/user_123/photo.jpg?token=$TOKEN"
  ```
- **Multipart** (`multipart/form-data`): the **first field that is a file** is stored; text fields and any later files are ignored.
  ```bash
  curl -X PUT -F "file=@photo.jpg" "localhost:3050/files/user_123/photo.jpg?token=$TOKEN"
  ```

If the file already exists, it is overwritten.

### Errors

| Code   | When |
|--------|------|
| `400`  | Invalid `id` or `file_name`; malformed or empty multipart, or one with no file field |
| `401`  | Missing, badly signed or expired token (`Error, auth is not valid`); or a valid token for another user/file/scope (`URL is not valid`) |
| `404`  | The file does not exist |
| `406`  | The file has no extension (download) |
| `413`  | The body exceeds `MAX_BODY_LIMIT` (direct or multipart upload) |
| `500`  | Disk failure |

The token is checked **before** the `id` and `file_name` regexes: without a valid token for that URL, the response is always `401`, giving no hints about which paths are valid.

## Design decisions

### Per-operation JWT authorization (*presigned URL* style)

Instead of sessions, API keys or an `Authorization: Bearer` header, each URL carries a token that authorizes **a single operation (`scope`) on a single file (`sub` + `file_name`)** and expires (`exp`).

- **Why**: the service doesn't need to know anything about users or keep any state. The main backend decides who can do what and signs the URL; this service only checks the signature. It's the same model as S3 *presigned URLs*.
- **Why in the query and not in a header**: so the URL works directly in an `<img src="...">` or a download link, where the browser doesn't let you add headers.
- **Why bind the token to the file and the scope**: a leaked download token for `photo.jpg` can't be used to delete it or to read another file.
- **Why `token` is an `Option` in `Params`**: if it were required, the `Query` extractor would reject the request with `400` before reaching the handler. Being optional, the handler decides and returns `401`, which is the right answer for "missing credential".
- **Trade-off**: tokens in URLs can end up in logs and browser history. That's why `exp` should be short (minutes).

There used to be an API key and a Bearer token with middleware (see git history); they were removed in favor of this simpler model.

### Configuration in `State` (`AppConfig`)

`FILES_ROOT_PATH` is read once when the router is built and stored in `AppConfig` (in `src/lib.rs`), which is passed to the handlers with `Router::with_state` and extracted with `State<AppConfig>`. This way handlers don't read the environment on every request and the path isn't hardcoded.

### Validating `id` and `file_name` with regexes

- `id`: `^[A-Za-z0-9_-]+$`
- `file_name`: `^[A-Za-z0-9_-]+\.[A-Za-z0-9]+$` (name + one extension)

Paths on disk are built as `{FILES_ROOT_PATH}/{id}/{file_name}`, so these regexes are the defense against **path traversal** (`../../etc/passwd`): `/`, `..` and odd characters are not allowed. They are allowlists and deliberately strict. The regexes are compiled only once with `LazyLock`.

### Atomic writes: `.tmp` + `rename`

Uploads are first written to `{FILES_ROOT_PATH}/{id}/{file_name}.tmp` and, once they finish successfully, `rename`d to the final name.

- **Why**: if an upload fails halfway (network drop, size limit exceeded), the previous file **is not corrupted** and no half-written files are visible. `rename` within the same filesystem is atomic.
- If something fails, the `.tmp` is deleted so no garbage is left behind (there are tests that check this).

### Streaming instead of loading into memory

Uploads and downloads go through streams (`into_data_stream`, `field.chunk()`, `ReaderStream`): data is written or sent in chunks. Memory usage doesn't depend on file size.

### Two upload formats

Both a direct body and multipart are accepted because they serve different clients: `fetch(url, { method: 'PUT', body: file })` sends a direct body; a `<form>` or `FormData` sends multipart. The format is chosen by looking at the `Content-Type`.

### `PUT` for uploads

The URL already identifies the resource (`/files/{id}/{file_name}`) and repeating the upload leaves the same result (it overwrites). That's `PUT` semantics (idempotent), not `POST`.

### Size limit: two layers

The router enforces `MAX_BODY_LIMIT` with two layers, because each one covers a different path:

- **`DefaultBodyLimit::max`** (Axum): only honored by Axum extractors such as `Multipart`. Covers multipart uploads.
- **`RequestBodyLimitLayer`** (`tower-http`, `limit` feature): limits the raw request body. Needed for direct uploads, which read the `Request` by hand with `into_data_stream()` and therefore bypass `DefaultBodyLimit`. If the `Content-Length` already exceeds the limit, it responds `413` without reaching the handler.

### CORS

`tower-http::CorsLayer` with a single origin (`ALLOWED_ORIGINS`), the `GET`, `PUT`, `DELETE` and `OPTIONS` (the browser *preflight*) methods, and the `Content-Type` header allowed, which the browser needs to `PUT` multipart or a specific type. This way only your frontend can call the service from the browser.

### Typed errors

All errors are variants of `ResponseError` (in `src/lib.rs`), which implements `IntoResponse`. Handlers return `Result<Response, ResponseError>` and use `?`; the mapping to HTTP status codes lives in one place. The upload handler returns `Result<Response, Response>` because it also passes Axum extractor errors through as-is (for example, the multipart `413`).

### `lib.rs` + `main.rs`

The app is built in `lib.rs` (`app()`) and `main.rs` only serves it. This lets the integration tests (`tests/`) import the app and start it on a random port (`0.0.0.0:0`) without duplicating code.

## Docker

### `Dockerfile`: multi-stage build

1. **Builder** (`rust:1-slim-bookworm`): compiles in release mode. It first copies only `Cargo.toml` and `Cargo.lock` and builds an empty `main`, so **dependencies end up in a cached layer**. When code in `src/` changes, only the crate is rebuilt, not all of Tokio/Axum.
2. **Runtime** (`debian:bookworm-slim`): just the binary, with `WORKDIR /app` and the `/app/files` folder created. The final image doesn't include the Rust toolchain (hundreds of MB smaller and a smaller attack surface).

Both stages use the same Debian base (bookworm) so the binary links against the same glibc. OpenSSL isn't needed at runtime: `jsonwebtoken` uses the `rust_crypto` backend (pure Rust).

### `docker-compose.yml`

- **`ports: 3050:3050`**: the server binds to `0.0.0.0:3050`. With `127.0.0.1` it wouldn't be reachable from outside the container.
- **`env_file: .env`**: configuration is passed at runtime, not baked into the image.
- **`volumes: ./files:/app/files`**: uploaded files live on the host. Without the volume they would be lost when the container is recreated. Remember that `FILES_ROOT_PATH` must point here (`files` or `/app/files`).
- **`restart: unless-stopped`**: if the process dies (for example, a `panic`), Docker brings it back up.

### `.dockerignore`

Excludes `target/` (avoids sending GBs of context to the build), `files/`, `.git` and the `.env` files. Secrets must **never** end up inside the image.

## Tests and CI

```bash
cargo test
cargo clippy --all-targets -- -D warnings
```

- 11 unit tests in `src/handlers/photos.rs` (regexes and JWT validation) and 30 integration tests in `tests/handlers_tests.rs`, which start the real server and send it requests with `reqwest`. They all pass right now.
- They cover: tokens wrong by user/file/scope, missing or expired tokens, status codes for each error, size limits (direct and multipart), multipart edge cases, overwriting, and no `.tmp` files left after failures.
- They need `JWT_SECRET`, `ALLOWED_ORIGINS`, `MAX_BODY_LIMIT` and `FILES_ROOT_PATH` in the environment (or in `.env`).
- **Heads up**: the integration tests still hardcode the `files/` path to create and check files, so they need `FILES_ROOT_PATH=files`. Any other value makes them fail.
- GitHub Actions (`.github/workflows/ci.yml`) sets those four variables to test values and runs clippy with warnings as errors plus the tests on every push to `main` and on every PR.

## Layout

```
src/
├── main.rs              # loads .env and starts the server on 0.0.0.0:3050
├── lib.rs               # AppConfig, ResponseError + app()
├── routes/mod.rs        # router, config loading, CORS, size limits
└── handlers/
    ├── mod.rs
    └── photos.rs        # handlers, JWT and regex validation, disk writes
tests/
└── handlers_tests.rs    # integration tests
files/                   # default storage (mounted as a volume in Docker)
```

## License

[MIT](LICENSE)
