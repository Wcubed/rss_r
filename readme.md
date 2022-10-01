In-development, web-based feed reader built in rust. Despite the name, it can read RSS, ATOM and JSON feeds.

Https is not built-in, because the application is supposed to be behind an nginx server.

# Building
- To build: `cargo make build`.
- To build + run: `cargo make run`, then go to [https://localhost:8443/](https://localhost:8443/) in a web browser.


TODO (Wybe 2022-09-27): For deployment on my raspberry pi, build everything in --release mode. How do I add that option neatly to the Makefile.toml? are there examples for that?
TODO (Wybe 2022-09-27): For deployment on the raspberry pi think about the following
  - The https is handled by the rpi itself (because it is also serving dnd foundryvtt) so doesn't need to be done in rust
  - I want backups of the persistence folder
  - Logging to a file is needed.
  - Add a script on my laptop i can use to make a backup of the persistence/log folder to my laptop (maybe even run it periodically).