# PDFiller
PDF Form filler API with a built-in reverse proxy with Nginx, Amazon S3 integration for storage and MongoDB.

## Running the web service
The application uses `docker-compose` in order to run all the needed services. You have to install last Docker CE with Docker compose in order to run it.

## Compiling
You must compile the binary before running it, use the command `cargo +stable build` and then `cargo +stable run` in order to compile and execute it.

For the standard compile just run `cargo +stable build --release` but if you have to run the application inside a container, you have to build it with `cargo +stable build --release --target x86_64-unknown-linux-musl --locked`.

## Commands
Running:

* `make start_local` starts the local instance (WARNING: you must have MongoDB running locally)
* `make start` for default: starts the dev instance
* `make start_prod` for the production instance

*Append `_recreate` to each command in order to force the recreation of containers.*

Stopping:

* `make stop` to shutdown all containers

## TO DO

The following features aren't implemented yet:
- [x] Image fields with a pattern for the field name
- [x] Merge all PDFs into one PDF in addition to ZIP option (default)
- [x] Fix merged PDF order while uploaded or compiled
- [ ] PDF's pages rasterization (flattening)
- [ ] Files caching
- [ ] Temporary files deletion monitor
