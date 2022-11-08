use mongodb;
use serde_json;

pub struct DbSession {
    pub client: mongodb::Client,
    pub db_name: String
}

impl DbSession {
    pub async fn new(db_name: &str)-> Self {
        let uri = std::env::var("MONGO_DB_URI").expect("error: MONGO_DB_URI env variable must be set");
        DbSession {
            client: mongodb::Client::with_uri_str(uri).await.expect("error: Couldn't connect to database with URI"),
            db_name: db_name.to_string()
        }
    }

    pub async fn add_server_json(&self, json: &serde_json::Value) -> Result<(), mongodb::error::Error> {
        let collection = self.client.database(&self.db_name).collection::<serde_json::Value>("servers_found"); // hard coded
        collection.insert_one(json, None).await?;
        Ok(())
    }
}