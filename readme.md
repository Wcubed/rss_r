In-development, web-based feed reader built in rust. Despite the name, it can read RSS, ATOM and JSON feeds.

Https is not built-in, because the application is supposed to be behind a proxy, like nginx.

# Configuration
After the first run, there will be an `persistence/app_config.ron` file in the working directory.
See [app_config.rs](src/app_config.rs) for explanation of the parameters, and the default values.

# Development

# Commit messages
This repository follows the [conventional commits](https://www.conventionalcommits.org/) specification.

Changes that are development related (like the readme getting updated, or the build scripts being changed),
and shouldn't end up in the user-facing changelog, should be scoped with `dev`:
```
feat(dev): Added auto deploy.
```

Changes that should be included in the user-facing changelog don't need a scope:
```
feat: Added unicorns to the UI.
```

## Building
- To build: `cargo make build` or `cargo make build --release`.
- To build + run: `cargo make run` or `cargo make run --release`, then go to [https://localhost:8443/](https://localhost:8443/) in a web browser.

## Deploy to raspberry pi
- Install `podman` or `docker` (if you use `docker` you need to have the daemon running before continuing).
- Install the `zip` command
- Run `cargo make rpi-release` (tested on linux. Should work on windows, but not tested).
- Copy the zip from `target/packages/` to the target, and extract it.
- Run `rss_r`.

TODO (Wybe 2022-09-27): For deployment on the raspberry pi think about the following
  - [ ] Put the "update all feeds" button in the top bar.
  - [ ] Add a way for the frontend and the backend to check if they have the same `rss_com_lib` version. That way they can check if they can reliably communicate.
  - [x] Print the backend version on startup.
  - [x] Show the frontend version somewhere on the ui.
  - [ ] "Mark all as read" button.
  - [ ] Save the feed entries separately from the feed info for users? And have separate files on disk for each users individual feed info (read/not read) / settings?
    - [ ] And abstract this away from the rest of the application.
  - [ ] Auto backup the persistence directory every x time. And keep only every week or so. (do this after the daily check of the feeds)
  - [X] Add logging to a file
  - [ ] Add a script on my laptop i can use to make a backup of the persistence/log folder to my laptop (maybe even run it periodically).
  - [x] Auto Check all the feeds of ever user every day (and on startup). The request timeout for this can be a lot higher than when the user does it.
  - [x] Remove the feed cache. The collections store already remembers feeds.
  - [x] Add a "refresh all feeds" button on the ui, this gets the back-end to check all the feeds again.
  - [ ] Get favicons and save them. Then display them next to the feed names on the ui.
  - [x] When a user add's a feed to their collection, immediately do the first update. So there are never any feeds in the collections that haven't been updated at least once by the back-end.
  - [ ] Create a new log file every X time?
  - [ ] Add a scroll bar to the feed list on the left. Right now it runs off the screen if it is too long.
