# Osmosis Bindings

CosmWasm bindings to custom Osmosis features

## Optimizing Workspace Builds

```sh
sudo docker run --rm -v "$(pwd)":/code   --mount type=volume,source="$(basename "$(pwd)")_cache",target=/code/target   --mount type=volume,source=registry_cache,target=/usr/local/cargo/registry   cosmwasm/workspace-optimizer:0.12.6
```
