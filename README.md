## Cross compilation 

### Mac -> Linux

Add the following to your `~/.cargo/config` file:

```toml
[target.x86_64-unknown-linux-gnu]
linker = "x86_64-unknown-linux-gnu-gcc"

[target.x86_64-unknown-linux-musl]
linker = "x86_64-linux-musl-gcc"
```

install the following packages:

```bash
brew install FiloSottile/musl-cross/musl-cross
brew install SergioBenitez/osxct/x86_64-unknown-linux-gnu
```

*Alpine*

This is what is currently used by the Dockerfile and should be run before running ```docker build```.
```bash
TARGET_CC=x86_64-linux-musl-gcc cargo build --release --target x86_64-unknown-linux-musl 
```

*Debian*

Run the following command to build the binary for linux:
```bash
TARGET_CC=x86_64-unknown-linux-gnu-gcc cargo build --target --release x86_64-unknown-linux-gnu 
```

## Docker Build

This Docker build uses [Dockerfile syntax 1.4](https://github.com/moby/buildkit/blob/master/frontend/dockerfile/docs/reference.md#syntax), 
this requires ```DOCKER_BUILDKIT=1``` to be set in your environment.

```bash
DOCKER_BUILDKIT=1 docker build -t rusty-poker .
```
## Environment Variables

| Variable              | Description                                                                                            |
|-----------------------|--------------------------------------------------------------------------------------------------------|
| ```RUST_LOG```        | Sets the log level for the API.                                                                        |
| ```RUST_BACKTRACE```  | Enables backtraces for the API.                                                                        |
| ```PORT```            | The port to bind the public API to.                                                                    |
| ```HOST```            | The host to bind the public API to. ( Should be 0.0.0.0 inside a docker contianer )                    |
| ```ORIGIN```          | The origin to allow for CORS on the frontend.                                                          |
| ```PUBLIC_API_HOST``` | The host to bind the public API to, required for frontend websocket.                                   |
| ```PUBLIC_API_URI```  | The URI to bind the public API to, required for frontend websocket. This must end in a trailing slash. |
