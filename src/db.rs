use sqlx::{migrate, postgres::PgPoolOptions, query, PgPool};

use crate::Pcb;

#[derive(Clone)]
pub struct Database(PgPool);

impl Database {
    pub async fn new() -> Self {
        let db_url = std::env::var("DATABASE_URL").expect("DATABASE_URL not set");

        let db = PgPoolOptions::new()
            .max_connections(5)
            .connect(&db_url)
            .await
            .expect("Could not connect to DB");

        migrate!()
            .run(&db)
            .await
            .expect("Could not run migration scripts");

        Self(db)
    }

    pub(crate) async fn get_all_entries(&self) -> Result<Vec<Pcb>, sqlx::Error> {
        query!("SELECT * FROM pcbs")
            .map(|pcb| Pcb {
                user_id: pcb.user_id as u64,
                username: pcb.username,
                location: (pcb.latitude, pcb.longitude),
                additional_information: pcb.additional_information,
            })
            .fetch_all(&self.0)
            .await
    }

    pub(crate) async fn insert_entry(&self, pcb: &Pcb) -> Result<(), sqlx::Error> {
        query!("INSERT INTO pcbs (user_id, username, latitude, longitude, additional_information) VALUES ($1, $2, $3, $4, $5)", pcb.user_id as i64, pcb.username, pcb.location.0, pcb.location.1, pcb.additional_information).execute(&self.0).await.map(|_| ())
    }
}
