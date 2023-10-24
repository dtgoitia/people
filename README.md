## Usage

Add a config file:

```yaml
# ~/.config/people/config.yaml
people_dir: ~/people  # directory where the people logs are stored
ignore:               # people to ignore from the log
  - JohnDoe
  - JaneDoe
```

Build and install CLI:

```shell
make install_cli_in_system
```

## Development

```shell
make set_up_repo_for_development
```

## Roadmap

- [x] Support config file
- [x] Write logic to parse log file
- [x] Add binary to show people summary
- [ ] Add binary to group interactions by person
