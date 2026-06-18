<div align="center">
  <h1>jsonhost</h1>
  <p>A JSON document store backed by <a href="https://github.com/nsg/stathost">stathost</a>.</p>
</div>

---

## About

jsonhost is a thin, stateless layer that turns [stathost](https://github.com/nsg/stathost)
into a JSON document store. Where stathost serves opaque files from buckets,
jsonhost serves JSON documents from collections — validating JSON on the way in,
serving `application/json` on the way out, and mapping each document to a file in
the backing bucket.

It is deliberately small. jsonhost stores no data and holds no secrets: a
collection maps 1:1 to a stathost bucket, a document `cars/volvo` maps to the
file `cars/volvo.json`, and the client's `Authorization` token is forwarded
straight through to stathost, which remains the sole authority on
authentication. The only thing jsonhost needs to know is the URL of its stathost
backend.

## Features

- **JSON in, JSON out** — request bodies are validated as JSON; responses are always `application/json`
- **Human-readable slugs** — name documents yourself: `PUT /cars/volvo`, not server-generated IDs
- **Stateless** — no database, no secrets, no local storage; one config value (the stathost URL)
- **Transparent auth** — bearer tokens pass through to stathost unchanged, so read/write tokens and public reads are whatever stathost provides
- **Nested collections** — slugs may be nested (`/cars/eu/volvo`)
- **Single static binary** — also published as a minimal container image

## How It Works

```
client ──► jsonhost ──► stathost ──► filesystem
           (validate     (buckets,
            JSON, map     tokens,
            paths,        bytes)
            forward token)
```

| jsonhost            | stathost                          |
| ------------------- | --------------------------------- |
| collection `cars`   | bucket `cars`                     |
| document `volvo`    | file `volvo.json`                 |
| `GET /cars`         | `GET /cars/_meta/list`            |
| client token        | forwarded verbatim (incl. absent) |

Because the token is forwarded as-is, jsonhost works against any stathost
version: with today's public-read stathost, `GET` without a token just works;
with a stathost that requires a read token, the same request returns whatever
stathost returns. jsonhost makes no auth decisions of its own.

## Quick Start

### 1. Run a stathost backend

See [stathost](https://github.com/nsg/stathost). Create a bucket named after the
collection you want, e.g. `cars`, with a write token.

### 2. Run jsonhost

Download a binary from [Releases](https://github.com/nsg/jsonhost/releases), or
build from source:

```bash
cargo build --release
JSONHOST_STATHOST_URL=http://localhost:8080 ./target/release/jsonhost
```

### 3. Use it

```bash
# Store a document (write token required by the backend)
curl -X PUT -H "Authorization: Bearer my-token" \
  -d '{"brand":"Volvo","model":"XC90"}' \
  http://localhost:8090/cars/volvo

# Fetch it (public read against a default stathost)
curl http://localhost:8090/cars/volvo

# List documents in the collection
curl -H "Authorization: Bearer my-token" http://localhost:8090/cars
# => ["volvo"]
```

## Configuration

Create a `jsonhost.toml`:

```toml
[server]
host = "0.0.0.0"
port = 8090
stathost_url = "http://localhost:8080"
```

All settings are optional and have sensible defaults. Environment variables
override the file — useful for containers:

| Variable                | Overrides              | Default                 |
| ----------------------- | ---------------------- | ----------------------- |
| `JSONHOST_STATHOST_URL` | `server.stathost_url`  | `http://localhost:8080` |
| `JSONHOST_HOST`         | `server.host`          | `0.0.0.0`               |
| `JSONHOST_PORT`         | `server.port`          | `8090`                  |

```bash
# Run with a custom config file
jsonhost --config /etc/jsonhost/jsonhost.toml
```

## API Reference

Authentication is handled entirely by stathost; jsonhost forwards the
`Authorization: Bearer <token>` header unchanged. Which operations require a
token (and whether reads are public) depends on your stathost configuration.

### Store a document

```http
PUT /{collection}/{slug}
Authorization: Bearer <token>
Content-Type: application/json

{ "any": "json" }
```

Validates the body as JSON and stores it as `{slug}.json` in the backing bucket.
Returns `400` if the body is not valid JSON or the name is unsafe. Create-or-replace.

### Fetch a document

```http
GET /{collection}/{slug}
```

Returns the JSON document.

### Delete a document

```http
DELETE /{collection}/{slug}
Authorization: Bearer <token>
```

### List documents

```http
GET /{collection}
Authorization: Bearer <token>
```

Returns a JSON array of document slugs (the `.json` suffix is stripped):

```json
["volvo", "volkswagen", "eu/polestar"]
```

### OpenAPI spec

```http
GET /openapi.json
```

## Docker

A minimal image (a single static binary on `scratch`) is published to GHCR on
each release:

```bash
docker run -p 8090:8090 \
  -e JSONHOST_STATHOST_URL=http://stathost:8080 \
  ghcr.io/nsg/jsonhost:latest
```

## License

MIT
