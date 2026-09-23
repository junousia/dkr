SHELL := /bin/sh

CARGO ?= cargo
ARGS ?=

.DEFAULT_GOAL := help

.PHONY: help check ci lint fmt-check fmt format test build package publish-dry-run publish clean run

help: ## Show available developer commands.
	@printf '\033[1mdkr developer commands\033[0m\n'
	@printf 'Run one-off commands from docker images using named per-image profiles.\n\n'
	@printf '\033[1mUsage\033[0m\n'
	@printf '  make \033[36m<target>\033[0m [ARGS=value]\n\n'
	@printf '\033[1mVerification\033[0m\n'
	@printf '  \033[36mcheck\033[0m       Format check, build-check, and run all tests\n'
	@printf '  \033[36mci\033[0m          Alias for check\n'
	@printf '  \033[36mlint\033[0m        cargo fmt --check + cargo check\n'
	@printf '  \033[36mfmt-check\033[0m   Check Rust formatting\n'
	@printf '  \033[36mfmt\033[0m         Apply Rust formatting\n'
	@printf '  \033[36mformat\033[0m      Alias for fmt\n\n'
	@printf '\033[1mTests\033[0m\n'
	@printf '  \033[36mtest\033[0m        Run all tests\n\n'
	@printf '\033[1mBuild And Release\033[0m\n'
	@printf '  \033[36mbuild\033[0m       Build release binary\n'
	@printf '  \033[36mpackage\033[0m     Verify crates.io package contents\n'
	@printf '  \033[36mpublish-dry-run\033[0m Validate crates.io publishing without uploading\n'
	@printf '  \033[36mpublish\033[0m     Publish the crate to crates.io\n'
	@printf '  \033[36mclean\033[0m       Remove Cargo build output\n\n'
	@printf '\033[1mRun\033[0m\n'
	@printf '  \033[36mrun\033[0m         Run dkr: make run ARGS="myimage --dry-run"\n\n'
	@printf '\033[1mVariables\033[0m\n'
	@printf '  ARGS=%s\n' '$(ARGS)'

check: lint test ## Format check, build-check, and run all tests.

ci: check ## Alias for check.

lint: fmt-check ## Run formatting check and cargo check.
	$(CARGO) check

fmt-check: ## Check Rust formatting.
	$(CARGO) fmt --check

fmt: ## Apply Rust formatting.
	$(CARGO) fmt

format: fmt ## Alias for fmt.

test: ## Run all tests.
	$(CARGO) test $(ARGS)

build: ## Build release binary.
	$(CARGO) build --release

package: ## Verify crates.io package contents.
	$(CARGO) package --list

publish-dry-run: ## Validate crates.io publishing without uploading.
	$(CARGO) publish --dry-run

publish: ## Publish the crate to crates.io.
	$(CARGO) publish

clean: ## Remove Cargo build output.
	$(CARGO) clean

run: ## Run dkr: make run ARGS="myimage --dry-run".
	$(CARGO) run -- $(ARGS)
