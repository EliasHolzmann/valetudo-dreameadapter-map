use sqlx::{PgPool, postgres::PgPoolOptions, migrate, query};

use crate::Pcb;

#[derive(Clone)]
pub struct Database(PgPool);

impl Database {
    pub async fn new() -> Self {
        let db_url = std::env::var("DATABASE_URL").expect("DATABASE_URL not set");

        let db = PgPoolOptions::new().max_connections(5).connect(&db_url).await.expect("Could not connect to DB");

        migrate!().run(&db).await.expect("Could not run migration scripts");

        Self(db)
    }

    pub(crate) async fn get_all_entries(&self) -> Vec<Pcb> {
        query!("SELECT * FROM pcbs");
        todo!()
    }
}