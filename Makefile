# Default domain and query type if not provided on the command line
METHOD ?= post
DOMAIN ?= example.com
TYPE  ?= A
DOH_URL ?= http://127.0.0.1:8443/dns-query

.PHONY: doh query-doh serve query help

## Run the DoH server
doh:
	cargo run -p dns-server --bin doh

## Run DNS resolver in UDP/TCP/DoT mode
serve:
	cargo run -p dns-server --bin dns_resolver -- serve

## Query via DoH (Usage: make query-doh METHOD=post DOMAIN=google.com TYPE=AAAA)
query-doh:
	cargo run -p dns-server --bin dns_resolver -- doh $(METHOD) $(DOH_URL) $(DOMAIN) $(TYPE)

## Standard DNS query (Usage: make query DOMAIN=google.com)
query:
	cargo run -p dns-server --bin dns_resolver -- $(DOMAIN)

## Display help menu
help:
	@echo "Available commands:"
	@echo "  make doh                       - Run the DoH server"
	@echo "  make serve                     - Run DNS resolver in server mode"
	@echo "  make query [DOMAIN=...]        - Query standard DNS (default: example.com)"
	@echo "  make query-doh [DOMAIN=...]    - Query via DoH (default: example.com A)"
