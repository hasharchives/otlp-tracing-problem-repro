## TL;DR

This example HTTP server showcases some problematic interactions with tracing libraries.
When using `error-stack` (which uses `SpanTrace::capture()` under the hood), our future containing
HTTP handling for a route will halt and never finish.

The halting can be 'prevented' in the following ways:

- Removing `tracing::*!` calls
- Disabling the application's tracing `opentelemetry_layer`
- Disabling the application's tracing `error_layer`
- Removing the `error-stack` `"spantrace"` feature

## How to run/test this, and what observations to make

For introspection, this application uses `tokio-console` which requires `tokio_unstable`

Run the application with

```console
RUSTFLAGS="--cfg tokio_unstable" RUST_LOG="trace" cargo run
```

Once running, going to http://localhost:3000/ should result in a never-responding connection.
Using any of the listed ways to prevent the halting from above (for example removing the `spantrace` feature of `error-stack`), you'll be greeted with an HTTP 500 response at http://localhost:3000/.
