In-development, web-based feed reader built in rust. Despite the name, it can read RSS, ATOM and JSON feeds.

The test certificate in `resources/local-ssl` has been generated with `mkcert localhost localhost`

# Building
- To build: `cargo make build`.
- To build + run: `cargo make run`, then go to [https://localhost:8443/](https://localhost:8443/) in a web browser.