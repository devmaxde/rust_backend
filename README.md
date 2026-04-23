# Backend

## Helpful commands
#### Build Models
```bash
sea-orm-cli generate entity -o db/src/models

```

#### Format code
```bash
cargo fmt --all
```

#### Running in watch mode
```bash
# install
# cargo install cargo-watch systemfd

systemfd --no-pid -s http::3000 -- cargo watch -x run
```

#### Running Stripe Webhook
```bash
stripe listen --forward-to localhost:3000/webhook
```
