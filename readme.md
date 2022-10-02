In-development, web-based feed reader built in rust. Despite the name, it can read RSS, ATOM and JSON feeds.

Https is not built-in, because the application is supposed to be behind a proxy, like nginx.

# Configuration
After the first run, there will be an `persistence/app_config.ron` file in the working directory.
See [app_config.rs](src/app_config.rs) for explanation of the parameters, and the default values.

# Building
- To build: `cargo make build` or `cargo make build --release`.
- To build + run: `cargo make run` or `cargo make run --release`, then go to [https://localhost:8443/](https://localhost:8443/) in a web browser.

# Deploy to raspberry pi
- Install `podman` or `docker` (if you use `docker` you need to have the daemon running before continuing).
- Run `cargo make release-rpi`.
- Copy the executable from `target/armv7-unknown-linux-gnueabihf/release/rss_r`, and the `resources` directory to the target.
- Run `rss_r`.

TODO (Wybe 2022-09-27): For deployment on the raspberry pi think about the following
  - [ ] Auto backup the persistence directory every x time. And keep only every week or so. (do this after the daily check of the feeds)
  - [X] Add logging to a file
  - [ ] Add a script on my laptop i can use to make a backup of the persistence/log folder to my laptop (maybe even run it periodically).
  - [x] Auto Check all the feeds of ever user every day (and on startup). The request timeout for this can be a lot higher than when the user does it.
  - [x] Remove the feed cache. The collections store already remembers feeds.
  - [x] Add a "refresh all feeds" button on the ui, this gets the back-end to check all the feeds again.
  - [ ] Get favicons and save them. Then display them next to the feed names on the ui.
  - [x] When a user add's a feed to their collection, immediately do the first update. So there are never any feeds in the collections that haven't been updated at least once by the back-end.