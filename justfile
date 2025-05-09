lint:
    yew-fmt ./client_2048/src/*.rs

release-build:
    APP_ORIGIN=https://2048.blue trunk build --config ./client_2048/Trunk.toml --release

release:
    #trunk build --config ./client_2048/Trunk.toml --release
    docker buildx build \
              --platform linux/arm64 \
              -t fatfingers23/at_2048_api:latest \
              -f dockerfiles/Api.Dockerfile \
              --push .
    docker buildx build \
              --platform linux/arm64 \
              -t fatfingers23/at_2048_web_server:latest \
              -f dockerfiles/Caddy.Dockerfile \
              --push .
