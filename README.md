## TL;DR

This is an example of circularly logging a span trace using `SpanTrace::capture()`. The future executing
the logging stalls forever.

## How to run/test this, and what observations to make

Run the application with

```console
$ docker run -d --rm --name jaeger -e COLLECTOR_OTLP_ENABLED=true -p 14269:14269 -p16686:16686 -p4317:4317 -p 4318:4318 jaegertracing/all-in-one:1.40
$ RUST_LOG="trace" cargo run
```

Expected behavior: program stalls and never finishes.
By removing the data part of the `error!` call, we can prevent the trace and program runs as expected.
