# Shortcut for common operations:

CRATES=proxmox proxmox-api-macro proxmox-sortable-macro

# By default we just run checks:
.PHONY: all
all: check

.PHONY: deb
deb: $(foreach c,$(CRATES), $c-deb)
	echo $(foreach c,$(CRATES), $c-deb)
	lintian build/*.deb

.PHONY: dinstall
dinstall:
	$(MAKE) clean
	$(MAKE) deb
	sudo -k dpkg -i build/librust-*.deb

%-deb:
	./build.sh $*
	touch $@

.PHONY: check
check:
	cargo +nightly fmt -- --check
	cargo test

# Run the api-test server, serving the api-test/www/ subdir as 'www' dir over
# http:
.PHONY: apitest
apitest:
	cargo run -p api-test -- api-test/www/

# Prints a diff between the current code and the one rustfmt would produce
.PHONY: fmt
checkfmt:
	cargo fmt --all -- --check

# Reformat the code (ppply the output of `make checkfmt`)
.PHONY: fmt
fmt:
	cargo fmt --all

# Doc without dependencies
.PHONY: doc
doc:
	cargo doc --no-deps

.PHONY: clean
clean:
	cargo clean
	rm -rf build *-deb

.PHONY: update
update:
	cargo update
