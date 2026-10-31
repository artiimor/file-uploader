# file-uploader

Servicio HTTP en Rust ([Axum](https://github.com/tokio-rs/axum) 0.8 + Tokio) para subir, descargar, borrar y consultar ficheros (pensado para fotos). No tiene usuarios ni base de datos: **otro backend firma un JWT** que autoriza una operación concreta sobre un fichero concreto, y este servicio solo lo verifica y ejecuta.

```
Cliente ──(1) pide permiso──▶ Tu backend ──(2) firma JWT──▶ Cliente
Cliente ──(3) PUT /files/{id}/{file}?token=JWT──▶ file-uploader ──▶ disco
```

## Arranque rápido

```bash
cp .env.examples .env    # y rellena los valores
docker compose up -d --build
curl localhost:3050/health
```

Sin Docker:

```bash
cargo run --release
```

### Variables de entorno

| Variable          | Ejemplo                  | Qué hace |
|-------------------|--------------------------|----------|
| `JWT_SECRET`      | `una-clave-larga`        | Secreto HMAC compartido con el backend que firma los tokens. |
| `ALLOWED_ORIGINS` | `http://localhost:6767`  | Origen permitido por CORS. **Es un único origen**, no una lista. |
| `MAX_BODY_LIMIT`  | `10`                     | Tamaño máximo del cuerpo de la petición, **en MB**. |

Se cargan con `dotenvy` desde `.env` si existe; si no, del entorno del proceso (que es lo que hace `docker compose` con `env_file`). Si falta alguna, el servidor hace `panic` al arrancar (o en la primera petición, en el caso de `JWT_SECRET`): preferimos fallar pronto a arrancar mal configurado.

## API

Todas las rutas de ficheros llevan el token en la query: `?token=<JWT>`.

| Método   | Ruta                                  | Scope del JWT | Respuesta OK |
|----------|---------------------------------------|---------------|--------------|
| `GET`    | `/health`                             | —             | `200` |
| `GET`    | `/files/{id}/{file_name}`             | `download`    | `200` + fichero en streaming |
| `PUT`    | `/files/{id}/{file_name}`             | `upload`      | `200 File uploaded!` |
| `DELETE` | `/files/{id}/{file_name}`             | `delete`      | `200` |
| `GET`    | `/files/{id}/{file_name}/metadata`    | `metadata`    | `200` + tamaño en bytes |

### Formato del JWT

Firmado con HS256 usando `JWT_SECRET`:

```json
{
  "sub": "user_123",          // debe coincidir con {id} de la URL
  "scope": "upload",          // debe coincidir con la operación
  "file_name": "foto.jpg",    // debe coincidir con {file_name} de la URL
  "exp": 1767225600           // obligatorio; se valida la expiración
}
```

### Subida

`PUT` acepta dos formatos, según el `Content-Type`:

- **Directo**: el cuerpo de la petición es el fichero tal cual.
  ```bash
  curl -X PUT --data-binary @foto.jpg "localhost:3050/files/user_123/foto.jpg?token=$TOKEN"
  ```
- **Multipart** (`multipart/form-data`): se guarda el **primer campo que sea un fichero**; los campos de texto y los ficheros posteriores se ignoran.
  ```bash
  curl -X PUT -F "file=@foto.jpg" "localhost:3050/files/user_123/foto.jpg?token=$TOKEN"
  ```

Si el fichero ya existe, se sobrescribe.

### Errores

| Código | Cuándo |
|--------|--------|
| `400`  | `id` o `file_name` no válidos, multipart mal formado o sin fichero |
| `401`  | Token ausente, mal firmado o caducado; o token válido pero para otro usuario/fichero/scope |
| `404`  | El fichero no existe |
| `406`  | El fichero no tiene extensión (descarga) |
| `413`  | El cuerpo supera `MAX_BODY_LIMIT` |
| `500`  | Fallo de disco |

> **Estado actual**: la tabla describe el comportamiento objetivo, que fijan los tests. Ahora mismo 4 tests fallan:
> - Sin `?token=` se devuelve `400` en vez de `401` (falla el extractor `Query` antes de llegar al handler).
> - Subida directa por encima del límite: `200` en vez de `413`.
> - Multipart por encima del límite: `500` en vez de `413`.
> - Multipart vacío: `500` en vez de `400`.

## Decisiones de diseño

### Autorización con JWT por operación (estilo *presigned URL*)

En lugar de sesiones, API keys o un `Authorization: Bearer`, cada URL lleva un token que autoriza **una sola operación (`scope`) sobre un solo fichero (`sub` + `file_name`)** y caduca (`exp`).

- **Por qué**: el servicio no necesita saber nada de usuarios ni guardar estado. El backend principal decide quién puede hacer qué y firma la URL; este servicio solo comprueba la firma. Es el mismo modelo que las *presigned URLs* de S3.
- **Por qué en la query y no en una cabecera**: así la URL funciona directamente en un `<img src="...">` o en un enlace de descarga, donde el navegador no deja añadir cabeceras.
- **Por qué atar el token al fichero y al scope**: un token filtrado de descarga de `foto.jpg` no sirve para borrarla ni para leer otro fichero.
- **Contrapartida**: los tokens en la URL pueden acabar en logs y en el historial del navegador. Por eso conviene que `exp` sea corto (minutos).

Antes hubo API key y Bearer token con middleware (ver historial de git); se quitaron en favor de este modelo, que es más simple.

### Validación de `id` y `file_name` con regex

- `id`: `^[A-Za-z0-9_-]+$`
- `file_name`: `^[A-Za-z0-9_-]+\.[A-Za-z0-9]+$` (nombre + una extensión)

Las rutas en disco se construyen como `files/{id}/{file_name}`, así que estas regex son la defensa contra **path traversal** (`../../etc/passwd`): no se permiten `/`, `..` ni caracteres raros. Son listas blancas y deliberadamente estrictas. Las regex se compilan una sola vez con `LazyLock`.

### Escritura atómica: `.tmp` + `rename`

Las subidas se escriben primero en `files/{id}/{file_name}.tmp` y, cuando terminan bien, se hace `rename` al nombre final.

- **Por qué**: si la subida falla a medias (corte de red, límite de tamaño superado), el fichero anterior **no se corrompe** y no quedan ficheros a medio escribir visibles. `rename` dentro del mismo sistema de ficheros es atómico.
- Si algo falla, se borra el `.tmp`, para no dejar basura (hay tests que lo comprueban).

### Streaming en vez de cargar en memoria

Subidas y descargas van por streams (`into_data_stream`, `field.chunk()`, `ReaderStream`): los datos se escriben o se envían a trozos. El uso de memoria no depende del tamaño del fichero.

### Dos formatos de subida

Se acepta tanto el cuerpo directo como multipart porque cubren clientes distintos: `fetch(url, { method: 'PUT', body: file })` envía el cuerpo directo; un `<form>` o `FormData` envía multipart. El formato se decide mirando el `Content-Type`.

### `PUT` para subir

La URL ya identifica el recurso (`/files/{id}/{file_name}`) y repetir la subida deja el mismo resultado (sobrescribe). Eso es la semántica de `PUT` (idempotente), no la de `POST`.

### Límite de tamaño con `DefaultBodyLimit`

El límite lo aplica Axum a nivel de router (`DefaultBodyLimit::max`), así que afecta tanto a subidas directas como a multipart y devuelve `413` sin que el handler tenga que contar bytes.

### CORS

`tower-http::CorsLayer` con un único origen (`ALLOWED_ORIGINS`) y solo los métodos `GET`, `PUT` y `DELETE`, para que solo tu frontend pueda llamar al servicio desde el navegador.

### Errores tipados

Todos los errores son variantes de `ResponseError` (en `src/lib.rs`), que implementa `IntoResponse`. Los handlers devuelven `Result<Response, ResponseError>` y usan `?`; la traducción a código HTTP está en un solo sitio.

### `lib.rs` + `main.rs`

La app se construye en `lib.rs` (`app()`) y `main.rs` solo la sirve. Así los tests de integración (`tests/`) pueden importar la app y levantarla en un puerto aleatorio (`0.0.0.0:0`) sin duplicar código.

## Docker

### `Dockerfile`: build multi-stage

1. **Builder** (`rust:1-slim-bookworm`): compila en release. Primero copia solo `Cargo.toml` y `Cargo.lock` y compila un `main` vacío, así **las dependencias quedan en una capa cacheada**. Al cambiar código en `src/` solo se recompila el crate, no todo Tokio/Axum.
2. **Runtime** (`debian:bookworm-slim`): solo el binario. La imagen final no lleva el toolchain de Rust (cientos de MB menos y menos superficie de ataque).

Se usa la misma base Debian (bookworm) en las dos etapas para que el binario enlace contra la misma glibc. No hace falta OpenSSL en runtime: `jsonwebtoken` usa el backend `rust_crypto` (Rust puro).

### `docker-compose.yml`

- **`ports: 3050:3050`**: el servidor hace bind a `0.0.0.0:3050`. Con `127.0.0.1` no sería accesible desde fuera del contenedor.
- **`env_file: .env`**: la configuración se pasa en runtime, no se mete en la imagen.
- **`volumes: ./files:/app/files`**: los ficheros subidos viven en el host. Sin el volumen, se perderían al recrear el contenedor.
- **`restart: unless-stopped`**: si el proceso cae (por ejemplo, un `panic`), Docker lo levanta otra vez.

### `.dockerignore`

Excluye `target/` (evita mandar GB de contexto al build), `files/`, `.git` y los `.env`. Los secretos **nunca** deben acabar dentro de la imagen.

## Tests y CI

```bash
cargo test
cargo clippy --all-targets -- -D warnings
```

- Tests unitarios en `src/handlers/photos.rs` y de integración en `tests/handlers_tests.rs`, que levantan el servidor real y le hacen peticiones con `reqwest`.
- Cubren: tokens incorrectos por usuario/fichero/scope, códigos de estado de cada error, límites de tamaño, casos raros de multipart y que no queden `.tmp` tras fallos.
- Necesitan `JWT_SECRET`, `ALLOWED_ORIGINS` y `MAX_BODY_LIMIT` en el entorno (o en `.env`).
- GitHub Actions (`.github/workflows/ci.yml`) ejecuta clippy con warnings como error y los tests en cada push a `main` y en cada PR.

## Estructura

```
src/
├── main.rs              # arranca el servidor en 0.0.0.0:3050
├── lib.rs               # ResponseError + app()
├── routes/mod.rs        # router, CORS, límite de tamaño
└── handlers/photos.rs   # handlers, validación JWT y regex, escritura en disco
tests/
└── handlers_tests.rs    # tests de integración
files/                   # almacenamiento (montado como volumen en Docker)
```
