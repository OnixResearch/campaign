# Cairn policy provenance

Campaign initialized its local lifecycle policy from Cairn revision `5d5aa7dfc02e6cfd076a18c54b09954d23e29f2b`.

Runtime validation consumes `cairn-policy/generated/cairn-policy.json`. Refresh it only with the pinned Cairn provider, the same revision that the repository flake pins through `inputs.cairn`:

```bash
nix shell github:OnixResearch/cairn/5d5aa7dfc02e6cfd076a18c54b09954d23e29f2b#cairn nixpkgs#nickel -c cairn policy export --output cairn-policy/generated/cairn-policy.json
```

That revision is not anonymously fetchable. The refresh needs read access to the private `OnixResearch/cairn` repository and its locked inputs; the README section "Building and checking" lists them.

A passing policy export or validation receipt proves only policy freshness and lifecycle-shape checks for the selected inputs. It does not prove lifecycle artifact truth, review quality, implementation correctness, or release eligibility.
