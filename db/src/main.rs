use sea_orm_migration::prelude::*;
mod migrations;

#[async_std::main]
async fn main() {
    cli::run_cli(migrations::Migrator).await;
}
