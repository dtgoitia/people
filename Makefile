set_up_repo_for_development:
	pre-commit install
	cargo build

build_for_release:
	cargo build --release

install_cli_in_system: build_for_release
	cp target/release/people_summary $(HOME)/.local/bin/people_summary
	cp target/release/people_per_person $(HOME)/.local/bin/people_per_person
