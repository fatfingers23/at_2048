lint:
    yew-fmt ./client_2048/src/*.rs

release-build:
    APP_ORIGIN=https://2048.blue trunk build --config ./client_2048/Trunk.toml --release
    cp ./production_configs/oauth-client-metadata.json ./client_2048/dist/oauth-client-metadata.json
    echo "/oauth-client-metadata.json /oauth-client-metadata.json" > ./client_2048/dist/_redirects

release:
    #trunk build --config ./client_2048/Trunk.toml --release
    docker buildx build \
              --platform linux/arm64 \
              -t fatfingers23/at_2048_appview:latest \
              -f dockerfiles/AppView.Dockerfile \
              --push .
    docker buildx build \
              --platform linux/arm64 \
              -t fatfingers23/at_2048_web_server:latest \
              -f dockerfiles/Caddy.Dockerfile \
              --push .
