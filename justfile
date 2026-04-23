# Database tasks
db-migrate-up:
	cd db && cargo run -- up

db-migrate-down:
	cd db && cargo run -- down

db-generate-models:
	sea-orm-cli generate entity -o db/src/models

up:
	docker compose -f docker-compose.dev.yaml up -d

down:
	docker compose -f docker-compose.dev.yaml down

# Main tasks
run:
	cargo run

watch:
	systemfd --no-pid -s http::3000 -- cargo watch -x run --features listenfd

build:
	cargo build

clean:
	cargo clean

# Git tasks
tag tag_name:
	git tag {{tag_name}}
	git push origin {{tag_name}}

# Convenience aliases
db: db-migrate-up
generate: db-generate-models
