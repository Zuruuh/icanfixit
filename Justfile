publish:
    docker buildx build . --tag registry.zuruh.dev/icanfixit --push --platform linux/amd64
